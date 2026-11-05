use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::error::{CoreError, CoreResult};
use crate::model::FileEntry;
use crate::protocol::RemoteFs;

/// Number of entries sent to the UI in a single batch.
pub const DEFAULT_BATCH_SIZE: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ListingState {
    Partial,
    Complete,
}

#[derive(Debug, Clone)]
pub struct ListingBatch {
    pub entries: Vec<FileEntry>,
    pub state: ListingState,
    pub first_entry_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Cancellation handle for one streaming listing.
#[derive(Debug, Clone, Default)]
pub struct ListingSession {
    cancelled: Arc<AtomicBool>,
}

impl ListingSession {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

fn emit_batch<F>(
    session: &ListingSession,
    batch: &mut Vec<FileEntry>,
    state: ListingState,
    first_entry_at: &mut Option<DateTime<Utc>>,
    emit: &mut F,
) -> CoreResult<()>
where
    F: FnMut(ListingBatch),
{
    if batch.is_empty() && state == ListingState::Partial {
        return Ok(());
    }
    if first_entry_at.is_none() && !batch.is_empty() {
        *first_entry_at = Some(Utc::now());
    }
    let entries = std::mem::take(batch);
    emit(ListingBatch {
        entries,
        state,
        first_entry_at: *first_entry_at,
        completed_at: (state == ListingState::Complete).then(Utc::now),
    });
    if session.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    Ok(())
}

/// Stream a local directory in bounded batches.
///
/// The completion batch is emitted exactly once when enumeration finishes.
/// Cancellation stops further enumeration and emits no completion batch.
pub fn stream_local<F>(
    path: &Path,
    batch_size: usize,
    session: &ListingSession,
    mut emit: F,
) -> CoreResult<()>
where
    F: FnMut(ListingBatch),
{
    let batch_size = batch_size.max(1);
    let mut buffer = Vec::with_capacity(batch_size);
    let mut first_entry_at = None;
    for entry in std::fs::read_dir(path)
        .map_err(|error| CoreError::internal(format!("filesystem listing error: {error}")))?
    {
        if session.is_cancelled() {
            return Err(CoreError::Cancelled);
        }
        let entry = entry
            .map_err(|error| CoreError::internal(format!("filesystem listing error: {error}")))?;
        buffer.push(super::file_entry_from_dir_entry(&entry)?);
        if buffer.len() >= batch_size {
            emit_batch(
                session,
                &mut buffer,
                ListingState::Partial,
                &mut first_entry_at,
                &mut emit,
            )?;
        }
    }
    emit_batch(
        session,
        &mut buffer,
        ListingState::Complete,
        &mut first_entry_at,
        &mut emit,
    )
}

/// Stream a remote directory in bounded batches.
pub async fn stream_remote<F>(
    remote: &mut dyn RemoteFs,
    path: &str,
    batch_size: usize,
    session: &ListingSession,
    mut emit: F,
) -> CoreResult<()>
where
    F: FnMut(ListingBatch) + Send,
{
    let batch_size = batch_size.max(1);
    if session.is_cancelled() {
        return Err(CoreError::Cancelled);
    }
    let mut first_entry_at = None;
    let mut pending: Option<Vec<FileEntry>> = None;
    let result = remote
        .list_dir_stream(path, batch_size, &mut |entries| {
            if entries.is_empty() {
                return true;
            }
            if first_entry_at.is_none() {
                first_entry_at = Some(Utc::now());
            }
            if let Some(previous) = pending.replace(entries) {
                emit(ListingBatch {
                    entries: previous,
                    state: ListingState::Partial,
                    first_entry_at,
                    completed_at: None,
                });
            }
            !session.is_cancelled()
        })
        .await;
    result?;
    emit(ListingBatch {
        entries: pending.unwrap_or_default(),
        state: ListingState::Complete,
        first_entry_at,
        completed_at: Some(Utc::now()),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str) -> FileEntry {
        FileEntry {
            name: name.to_owned(),
            path: name.to_owned(),
            entry_type: crate::model::EntryType::File,
            size: Some(1),
            modified_at: None,
            permissions: None,
            ownership: None,
            hidden: false,
            symlink_target: None,
            capabilities: crate::model::ProtocolCapabilities::default(),
        }
    }

    #[test]
    fn batches_emit_partial_then_complete() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(dir.path().join(format!("file-{index}")), b"x").unwrap();
        }
        let session = ListingSession::new();
        let mut batches = Vec::new();
        stream_local(dir.path(), 2, &session, |batch| batches.push(batch)).unwrap();

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].state, ListingState::Partial);
        assert_eq!(batches[1].state, ListingState::Partial);
        assert_eq!(batches[2].state, ListingState::Complete);
        assert_eq!(
            batches
                .iter()
                .map(|batch| batch.entries.len())
                .sum::<usize>(),
            5
        );
        assert!(batches[0].first_entry_at.is_some());
        assert!(batches[2].completed_at.is_some());
    }

    #[test]
    fn cancellation_stops_enumeration() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(dir.path().join(format!("file-{index}")), b"x").unwrap();
        }
        let session = ListingSession::new();
        let mut batches = 0;
        let result = stream_local(dir.path(), 1, &session, |_| {
            batches += 1;
            session.cancel();
        });
        assert_eq!(result, Err(CoreError::Cancelled));
        assert_eq!(batches, 1);
    }

    #[test]
    fn empty_directory_still_completes() {
        let dir = tempfile::tempdir().unwrap();
        let session = ListingSession::new();
        let mut batches = Vec::new();
        stream_local(dir.path(), 10, &session, |batch| batches.push(batch)).unwrap();
        assert_eq!(batches.len(), 1);
        assert!(batches[0].entries.is_empty());
        assert_eq!(batches[0].state, ListingState::Complete);
        assert!(batches[0].first_entry_at.is_none());
    }

    #[test]
    fn entries_keep_directory_order_from_the_source() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("b"), b"x").unwrap();
        std::fs::write(dir.path().join("a"), b"x").unwrap();
        let session = ListingSession::new();
        let mut names = Vec::new();
        stream_local(dir.path(), 10, &session, |batch| {
            names.extend(batch.entries.into_iter().map(|entry| entry.name));
        })
        .unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"a".to_owned()));
        assert!(names.contains(&"b".to_owned()));
    }

    #[test]
    fn batch_entry_helper_is_a_file() {
        assert_eq!(entry("a").name, "a");
    }
}
