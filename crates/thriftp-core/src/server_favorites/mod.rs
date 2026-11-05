//! Server-favorites application service.
//!
//! The service is Tauri-independent and reuses the existing connection-profile
//! and credential models. The existing `favorites` module remains the separate
//! directory-favorites feature.

#![deny(missing_docs)]

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::credentials::{CredentialService, Secret};
use crate::error::{CoreError, CoreResult};
use crate::model::{
    AuthMethod, ConnectionProfile, CredentialPolicy, CredentialRef, FavoritePreferences, Protocol,
    ServerFavorite,
};
use crate::repository::{AppState, MetadataStore};
use crate::validation::{identity_matches, validate_profile};

pub mod sorting;

/// Non-secret connection settings supplied when creating or editing a favorite.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFavoriteInput {
    /// User-visible favorite name.
    pub display_name: String,
    /// Optional one-level group label.
    #[serde(default)]
    pub group_label: Option<String>,
    /// Transport protocol.
    pub protocol: Protocol,
    /// Server host name or address.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Login user name.
    pub username: String,
    /// Authentication method.
    pub auth_method: AuthMethod,
    /// Connection timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    /// Protected reference to an SSH private-key file.
    #[serde(default)]
    pub ssh_key_ref: Option<String>,
    /// Whether the favorite is pinned above unpinned entries.
    #[serde(default)]
    pub pinned: bool,
}

fn default_timeout() -> u32 {
    30
}

impl From<ConnectionProfile> for ServerFavoriteInput {
    fn from(profile: ConnectionProfile) -> Self {
        Self {
            display_name: profile.display_name,
            group_label: profile.group_label,
            protocol: profile.protocol,
            host: profile.host,
            port: profile.port,
            username: profile.username,
            auth_method: profile.auth_method,
            timeout_seconds: profile.timeout_seconds,
            ssh_key_ref: profile.ssh_key_ref,
            pinned: false,
        }
    }
}

impl ServerFavoriteInput {
    /// Convert the non-secret settings into a connection profile.
    #[tracing::instrument(skip_all)]
    pub fn into_profile(self) -> ConnectionProfile {
        let mut profile = ConnectionProfile::new(
            self.display_name,
            self.protocol,
            self.host,
            self.port,
            self.username,
            self.auth_method,
            self.timeout_seconds,
        );
        profile.group_label = self.group_label;
        profile.ssh_key_ref = self.ssh_key_ref;
        profile
    }
}

/// Redacted, UI-facing projection of a server favorite.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFavoriteView {
    /// Favorite identifier.
    pub id: Uuid,
    /// Identifier of the underlying connection profile.
    pub connection_profile_id: Uuid,
    /// User-visible favorite name.
    pub display_name: String,
    /// Optional group label.
    pub group_label: Option<String>,
    /// Transport protocol.
    pub protocol: Protocol,
    /// Server host name or address.
    pub host: String,
    /// Server port.
    pub port: u16,
    /// Login user name.
    pub username: String,
    /// Authentication method.
    pub auth_method: AuthMethod,
    /// Connection timeout in seconds.
    pub timeout_seconds: u32,
    /// Protected reference to an SSH private-key file.
    pub ssh_key_ref: Option<String>,
    /// Whether the favorite is pinned.
    pub pinned: bool,
    /// Last successful connection time.
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether credentials are saved or requested at connect time.
    pub credential_policy: CredentialPolicy,
    /// Whether a saved credential reference exists.
    pub has_saved_credential: bool,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Non-fatal credential warning produced while saving.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_warning: Option<CredentialWarning>,
}

/// Non-fatal warning about credential persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialWarning {
    /// Stable warning code.
    pub code: String,
    /// User-facing warning message.
    pub message: String,
}

/// Result of changing or deleting a shared credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialImpact {
    /// Credential reference affected by the operation.
    pub credential_ref_id: Uuid,
    /// Favorites that reference the credential.
    pub affected_favorites: Vec<AffectedFavorite>,
    /// Whether more than one favorite references the credential.
    pub is_shared: bool,
}

/// A favorite affected by a credential change.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedFavorite {
    /// Favorite identifier.
    pub id: Uuid,
    /// User-visible favorite name.
    pub display_name: String,
}

