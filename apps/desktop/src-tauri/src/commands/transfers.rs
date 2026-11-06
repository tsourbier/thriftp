use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};

use serde::Serialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Emitter, State};
use thriftp_core::connections::ConnectionService;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    EntryType, PartialTransferData, Priority, Transfer, TransferDirection, TransferHistory,
    TransferQueue, TransferState,
};
use thriftp_core::protocol::RemoteFs;
use thriftp_core::queue::QueueService;
use thriftp_core::repository::{TransferState as PersistedTransferState, TransferStore};
use thriftp_core::transfers::TransferControl;
use thriftp_core::transfers::conflict::{ConflictPolicy, rename_candidate};
use thriftp_core::transfers::engine::{ProgressObserver, execute_download, execute_upload_to};
use thriftp_core::transfers::recursive::{SymlinkPolicy, plan_download, plan_upload};
use thriftp_core::transfers::resume::{
    new_partial, partial_path as local_partial_path, remove_artifacts, write_metadata,
};
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;

/// Synchronous global token bucket used by transfer progress callbacks.
///
/// The protocol adapters invoke progress synchronously, so the shared bucket
/// throttles the worker rather than requiring an async callback contract.
#[derive(Debug)]
struct SyncBandwidth {
    rate: Option<u64>,
    capacity: f64,
    tokens: f64,
    last: Instant,
}

impl SyncBandwidth {
    fn new(rate: Option<u64>) -> Self {
        let capacity = rate.unwrap_or(0) as f64;
        Self {
            rate,
            capacity,
            tokens: capacity,
            last: Instant::now(),
        }
    }

    fn set_rate(&mut self, rate: Option<u64>) {
        self.rate = rate.filter(|value| *value > 0);
        self.capacity = self.rate.unwrap_or(0) as f64;
        self.tokens = self.capacity;
        self.last = Instant::now();
    }

    fn throttle(&mut self, bytes: u64) {
        let Some(rate) = self.rate else {
            return;
        };
        if bytes == 0 {
            return;
        }
        let request = (bytes as f64).min(self.capacity.max(1.0));
        let now = Instant::now();
        self.tokens = (self.tokens + now.duration_since(self.last).as_secs_f64() * rate as f64)
            .min(self.capacity);
        self.last = now;
        if self.tokens < request {
            let wait = Duration::from_secs_f64(((request - self.tokens) / rate as f64).min(1.0));
            tokio::task::block_in_place(|| std::thread::sleep(wait));
            self.tokens = 0.0;
            self.last = Instant::now();
        } else {
            self.tokens -= request;
        }
    }
}

pub struct TransferManager {
    pub queue: QueueService,
    controls: HashMap<Uuid, TransferControl>,
    local_paths: HashMap<Uuid, PathBuf>,
    partials: HashMap<Uuid, PartialTransferData>,
    history: TransferHistory,
    bandwidth: Arc<Mutex<SyncBandwidth>>,
    store: Option<Arc<dyn TransferStore>>,
}

impl TransferManager {
    #[must_use]
    pub fn new(concurrency_limit: usize, store: Option<Arc<dyn TransferStore>>) -> Self {
        Self {
            queue: QueueService::new(concurrency_limit),
            controls: HashMap::new(),
            local_paths: HashMap::new(),
            partials: HashMap::new(),
            history: TransferHistory::default(),
            bandwidth: Arc::new(Mutex::new(SyncBandwidth::new(None))),
            store,
        }
    }

    pub fn load(
        &mut self,
        queue: thriftp_core::model::TransferQueue,
        transfers: Vec<Transfer>,
        partials: Vec<PartialTransferData>,
        history: TransferHistory,
    ) {
        self.bandwidth
            .lock()
            .expect("bandwidth lock")
            .set_rate(queue.bandwidth_limit_bytes_per_second);
        self.queue.load(queue, transfers);
        self.partials = partials
            .into_iter()
            .map(|partial| (partial.transfer_id, partial))
            .collect();
        self.history = history;
    }

    pub fn persist(&self) -> CoreResult<()> {
        let Some(store) = &self.store else {
            return Ok(());
        };
        let state = PersistedTransferState {
            queue: self.queue.queue.clone(),
            transfers: self.queue.list(),
            partial_transfers: self.partials.values().cloned().collect(),
            history: self.history.clone(),
        };
        store.save(&state)
    }

    fn record_terminal(&mut self, transfer: &Transfer) {
        if !matches!(
            transfer.state,
            TransferState::Completed | TransferState::Failed | TransferState::Cancelled
        ) {
            return;
        }
        self.history.entries.retain(|entry| entry.id != transfer.id);
        self.history.push(transfer.clone());
    }

    fn remember_partial(
        &mut self,
        transfer: &Transfer,
        local_path: Option<PathBuf>,
        source_modified_at: Option<DateTime<Utc>>,
    ) {
        let partial = new_partial(
            transfer.id,
            local_path.as_deref(),
            (transfer.direction == TransferDirection::Upload)
                .then_some(transfer.destination_path.as_str()),
            transfer.transferred_bytes,
            transfer.total_bytes,
            source_modified_at,
        );
        if let Some(path) = local_path.as_deref()
            && let Err(error) = write_metadata(path, &partial)
        {
            tracing::warn!(%error, transfer_id = %transfer.id, "could not write resume metadata");
        }
        self.partials.insert(transfer.id, partial);
    }

