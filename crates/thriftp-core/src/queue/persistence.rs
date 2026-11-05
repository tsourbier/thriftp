use std::time::{Duration, Instant};

use crate::error::CoreResult;
use crate::repository::{TransferState, TransferStore};

/// Debounces queue/history checkpoints so progress updates are not written to
/// disk on every chunk.
pub struct CheckpointScheduler {
    interval: Duration,
    last: Option<Instant>,
}

impl CheckpointScheduler {
    #[must_use]
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    #[must_use]
    pub fn should_checkpoint(&mut self, now: Instant) -> bool {
        match self.last {
            Some(last) if now.duration_since(last) < self.interval => false,
            _ => {
                self.last = Some(now);
                true
            }
        }
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

impl Default for CheckpointScheduler {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
    }
}

pub fn checkpoint(store: &dyn TransferStore, state: &TransferState) -> CoreResult<()> {
    store.save(state)
}

pub fn checkpoint_if_due(
    store: &dyn TransferStore,
    state: &TransferState,
    scheduler: &mut CheckpointScheduler,
) -> CoreResult<bool> {
    if scheduler.should_checkpoint(Instant::now()) {
        checkpoint(store, state)?;
        return Ok(true);
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::MemoryTransferStore;

    #[test]
    fn scheduler_debounces_checkpoints() {
        let mut scheduler = CheckpointScheduler::new(Duration::from_secs(60));
        let now = Instant::now();
        assert!(scheduler.should_checkpoint(now));
        assert!(!scheduler.should_checkpoint(now + Duration::from_secs(1)));
        assert!(scheduler.should_checkpoint(now + Duration::from_secs(61)));
    }

    #[test]
    fn checkpoints_reach_the_store() {
        let store = MemoryTransferStore::default();
        let mut state = TransferState::default();
        state.queue.paused = true;
        let mut scheduler = CheckpointScheduler::default();
        assert!(checkpoint_if_due(&store, &state, &mut scheduler).unwrap());
        assert!(store.load().unwrap().queue.paused);
    }
}
