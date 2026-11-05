use chrono::Utc;
use thriftp_core::browser;
use thriftp_core::model::{
    PartialTransferData, Priority, Transfer, TransferDirection, TransferState as TransferStatus,
};
use thriftp_core::queue::QueueService;
use thriftp_core::recovery::{discard_partial, recover_transfers, verify_resume};
use thriftp_core::repository::TransferState;
use uuid::Uuid;

#[test]
fn local_browser_lists_and_manages_entries() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("a.txt"), b"hello").unwrap();
    std::fs::create_dir(root.join("folder")).unwrap();

    let entries = browser::list_local(root).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries.iter().any(|entry| entry.name == "a.txt"));
    assert!(entries.iter().any(|entry| entry.name == "folder"
        && entry.entry_type == thriftp_core::model::EntryType::Directory));

    let stat = browser::stat_local(&root.join("a.txt")).unwrap();
    assert_eq!(stat.size, Some(5));

    browser::create_dir_local(&root.join("new")).unwrap();
    browser::rename_local(&root.join("new"), &root.join("renamed")).unwrap();
    assert!(root.join("renamed").is_dir());
}

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
fn queue_orders_priorities_and_respects_pause_and_concurrency() {
    let mut queue = QueueService::new(1);
    let low = queue.enqueue(transfer(TransferDirection::Upload)).unwrap();
    let high = queue
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    queue.set_priority(high, Priority::High).unwrap();

    queue.pause();
    assert!(queue.next_eligible().is_none());
    queue.resume();

    let next = queue.next_eligible().unwrap();
    assert_eq!(next.id, high);
    queue.mark_active(high).unwrap();
    assert!(queue.next_eligible().is_none());

    queue.mark_state(high, TransferStatus::Completed).unwrap();
    let next = queue.next_eligible().unwrap();
    assert_eq!(next.id, low);

    queue.cancel_all_pending();
    assert_eq!(queue.get(low).unwrap().state, TransferStatus::Cancelled);
}

#[test]
fn queue_reorders_and_rejects_unknown_ids() {
    let mut queue = QueueService::new(4);
    let first = queue.enqueue(transfer(TransferDirection::Upload)).unwrap();
    let second = queue
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    queue.reorder(&[second, first]).unwrap();
    assert_eq!(queue.get(second).unwrap().order, 0);
    assert!(queue.reorder(&[Uuid::new_v4()]).is_err());
    assert!(queue.reorder(&[first, first]).is_err());
}

#[test]
fn recovery_maps_active_to_interrupted_and_verifies_resume() {
    let mut state = TransferState::default();
    let mut active = transfer(TransferDirection::Upload);
    active.state = TransferStatus::Active;
    let active_id = active.id;
    state.transfers.push(active);
    state.queue.active_transfer_ids.push(active_id);

    assert_eq!(recover_transfers(&mut state), 1);
    assert_eq!(state.transfers[0].state, TransferStatus::Interrupted);
    assert!(state.queue.active_transfer_ids.is_empty());

    let now = Utc::now();
    let partial = PartialTransferData {
        id: Uuid::new_v4(),
        transfer_id: active_id,
        local_path: Some("/tmp/partial".to_owned()),
        remote_path: None,
        offset: 50,
        source_size: Some(100),
        source_modified_at: Some(now),
        created_at: now,
        updated_at: now,
    };
    assert!(verify_resume(&partial, Some(100), Some(now), 50).is_ok());
    assert!(verify_resume(&partial, Some(99), Some(now), 50).is_err());
    assert!(verify_resume(&partial, Some(100), Some(now), 49).is_err());

    state.partial_transfers.push(partial);
    assert!(discard_partial(&mut state, active_id));
    assert!(state.partial_transfers.is_empty());
}
