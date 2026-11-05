use tauri::{AppHandle, Emitter, State};
use thriftp_core::model::Preference;

use crate::AppState;
use crate::error::DesktopError;

#[tauri::command]
pub async fn preferences_get(state: State<'_, AppState>) -> Result<Preference, DesktopError> {
    Ok(state.preferences.lock().await.get())
}

#[tauri::command]
pub async fn preferences_set(
    app: AppHandle,
    state: State<'_, AppState>,
    preference: Preference,
) -> Result<Preference, DesktopError> {
    {
        let mut service = state.preferences.lock().await;
        service.set(preference.clone());
    }
    let queue = {
        let mut manager = state.transfers.lock().await;
        manager
            .queue
            .set_concurrency(preference.default_concurrency);
        manager.set_bandwidth(preference.bandwidth_limit_bytes_per_second);
        let _ = manager.persist();
        manager.queue.queue.clone()
    };
    let _ = app.emit("queue:state", queue);
    state.persist_metadata().await.map_err(DesktopError::from)?;
    Ok(preference)
}
