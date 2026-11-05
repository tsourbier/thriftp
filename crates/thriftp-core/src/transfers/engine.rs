use std::path::Path;
use std::sync::Arc;

use chrono::Utc;

use crate::error::{CoreError, CoreResult};
use crate::model::{Transfer, TransferState};
use crate::protocol::{ProgressCallback, RemoteFs};

use super::control::TransferControl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRun {
    pub state: TransferState,
    pub transferred_bytes: u64,
}

/// Callback invoked with the cumulative number of bytes transferred.
pub type ProgressObserver = Arc<dyn Fn(u64) + Send + Sync>;

fn mark_active(transfer: &mut Transfer) {
    transfer.state = TransferState::Active;
    transfer.started_at.get_or_insert_with(Utc::now);
    transfer.updated_at = Utc::now();
    transfer.error = None;
}

fn mark_finished(transfer: &mut Transfer, state: TransferState) {
    transfer.state = state;
    transfer.updated_at = Utc::now();
    if matches!(
        state,
        TransferState::Completed | TransferState::Failed | TransferState::Cancelled
    ) {
        transfer.completed_at = Some(Utc::now());
    }
    transfer.resumable = matches!(
        state,
        TransferState::Paused | TransferState::Failed | TransferState::Interrupted
    );
}

fn adapter_progress(
    control: &TransferControl,
    offset: u64,
    observer: Option<ProgressObserver>,
) -> ProgressCallback {
    let control = control.clone();
    Arc::new(move |moved| {
        let total = offset.saturating_add(moved);
        control.set_transferred(total);
        if let Some(observer) = &observer {
            observer(total);
        }
        control.should_continue()
    })
}

/// Download a remote file into a local destination.
pub async fn execute_download(
    remote: &mut dyn RemoteFs,
    transfer: &mut Transfer,
    local_destination: &Path,
    control: &TransferControl,
    observer: Option<ProgressObserver>,
) -> CoreResult<TransferRun> {
    if control.is_cancelled() {
        mark_finished(transfer, TransferState::Cancelled);
        return Ok(TransferRun {
            state: TransferState::Cancelled,
            transferred_bytes: transfer.transferred_bytes,
        });
    }
    if control.is_paused() {
        mark_finished(transfer, TransferState::Paused);
        return Ok(TransferRun {
            state: TransferState::Paused,
            transferred_bytes: transfer.transferred_bytes,
        });
    }
    mark_active(transfer);
    let offset = transfer.transferred_bytes;
    control.set_transferred(offset);
    let progress = adapter_progress(control, offset, observer);

    match remote
        .download_to(&transfer.source_path, local_destination, offset, progress)
        .await
    {
        Ok(moved) => {
            transfer.transferred_bytes = offset.saturating_add(moved);
            let state = control.outcome();
            mark_finished(transfer, state);
            Ok(TransferRun {
                state,
                transferred_bytes: transfer.transferred_bytes,
            })
        }
        Err(error) => {
            transfer.error = Some(error.to_string());
            mark_finished(transfer, TransferState::Failed);
            Err(error)
        }
    }
}

/// Upload a local file to a remote destination.
pub async fn execute_upload(
    remote: &mut dyn RemoteFs,
    transfer: &mut Transfer,
    local_source: &Path,
    control: &TransferControl,
    observer: Option<ProgressObserver>,
) -> CoreResult<TransferRun> {
    let destination = transfer.destination_path.clone();
    execute_upload_to(
        remote,
        transfer,
        local_source,
        &destination,
        control,
        observer,
    )
    .await
}

/// Upload a local file to an explicit remote destination.
///
/// This allows callers to stream to a temporary partial path and rename it to
/// `transfer.destination_path` only after the transfer completes.
pub async fn execute_upload_to(
    remote: &mut dyn RemoteFs,
    transfer: &mut Transfer,
    local_source: &Path,
    remote_destination: &str,
    control: &TransferControl,
    observer: Option<ProgressObserver>,
) -> CoreResult<TransferRun> {
    if control.is_cancelled() {
        mark_finished(transfer, TransferState::Cancelled);
        return Ok(TransferRun {
            state: TransferState::Cancelled,
            transferred_bytes: transfer.transferred_bytes,
        });
    }
    if control.is_paused() {
        mark_finished(transfer, TransferState::Paused);
        return Ok(TransferRun {
            state: TransferState::Paused,
            transferred_bytes: transfer.transferred_bytes,
        });
    }
    mark_active(transfer);
    let offset = transfer.transferred_bytes;
    control.set_transferred(offset);
    let progress = adapter_progress(control, offset, observer);

    match remote
        .upload_from(local_source, remote_destination, offset, progress)
        .await
    {
        Ok(moved) => {
            transfer.transferred_bytes = offset.saturating_add(moved);
            let state = control.outcome();
            mark_finished(transfer, state);
            Ok(TransferRun {
                state,
                transferred_bytes: transfer.transferred_bytes,
            })
        }
        Err(error) => {
            transfer.error = Some(error.to_string());
            mark_finished(transfer, TransferState::Failed);
            Err(error)
        }
    }
}

