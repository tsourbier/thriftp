mod support;

use support::{
    FakeRemoteFs, MemoryCredentialStore, TempJsonMetadataStore, TempJsonStores,
    TempJsonTransferStore,
};
use thriftp_core::browser;
use thriftp_core::credentials::CredentialService;
use thriftp_core::model::{EntryType, ProtocolCapabilities};
use thriftp_core::protocol::RemoteFs;
use thriftp_core::repository::{MetadataStore, TransferStore};

#[tokio::test]
async fn fake_remote_supports_listing_and_deletion() {
    let mut remote = FakeRemoteFs::with_entries(vec![
        FakeRemoteFs::sample_entry("alpha", EntryType::File),
        FakeRemoteFs::sample_entry("folder", EntryType::Directory),
    ]);
    let entries = browser::list_remote(&mut remote, "/fake").await.unwrap();
    assert_eq!(entries.len(), 2);
    browser::delete_remote_file(&mut remote, "/fake/alpha")
        .await
        .unwrap();
    assert_eq!(remote.deleted, vec!["/fake/alpha"]);
}

#[test]
fn fake_credential_store_round_trips() {
    let service = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let reference = thriftp_core::model::CredentialRef {
        id: uuid::Uuid::new_v4(),
        protocol: thriftp_core::model::Protocol::Sftp,
        host: "example.test".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: thriftp_core::model::AuthMethod::Password,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    service
        .set(
            &reference,
            thriftp_core::credentials::Secret::new(b"secret".to_vec()),
        )
        .unwrap();
    assert!(service.get(&reference).unwrap().is_some());
    service.delete(&reference).unwrap();
    assert!(service.get(&reference).unwrap().is_none());
}

#[test]
fn temporary_json_stores_round_trip() {
    let stores = TempJsonStores::new();
    let metadata = TempJsonMetadataStore::new(stores.metadata_path());
    let mut state = thriftp_core::repository::AppState::default();
    state.preference.show_hidden_files = true;
    metadata.save(&state).unwrap();
    assert!(metadata.load().unwrap().preference.show_hidden_files);

    let transfer = TempJsonTransferStore::new(stores.transfer_path());
    let mut transfer_state = thriftp_core::repository::TransferState::default();
    transfer_state.queue.paused = true;
    transfer.save(&transfer_state).unwrap();
    assert!(transfer.load().unwrap().queue.paused);
}

#[test]
fn fake_capabilities_are_available() {
    let remote = FakeRemoteFs::default();
    assert_eq!(remote.capabilities(), ProtocolCapabilities::default());
}
