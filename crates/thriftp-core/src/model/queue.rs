use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::transfer::Transfer;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferQueue {
    pub concurrency_limit: usize,
    pub bandwidth_limit_bytes_per_second: Option<u64>,
    pub paused: bool,
    pub ordered_transfer_ids: Vec<Uuid>,
    pub active_transfer_ids: Vec<Uuid>,
    pub last_checkpoint_at: Option<DateTime<Utc>>,
}

impl Default for TransferQueue {
    fn default() -> Self {
        Self {
            concurrency_limit: 4,
            bandwidth_limit_bytes_per_second: None,
            paused: false,
            ordered_transfer_ids: Vec::new(),
            active_transfer_ids: Vec::new(),
            last_checkpoint_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferHistory {
    pub entries: Vec<Transfer>,
    pub retention_limit: usize,
}

impl Default for TransferHistory {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            retention_limit: 1_000,
        }
    }
}

impl TransferHistory {
    pub fn push(&mut self, transfer: Transfer) {
        self.entries.push(transfer);
        if self.entries.len() > self.retention_limit {
            let excess = self.entries.len() - self.retention_limit;
            self.entries.drain(0..excess);
        }
    }
}
