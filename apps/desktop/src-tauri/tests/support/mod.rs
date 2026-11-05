//! Shared IPC contract-test harness for the desktop commands.
//!
//! The real Tauri layer is a thin adapter around `thriftp-core`. These helpers
//! build a temporary application-data directory, a JSON metadata store, and a
//! deterministic in-memory credential store so command payloads, persistence,
//! errors, and redaction can be asserted without a native window.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use serde_json::{Value, json};
use tempfile::TempDir;
use thriftp_core::credentials::{
    CredentialAvailability, CredentialService, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{AuthMethod, CredentialPolicy, CredentialRef, Protocol, ServerFavorite};
use thriftp_core::repository::{AppState, MetadataStore};
use thriftp_core::server_favorites::{
    ServerFavoriteInput, ServerFavoriteService, ServerFavoriteView,
};
use thriftp_desktop_lib::error::DesktopError;
use thriftp_storage_json::JsonMetadataStore;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CredentialStoreMode {
    Available,
    Locked,
    Unavailable,
    Denied,
}

pub struct MemoryCredentialStore {
    values: Arc<Mutex<HashMap<uuid::Uuid, Vec<u8>>>>,
    mode: CredentialStoreMode,
}

impl MemoryCredentialStore {
    fn new(mode: CredentialStoreMode) -> Self {
        Self {
            values: Arc::new(Mutex::new(HashMap::new())),
            mode,
        }
    }

    fn failure(&self) -> Option<CredentialStoreError> {
        match self.mode {
            CredentialStoreMode::Available => None,
            CredentialStoreMode::Locked => Some(CredentialStoreError::Locked),
            CredentialStoreMode::Unavailable => Some(CredentialStoreError::Unavailable),
            CredentialStoreMode::Denied => Some(CredentialStoreError::Denied),
        }
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        match self.mode {
            CredentialStoreMode::Available => Ok(CredentialAvailability::Available),
            CredentialStoreMode::Locked => Ok(CredentialAvailability::Locked),
            CredentialStoreMode::Unavailable => Ok(CredentialAvailability::Unavailable),
            CredentialStoreMode::Denied => Ok(CredentialAvailability::Denied),
        }
    }

    fn get(&self, reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError> {
        if let Some(error) = self.failure() {
            return Err(error);
        }
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(&reference.id)
            .map(|value| Secret::new(value.clone())))
    }

    fn set(&self, reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError> {
        if let Some(error) = self.failure() {
            return Err(error);
        }
        self.values
            .lock()
            .unwrap()
            .insert(reference.id, secret.expose().to_vec());
        Ok(())
    }

    fn delete(&self, reference: &CredentialRef) -> Result<(), CredentialStoreError> {
        if let Some(error) = self.failure() {
            return Err(error);
        }
        self.values
            .lock()
            .unwrap()
            .remove(&reference.id)
            .map(|_| ())
            .ok_or(CredentialStoreError::NotFound)
    }
}

/// A temporary desktop application-data environment.
pub struct TestApp {
    directory: TempDir,
    pub store: Arc<dyn MetadataStore>,
    pub service: ServerFavoriteService,
    pub credentials: CredentialService,
}

impl Default for TestApp {
    fn default() -> Self {
        Self::new()
    }
}

impl TestApp {
    #[must_use]
    pub fn new() -> Self {
        self_with_mode(CredentialStoreMode::Available)
    }

    #[must_use]
    pub fn with_credential_mode(mode: CredentialStoreMode) -> Self {
        self_with_mode(mode)
    }

    #[must_use]
    pub fn state(&self) -> AppState {
        self.store.load().expect("metadata store")
    }

    pub fn save_state(&self, state: &AppState) {
        self.store.save(state).expect("metadata store");
    }

    pub fn create(
        &self,
        input: ServerFavoriteInput,
        policy: CredentialPolicy,
        reference: Option<uuid::Uuid>,
        secret: Option<&str>,
    ) -> CoreResult<ServerFavoriteView> {
        self.service.create(
            input,
            policy,
            reference,
            secret.map(|value| Secret::new(value.as_bytes().to_vec())),
            &self.credentials,
        )
    }

    #[must_use]
    pub fn metadata_path(&self) -> std::path::PathBuf {
        self.directory.path().join("metadata.json")
    }

    #[must_use]
    pub fn credentials(&self) -> &CredentialService {
        &self.credentials
    }

    #[must_use]
    pub fn store_path(&self) -> &std::path::Path {
        self.directory.path()
    }
}

fn self_with_mode(mode: CredentialStoreMode) -> TestApp {
    let directory = TempDir::new().expect("temporary app data");
    let store: Arc<dyn MetadataStore> = Arc::new(JsonMetadataStore::new(
        directory.path().join("metadata.json"),
    ));
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::new(mode)));
    TestApp {
        directory,
        store,
        service,
        credentials,
    }
}

