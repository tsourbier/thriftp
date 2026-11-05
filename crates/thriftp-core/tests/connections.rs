use async_trait::async_trait;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::Secret;
use thriftp_core::error::CoreError;
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, FileEntry, Protocol, ProtocolCapabilities,
};
use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};

struct FakeSession;

#[async_trait]
impl RemoteFs for FakeSession {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> thriftp_core::CoreResult<()> {
        match credential {
            None => Err(CoreError::CredentialMissing),
            Some(secret) if secret.expose() == b"rejected" => Err(CoreError::CredentialRejected),
            Some(_) => Ok(()),
        }
    }

    async fn disconnect(&mut self) -> thriftp_core::CoreResult<()> {
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

    async fn list_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }

    async fn stat(&mut self, _path: &str) -> thriftp_core::CoreResult<FileEntry> {
        Err(CoreError::UnsupportedCapability("stat".to_owned()))
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
        Box::new(FakeSession)
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

fn secret(value: &str) -> Option<Secret> {
    Some(Secret::new(value.as_bytes().to_vec()))
}

#[test]
fn profiles_are_created_listed_updated_and_deleted() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let mut first = profile("beta");
    let second = profile("alpha");
    service.create_profile(first.clone()).unwrap();
    service.create_profile(second.clone()).unwrap();

    let listed = service.list_profiles();
    assert_eq!(listed[0].display_name, "alpha");
    assert!(service.create_profile(first.clone()).is_err());

    first.display_name = "renamed".to_owned();
    service.update_profile(first.clone()).unwrap();
    assert_eq!(
        service.get_profile(first.id).unwrap().display_name,
        "renamed"
    );

    service.delete_profile(second.id).unwrap();
    assert!(service.get_profile(second.id).is_err());
    assert_eq!(service.list_profiles().len(), 1);
}

#[tokio::test]
async fn supports_multiple_connections_and_last_used_updates() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let profile = profile("server");
    let id = service.create_profile(profile).unwrap();
    assert!(service.get_profile(id).unwrap().last_used_at.is_none());

    let first = service.connect(id, secret("ok")).await.unwrap();
    let second = service.connect(id, secret("ok")).await.unwrap();

    assert_eq!(service.active_connections().len(), 2);
    assert_ne!(first.id, second.id);
    assert!(service.get_profile(id).unwrap().last_used_at.is_some());

    service.disconnect(first.id).await.unwrap();
    assert_eq!(service.active_connections().len(), 1);
}

#[tokio::test]
async fn reconnect_replaces_the_session() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let id = service.create_profile(profile("server")).unwrap();
    let connection = service.connect(id, secret("ok")).await.unwrap();

    let replacement = service
        .reconnect(connection.id, secret("ok"))
        .await
        .unwrap();

    assert_ne!(replacement.id, connection.id);
    assert_eq!(service.active_connections().len(), 1);
    assert_eq!(service.active_connections()[0].id, replacement.id);
    assert!(service.session(connection.id).is_err());
    assert!(service.session(replacement.id).is_ok());
}

#[tokio::test]
async fn credential_failures_are_reported() {
    let mut service = ConnectionService::new(Box::new(FakeFactory));
    let id = service.create_profile(profile("server")).unwrap();

    assert_eq!(
        service.connect(id, None).await.unwrap_err(),
        CoreError::CredentialMissing
    );
    assert_eq!(
        service.connect(id, secret("rejected")).await.unwrap_err(),
        CoreError::CredentialRejected
    );
    assert!(service.active_connections().is_empty());
}
