//! Tauri commands for server favorites, connections, and credential management.

#![deny(missing_docs)]

use tauri::{AppHandle, Emitter, State};
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::Secret;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::history::ConnectionHistoryService;
use thriftp_core::model::{CredentialPolicy, FavoritePreferences};
use thriftp_core::server_favorites::{
    CredentialImpact, ReusableCredential, ServerFavoriteInput, ServerFavoriteView,
};
use thriftp_protocols::ProtocolFactory;
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;
use crate::views::{
    ConnectionTestView, ConnectionView, CredentialRemoveView, CredentialsChangedEvent, DeletedView,
    FavoriteDeleteView, FavoritesChangedEvent, RenameGroupView,
};

fn parse_id(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| CoreError::validation("invalid id"))
}

fn secret_from(value: Option<String>) -> Option<Secret> {
    value.map(|value| Secret::new(value.into_bytes()))
}

async fn refresh_profiles(state: &AppState) -> CoreResult<()> {
    let metadata = state.metadata_store.load()?;
    let mut connections = state.connections.lock().await;
    connections.load_profiles(metadata.connection_profiles);
    Ok(())
}

fn emit_favorite_changed(app: &AppHandle, favorite_id: Uuid, reason: &str) {
    let _ = app.emit(
        "favorites:changed",
        FavoritesChangedEvent {
            favorite_id: favorite_id.to_string(),
            reason: reason.to_owned(),
        },
    );
}

pub(crate) async fn remove_profile_dependents(state: &AppState, profile_id: Uuid) {
    {
        let mut favorites = state.favorites.lock().await;
        let remaining = favorites
            .snapshot()
            .into_iter()
            .filter(|favorite| favorite.connection_profile_id != profile_id)
            .collect();
        favorites.load(remaining);
    }
    {
        let mut tabs = state.tabs.lock().await;
        let remaining = tabs
            .list()
            .into_iter()
            .filter(|tab| tab.connection_profile_id != Some(profile_id))
            .collect();
        tabs.save(remaining);
    }
}

fn affected_favorites(state: &AppState, ref_id: Uuid) -> Result<Vec<Uuid>, DesktopError> {
    Ok(state
        .metadata_store
        .load()
        .map_err(DesktopError::from)?
        .server_favorites
        .iter()
        .filter(|favorite| favorite.credential_ref_id == Some(ref_id))
        .map(|favorite| favorite.id)
        .collect())
}

fn credential_ref_for(state: &AppState, favorite_id: Uuid) -> Result<Option<Uuid>, DesktopError> {
    Ok(state
        .metadata_store
        .load()
        .map_err(DesktopError::from)?
        .server_favorites
        .iter()
        .find(|favorite| favorite.id == favorite_id)
        .and_then(|favorite| favorite.credential_ref_id))
}

fn emit_credentials_changed(
    app: &AppHandle,
    credential_ref_id: Uuid,
    affected: &[Uuid],
    reason: &str,
) {
    let _ = app.emit(
        "credentials:changed",
        CredentialsChangedEvent {
            credential_ref_id: credential_ref_id.to_string(),
            affected_favorite_ids: affected.iter().map(ToString::to_string).collect(),
            reason: reason.to_owned(),
        },
    );
}

#[tauri::command]
/// List saved server favorites.
pub async fn favorites_list(
    state: State<'_, AppState>,
) -> Result<Vec<ServerFavoriteView>, DesktopError> {
    state
        .server_favorites
        .ensure_favorites()
        .map_err(DesktopError::from)?;
    state
        .server_favorites
        .reconcile_profiles(&state.credentials)
        .map_err(DesktopError::from)?;
    state.server_favorites.list().map_err(DesktopError::from)
}

#[tauri::command]
/// Return one saved server favorite.
pub async fn favorites_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<ServerFavoriteView, DesktopError> {
    state
        .server_favorites
        .get(parse_id(&id)?)
        .map_err(DesktopError::from)
}

#[tauri::command]
/// Create a favorite and optionally save its credential.
#[allow(clippy::too_many_arguments)]
pub async fn favorites_create(
    app: AppHandle,
    state: State<'_, AppState>,
    mut profile: ServerFavoriteInput,
    credential_policy: CredentialPolicy,
    pinned: Option<bool>,
    credential_ref_id: Option<String>,
    secret: Option<String>,
    secret_kind: Option<String>,
) -> Result<ServerFavoriteView, DesktopError> {
    let _ = secret_kind;
    if let Some(pinned) = pinned {
        profile.pinned = pinned;
    }
    let result = state
        .server_favorites
        .create(
            profile,
            credential_policy,
            credential_ref_id.as_deref().map(parse_id).transpose()?,
            secret_from(secret),
            &state.credentials,
        )
        .map_err(DesktopError::from)?;
    refresh_profiles(&state).await.map_err(DesktopError::from)?;
    emit_favorite_changed(&app, result.id, "created");
    if let Some(reference) = credential_ref_for(&state, result.id)? {
        let affected = affected_favorites(&state, reference)?;
        emit_credentials_changed(&app, reference, &affected, "updated");
    }
    Ok(result)
}

