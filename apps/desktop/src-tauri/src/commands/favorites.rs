use tauri::State;
use thriftp_core::model::DirectoryFavorite;
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;
use crate::views::DeletedView;

fn parse_id(value: &str) -> Result<Uuid, DesktopError> {
    Uuid::parse_str(value).map_err(|error| {
        DesktopError::from(thriftp_core::error::CoreError::validation(
            error.to_string(),
        ))
    })
}

fn parse_ids(values: &[String]) -> Result<Vec<Uuid>, DesktopError> {
    values.iter().map(|value| parse_id(value)).collect()
}

#[tauri::command]
pub async fn directory_favorites_list(
    state: State<'_, AppState>,
    profile_id: Option<String>,
) -> Result<Vec<DirectoryFavorite>, DesktopError> {
    let profile_id = profile_id.as_deref().map(parse_id).transpose()?;
    let favorites = state.favorites.lock().await;
    Ok(favorites.list(profile_id))
}

#[tauri::command]
pub async fn directory_favorites_create(
    state: State<'_, AppState>,
    profile_id: String,
    path: String,
    label: String,
) -> Result<DirectoryFavorite, DesktopError> {
    let profile_id = parse_id(&profile_id)?;
    let mut favorites = state.favorites.lock().await;
    let order = favorites.list(Some(profile_id)).len() as i64;
    let favorite = DirectoryFavorite::new(profile_id, path, label, order);
    let id = favorites.create(favorite).map_err(DesktopError::from)?;
    let result = favorites.get(id).map_err(DesktopError::from);
    drop(favorites);
    if result.is_ok() {
        let _ = state.persist_metadata().await;
    }
    result
}

#[tauri::command]
pub async fn directory_favorites_update(
    state: State<'_, AppState>,
    id: String,
    label: Option<String>,
    path: Option<String>,
) -> Result<DirectoryFavorite, DesktopError> {
    let id = parse_id(&id)?;
    let mut favorites = state.favorites.lock().await;
    let mut favorite = favorites.get(id).map_err(DesktopError::from)?;
    if let Some(label) = label {
        favorite.label = label;
    }
    if let Some(path) = path {
        favorite.path = path;
    }
    favorite.updated_at = chrono::Utc::now();
    favorites.update(favorite).map_err(DesktopError::from)?;
    let result = favorites.get(id).map_err(DesktopError::from);
    drop(favorites);
    if result.is_ok() {
        let _ = state.persist_metadata().await;
    }
    result
}

#[tauri::command]
pub async fn directory_favorites_delete(
    state: State<'_, AppState>,
    id: String,
    confirmed: bool,
) -> Result<DeletedView, DesktopError> {
    if !confirmed {
        return Err(DesktopError::from(
            thriftp_core::error::CoreError::Conflict(
                "deleting a directory favorite requires confirmation".to_owned(),
            ),
        ));
    }
    let id = parse_id(&id)?;
    let mut favorites = state.favorites.lock().await;
    favorites.delete(id).map_err(DesktopError::from)?;
    drop(favorites);
    let _ = state.persist_metadata().await;
    Ok(DeletedView { deleted: true })
}

#[tauri::command]
pub async fn directory_favorites_reorder(
    state: State<'_, AppState>,
    profile_id: String,
    ids: Vec<String>,
) -> Result<Vec<DirectoryFavorite>, DesktopError> {
    let profile_id = parse_id(&profile_id)?;
    let ids = parse_ids(&ids)?;
    let mut favorites = state.favorites.lock().await;
    favorites
        .reorder(profile_id, &ids)
        .map_err(DesktopError::from)?;
    let result = favorites.list(Some(profile_id));
    drop(favorites);
    let _ = state.persist_metadata().await;
    Ok(result)
}
