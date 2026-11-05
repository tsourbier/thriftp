use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;

use crate::credentials::Secret;
use crate::error::{CoreError, CoreResult};
use crate::model::{ConnectionProfile, FileEntry, Protocol, ProtocolCapabilities};

/// Size of the chunks moved by streaming transfers.
pub const TRANSFER_CHUNK_SIZE: usize = 64 * 1024;

/// Called after each chunk with the number of bytes moved during this call.
///
/// Returning `false` requests that the transfer stop at the next chunk
/// boundary so the engine can pause or cancel without buffering the file.
pub type ProgressCallback = Arc<dyn Fn(u64) -> bool + Send + Sync>;

#[async_trait]
pub trait RemoteFs: Send + Sync {
    async fn connect(
        &mut self,
        profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()>;

    async fn disconnect(&mut self) -> CoreResult<()>;

    fn capabilities(&self) -> ProtocolCapabilities;

    async fn list_dir(&mut self, path: &str) -> CoreResult<Vec<FileEntry>>;

    /// Stream directory entries to `emit` in bounded batches.
    ///
    /// The default implementation adapts [`RemoteFs::list_dir`]; protocol
    /// adapters that can enumerate incrementally should override it so the UI
    /// can render the first entries before the listing completes.
    async fn list_dir_stream(
        &mut self,
        path: &str,
        batch_size: usize,
        emit: &mut (dyn FnMut(Vec<FileEntry>) -> bool + Send),
    ) -> CoreResult<()> {
        let entries = self.list_dir(path).await?;
        let batch_size = batch_size.max(1);
        for chunk in entries.chunks(batch_size) {
            if !emit(chunk.to_vec()) {
                return Err(CoreError::Cancelled);
            }
        }
        Ok(())
    }

    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry>;

    async fn mkdir(&mut self, path: &str) -> CoreResult<()>;

    async fn rename(&mut self, from: &str, to: &str) -> CoreResult<()>;

    async fn delete_file(&mut self, path: &str) -> CoreResult<()>;

    async fn delete_dir(&mut self, path: &str) -> CoreResult<()>;

    async fn chmod(&mut self, path: &str, _mode: u32) -> CoreResult<()>;

    /// Create a symbolic link pointing at `target`.
    async fn create_symlink(&mut self, _target: &str, _link_path: &str) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "this protocol cannot create symbolic links".to_owned(),
        ))
    }

    /// Stream a remote file into a local file starting at `offset`.
    ///
    /// Returns the number of bytes moved. Implementations must use bounded
    /// buffers and must call `progress` after every chunk.
    async fn download_to(
        &mut self,
        _remote_path: &str,
        _local_path: &Path,
        _offset: u64,
        _progress: ProgressCallback,
    ) -> CoreResult<u64> {
        Err(CoreError::UnsupportedCapability(
            "this protocol does not support streaming downloads".to_owned(),
        ))
    }

    /// Stream a local file to a remote file starting at `offset`.
    async fn upload_from(
        &mut self,
        _local_path: &Path,
        _remote_path: &str,
        _offset: u64,
        _progress: ProgressCallback,
    ) -> CoreResult<u64> {
        Err(CoreError::UnsupportedCapability(
            "this protocol does not support streaming uploads".to_owned(),
        ))
    }
}

pub trait RemoteFsFactory: Send + Sync {
    fn create(&self, protocol: Protocol) -> Box<dyn RemoteFs>;
}

/// Machine-readable warning emitted for unencrypted transports.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransportWarning {
    pub code: String,
    pub message: String,
}

/// Return the plain-FTP warning required by every profile, browse, stat,
/// upload, download, transfer-status, and transfer-list result.
#[must_use]
pub fn plain_ftp_warning(protocol: Protocol) -> Option<TransportWarning> {
    (protocol == Protocol::Ftp).then(|| TransportWarning {
        code: "unencrypted_transport".to_owned(),
        message:
            "This connection uses plain FTP; credentials and data are not encrypted in transit."
                .to_owned(),
    })
}
