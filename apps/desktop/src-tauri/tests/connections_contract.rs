use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{
    CredentialAvailability, CredentialService, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, CredentialRef, FileEntry, Protocol, ProtocolCapabilities,
};
use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};

use thriftp_desktop_lib::commands::connections;
use thriftp_desktop_lib::error::DesktopError;
use thriftp_desktop_lib::views::ConnectionView;

struct FakeSession;

#[async_trait]
impl RemoteFs for FakeSession {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        match credential {
            None => Err(CoreError::CredentialMissing),
            Some(secret) if secret.expose() == b"rejected" => Err(CoreError::CredentialRejected),
            Some(secret) if secret.expose() == b"locked" => Err(
                CoreError::CredentialStoreUnavailable("credential store is locked".to_owned()),
            ),
            Some(_) => Ok(()),
        }
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            resume: true,
            permissions: true,
            ownership: false,
            hidden_files: true,
            symbolic_links: true,
            server_side_rename: true,
            server_side_delete: true,
            max_path_length: None,
        }
    }

    async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }

    async fn stat(&mut self, _path: &str) -> CoreResult<FileEntry> {
        Err(CoreError::UnsupportedCapability("stat".to_owned()))
    }

    async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
        Ok(())
    }
}

struct FakeFactory;

impl RemoteFsFactory for FakeFactory {
    fn create(&self, _protocol: Protocol) -> Box<dyn RemoteFs> {
        Box::new(FakeSession)
    }
}

#[derive(Default)]
struct MemoryCredentialStore {
    values: Mutex<HashMap<uuid::Uuid, Vec<u8>>>,
}

impl CredentialStore for MemoryCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        Ok(CredentialAvailability::Available)
    }

    fn get(&self, reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError> {
        Ok(self
            .values
            .lock()
            .unwrap()
            .get(&reference.id)
            .map(|value| Secret::new(value.clone())))
    }

    fn set(&self, reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError> {
        self.values
            .lock()
            .unwrap()
            .insert(reference.id, secret.expose().to_vec());
        Ok(())
    }

    fn delete(&self, reference: &CredentialRef) -> Result<(), CredentialStoreError> {
        self.values
            .lock()
            .unwrap()
            .remove(&reference.id)
            .map(|_| ())
            .ok_or(CredentialStoreError::NotFound)
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

fn setup() -> (ConnectionService, CredentialService) {
    (
        ConnectionService::new(Box::new(FakeFactory)),
        CredentialService::new(Box::new(MemoryCredentialStore::default())),
    )
}

#[tokio::test]
async fn create_binds_a_credential_and_connect_uses_it() {
    let (mut service, credentials) = setup();
    let stored = connections::create(
        &mut service,
        &credentials,
        profile("server"),
        Some(Secret::new(b"saved".to_vec())),
    )
    .unwrap();

    let connection = connections::connect(&mut service, &credentials, stored.id, None)
        .await
        .unwrap();
    assert_eq!(connection.connection_profile_id, stored.id);
    assert_eq!(connections::list(&service).len(), 1);
}

#[tokio::test]
async fn missing_and_rejected_credentials_map_to_distinct_errors() {
    let (mut service, credentials) = setup();
    let stored = connections::create(&mut service, &credentials, profile("server"), None).unwrap();

    let missing = connections::connect(&mut service, &credentials, stored.id, None)
        .await
        .unwrap_err();
    assert_eq!(missing, CoreError::CredentialMissing);

    let rejected = connections::connect(
        &mut service,
        &credentials,
        stored.id,
        Some(Secret::new(b"rejected".to_vec())),
    )
    .await
    .unwrap_err();
    assert_eq!(rejected, CoreError::CredentialRejected);
    assert_eq!(DesktopError::from(rejected).code, "credential_rejected");
}

#[tokio::test]
async fn update_replaces_the_bound_credential_and_delete_removes_it() {
    let (mut service, credentials) = setup();
    let stored = connections::create(
        &mut service,
        &credentials,
        profile("server"),
        Some(Secret::new(b"old".to_vec())),
    )
    .unwrap();

    let mut updated = stored.clone();
    updated.display_name = "renamed".to_owned();
    let updated = connections::update(
        &mut service,
        &credentials,
        updated,
        Some(Secret::new(b"new".to_vec())),
    )
    .unwrap();
    assert_eq!(updated.display_name, "renamed");

    connections::delete(&mut service, &credentials, updated.id).unwrap();
    assert!(connections::list(&service).is_empty());
    assert_eq!(
        connections::connect(&mut service, &credentials, updated.id, None)
            .await
            .unwrap_err(),
        CoreError::NotFound(format!("connection profile {}", updated.id))
    );
}

#[tokio::test]
async fn test_connection_does_not_create_an_active_session() {
    let (mut service, credentials) = setup();
    let stored = connections::create(&mut service, &credentials, profile("server"), None).unwrap();

    connections::test(
        &mut service,
        &credentials,
        stored.id,
        Some(Secret::new(b"ok".to_vec())),
    )
    .await
    .unwrap();
    assert!(service.active_connections().is_empty());
    assert!(
        service
            .get_profile(stored.id)
            .unwrap()
            .last_used_at
            .is_none()
    );
}

#[test]
fn error_payload_serializes_for_the_ui_contract() {
    let error = DesktopError::from(CoreError::validation("port must be 1-65535"));
    let json = serde_json::to_value(&error).unwrap();
    assert_eq!(json["code"], "validation");
    assert_eq!(json["field"], "port");
    assert_eq!(json["retryable"], false);
}

#[test]
fn connection_view_uses_contract_field_names() {
    let (mut service, credentials) = setup();
    let stored = connections::create(&mut service, &credentials, profile("server"), None).unwrap();
    let active = thriftp_core::model::ActiveConnection::new(stored.id, stored.protocol);
    let view = ConnectionView::from(active);
    let json = serde_json::to_value(view).unwrap();
    assert!(json.get("connectionId").is_some());
    assert!(json.get("profileId").is_some());
    assert!(json.get("state").is_some());
}

struct UnavailableCredentialStore;

impl CredentialStore for UnavailableCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        Ok(CredentialAvailability::Unavailable)
    }

    fn get(&self, _reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError> {
        Err(CredentialStoreError::Unavailable)
    }

    fn set(&self, _reference: &CredentialRef, _secret: Secret) -> Result<(), CredentialStoreError> {
        Err(CredentialStoreError::Unavailable)
    }

    fn delete(&self, _reference: &CredentialRef) -> Result<(), CredentialStoreError> {
        Err(CredentialStoreError::Unavailable)
    }
}

#[test]
fn unavailable_store_does_not_leave_a_partial_profile() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let credentials = CredentialService::new(Box::new(UnavailableCredentialStore));
    let error = connections::create(
        &mut service,
        &credentials,
        profile("partial"),
        Some(Secret::new(b"secret".to_vec())),
    )
    .unwrap_err();
    assert!(matches!(error, CoreError::CredentialStoreUnavailable(_)));
    assert!(service.list_profiles().is_empty());
}
