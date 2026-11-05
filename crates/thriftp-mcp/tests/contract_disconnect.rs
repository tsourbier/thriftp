//! Disconnect-policy behavior for MCP sessions.

use tempfile::TempDir;
use thriftp_core::automation::service::AutomationService;
use thriftp_core::automation::{AuditStore, AutomationStore, base64_encode};
use thriftp_core::model::{Transfer, TransferDirection, TransferState};
use thriftp_core::repository::TransferState as PersistedTransfers;
use uuid::Uuid;

fn session(policy: &str) -> (AutomationService, String) {
    let home = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let request = store.enroll(&base64_encode(&[7u8; 32]), "agent").unwrap();
    store
        .approve(
            request.id,
            vec![Uuid::new_v4()],
            vec![(
                "local".to_owned(),
                None,
                scope.path().to_string_lossy().into_owned(),
            )],
            vec!["transfers.upload".to_owned()],
            None,
            policy,
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
fn continue_keeps_transfers_and_cancel_cancels_them() {
    for (policy, expected) in [
        ("continue", TransferState::Active),
        ("cancel", TransferState::Cancelled),
    ] {
        let (mut service, fingerprint) = session(policy);
        let session = service.open_session("token", &fingerprint, 8).unwrap();
        let mut transfers = PersistedTransfers::default();
        let mut transfer = Transfer::new(
            TransferDirection::Upload,
            Uuid::new_v4(),
            "/local/file",
            "/remote/file",
            None,
        );
        transfer.state = TransferState::Active;
        transfers.transfers.push(transfer.clone());
        service.note_transfer(&session.token, transfer.id).unwrap();
        let applied = service
            .disconnect_with_transfers(&session.token, &mut transfers)
            .unwrap();
        assert_eq!(applied, policy);
        assert_eq!(transfers.transfers[0].state, expected);
    }
}
