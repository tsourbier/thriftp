use std::path::Path;

use tauri::{AppHandle, Emitter, State};
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{CredentialService, Secret};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::history::ConnectionHistoryService;
use thriftp_core::model::{
    ActiveConnection, ConnectionProfile, CredentialPolicy, CredentialRef, Protocol,
};
use thriftp_core::server_favorites::ServerFavoriteInput;
use uuid::Uuid;

use crate::AppState;
use crate::error::DesktopError;
use crate::views::{
    ActiveProfileView, ConnectionTestView, ConnectionView, CredentialsChangedEvent, DeletedView,
    DisconnectedView, FavoritesChangedEvent, TrustedHostKeyView,
};

fn parse_id(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|error| CoreError::Validation(format!("invalid id: {error}")))
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

fn emit_favorite(app: &AppHandle, favorite_id: uuid::Uuid, reason: &str) {
    let _ = app.emit(
        "favorites:changed",
        FavoritesChangedEvent {
            favorite_id: favorite_id.to_string(),
            reason: reason.to_owned(),
        },
    );
}

fn emit_credential_deleted(app: &AppHandle, reference: uuid::Uuid) {
    let _ = app.emit(
        "credentials:changed",
        CredentialsChangedEvent {
            credential_ref_id: reference.to_string(),
            affected_favorite_ids: Vec::new(),
            reason: "deleted".to_owned(),
        },
    );
}

/// Resolve a supplied session secret or the credential bound to the profile.
///
/// Key-based SSH profiles may legitimately have no credential when the key is
/// not passphrase protected.
pub fn credential_for(
    credentials: &CredentialService,
    profile: &ConnectionProfile,
    supplied: Option<Secret>,
) -> CoreResult<Option<Secret>> {
    if let Some(secret) = supplied {
        return Ok(Some(secret));
    }
    let stored = credentials.get(&CredentialRef::for_profile(profile))?;
    if stored.is_none() && profile.auth_method != thriftp_core::model::AuthMethod::SshPrivateKey {
        return Err(CoreError::CredentialMissing);
    }
    Ok(stored)
}

pub fn list(service: &ConnectionService) -> Vec<ConnectionProfile> {
    service.list_profiles()
}

pub fn create(
    service: &mut ConnectionService,
    credentials: &CredentialService,
    profile: ConnectionProfile,
    secret: Option<Secret>,
) -> CoreResult<ConnectionProfile> {
    let id = service.create_profile(profile)?;
    let stored = service.get_profile(id)?;
    if let Some(secret) = secret
        && let Err(error) = credentials.set(&CredentialRef::for_profile(&stored), secret)
    {
        let _ = service.delete_profile(id);
        return Err(error);
    }
    Ok(stored)
}

pub fn update(
    service: &mut ConnectionService,
    credentials: &CredentialService,
    profile: ConnectionProfile,
    secret: Option<Secret>,
) -> CoreResult<ConnectionProfile> {
    let id = profile.id;
    service.update_profile(profile)?;
    let stored = service.get_profile(id)?;
    if let Some(secret) = secret {
        credentials.set(&CredentialRef::for_profile(&stored), secret)?;
    }
    Ok(stored)
}

pub fn delete(
    service: &mut ConnectionService,
    credentials: &CredentialService,
    id: Uuid,
) -> CoreResult<()> {
    let profile = service.get_profile(id)?;
    service.delete_profile(id)?;
    match credentials.delete(&CredentialRef::for_profile(&profile)) {
        Ok(()) | Err(CoreError::CredentialMissing) | Err(CoreError::NotFound(_)) => Ok(()),
        Err(error) => Err(error),
    }
}

pub async fn connect(
    service: &mut ConnectionService,
    credentials: &CredentialService,
    profile_id: Uuid,
    supplied: Option<Secret>,
) -> CoreResult<ActiveConnection> {
    let profile = service.get_profile(profile_id)?;
    let secret = credential_for(credentials, &profile, supplied)?;
    service.connect(profile_id, secret).await
}

pub async fn test(
    service: &mut ConnectionService,
    credentials: &CredentialService,
    profile_id: Uuid,
    supplied: Option<Secret>,
) -> CoreResult<()> {
    let profile = service.get_profile(profile_id)?;
    let secret = credential_for(credentials, &profile, supplied)?;
    service.test(profile_id, secret).await
}

/// Persist an explicit host-key trust decision for an SFTP profile.
pub fn trust_host_key(
    service: &ConnectionService,
    profile_id: Uuid,
    fingerprint: &str,
    store_path: &Path,
) -> CoreResult<()> {
    let profile = service.get_profile(profile_id)?;
    if profile.protocol != Protocol::Sftp {
        return Err(CoreError::validation(
            "host-key trust applies only to SFTP profiles",
        ));
    }
    thriftp_protocols::trust_fingerprint(store_path, &profile.host, profile.port, fingerprint)
}

