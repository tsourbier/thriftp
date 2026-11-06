use std::fs;

use serde_json::json;
use tempfile::TempDir;
use thriftp_core::model::{BrowsingSession, HubTab, SortDirection, WorkspaceLayout};
use thriftp_core::repository::{AppState, MetadataStore};
use thriftp_storage_json::JsonMetadataStore;

#[test]
fn version_zero_and_one_documents_load_with_defaults() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());

    let zero = serde_json::to_value(AppState::default()).unwrap();
    fs::write(&path, serde_json::to_vec_pretty(&zero).unwrap()).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.workspace_layout, WorkspaceLayout::default());
    assert!(loaded.connection_history.is_empty());

    let mut one = serde_json::to_value(AppState::default()).unwrap();
    one["schemaVersion"] = json!(1);
    fs::write(&path, serde_json::to_vec_pretty(&one).unwrap()).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.workspace_layout.split_ratio, 0.5);
    assert!(loaded.connection_history.is_empty());
}

#[test]
fn version_two_round_trips_layout_and_history() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut state = AppState::default();
    state.workspace_layout.split_ratio = 0.65;
    state.workspace_layout.hub_tab = HubTab::History;
    store.save(&state).unwrap();

    let raw = fs::read_to_string(&path).unwrap();
    assert!(raw.contains("\"schemaVersion\": 2"));
    assert!(raw.contains("workspaceLayout"));
    let loaded = store.load().unwrap();
    assert_eq!(loaded.workspace_layout.split_ratio, 0.65);
    assert_eq!(loaded.workspace_layout.hub_tab, HubTab::History);
}

#[test]
fn missing_sort_directions_default_to_ascending() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut state = AppState::default();
    state.tabs.push(BrowsingSession::new(None, "."));
    store.save(&state).unwrap();

    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    document["tabs"][0]["viewSettings"]
        .as_object_mut()
        .unwrap()
        .remove("sortDirection");
    document["preference"]
        .as_object_mut()
        .unwrap()
        .remove("sortDirection");
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(
        loaded.tabs[0].view_settings.sort_direction,
        SortDirection::Asc
    );
    assert_eq!(loaded.preference.sort_direction, SortDirection::Asc);
}

#[test]
fn unknown_future_versions_fail_closed() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut document = serde_json::to_value(AppState::default()).unwrap();
    document["schemaVersion"] = json!(99);
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    assert!(store.load().is_err());
}

#[test]
fn atomic_save_keeps_a_backup() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    store.save(&AppState::default()).unwrap();
    store.save(&AppState::default()).unwrap();
    assert!(path.with_extension("bak").is_file());
}
