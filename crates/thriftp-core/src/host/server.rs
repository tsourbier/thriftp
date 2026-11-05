//! Local-only host control server.
//!
//! Unix domain sockets are used on macOS/Linux. Windows falls back to a
//! loopback-only TCP endpoint because the project does not depend on a named
//! pipe crate. Both transports require the per-instance token and accept only
//! newline-delimited JSON-RPC 2.0.

use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::lock::{HostLock, cleanup_runtime_files, new_status, read_status};
use super::model::{HostSnapshot, HostStatus};
use super::protocol::{RpcError, RpcRequest, RpcResponse, codes, constant_time_eq};
use crate::automation::hex;
use crate::credentials::CredentialStore;
use crate::error::{CoreError, CoreResult};
use crate::model::CredentialRef;
use crate::model::{
    ConnectionProfile, EntryType, Priority, Transfer, TransferDirection, TransferState,
};
use crate::protocol::RemoteFs;
use crate::repository::{MetadataStore, TransferState as PersistedTransfers, TransferStore};
use crate::transfers::TransferControl;
use crate::transfers::engine::{ProgressObserver, execute_download, execute_upload};

/// Services injected by the adapter that starts the host.
pub struct HostServices {
    pub metadata: Arc<dyn MetadataStore>,
    pub transfers: Arc<dyn TransferStore>,
    pub credentials: Arc<dyn CredentialStore>,
    pub factory: Arc<dyn crate::protocol::RemoteFsFactory>,
}

#[cfg(unix)]
pub type ControlStream = std::os::unix::net::UnixStream;
#[cfg(windows)]
pub type ControlStream = std::net::TcpStream;

pub struct HostServer {
    status: HostStatus,
    token: String,
    shutdown: Arc<AtomicBool>,
    subscribers: Arc<Mutex<Vec<ControlStream>>>,
    handle: Option<JoinHandle<()>>,
    run_dir: PathBuf,
}

enum BoundListener {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixListener),
    #[cfg(windows)]
    Tcp(std::net::TcpListener),
}

struct ServerState {
    token: String,
    services: HostServices,
    shutdown: Arc<AtomicBool>,
    subscribers: Arc<Mutex<Vec<ControlStream>>>,
    status: HostStatus,
}

fn broadcast_to(subscribers: &Mutex<Vec<ControlStream>>, method: &str, params: Value) {
    let message = json!({"jsonrpc": "2.0", "method": method, "params": params}).to_string();
    if let Ok(mut subscribers) = subscribers.lock() {
        subscribers.retain_mut(|stream| {
            writeln!(stream, "{message}")
                .and_then(|()| stream.flush())
                .is_ok()
        });
    }
}

impl ServerState {
    fn broadcast(&self, method: &str, params: Value) {
        broadcast_to(&self.subscribers, method, params);
    }
}

impl HostServer {
    /// Bind the local endpoint and publish host metadata and the access token.
    pub fn start(lock: &HostLock, mode: &str, services: HostServices) -> CoreResult<Self> {
        let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
        let (endpoint, transport, listener) = bind_endpoint(lock)?;
        let mut status = new_status(mode);
        status.endpoint = Some(endpoint);
        status.transport = transport.to_owned();
        status.token_hash = hex(&Sha256::digest(token.as_bytes()));
        lock.publish(&status)?;
        lock.publish_token(&token)?;

        let shutdown = Arc::new(AtomicBool::new(false));
        let subscribers = Arc::new(Mutex::new(Vec::new()));
        let state = Arc::new(ServerState {
            token: token.clone(),
            services,
            shutdown: Arc::clone(&shutdown),
            subscribers: Arc::clone(&subscribers),
            status: status.clone(),
        });
        let handle = std::thread::Builder::new()
            .name("thriftp-host".to_owned())
            .spawn(move || accept_loop(state, listener))
            .map_err(|error| {
                CoreError::internal(format!("failed to start transfer host: {error}"))
            })?;
        Ok(Self {
            status,
            token,
            shutdown,
            subscribers,
            handle: Some(handle),
            run_dir: lock.run_dir().to_path_buf(),
        })
    }

