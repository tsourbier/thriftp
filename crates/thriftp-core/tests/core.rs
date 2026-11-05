use std::sync::Mutex;

use async_trait::async_trait;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{
    CredentialAvailability, CredentialService, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::error::CoreError;
use thriftp_core::favorites::DirectoryFavoriteService;
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, CredentialRef, DirectoryFavorite, FileEntry, Preference,
    Priority, Protocol, ProtocolCapabilities, SortKey, Transfer, TransferDirection,
    TransferHistory,
};
use thriftp_core::preferences::PreferencesService;
use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};
use thriftp_core::repository::{AppState, MemoryStore, MetadataStore};
use thriftp_core::sorting::{sort_file_entries, sort_profiles};
use thriftp_core::validation::{identity_matches, validate_profile};

#[derive(Default)]
struct FakeRemote {
    connected: bool,
}

#[async_trait]
impl RemoteFs for FakeRemote {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        _credential: Option<Secret>,
    ) -> thriftp_core::CoreResult<()> {
        self.connected = true;
        Ok(())
    }

    async fn disconnect(&mut self) -> thriftp_core::CoreResult<()> {
        self.connected = false;
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }

    async fn list_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }

    async fn stat(&mut self, path: &str) -> thriftp_core::CoreResult<FileEntry> {
        Ok(FileEntry::directory(
            path,
            path,
            ProtocolCapabilities::default(),
        ))
    }

    async fn mkdir(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn rename(&mut self, _from: &str, _to: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn delete_file(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn delete_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn chmod(&mut self, _path: &str, _mode: u32) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
}

struct FakeFactory;

impl RemoteFsFactory for FakeFactory {
    fn create(&self, _protocol: Protocol) -> Box<dyn RemoteFs> {
        Box::new(FakeRemote::default())
    }
}

#[derive(Default)]
struct FakeCredentialStore {
    secret: Mutex<Option<Vec<u8>>>,
}

impl CredentialStore for FakeCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        Ok(CredentialAvailability::Available)
    }

    fn get(&self, _reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError> {
        Ok(self
            .secret
            .lock()
            .unwrap()
            .as_ref()
            .map(|value| Secret::new(value.clone())))
    }

    fn set(&self, _reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError> {
        *self.secret.lock().unwrap() = Some(secret.expose().to_vec());
        Ok(())
    }

    fn delete(&self, _reference: &CredentialRef) -> Result<(), CredentialStoreError> {
        *self.secret.lock().unwrap() = None;
        Ok(())
    }
}

fn profile(name: &str) -> ConnectionProfile {
    ConnectionProfile::new(
        name,
        Protocol::Sftp,
        "example.test",
        22,
        "tester",
        AuthMethod::Password,
        30,
    )
}

fn credential_ref() -> CredentialRef {
    let now = chrono::Utc::now();
    CredentialRef {
        id: uuid::Uuid::new_v4(),
        protocol: Protocol::Sftp,
        host: "example.test".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn profile_validation_and_identity_binding() {
    let profile = profile("Test Server");
    assert!(validate_profile(&profile).is_ok());
    let reference = credential_ref();
    assert!(identity_matches(&profile, &reference));

    let mut invalid = profile.clone();
    invalid.display_name.clear();
    assert!(matches!(
        validate_profile(&invalid),
        Err(CoreError::Validation(_))
    ));
}

#[test]
fn sorting_is_deterministic() {
    let mut profiles = vec![profile("zeta"), profile("Alpha"), profile("beta")];
    sort_profiles(&mut profiles);
    let names: Vec<_> = profiles
        .iter()
        .map(|profile| profile.display_name.as_str())
        .collect();
    assert_eq!(names, vec!["Alpha", "beta", "zeta"]);

    let capabilities = ProtocolCapabilities::default();
    let mut entries = vec![
        FileEntry {
            name: "b".to_owned(),
            size: Some(2),
            ..FileEntry::directory("b", "/b", capabilities.clone())
        },
        FileEntry {
            name: "a".to_owned(),
            size: Some(1),
            ..FileEntry::directory("a", "/a", capabilities)
        },
    ];
    sort_file_entries(&mut entries, SortKey::Size);
    assert_eq!(entries[0].name, "a");
}

#[tokio::test]
async fn connection_service_connects_and_disconnects() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let profile = profile("Test Server");
    let id = service.create_profile(profile).unwrap();
    let active = service.connect(id, None).await.unwrap();
    assert_eq!(service.active_connections().len(), 1);
    service.disconnect(active.id).await.unwrap();
    assert!(service.active_connections().is_empty());
}

#[test]
fn credential_service_round_trip() {
    let store = FakeCredentialStore::default();
    let service = CredentialService::new(Box::new(store));
    let reference = credential_ref();
    assert!(service.get(&reference).unwrap().is_none());
    service
        .set(&reference, Secret::new(b"secret".to_vec()))
        .unwrap();
    assert!(service.get(&reference).unwrap().is_some());
    service.delete(&reference).unwrap();
    assert!(service.get(&reference).unwrap().is_none());
}

#[test]
fn directory_favorites_and_preferences_round_trip() {
    let mut favorites = DirectoryFavoriteService::default();
    let favorite = DirectoryFavorite::new(uuid::Uuid::new_v4(), "/tmp", "Temp", 1);
    favorites.create(favorite.clone()).unwrap();
    assert_eq!(favorites.list(None).len(), 1);
    favorites.delete(favorite.id).unwrap();
    assert!(favorites.list(None).is_empty());

    let mut preferences = PreferencesService::default();
    let preference = Preference {
        default_concurrency: 8,
        ..Preference::default()
    };
    preferences.set(preference);
    assert_eq!(preferences.get().default_concurrency, 8);
}

#[test]
fn memory_store_round_trip() {
    let store = MemoryStore::default();
    let mut state = AppState::default();
    state.connection_profiles.push(profile("Stored"));
    store.save(&state).unwrap();
    assert_eq!(store.load().unwrap().connection_profiles.len(), 1);
}

#[test]
fn transfer_history_is_bounded() {
    let mut history = TransferHistory {
        retention_limit: 2,
        ..TransferHistory::default()
    };
    for _ in 0..3 {
        let transfer = Transfer::new(
            TransferDirection::Download,
            uuid::Uuid::new_v4(),
            "/remote",
            "/local",
            Some(10),
        );
        history.push(transfer);
    }
    assert_eq!(history.entries.len(), 2);
    assert_eq!(history.entries[0].priority, Priority::Normal);
}
