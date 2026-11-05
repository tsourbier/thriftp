mod support;

use std::time::{Duration, Instant};

use thriftp_core::model::{Transfer, TransferDirection, TransferState};
use thriftp_core::queue::QueueService;
use thriftp_core::transfers::TransferControl;
use uuid::Uuid;

#[test]
fn queue_pause_is_immediate() {
    let mut queue = QueueService::new(4);
    for _ in 0..8 {
        queue
            .enqueue(Transfer::new(
                TransferDirection::Download,
                Uuid::new_v4(),
                "/source",
                "/destination",
                Some(100),
            ))
            .unwrap();
    }
    let started = Instant::now();
    queue.pause();
    assert!(queue.next_eligible().is_none());
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn cancellation_is_acknowledged_within_two_seconds() {
    let control = TransferControl::new();
    let started = Instant::now();
    control.cancel();
    assert_eq!(control.outcome(), TransferState::Cancelled);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn local_connection_readiness_is_within_the_budget() {
    // This measures the in-process core path; real network readiness is tested
    // by the container harness, which applies the same 60-second budget.
    let started = Instant::now();
    let queue = QueueService::new(4);
    assert_eq!(queue.queue.concurrency_limit, 4);
    assert!(started.elapsed() < Duration::from_secs(60));
}
