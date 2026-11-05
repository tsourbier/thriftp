//! Desktop automation command tests delegate to the shared core.

use tempfile::TempDir;
use thriftp_core::automation::{AuditStore, AutomationStore, base64_encode};
use uuid::Uuid;

fn grant(store: &mut AutomationStore) -> (String, Uuid) {
    let scope = TempDir::new().unwrap();
    let request = store
        .enroll(&base64_encode(&[11u8; 32]), "desktop agent")
        .unwrap();
    let grant = store
        .approve(
            request.id,
            vec![Uuid::new_v4()],
            vec![(
                "local".to_owned(),
                None,
                scope.path().to_string_lossy().into_owned(),
            )],
            vec!["browse.list".to_owned()],
            None,
            "continue",
            "desktop",
        )
        .unwrap();
    (request.fingerprint, grant.id)
}

#[test]
fn enable_approve_revoke_and_audit_are_visible() {
    let home = TempDir::new().unwrap();
    let path = home.path().join("automation.json");
    let mut store = AutomationStore::open(&path).unwrap();
    store.enable("embedded").unwrap();
    let (fingerprint, grant_id) = grant(&mut store);
    assert!(store.state.settings.mcp_enabled);
    assert!(store.grant_for(&fingerprint).is_some());
    store.revoke(&fingerprint).unwrap();
    assert!(store.grant_for(&fingerprint).is_none());
    assert!(
        store
            .state
            .grants
            .iter()
            .any(|grant| grant.id == grant_id && grant.state == "revoked")
    );
    store.disable().unwrap();
    assert!(!store.state.settings.mcp_enabled);
}

#[test]
fn audit_verify_detects_tampering_from_the_desktop_surface() {
    let home = TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let mut event = thriftp_core::automation::AuditEvent {
        seq: 0,
        timestamp: chrono::Utc::now(),
        interface: "desktop".to_owned(),
        mode: "embedded".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: "mcp.clients.approve".to_owned(),
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
    let contents = std::fs::read_to_string(&path)
        .unwrap()
        .replace("allowed", "denied");
    std::fs::write(path, contents).unwrap();
    assert!(!store.verify().unwrap());
}
