pub mod bandwidth;
pub mod persistence;

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::model::{Priority, Transfer, TransferQueue, TransferState};

pub struct QueueService {
    pub queue: TransferQueue,
    transfers: HashMap<Uuid, Transfer>,
}

impl QueueService {
    pub fn new(concurrency_limit: usize) -> Self {
        Self {
            queue: TransferQueue {
                concurrency_limit: concurrency_limit.max(1),
                ..TransferQueue::default()
            },
            transfers: HashMap::new(),
        }
    }

    /// Restore a persisted queue and its transfers.
    pub fn load(&mut self, queue: TransferQueue, transfers: Vec<Transfer>) {
        self.queue = queue;
        self.transfers = transfers
            .into_iter()
            .map(|transfer| (transfer.id, transfer))
            .collect();
    }

    pub fn enqueue(&mut self, mut transfer: Transfer) -> CoreResult<Uuid> {
        if self.transfers.contains_key(&transfer.id) {
            return Err(CoreError::Conflict(format!("transfer {}", transfer.id)));
        }
        transfer.state = TransferState::Pending;
        transfer.order = self.queue.ordered_transfer_ids.len() as i64;
        let id = transfer.id;
        self.transfers.insert(id, transfer);
        self.queue.ordered_transfer_ids.push(id);
        Ok(id)
    }

    pub fn list(&self) -> Vec<Transfer> {
        let mut transfers: Vec<_> = self.transfers.values().cloned().collect();
        transfers.sort_by_key(|transfer| transfer.order);
        transfers
    }

    pub fn get(&self, id: Uuid) -> CoreResult<Transfer> {
        self.transfers
            .get(&id)
            .cloned()
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))
    }

    pub fn pause(&mut self) {
        self.queue.paused = true;
    }

    pub fn resume(&mut self) {
        self.queue.paused = false;
    }

    pub fn set_concurrency(&mut self, limit: usize) {
        self.queue.concurrency_limit = limit.max(1);
    }

    pub fn set_bandwidth(&mut self, bytes_per_second: Option<u64>) {
        self.queue.bandwidth_limit_bytes_per_second = bytes_per_second;
    }

    pub fn reorder(&mut self, ids: &[Uuid]) -> CoreResult<()> {
        let unique: HashSet<_> = ids.iter().copied().collect();
        if unique.len() != ids.len() {
            return Err(CoreError::validation("reorder list contains duplicates"));
        }
        if ids.iter().any(|id| !self.transfers.contains_key(id)) {
            return Err(CoreError::NotFound(
                "reorder list contains unknown transfer".to_owned(),
            ));
        }
        for (order, id) in ids.iter().enumerate() {
            if let Some(transfer) = self.transfers.get_mut(id) {
                transfer.order = order as i64;
            }
        }
        let mut remaining: Vec<_> = self
            .queue
            .ordered_transfer_ids
            .iter()
            .copied()
            .filter(|id| !unique.contains(id))
            .collect();
        let mut ordered: Vec<_> = ids.to_vec();
        ordered.append(&mut remaining);
        self.queue.ordered_transfer_ids = ordered;
        Ok(())
    }

    pub fn set_priority(&mut self, id: Uuid, priority: Priority) -> CoreResult<()> {
        let transfer = self
            .transfers
            .get_mut(&id)
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        transfer.priority = priority;
        Ok(())
    }

    pub fn next_eligible(&self) -> Option<Transfer> {
        if self.queue.paused || self.active_count() >= self.queue.concurrency_limit {
            return None;
        }
        self.queue
            .ordered_transfer_ids
            .iter()
            .filter_map(|id| self.transfers.get(id))
            .filter(|transfer| transfer.state == TransferState::Pending)
            .min_by_key(|transfer| (priority_rank(transfer.priority), transfer.order))
            .cloned()
    }

    pub fn mark_active(&mut self, id: Uuid) -> CoreResult<()> {
        let transfer = self
            .transfers
            .get_mut(&id)
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        transfer.state = TransferState::Active;
        self.queue.active_transfer_ids.push(id);
        Ok(())
    }

    pub fn mark_state(&mut self, id: Uuid, state: TransferState) -> CoreResult<()> {
        let transfer = self
            .transfers
            .get_mut(&id)
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        transfer.state = state;
        if state != TransferState::Active {
            self.queue
                .active_transfer_ids
                .retain(|active| *active != id);
        }
        Ok(())
    }

    /// Replace a transfer with an updated snapshot from a worker.
    pub fn replace(&mut self, transfer: Transfer) -> CoreResult<()> {
        let id = transfer.id;
        if !self.transfers.contains_key(&id) {
            return Err(CoreError::NotFound(format!("transfer {id}")));
        }
        if transfer.state != TransferState::Active {
            self.queue
                .active_transfer_ids
                .retain(|active| *active != id);
        }
        if !self.queue.ordered_transfer_ids.contains(&id) {
            self.queue.ordered_transfer_ids.push(id);
        }
        self.transfers.insert(id, transfer);
        Ok(())
    }

    pub fn active_count(&self) -> usize {
        self.queue.active_transfer_ids.len()
    }

    pub fn cancel_all_pending(&mut self) {
        for transfer in self.transfers.values_mut() {
            if transfer.state == TransferState::Pending {
                transfer.state = TransferState::Cancelled;
            }
        }
    }

    /// Remove a terminal transfer from the visible queue.
    pub fn remove(&mut self, id: Uuid) -> CoreResult<Transfer> {
        let transfer = self
            .transfers
            .remove(&id)
            .ok_or_else(|| CoreError::NotFound(format!("transfer {id}")))?;
        if matches!(
            transfer.state,
            TransferState::Active | TransferState::Pending | TransferState::Paused
        ) {
            self.transfers.insert(id, transfer.clone());
            return Err(CoreError::Conflict(
                "only terminal transfers can be dismissed".to_owned(),
            ));
        }
        self.queue
            .ordered_transfer_ids
            .retain(|candidate| *candidate != id);
        self.queue
            .active_transfer_ids
            .retain(|candidate| *candidate != id);
        Ok(transfer)
    }

    /// Return the current transfer IDs.
    #[must_use]
    pub fn ids(&self) -> Vec<Uuid> {
        self.transfers.keys().copied().collect()
    }

    /// Remove a queued transfer unconditionally, used to roll back a failed
    /// enqueue operation.
    pub fn discard(&mut self, id: Uuid) {
        self.transfers.remove(&id);
        self.queue
            .ordered_transfer_ids
            .retain(|candidate| *candidate != id);
        self.queue
            .active_transfer_ids
            .retain(|candidate| *candidate != id);
    }
}

fn priority_rank(priority: Priority) -> u8 {
    match priority {
        Priority::High => 0,
        Priority::Normal => 1,
        Priority::Low => 2,
    }
}
