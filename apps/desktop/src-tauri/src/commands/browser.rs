use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, State};
use thriftp_core::browser;
use thriftp_core::browser::listing::{
    self, DEFAULT_BATCH_SIZE, ListingBatch, ListingSession, ListingState,
};
use thriftp_core::connections::ConnectionService;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::FileEntry;
use thriftp_core::navigation::{self, PathFallback, PathResolution};
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;
use crate::views::{CancelledView, DeletedView, ListingHandle};

#[derive(Debug, Clone)]
enum ListingTarget {
    Local(PathBuf),
    Remote { connection_id: Uuid, path: String },
}

#[derive(Debug, Clone)]
struct ListingEntry {
    session: ListingSession,
    target: ListingTarget,
}

/// Registry of in-flight directory listings.
#[derive(Default)]
pub struct BrowserListings {
    entries: Mutex<HashMap<Uuid, ListingEntry>>,
}

impl BrowserListings {
    fn insert(&self, id: Uuid, entry: ListingEntry) {
        self.entries
            .lock()
            .expect("listing registry lock")
            .insert(id, entry);
    }

    fn session(&self, id: Uuid) -> Option<ListingSession> {
        self.entries
            .lock()
            .expect("listing registry lock")
            .get(&id)
            .map(|entry| entry.session.clone())
    }

    fn target(&self, id: Uuid) -> Option<ListingTarget> {
        self.entries
            .lock()
            .expect("listing registry lock")
            .get(&id)
            .map(|entry| entry.target.clone())
    }