    #[must_use]
    pub fn status(&self) -> &HostStatus {
        &self.status
    }

    #[must_use]
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Notify control clients and stop the accept loop.
    pub fn shutdown(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let message = json!({
            "jsonrpc": "2.0",
            "method": "event.hostShutdown",
            "params": {"reason": "clean shutdown", "expectedInterruptionMs": 0}
        })
        .to_string();
        if let Ok(mut subscribers) = self.subscribers.lock() {
            for stream in subscribers.iter_mut() {
                let _ = writeln!(stream, "{message}");
                let _ = stream.flush();
            }
            subscribers.clear();
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        cleanup_runtime_files(&self.run_dir);
    }
}

impl Drop for HostServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

#[cfg(unix)]
fn bind_endpoint(lock: &HostLock) -> CoreResult<(String, &'static str, BoundListener)> {
    let path = lock.endpoint_path();
    let _ = fs::remove_file(&path);
    let listener = std::os::unix::net::UnixListener::bind(&path).map_err(|error| {
        CoreError::HostUnavailable(format!("failed to bind control socket: {error}"))
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| CoreError::internal(format!("control socket setup: {error}")))?;
    let endpoint = path.to_string_lossy().into_owned();
    Ok((endpoint, "unixSocket", BoundListener::Unix(listener)))
}

#[cfg(windows)]
fn bind_endpoint(_lock: &HostLock) -> CoreResult<(String, &'static str, BoundListener)> {
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).map_err(|error| {
        CoreError::HostUnavailable(format!("failed to bind control port: {error}"))
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| CoreError::internal(format!("control listener setup: {error}")))?;
    let endpoint = listener
        .local_addr()
        .map_err(|error| CoreError::internal(format!("control address: {error}")))?
        .to_string();
    Ok((endpoint, "tcpLoopback", BoundListener::Tcp(listener)))
}

fn accept_loop(state: Arc<ServerState>, listener: BoundListener) {
    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            break;
        }
        #[cfg(unix)]
        let accepted = match &listener {
            BoundListener::Unix(listener) => listener.accept().map(|(stream, _)| stream),
        };
        #[cfg(windows)]
        let accepted = match &listener {
            BoundListener::Tcp(listener) => listener.accept().map(|(stream, _)| stream),
        };
        match accepted {
            Ok(stream) => {
                let state = Arc::clone(&state);
                let _ = std::thread::Builder::new()
                    .name("thriftp-host-conn".to_owned())
                    .spawn(move || serve_connection(state, stream));
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => break,
        }
    }
}

fn serve_connection(state: Arc<ServerState>, mut stream: ControlStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let reader_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(_) => return,
    };
    let mut reader = BufReader::new(reader_stream);
    loop {
        if state.shutdown.load(Ordering::SeqCst) {
            break;
        }
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(error)
                if error.kind() == std::io::ErrorKind::WouldBlock
                    || error.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => break,
        }
        if line.trim().is_empty() {
            continue;
        }
        let response = match RpcRequest::parse(line.trim()) {
            Ok(request) => dispatch(&state, &request, &stream),
            Err(error) => RpcResponse::failure(
                None,
                RpcError {
                    code: codes::PARSE_ERROR,
                    message: crate::observability::redact_text(&error.to_string()),
                    data: Some(json!({"code": error.code()})),
                },
            ),
        };
        let Ok(encoded) = response.encode() else {
            break;
        };
        if writeln!(stream, "{encoded}").is_err() || stream.flush().is_err() {
            break;
        }
    }
}