#[tauri::command]
/// Update a favorite and invalidate stale credentials.
#[allow(clippy::too_many_arguments)]
pub async fn favorites_update(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mut profile: ServerFavoriteInput,
    credential_policy: CredentialPolicy,
    pinned: Option<bool>,
    credential_ref_id: Option<String>,
    secret: Option<String>,
    secret_kind: Option<String>,
) -> Result<ServerFavoriteView, DesktopError> {
    let _ = secret_kind;
    if let Some(pinned) = pinned {
        profile.pinned = pinned;
    }
    let favorite_id = parse_id(&id)?;
    let result = state
        .server_favorites
        .update(
            favorite_id,
            profile,
            credential_policy,
            credential_ref_id.as_deref().map(parse_id).transpose()?,
            secret_from(secret),
            &state.credentials,
        )
        .map_err(DesktopError::from)?;
    refresh_profiles(&state).await.map_err(DesktopError::from)?;
    emit_favorite_changed(&app, favorite_id, "updated");
    if let Some(reference) = credential_ref_for(&state, favorite_id)? {
        let affected = affected_favorites(&state, reference)?;
        emit_credentials_changed(&app, reference, &affected, "updated");
    }
    Ok(result)
}

#[tauri::command]
/// Delete a favorite and clean dependent metadata.
pub async fn favorites_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<FavoriteDeleteView, DesktopError> {
    let favorite_id = parse_id(&id)?;
    let metadata = state.metadata_store.load().map_err(DesktopError::from)?;
    let stored = metadata
        .server_favorites
        .iter()
        .find(|favorite| favorite.id == favorite_id);
    let credential_ref_id = stored.and_then(|favorite| favorite.credential_ref_id);
    let profile_id = stored.map(|favorite| favorite.connection_profile_id);
    let impact = state
        .server_favorites
        .delete(favorite_id)
        .map_err(DesktopError::from)?;
    if let Some(profile_id) = profile_id {
        remove_profile_dependents(&state, profile_id).await;
    }
    refresh_profiles(&state).await.map_err(DesktopError::from)?;
    let orphaned_credential_ref_id = credential_ref_id
        .filter(|_| impact.affected_favorites.is_empty())
        .map(|reference| reference.to_string());
    emit_favorite_changed(&app, favorite_id, "deleted");
    if let Some(reference) = credential_ref_id {
        let affected = affected_favorites(&state, reference)?;
        if affected.is_empty() {
            emit_credentials_changed(&app, reference, &[], "deleted");
        } else {
            emit_credentials_changed(&app, reference, &affected, "removed");
        }
    }
    Ok(FavoriteDeleteView {
        deleted: true,
        orphaned_credential_ref_id,
        credential_impact: Some(impact),
    })
}

#[tauri::command]
/// Pin or unpin a favorite.
pub async fn favorites_set_pinned(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    pinned: bool,
) -> Result<ServerFavoriteView, DesktopError> {
    let favorite_id = parse_id(&id)?;
    let view = state
        .server_favorites
        .set_pinned(favorite_id, pinned)
        .map_err(DesktopError::from)?;
    emit_favorite_changed(&app, favorite_id, "pinned");
    Ok(view)
}

#[tauri::command]
/// Assign or clear a favorite group label.
pub async fn favorites_set_group(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    group_label: Option<String>,
) -> Result<ServerFavoriteView, DesktopError> {
    let favorite_id = parse_id(&id)?;
    let view = state
        .server_favorites
        .set_group(favorite_id, group_label)
        .map_err(DesktopError::from)?;
    emit_favorite_changed(&app, favorite_id, "groupChanged");
    Ok(view)
}

#[tauri::command]
/// Rename a group label across its favorites.
pub async fn favorites_rename_group(
    app: AppHandle,
    state: State<'_, AppState>,
    old_label: String,
    new_label: String,
) -> Result<RenameGroupView, DesktopError> {
    let updated = state
        .server_favorites
        .rename_group(&old_label, &new_label)
        .map_err(DesktopError::from)?;
    for favorite in &updated {
        emit_favorite_changed(&app, favorite.id, "groupChanged");
    }
    Ok(RenameGroupView { updated })
}

