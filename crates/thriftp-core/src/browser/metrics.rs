use std::time::{Duration, Instant};

use crate::error::CoreResult;
use crate::model::FileEntry;
use crate::protocol::RemoteFs;

/// Timing information for one directory listing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListingMetrics {
    pub entry_count: usize,
    pub batch_count: usize,
    pub first_entry: Option<Duration>,
    pub total: Duration,
}

impl ListingMetrics {
    #[must_use]
    pub fn first_entry_millis(&self) -> Option<u128> {
        self.first_entry.map(|duration| duration.as_millis())
    }

    #[must_use]
    pub fn total_millis(&self) -> u128 {
        self.total.as_millis()
    }
}

/// Collect timing data for a local listing operation.
pub fn measure_local<F>(operation: F) -> CoreResult<(Vec<FileEntry>, ListingMetrics)>
where
    F: FnOnce() -> CoreResult<Vec<FileEntry>>,
{
    let started = Instant::now();
    let entries = operation()?;
    let entry_count = entries.len();
    Ok((entries, metrics(started, entry_count, 1)))
}

/// Collect timing data for a remote listing operation.
pub async fn measure_remote(
    remote: &mut dyn RemoteFs,
    path: &str,
) -> CoreResult<(Vec<FileEntry>, ListingMetrics)> {
    let started = Instant::now();
    let entries = remote.list_dir(path).await?;
    let entry_count = entries.len();
    Ok((entries, metrics(started, entry_count, 1)))
}

fn metrics(started: Instant, entry_count: usize, batch_count: usize) -> ListingMetrics {
    let total = started.elapsed();
    tracing::debug!(
        entry_count,
        batch_count,
        total_millis = total.as_millis(),
        "directory listing completed"
    );
    ListingMetrics {
        entry_count,
        batch_count,
        // Synchronous measurements only observe the completed result, so the
        // first-entry timestamp is unavailable rather than fabricated.
        first_entry: None,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_duration_and_entry_count() {
        let (entries, metrics) = measure_local(|| Ok(Vec::new())).unwrap();
        assert!(entries.is_empty());
        assert_eq!(metrics.entry_count, 0);
        assert_eq!(metrics.batch_count, 1);
        assert!(metrics.total >= Duration::ZERO);
    }
}