fn dispatch(state: &ServerState, request: &RpcRequest, stream: &ControlStream) -> RpcResponse {
    let Some(auth) = request.auth.as_deref() else {
        return unauthorized(request, "missing host token");
    };
    if !constant_time_eq(auth, &state.token) {
        return unauthorized(request, "invalid host token");
    }
    let result = match request.method.as_str() {
        "host.status" => Ok(host_snapshot(state)),
        "transfers.list" => list_transfers(state, &request.params),
        "transfers.status" => transfer_by_id(state, &request.params),
        "transfers.submit" => submit(state, &request.params),
        "transfers.pause" | "transfers.resume" | "transfers.cancel" => {
            set_transfer_state(state, &request.params, &request.method)
        }
        "transfers.priority" => set_priority(state, &request.params),
        "transfers.reorder" => reorder(state, &request.params),
        "transfers.clearHistory" => clear_history(state),
        "queue.get" => queue_get(state),
        "queue.setConcurrency" => set_concurrency(state, &request.params),
        "queue.setBandwidth" => set_bandwidth(state, &request.params),
        "queue.pause" => set_queue_paused(state, true),
        "queue.resume" => set_queue_paused(state, false),
        "queue.cancelAll" => cancel_all(state, &request.params),
        "events.subscribe" => {
            let topics = request
                .params
                .get("topics")
                .cloned()
                .unwrap_or_else(|| json!([]));
            if let Ok(mut subscribers) = state.subscribers.lock()
                && let Ok(clone) = stream.try_clone()
            {
                subscribers.push(clone);
            }
            Ok(json!({"subscribed": true, "topics": topics}))
        }
        _ => Err(CoreError::UnsupportedCapability(format!(
            "unknown method {}",
            request.method
        ))),
    };
    match result {
        Ok(value) => RpcResponse::success(request.id.clone(), value),
        Err(error) => RpcResponse::failure(request.id.clone(), RpcError::from_core(&error)),
    }
}

fn unauthorized(request: &RpcRequest, message: &str) -> RpcResponse {
    RpcResponse::failure(
        request.id.clone(),
        RpcError {
            code: codes::UNAUTHORIZED,
            message: message.to_owned(),
            data: Some(json!({"code": "permission_denied"})),
        },
    )
}

fn host_snapshot(state: &ServerState) -> Value {
    let owned = state
        .services
        .transfers
        .load()
        .map(|transfers| {
            transfers
                .transfers
                .iter()
                .filter(|transfer| transfer.state == TransferState::Active)
                .count()
        })
        .unwrap_or(0);
    json!(HostSnapshot {
        instance_id: state.status.instance_id,
        pid: state.status.pid,
        started_at: state.status.started_at,
        version: state.status.version.clone(),
        mode: state.status.mode.clone(),
        role: "host".to_owned(),
        owned_transfer_count: owned,
    })
}

fn load_transfers(state: &ServerState) -> CoreResult<PersistedTransfers> {
    state.services.transfers.load()
}

fn save_transfers(state: &ServerState, transfers: &PersistedTransfers) -> CoreResult<()> {
    state.services.transfers.save(transfers)
}

fn list_transfers(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let transfers = load_transfers(state)?;
    let filter = params.get("state").and_then(Value::as_str);
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(100)
        .min(1_000) as usize;
    let mut items: Vec<_> = transfers
        .transfers
        .into_iter()
        .filter(|transfer| {
            filter.is_none_or(|value| format!("{:?}", transfer.state).eq_ignore_ascii_case(value))
        })
        .collect();
    items.truncate(limit);
    Ok(json!({"transfers": items, "nextCursor": null}))
}

fn transfer_by_id(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let id = parse_uuid(params.get("transferId"))
        .ok_or_else(|| CoreError::invalid_argument("transferId is required"))?;
    let transfer = load_transfers(state)?
        .transfers
        .into_iter()
        .find(|transfer| transfer.id == id)
        .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
    Ok(json!(transfer))
}

fn set_transfer_state(state: &ServerState, params: &Value, method: &str) -> CoreResult<Value> {
    let id = parse_uuid(params.get("transferId"))
        .ok_or_else(|| CoreError::invalid_argument("transferId is required"))?;
    let mut transfers = load_transfers(state)?;
    let next = match method {
        "transfers.pause" => TransferState::Paused,
        "transfers.resume" => TransferState::Pending,
        _ => TransferState::Cancelled,
    };
    let transfer = transfers
        .transfers
        .iter_mut()
        .find(|transfer| transfer.id == id)
        .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
    transfer.state = next;
    transfer.updated_at = chrono::Utc::now();
    let view = transfer.clone();
    save_transfers(state, &transfers)?;
    Ok(json!(view))
}

