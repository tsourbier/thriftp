use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine;
use serde::Serialize;
use tauri::State;
use thriftp_core::error::CoreError;
use thriftp_core::model::PreviewMode as PreferencePreviewMode;
use thriftp_core::previews::{
    MAX_INLINE_BYTES, PreviewContent, PreviewMode, PreviewPlan, classify, plan_local, plan_remote,
};
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewOpenView {
    pub preview_id: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReadView {
    pub kind: String,
    pub content: String,
    pub truncated: bool,
}

fn mode_label(mode: PreviewMode) -> &'static str {
    match mode {
        PreviewMode::Native => "native",
        PreviewMode::Internal => "internal",
        PreviewMode::Metadata => "metadata",
    }
}

fn native_open(path: &Path) -> Result<(), CoreError> {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = Command::new("open");
        command.arg(path);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]).arg(path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = Command::new("xdg-open");
        command.arg(path);
        command
    };
    command
        .spawn()
        .map(|_| ())
        .map_err(|error| CoreError::internal(format!("could not open preview: {error}")))
}

#[tauri::command]
pub async fn preview_open(
    state: State<'_, AppState>,
    source: String,
    connection_id: Option<String>,
    path: String,
    allow_download: Option<bool>,
) -> Result<PreviewOpenView, DesktopError> {
    let preview_id = Uuid::new_v4().to_string();
    match source.as_str() {
        "local" => {
            let local_path = PathBuf::from(&path);
            let plan = plan_local(&local_path).map_err(DesktopError::from)?;
            let preference = state.preferences.lock().await.get();
            if preference.preview_mode != PreferencePreviewMode::Internal
                && native_open(&local_path).is_ok()
            {
                return Ok(PreviewOpenView {
                    preview_id,
                    mode: "native".to_owned(),
                    reason: None,
                });
            }
            let mode = if plan.mode == PreviewMode::Native {
                PreviewMode::Metadata
            } else {
                plan.mode
            };
            Ok(PreviewOpenView {
                preview_id,
                mode: mode_label(mode).to_owned(),
                reason: plan.reason.or_else(|| {
                    (mode == PreviewMode::Metadata)
                        .then(|| "native preview is unavailable; metadata only".to_owned())
                }),
            })
        }
        "remote" => {
            let connection_id = Uuid::parse_str(connection_id.as_deref().unwrap_or_default())
                .map_err(|error| DesktopError::from(CoreError::validation(error.to_string())))?;
            let shared = {
                let service = state.connections.lock().await;
                service.session(connection_id).map_err(DesktopError::from)?
            };
            let mut remote = shared.lock().await;
            let entry = remote.stat(&path).await.map_err(DesktopError::from)?;
            let plan: PreviewPlan =
                plan_remote(entry.size, &entry.name, allow_download.unwrap_or(false));
            if plan.mode != PreviewMode::Internal {
                return Ok(PreviewOpenView {
                    preview_id,
                    mode: mode_label(plan.mode).to_owned(),
                    reason: plan.reason,
                });
            }
            let temp_path = std::env::temp_dir().join(format!("thriftp-preview-{preview_id}"));
            let progress: thriftp_core::protocol::ProgressCallback = std::sync::Arc::new(|_| true);
            remote
                .download_to(&path, &temp_path, 0, progress)
                .await
                .map_err(DesktopError::from)?;
            state
                .previews
                .lock()
                .expect("preview registry lock")
                .insert(preview_id.clone(), temp_path);
            Ok(PreviewOpenView {
                preview_id,
                mode: mode_label(PreviewMode::Internal).to_owned(),
                reason: None,
            })
        }
        other => Err(DesktopError::from(CoreError::validation(format!(
            "unknown preview source: {other}"
        )))),
    }
}

#[tauri::command]
pub async fn preview_read(
    state: State<'_, AppState>,
    source: String,
    path: String,
    preview_id: Option<String>,
) -> Result<PreviewReadView, DesktopError> {
    let read_path = match source.as_str() {
        "local" => PathBuf::from(&path),
        "remote" => {
            let preview_id = preview_id.ok_or_else(|| {
                DesktopError::from(CoreError::validation(
                    "previewId is required for a remote preview",
                ))
            })?;
            state
                .previews
                .lock()
                .expect("preview registry lock")
                .get(&preview_id)
                .cloned()
                .ok_or_else(|| {
                    DesktopError::from(CoreError::NotFound(
                        "remote preview has not been downloaded".to_owned(),
                    ))
                })?
        }
        other => {
            return Err(DesktopError::from(CoreError::validation(format!(
                "unknown preview source: {other}"
            ))));
        }
    };
    let content = classify(
        Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(""),
    );
    if content == PreviewContent::Binary {
        return Err(DesktopError::from(CoreError::UnsupportedCapability(
            "binary files cannot be rendered internally".to_owned(),
        )));
    }
    let bytes = std::fs::read(&read_path).map_err(|error| {
        DesktopError::from(CoreError::internal(format!(
            "could not read preview file: {error}"
        )))
    })?;
    let truncated = bytes.len() as u64 > MAX_INLINE_BYTES;
    let bytes = &bytes[..bytes.len().min(MAX_INLINE_BYTES as usize)];
    let (kind, content) = match content {
        PreviewContent::Text => ("text", String::from_utf8_lossy(bytes).into_owned()),
        PreviewContent::Image => (
            "image",
            base64::engine::general_purpose::STANDARD.encode(bytes),
        ),
        PreviewContent::Binary => unreachable!(),
    };
    Ok(PreviewReadView {
        kind: kind.to_owned(),
        content,
        truncated,
    })
}

#[tauri::command]
pub async fn preview_close(
    state: State<'_, AppState>,
    preview_id: String,
) -> Result<serde_json::Value, DesktopError> {
    let path = state
        .previews
        .lock()
        .expect("preview registry lock")
        .remove(&preview_id);
    if let Some(path) = path {
        let _ = std::fs::remove_file(path);
    }
    Ok(serde_json::json!({ "closed": true }))
}
