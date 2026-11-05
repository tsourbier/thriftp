use std::path::Path;

use async_trait::async_trait;
use chrono::Utc;
use thriftp_core::model::{
    ConnectionProfile, EntryType, FileEntry, PartialTransferData, ProtocolCapabilities, Transfer,
    TransferDirection, TransferState,
};
use thriftp_core::protocol::{ProgressCallback, RemoteFs};
use thriftp_core::transfers::conflict::{ConflictChoice, ConflictPolicy, ConflictResolver};
use thriftp_core::transfers::control::TransferControl;
use thriftp_core::transfers::engine::{execute_download, execute_upload};
use thriftp_core::transfers::recursive::{SymlinkPolicy, plan_upload};
use thriftp_core::transfers::stream::copy_chunked;
use uuid::Uuid;

struct FakeRemote {
    payload: Vec<u8>,
    deleted: Vec<String>,
}

#[async_trait]
impl RemoteFs for FakeRemote {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        _credential: Option<thriftp_core::credentials::Secret>,
    ) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn disconnect(&mut self) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }
    async fn list_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }
    async fn stat(&mut self, path: &str) -> thriftp_core::CoreResult<FileEntry> {
        Err(thriftp_core::CoreError::NotFound(path.to_owned()))
    }
    async fn mkdir(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn rename(&mut self, _from: &str, _to: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn delete_file(&mut self, path: &str) -> thriftp_core::CoreResult<()> {
        self.deleted.push(path.to_owned());
        Ok(())
    }
    async fn delete_dir(&mut self, path: &str) -> thriftp_core::CoreResult<()> {
        self.deleted.push(path.to_owned());
        Ok(())
    }
    async fn chmod(&mut self, _path: &str, _mode: u32) -> thriftp_core::CoreResult<()> {
        Ok(())
    }

    async fn download_to(
        &mut self,
        _remote_path: &str,
        local_path: &Path,
        offset: u64,
        progress: ProgressCallback,
    ) -> thriftp_core::CoreResult<u64> {
        use tokio::io::{AsyncSeekExt, AsyncWriteExt};
        let mut output = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(local_path)
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        output
            .seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        let payload = &self.payload[offset as usize..];
        let mut moved = 0_u64;
        for chunk in payload.chunks(64 * 1024) {
            output
                .write_all(chunk)
                .await
                .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
            moved += chunk.len() as u64;
            if !progress(moved) {
                break;
            }
        }
        output
            .flush()
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        Ok(moved)
    }

    async fn upload_from(
        &mut self,
        local_path: &Path,
        _remote_path: &str,
        offset: u64,
        progress: ProgressCallback,
    ) -> thriftp_core::CoreResult<u64> {
        let bytes = tokio::fs::read(local_path)
            .await
            .map_err(|error| thriftp_core::CoreError::internal(error.to_string()))?;
        self.payload.extend_from_slice(&bytes[offset as usize..]);
        let moved = (bytes.len() as u64).saturating_sub(offset);
        let _ = progress(moved);
        Ok(moved)
    }
}

fn transfer(direction: TransferDirection, size: Option<u64>) -> Transfer {
    Transfer::new(direction, Uuid::new_v4(), "/source", "/destination", size)
}

#[tokio::test]
async fn chunked_copy_keeps_bounded_chunks() {
    let total = 256 * 1024;
    let (mut writer, mut reader) = tokio::io::duplex(16 * 1024);
    let producer = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;
        let chunk = vec![7_u8; 8 * 1024];
        for _ in 0..total / chunk.len() {
            writer.write_all(&chunk).await.unwrap();
        }
        writer.shutdown().await.unwrap();
    });

    let mut output = Vec::new();
    let stats = copy_chunked(&mut reader, &mut output, 16 * 1024, |_| true)
        .await
        .unwrap();
    producer.await.unwrap();

    assert_eq!(stats.bytes, total as u64);
    assert_eq!(stats.chunks, 16);
    assert_eq!(output.len(), total);
}