fn set_priority(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let id = parse_uuid(params.get("transferId"))
        .ok_or_else(|| CoreError::invalid_argument("transferId is required"))?;
    let priority = params
        .get("priority")
        .and_then(Value::as_str)
        .unwrap_or("normal");
    let priority = parse_priority(priority);
    let mut transfers = load_transfers(state)?;
    let transfer = transfers
        .transfers
        .iter_mut()
        .find(|transfer| transfer.id == id)
        .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
    transfer.priority = priority;
    let view = transfer.clone();
    save_transfers(state, &transfers)?;
    Ok(json!(view))
}

fn reorder(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let ids = params
        .get("transferIds")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::invalid_argument("transferIds is required"))?;
    let mut transfers = load_transfers(state)?;
    transfers.queue.ordered_transfer_ids = ids
        .iter()
        .filter_map(Value::as_str)
        .filter_map(|value| Uuid::parse_str(value).ok())
        .collect();
    let order = transfers.queue.ordered_transfer_ids.clone();
    save_transfers(state, &transfers)?;
    Ok(json!({"orderedTransferIds": order}))
}

fn clear_history(state: &ServerState) -> CoreResult<Value> {
    let mut transfers = load_transfers(state)?;
    let before = transfers.transfers.len();
    transfers.transfers.retain(|transfer| {
        !matches!(
            transfer.state,
            TransferState::Completed
                | TransferState::Failed
                | TransferState::Cancelled
                | TransferState::Interrupted
        )
    });
    let cleared = before.saturating_sub(transfers.transfers.len());
    save_transfers(state, &transfers)?;
    Ok(json!({"cleared": cleared}))
}

fn queue_get(state: &ServerState) -> CoreResult<Value> {
    Ok(json!(load_transfers(state)?.queue))
}

fn set_concurrency(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let value = params
        .get("concurrency")
        .and_then(Value::as_u64)
        .ok_or_else(|| CoreError::invalid_argument("concurrency is required"))?
        .clamp(1, 64) as usize;
    let mut transfers = load_transfers(state)?;
    transfers.queue.concurrency_limit = value;
    save_transfers(state, &transfers)?;
    Ok(json!(transfers.queue))
}

fn set_bandwidth(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let value = params.get("bytesPerSecond").and_then(Value::as_u64);
    let mut transfers = load_transfers(state)?;
    transfers.queue.bandwidth_limit_bytes_per_second = value;
    save_transfers(state, &transfers)?;
    Ok(json!(transfers.queue))
}

fn set_queue_paused(state: &ServerState, paused: bool) -> CoreResult<Value> {
    let mut transfers = load_transfers(state)?;
    transfers.queue.paused = paused;
    save_transfers(state, &transfers)?;
    Ok(json!(transfers.queue))
}

fn cancel_all(state: &ServerState, params: &Value) -> CoreResult<Value> {
    if params.get("confirmed").and_then(Value::as_bool) != Some(true) {
        return Err(CoreError::ConfirmationRequired);
    }
    let mut transfers = load_transfers(state)?;
    let mut cancelled = Vec::new();
    for transfer in &mut transfers.transfers {
        if transfer.state == TransferState::Pending || transfer.state == TransferState::Active {
            transfer.state = TransferState::Cancelled;
            transfer.updated_at = chrono::Utc::now();
            cancelled.push(transfer.id);
        }
    }
    save_transfers(state, &transfers)?;
    Ok(json!({"cancelledTransferIds": cancelled}))
}