/// A credential reference that may be explicitly reused.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReusableCredential {
    /// Credential reference identifier.
    pub credential_ref_id: Uuid,
    /// Transport protocol of the stored identity.
    pub protocol: Protocol,
    /// Host of the stored identity.
    pub host: String,
    /// Port of the stored identity.
    pub port: u16,
    /// Username of the stored identity.
    pub username: String,
    /// Authentication method of the stored identity.
    pub auth_method: AuthMethod,
    /// Number of favorites currently referencing the credential.
    pub affected_favorite_count: usize,
}

/// Application service for creating, organizing, connecting, and editing
/// server favorites.
pub struct ServerFavoriteService {
    store: Arc<dyn MetadataStore>,
}

impl ServerFavoriteService {
    #[tracing::instrument(skip_all)]
    /// Create a service backed by the supplied metadata store.
    pub fn new(store: Arc<dyn MetadataStore>) -> Self {
        Self { store }
    }

    #[tracing::instrument(skip_all)]
    /// Return favorites ordered by the persisted preferences.
    pub fn list(&self) -> CoreResult<Vec<ServerFavoriteView>> {
        let state = self.store.load()?;
        let mut views = build_views(&state);
        sort_views(&mut views, &state.favorite_preferences);
        Ok(views)
    }

    #[tracing::instrument(skip_all)]
    /// Find the favorite linked to a connection profile.
    pub fn favorite_id_for_profile(&self, profile_id: Uuid) -> CoreResult<Option<Uuid>> {
        let state = self.store.load()?;
        Ok(state
            .server_favorites
            .iter()
            .find(|favorite| favorite.connection_profile_id == profile_id)
            .map(|favorite| favorite.id))
    }

