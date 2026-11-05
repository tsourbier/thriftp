use std::path::PathBuf;

use thriftp_core::automation::data_dir;
use thriftp_core::error::CoreResult;
use thriftp_core::repository::{AppState, MetadataStore, TransferState, TransferStore};
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};

pub fn metadata_path() -> PathBuf {
    data_dir().join("metadata.json")
}

pub fn transfer_path() -> PathBuf {
    data_dir().join("transfers.json")
}

pub fn load_metadata() -> CoreResult<AppState> {
    JsonMetadataStore::new(metadata_path()).load()
}

pub fn save_metadata(state: &AppState) -> CoreResult<()> {
    JsonMetadataStore::new(metadata_path()).save(state)
}

pub fn load_transfers() -> CoreResult<TransferState> {
    let store = JsonTransferStore::new(transfer_path());
    let mut state = store.load()?;
    if thriftp_core::recovery::recover_transfers(&mut state) > 0 {
        store.save(&state)?;
    }
    Ok(state)
}

pub fn save_transfers(state: &TransferState) -> CoreResult<()> {
    JsonTransferStore::new(transfer_path()).save(state)
}
