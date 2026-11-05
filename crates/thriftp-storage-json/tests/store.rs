use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol, Transfer, TransferDirection};
use thriftp_core::repository::{AppState, MetadataStore, TransferState, TransferStore};
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};

#[test]
fn metadata_store_round_trip_and_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("favorites.json");
    let store = JsonMetadataStore::new(&path);
    assert!(store.load().unwrap().connection_profiles.is_empty());

    let mut state = AppState::default();
    state.connection_profiles.push(ConnectionProfile::new(
        "Saved",
        Protocol::Sftp,
        "example.test",
        22,
        "tester",
        AuthMethod::Password,
        30,
    ));
    store.save(&state).unwrap();

    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(raw.contains("schemaVersion"));
    assert!(!raw.contains("secret"));
    assert_eq!(store.load().unwrap().connection_profiles.len(), 1);
}

#[test]
fn transfer_store_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transfers.json");
    let store = JsonTransferStore::new(&path);
    let mut state = TransferState::default();
    state.transfers.push(Transfer::new(
        TransferDirection::Upload,
        uuid::Uuid::new_v4(),
        "/local/file.txt",
        "/remote/file.txt",
        Some(100),
    ));
    store.save(&state).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.transfers.len(), 1);
    assert_eq!(loaded.queue.concurrency_limit, 4);
}
