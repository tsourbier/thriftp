//! Audit retention, storage limits, and the seven-day deletion notice.
//!
//! Retention never deletes an unsealed (current-day) segment. Before removing
//! old segments the newest removed hash is written to a checkpoint so the
//! remaining chain still verifies. Deletions are audited.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::{
    AuditEvent, AuditStore, DEFAULT_AUDIT_MAX_BYTES, DEFAULT_NOTICE_DAYS, DEFAULT_RETENTION_DAYS,
};
use crate::error::{CoreError, CoreResult};

/// Retention policy loaded from automation settings.
///
/// `retention_days` controls the age threshold, `notice_days` the notice window,
/// and `max_bytes` the optional audit storage limit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub retention_days: u32,
    pub notice_days: u32,
    pub max_bytes: Option<u64>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            retention_days: DEFAULT_RETENTION_DAYS,
            notice_days: DEFAULT_NOTICE_DAYS,
            max_bytes: Some(DEFAULT_AUDIT_MAX_BYTES),
        }
    }
}

/// Outcome of one retention evaluation.
///
/// Reports which segments were evaluated, expired, or deleted, and whether a
/// notice was issued or is still pending.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionReport {
    pub evaluated_segments: usize,
    pub expired_segments: usize,
    pub deleted_segments: Vec<String>,
    pub notice_issued: bool,
    pub notice_pending: bool,
    pub storage_limit_exceeded: bool,
    pub total_bytes: u64,
}

/// Apply retention and storage limits. Safe to call daily; idempotent when
/// nothing is eligible for deletion.
pub fn apply_retention(
    store: &AuditStore,
    policy: RetentionPolicy,
    now: DateTime<Utc>,
) -> CoreResult<RetentionReport> {
    let segments = store.segments()?;
    let today = now.date_naive();
    let cutoff = today - Duration::days(i64::from(policy.retention_days.max(1)));
    let mut report = RetentionReport {
        evaluated_segments: segments.len(),
        total_bytes: segments.iter().map(|segment| segment.bytes).sum(),
        ..RetentionReport::default()
    };

    let mut deletable = Vec::new();
    for segment in &segments {
        let Ok(date) = NaiveDate::parse_from_str(&segment.date, "%Y-%m-%d") else {
            continue;
        };
        if date < cutoff {
            report.expired_segments += 1;
            deletable.push(segment.clone());
        }
    }

    let over_limit = policy.max_bytes.is_some_and(|max| report.total_bytes > max);
    report.storage_limit_exceeded = over_limit;
    if over_limit {
        // Oldest sealed segments are eligible until the limit is satisfied.
        let mut remaining: u64 = report.total_bytes;
        for segment in &segments {
            if remaining <= policy.max_bytes.unwrap_or(u64::MAX) {
                break;
            }
            if segment.date.as_str() == today.format("%Y-%m-%d").to_string() {
                continue;
            }
            if !deletable.iter().any(|item| item.path == segment.path) {
                deletable.push(segment.clone());
            }
            remaining = remaining.saturating_sub(segment.bytes);
        }
    }

    if deletable.is_empty() {
        if over_limit {
            return Err(CoreError::audit_unavailable(
                "audit storage limit exceeded and no sealed segment can be safely deleted",
            ));
        }
        return Ok(report);
    }

    if report.expired_segments > 0 && !over_limit {
        if !notice_started(store, now)? {
            issue_notice(store, now)?;
            report.notice_issued = true;
            return Ok(report);
        }
        if !notice_elapsed(store, policy.notice_days.max(1), now)? {
            report.notice_pending = true;
            return Ok(report);
        }
    }

    // Anchor the chain at the newest hash being removed.
    if let Some(last) = deletable.last()
        && let Some((seq, hash)) = last_record(store, last)?
    {
        store.write_checkpoint(seq, &hash)?;
    }

    for segment in &deletable {
        std::fs::remove_file(&segment.path).map_err(|error| {
            CoreError::audit_unavailable(format!(
                "cannot delete expired audit segment {}: {error}",
                segment.path.display()
            ))
        })?;
        report.deleted_segments.push(segment.date.clone());
    }

    let mut event = retention_event("audit.retention_delete", &report);
    store.append(&mut event)?;
    let _ = clear_notice(store);
    Ok(report)
}