#[tauri::command]
pub async fn connections_list(
    state: State<'_, AppState>,
) -> Result<Vec<ConnectionProfile>, DesktopError> {
    let service = state.connections.lock().await;
    Ok(list(&service))
}

#[tauri::command]
pub async fn connections_create(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: ConnectionProfile,
    secret: Option<String>,
) -> Result<ConnectionProfile, DesktopError> {
    let result = {
        let mut service = state.connections.lock().await;
        create(
            &mut service,
            &state.credentials,
            profile,
            secret_from(secret),
        )
        .map_err(DesktopError::from)
    }?;
    state.persist_metadata().await.map_err(DesktopError::from)?;
    state
        .server_favorites
        .ensure_favorites()
        .map_err(DesktopError::from)?;
    if let Some(favorite_id) = state
        .server_favorites
        .favorite_id_for_profile(result.id)
        .map_err(DesktopError::from)?
    {
        emit_favorite(&app, favorite_id, "created");
    }
    Ok(result)
}

#[tauri::command]
pub async fn connections_update(
    app: AppHandle,
    state: State<'_, AppState>,
    profile: ConnectionProfile,
    secret: Option<String>,
) -> Result<ConnectionProfile, DesktopError> {
    if let Some(favorite_id) = state
        .server_favorites
        .favorite_id_for_profile(profile.id)
        .map_err(DesktopError::from)?
    {
        let existing = state
            .server_favorites
            .get(favorite_id)
            .map_err(DesktopError::from)?;
        let policy = if secret.is_some() || existing.has_saved_credential {
            CredentialPolicy::Save
        } else {
            CredentialPolicy::Ask
        };
        let profile_id = profile.id;
        let mut input: ServerFavoriteInput = profile.into();
        input.local_path = existing.local_path.clone();
        input.remote_path = existing.remote_path.clone();
        state
            .server_favorites
            .update(
                favorite_id,
                input,
                policy,
                None,
                secret_from(secret),
                &state.credentials,
            )
            .map_err(DesktopError::from)?;
        refresh_profiles(&state).await.map_err(DesktopError::from)?;
        emit_favorite(&app, favorite_id, "updated");
        let service = state.connections.lock().await;
        return service.get_profile(profile_id).map_err(DesktopError::from);
    }

    let result = {
        let mut service = state.connections.lock().await;
        update(
            &mut service,
            &state.credentials,
            profile,
            secret_from(secret),
        )
        .map_err(DesktopError::from)
    }?;
    state.persist_metadata().await.map_err(DesktopError::from)?;
    state
        .server_favorites
        .ensure_favorites()
        .map_err(DesktopError::from)?;
    Ok(result)
}

