use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::model::{
    BrowsingSession, ConnectionProfile, CredentialRef, DirectoryFavorite, FavoritePreferences,
    PartialTransferData, Preference, ServerFavorite, Transfer, TransferHistory, TransferQueue,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub connection_profiles: Vec<ConnectionProfile>,
    pub directory_favorites: Vec<DirectoryFavorite>,
    pub credential_refs: Vec<CredentialRef>,
    pub preference: Preference,
    pub tabs: Vec<BrowsingSession>,
    #[serde(default)]
    pub server_favorites: Vec<ServerFavorite>,
    #[serde(default)]
    pub favorite_preferences: FavoritePreferences,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferState {
    pub queue: TransferQueue,
    pub transfers: Vec<Transfer>,
    pub partial_transfers: Vec<PartialTransferData>,
    pub history: TransferHistory,
}

pub trait MetadataStore: Send + Sync {
    fn load(&self) -> CoreResult<AppState>;
    fn save(&self, state: &AppState) -> CoreResult<()>;
}

pub trait TransferStore: Send + Sync {
    fn load(&self) -> CoreResult<TransferState>;
    fn save(&self, state: &TransferState) -> CoreResult<()>;
}

#[derive(Default)]
pub struct MemoryStore {
    state: Mutex<AppState>,
}

impl MetadataStore for MemoryStore {
    fn load(&self) -> CoreResult<AppState> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| crate::error::CoreError::internal("metadata lock poisoned"))
    }

    fn save(&self, state: &AppState) -> CoreResult<()> {
        let mut stored = self
            .state
            .lock()
            .map_err(|_| crate::error::CoreError::internal("metadata lock poisoned"))?;
        *stored = state.clone();
        Ok(())
    }
}

#[derive(Default)]
pub struct MemoryTransferStore {
    state: Mutex<TransferState>,
}

impl TransferStore for MemoryTransferStore {
    fn load(&self) -> CoreResult<TransferState> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| crate::error::CoreError::internal("transfer lock poisoned"))
    }

    fn save(&self, state: &TransferState) -> CoreResult<()> {
        let mut stored = self
            .state
            .lock()
            .map_err(|_| crate::error::CoreError::internal("transfer lock poisoned"))?;
        *stored = state.clone();
        Ok(())
    }
}
