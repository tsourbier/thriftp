use chrono::{DateTime, Utc};

use crate::error::{CoreError, CoreResult};
use crate::model::{PartialTransferData, TransferState as TransferStatus};
use crate::repository::TransferState;

pub fn recover_transfers(state: &mut TransferState) -> usize {
    let mut interrupted = 0;
    for transfer in &mut state.transfers {
        if transfer.state == TransferStatus::Active {
            transfer.state = TransferStatus::Interrupted;
            interrupted += 1;
        }
    }
    state.queue.active_transfer_ids.clear();
    interrupted
}

pub fn verify_resume(
    partial: &PartialTransferData,
    source_size: Option<u64>,
    source_modified_at: Option<DateTime<Utc>>,
    destination_offset: u64,
) -> CoreResult<()> {
    if partial.source_size.is_some() && partial.source_size != source_size {
        return Err(CoreError::Conflict(
            "source size changed since the transfer was interrupted".to_owned(),
        ));
    }
    if partial.source_modified_at.is_some() && partial.source_modified_at != source_modified_at {
        return Err(CoreError::Conflict(
            "source modification time changed since the transfer was interrupted".to_owned(),
        ));
    }
    if partial.offset != destination_offset {
        return Err(CoreError::Conflict(
            "destination partial data does not match the recorded offset".to_owned(),
        ));
    }
    Ok(())
}

pub fn discard_partial(state: &mut TransferState, transfer_id: uuid::Uuid) -> bool {
    let before = state.partial_transfers.len();
    state
        .partial_transfers
        .retain(|partial| partial.transfer_id != transfer_id);
    state.partial_transfers.len() != before
}
