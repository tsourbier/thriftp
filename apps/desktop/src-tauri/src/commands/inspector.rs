use tauri::State;
use thriftp_core::browser::metadata::{FileMetadata, metadata_from_entry, metadata_remote};
use thriftp_core::browser::{self};
use thriftp_core::error::CoreError;
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;

#[tauri::command]
pub async fn inspector_get(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    path: String,
) -> Result<FileMetadata, DesktopError> {
    match source.as_str() {
        "local" => {
            let entry =
                browser::stat_local(std::path::Path::new(&path)).map_err(DesktopError::from)?;
            Ok(metadata_from_entry(&entry))
        }
        "remote" => {
            let connection_id = Uuid::parse_str(connection_id.as_deref().unwrap_or_default())
                .map_err(|error| DesktopError::from(CoreError::validation(error.to_string())))?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            metadata_remote(remote.as_mut(), &path)
                .await
                .map_err(DesktopError::from)
        }
        other => Err(DesktopError::from(CoreError::validation(format!(
            "unknown inspector source: {other}"
        )))),
    }
}
