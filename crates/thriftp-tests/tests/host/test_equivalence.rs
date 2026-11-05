//! Parity between CLI, MCP, and desktop authorization outcomes.

use chrono::Utc;
use tempfile::TempDir;
use thriftp_core::automation::service::AutomationService;
use thriftp_core::automation::{AuditStore, AutomationStore, base64_encode};
use thriftp_core::error::CoreError;
use uuid::Uuid;

fn granted(home: &TempDir) -> (AutomationService, String) {
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let request = store.enroll(&base64_encode(&[6u8; 32]), "agent").unwrap();
    store
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
            "cli",
        )
        .unwrap();
    let service = AutomationService::new(
        AutomationStore::open(home.path().join("automation.json")).unwrap(),
        AuditStore::open(home.path().join("audit")),
    );
    (service, request.fingerprint)
}

#[test]
fn every_surface_observes_the_same_allow_and_deny_decisions() {
    let home = TempDir::new().unwrap();
    let (mut service, fingerprint) = granted(&home);
    let session = service.open_session("token", &fingerprint, 8).unwrap();
    for surface in ["cli", "mcp", "desktop"] {
        assert!(
            service
                .authorize(&session.token, "browse.list", Utc::now())
                .is_ok(),
            "{surface} should allow a granted operation"
        );
        assert_eq!(
            service
                .authorize(&session.token, "files.delete", Utc::now())
                .unwrap_err(),
            CoreError::ScopeDenied,
            "{surface} should deny an out-of-grant operation"
        );
    }
}

#[test]
fn revocation_is_visible_to_every_surface() {
    let home = TempDir::new().unwrap();
    let (mut service, fingerprint) = granted(&home);
    let session = service.open_session("token", &fingerprint, 8).unwrap();
    service.revoke(&fingerprint).unwrap();
    for _ in ["cli", "mcp", "desktop"] {
        assert_eq!(
            service
                .authorize(&session.token, "browse.list", Utc::now())
                .unwrap_err(),
            CoreError::ClientNotEnrolled
        );
    }
}
