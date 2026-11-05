//! Audit fail-closed behavior: an unwritable audit store blocks results.

use tempfile::TempDir;
use thriftp_core::automation::service::AutomationService;
use thriftp_core::automation::{AuditEvent, AuditStore, AutomationStore};

fn event() -> AuditEvent {
    AuditEvent {
        seq: 0,
        timestamp: chrono::Utc::now(),
        interface: "mcp".to_owned(),
        mode: "standalone".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: "files.delete".to_owned(),
        target: None,
        result: "allowed".to_owned(),
        error_code: None,
        duration_ms: None,
        details: None,
        prev_hash: String::new(),
        hash: String::new(),
    }
}

#[test]
fn unwritable_audit_store_blocks_the_operation() {
    let home = TempDir::new().unwrap();
    let blocker = home.path().join("audit-is-a-file");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let store = AuditStore::open(blocker.join("audit"));
    let error = store.append(&mut event()).unwrap_err();
    assert_eq!(error.code(), "audit_unavailable");
    assert!(error.to_string().contains("audit directory is unavailable"));
}

#[test]
fn automation_service_propagates_audit_failure() {
    let home = TempDir::new().unwrap();
    let blocker = home.path().join("audit-is-a-file");
    std::fs::write(&blocker, b"not a directory").unwrap();
    let mut service = AutomationService::new(
        AutomationStore::open(home.path().join("automation.json")).unwrap(),
        AuditStore::open(blocker.join("audit")),
    );
    let error = service.enable("standalone").unwrap_err();
    assert_eq!(error.code(), "audit_unavailable");
}

#[test]
fn allowed_denied_blocked_and_failed_operations_are_recorded() {
    let home = TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    for (result, error) in [
        ("allowed", None),
        ("denied", Some("scope_denied")),
        ("blocked", Some("confirmation_required")),
        ("failed", Some("network")),
    ] {
        let mut event = event();
        event.result = result.to_owned();
        event.error_code = error.map(str::to_owned);
        store.append(&mut event).unwrap();
    }
    assert_eq!(store.list(usize::MAX).unwrap().len(), 4);
    assert!(store.verify().unwrap());
}
