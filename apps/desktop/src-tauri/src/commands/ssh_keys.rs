//! Tauri commands for SSH private-key discovery.

use tauri::State;
use thriftp_core::ssh_keys::{SshKeyListing, SshKeyValidation, list_default_candidates, validate_key_path};

use crate::AppState;
use crate::error::DesktopError;

/// List conventional private-key candidates in the platform SSH directory.
#[tauri::command]
pub async fn ssh_keys_list(_state: State<'_, AppState>) -> Result<SshKeyListing, DesktopError> {
    list_default_candidates().map_err(DesktopError::from)
}

/// Validate a selected private-key path without reading key contents.
#[tauri::command]
pub async fn ssh_keys_validate(
    _state: State<'_, AppState>,
    path: String,
) -> Result<SshKeyValidation, DesktopError> {
    validate_key_path(&path).map_err(DesktopError::from)
}