fn submit(state: &ServerState, params: &Value) -> CoreResult<Value> {
    let direction = params
        .get("direction")
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::invalid_argument("direction is required"))?;
    let profile_id = parse_uuid(params.get("profileId"))
        .ok_or_else(|| CoreError::invalid_argument("profileId is required"))?;
    let destination = params
        .get("destination")
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::invalid_argument("destination is required"))?
        .to_owned();
    let sources: Vec<String> = params
        .get("sources")
        .and_then(Value::as_array)
        .ok_or_else(|| CoreError::invalid_argument("sources is required"))?
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let recursive = params
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let conflict = params
        .get("conflictPolicy")
        .and_then(Value::as_str)
        .unwrap_or("fail")
        .to_owned();
    let priority = parse_priority(
        params
            .get("priority")
            .and_then(Value::as_str)
            .unwrap_or("normal"),
    );
    let execute = params
        .get("execute")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let wait = params.get("wait").and_then(Value::as_bool).unwrap_or(true);
    let timeout_ms = params.get("timeoutMs").and_then(Value::as_u64);
    let execute = execute && wait;
    let direction = match direction {
        "upload" => TransferDirection::Upload,
        "download" => TransferDirection::Download,
        other => {
            return Err(CoreError::invalid_argument(format!(
                "unknown direction {other}"
            )));
        }
    };

    let metadata = state.services.metadata.load()?;
    let profile = metadata
        .connection_profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| CoreError::NotFound("profile".to_owned()))?;

    let mut records = Vec::new();
    if !execute {
        for source in &sources {
            let mut transfer = Transfer::new(
                direction,
                profile.id,
                source.clone(),
                destination.clone(),
                None,
            );
            transfer.priority = priority;
            records.push(transfer);
        }
        let mut state_transfers = load_transfers(state)?;
        for transfer in &records {
            state_transfers.transfers.push(transfer.clone());
            state_transfers.history.push(transfer.clone());
        }
        save_transfers(state, &state_transfers)?;
        state.broadcast(
            "event.queueChanged",
            json!({"queue": state_transfers.queue}),
        );
        return Ok(json!({"transfers": records, "state": "pending"}));
    }

    let credential = match state
        .services
        .credentials
        .get(&CredentialRef::for_profile(&profile))
    {
        Ok(secret) => secret,
        Err(crate::credentials::CredentialStoreError::NotFound) => None,
        Err(error) => {
            return Err(CoreError::CredentialStoreUnavailable(error.to_string()));
        }
    };
    let mut remote = state.services.factory.create(profile.protocol);
    block_on(remote.connect(&profile, credential))??;
    let subscribers = Arc::clone(&state.subscribers);
    let last_progress = Arc::new(Mutex::new(Instant::now()));
    let progress: HostProgress = Arc::new(move |transfer_id, bytes| {
        let Ok(mut last) = last_progress.lock() else {
            return;
        };
        if last.elapsed() < Duration::from_millis(250) {
            return;
        }
        *last = Instant::now();
        broadcast_to(
            &subscribers,
            "event.transferProgress",
            json!({"transferId": transfer_id, "confirmedBytes": bytes}),
        );
    });
    let result = if let Some(milliseconds) = timeout_ms {
        block_on(async {
            match tokio::time::timeout(
                Duration::from_millis(milliseconds),
                execute_sources(
                    &mut *remote,
                    &profile,
                    direction,
                    &sources,
                    &destination,
                    recursive,
                    &conflict,
                    priority,
                    &mut records,
                    Some(progress.clone()),
                ),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => Err(CoreError::Timeout("host transfer timed out".to_owned())),
            }
        })?
    } else {
        block_on(execute_sources(
            &mut *remote,
            &profile,
            direction,
            &sources,
            &destination,
            recursive,
            &conflict,
            priority,
            &mut records,
            Some(progress.clone()),
        ))?
    };
    let _ = block_on(remote.disconnect())?;
    if let Err(error) = &result {
        for record in &mut records {
            if record.state != TransferState::Completed {
                record.state = TransferState::Failed;
                record.error = Some(error.to_string());
            }
        }
    }
    let mut state_transfers = load_transfers(state)?;
    for record in &records {
        state_transfers.transfers.push(record.clone());
        state_transfers.history.push(record.clone());
    }
    save_transfers(state, &state_transfers)?;
    for record in &records {
        state.broadcast(
            "event.transferProgress",
            json!({
                "transferId": record.id,
                "confirmedBytes": record.transferred_bytes,
                "totalBytes": record.total_bytes,
                "speedBytesPerSecond": record.speed_bytes_per_second,
                "estimatedRemainingSeconds": record.estimated_remaining_seconds
            }),
        );
        state.broadcast(
            "event.transferState",
            json!({
                "transferId": record.id,
                "newState": record.state,
                "errorCode": record.error
            }),
        );
    }
    state.broadcast(
        "event.queueChanged",
        json!({"queue": state_transfers.queue}),
    );
    result.map(|()| json!({"transfers": records}))?;
    Ok(json!({"transfers": records}))
}

