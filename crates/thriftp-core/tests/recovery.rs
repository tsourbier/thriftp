use chrono::Utc;
use thriftp_core::model::{
    PartialTransferData, Transfer, TransferDirection, TransferHistory, TransferState,
};
use thriftp_core::queue::QueueService;
use thriftp_core::recovery::{discard_partial, recover_transfers, verify_resume};
use thriftp_core::repository::TransferState as PersistedTransferState;
use thriftp_core::transfers::resume::{
    metadata_path, new_partial, partial_path, read_metadata, remove_artifacts, write_metadata,
};
use uuid::Uuid;

fn transfer(direction: TransferDirection) -> Transfer {
    Transfer::new(
        direction,
        Uuid::new_v4(),
        "/source",
        "/destination",
        Some(100),
    )
}

#[test]
fn startup_maps_active_transfers_to_interrupted() {
    let mut state = PersistedTransferState::default();
    let mut active = transfer(TransferDirection::Download);
    active.state = TransferState::Active;
    let active_id = active.id;
    state.transfers.push(active);
    state.queue.active_transfer_ids.push(active_id);

    assert_eq!(recover_transfers(&mut state), 1);
    assert_eq!(state.transfers[0].state, TransferState::Interrupted);
    assert!(state.queue.active_transfer_ids.is_empty());
}

#[test]
fn resume_is_refused_when_the_source_changed() {
    let now = Utc::now();
    let partial = PartialTransferData {
        id: Uuid::new_v4(),
        transfer_id: Uuid::new_v4(),
        local_path: Some("/tmp/partial".to_owned()),
        remote_path: None,
        offset: 50,
        source_size: Some(100),
        source_modified_at: Some(now),
        created_at: now,
        updated_at: now,
    };

    assert!(verify_resume(&partial, Some(200), Some(now), 50).is_err());
    assert!(
        verify_resume(
            &partial,
            Some(100),
            Some(now + chrono::Duration::seconds(1)),
            50
        )
        .is_err()
    );
    assert!(verify_resume(&partial, Some(100), Some(now), 49).is_err());
    assert!(verify_resume(&partial, Some(100), Some(now), 50).is_ok());
}

#[test]
fn partial_files_can_be_retained_or_discarded() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("file.bin");
    let partial = new_partial(
        Uuid::new_v4(),
        Some(&destination),
        Some("/remote/file.bin"),
        64,
        Some(128),
        None,
    );
    write_metadata(&destination, &partial).unwrap();
    std::fs::write(partial_path(&destination), b"partial").unwrap();

    assert_eq!(read_metadata(&destination).unwrap().offset, 64);
    remove_artifacts(&destination).unwrap();
    assert!(!partial_path(&destination).exists());
    assert!(!metadata_path(&destination).exists());

    let mut state = PersistedTransferState::default();
    let transfer_id = partial.transfer_id;
    state.partial_transfers.push(partial);
    assert!(discard_partial(&mut state, transfer_id));
    assert!(state.partial_transfers.is_empty());
}

#[test]
fn restarting_a_transfer_resets_progress_and_cancel_is_terminal() {
    let mut queue = QueueService::new(1);
    let id = queue
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    queue.mark_active(id).unwrap();
    let mut current = queue.get(id).unwrap();
    current.transferred_bytes = 80;
    current.state = TransferState::Paused;
    queue.replace(current).unwrap();

    let mut restarted = queue.get(id).unwrap();
    restarted.transferred_bytes = 0;
    restarted.state = TransferState::Pending;
    queue.replace(restarted).unwrap();
    assert_eq!(queue.get(id).unwrap().transferred_bytes, 0);

    queue.mark_state(id, TransferState::Cancelled).unwrap();
    assert_eq!(queue.get(id).unwrap().state, TransferState::Cancelled);
}

#[test]
fn history_keeps_only_one_thousand_records() {
    let mut history = TransferHistory::default();
    for _ in 0..1_050 {
        history.push(transfer(TransferDirection::Upload));
    }
    assert_eq!(history.entries.len(), 1_000);
    assert_eq!(history.retention_limit, 1_000);
    assert_eq!(history.entries[0].source_path, "/source");
}
