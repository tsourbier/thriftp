use serde::Serialize;
use tauri::State;
use thriftp_core::model::BrowsingSession;
use thriftp_core::preferences::tabs::TabService;

use crate::AppState;
use crate::error::DesktopError;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabsSavedView {
    pub saved: bool,
}

#[tauri::command]
pub async fn tabs_list(state: State<'_, AppState>) -> Result<Vec<BrowsingSession>, DesktopError> {
    let tabs = state.tabs.lock().await;
    Ok(tabs.list())
}

#[tauri::command]
pub async fn tabs_save(
    state: State<'_, AppState>,
    tabs: Vec<BrowsingSession>,
) -> Result<TabsSavedView, DesktopError> {
    {
        let mut service = state.tabs.lock().await;
        service.save(tabs);
    }
    let _ = state.persist_metadata().await;
    Ok(TabsSavedView { saved: true })
}

#[tauri::command]
pub async fn tabs_restore(
    state: State<'_, AppState>,
) -> Result<Vec<BrowsingSession>, DesktopError> {
    let mut service = state.tabs.lock().await;
    let restored = service.list();
    if restored.is_empty() {
        return Ok(Vec::new());
    }
    let state = thriftp_core::repository::AppState {
        tabs: restored,
        ..Default::default()
    };
    let restored = TabService::from_state(&state, true).list();
    service.save(restored.clone());
    Ok(restored)
}
