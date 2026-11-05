//! Disconnect-policy behavior for active and queued transfers.

use std::sync::Arc;

use chrono::Utc;
use tempfile::TempDir;
use thriftp_core::automation::service::AutomationService;
use thriftp_core::automation::{AuditStore, AutomationStore, base64_encode};
use thriftp_core::error::CoreError;
use thriftp_core::model::{Transfer, TransferDirection, TransferState};
use thriftp_core::repository::{
    MemoryTransferStore, TransferState as PersistedTransfers, TransferStore,
};
use uuid::Uuid;

fn service(home: &TempDir, policy: &str) -> (AutomationService, String, TempDir) {
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let request = store.enroll(&base64_encode(&[5u8; 32]), "agent").unwrap();
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
    (service, request.fingerprint, scope)
}

fn transfer() -> Transfer {
    let mut transfer = Transfer::new(
        TransferDirection::Upload,
        Uuid::new_v4(),
        "/local/file",
        "/remote/file",
        None,
    );
    transfer.state = TransferState::Active;
    transfer
}

#[test]
fn cancel_policy_cancels_owned_transfers_on_disconnect() {
    let home = TempDir::new().unwrap();
    let (mut service, fingerprint, _scope) = service(&home, "cancel");
    let session = service
        .open_session("token-cancel", &fingerprint, 8)
        .unwrap();
    let mut transfers = PersistedTransfers::default();
    let owned = transfer();
    transfers.transfers.push(owned.clone());
    service.note_transfer(&session.token, owned.id).unwrap();
    assert_eq!(
        service
            .disconnect_with_transfers(&session.token, &mut transfers)
            .unwrap(),
        "cancel"
    );
    assert_eq!(transfers.transfers[0].state, TransferState::Cancelled);
    assert_eq!(
        service
            .authorize(&session.token, "transfers.upload", Utc::now())
            .unwrap_err(),
        CoreError::ClientNotEnrolled
    );
}

#[test]
fn continue_policy_leaves_transfers_running() {
    let home = TempDir::new().unwrap();
    let (mut service, fingerprint, _scope) = service(&home, "continue");
    let session = service
        .open_session("token-continue", &fingerprint, 8)
        .unwrap();
    let mut transfers = PersistedTransfers::default();
    let owned = transfer();
    transfers.transfers.push(owned.clone());
    service.note_transfer(&session.token, owned.id).unwrap();
    assert_eq!(
        service
            .disconnect_with_transfers(&session.token, &mut transfers)
            .unwrap(),
        "continue"
    );
    assert_eq!(transfers.transfers[0].state, TransferState::Active);
    // The session is still terminated even though work continues.
    assert_eq!(
        service
            .authorize(&session.token, "transfers.upload", Utc::now())
            .unwrap_err(),
        CoreError::ClientNotEnrolled
    );
}

#[test]
fn transfer_store_keeps_the_audited_outcome() {
    let store = Arc::new(MemoryTransferStore::default());
    let mut transfers = PersistedTransfers::default();
    transfers.transfers.push(transfer());
    store.save(&transfers).unwrap();
    let persisted = store.load().unwrap();
    assert_eq!(persisted.transfers[0].state, TransferState::Active);
}
