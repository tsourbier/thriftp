mod support;

use std::fs;

use support::TestApp;
use tempfile::TempDir;
use thriftp_core::history::ConnectionHistoryService;
use thriftp_core::model::{
    AuthMethod, ConnectionHistoryEntry, CredentialPolicy, HubTab, Protocol, WorkspaceLayout,
};
use thriftp_core::repository::MetadataStore;
use thriftp_core::ssh_keys::validate_key_path;
use thriftp_core::workspace::WorkspaceService;
use thriftp_storage_json::JsonMetadataStore;

#[test]
fn workspace_layout_normalizes_and_round_trips() {
    let home = TempDir::new().unwrap();
    let store = std::sync::Arc::new(JsonMetadataStore::new(home.path().join("metadata.json")));
    let service = WorkspaceService::new(store);
    assert_eq!(service.get().unwrap(), WorkspaceLayout::default());
    let saved = service
        .set(WorkspaceLayout {
            split_ratio: 0.99,
            sidebar_visible: false,
            sidebar_sections: Default::default(),
            hub_tab: HubTab::History,
        })
        .unwrap();
    assert_eq!(saved.split_ratio, 0.8);
    assert_eq!(saved.hub_tab, HubTab::History);
    assert_eq!(service.get().unwrap(), saved);
}

#[test]
fn history_deduplicates_by_favorite_and_clear_requires_confirmation() {
    let home = TempDir::new().unwrap();
    let store = std::sync::Arc::new(JsonMetadataStore::new(home.path().join("metadata.json")));
    let service = ConnectionHistoryService::new(store);
    let profile = uuid::Uuid::new_v4();
    let favorite = uuid::Uuid::new_v4();
    service
        .record_success(ConnectionHistoryEntry {
            favorite_id: Some(favorite),
            connection_profile_id: profile,
            display_name: "old".to_owned(),
            protocol: Protocol::Sftp,
            host: "example.test".to_owned(),
            port: 22,
            username: "tester".to_owned(),
            last_used_at: chrono::Utc::now(),
        })
        .unwrap();
    service
        .record_success(ConnectionHistoryEntry {
            favorite_id: Some(favorite),
            connection_profile_id: profile,
            display_name: "new".to_owned(),
            protocol: Protocol::Sftp,
            host: "example.test".to_owned(),
            port: 22,
            username: "tester".to_owned(),
            last_used_at: chrono::Utc::now(),
        })
        .unwrap();
    assert_eq!(service.list().unwrap().len(), 1);
    assert!(service.clear(false).is_err());
    assert_eq!(service.clear(true).unwrap(), 1);
}

#[test]
fn favorite_paths_are_preserved_and_validated() {
    let app = TestApp::new();
    let mut input = support::favorite_input("paths");
    input.local_path = Some("/tmp".to_owned());
    input.remote_path = Some("/srv/www".to_owned());
    let view = app.create(input, CredentialPolicy::Ask, None, None).unwrap();
    assert_eq!(view.local_path.as_deref(), Some("/tmp"));
    assert_eq!(view.remote_path.as_deref(), Some("/srv/www"));

    let mut invalid = support::favorite_input("invalid");
    invalid.local_path = Some("relative/path".to_owned());
    let error = app
        .create(invalid, CredentialPolicy::Ask, None, None)
        .unwrap_err();
    assert_eq!(error.field().as_deref(), Some("localPath"));
}

#[test]
fn ssh_key_validation_never_returns_key_contents() {
    let home = TempDir::new().unwrap();
    let key = home.path().join("id_ed25519");
    fs::write(&key, "PRIVATE-CONTENT").unwrap();
    let result = validate_key_path(&key.display().to_string()).unwrap();
    assert!(result.valid);
    assert!(result.code.is_none());
    assert!(!format!("{result:?}").contains("PRIVATE-CONTENT"));

    let public = home.path().join("id_ed25519.pub");
    fs::write(&public, "PUBLIC").unwrap();
    let result = validate_key_path(&public.display().to_string()).unwrap();
    assert!(!result.valid);
    assert_eq!(result.code.as_deref(), Some("public_key"));
}

#[test]
fn existing_favorites_without_paths_still_load() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut state = support::TestApp::new().state();
    state.connection_profiles.push(thriftp_core::model::ConnectionProfile::new(
        "legacy",
        Protocol::Sftp,
        "legacy.test",
        22,
        "tester",
        AuthMethod::Password,
        30,
    ));
    store.save(&state).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.workspace_layout, WorkspaceLayout::default());
    assert!(loaded.connection_history.is_empty());
}
