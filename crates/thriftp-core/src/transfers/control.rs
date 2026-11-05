use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use crate::model::TransferState;

#[derive(Debug, Default)]
struct ControlInner {
    paused: AtomicBool,
    cancelled: AtomicBool,
    transferred: AtomicU64,
}

/// Shared pause/cancel/progress state for one running transfer.
#[derive(Debug, Clone, Default)]
pub struct TransferControl {
    inner: Arc<ControlInner>,
}

impl TransferControl {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pause(&self) {
        self.inner.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.inner.paused.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.inner.cancelled.store(true, Ordering::SeqCst);
    }

    /// Clear both pause and cancellation flags before restarting a transfer.
    pub fn reset(&self) {
        self.inner.paused.store(false, Ordering::SeqCst);
        self.inner.cancelled.store(false, Ordering::SeqCst);
        self.inner.transferred.store(0, Ordering::SeqCst);
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Returns false once a pause or cancellation has been requested.
    #[must_use]
    pub fn should_continue(&self) -> bool {
        !self.is_paused() && !self.is_cancelled()
    }

    pub fn set_transferred(&self, bytes: u64) {
        self.inner.transferred.store(bytes, Ordering::SeqCst);
    }

    #[must_use]
    pub fn transferred(&self) -> u64 {
        self.inner.transferred.load(Ordering::SeqCst)
    }

    /// Record a cumulative byte count and return the resulting state.
    #[must_use]
    pub fn outcome(&self) -> TransferState {
        if self.is_cancelled() {
            TransferState::Cancelled
        } else if self.is_paused() {
            TransferState::Paused
        } else {
            TransferState::Completed
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSnapshot {
    pub transferred_bytes: u64,
    pub speed_bytes_per_second: Option<u64>,
    pub estimated_remaining_seconds: Option<u64>,
}

/// Speed and ETA estimator based on elapsed wall-clock time.
pub struct ProgressTracker {
    started: Instant,
    total: Option<u64>,
}

impl ProgressTracker {
    #[must_use]
    pub fn new(total: Option<u64>) -> Self {
        Self {
            started: Instant::now(),
            total,
        }
    }

    #[must_use]
    pub fn snapshot(&self, transferred: u64) -> ProgressSnapshot {
        let elapsed = self.started.elapsed().as_secs_f64();
        let speed = (elapsed > 0.0).then(|| (transferred as f64 / elapsed).round() as u64);
        let estimated = match (self.total, speed) {
            (Some(total), Some(speed)) if speed > 0 => {
                Some(total.saturating_sub(transferred).div_ceil(speed))
            }
            _ => None,
        };
        ProgressSnapshot {
            transferred_bytes: transferred,
            speed_bytes_per_second: speed,
            estimated_remaining_seconds: estimated,
        }
    }

    #[must_use]
    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}