#[tauri::command]
pub async fn connections_get_profile(
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<ActiveProfileView, DesktopError> {
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let profile_id = {
        let service = state.connections.lock().await;
        service
            .active_connections()
            .into_iter()
            .find(|connection| connection.id == connection_id)
            .map(|connection| connection.connection_profile_id)
            .ok_or_else(|| {
                DesktopError::from(CoreError::NotFound("active connection".to_owned()))
            })?
    };
    let favorite_id = state
        .server_favorites
        .favorite_id_for_profile(profile_id)
        .map_err(DesktopError::from)?;
    let profile = {
        let service = state.connections.lock().await;
        service
            .get_profile(profile_id)
            .map_err(DesktopError::from)?
    };
    Ok(ActiveProfileView {
        connection_id: connection_id.to_string(),
        favorite_id: favorite_id.map(|id| id.to_string()),
        profile,
    })
}

#[tauri::command]
pub async fn connections_delete(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<DeletedView, DesktopError> {
    let id = parse_id(&id).map_err(DesktopError::from)?;
    let favorite_id = state
        .server_favorites
        .favorite_id_for_profile(id)
        .map_err(DesktopError::from)?;
    if let Some(favorite_id) = favorite_id {
        let credential_ref_id = state
            .metadata_store
            .load()
            .map_err(DesktopError::from)?
            .server_favorites
            .iter()
            .find(|favorite| favorite.id == favorite_id)
            .and_then(|favorite| favorite.credential_ref_id);
        let impact = state
            .server_favorites
            .delete(favorite_id)
            .map_err(DesktopError::from)?;
        {
            let mut service = state.connections.lock().await;
            service.delete_profile(id).map_err(DesktopError::from)?;
        }
        crate::commands::server_favorites::remove_profile_dependents(&state, id).await;
        state.persist_metadata().await.map_err(DesktopError::from)?;
        emit_favorite(&app, favorite_id, "deleted");
        if let Some(reference) = credential_ref_id
            && impact.affected_favorites.is_empty()
        {
            emit_credential_deleted(&app, reference);
        }
    } else {
        {
            let mut service = state.connections.lock().await;
            delete(&mut service, &state.credentials, id)?;
        }
        state.persist_metadata().await.map_err(DesktopError::from)?;
    }
    Ok(DeletedView { deleted: true })
}

#[tauri::command]
pub async fn connections_connect(
    app: AppHandle,
    state: State<'_, AppState>,
    profile_id: String,
    secret: Option<String>,
) -> Result<ConnectionView, DesktopError> {
    let profile_id = parse_id(&profile_id).map_err(DesktopError::from)?;
    let connection = {
        let mut service = state.connections.lock().await;
        connect(
            &mut service,
            &state.credentials,
            profile_id,
            secret_from(secret),
        )
        .await?
    };
    let _ = state.persist_metadata().await;
    let mut view = ConnectionView::from(connection);
    view.favorite_id = state
        .server_favorites
        .favorite_id_for_profile(profile_id)
        .map_err(DesktopError::from)?
        .map(|id| id.to_string());
    let profile = {
        let service = state.connections.lock().await;
        service.get_profile(profile_id).map_err(DesktopError::from)?
    };
    ConnectionHistoryService::new(state.metadata_store.clone())
        .record_profile(
            &profile,
            view.favorite_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok()),
        )
        .map_err(DesktopError::from)?;
    let _ = app.emit("connection:state", view.clone());
    Ok(view)
}

#[tauri::command]
pub async fn connections_trust_host_key(
    state: State<'_, AppState>,
    profile_id: String,
    fingerprint: String,
) -> Result<TrustedHostKeyView, DesktopError> {
    let profile_id = parse_id(&profile_id).map_err(DesktopError::from)?;
    let service = state.connections.lock().await;
    trust_host_key(
        &service,
        profile_id,
        &fingerprint,
        &thriftp_protocols::known_hosts_store_path(),
    )
    .map_err(DesktopError::from)?;
    Ok(TrustedHostKeyView {
        trusted: true,
        fingerprint: thriftp_protocols::normalize_fingerprint(&fingerprint),
    })
}

#[tauri::command]
pub async fn connections_reconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: String,
    secret: Option<String>,
) -> Result<ConnectionView, DesktopError> {
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let (profile_id, connection) = {
        let mut service = state.connections.lock().await;
        let profile_id = service
            .active_connections()
            .into_iter()
            .find(|connection| connection.id == connection_id)
            .map(|connection| connection.connection_profile_id)
            .ok_or_else(|| {
                DesktopError::from(CoreError::NotFound(
                    "active connection could not be found".to_owned(),
                ))
            })?;
        let _ = service.disconnect(connection_id).await;
        let connection = connect(
            &mut service,
            &state.credentials,
            profile_id,
            secret_from(secret),
        )
        .await?;
        (profile_id, connection)
    };
    let _ = state.persist_metadata().await;
    let mut view = ConnectionView::from(connection);
    view.favorite_id = state
        .server_favorites
        .favorite_id_for_profile(profile_id)
        .map_err(DesktopError::from)?
        .map(|id| id.to_string());
    let profile = {
        let service = state.connections.lock().await;
        service.get_profile(profile_id).map_err(DesktopError::from)?
    };
    ConnectionHistoryService::new(state.metadata_store.clone())
        .record_profile(
            &profile,
            view.favorite_id
                .as_deref()
                .and_then(|id| Uuid::parse_str(id).ok()),
        )
        .map_err(DesktopError::from)?;
    let _ = app.emit("connection:state", view.clone());
    Ok(view)
}

#[tauri::command]
pub async fn connections_disconnect(
    app: AppHandle,
    state: State<'_, AppState>,
    connection_id: String,
) -> Result<DisconnectedView, DesktopError> {
    let connection_id = parse_id(&connection_id).map_err(DesktopError::from)?;
    let previous = {
        let service = state.connections.lock().await;
        service
            .active_connections()
            .into_iter()
            .find(|connection| connection.id == connection_id)
    };
    {
        let mut service = state.connections.lock().await;
        service
            .disconnect(connection_id)
            .await
            .map_err(DesktopError::from)?;
    }
    if let Some(previous) = previous {
        let mut view = ConnectionView::from(previous.clone());
        view.state = "disconnected".to_owned();
        view.favorite_id = state
            .server_favorites
            .favorite_id_for_profile(previous.connection_profile_id)
            .map_err(DesktopError::from)?
            .map(|id| id.to_string());
        let _ = app.emit("connection:state", view);
    }
    Ok(DisconnectedView { disconnected: true })
}

#[tauri::command]
pub async fn connections_test(
    state: State<'_, AppState>,
    profile_id: String,
    secret: Option<String>,
) -> Result<ConnectionTestView, DesktopError> {
    let profile_id = parse_id(&profile_id).map_err(DesktopError::from)?;
    let mut service = state.connections.lock().await;
    match test(
        &mut service,
        &state.credentials,
        profile_id,
        secret_from(secret),
    )
    .await
    {
        Ok(()) => Ok(ConnectionTestView {
            ok: true,
            error: None,
        }),
        Err(error) if matches!(error, CoreError::Validation(_) | CoreError::NotFound(_)) => {
            Err(DesktopError::from(error))
        }
        Err(error) => Ok(ConnectionTestView {
            ok: false,
            error: Some(DesktopError::from(error)),
        }),
    }
}
