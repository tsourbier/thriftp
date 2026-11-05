use std::fs;

use chrono::Utc;
use tempfile::TempDir;
use thriftp_core::automation::{
    AuditEvent, AuditStore, AutomationStore, base64_encode, canonical_remote, path_in_scope,
};

#[test]
fn enrollment_requires_approval_and_binds_a_grant() {
    let home = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let request = store
        .enroll(&base64_encode(&[9u8; 32]), "test client")
        .unwrap();
    assert!(store.grant_for(&request.fingerprint).is_none());

    let grant = store
        .approve(
            request.id,
            vec![uuid::Uuid::new_v4()],
            vec![(
                "local".to_owned(),
                None,
                scope.path().to_string_lossy().into_owned(),
            )],
            vec!["connections.list".to_owned()],
            None,
            "cancel",
            "cli",
        )
        .unwrap();
    assert_eq!(grant.disconnect_policy, "cancel");
    assert!(store.grant_for(&request.fingerprint).is_some());
    assert!(store.pending().is_empty());
}

#[test]
fn revocation_increments_epoch_and_removes_the_grant() {
    let home = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    let request = store.enroll(&base64_encode(&[8u8; 32]), "client").unwrap();
    let grant = store
        .approve(
            request.id,
            vec![uuid::Uuid::new_v4()],
            vec![(
                "local".to_owned(),
                None,
                scope.path().to_string_lossy().into_owned(),
            )],
            vec!["connections.list".to_owned()],
            None,
            "continue",
            "cli",
        )
        .unwrap();
    store.revoke(&request.fingerprint).unwrap();
    assert!(store.grant_for(&request.fingerprint).is_none());
    let revoked = store
        .state
        .grants
        .iter()
        .find(|candidate| candidate.id == grant.id)
        .unwrap();
    assert_eq!(revoked.state, "revoked");
    assert_eq!(revoked.revocation_epoch, 1);
}

#[test]
fn audit_chain_detects_tampering() {
    let home = TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let mut event = AuditEvent {
        seq: 0,
        timestamp: Utc::now(),
        interface: "mcp".to_owned(),
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
    assert!(store.verify().unwrap());

    let path = home
        .path()
        .join("audit")
        .join(format!("{}.jsonl", event.timestamp.format("%Y-%m-%d")));
    let content = fs::read_to_string(&path)
        .unwrap()
        .replace("allowed", "denied");
    fs::write(&path, content).unwrap();
    assert!(!store.verify().unwrap());
}

#[test]
fn remote_scope_rejects_traversal_and_sibling_prefixes() {
    assert!(canonical_remote("/srv/app/../etc").is_err());
    assert!(canonical_remote("/srv/%2e%2e/etc").is_err());
    assert!(path_in_scope("/srv/app/file", "/srv/app"));
    assert!(!path_in_scope("/srv/app2/file", "/srv/app"));
}