    /// Return one favorite or a not-found error.
    pub fn get(&self, id: Uuid) -> CoreResult<ServerFavoriteView> {
        let state = self.store.load()?;
        build_views(&state)
            .into_iter()
            .find(|view| view.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))
    }

    #[tracing::instrument(skip_all)]
    /// Create a favorite, validating settings and binding credentials.
    pub fn create(
        &self,
        input: ServerFavoriteInput,
        policy: CredentialPolicy,
        credential_ref_id: Option<Uuid>,
        secret: Option<Secret>,
        credentials: &CredentialService,
    ) -> CoreResult<ServerFavoriteView> {
        let mut state = self.store.load()?;
        let pinned = input.pinned;
        let profile = input.into_profile();
        validate_profile(&profile)?;
        if state
            .connection_profiles
            .iter()
            .any(|existing| existing.id == profile.id)
        {
            return Err(CoreError::Conflict("profile already exists".to_owned()));
        }
        let (reference, warning, effective_policy) = match resolve_reference(
            &mut state,
            &profile,
            policy,
            credential_ref_id,
            secret,
            credentials,
        ) {
            Ok(reference) => (reference, None, policy),
            Err(CoreError::CredentialStoreUnavailable(_)) if policy == CredentialPolicy::Save => (
                None,
                Some(CredentialWarning {
                    code: "credential_store_unavailable".to_owned(),
                    message: "The credential store is unavailable. The favorite was saved without credentials.".to_owned(),
                }),
                CredentialPolicy::Ask,
            ),
            Err(error) => return Err(error),
        };
        let mut favorite = ServerFavorite::new(
            profile.id,
            effective_policy,
            reference.as_ref().map(|item| item.id),
        );
        favorite.pinned = pinned;
        state.connection_profiles.push(profile.clone());
        state.server_favorites.push(favorite);
        self.store.save(&state)?;
        let mut view = view_for(&state, profile.id).expect("created favorite");
        view.credential_warning = warning;
        Ok(view)
    }

    /// Ensure every saved connection profile has exactly one server favorite.
    ///
    /// Existing profiles created by the desktop connection editor, CLI, or MCP
    /// are backfilled so the favorites view is the single management surface.
    /// Duplicate favorite rows are collapsed deterministically.
    pub fn ensure_favorites(&self) -> CoreResult<usize> {
        let mut state = self.store.load()?;
        let mut seen = std::collections::HashSet::new();
        let before = state.server_favorites.len();
        let profile_ids: std::collections::HashSet<_> = state
            .connection_profiles
            .iter()
            .map(|profile| profile.id)
            .collect();
        state.server_favorites.retain(|favorite| {
            profile_ids.contains(&favorite.connection_profile_id)
                && seen.insert(favorite.connection_profile_id)
        });
        let mut created = 0;
        for profile in state.connection_profiles.clone() {
            if state
                .server_favorites
                .iter()
                .any(|favorite| favorite.connection_profile_id == profile.id)
            {
                continue;
            }
            let derived = CredentialRef::for_profile(&profile);
            let reference = state
                .credential_refs
                .iter()
                .find(|reference| {
                    reference.id == derived.id && identity_matches(&profile, reference)
                })
                .cloned();
            let policy = if reference.is_some() {
                CredentialPolicy::Save
            } else {
                CredentialPolicy::Ask
            };
            state.server_favorites.push(ServerFavorite::new(
                profile.id,
                policy,
                reference.map(|reference| reference.id),
            ));
            created += 1;
        }
        if created > 0 || state.server_favorites.len() != before {
            self.store.save(&state)?;
        }
        Ok(created)
    }

    /// Invalidate credential references whose bound identity no longer
    /// matches the current connection profile.
    ///
    /// This covers profiles changed outside the favorites editor (for example
    /// by the CLI) and removes unshared secrets from the native store.
    pub fn reconcile_profiles(&self, credentials: &CredentialService) -> CoreResult<usize> {
        let mut state = self.store.load()?;
        let mut invalidated = Vec::new();
        for index in 0..state.server_favorites.len() {
            let profile_id = state.server_favorites[index].connection_profile_id;
            let Some(profile) = state
                .connection_profiles
                .iter()
                .find(|profile| profile.id == profile_id)
                .cloned()
            else {
                continue;
            };
            let Some(ref_id) = state.server_favorites[index].credential_ref_id else {
                continue;
            };
            let matches = state
                .credential_refs
                .iter()
                .find(|reference| reference.id == ref_id)
                .is_some_and(|reference| identity_matches(&profile, reference));
            if !matches {
                state.server_favorites[index].credential_policy = CredentialPolicy::Ask;
                state.server_favorites[index].credential_ref_id = None;
                state.server_favorites[index].updated_at = Utc::now();
                invalidated.push(ref_id);
            }
        }
        for ref_id in &invalidated {
            let still_used = state
                .server_favorites
                .iter()
                .any(|favorite| favorite.credential_ref_id == Some(*ref_id));
            if !still_used {
                if let Some(reference) =
                    state.credential_refs.iter().find(|item| item.id == *ref_id)
                {
                    let _ = credentials.delete(reference);
                }
                state.credential_refs.retain(|item| item.id != *ref_id);
            }
        }
        if !invalidated.is_empty() {
            self.store.save(&state)?;
        }
        Ok(invalidated.len())
    }

    /// Reconcile a connection-profile edit with its favorite metadata.
    ///
    /// Identity changes invalidate the favorite's credential reference without
    /// disturbing credentials still referenced by other favorites.
    pub fn sync_profile(
        &self,
        profile: ConnectionProfile,
        credentials: &CredentialService,
    ) -> CoreResult<ServerFavoriteView> {
        let mut state = self.store.load()?;
        let favorite_index = state
            .server_favorites
            .iter()
            .position(|favorite| favorite.connection_profile_id == profile.id);
        let Some(favorite_index) = favorite_index else {
            self.ensure_favorites()?;
            let state = self.store.load()?;
            return Ok(view_for(&state, profile.id).expect("backfilled favorite"));
        };
        let old_profile = state
            .connection_profiles
            .iter()
            .find(|old| old.id == profile.id)
            .cloned();
        if let Some(old_profile) = old_profile
            && !identity_matches(&profile, &CredentialRef::for_profile(&old_profile))
        {
            let ref_id = state.server_favorites[favorite_index].credential_ref_id;
            if let Some(ref_id) = ref_id {
                state.server_favorites[favorite_index].credential_policy = CredentialPolicy::Ask;
                state.server_favorites[favorite_index].credential_ref_id = None;
                let still_used =
                    state
                        .server_favorites
                        .iter()
                        .enumerate()
                        .any(|(index, favorite)| {
                            index != favorite_index && favorite.credential_ref_id == Some(ref_id)
                        });
                if !still_used {
                    if let Some(reference) =
                        state.credential_refs.iter().find(|item| item.id == ref_id)
                    {
                        let _ = credentials.delete(reference);
                    }
                    state.credential_refs.retain(|item| item.id != ref_id);
                }
            }
        }
        state
            .connection_profiles
            .retain(|existing| existing.id != profile.id);
        state.connection_profiles.push(profile.clone());
        state.server_favorites[favorite_index].updated_at = Utc::now();
        self.store.save(&state)?;
        Ok(view_for(&state, profile.id).expect("synced favorite"))
    }

    #[tracing::instrument(skip_all)]
    /// Update a favorite, invalidating credentials when identity changes.
    pub fn update(
        &self,
        id: Uuid,
        input: ServerFavoriteInput,
        policy: CredentialPolicy,
        credential_ref_id: Option<Uuid>,
        secret: Option<Secret>,
        credentials: &CredentialService,
    ) -> CoreResult<ServerFavoriteView> {
        let mut state = self.store.load()?;
        let favorite_index = state
            .server_favorites
            .iter()
            .position(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        let old_profile_id = state.server_favorites[favorite_index].connection_profile_id;
        let old_profile = state
            .connection_profiles
            .iter()
            .find(|profile| profile.id == old_profile_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound("connection profile".to_owned()))?;
        let mut profile = input.into_profile();
        profile.id = old_profile.id;
        profile.created_at = old_profile.created_at;
        profile.updated_at = Utc::now();
        profile.last_used_at = old_profile.last_used_at;
        validate_profile(&profile)?;

        let identity_changed =
            !identity_matches(&profile, &CredentialRef::for_profile(&old_profile));
        let existing_ref = state.server_favorites[favorite_index]
            .credential_ref_id
            .and_then(|ref_id| {
                state
                    .credential_refs
                    .iter()
                    .find(|item| item.id == ref_id)
                    .cloned()
            });
        let mut effective_policy = policy;
        let mut warning = None;
        let reference = if policy == CredentialPolicy::Ask {
            if credential_ref_id.is_some() || secret.is_some() {
                return Err(CoreError::validation(
                    "credential policy ask does not accept a secret or reference",
                ));
            }
            None
        } else if let Some(ref_id) = credential_ref_id {
            resolve_reference(
                &mut state,
                &profile,
                policy,
                Some(ref_id),
                None,
                credentials,
            )?
        } else if !identity_changed {
            if let Some(existing) = existing_ref.clone() {
                if let Some(secret) = secret {
                    match credentials.set(&existing, secret) {
                        Ok(()) => Some(existing),
                        Err(CoreError::CredentialStoreUnavailable(_)) => {
                            warning = Some(CredentialWarning {
                                code: "credential_store_unavailable".to_owned(),
                                message: "The credential store is unavailable. The saved credential was not updated.".to_owned(),
                            });
                            Some(existing)
                        }
                        Err(error) => return Err(error),
                    }
                } else {
                    Some(existing)
                }
            } else {
                match resolve_reference(&mut state, &profile, policy, None, secret, credentials) {
                    Ok(reference) => reference,
                    Err(CoreError::CredentialStoreUnavailable(_)) => {
                        warning = Some(CredentialWarning {
                            code: "credential_store_unavailable".to_owned(),
                            message: "The credential store is unavailable. The favorite was saved without credentials.".to_owned(),
                        });
                        effective_policy = CredentialPolicy::Ask;
                        None
                    }
                    Err(error) => return Err(error),
                }
            }
        } else {
            match resolve_reference(&mut state, &profile, policy, None, secret, credentials) {
                Ok(reference) => reference,
                Err(CoreError::CredentialStoreUnavailable(_)) => {
                    warning = Some(CredentialWarning {
                        code: "credential_store_unavailable".to_owned(),
                        message: "The credential store is unavailable. The favorite was saved without credentials.".to_owned(),
                    });
                    effective_policy = CredentialPolicy::Ask;
                    None
                }
                Err(error) => return Err(error),
            }
        };

        if let Some(old_ref) = existing_ref
            && reference.as_ref().map(|item| item.id) != Some(old_ref.id)
        {
            let still_used = state
                .server_favorites
                .iter()
                .enumerate()
                .any(|(index, favorite)| {
                    index != favorite_index && favorite.credential_ref_id == Some(old_ref.id)
                });
            if !still_used {
                let _ = credentials.delete(&old_ref);
                state.credential_refs.retain(|item| item.id != old_ref.id);
            }
        }
        state
            .connection_profiles
            .retain(|item| item.id != old_profile_id);
        state.connection_profiles.push(profile.clone());
        let favorite = &mut state.server_favorites[favorite_index];
        favorite.connection_profile_id = profile.id;
        favorite.credential_policy = effective_policy;
        favorite.credential_ref_id = reference.as_ref().map(|item| item.id);
        favorite.updated_at = Utc::now();
        self.store.save(&state)?;
        let mut view = view_for(&state, profile.id).expect("updated favorite");
        view.credential_warning = warning;
        Ok(view)
    }

    #[tracing::instrument(skip_all)]
    /// Delete a favorite and clean dependent directory favorites and tabs.
    pub fn delete(&self, id: Uuid) -> CoreResult<CredentialImpact> {
        let mut state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter()
            .find(|favorite| favorite.id == id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        state.server_favorites.retain(|item| item.id != id);
        state
            .connection_profiles
            .retain(|profile| profile.id != favorite.connection_profile_id);
        state
            .directory_favorites
            .retain(|item| item.connection_profile_id != favorite.connection_profile_id);
        state
            .tabs
            .retain(|tab| tab.connection_profile_id != Some(favorite.connection_profile_id));
        let impact = favorite
            .credential_ref_id
            .map(|ref_id| impact_for(&state, ref_id))
            .transpose()?
            .unwrap_or(CredentialImpact {
                credential_ref_id: Uuid::nil(),
                affected_favorites: Vec::new(),
                is_shared: false,
            });
        self.store.save(&state)?;
        Ok(impact)
    }

    #[tracing::instrument(skip_all)]
    /// Pin or unpin a favorite.
    pub fn set_pinned(&self, id: Uuid, pinned: bool) -> CoreResult<ServerFavoriteView> {
        let mut state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter_mut()
            .find(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        favorite.pinned = pinned;
        favorite.updated_at = Utc::now();
        let profile_id = favorite.connection_profile_id;
        self.store.save(&state)?;
        Ok(view_for(&state, profile_id).expect("pinned favorite"))
    }

    #[tracing::instrument(skip_all)]
    /// Assign or clear a favorite group label.
    pub fn set_group(&self, id: Uuid, group: Option<String>) -> CoreResult<ServerFavoriteView> {
        let mut state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter()
            .find(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        let profile_id = favorite.connection_profile_id;
        let profile = state
            .connection_profiles
            .iter_mut()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| CoreError::NotFound("connection profile".to_owned()))?;
        profile.group_label = group;
        profile.updated_at = Utc::now();
        validate_profile(profile)?;
        self.store.save(&state)?;
        Ok(view_for(&state, profile_id).expect("grouped favorite"))
    }

    #[tracing::instrument(skip_all)]
    /// Rename a group label across all favorites that use it.
    pub fn rename_group(
        &self,
        old_label: &str,
        new_label: &str,
    ) -> CoreResult<Vec<ServerFavoriteView>> {
        let new_label = new_label.trim();
        if new_label.is_empty() || new_label.chars().count() > 100 {
            return Err(CoreError::validation(
                "new group label must be 1-100 characters",
            ));
        }
        let mut state = self.store.load()?;
        for profile in &mut state.connection_profiles {
            if profile.group_label.as_deref() == Some(old_label) {
                profile.group_label = Some(new_label.to_owned());
                profile.updated_at = Utc::now();
            }
        }
        let views: Vec<_> = state
            .server_favorites
            .iter()
            .filter_map(|favorite| view_for(&state, favorite.connection_profile_id))
            .collect();
        self.store.save(&state)?;
        Ok(views)
    }

    #[tracing::instrument(skip_all)]
    /// Resolve the profile and optional stored credential for a favorite.
    #[tracing::instrument(skip_all)]
    /// Resolve the profile and optional saved credential for a favorite.
    pub fn connection_target(
        &self,
        id: Uuid,
        credentials: &CredentialService,
    ) -> CoreResult<(ConnectionProfile, Option<Secret>)> {
        let state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter()
            .find(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        let profile = state
            .connection_profiles
            .iter()
            .find(|profile| profile.id == favorite.connection_profile_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound("connection profile".to_owned()))?;
        let secret = match favorite
            .credential_ref_id
            .and_then(|ref_id| state.credential_refs.iter().find(|item| item.id == ref_id))
        {
            Some(reference) if identity_matches(&profile, reference) => {
                credentials.get(reference)?
            }
            _ => None,
        };
        Ok((profile, secret))
    }

    /// Record a successful connection time for a favorite.
    pub fn mark_used(&self, id: Uuid) -> CoreResult<()> {
        let mut state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter_mut()
            .find(|favorite| favorite.id == id)
            .ok_or_else(|| CoreError::NotFound(format!("server favorite {id}")))?;
        favorite.last_used_at = Some(Utc::now());
        favorite.updated_at = Utc::now();
        self.store.save(&state)
    }

    #[tracing::instrument(skip_all)]
    /// Return the persisted favorite sort preferences.
    pub fn preferences(&self) -> CoreResult<FavoritePreferences> {
        Ok(self.store.load()?.favorite_preferences)
    }

    #[tracing::instrument(skip_all)]
    /// Persist favorite sort preferences.
    pub fn set_preferences(
        &self,
        preferences: FavoritePreferences,
    ) -> CoreResult<FavoritePreferences> {
        let mut state = self.store.load()?;
        state.favorite_preferences = preferences.clone();
        self.store.save(&state)?;
        Ok(preferences)
    }

    #[tracing::instrument(skip_all)]
    /// Describe the favorites affected by a favorite credential change.
    pub fn impact(&self, favorite_id: Uuid) -> CoreResult<CredentialImpact> {
        let state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter()
            .find(|favorite| favorite.id == favorite_id)
            .ok_or_else(|| CoreError::NotFound("server favorite".to_owned()))?;
        let ref_id = favorite
            .credential_ref_id
            .ok_or_else(|| CoreError::NotFound("credential reference".to_owned()))?;
        impact_for(&state, ref_id)
    }

    #[tracing::instrument(skip_all)]
    /// List credential references available for explicit reuse.
    pub fn reusable(&self) -> CoreResult<Vec<ReusableCredential>> {
        let state = self.store.load()?;
        Ok(state
            .credential_refs
            .iter()
            .map(|reference| ReusableCredential {
                credential_ref_id: reference.id,
                protocol: reference.protocol,
                host: reference.host.clone(),
                port: reference.port,
                username: reference.username.clone(),
                auth_method: reference.auth_method,
                affected_favorite_count: state
                    .server_favorites
                    .iter()
                    .filter(|favorite| favorite.credential_ref_id == Some(reference.id))
                    .count(),
            })
            .collect())
    }

    #[tracing::instrument(skip_all)]
    /// Remove a favorite credential reference and prompt-safe impact.
    pub fn remove_credential(
        &self,
        favorite_id: Uuid,
        credentials: &CredentialService,
    ) -> CoreResult<CredentialImpact> {
        let mut state = self.store.load()?;
        let favorite = state
            .server_favorites
            .iter_mut()
            .find(|favorite| favorite.id == favorite_id)
            .ok_or_else(|| CoreError::NotFound("server favorite".to_owned()))?;
        let ref_id = favorite
            .credential_ref_id
            .ok_or_else(|| CoreError::NotFound("credential reference".to_owned()))?;
        let impact = impact_for(&state, ref_id)?;
        let favorite = state
            .server_favorites
            .iter_mut()
            .find(|favorite| favorite.id == favorite_id)
            .ok_or_else(|| CoreError::NotFound("server favorite".to_owned()))?;
        favorite.credential_policy = CredentialPolicy::Ask;
        favorite.credential_ref_id = None;
        favorite.updated_at = Utc::now();
        if !impact.is_shared {
            if let Some(reference) = state.credential_refs.iter().find(|item| item.id == ref_id) {
                let _ = credentials.delete(reference);
            }
            state.credential_refs.retain(|item| item.id != ref_id);
        }
        self.store.save(&state)?;
        Ok(impact)
    }

    #[tracing::instrument(skip_all)]
    /// Delete an unreferenced credential after explicit confirmation.
    pub fn delete_credential(
        &self,
        ref_id: Uuid,
        confirmed: bool,
        credentials: &CredentialService,
    ) -> CoreResult<()> {
        if !confirmed {
            return Err(CoreError::Conflict(
                "credential deletion requires confirmation".to_owned(),
            ));
        }
        let mut state = self.store.load()?;
        if state
            .server_favorites
            .iter()
            .any(|favorite| favorite.credential_ref_id == Some(ref_id))
        {
            return Err(CoreError::Conflict("credential is still in use".to_owned()));
        }
        if let Some(reference) = state.credential_refs.iter().find(|item| item.id == ref_id) {
            let _ = credentials.delete(reference);
        }
        state.credential_refs.retain(|item| item.id != ref_id);
        self.store.save(&state)?;
        Ok(())
    }
}

fn resolve_reference(
    state: &mut AppState,
    profile: &ConnectionProfile,
    policy: CredentialPolicy,
    credential_ref_id: Option<Uuid>,
    secret: Option<Secret>,
    credentials: &CredentialService,
) -> CoreResult<Option<CredentialRef>> {
    if policy == CredentialPolicy::Ask {
        if credential_ref_id.is_some() || secret.is_some() {
            return Err(CoreError::validation(
                "credential policy ask does not accept a secret or reference",
            ));
        }
        return Ok(None);
    }
    if let Some(ref_id) = credential_ref_id {
        let reference = state
            .credential_refs
            .iter()
            .find(|item| item.id == ref_id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound("credential reference".to_owned()))?;
        if !identity_matches(profile, &reference) {
            return Err(CoreError::Conflict(
                "credential reference identity does not match the profile".to_owned(),
            ));
        }
        return Ok(Some(reference));
    }
    let secret = secret.ok_or(CoreError::CredentialMissing)?;
    let reference = CredentialRef::for_profile(profile);
    credentials.set(&reference, secret)?;
    if !state
        .credential_refs
        .iter()
        .any(|item| item.id == reference.id)
    {
        state.credential_refs.push(reference.clone());
    }
    Ok(Some(reference))
}

fn build_views(state: &AppState) -> Vec<ServerFavoriteView> {
    state
        .server_favorites
        .iter()
        .filter_map(|favorite| view_for(state, favorite.connection_profile_id))
        .collect()
}

fn view_for(state: &AppState, profile_id: Uuid) -> Option<ServerFavoriteView> {
    let favorite = state
        .server_favorites
        .iter()
        .find(|favorite| favorite.connection_profile_id == profile_id)?;
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == profile_id)?;
    Some(ServerFavoriteView {
        id: favorite.id,
        connection_profile_id: profile.id,
        display_name: profile.display_name.clone(),
        group_label: profile.group_label.clone(),
        protocol: profile.protocol,
        host: profile.host.clone(),
        port: profile.port,
        username: profile.username.clone(),
        auth_method: profile.auth_method,
        timeout_seconds: profile.timeout_seconds,
        ssh_key_ref: profile.ssh_key_ref.clone(),
        pinned: favorite.pinned,
        last_used_at: favorite.last_used_at,
        credential_policy: favorite.credential_policy,
        has_saved_credential: favorite.credential_policy == CredentialPolicy::Save
            && favorite.credential_ref_id.is_some(),
        created_at: favorite.created_at,
        updated_at: favorite.updated_at,
        credential_warning: None,
    })
}

fn sort_views(views: &mut [ServerFavoriteView], preferences: &FavoritePreferences) {
    sorting::sort_views(views, preferences);
}

fn impact_for(state: &AppState, ref_id: Uuid) -> CoreResult<CredentialImpact> {
    let affected: Vec<_> = state
        .server_favorites
        .iter()
        .filter(|favorite| favorite.credential_ref_id == Some(ref_id))
        .filter_map(|favorite| {
            let profile = state
                .connection_profiles
                .iter()
                .find(|profile| profile.id == favorite.connection_profile_id)?;
            Some(AffectedFavorite {
                id: favorite.id,
                display_name: profile.display_name.clone(),
            })
        })
        .collect();
    Ok(CredentialImpact {
        credential_ref_id: ref_id,
        is_shared: affected.len() > 1,
        affected_favorites: affected,
    })
}
