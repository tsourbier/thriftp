//! Tauri commands for workspace layout persistence.

use tauri::State;
use thriftp_core::model::WorkspaceLayout;
use thriftp_core::workspace::WorkspaceService;

use crate::AppState;
use crate::error::DesktopError;

/// Return the normalized persisted workspace layout.
#[tauri::command]
pub async fn workspace_get(state: State<'_, AppState>) -> Result<WorkspaceLayout, DesktopError> {
    let service = WorkspaceService::new(state.metadata_store.clone());
    service.get().map_err(DesktopError::from)
}

/// Normalize and persist a workspace layout.
#[tauri::command]
pub async fn workspace_set(
    state: State<'_, AppState>,
    layout: WorkspaceLayout,
) -> Result<WorkspaceLayout, DesktopError> {
    let service = WorkspaceService::new(state.metadata_store.clone());
    service.set(layout).map_err(DesktopError::from)
}