    fn remove(&self, id: Uuid) {
        self.entries
            .lock()
            .expect("listing registry lock")
            .remove(&id);
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryBatchPayload {
    pub listing_id: String,
    pub entries: Vec<FileEntry>,
    pub state: ListingState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_entry_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryErrorPayload {
    pub listing_id: String,
    pub error: DesktopError,
}

/// Serialize a listing batch exactly as it is emitted to the UI.
#[must_use]
pub fn batch_payload(listing_id: &str, batch: ListingBatch) -> DirectoryBatchPayload {
    DirectoryBatchPayload {
        listing_id: listing_id.to_owned(),
        entries: batch.entries,
        state: batch.state,
        first_entry_at: batch.first_entry_at.map(|value| value.to_rfc3339()),
        completed_at: batch.completed_at.map(|value| value.to_rfc3339()),
    }
}

fn emit_batch(app: &AppHandle, listing_id: &str, batch: ListingBatch) {
    let _ = app.emit("directory:batch", batch_payload(listing_id, batch));
}

fn finish(app: &AppHandle, registry: &BrowserListings, id: Uuid, result: CoreResult<()>) {
    if let Err(error) = result
        && !matches!(error, CoreError::Cancelled)
    {
        let _ = app.emit(
            "directory:error",
            DirectoryErrorPayload {
                listing_id: id.to_string(),
                error: DesktopError::from(error),
            },
        );
    }
    registry.remove(id);
}

fn start_local(
    app: AppHandle,
    registry: Arc<BrowserListings>,
    id: Uuid,
    path: PathBuf,
) -> ListingHandle {
    let session = ListingSession::new();
    registry.insert(
        id,
        ListingEntry {
            session: session.clone(),
            target: ListingTarget::Local(path.clone()),
        },
    );
    let listing_id = id.to_string();
    let task_registry = Arc::clone(&registry);
    drop(tauri::async_runtime::spawn_blocking(move || {
        let result = listing::stream_local(&path, DEFAULT_BATCH_SIZE, &session, |batch| {
            emit_batch(&app, &listing_id, batch);
        });
        finish(&app, &task_registry, id, result);
    }));
    ListingHandle {
        listing_id: id.to_string(),
    }
}

fn start_remote(
    app: AppHandle,
    connections: Arc<tokio::sync::Mutex<ConnectionService>>,
    registry: Arc<BrowserListings>,
    id: Uuid,
    connection_id: Uuid,
    path: String,
) -> ListingHandle {
    let session = ListingSession::new();
    registry.insert(
        id,
        ListingEntry {
            session: session.clone(),
            target: ListingTarget::Remote {
                connection_id,
                path: path.clone(),
            },
        },
    );
    let listing_id = id.to_string();
    let task_registry = Arc::clone(&registry);
    drop(tauri::async_runtime::spawn(async move {
        let service = connections.lock().await;
        let shared = service.session(connection_id);
        drop(service);
        let result = match shared {
            Ok(shared) => {
                let mut remote = shared.lock().await;
                listing::stream_remote(
                    remote.as_mut(),
                    &path,
                    DEFAULT_BATCH_SIZE,
                    &session,
                    |batch| {
                        emit_batch(&app, &listing_id, batch);
                    },
                )
                .await
            }
            Err(error) => Err(error),
        };
        finish(&app, &task_registry, id, result);
    }));
    ListingHandle {
        listing_id: id.to_string(),
    }
}

fn parse_id(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| CoreError::Validation(format!("invalid id: {error}")))
}

#[tauri::command]
pub async fn browser_open_local(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<ListingHandle, DesktopError> {
    let id = Uuid::new_v4();
    Ok(start_local(
        app,
        Arc::clone(&state.listings),
        id,
        PathBuf::from(path),
    ))
}

#[tauri::command]
pub async fn browser_open_remote(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
) -> Result<ListingHandle, DesktopError> {
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let id = Uuid::new_v4();
    Ok(start_remote(
        app,
        Arc::clone(&state.connections),
        Arc::clone(&state.listings),
        id,
        connection_id,
        path,
    ))
}

#[tauri::command]
pub async fn browser_refresh(
    app: AppHandle,
    state: State<'_, AppState>,
    listing_id: String,
) -> Result<ListingHandle, DesktopError> {
    let id = parse_id(&listing_id).map_err(DesktopError::from)?;
    let target = state
        .listings
        .target(id)
        .ok_or_else(|| DesktopError::from(CoreError::NotFound("directory listing".to_owned())))?;
    if let Some(session) = state.listings.session(id) {
        session.cancel();
    }
    state.listings.remove(id);
    Ok(match target {
        ListingTarget::Local(path) => start_local(app, Arc::clone(&state.listings), id, path),
        ListingTarget::Remote {
            connection_id,
            path,
        } => start_remote(
            app,
            Arc::clone(&state.connections),
            Arc::clone(&state.listings),
            id,
            connection_id,
            path,
        ),
    })
}

#[tauri::command]
pub async fn browser_cancel_listing(
    state: State<'_, AppState>,
    listing_id: String,
) -> Result<CancelledView, DesktopError> {
    let id = parse_id(&listing_id).map_err(DesktopError::from)?;
    if let Some(session) = state.listings.session(id) {
        session.cancel();
    }
    state.listings.remove(id);
    Ok(CancelledView { cancelled: true })
}

#[tauri::command]
pub async fn browser_stat(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    path: String,
) -> Result<FileEntry, DesktopError> {
    match source.as_str() {
        "local" => browser::stat_local(std::path::Path::new(&path)).map_err(DesktopError::from),
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            browser::stat_remote(remote.as_mut(), &path)
                .await
                .map_err(DesktopError::from)
        }
        other => Err(DesktopError::from(CoreError::Validation(format!(
            "unknown browser source: {other}"
        )))),
    }
}

#[tauri::command]
pub async fn browser_mkdir(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    path: String,
) -> Result<FileEntry, DesktopError> {
    match source.as_str() {
        "local" => {
            let target = std::path::PathBuf::from(&path);
            browser::create_dir_local(&target).map_err(DesktopError::from)?;
            browser::stat_local(&target).map_err(DesktopError::from)
        }
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            browser::create_dir_remote(remote.as_mut(), &path)
                .await
                .map_err(DesktopError::from)?;
            browser::stat_remote(remote.as_mut(), &path)
                .await
                .map_err(DesktopError::from)
        }
        other => Err(DesktopError::from(CoreError::Validation(format!(
            "unknown browser source: {other}"
        )))),
    }
}

#[tauri::command]
pub async fn browser_rename(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    from: String,
    to: String,
) -> Result<FileEntry, DesktopError> {
    match source.as_str() {
        "local" => {
            browser::rename_local(std::path::Path::new(&from), std::path::Path::new(&to))
                .map_err(DesktopError::from)?;
            browser::stat_local(std::path::Path::new(&to)).map_err(DesktopError::from)
        }
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            browser::rename_remote(remote.as_mut(), &from, &to)
                .await
                .map_err(DesktopError::from)?;
            browser::stat_remote(remote.as_mut(), &to)
                .await
                .map_err(DesktopError::from)
        }
        other => Err(DesktopError::from(CoreError::Validation(format!(
            "unknown browser source: {other}"
        )))),
    }
}