/// Verify that the native protocol supports streaming transfers.
pub fn ensure_streaming_supported(remote: &dyn RemoteFs) -> CoreResult<()> {
    let capabilities = remote.capabilities();
    if capabilities.resume {
        Ok(())
    } else {
        Err(CoreError::UnsupportedCapability(
            "this server does not support resumable transfers".to_owned(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Priority, TransferDirection};
    use crate::protocol::ProgressCallback;
    use async_trait::async_trait;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct FakeRemote {
        payload: Vec<u8>,
    }

    #[async_trait]
    impl RemoteFs for FakeRemote {
        async fn connect(
            &mut self,
            _profile: &crate::model::ConnectionProfile,
            _credential: Option<crate::credentials::Secret>,
        ) -> CoreResult<()> {
            Ok(())
        }
        async fn disconnect(&mut self) -> CoreResult<()> {
            Ok(())
        }
        fn capabilities(&self) -> crate::model::ProtocolCapabilities {
            crate::model::ProtocolCapabilities::default()
        }
        async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<crate::model::FileEntry>> {
            Ok(Vec::new())
        }
        async fn stat(&mut self, path: &str) -> CoreResult<crate::model::FileEntry> {
            Err(CoreError::NotFound(path.to_owned()))
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
        async fn download_to(
            &mut self,
            _remote_path: &str,
            local_path: &Path,
            offset: u64,
            progress: ProgressCallback,
        ) -> CoreResult<u64> {
            use tokio::io::AsyncSeekExt;
            use tokio::io::AsyncWriteExt;
            let payload = &self.payload[offset as usize..];
            let mut output = tokio::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(false)
                .open(local_path)
                .await
                .unwrap();
            output.seek(std::io::SeekFrom::Start(offset)).await.unwrap();
            // Emit in small chunks so pause/cancel callbacks are exercised.
            let mut sent = 0_u64;
            for chunk in payload.chunks(4) {
                output.write_all(chunk).await.unwrap();
                sent += chunk.len() as u64;
                if !progress(sent) {
                    break;
                }
            }
            output.flush().await.unwrap();
            Ok(sent)
        }
        async fn upload_from(
            &mut self,
            local_path: &Path,
            _remote_path: &str,
            offset: u64,
            progress: ProgressCallback,
        ) -> CoreResult<u64> {
            let bytes = tokio::fs::read(local_path).await.unwrap();
            self.payload.extend_from_slice(&bytes[offset as usize..]);
            let moved = (bytes.len() as u64).saturating_sub(offset);
            let _ = progress(moved);
            Ok(moved)
        }
    }

    fn transfer(direction: TransferDirection, source: &str, destination: &str) -> Transfer {
        Transfer::new(direction, Uuid::new_v4(), source, destination, Some(8))
    }

    #[tokio::test]
    async fn download_completes_and_reports_progress() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("file.bin");
        let mut remote = FakeRemote {
            payload: b"abcdefgh".to_vec(),
        };
        let mut transfer = transfer(TransferDirection::Download, "/remote/file.bin", "file.bin");
        let control = TransferControl::new();

        let run = execute_download(
            &mut remote,
            &mut transfer,
            &destination,
            &control,
            Some(Arc::new(|_| {})),
        )
        .await
        .unwrap();

        assert_eq!(run.state, TransferState::Completed);
        assert_eq!(run.transferred_bytes, 8);
        assert_eq!(std::fs::read(&destination).unwrap(), b"abcdefgh");
        assert!(transfer.completed_at.is_some());
    }

    #[tokio::test]
    async fn pause_stops_at_a_chunk_boundary_and_preserves_progress() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("file.bin");
        let mut remote = FakeRemote {
            payload: b"abcdefgh".to_vec(),
        };
        let mut transfer = transfer(TransferDirection::Download, "/remote/file.bin", "file.bin");
        let control = TransferControl::new();
        let observer_control = control.clone();
        let stop = Arc::new(move |_| observer_control.pause());

        let run = execute_download(
            &mut remote,
            &mut transfer,
            &destination,
            &control,
            Some(stop),
        )
        .await
        .unwrap();

        assert_eq!(run.state, TransferState::Paused);
        assert!(transfer.transferred_bytes > 0);
        assert!(transfer.resumable);
    }

    #[tokio::test]
    async fn cancel_is_terminal_and_does_not_complete() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("file.bin");
        let mut remote = FakeRemote {
            payload: b"abcdefgh".to_vec(),
        };
        let mut transfer = transfer(TransferDirection::Download, "/remote/file.bin", "file.bin");
        let control = TransferControl::new();
        control.cancel();

        let run = execute_download(&mut remote, &mut transfer, &destination, &control, None)
            .await
            .unwrap();

        assert_eq!(run.state, TransferState::Cancelled);
        assert!(transfer.completed_at.is_some());
    }

    #[tokio::test]
    async fn upload_records_the_moved_bytes() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("file.bin");
        std::fs::write(&source, b"abcdefgh").unwrap();
        let mut remote = FakeRemote {
            payload: Vec::new(),
        };
        let mut transfer = transfer(TransferDirection::Upload, "file.bin", "/remote/file.bin");
        transfer.priority = Priority::High;
        let control = TransferControl::new();

        let run = execute_upload(&mut remote, &mut transfer, &source, &control, None)
            .await
            .unwrap();

        assert_eq!(run.state, TransferState::Completed);
        assert_eq!(remote.payload, b"abcdefgh");
        assert_eq!(run.transferred_bytes, 8);
    }

    #[test]
    fn default_transfer_is_pending() {
        let transfer = transfer(TransferDirection::Download, "a", "b");
        assert_eq!(transfer.state, TransferState::Pending);
        assert_eq!(transfer.destination_path, "b");
        assert!(PathBuf::from(transfer.source_path).is_relative());
    }
}
