//! Versioned JSON persistence adapters for Thriftp metadata and transfer state.
//! Secrets are never written here.

#![deny(missing_docs)]

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::repository::{AppState, MetadataStore};

/// Versioned JSON transfer-state persistence.
pub mod transfers;

/// Versioned server-favorites migration helpers.
pub mod server_favorites;

pub use transfers::JsonTransferStore;

pub(crate) const SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MetadataDocument {
    #[serde(default)]
    schema_version: u32,
    #[serde(flatten)]
    state: AppState,
}

/// Versioned JSON persistence for connection profiles, favorites, tabs, and
/// preferences. Secrets are never written here.
pub struct JsonMetadataStore {
    path: PathBuf,
}

impl JsonMetadataStore {
    /// Create a metadata store at the supplied path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the metadata document path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("bak")
    }
}

impl MetadataStore for JsonMetadataStore {
    fn load(&self) -> CoreResult<AppState> {
        if !self.path.is_file() {
            return Ok(AppState::default());
        }
        match load_document(&self.path) {
            Ok(state) => Ok(state),
            Err(primary_error) => {
                let backup = self.backup_path();
                if backup.is_file() {
                    load_document(&backup).map_err(|_| primary_error)
                } else {
                    Err(primary_error)
                }
            }
        }
    }

    fn save(&self, state: &AppState) -> CoreResult<()> {
        let document = MetadataDocument {
            schema_version: SCHEMA_VERSION,
            state: state.clone(),
        };
        if self.path.is_file()
            && let Ok(previous) = fs::read(&self.path)
            && serde_json::from_slice::<serde_json::Value>(&previous).is_ok()
        {
            write_bytes_atomic(&self.backup_path(), &previous)?;
        }
        write_atomic(&self.path, &document)
    }
}

fn load_document(path: &Path) -> CoreResult<AppState> {
    let bytes = fs::read(path).map_err(io_error)?;
    let mut document: MetadataDocument = serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::internal(format!("failed to parse metadata store: {error}")))?;
    let version = server_favorites::migrate_version(document.schema_version)?;
    document.schema_version = version;
    Ok(document.state)
}

pub(crate) fn write_atomic<T: Serialize>(path: &Path, value: &T) -> CoreResult<()> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| CoreError::internal(format!("failed to serialize store: {error}")))?;
    write_bytes_atomic(path, &bytes)
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let temporary = path.with_extension("tmp");
    write_private(&temporary, bytes)?;
    fs::rename(&temporary, path).map_err(io_error)
}

fn write_private(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(io_error)?;
    file.write_all(bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}

pub(crate) fn io_error(error: std::io::Error) -> CoreError {
    CoreError::internal(format!("storage I/O error: {error}"))
}
