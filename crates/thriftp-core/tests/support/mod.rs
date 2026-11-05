//! Shared test fakes used by the Thriftp core integration tests.

#![allow(dead_code)]

pub mod fakes;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use thriftp_core::credentials::{
    CredentialAvailability, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    ConnectionProfile, CredentialRef, EntryType, FileEntry, ProtocolCapabilities,
};
use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};
use thriftp_core::repository::{AppState, MetadataStore, TransferState, TransferStore};

#[derive(Default)]
pub struct FakeRemoteFs {
    pub entries: Vec<FileEntry>,
    pub fail_listing: bool,
    pub deleted: Vec<String>,
}

impl FakeRemoteFs {
    #[must_use]
    pub fn with_entries(entries: Vec<FileEntry>) -> Self {
        Self {
            entries,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn sample_entry(name: &str, entry_type: EntryType) -> FileEntry {
        FileEntry {
            name: name.to_owned(),
            path: format!("/fake/{name}"),
            entry_type,
            size: (entry_type == EntryType::File).then_some(10),
            modified_at: None,
            permissions: None,
            ownership: None,
            hidden: name.starts_with('.'),
            symlink_target: None,
            capabilities: ProtocolCapabilities::default(),
        }
    }
}

#[async_trait]
impl RemoteFs for FakeRemoteFs {
    async fn connect(
        &mut self,
        profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        match profile.host.as_str() {
            "certificate.invalid" => Err(CoreError::CertificateInvalid(
                "the TLS certificate is not trusted".to_owned(),
            )),
            "hostkey.unknown" => Err(CoreError::HostKeyUnknown(
                "the host key is not present in known_hosts".to_owned(),
            )),
            "hostkey.changed" => Err(CoreError::HostKeyChanged(
                "the host key no longer matches known_hosts".to_owned(),
            )),
            "key.missing" => Err(CoreError::AuthFailed(
                "the SSH private-key file is missing or was moved".to_owned(),
            )),
            _ => match credential {
                None => Err(CoreError::CredentialMissing),
                Some(secret) if secret.expose() == b"rejected" => {
                    Err(CoreError::CredentialRejected)
                }
                Some(_) => Ok(()),
            },
        }
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }

    async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
        if self.fail_listing {
            return Err(CoreError::internal("fake listing failure"));
        }
        Ok(self.entries.clone())
    }

    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
        self.entries
            .iter()
            .find(|entry| entry.path == path)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(path.to_owned()))
    }

    async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn delete_file(&mut self, path: &str) -> CoreResult<()> {
        self.deleted.push(path.to_owned());
        Ok(())
    }

    async fn delete_dir(&mut self, path: &str) -> CoreResult<()> {
        self.deleted.push(path.to_owned());
        Ok(())
    }

    async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
        Ok(())
    }
}

pub struct FakeFactory;

impl RemoteFsFactory for FakeFactory {
    fn create(&self, _protocol: thriftp_core::model::Protocol) -> Box<dyn RemoteFs> {
        Box::new(FakeRemoteFs::default())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CredentialStoreMode {
    Available,
    Unavailable,
    Locked,
    Denied,
}

#[derive(Default)]
pub struct MemoryCredentialStore {
    values: Mutex<HashMap<uuid::Uuid, Vec<u8>>>,
    mode: Option<CredentialStoreMode>,
}

impl MemoryCredentialStore {
    #[must_use]
    pub fn available() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            mode: Some(CredentialStoreMode::Available),
        }
    }

    #[must_use]
    pub fn unavailable() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            mode: Some(CredentialStoreMode::Unavailable),
        }
    }

    #[must_use]
    pub fn locked() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            mode: Some(CredentialStoreMode::Locked),
        }
    }

    #[must_use]
    pub fn denied() -> Self {
        Self {
            values: Mutex::new(HashMap::new()),
            mode: Some(CredentialStoreMode::Denied),
        }
    }

    fn failure(&self) -> Option<CredentialStoreError> {
        match self.mode.unwrap_or(CredentialStoreMode::Available) {
            CredentialStoreMode::Available => None,
            CredentialStoreMode::Unavailable => Some(CredentialStoreError::Unavailable),
            CredentialStoreMode::Locked => Some(CredentialStoreError::Locked),
            CredentialStoreMode::Denied => Some(CredentialStoreError::Denied),
        }
    }
}

impl CredentialStore for MemoryCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        match self.mode.unwrap_or(CredentialStoreMode::Available) {
            CredentialStoreMode::Available => Ok(CredentialAvailability::Available),
            CredentialStoreMode::Unavailable => Ok(CredentialAvailability::Unavailable),
            CredentialStoreMode::Locked => Ok(CredentialAvailability::Locked),
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

/// JSON-backed stores in a temporary directory.
pub struct TempJsonStores {
    directory: tempfile::TempDir,
}

impl TempJsonStores {
    pub fn new() -> Self {
        Self {
            directory: tempfile::tempdir().expect("temporary directory"),
        }
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.directory.path().join("metadata.json")
    }

    pub fn transfer_path(&self) -> PathBuf {
        self.directory.path().join("transfers.json")
    }
}

/// Minimal JSON store used by tests without depending on the storage crate.
pub struct TempJsonMetadataStore {
    path: PathBuf,
}

impl TempJsonMetadataStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl MetadataStore for TempJsonMetadataStore {
    fn load(&self) -> CoreResult<AppState> {
        if !self.path.is_file() {
            return Ok(AppState::default());
        }
        let bytes = std::fs::read(&self.path).map_err(io_error)?;
        serde_json::from_slice(&bytes).map_err(|error| CoreError::internal(error.to_string()))
    }

    fn save(&self, state: &AppState) -> CoreResult<()> {
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| CoreError::internal(error.to_string()))?;
        std::fs::write(&self.path, bytes).map_err(io_error)
    }
}

pub struct TempJsonTransferStore {
    path: PathBuf,
}

impl TempJsonTransferStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl TransferStore for TempJsonTransferStore {
    fn load(&self) -> CoreResult<TransferState> {
        if !self.path.is_file() {
            return Ok(TransferState::default());
        }
        let bytes = std::fs::read(&self.path).map_err(io_error)?;
        serde_json::from_slice(&bytes).map_err(|error| CoreError::internal(error.to_string()))
    }

    fn save(&self, state: &TransferState) -> CoreResult<()> {
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| CoreError::internal(error.to_string()))?;
        std::fs::write(&self.path, bytes).map_err(io_error)
    }
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::internal(format!("test storage error: {error}"))
}
