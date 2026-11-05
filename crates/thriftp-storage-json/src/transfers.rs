use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::Transfer;
use thriftp_core::repository::{TransferState, TransferStore};

use crate::{SCHEMA_VERSION, io_error, write_atomic};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferDocument {
    schema_version: u32,
    #[serde(flatten)]
    state: TransferState,
}

/// Versioned JSON persistence for transfer queue and history state.
pub struct JsonTransferStore {
    path: PathBuf,
}

impl JsonTransferStore {
    /// Create a store at the supplied path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Return the store path.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl TransferStore for JsonTransferStore {
    fn load(&self) -> CoreResult<TransferState> {
        if !self.path.is_file() {
            return Ok(TransferState::default());
        }
        let bytes = fs::read(&self.path).map_err(io_error)?;
        let document: TransferDocument = serde_json::from_slice(&bytes).map_err(|error| {
            CoreError::internal(format!("failed to parse transfer store: {error}"))
        })?;
        if document.schema_version != SCHEMA_VERSION {
            return Err(CoreError::internal(format!(
                "unsupported transfer schema version {}",
                document.schema_version
            )));
        }
        Ok(document.state)
    }

    fn save(&self, state: &TransferState) -> CoreResult<()> {
        let document = TransferDocument {
            schema_version: SCHEMA_VERSION,
            state: state.clone(),
        };
        write_atomic(&self.path, &document)
    }
}

/// Add a terminal transfer to history, evicting the oldest records beyond
/// the configured retention limit.
pub fn record_history(state: &mut TransferState, transfer: Transfer) {
    state.history.push(transfer);
}

/// Remove all persisted transfer history and return the number of records cleared.
pub fn clear_history(state: &mut TransferState) -> usize {
    let cleared = state.history.entries.len();
    state.history.entries.clear();
    cleared
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    use thriftp_core::model::{TransferDirection, TransferState as TransferStatus};
    use uuid::Uuid;

    fn transfer() -> Transfer {
        Transfer::new(
            TransferDirection::Download,
            Uuid::new_v4(),
            "/remote/file",
            "/local/file",
            Some(10),
        )
    }

    #[test]
    fn store_round_trips_and_clears_history() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("transfers.json");
        let store = JsonTransferStore::new(&path);

        let mut state = TransferState::default();
        let mut completed = transfer();
        completed.state = TransferStatus::Completed;
        record_history(&mut state, completed);
        store.save(&state).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded.history.entries.len(), 1);
    }

    #[test]
    fn history_is_bounded_to_one_thousand_records() {
        let mut state = TransferState::default();
        for _ in 0..1_050 {
            record_history(&mut state, transfer());
        }
        assert_eq!(state.history.entries.len(), 1_000);
        assert_eq!(clear_history(&mut state), 1_000);
    }
}