fn notice_path(store: &AuditStore) -> std::path::PathBuf {
    store.directory().join("retention.notice")
}

fn notice_started(store: &AuditStore, now: DateTime<Utc>) -> CoreResult<bool> {
    let path = notice_path(store);
    if path.is_file() {
        return Ok(true);
    }
    let _ = now;
    Ok(false)
}

fn issue_notice(store: &AuditStore, now: DateTime<Utc>) -> CoreResult<()> {
    std::fs::create_dir_all(store.directory())
        .map_err(|error| CoreError::audit_unavailable(format!("audit directory: {error}")))?;
    std::fs::write(notice_path(store), now.to_rfc3339()).map_err(|error| {
        CoreError::audit_unavailable(format!("cannot record retention notice: {error}"))
    })?;
    let mut event = retention_event("retention_notice", &RetentionReport::default());
    store.append(&mut event)
}

fn notice_elapsed(store: &AuditStore, notice_days: u32, now: DateTime<Utc>) -> CoreResult<bool> {
    let Ok(contents) = std::fs::read_to_string(notice_path(store)) else {
        return Ok(false);
    };
    let started = DateTime::parse_from_rfc3339(contents.trim())
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or(now);
    Ok(now - started >= Duration::days(i64::from(notice_days)))
}

fn clear_notice(store: &AuditStore) -> CoreResult<()> {
    let _ = std::fs::remove_file(notice_path(store));
    Ok(())
}

fn last_record(
    store: &AuditStore,
    segment: &super::AuditSegment,
) -> CoreResult<Option<(u64, String)>> {
    let contents = std::fs::read_to_string(&segment.path)
        .map_err(|error| CoreError::audit_unavailable(format!("audit read: {error}")))?;
    let Some(line) = contents.lines().rfind(|line| !line.trim().is_empty()) else {
        return Ok(None);
    };
    let event: AuditEvent = serde_json::from_str(line)
        .map_err(|error| CoreError::audit_unavailable(format!("audit parse: {error}")))?;
    let _ = store;
    Ok(Some((event.seq, event.hash)))
}

fn retention_event(operation: &str, report: &RetentionReport) -> AuditEvent {
    AuditEvent {
        seq: 0,
        timestamp: Utc::now(),
        interface: "system".to_owned(),
        mode: "host".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: operation.to_owned(),
        target: None,
        result: "completed".to_owned(),
        error_code: None,
        duration_ms: None,
        details: serde_json::to_value(report).ok(),
        prev_hash: String::new(),
        hash: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn append(store: &AuditStore, timestamp: DateTime<Utc>) {
        let mut event = AuditEvent {
            seq: 0,
            timestamp,
            interface: "test".to_owned(),
            mode: "standalone".to_owned(),
            client_fingerprint: None,
            client_display_name: None,
            session_id: None,
            operation: "test".to_owned(),
            target: None,
            result: "allowed".to_owned(),
            error_code: None,
            duration_ms: None,
            details: Some(serde_json::json!({"id": Uuid::new_v4()})),
            prev_hash: String::new(),
            hash: String::new(),
        };
        store.append(&mut event).unwrap();
    }

    #[test]
    fn notice_precedes_deletion_by_notice_days() {
        let home = tempfile::TempDir::new().unwrap();
        let store = AuditStore::open(home.path().join("audit"));
        let now = Utc::now();
        append(&store, now - Duration::days(100));
        let policy = RetentionPolicy {
            retention_days: 90,
            notice_days: 7,
            max_bytes: None,
        };
        let first = apply_retention(&store, policy, now).unwrap();
        assert!(first.notice_issued);
        let second = apply_retention(&store, policy, now + Duration::days(1)).unwrap();
        assert!(second.notice_pending);
        let third = apply_retention(&store, policy, now + Duration::days(8)).unwrap();
        assert!(!third.deleted_segments.is_empty());
        assert!(store.verify().unwrap());
    }
}
