//! Retention, storage-limit, notice, and deletion-audit scenarios.

use chrono::{Duration, Utc};
use thriftp_core::automation::retention::{RetentionPolicy, apply_retention};
use thriftp_core::automation::{AuditEvent, AuditStore};

fn append(store: &AuditStore, timestamp: chrono::DateTime<Utc>) {
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
        details: None,
        prev_hash: String::new(),
        hash: String::new(),
    };
    store.append(&mut event).unwrap();
}

#[test]
fn retention_requires_a_seven_day_notice_before_deletion() {
    let home = tempfile::TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let now = Utc::now();
    append(&store, now - Duration::days(120));
    let policy = RetentionPolicy {
        retention_days: 90,
        notice_days: 7,
        max_bytes: None,
    };
    let notice = apply_retention(&store, policy, now).unwrap();
    assert!(notice.notice_issued);
    assert!(notice.deleted_segments.is_empty());
    let pending = apply_retention(&store, policy, now + Duration::days(6)).unwrap();
    assert!(pending.notice_pending);
    let deleted = apply_retention(&store, policy, now + Duration::days(8)).unwrap();
    assert_eq!(deleted.deleted_segments.len(), 1);
    assert!(store.verify().unwrap());
}

#[test]
fn storage_limit_deletes_the_oldest_sealed_segments() {
    let home = tempfile::TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let now = Utc::now();
    for days in [30, 20, 10] {
        append(&store, now - Duration::days(days));
    }
    let policy = RetentionPolicy {
        retention_days: 365,
        notice_days: 7,
        max_bytes: Some(1),
    };
    let report = apply_retention(&store, policy, now).unwrap();
    assert!(report.storage_limit_exceeded);
    assert!(!report.deleted_segments.is_empty());
    assert!(store.verify().unwrap());
}

#[test]
fn current_day_segment_is_never_deleted() {
    let home = tempfile::TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let now = Utc::now();
    append(&store, now);
    let policy = RetentionPolicy {
        retention_days: 1,
        notice_days: 1,
        max_bytes: Some(1),
    };
    let result = apply_retention(&store, policy, now);
    assert!(matches!(
        result,
        Err(thriftp_core::CoreError::AuditUnavailable(_))
    ));
    assert!(store.verify().unwrap());
}
