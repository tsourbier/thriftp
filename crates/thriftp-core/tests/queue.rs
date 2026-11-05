use std::time::{Duration, Instant};

use thriftp_core::model::{Priority, Transfer, TransferDirection, TransferState};
use thriftp_core::queue::QueueService;
use thriftp_core::queue::bandwidth::BandwidthLimiter;
use thriftp_core::queue::persistence::{CheckpointScheduler, checkpoint_if_due};
use thriftp_core::repository::{
    MemoryTransferStore, TransferState as PersistedState, TransferStore,
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
fn ordering_priority_and_concurrency_are_enforced() {
    let mut queue = QueueService::new(2);
    let first = queue.enqueue(transfer(TransferDirection::Upload)).unwrap();
    let second = queue
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    let third = queue.enqueue(transfer(TransferDirection::Upload)).unwrap();

    queue.set_priority(third, Priority::High).unwrap();
    assert_eq!(queue.next_eligible().unwrap().id, third);

    queue.mark_active(third).unwrap();
    queue.mark_active(first).unwrap();
    assert!(queue.next_eligible().is_none());
    queue.mark_state(first, TransferState::Completed).unwrap();
    assert_eq!(queue.next_eligible().unwrap().id, second);
}

#[test]
fn queue_pause_and_resume_block_starts() {
    let mut queue = QueueService::new(1);
    queue.enqueue(transfer(TransferDirection::Upload)).unwrap();
    queue.pause();
    assert!(queue.next_eligible().is_none());
    queue.resume();
    assert!(queue.next_eligible().is_some());
}

#[test]
fn cancel_all_pending_leaves_active_work_alone() {
    let mut queue = QueueService::new(1);
    let pending = queue.enqueue(transfer(TransferDirection::Upload)).unwrap();
    let active = queue
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    queue.mark_active(active).unwrap();
    queue.cancel_all_pending();

    assert_eq!(queue.get(pending).unwrap().state, TransferState::Cancelled);
    assert_eq!(queue.get(active).unwrap().state, TransferState::Active);
}

#[tokio::test]
async fn bandwidth_limiter_delays_above_the_bucket_size() {
    let limiter = BandwidthLimiter::new(Some(1024)).unwrap();
    limiter.acquire(1024).await;
    let started = Instant::now();
    limiter.acquire(512).await;
    assert!(started.elapsed() >= Duration::from_millis(100));
}

#[test]
fn checkpoint_scheduler_and_store_round_trip() {
    let store = MemoryTransferStore::default();
    let mut state = PersistedState::default();
    state.queue.paused = true;
    let mut scheduler = CheckpointScheduler::new(Duration::from_secs(60));
    assert!(checkpoint_if_due(&store, &state, &mut scheduler).unwrap());
    assert!(!checkpoint_if_due(&store, &state, &mut scheduler).unwrap());
    assert!(store.load().unwrap().queue.paused);
}

#[test]
fn queue_load_restores_persisted_state() {
    let mut original = QueueService::new(4);
    let id = original
        .enqueue(transfer(TransferDirection::Download))
        .unwrap();
    original.mark_state(id, TransferState::Interrupted).unwrap();
    let queue = original.queue.clone();
    let transfers = original.list();

    let mut restored = QueueService::new(1);
    restored.load(queue, transfers);
    assert_eq!(restored.queue.concurrency_limit, 4);
    assert_eq!(restored.get(id).unwrap().state, TransferState::Interrupted);
}
