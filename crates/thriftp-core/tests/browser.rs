use async_trait::async_trait;
use thriftp_core::browser::listing::{ListingBatch, ListingSession, stream_remote};
use thriftp_core::browser::{self, delete, metrics};
use thriftp_core::credentials::Secret;
use thriftp_core::error::CoreError;
use thriftp_core::model::{ConnectionProfile, EntryType, FileEntry, ProtocolCapabilities, SortKey};
use thriftp_core::previews::{PreviewMode, plan_local, plan_remote};
use thriftp_core::protocol::RemoteFs;
use thriftp_core::sorting::sort_file_entries;

struct FakeRemote {
    entries: Vec<FileEntry>,
    deleted: Vec<String>,
}

#[async_trait]
impl RemoteFs for FakeRemote {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        _credential: Option<Secret>,
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
        Ok(self.entries.clone())
    }

    async fn stat(&mut self, path: &str) -> thriftp_core::CoreResult<FileEntry> {
        self.entries
            .iter()
            .find(|entry| entry.path == path)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(path.to_owned()))
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
}

fn entry(name: &str, size: Option<u64>, entry_type: EntryType) -> FileEntry {
    FileEntry {
        name: name.to_owned(),
        path: format!("/remote/{name}"),
        entry_type,
        size,
        modified_at: None,
        permissions: Some("644".to_owned()),
        ownership: None,
        hidden: name.starts_with('.'),
        symlink_target: None,
        capabilities: ProtocolCapabilities::default(),
    }
}

#[test]
fn local_listing_reports_metadata_hidden_files_and_sorting() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("visible.txt"), b"hello").unwrap();
    std::fs::write(directory.path().join(".hidden.txt"), b"x").unwrap();
    std::fs::create_dir(directory.path().join("folder")).unwrap();

    let entries = browser::list_local(directory.path()).unwrap();
    assert_eq!(entries.len(), 3);
    assert!(
        entries
            .iter()
            .any(|entry| entry.name == ".hidden.txt" && entry.hidden)
    );
    let file = entries
        .iter()
        .find(|entry| entry.name == "visible.txt")
        .unwrap();
    assert_eq!(file.size, Some(5));
    assert_eq!(file.entry_type, EntryType::File);

    let mut sorted = entries.clone();
    sort_file_entries(&mut sorted, SortKey::Size);
    let names: Vec<_> = sorted.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(names, vec!["folder", ".hidden.txt", "visible.txt"]);
}

#[test]
fn listing_metrics_record_the_completed_listing() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("a"), b"x").unwrap();
    let (entries, metrics) =
        metrics::measure_local(|| browser::list_local(directory.path())).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(metrics.entry_count, 1);
    assert_eq!(metrics.batch_count, 1);
}

#[tokio::test]
async fn remote_listing_streams_batches() {
    let entries = (0..450)
        .map(|index| entry(&format!("file-{index}"), Some(index), EntryType::File))
        .collect();
    let mut remote = FakeRemote {
        entries,
        deleted: Vec::new(),
    };
    let session = ListingSession::new();
    let mut batches: Vec<ListingBatch> = Vec::new();
    stream_remote(&mut remote, "/remote", 200, &session, |batch| {
        batches.push(batch)
    })
    .await
    .unwrap();

    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].entries.len(), 200);
    assert_eq!(batches[1].entries.len(), 200);
    assert_eq!(batches[2].entries.len(), 50);
    assert_eq!(
        batches.last().unwrap().state,
        thriftp_core::browser::listing::ListingState::Complete
    );
}

#[tokio::test]
async fn remote_deletion_requires_confirmation_and_marks_the_entry() {
    let file = entry("old.txt", Some(1), EntryType::File);
    let mut remote = FakeRemote {
        entries: vec![file.clone()],
        deleted: Vec::new(),
    };

    let error = delete::delete_remote(&mut remote, &file.path, file.entry_type, false)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::Conflict(_)));
    assert!(remote.deleted.is_empty());

    delete::delete_remote(&mut remote, &file.path, file.entry_type, true)
        .await
        .unwrap();
    assert_eq!(remote.deleted, vec![file.path]);
}

#[test]
fn preview_fallback_respects_size_and_content() {
    let directory = tempfile::tempdir().unwrap();
    let text = directory.path().join("notes.txt");
    std::fs::write(&text, b"hello").unwrap();
    let plan = plan_local(&text).unwrap();
    assert_eq!(plan.mode, PreviewMode::Internal);

    let large = directory.path().join("large.txt");
    let file = std::fs::File::create(&large).unwrap();
    file.set_len(thriftp_core::previews::MAX_INLINE_BYTES + 1)
        .unwrap();
    assert_eq!(plan_local(&large).unwrap().mode, PreviewMode::Metadata);

    assert_eq!(
        plan_remote(Some(10), "notes.txt", false).mode,
        PreviewMode::Metadata
    );
}

#[test]
fn local_delete_reports_not_found_for_missing_paths() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.txt");
    assert!(matches!(
        delete::delete_local(&missing),
        Err(CoreError::NotFound(_))
    ));
}