#[tokio::test]
async fn download_and_upload_preserve_progress_and_resume_offsets() {
    let directory = tempfile::tempdir().unwrap();
    let local = directory.path().join("file.bin");
    let payload = vec![9_u8; 128 * 1024];

    let mut remote = FakeRemote {
        payload,
        deleted: Vec::new(),
    };
    let mut download = transfer(TransferDirection::Download, Some(128 * 1024));
    let control = TransferControl::new();
    let run = execute_download(&mut remote, &mut download, &local, &control, None)
        .await
        .unwrap();
    assert_eq!(run.state, TransferState::Completed);
    assert_eq!(run.transferred_bytes, 128 * 1024);

    let mut upload = transfer(TransferDirection::Upload, Some(128 * 1024));
    let run = execute_upload(&mut remote, &mut upload, &local, &control, None)
        .await
        .unwrap();
    assert_eq!(run.state, TransferState::Completed);
    assert_eq!(run.transferred_bytes, 128 * 1024);
}

#[tokio::test]
async fn pause_and_cancel_stop_the_transfer() {
    let directory = tempfile::tempdir().unwrap();
    let local = directory.path().join("file.bin");
    let mut remote = FakeRemote {
        payload: vec![1_u8; 256 * 1024],
        deleted: Vec::new(),
    };

    let mut paused = transfer(TransferDirection::Download, Some(256 * 1024));
    let control = TransferControl::new();
    let stop = control.clone();
    let run = execute_download(
        &mut remote,
        &mut paused,
        &local,
        &control,
        Some(std::sync::Arc::new(move |_| stop.pause())),
    )
    .await
    .unwrap();
    assert_eq!(run.state, TransferState::Paused);
    assert!(run.transferred_bytes > 0);
    assert!(paused.resumable);

    let mut cancelled = transfer(TransferDirection::Download, Some(256 * 1024));
    let control = TransferControl::new();
    control.cancel();
    let run = execute_download(&mut remote, &mut cancelled, &local, &control, None)
        .await
        .unwrap();
    assert_eq!(run.state, TransferState::Cancelled);
}

#[test]
fn conflict_policies_and_apply_to_all_work() {
    let mut resolver = ConflictResolver::new(ConflictPolicy::Rename);
    assert_eq!(
        resolver
            .resolve("/tmp/report.txt", true, |_| ConflictChoice::Cancel)
            .unwrap(),
        ConflictChoice::Rename("/tmp/report (1).txt".to_owned())
    );

    let mut resolver = ConflictResolver::new(ConflictPolicy::Ask);
    resolver.set_apply_to_all(ConflictChoice::Skip);
    assert_eq!(
        resolver
            .resolve("/tmp/a", true, |_| ConflictChoice::Overwrite)
            .unwrap(),
        ConflictChoice::Skip
    );
}

#[test]
fn recursive_plan_skips_symlinks_and_keeps_structure() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("nested")).unwrap();
    std::fs::write(directory.path().join("root.txt"), b"root").unwrap();
    std::fs::write(directory.path().join("nested/child.txt"), b"child").unwrap();

    let plan = plan_upload(directory.path(), SymlinkPolicy::Skip).unwrap();
    let names: Vec<_> = plan
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect();
    assert_eq!(names, vec!["nested", "nested/child.txt", "root.txt"]);
    assert_eq!(
        plan.iter()
            .filter(|entry| entry.entry_type == EntryType::Directory)
            .count(),
        1
    );
}

#[test]
fn resume_refuses_a_changed_source() {
    let partial = PartialTransferData {
        id: Uuid::new_v4(),
        transfer_id: Uuid::new_v4(),
        local_path: Some("/tmp/partial".to_owned()),
        remote_path: None,
        offset: 100,
        source_size: Some(200),
        source_modified_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    assert!(thriftp_core::transfers::resume::verify(&partial, Some(201), None, 100).is_err());
    assert!(thriftp_core::transfers::resume::verify(&partial, Some(200), None, 100).is_ok());
}
