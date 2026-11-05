//! Secret-redaction and no-leak security scenarios.

use std::fs;

use tempfile::TempDir;
use thriftp_core::automation::{AuditEvent, AuditStore};
use thriftp_core::error::CoreError;
use thriftp_core::observability::{REDACTED, redact_text, redact_value};

#[test]
fn credentials_and_tokens_never_appear_in_redacted_output() {
    for secret in [
        "password=hunter2",
        "passphrase: swordfish",
        "secret=abc123",
        "token=deadbeef",
        "private key-----BEGIN",
    ] {
        let redacted = redact_text(&format!("request failed with {secret}"));
        assert!(redacted.contains(REDACTED), "{redacted}");
        for value in ["hunter2", "swordfish", "abc123", "deadbeef", "BEGIN"] {
            assert!(!redacted.contains(value), "{redacted}");
        }
    }
}

#[test]
fn nested_error_and_warning_values_are_redacted() {
    let value = serde_json::json!({
        "error": {"message": "password=hunter2 rejected"},
        "warnings": ["token=deadbeef"],
        "meta": {"raw": "private key data"}
    });
    let redacted = redact_value(&value).to_string();
    for secret in ["hunter2", "deadbeef", "private key"] {
        assert!(!redacted.contains(secret), "{redacted}");
    }
}

#[test]
fn audit_records_do_not_contain_secret_material() {
    let home = TempDir::new().unwrap();
    let store = AuditStore::open(home.path().join("audit"));
    let mut event = AuditEvent {
        seq: 0,
        timestamp: chrono::Utc::now(),
        interface: "mcp".to_owned(),
        mode: "standalone".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: "files.delete".to_owned(),
        target: Some(serde_json::json!({"path": "/srv/app/file"})),
        result: "denied".to_owned(),
        error_code: Some("scope_denied".to_owned()),
        duration_ms: Some(2),
        details: None,
        prev_hash: String::new(),
        hash: String::new(),
    };
    store.append(&mut event).unwrap();
    let path = home
        .path()
        .join("audit")
        .join(format!("{}.jsonl", event.timestamp.format("%Y-%m-%d")));
    let contents = fs::read_to_string(path).unwrap();
    for secret in ["password", "passphrase", "secret=", "BEGIN PRIVATE KEY"] {
        assert!(!contents.contains(secret), "{contents}");
    }
}

#[test]
fn error_messages_do_not_echo_raw_backend_details() {
    let error = CoreError::CredentialStoreUnavailable("backend said: password=hunter2".to_owned());
    let redacted = redact_text(&error.to_string());
    assert!(!redacted.contains("hunter2"));
    assert_eq!(error.code(), "credential_store_unavailable");
}
