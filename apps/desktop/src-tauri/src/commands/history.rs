//! Tauri commands for connection history.

use serde::Serialize;
use tauri::State;
use thriftp_core::history::ConnectionHistoryService;
use thriftp_core::model::ConnectionHistoryEntry;

use crate::AppState;
use crate::error::DesktopError;

/// Result of clearing history.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryClearView {
    /// Always true after a successful clear.
    pub cleared: bool,
    /// Number of entries removed.
    pub removed: usize,
}

/// List recent successful connections newest first.
#[tauri::command]
pub async fn connection_history_list(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionHistoryEntry>, DesktopError> {
    ConnectionHistoryService::new(state.metadata_store.clone())
        .list()
        .map_err(DesktopError::from)
}

/// Clear history after explicit confirmation.
#[tauri::command]
pub async fn connection_history_clear(
    state: State<'_, AppState>,
    confirmed: bool,
) -> Result<HistoryClearView, DesktopError> {
    let removed = ConnectionHistoryService::new(state.metadata_store.clone())
        .clear(confirmed)
        .map_err(DesktopError::from)?;
    Ok(HistoryClearView {
        cleared: true,
        removed,
    })
}