/// Typed builder for the `favorites_create` / `favorites_update` profile payload.
#[must_use]
pub fn favorite_input(name: &str) -> ServerFavoriteInput {
    ServerFavoriteInput {
        display_name: name.to_owned(),
        group_label: None,
        protocol: Protocol::Sftp,
        host: "example.test".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
        local_path: None,
        remote_path: None,
    }
}

/// JSON payload as sent by the Svelte client, using camelCase field names.
#[must_use]
pub fn favorite_payload(name: &str) -> Value {
    json!({
        "displayName": name,
        "protocol": "sftp",
        "host": "example.test",
        "port": 22,
        "username": "tester",
        "authMethod": "password",
        "timeoutSeconds": 30,
        "pinned": false
    })
}

#[must_use]
pub fn decode_favorite_payload(value: Value) -> ServerFavoriteInput {
    serde_json::from_value(value).expect("favorite payload")
}

/// Assert that a serialized command result never contains a secret sentinel.
pub fn assert_no_secret<T: Serialize>(value: &T, sentinel: &str) -> String {
    let text = serde_json::to_string(value).expect("serialize payload");
    assert!(
        !text.contains(sentinel),
        "secret sentinel leaked into an IPC payload"
    );
    text
}

/// Assert that the command's user-facing error is redacted and stable.
pub fn assert_redacted(error: &DesktopError, sentinel: &str) {
    assert!(!error.message.contains(sentinel));
    assert!(!error.code.contains(sentinel));
    assert!(
        !error
            .field
            .as_deref()
            .unwrap_or_default()
            .contains(sentinel)
    );
}

/// Assert that a core error maps to the expected desktop error code.
#[must_use]
pub fn desktop_code(error: &CoreError) -> String {
    DesktopError::from_core(error).code
}

/// Seed unrelated server favorites plus directory favorites and tabs that
/// reference a profile, for deletion-isolation contracts.
pub fn seed_dependent_data(app: &TestApp, profile_id: uuid::Uuid, keep_profile_id: uuid::Uuid) {
    let mut state = app.state();
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == keep_profile_id)
        .cloned()
        .unwrap_or_else(|| {
            thriftp_core::model::ConnectionProfile::new(
                "keep",
                Protocol::Sftp,
                "keep.example",
                22,
                "tester",
                AuthMethod::Password,
                30,
            )
        });
    if !state
        .connection_profiles
        .iter()
        .any(|existing| existing.id == keep_profile_id)
    {
        state.connection_profiles.push(profile.clone());
    }
    if !state
        .server_favorites
        .iter()
        .any(|favorite| favorite.connection_profile_id == keep_profile_id)
    {
        state.server_favorites.push(ServerFavorite::new(
            keep_profile_id,
            CredentialPolicy::Ask,
            None,
        ));
    }
    state
        .directory_favorites
        .push(thriftp_core::model::DirectoryFavorite::new(
            profile_id, "/target", "target", 0,
        ));
    state
        .directory_favorites
        .push(thriftp_core::model::DirectoryFavorite::new(
            keep_profile_id,
            "/keep",
            "keep",
            1,
        ));
    state.tabs.push(thriftp_core::model::BrowsingSession::new(
        Some(profile_id),
        "/target",
    ));
    state.tabs.push(thriftp_core::model::BrowsingSession::new(
        Some(keep_profile_id),
        "/keep",
    ));
    app.save_state(&state);
}
