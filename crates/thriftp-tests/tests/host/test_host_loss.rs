//! Host-loss, interruption, and shared-state recovery scenarios.

#[path = "../support/mod.rs"]
mod support;

use std::time::Duration;

use support::{Worker, transfer_store};
use tempfile::TempDir;
use thriftp_core::host::HostRuntime;
use thriftp_core::model::{Transfer, TransferDirection, TransferState};
use thriftp_core::repository::{TransferState as PersistedTransfers, TransferStore};
use uuid::Uuid;

#[test]
fn killed_host_leaves_transfers_interrupted_and_resumable() {
    let home = TempDir::new().unwrap();
    let store = transfer_store(home.path());
    let mut state = PersistedTransfers::default();
    let mut transfer = Transfer::new(
        TransferDirection::Upload,
        Uuid::new_v4(),
        "/local/data.bin",
        "/remote/data.bin",
        Some(1_024),
    );
    transfer.state = TransferState::Active;
    transfer.resumable = true;
    state.transfers.push(transfer.clone());
    store.save(&state).unwrap();

    let worker = Worker::spawn(home.path());
    assert!(HostRuntime::is_host_ready(&home.path().join("run")));
    let recovered = store.load().unwrap();
    assert_eq!(recovered.transfers[0].state, TransferState::Interrupted);
    assert!(recovered.transfers[0].resumable);

    drop(worker);
    assert!(support::wait_for(
        || !HostRuntime::is_host_ready(&home.path().join("run")),
        Duration::from_secs(5)
    ));
    assert_eq!(
        store.load().unwrap().transfers[0].state,
        TransferState::Interrupted
    );
}

#[test]
fn pending_and_paused_transfers_survive_recovery() {
    let home = TempDir::new().unwrap();
    let store = transfer_store(home.path());
    let mut state = PersistedTransfers::default();
    for transfer_state in [TransferState::Pending, TransferState::Paused] {
        let mut transfer = Transfer::new(
            TransferDirection::Download,
            Uuid::new_v4(),
            "/remote/file",
            "/local/file",
            None,
        );
        transfer.state = transfer_state;
        state.transfers.push(transfer);
    }
    store.save(&state).unwrap();

    let worker = Worker::spawn(home.path());
    let recovered = store.load().unwrap();
    assert_eq!(recovered.transfers[0].state, TransferState::Pending);
    assert_eq!(recovered.transfers[1].state, TransferState::Paused);
    drop(worker);
}