type HostProgress = Arc<dyn Fn(Uuid, u64) + Send + Sync>;

fn transfer_observer(progress: &Option<HostProgress>, id: Uuid) -> Option<ProgressObserver> {
    progress.as_ref().map(|progress| {
        let progress = Arc::clone(progress);
        Arc::new(move |bytes| progress(id, bytes)) as ProgressObserver
    })
}

#[allow(clippy::too_many_arguments)]
async fn execute_sources(
    remote: &mut dyn RemoteFs,
    profile: &ConnectionProfile,
    direction: TransferDirection,
    sources: &[String],
    destination: &str,
    recursive: bool,
    conflict: &str,
    priority: Priority,
    records: &mut Vec<Transfer>,
    progress: Option<HostProgress>,
) -> CoreResult<()> {
    for source in sources {
        match direction {
            TransferDirection::Upload => {
                let local = Path::new(source);
                if local.is_dir() {
                    if !recursive {
                        return Err(CoreError::Conflict(
                            "directory upload requires recursive".to_owned(),
                        ));
                    }
                    let target = join_remote(
                        destination,
                        &local.file_name().unwrap_or_default().to_string_lossy(),
                    );
                    let _ = remote.mkdir(&target).await;
                    Box::pin(upload_directory(
                        remote, profile, local, &target, conflict, priority, records, &progress,
                    ))
                    .await?;
                } else {
                    let name = local
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let Some(remote_path) =
                        resolve_remote_destination(remote, destination, &name, conflict, None)
                            .await?
                    else {
                        continue;
                    };
                    let mut transfer =
                        Transfer::new(direction, profile.id, source.clone(), remote_path, None);
                    transfer.priority = priority;
                    let transfer_id = transfer.id;
                    execute_upload(
                        remote,
                        &mut transfer,
                        local,
                        &TransferControl::new(),
                        transfer_observer(&progress, transfer_id),
                    )
                    .await?;
                    records.push(transfer);
                }
            }
            TransferDirection::Download => {
                let entry = remote.stat(source).await?;
                if entry.entry_type == EntryType::Directory {
                    if !recursive {
                        return Err(CoreError::Conflict(
                            "directory download requires recursive".to_owned(),
                        ));
                    }
                    Box::pin(download_directory(
                        remote,
                        profile,
                        source,
                        Path::new(destination),
                        conflict,
                        priority,
                        records,
                        &progress,
                    ))
                    .await?;
                } else {
                    let Some(local_path) =
                        resolve_local_destination(Path::new(destination), &entry.name, conflict)?
                    else {
                        continue;
                    };
                    let mut transfer = Transfer::new(
                        direction,
                        profile.id,
                        source.clone(),
                        local_path.display().to_string(),
                        entry.size,
                    );
                    transfer.priority = priority;
                    let transfer_id = transfer.id;
                    execute_download(
                        remote,
                        &mut transfer,
                        &local_path,
                        &TransferControl::new(),
                        transfer_observer(&progress, transfer_id),
                    )
                    .await?;
                    records.push(transfer);
                }
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn upload_directory(
    remote: &mut dyn RemoteFs,
    profile: &ConnectionProfile,
    local: &Path,
    remote_dir: &str,
    conflict: &str,
    priority: Priority,
    records: &mut Vec<Transfer>,
    progress: &Option<HostProgress>,
) -> CoreResult<()> {
    for entry in fs::read_dir(local).map_err(|error| CoreError::internal(error.to_string()))? {
        let entry = entry.map_err(|error| CoreError::internal(error.to_string()))?;
        let path = entry.path();
        if path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            continue;
        }
        if path.is_dir() {
            let target = join_remote(remote_dir, &entry.file_name().to_string_lossy());
            let _ = remote.mkdir(&target).await;
            Box::pin(upload_directory(
                remote, profile, &path, &target, conflict, priority, records, progress,
            ))
            .await?;
        } else {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(destination) =
                resolve_remote_destination(remote, remote_dir, &name, conflict, None).await?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Upload,
                profile.id,
                path.display().to_string(),
                destination,
                None,
            );
            transfer.priority = priority;
            let transfer_id = transfer.id;
            execute_upload(
                remote,
                &mut transfer,
                &path,
                &TransferControl::new(),
                transfer_observer(progress, transfer_id),
            )
            .await?;
            records.push(transfer);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_directory(
    remote: &mut dyn RemoteFs,
    profile: &ConnectionProfile,
    remote_dir: &str,
    local_dir: &Path,
    conflict: &str,
    priority: Priority,
    records: &mut Vec<Transfer>,
    progress: &Option<HostProgress>,
) -> CoreResult<()> {
    let target = local_dir.join(remote_dir.rsplit('/').next().unwrap_or("download"));
    fs::create_dir_all(&target).map_err(|error| CoreError::internal(error.to_string()))?;
    for entry in remote.list_dir(remote_dir).await? {
        if entry.entry_type == EntryType::Directory {
            Box::pin(download_directory(
                remote,
                profile,
                &entry.path,
                &target,
                conflict,
                priority,
                records,
                progress,
            ))
            .await?;
        } else if entry.entry_type != EntryType::Symlink {
            let Some(local_path) = resolve_local_destination(&target, &entry.name, conflict)?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Download,
                profile.id,
                entry.path.clone(),
                local_path.display().to_string(),
                entry.size,
            );
            transfer.priority = priority;
            let transfer_id = transfer.id;
            execute_download(
                remote,
                &mut transfer,
                &local_path,
                &TransferControl::new(),
                transfer_observer(progress, transfer_id),
            )
            .await?;
            records.push(transfer);
        }
    }
    Ok(())
}

async fn resolve_remote_destination(
    remote: &mut dyn RemoteFs,
    directory: &str,
    name: &str,
    policy: &str,
    attempt: Option<u32>,
) -> CoreResult<Option<String>> {
    let candidate = join_remote(directory, name);
    if remote.stat(&candidate).await.is_err() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            let start = attempt.unwrap_or(1);
            for suffix in start..start + 1_000 {
                let candidate = join_remote(directory, &format!("{name}.{suffix}"));
                if remote.stat(&candidate).await.is_err() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict("no free destination name".to_owned()))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {candidate}"
        ))),
        other => Err(CoreError::invalid_argument(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn resolve_local_destination(
    directory: &Path,
    name: &str,
    policy: &str,
) -> CoreResult<Option<PathBuf>> {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            for suffix in 1..1_000 {
                let candidate = directory.join(format!("{name}.{suffix}"));
                if !candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict("no free destination name".to_owned()))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {}",
            candidate.display()
        ))),
        other => Err(CoreError::invalid_argument(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn join_remote(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

fn parse_priority(value: &str) -> Priority {
    match value {
        "low" => Priority::Low,
        "high" => Priority::High,
        _ => Priority::Normal,
    }
}

fn parse_uuid(value: Option<&Value>) -> Option<Uuid> {
    value
        .and_then(Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
}

fn block_on<F: std::future::Future>(future: F) -> CoreResult<F::Output> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CoreError::internal(format!("could not start host runtime: {error}")))?;
    Ok(runtime.block_on(future))
}

/// Read host metadata for observer display.
pub fn read_host_status(run_dir: &Path) -> CoreResult<HostStatus> {
    read_status(run_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_priority_values() {
        assert_eq!(parse_priority("high"), Priority::High);
        assert_eq!(parse_priority("low"), Priority::Low);
        assert_eq!(parse_priority("anything"), Priority::Normal);
    }

    #[test]
    fn joins_remote_paths() {
        assert_eq!(join_remote("/srv/app", "file"), "/srv/app/file");
        assert_eq!(join_remote("/srv/app/", "file"), "/srv/app/file");
    }
}