    fn clear_partial(&mut self, transfer_id: Uuid) {
        self.partials.remove(&transfer_id);
    }

    fn partial(&self, transfer_id: Uuid) -> Option<PartialTransferData> {
        self.partials.get(&transfer_id).cloned()
    }

    pub fn set_bandwidth(&mut self, bytes_per_second: Option<u64>) {
        self.queue.set_bandwidth(bytes_per_second);
        self.bandwidth
            .lock()
            .expect("bandwidth lock")
            .set_rate(bytes_per_second);
    }

    fn bandwidth_limiter(&self) -> Arc<Mutex<SyncBandwidth>> {
        Arc::clone(&self.bandwidth)
    }

    fn clear_history(&mut self) -> usize {
        let cleared = self.history.entries.len();
        self.history.entries.clear();
        cleared
    }

    fn register(&mut self, transfer: Transfer, local_path: PathBuf) -> CoreResult<Uuid> {
        let control = TransferControl::new();
        let id = self.queue.enqueue(transfer)?;
        self.controls.insert(id, control);
        self.local_paths.insert(id, local_path);
        Ok(id)
    }

    pub fn pause_all(&self) {
        for control in self.controls.values() {
            control.pause();
        }
    }

    pub fn cancel_all(&self) -> usize {
        for control in self.controls.values() {
            control.cancel();
        }
        self.controls.len()
    }

    fn control(&self, id: Uuid) -> CoreResult<TransferControl> {
        self.controls
            .get(&id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))
    }
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new(4, None)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartTransfersView {
    pub transfer_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TransferProgressPayload {
    transfer_id: String,
    transferred_bytes: u64,
    total_bytes: Option<u64>,
    speed_bytes_per_second: Option<u64>,
    estimated_remaining_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferStatePayload {
    pub transfer_id: String,
    pub connection_profile_id: String,
    pub direction: TransferDirection,
    pub state: TransferState,
    /// Authoritative bytes moved so far, including the final completion amount.
    pub transferred_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DesktopError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// Build the serializable state event payload for one transfer.
#[must_use]
pub fn transfer_state_payload(
    transfer: &Transfer,
    error: Option<DesktopError>,
) -> TransferStatePayload {
    TransferStatePayload {
        transfer_id: transfer.id.to_string(),
        connection_profile_id: transfer.connection_profile_id.to_string(),
        direction: transfer.direction,
        state: transfer.state,
        transferred_bytes: transfer.transferred_bytes,
        total_bytes: transfer.total_bytes,
        error,
        started_at: transfer.started_at,
        completed_at: transfer.completed_at,
    }
}

fn parse_id(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| CoreError::Validation(format!("invalid id: {error}")))
}

fn join_remote(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

fn emit_queue_state(app: &AppHandle, queue: &TransferQueue) {
    let _ = app.emit("queue:state", queue.clone());
}

fn emit_state(app: &AppHandle, transfer: &Transfer, error: Option<DesktopError>) {
    let _ = app.emit("transfer:state", transfer_state_payload(transfer, error));
}

async fn destination_exists(
    remote: &mut dyn RemoteFs,
    direction: TransferDirection,
    destination: &str,
) -> bool {
    match direction {
        TransferDirection::Upload => remote.stat(destination).await.is_ok(),
        TransferDirection::Download => Path::new(destination).exists(),
    }
}

async fn resolve_destination(
    remote: &mut dyn RemoteFs,
    direction: TransferDirection,
    destination: &str,
    policy: ConflictPolicy,
) -> CoreResult<Option<String>> {
    if !destination_exists(remote, direction, destination).await {
        return Ok(Some(destination.to_owned()));
    }
    match policy {
        ConflictPolicy::Overwrite => Ok(Some(destination.to_owned())),
        ConflictPolicy::Skip => Ok(None),
        ConflictPolicy::Rename => {
            let candidate = rename_candidate(destination);
            if destination_exists(remote, direction, &candidate).await {
                return Err(CoreError::Conflict(format!(
                    "both {destination} and {candidate} already exist"
                )));
            }
            Ok(Some(candidate))
        }
        ConflictPolicy::Ask => Err(CoreError::Conflict(format!(
            "destination already exists: {destination}"
        ))),
    }
}

#[allow(clippy::too_many_arguments)]
async fn enqueue_upload(
    manager: &mut TransferManager,
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    profile_id: Uuid,
    source: &Path,
    destination: &str,
    conflict_policy: ConflictPolicy,
) -> CoreResult<(Vec<Uuid>, Vec<String>)> {
    let plan = plan_upload(source, SymlinkPolicy::Recreate)?;
    for entry in &plan {
        if entry.entry_type == EntryType::Directory {
            let path = join_remote(destination, &entry.relative_path);
            let _ = remote.mkdir(&path).await;
        }
    }
    let mut ids = Vec::new();
    let mut skipped_links = Vec::new();
    for entry in plan {
        let remote_path = join_remote(destination, &entry.relative_path);
        match entry.entry_type {
            EntryType::Directory => {}
            EntryType::Symlink => {
                let Some(target) = entry.target.as_deref() else {
                    skipped_links.push(remote_path);
                    continue;
                };
                if let Err(error) = remote.create_symlink(target, &remote_path).await {
                    skipped_links.push(format!("{remote_path}: {error}"));
                }
            }
            EntryType::File => {
                let Some(remote_path) = resolve_destination(
                    remote,
                    TransferDirection::Upload,
                    &remote_path,
                    conflict_policy,
                )
                .await?
                else {
                    continue;
                };
                let local_path = source.join(
                    entry
                        .relative_path
                        .replace('/', std::path::MAIN_SEPARATOR_STR),
                );
                let transfer = Transfer::new(
                    TransferDirection::Upload,
                    profile_id,
                    local_path.display().to_string(),
                    remote_path,
                    entry.size,
                );
                ids.push(manager.register(transfer, local_path)?);
            }
            EntryType::Other => {}
        }
    }
    Ok((ids, skipped_links))
}

async fn enqueue_download(
    manager: &mut TransferManager,
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    profile_id: Uuid,
    source: &str,
    destination: &Path,
    conflict_policy: ConflictPolicy,
) -> CoreResult<(Vec<Uuid>, Vec<String>)> {
    let entry = remote.stat(source).await?;
    if entry.entry_type == EntryType::Symlink {
        let local_path = destination.join(&entry.name);
        let restored = entry
            .symlink_target
            .as_deref()
            .is_some_and(|target| create_local_symlink(target, &local_path).is_ok());
        return Ok((
            Vec::new(),
            if restored {
                Vec::new()
            } else {
                vec![local_path.display().to_string()]
            },
        ));
    }
    if entry.entry_type != EntryType::Directory {
        let local_path = destination.join(entry.name);
        let local_path = resolve_destination(
            remote,
            TransferDirection::Download,
            &local_path.display().to_string(),
            conflict_policy,
        )
        .await?
        .map(PathBuf::from);
        let Some(local_path) = local_path else {
            return Ok((Vec::new(), Vec::new()));
        };
        let transfer = Transfer::new(
            TransferDirection::Download,
            profile_id,
            source.to_owned(),
            local_path.display().to_string(),
            entry.size,
        );
        return Ok((vec![manager.register(transfer, local_path)?], Vec::new()));
    }

    let root = entry.name;
    let plan = plan_download(remote, source, SymlinkPolicy::Recreate).await?;
    let local_root = destination.join(&root);
    for planned in &plan {
        if planned.entry_type == EntryType::Directory {
            let path = local_root.join(
                planned
                    .relative_path
                    .replace('/', std::path::MAIN_SEPARATOR_STR),
            );
            std::fs::create_dir_all(path).map_err(|error| {
                CoreError::internal(format!("could not create directory: {error}"))
            })?;
        }
    }
    let mut ids = Vec::new();
    let mut skipped_links = Vec::new();
    for planned in plan {
        let local_path = local_root.join(
            planned
                .relative_path
                .replace('/', std::path::MAIN_SEPARATOR_STR),
        );
        match planned.entry_type {
            EntryType::Directory => {}
            EntryType::Symlink => {
                let Some(target) = planned.target.as_deref() else {
                    skipped_links.push(local_path.display().to_string());
                    continue;
                };
                if let Err(error) = create_local_symlink(target, &local_path) {
                    skipped_links.push(format!("{}: {error}", local_path.display()));
                }
            }
            EntryType::File => {
                let Some(local_path) = resolve_destination(
                    remote,
                    TransferDirection::Download,
                    &local_path.display().to_string(),
                    conflict_policy,
                )
                .await?
                .map(PathBuf::from) else {
                    continue;
                };
                let remote_path = join_remote(source, &planned.relative_path);
                let transfer = Transfer::new(
                    TransferDirection::Download,
                    profile_id,
                    remote_path,
                    local_path.display().to_string(),
                    planned.size,
                );
                ids.push(manager.register(transfer, local_path)?);
            }
            EntryType::Other => {}
        }
    }
    Ok((ids, skipped_links))
}

fn create_local_symlink(target: &str, link_path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link_path)
    }
    #[cfg(not(unix))]
    {
        let _ = (target, link_path);
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "symbolic links are not supported on this platform",
        ))
    }
}

