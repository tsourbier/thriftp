use crate::error::{CoreError, CoreResult};
use crate::model::CredentialRef;

use super::traits::{CredentialAvailability, CredentialStore, CredentialStoreError, Secret};

pub struct CredentialService {
    store: Box<dyn CredentialStore>,
}

impl CredentialService {
    pub fn new(store: Box<dyn CredentialStore>) -> Self {
        Self { store }
    }

    pub fn availability(&self) -> CredentialAvailability {
        let availability = self
            .store
            .availability()
            .unwrap_or(CredentialAvailability::Unavailable);
        tracing::debug!(?availability, "credential store availability checked");
        availability
    }

    #[tracing::instrument(skip(self), fields(reference_id = %reference.id))]
    pub fn get(&self, reference: &CredentialRef) -> CoreResult<Option<Secret>> {
        let secret = self.store.get(reference).map_err(map_store_error)?;
        tracing::debug!(found = secret.is_some(), "credential lookup completed");
        Ok(secret)
    }

    #[tracing::instrument(skip(self, secret), fields(reference_id = %reference.id))]
    pub fn set(&self, reference: &CredentialRef, secret: Secret) -> CoreResult<()> {
        self.store.set(reference, secret).map_err(map_store_error)
    }

    #[tracing::instrument(skip(self), fields(reference_id = %reference.id))]
    pub fn delete(&self, reference: &CredentialRef) -> CoreResult<()> {
        self.store.delete(reference).map_err(map_store_error)
    }
}

fn map_store_error(error: CredentialStoreError) -> CoreError {
    match error {
        CredentialStoreError::Unavailable => {
            CoreError::CredentialStoreUnavailable("the credential store is unavailable".to_owned())
        }
        CredentialStoreError::Locked => {
            CoreError::CredentialStoreUnavailable("the credential store is locked".to_owned())
        }
        CredentialStoreError::Denied => {
            CoreError::CredentialStoreUnavailable("credential access was denied".to_owned())
        }
        CredentialStoreError::NotFound => CoreError::CredentialMissing,
        CredentialStoreError::InvalidReference => {
            CoreError::Validation("invalid credential reference".to_owned())
        }
        CredentialStoreError::Backend(message) => CoreError::internal(message),
    }
}