#[tauri::command]
pub async fn browser_delete(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    path: String,
    confirmed: bool,
) -> Result<DeletedView, DesktopError> {
    match source.as_str() {
        "local" => {
            browser::delete::delete_local(std::path::Path::new(&path))
                .map_err(DesktopError::from)?;
        }
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            let entry = remote.stat(&path).await.map_err(DesktopError::from)?;
            let entry_type = entry.entry_type;
            browser::delete::delete_remote(remote.as_mut(), &path, entry_type, confirmed)
                .await
                .map_err(DesktopError::from)?;
        }
        other => {
            return Err(DesktopError::from(CoreError::Validation(format!(
                "unknown browser source: {other}"
            ))));
        }
    }
    Ok(DeletedView { deleted: true })
}

#[tauri::command]
pub async fn browser_chmod(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
    mode: u32,
) -> Result<FileEntry, DesktopError> {
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let shared = {
        let service = state.connections.lock().await;
        service.session(connection_id).map_err(DesktopError::from)?
    };
    let mut remote = shared.lock().await;
    browser::permissions::chmod_remote(remote.as_mut(), &path, mode)
        .await
        .map_err(DesktopError::from)
}

/// Reject recursive permission changes that were not explicitly confirmed.
pub fn require_recursive_confirmation(confirmed: bool) -> Result<(), DesktopError> {
    if confirmed {
        return Ok(());
    }
    Err(DesktopError::from(CoreError::Conflict(
        "recursive permission changes require confirmation".to_owned(),
    )))
}

/// Apply a permission mode to every descendant of a remote directory.
#[tauri::command]
pub async fn browser_chmod_recursive(
    state: State<'_, AppState>,
    connection_id: String,
    path: String,
    mode: u32,
    confirmed: bool,
) -> Result<browser::permissions::RecursivePermissionResult, DesktopError> {
    require_recursive_confirmation(confirmed)?;
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let shared = {
        let service = state.connections.lock().await;
        service.session(connection_id).map_err(DesktopError::from)?
    };
    let mut remote = shared.lock().await;
    browser::permissions::chmod_remote_recursive(remote.as_mut(), &path, mode)
        .await
        .map_err(DesktopError::from)
}

/// Return whether a local directory can be opened for listing.
#[must_use]
pub fn probe_local_directory(path: &Path) -> bool {
    std::fs::read_dir(path).is_ok()
}

/// Check whether a local or remote directory is readable before navigating to it.
#[tauri::command]
pub async fn browser_probe_directory(
    state: State<'_, AppState>,
    source: String,
    path: String,
    connection_id: Option<String>,
) -> Result<bool, DesktopError> {
    match source.as_str() {
        "local" => Ok(probe_local_directory(Path::new(&path))),
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            Ok(remote.list_dir(&path).await.is_ok())
        }
        other => Err(DesktopError::from(CoreError::validation(format!(
            "unknown directory source: {other}"
        )))),
    }
}

/// Resolve a saved local or remote path to an accessible location.
#[tauri::command]
pub async fn browser_resolve_path(
    state: State<'_, AppState>,
    source: String,
    path: String,
    connection_id: Option<String>,
) -> Result<PathResolution, DesktopError> {
    match source.as_str() {
        "local" => Ok(navigation::resolve_local(&path)),
        "remote" => {
            let connection_id = parse_id(connection_id.as_deref().unwrap_or_default())
                .map_err(DesktopError::from)?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            if let Ok(entry) = remote.stat(&path).await
                && entry.entry_type == thriftp_core::model::EntryType::Directory
            {
                return Ok(PathResolution::new(&path, &path, PathFallback::None, None));
            }
            for candidate in navigation::posix_ancestor_candidates(&path) {
                if let Ok(entry) = remote.stat(&candidate).await
                    && entry.entry_type == thriftp_core::model::EntryType::Directory
                {
                    return Ok(PathResolution::new(
                        &path,
                        candidate,
                        PathFallback::Ancestor,
                        Some("The saved remote folder could not be opened. Using the nearest available folder instead.".to_owned()),
                    ));
                }
            }
            Ok(PathResolution::new(
                &path,
                ".",
                PathFallback::Login,
                Some("The saved remote folder could not be opened. Using the server login folder instead.".to_owned()),
            ))
        }
        other => Err(DesktopError::from(CoreError::Validation(format!(
            "unknown path source: {other}"
        )))),
    }
}