async fn verify_resume_state(
    remote: &mut dyn RemoteFs,
    transfer: &Transfer,
    local_path: &Path,
    partial: Option<&PartialTransferData>,
) -> CoreResult<()> {
    if let Some(partial) = partial
        && let Some(recorded) = partial.source_modified_at
    {
        let current = match transfer.direction {
            TransferDirection::Download => remote
                .stat(&transfer.source_path)
                .await
                .ok()
                .and_then(|entry| entry.modified_at),
            TransferDirection::Upload => std::fs::metadata(&transfer.source_path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .map(DateTime::<Utc>::from),
        };
        if current != Some(recorded) {
            return Err(CoreError::Conflict(
                "the source modification time changed since the transfer stopped".to_owned(),
            ));
        }
    }
    match transfer.direction {
        TransferDirection::Download => {
            let partial = local_partial_path(local_path);
            let length = std::fs::metadata(&partial)
                .map(|metadata| metadata.len())
                .unwrap_or(0);
            if length != transfer.transferred_bytes {
                return Err(CoreError::Conflict(
                    "the partial download no longer matches the recorded offset".to_owned(),
                ));
            }
            let source = remote.stat(&transfer.source_path).await?;
            if let (Some(total), Some(size)) = (transfer.total_bytes, source.size)
                && total != size
            {
                return Err(CoreError::Conflict(
                    "the remote source size changed since the transfer stopped".to_owned(),
                ));
            }
        }
        TransferDirection::Upload => {
            let source = std::fs::metadata(&transfer.source_path).map_err(|error| {
                CoreError::NotFound(format!("source file could not be read: {error}"))
            })?;
            if let Some(total) = transfer.total_bytes
                && source.len() != total
            {
                return Err(CoreError::Conflict(
                    "the local source size changed since the transfer stopped".to_owned(),
                ));
            }
            let remote_partial = format!("{}.thriftp-part", transfer.destination_path);
            let length = match remote.stat(&remote_partial).await {
                Ok(entry) => entry.size.unwrap_or(0),
                Err(CoreError::NotFound(_)) => 0,
                Err(error) => return Err(error),
            };
            if length != transfer.transferred_bytes {
                return Err(CoreError::Conflict(
                    "the remote partial file no longer matches the recorded offset".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

async fn dispatch_available(
    app: AppHandle,
    connections: Arc<tokio::sync::Mutex<ConnectionService>>,
    manager: Arc<tokio::sync::Mutex<TransferManager>>,
) {
    loop {
        let id = {
            let mut guard = manager.lock().await;
            if guard.queue.queue.paused {
                break;
            }
            let Some(transfer) = guard.queue.next_eligible() else {
                break;
            };
            let id = transfer.id;
            if guard.queue.mark_active(id).is_err() {
                break;
            }
            let active = guard.queue.get(id).ok();
            let queue = guard.queue.queue.clone();
            (id, active, queue)
        };
        let (id, active, queue) = id;
        if let Some(active) = active {
            emit_state(&app, &active, None);
        }
        emit_queue_state(&app, &queue);
        spawn_worker(
            app.clone(),
            Arc::clone(&connections),
            Arc::clone(&manager),
            id,
        );
    }
}

fn spawn_worker(
    app: AppHandle,
    connections: Arc<tokio::sync::Mutex<ConnectionService>>,
    manager: Arc<tokio::sync::Mutex<TransferManager>>,
    id: Uuid,
) {
    let task_manager = Arc::clone(&manager);
    let dispatch_app = app.clone();
    let dispatch_connections = Arc::clone(&connections);
    let dispatch_manager = Arc::clone(&manager);
    drop(tauri::async_runtime::spawn(async move {
        if let Err(error) = run_worker(
            app.clone(),
            Arc::clone(&connections),
            Arc::clone(&manager),
            id,
        )
        .await
        {
            let mut manager = task_manager.lock().await;
            if let Ok(mut transfer) = manager.queue.get(id) {
                transfer.state = TransferState::Failed;
                transfer.error = Some(error.to_string());
                let _ = manager.queue.replace(transfer.clone());
                emit_state(&app, &transfer, Some(DesktopError::from(error)));
            }
        }
        dispatch_available(dispatch_app, dispatch_connections, dispatch_manager).await;
    }));
}

async fn run_worker(
    app: AppHandle,
    connections: Arc<tokio::sync::Mutex<ConnectionService>>,
    manager: Arc<tokio::sync::Mutex<TransferManager>>,
    id: Uuid,
) -> CoreResult<()> {
    let (mut transfer, control, local_path, bandwidth) = {
        let mut manager = manager.lock().await;
        let transfer = manager.queue.get(id)?;
        if !matches!(
            transfer.state,
            TransferState::Pending
                | TransferState::Active
                | TransferState::Paused
                | TransferState::Interrupted
                | TransferState::Failed
        ) {
            return Ok(());
        }
        if transfer.state == TransferState::Pending {
            manager.queue.mark_active(id)?;
        }
        let control = manager.control(id)?;
        let local_path = manager
            .local_paths
            .get(&id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        (transfer, control, local_path, manager.bandwidth_limiter())
    };
    let mut source_modified_at = if transfer.direction == TransferDirection::Upload {
        std::fs::metadata(&local_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .map(DateTime::<Utc>::from)
    } else {
        None
    };

    let connection_id = {
        let service = connections.lock().await;
        service
            .active_for_profile(transfer.connection_profile_id)
            .ok_or_else(|| {
                CoreError::NotFound("no active connection for this profile".to_owned())
            })?
    };
    let shared = {
        let service = connections.lock().await;
        service.session(connection_id)?
    };
    if transfer.direction == TransferDirection::Download {
        let mut remote = shared.lock().await;
        source_modified_at = remote
            .stat(&transfer.source_path)
            .await
            .ok()
            .and_then(|entry| entry.modified_at);
    }
    let total = transfer.total_bytes;
    let started = Instant::now();
    let transfer_id = transfer.id.to_string();
    let observer_app = app.clone();
    let last_bytes = Arc::new(AtomicU64::new(0));
    let live_progress = Arc::new(AtomicU64::new(transfer.transferred_bytes));
    let last_emit = Arc::new(Mutex::new(Instant::now()));
    let observer_bandwidth = Arc::clone(&bandwidth);
    let observer_progress = Arc::clone(&live_progress);
    let observer: ProgressObserver = Arc::new(move |transferred| {
        observer_progress.store(transferred, Ordering::Relaxed);
        let previous = last_bytes.swap(transferred, Ordering::Relaxed);
        let delta = transferred.saturating_sub(previous);
        if let Ok(mut limiter) = observer_bandwidth.lock() {
            limiter.throttle(delta);
        }
        let should_emit = {
            let mut last = last_emit.lock().expect("progress clock lock");
            if last.elapsed() >= Duration::from_millis(100) || delta >= 1024 * 1024 {
                *last = Instant::now();
                true
            } else {
                false
            }
        };
        if !should_emit {
            return;
        }
        let elapsed = started.elapsed().as_secs_f64();
        let speed = (elapsed > 0.0).then(|| (transferred as f64 / elapsed).round() as u64);
        let eta = total.and_then(|total| {
            speed
                .filter(|speed| *speed > 0)
                .map(|speed| total.saturating_sub(transferred).div_ceil(speed))
        });
        let _ = observer_app.emit(
            "transfer:progress",
            TransferProgressPayload {
                transfer_id: transfer_id.clone(),
                transferred_bytes: transferred,
                total_bytes: total,
                speed_bytes_per_second: speed,
                estimated_remaining_seconds: eta,
            },
        );
    });

    let checkpoint_manager = Arc::clone(&manager);
    let checkpoint_progress = Arc::clone(&live_progress);
    let checkpoint_local_path = local_path.clone();
    let checkpoint_direction = transfer.direction;
    drop(tauri::async_runtime::spawn(async move {
        let mut last_persist = Instant::now();
        let mut last_persisted_bytes = checkpoint_progress.load(Ordering::Relaxed);
        loop {
            tokio::time::sleep(Duration::from_millis(500)).await;
            let mut guard = checkpoint_manager.lock().await;
            let Ok(mut current) = guard.queue.get(id) else {
                break;
            };
            if current.state != TransferState::Active {
                break;
            }
            let progress = checkpoint_progress.load(Ordering::Relaxed);
            if last_persist.elapsed() >= Duration::from_secs(5)
                || progress.saturating_sub(last_persisted_bytes) >= 1024 * 1024
            {
                current.transferred_bytes = progress;
                let snapshot = current.clone();
                let _ = guard.queue.replace(current);
                let anchor = (checkpoint_direction == TransferDirection::Download)
                    .then(|| checkpoint_local_path.clone());
                guard.remember_partial(&snapshot, anchor, source_modified_at);
                let _ = guard.persist();
                last_persist = Instant::now();
                last_persisted_bytes = progress;
            }
        }
    }));

    let download_partial = local_partial_path(&local_path);
    let upload_partial = format!("{}.thriftp-part", transfer.destination_path);
    let result = {
        let mut remote = shared.lock().await;
        let mut result = match transfer.direction {
            TransferDirection::Download => {
                execute_download(
                    remote.as_mut(),
                    &mut transfer,
                    &download_partial,
                    &control,
                    Some(observer),
                )
                .await
            }
            TransferDirection::Upload => {
                execute_upload_to(
                    remote.as_mut(),
                    &mut transfer,
                    &local_path,
                    &upload_partial,
                    &control,
                    Some(observer),
                )
                .await
            }
        };
        if result.is_ok() && transfer.state == TransferState::Completed {
            let finalize = match transfer.direction {
                TransferDirection::Download => {
                    if local_path.exists() {
                        let _ = std::fs::remove_file(&local_path);
                    }
                    std::fs::rename(&download_partial, &local_path).map_err(|error| {
                        CoreError::internal(format!("could not finalize downloaded file: {error}"))
                    })
                }
                TransferDirection::Upload => {
                    remote
                        .rename(&upload_partial, &transfer.destination_path)
                        .await
                }
            };
            if let Err(error) = finalize {
                transfer.state = TransferState::Failed;
                transfer.error = Some(error.to_string());
                result = Err(error);
            }
        }
        result
    };

    let mut manager = manager.lock().await;
    match result {
        Ok(run) => {
            transfer.transferred_bytes = run.transferred_bytes;
            if transfer.state == TransferState::Completed {
                manager.clear_partial(transfer.id);
                if transfer.direction == TransferDirection::Download {
                    let _ = remove_artifacts(&local_path);
                }
            } else {
                let partial_anchor =
                    (transfer.direction == TransferDirection::Download).then(|| local_path.clone());
                manager.remember_partial(&transfer, partial_anchor, source_modified_at);
            }
            manager.record_terminal(&transfer);
            manager.queue.replace(transfer.clone())?;
            let _ = manager.persist();
            emit_state(&app, &transfer, None);
        }
        Err(error) => {
            if transfer.direction == TransferDirection::Upload
                && transfer.error.is_none()
                && transfer.transferred_bytes > 0
            {
                transfer.error = Some(format!("remote partial file retained at {upload_partial}"));
            }
            let partial_anchor =
                (transfer.direction == TransferDirection::Download).then(|| local_path.clone());
            manager.remember_partial(&transfer, partial_anchor, source_modified_at);
            manager.record_terminal(&transfer);
            manager.queue.replace(transfer.clone())?;
            let _ = manager.persist();
            emit_state(&app, &transfer, Some(DesktopError::from(error)));
        }
    }
    emit_queue_state(&app, &manager.queue.queue);
    Ok(())
}

async fn delegate_value(
    state: &AppState,
    method: &str,
    params: Value,
) -> Result<Option<Value>, DesktopError> {
    let mut guard = state.transfer_host.lock().await;
    let Some(runtime) = guard.as_mut() else {
        return Ok(None);
    };
    if runtime.is_host() {
        return Ok(None);
    }
    let client = runtime.client().ok_or_else(|| {
        DesktopError::from(CoreError::HostUnavailable(
            "control channel unavailable".to_owned(),
        ))
    })?;
    client
        .call(method, params)
        .map(Some)
        .map_err(DesktopError::from)
}

fn parse_delegated<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, DesktopError> {
    serde_json::from_value(value).map_err(|error| {
        DesktopError::from(CoreError::internal(format!(
            "invalid host response: {error}"
        )))
    })
}

#[tauri::command]
pub async fn transfers_start(
    app: AppHandle,
    state: State<'_, AppState>,
    direction: String,
    profile_id: String,
    sources: Vec<String>,
    destination: String,
    conflict_policy: Option<String>,
) -> Result<StartTransfersView, DesktopError> {
    if sources.is_empty() {
        return Err(DesktopError::from(CoreError::validation(
            "at least one source is required",
        )));
    }
    if let Some(value) = delegate_value(
        &state,
        "transfers.submit",
        json!({
            "direction": direction.clone(),
            "profileId": profile_id.clone(),
            "sources": sources,
            "destination": destination,
            "recursive": true,
            "conflictPolicy": conflict_policy.clone().unwrap_or_else(|| "fail".to_owned()),
            "priority": "normal",
            "execute": true
        }),
    )
    .await?
    {
        let ids = value
            .get("transfers")
            .and_then(Value::as_array)
            .map(|transfers| {
                transfers
                    .iter()
                    .filter_map(|transfer| {
                        transfer
                            .get("id")
                            .and_then(Value::as_str)
                            .map(str::to_owned)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        return Ok(StartTransfersView {
            transfer_ids: ids,
            skipped_links: Vec::new(),
        });
    }
    let profile_id = parse_id(&profile_id).map_err(DesktopError::from)?;
    let direction = match direction.as_str() {
        "upload" => TransferDirection::Upload,
        "download" => TransferDirection::Download,
        other => {
            return Err(DesktopError::from(CoreError::validation(format!(
                "unknown transfer direction: {other}"
            ))));
        }
    };
    let conflict_policy = match conflict_policy.as_deref().unwrap_or("ask") {
        "ask" => ConflictPolicy::Ask,
        "overwrite" => ConflictPolicy::Overwrite,
        "skip" => ConflictPolicy::Skip,
        "rename" => ConflictPolicy::Rename,
        other => {
            return Err(DesktopError::from(CoreError::validation(format!(
                "unknown conflict policy: {other}"
            ))));
        }
    };
    let connection_id = {
        let service = state.connections.lock().await;
        service.active_for_profile(profile_id).ok_or_else(|| {
            DesktopError::from(CoreError::NotFound(
                "connect to the server before transferring files".to_owned(),
            ))
        })?
    };
    let shared = {
        let service = state.connections.lock().await;
        service.session(connection_id).map_err(DesktopError::from)?
    };

    // Keep the lock order consistent with transfer workers (session, then
    // manager) so a running transfer cannot deadlock a new start request.
    let mut remote = shared.lock().await;
    let mut manager = state.transfers.lock().await;
    let before: HashSet<Uuid> = manager.queue.ids().into_iter().collect();
    let mut ids = Vec::new();
    let mut skipped_links = Vec::new();
    for source in &sources {
        let result = match direction {
            TransferDirection::Upload => {
                enqueue_upload(
                    &mut manager,
                    remote.as_mut(),
                    profile_id,
                    Path::new(source),
                    &destination,
                    conflict_policy,
                )
                .await
            }
            TransferDirection::Download => {
                enqueue_download(
                    &mut manager,
                    remote.as_mut(),
                    profile_id,
                    source,
                    Path::new(&destination),
                    conflict_policy,
                )
                .await
            }
        };
        match result {
            Ok((new_ids, links)) => {
                ids.extend(new_ids);
                skipped_links.extend(links);
            }
            Err(error) => {
                for queued in manager.queue.ids() {
                    if !before.contains(&queued) {
                        manager.queue.discard(queued);
                    }
                }
                let _ = manager.persist();
                drop(remote);
                return Err(DesktopError::from(error));
            }
        }
    }
    let paused = manager.queue.queue.paused;
    drop(remote);
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    drop(manager);

    if !paused {
        dispatch_available(
            app,
            Arc::clone(&state.connections),
            Arc::clone(&state.transfers),
        )
        .await;
    }

    Ok(StartTransfersView {
        transfer_ids: ids.iter().map(ToString::to_string).collect(),
        skipped_links,
    })
}

#[tauri::command]
pub async fn transfers_list(state: State<'_, AppState>) -> Result<Vec<Transfer>, DesktopError> {
    if let Some(value) = delegate_value(&state, "transfers.list", json!({"limit": 1_000})).await? {
        return Ok(serde_json::from_value(
            value.get("transfers").cloned().unwrap_or_else(|| json!([])),
        )
        .unwrap_or_default());
    }
    let manager = state.transfers.lock().await;
    Ok(manager.queue.list())
}

#[tauri::command]
pub async fn transfers_get(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.status",
        json!({"transferId": transfer_id}),
    )
    .await?
    {
        return serde_json::from_value(value).map_err(|error| {
            DesktopError::from(CoreError::internal(format!(
                "invalid host transfer response: {error}"
            )))
        });
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let manager = state.transfers.lock().await;
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_pause(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.pause",
        json!({"transferId": transfer_id}),
    )
    .await?
    {
        return serde_json::from_value(value).map_err(|error| {
            DesktopError::from(CoreError::internal(format!(
                "invalid host transfer response: {error}"
            )))
        });
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let mut manager = state.transfers.lock().await;
    let transfer = manager.queue.get(id).map_err(DesktopError::from)?;
    match transfer.state {
        TransferState::Active => manager.control(id).map_err(DesktopError::from)?.pause(),
        TransferState::Pending => manager
            .queue
            .mark_state(id, TransferState::Paused)
            .map_err(DesktopError::from)?,
        _ => {}
    }
    let _ = manager.persist();
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_resume(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.resume",
        json!({"transferId": transfer_id.clone()}),
    )
    .await?
    {
        return serde_json::from_value(value).map_err(|error| {
            DesktopError::from(CoreError::internal(format!(
                "invalid host transfer response: {error}"
            )))
        });
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let (transfer, local_path, partial) = {
        let manager = state.transfers.lock().await;
        let transfer = manager.queue.get(id).map_err(DesktopError::from)?;
        let local_path = manager
            .local_paths
            .get(&id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        let partial = manager.partial(id);
        (transfer, local_path, partial)
    };
    if matches!(
        transfer.state,
        TransferState::Paused | TransferState::Failed | TransferState::Interrupted
    ) && transfer.transferred_bytes > 0
    {
        let connection_id = {
            let service = state.connections.lock().await;
            service.active_for_profile(transfer.connection_profile_id)
        }
        .ok_or_else(|| {
            DesktopError::from(CoreError::NotFound(
                "connect to the server before resuming transfers".to_owned(),
            ))
        })?;
        let shared = {
            let service = state.connections.lock().await;
            service.session(connection_id).map_err(DesktopError::from)?
        };
        let mut remote = shared.lock().await;
        verify_resume_state(remote.as_mut(), &transfer, &local_path, partial.as_ref())
            .await
            .map_err(DesktopError::from)?;
    }
    {
        let mut manager = state.transfers.lock().await;
        let transfer = manager.queue.get(id).map_err(DesktopError::from)?;
        if matches!(
            transfer.state,
            TransferState::Paused | TransferState::Failed | TransferState::Interrupted
        ) {
            manager.control(id).map_err(DesktopError::from)?.resume();
            manager
                .queue
                .mark_state(id, TransferState::Pending)
                .map_err(DesktopError::from)?;
        }
    }
    dispatch_available(
        app,
        Arc::clone(&state.connections),
        Arc::clone(&state.transfers),
    )
    .await;
    let manager = state.transfers.lock().await;
    let _ = manager.persist();
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_cancel(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.cancel",
        json!({"transferId": transfer_id.clone()}),
    )
    .await?
    {
        return serde_json::from_value(value).map_err(|error| {
            DesktopError::from(CoreError::internal(format!(
                "invalid host transfer response: {error}"
            )))
        });
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let mut manager = state.transfers.lock().await;
    let transfer = manager.queue.get(id).map_err(DesktopError::from)?;
    match transfer.state {
        TransferState::Pending
        | TransferState::Paused
        | TransferState::Failed
        | TransferState::Interrupted => {
            manager
                .queue
                .mark_state(id, TransferState::Cancelled)
                .map_err(DesktopError::from)?;
            if let Ok(cancelled) = manager.queue.get(id) {
                manager.record_terminal(&cancelled);
            }
        }
        TransferState::Active => {
            manager.control(id).map_err(DesktopError::from)?.cancel();
        }
        _ => {}
    }
    let _ = manager.persist();
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_restart(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.resume",
        json!({"transferId": transfer_id.clone()}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    {
        let mut manager = state.transfers.lock().await;
        let mut transfer = manager.queue.get(id).map_err(DesktopError::from)?;
        manager.control(id).map_err(DesktopError::from)?.reset();
        transfer.transferred_bytes = 0;
        transfer.state = TransferState::Pending;
        transfer.error = None;
        transfer.completed_at = None;
        transfer.resumable = false;
        manager
            .queue
            .replace(transfer)
            .map_err(DesktopError::from)?;
    }
    dispatch_available(
        app,
        Arc::clone(&state.connections),
        Arc::clone(&state.transfers),
    )
    .await;
    let manager = state.transfers.lock().await;
    let _ = manager.persist();
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_discard_partial(
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<Transfer, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.cancel",
        json!({"transferId": transfer_id.clone()}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let transfer = {
        let manager = state.transfers.lock().await;
        let transfer = manager.queue.get(id).map_err(DesktopError::from)?;
        manager.control(id).map_err(DesktopError::from)?.cancel();
        transfer
    };
    if transfer.direction == TransferDirection::Download {
        remove_artifacts(Path::new(&transfer.destination_path)).map_err(DesktopError::from)?;
    } else if let Some(connection_id) = {
        let service = state.connections.lock().await;
        service.active_for_profile(transfer.connection_profile_id)
    } {
        let shared = {
            let service = state.connections.lock().await;
            service.session(connection_id).ok()
        };
        if let Some(shared) = shared {
            let mut remote = shared.lock().await;
            let _ = remote
                .delete_file(&format!("{}.thriftp-part", transfer.destination_path))
                .await;
        }
    }
    let mut manager = state.transfers.lock().await;
    manager.clear_partial(id);
    manager
        .queue
        .mark_state(id, TransferState::Cancelled)
        .map_err(DesktopError::from)?;
    let _ = manager.persist();
    manager.queue.get(id).map_err(DesktopError::from)
}

#[tauri::command]
pub async fn transfers_clear_history(state: State<'_, AppState>) -> Result<usize, DesktopError> {
    if let Some(value) = delegate_value(&state, "transfers.clearHistory", json!({})).await? {
        let cleared = value.get("cleared").and_then(Value::as_u64).unwrap_or(0);
        return Ok(cleared as usize);
    }
    let mut manager = state.transfers.lock().await;
    let cleared = manager.clear_history();
    let _ = manager.persist();
    Ok(cleared)
}

#[tauri::command]
pub async fn transfers_dismiss(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
) -> Result<TransferQueue, DesktopError> {
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let mut manager = state.transfers.lock().await;
    manager.queue.remove(id).map_err(DesktopError::from)?;
    let _ = manager.persist();
    let queue = manager.queue.queue.clone();
    emit_queue_state(&app, &queue);
    Ok(queue)
}

#[tauri::command]
pub async fn queue_get(state: State<'_, AppState>) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(&state, "queue.get", json!({})).await? {
        return serde_json::from_value(value).map_err(|error| {
            DesktopError::from(CoreError::internal(format!(
                "invalid host queue response: {error}"
            )))
        });
    }
    let manager = state.transfers.lock().await;
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_pause(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(&state, "queue.pause", json!({})).await? {
        return parse_delegated(value);
    }
    let mut manager = state.transfers.lock().await;
    manager.queue.pause();
    for control in manager.controls.values() {
        control.pause();
    }
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_resume(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(&state, "queue.resume", json!({})).await? {
        return parse_delegated(value);
    }
    {
        let mut manager = state.transfers.lock().await;
        manager.queue.resume();
        for transfer in manager.queue.list() {
            if transfer.state == TransferState::Paused
                && let Ok(control) = manager.control(transfer.id)
            {
                control.resume();
                let _ = manager
                    .queue
                    .mark_state(transfer.id, TransferState::Pending);
            }
        }
        emit_queue_state(&app, &manager.queue.queue);
        let _ = manager.persist();
    }
    dispatch_available(
        app,
        Arc::clone(&state.connections),
        Arc::clone(&state.transfers),
    )
    .await;
    let manager = state.transfers.lock().await;
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_set_concurrency(
    app: AppHandle,
    state: State<'_, AppState>,
    limit: usize,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "queue.setConcurrency",
        json!({"concurrency": limit}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let queue = {
        let mut manager = state.transfers.lock().await;
        manager.queue.set_concurrency(limit);
        let _ = manager.persist();
        emit_queue_state(&app, &manager.queue.queue);
        manager.queue.queue.clone()
    };
    dispatch_available(
        app,
        Arc::clone(&state.connections),
        Arc::clone(&state.transfers),
    )
    .await;
    Ok(queue)
}

#[tauri::command]
pub async fn queue_set_bandwidth(
    app: AppHandle,
    state: State<'_, AppState>,
    bytes_per_second: Option<u64>,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "queue.setBandwidth",
        json!({"bytesPerSecond": bytes_per_second}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let mut manager = state.transfers.lock().await;
    manager.set_bandwidth(bytes_per_second);
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_reorder(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_ids: Vec<String>,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.reorder",
        json!({"transferIds": transfer_ids}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let ids: Vec<Uuid> = transfer_ids
        .iter()
        .map(|id| parse_id(id))
        .collect::<CoreResult<_>>()
        .map_err(DesktopError::from)?;
    let mut manager = state.transfers.lock().await;
    manager.queue.reorder(&ids).map_err(DesktopError::from)?;
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_set_priority(
    app: AppHandle,
    state: State<'_, AppState>,
    transfer_id: String,
    priority: String,
) -> Result<TransferQueue, DesktopError> {
    if let Some(value) = delegate_value(
        &state,
        "transfers.priority",
        json!({"transferId": transfer_id, "priority": priority}),
    )
    .await?
    {
        return parse_delegated(value);
    }
    let id = parse_id(&transfer_id).map_err(DesktopError::from)?;
    let priority = match priority.as_str() {
        "low" => Priority::Low,
        "normal" => Priority::Normal,
        "high" => Priority::High,
        other => {
            return Err(DesktopError::from(CoreError::validation(format!(
                "unknown priority: {other}"
            ))));
        }
    };
    let mut manager = state.transfers.lock().await;
    manager
        .queue
        .set_priority(id, priority)
        .map_err(DesktopError::from)?;
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    Ok(manager.queue.queue.clone())
}

#[tauri::command]
pub async fn queue_cancel_all(
    app: AppHandle,
    state: State<'_, AppState>,
    confirmed: bool,
) -> Result<TransferQueue, DesktopError> {
    if confirmed
        && let Some(value) =
            delegate_value(&state, "queue.cancelAll", json!({"confirmed": true})).await?
    {
        return parse_delegated(value);
    }
    if !confirmed {
        return Err(DesktopError::from(CoreError::Conflict(
            "cancelling all transfers requires confirmation".to_owned(),
        )));
    }
    let mut manager = state.transfers.lock().await;
    let controls: Vec<_> = manager
        .controls
        .iter()
        .map(|(id, control)| (*id, control.clone()))
        .collect();
    for (id, control) in controls {
        control.cancel();
        let _ = manager.queue.mark_state(id, TransferState::Cancelled);
    }
    manager.queue.cancel_all_pending();
    let cancelled: Vec<Transfer> = manager
        .queue
        .list()
        .into_iter()
        .filter(|transfer| transfer.state == TransferState::Cancelled)
        .collect();
    for transfer in &cancelled {
        manager.record_terminal(transfer);
    }
    let _ = manager.persist();
    emit_queue_state(&app, &manager.queue.queue);
    Ok(manager.queue.queue.clone())
}