#[tauri::command]
/// Return the persisted favorite preferences.
pub async fn favorites_get_preferences(
    state: State<'_, AppState>,
) -> Result<FavoritePreferences, DesktopError> {
    state
        .server_favorites
        .preferences()
        .map_err(DesktopError::from)
}

#[tauri::command]
/// Persist the favorite sort preferences.
pub async fn favorites_set_preferences(
    state: State<'_, AppState>,
    preferences: FavoritePreferences,
) -> Result<FavoritePreferences, DesktopError> {
    state
        .server_favorites
        .set_preferences(preferences)
        .map_err(DesktopError::from)
}

#[tauri::command]
/// Test pending settings without persisting them.
pub async fn favorites_test_connection(
    _state: State<'_, AppState>,
    profile: ServerFavoriteInput,
    secret: Option<String>,
    secret_kind: Option<String>,
) -> Result<ConnectionTestView, DesktopError> {
    let _ = secret_kind;
    let profile = profile.into_profile();
    let mut service = ConnectionService::new(Box::new(ProtocolFactory));
    service.load_profiles(vec![profile.clone()]);
    match service.test(profile.id, secret_from(secret)).await {
        Ok(()) => Ok(ConnectionTestView {
            ok: true,
            error: None,
        }),
        Err(error) => Ok(ConnectionTestView {
            ok: false,
            error: Some(DesktopError::from(error)),
        }),
    }
}

#[tauri::command]
/// Connect to a favorite with saved or session credentials.
pub async fn favorites_connect(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    secret: Option<String>,
    secret_kind: Option<String>,
) -> Result<ConnectionView, DesktopError> {
    let _ = secret_kind;
    let favorite_id = parse_id(&id)?;
    let (profile, stored) = state
        .server_favorites
        .connection_target(favorite_id, &state.credentials)
        .map_err(DesktopError::from)?;
    refresh_profiles(&state).await.map_err(DesktopError::from)?;
    let connection = {
        let mut service = state.connections.lock().await;
        crate::commands::connections::connect(
            &mut service,
            &state.credentials,
            profile.id,
            secret_from(secret).or(stored),
        )
        .await?
    };
    state
        .server_favorites
        .mark_used(favorite_id)
        .map_err(DesktopError::from)?;
    ConnectionHistoryService::new(state.metadata_store.clone())
        .record_profile(&profile, Some(favorite_id))
        .map_err(DesktopError::from)?;
    let mut view = ConnectionView::from(connection);
    view.favorite_id = Some(favorite_id.to_string());
    let _ = app.emit("connection:state", view.clone());
    Ok(view)
}

#[tauri::command]
/// Describe the favorites affected by a credential change.
pub async fn credentials_get_impact(
    state: State<'_, AppState>,
    favorite_id: String,
) -> Result<CredentialImpact, DesktopError> {
    state
        .server_favorites
        .impact(parse_id(&favorite_id)?)
        .map_err(DesktopError::from)
}

#[tauri::command]
/// List credential references available for reuse.
pub async fn credentials_list_reusable(
    state: State<'_, AppState>,
) -> Result<Vec<ReusableCredential>, DesktopError> {
    state
        .server_favorites
        .reusable()
        .map_err(DesktopError::from)
}

#[tauri::command]
/// Remove a favorite credential reference.
pub async fn credentials_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    favorite_id: String,
    confirmed: bool,
) -> Result<CredentialRemoveView, DesktopError> {
    if !confirmed {
        return Err(DesktopError::from(CoreError::Conflict(
            "credential removal requires confirmation".to_owned(),
        )));
    }
    let favorite_id = parse_id(&favorite_id)?;
    let impact = state
        .server_favorites
        .remove_credential(favorite_id, &state.credentials)
        .map_err(DesktopError::from)?;
    let affected: Vec<_> = impact
        .affected_favorites
        .iter()
        .map(|favorite| favorite.id)
        .filter(|id| *id != favorite_id)
        .collect();
    emit_credentials_changed(&app, impact.credential_ref_id, &affected, "removed");
    emit_favorite_changed(&app, favorite_id, "credentialChanged");
    Ok(CredentialRemoveView {
        removed: true,
        impact,
    })
}

#[tauri::command]
/// Delete an unreferenced credential after confirmation.
pub async fn credentials_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    credential_ref_id: String,
    confirmed: bool,
) -> Result<DeletedView, DesktopError> {
    let credential_ref_id = parse_id(&credential_ref_id)?;
    state
        .server_favorites
        .delete_credential(credential_ref_id, confirmed, &state.credentials)
        .map_err(DesktopError::from)?;
    emit_credentials_changed(&app, credential_ref_id, &[], "deleted");
    Ok(DeletedView { deleted: true })
}
