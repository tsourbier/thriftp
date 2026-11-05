mod support;

use async_trait::async_trait;
use std::sync::Arc;
use thriftp_core::browser;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::Secret;
use thriftp_core::error::CoreResult;
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, EntryType, FileEntry, Protocol, ProtocolCapabilities, Transfer,
    TransferDirection, TransferState,
};
use thriftp_core::protocol::{ProgressCallback, RemoteFs, RemoteFsFactory};
use thriftp_core::transfers::TransferControl;
use thriftp_core::transfers::engine::{execute_download, execute_upload};
use uuid::Uuid;

/// A fully local protocol implementation: no account, subscription, cloud
/// service, or remote authentication endpoint is involved.
#[derive(Default)]
struct OfflineRemote {
    files: std::collections::HashMap<String, Vec<u8>>,
}

#[async_trait]
impl RemoteFs for OfflineRemote {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        match credential {
            Some(secret) if secret.expose() == b"offline-secret" => Ok(()),
            _ => Err(thriftp_core::CoreError::CredentialRejected),
        }
    }
    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }
    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }
    async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
        Ok(self
            .files
            .keys()
            .map(|name| FileEntry {
                name: name.clone(),
                path: format!("/remote/{name}"),
                entry_type: EntryType::File,
                size: self.files.get(name).map(|bytes| bytes.len() as u64),
                modified_at: None,
                permissions: None,
                ownership: None,
                hidden: false,
                symlink_target: None,
                capabilities: ProtocolCapabilities::default(),
            })
            .collect())
    }
    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
        let name = path.trim_start_matches("/remote/");
        self.files
            .get(name)
            .map(|bytes| FileEntry {
                name: name.to_owned(),
                path: path.to_owned(),
                entry_type: EntryType::File,
                size: Some(bytes.len() as u64),
                modified_at: None,
                permissions: None,
                ownership: None,
                hidden: false,
                symlink_target: None,
                capabilities: ProtocolCapabilities::default(),
            })
            .ok_or_else(|| thriftp_core::CoreError::NotFound(path.to_owned()))
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
    async fn upload_from(
        &mut self,
        local_path: &std::path::Path,
        remote_path: &str,
        _offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        let bytes = tokio::fs::read(local_path)
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        let name = remote_path.trim_start_matches("/remote/").to_owned();
        self.files.insert(name, bytes.clone());
        let len = bytes.len() as u64;
        let _ = progress(len);
        Ok(len)
    }
    async fn download_to(
        &mut self,
        remote_path: &str,
        local_path: &std::path::Path,
        _offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        let name = remote_path.trim_start_matches("/remote/");
        let bytes = self
            .files
            .get(name)
            .cloned()
            .ok_or_else(|| thriftp_core::CoreError::NotFound(remote_path.to_owned()))?;
        tokio::fs::write(local_path, &bytes)
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        let len = bytes.len() as u64;
        let _ = progress(len);
        Ok(len)
    }
}

struct OfflineFactory;

impl RemoteFsFactory for OfflineFactory {
    fn create(&self, _protocol: Protocol) -> Box<dyn RemoteFs> {
        Box::new(OfflineRemote::default())
    }
}

#[tokio::test]
async fn core_workflow_runs_without_any_external_service() {
    let profile = ConnectionProfile::new(
        "offline",
        Protocol::Sftp,
        "localhost",
        22,
        "tester",
        AuthMethod::Password,
        30,
    );
    let mut service = ConnectionService::new(Box::new(OfflineFactory));
    let id = service.create_profile(profile).unwrap();
    let connection = service
        .connect(id, Some(Secret::new(b"offline-secret".to_vec())))
        .await
        .unwrap();
    let session = service.session(connection.id).unwrap();

    let directory = tempfile::tempdir().unwrap();
    let local_source = directory.path().join("upload.txt");
    std::fs::write(&local_source, b"offline workflow").unwrap();

    let mut upload = Transfer::new(
        TransferDirection::Upload,
        id,
        local_source.display().to_string(),
        "/remote/upload.txt",
        Some(16),
    );
    {
        let mut remote = session.lock().await;
        let control = TransferControl::new();
        let run = execute_upload(remote.as_mut(), &mut upload, &local_source, &control, None)
            .await
            .unwrap();
        assert_eq!(run.state, TransferState::Completed);
        let entries = browser::list_remote(remote.as_mut(), "/remote")
            .await
            .unwrap();
        assert_eq!(entries.len(), 1);
    }

    let local_destination = directory.path().join("downloaded.txt");
    let mut download = Transfer::new(
        TransferDirection::Download,
        id,
        "/remote/upload.txt",
        local_destination.display().to_string(),
        Some(16),
    );
    {
        let mut remote = session.lock().await;
        let control = TransferControl::new();
        let run = execute_download(
            remote.as_mut(),
            &mut download,
            &local_destination,
            &control,
            None,
        )
        .await
        .unwrap();
        assert_eq!(run.state, TransferState::Completed);
    }
    assert_eq!(
        std::fs::read(&local_destination).unwrap(),
        b"offline workflow"
    );

    service.disconnect(connection.id).await.unwrap();
    assert!(Uuid::nil() != connection.id);
    assert!(Arc::strong_count(&session) >= 1);
}
