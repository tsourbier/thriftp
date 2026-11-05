//! Ownership acquisition, delegation, and recovery.

use std::path::PathBuf;

use super::client::HostClient;
use super::lock::HostLock;
use super::model::HostRole;
use super::server::{HostServer, HostServices};
use crate::error::{CoreError, CoreResult};
use crate::repository::TransferStore;

pub struct HostRuntime {
    role: HostRole,
    lock: Option<HostLock>,
    server: Option<HostServer>,
    client: Option<HostClient>,
}

impl HostRuntime {
    /// Try to become the host, otherwise connect as a control client.
    pub fn acquire_at(
        run_dir: impl Into<PathBuf>,
        mode: &str,
        services: HostServices,
    ) -> CoreResult<Self> {
        let run_dir = run_dir.into();
        match HostLock::acquire_at(&run_dir) {
            Ok(lock) => {
                recover_transfers(services.transfers.as_ref())?;
                let server = HostServer::start(&lock, mode, services)?;
                Ok(Self {
                    role: HostRole::Host,
                    lock: Some(lock),
                    server: Some(server),
                    client: None,
                })
            }
            Err(CoreError::Conflict(_)) => {
                let client = HostClient::connect_at(&run_dir)?;
                Ok(Self {
                    role: HostRole::Client,
                    lock: None,
                    server: None,
                    client: Some(client),
                })
            }
            Err(error) => Err(error),
        }
    }

    /// Acquire ownership in the default application data directory.
    pub fn acquire(mode: &str, services: HostServices) -> CoreResult<Self> {
        Self::acquire_at(crate::automation::data_dir().join("run"), mode, services)
    }

    #[must_use]
    pub fn role(&self) -> HostRole {
        self.role
    }

    #[must_use]
    pub fn is_host(&self) -> bool {
        self.role == HostRole::Host
    }

    #[must_use]
    pub fn client(&mut self) -> Option<&mut HostClient> {
        self.client.as_mut()
    }

    #[must_use]
    pub fn server(&self) -> Option<&HostServer> {
        self.server.as_ref()
    }

    /// Whether a published host answers `host.status`.
    #[must_use]
    pub fn is_host_ready(run_dir: &std::path::Path) -> bool {
        HostClient::connect_at(run_dir)
            .and_then(|mut client| client.status())
            .is_ok()
    }

    /// Release the endpoint and lock. Control clients simply disconnect.
    pub fn shutdown(mut self) {
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
        if let Some(lock) = self.lock.take() {
            let _ = lock.release();
        }
    }
}

impl Drop for HostRuntime {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
        if let Some(lock) = self.lock.take() {
            let _ = lock.release();
        }
    }
}

/// Mark transfers left `active` by a previous owner as interrupted.
pub fn recover_transfers(store: &dyn TransferStore) -> CoreResult<usize> {
    let mut state = store.load()?;
    let recovered = crate::recovery::recover_transfers(&mut state);
    if recovered > 0 {
        store.save(&state)?;
    }
    Ok(recovered)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovers_active_transfers_as_interrupted() {
        use crate::model::{Transfer, TransferDirection, TransferState};
        use crate::repository::{MemoryTransferStore, TransferState as Persisted};

        let store = MemoryTransferStore::default();
        let mut state = Persisted::default();
        let transfer = Transfer::new(
            TransferDirection::Upload,
            uuid::Uuid::new_v4(),
            "/local/file",
            "/remote/file",
            None,
        );
        let mut active = transfer.clone();
        active.state = TransferState::Active;
        state.transfers.push(active);
        state.transfers.push(transfer);
        store.save(&state).unwrap();

        assert_eq!(recover_transfers(&store).unwrap(), 1);
        let recovered = store.load().unwrap();
        assert_eq!(recovered.transfers[0].state, TransferState::Interrupted);
        assert_eq!(recovered.transfers[1].state, TransferState::Pending);
    }
}
