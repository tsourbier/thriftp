//! Native credential-store adapter.
//!
//! Uses the operating-system credential store through the `keyring` crate.
//! There is no plaintext fallback.

#![deny(missing_docs)]

use keyring::Entry;
use thriftp_core::credentials::{
    CredentialAvailability, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::model::CredentialRef;
use zeroize::Zeroizing;

const SERVICE: &str = "dev.thriftp.credentials";

/// Platform-native implementation of the core credential-store boundary.
pub struct KeyringCredentialStore;

impl KeyringCredentialStore {
    /// Create an adapter that uses the current user's native credential store.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    fn entry(reference: &CredentialRef) -> Result<Entry, CredentialStoreError> {
        Entry::new(SERVICE, &reference.id.to_string()).map_err(map_keyring_error)
    }
}

impl Default for KeyringCredentialStore {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError> {
        let entry = Entry::new(SERVICE, "__availability_probe__")
            .map_err(|error| CredentialStoreError::Backend(error.to_string()))?;
        match entry.get_password() {
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(CredentialAvailability::Available),
            Err(keyring::Error::NoStorageAccess(message)) => Ok(if is_locked(&message) {
                CredentialAvailability::Locked
            } else {
                CredentialAvailability::Denied
            }),
            Err(keyring::Error::PlatformFailure(_)) => Ok(CredentialAvailability::Unavailable),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn get(&self, reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError> {
        let entry = Self::entry(reference)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(Secret::new(password.into_bytes()))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(map_keyring_error(error)),
        }
    }

    fn set(&self, reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError> {
        let entry = Self::entry(reference)?;
        let password = Zeroizing::new(String::from_utf8_lossy(secret.expose()).into_owned());
        entry.set_password(&password).map_err(map_keyring_error)
    }

    fn delete(&self, reference: &CredentialRef) -> Result<(), CredentialStoreError> {
        let entry = Self::entry(reference)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Err(CredentialStoreError::NotFound),
            Err(error) => Err(map_keyring_error(error)),
        }
    }
}

/// Maps a platform keyring error to the stable credential-store error surface.
///
/// Raw platform diagnostics are not exposed to the UI; unavailable, locked,
/// and denied states remain distinct so the UI can explain recovery options.
#[must_use]
pub fn map_keyring_error(error: keyring::Error) -> CredentialStoreError {
    match error {
        keyring::Error::NoEntry => CredentialStoreError::NotFound,
        keyring::Error::NoStorageAccess(message) => {
            if is_locked(&message) {
                CredentialStoreError::Locked
            } else {
                CredentialStoreError::Denied
            }
        }
        keyring::Error::PlatformFailure(_message) => CredentialStoreError::Unavailable,
        keyring::Error::Invalid(_, _) => CredentialStoreError::InvalidReference,
        other => CredentialStoreError::Backend(other.to_string()),
    }
}

fn is_locked(message: &impl std::fmt::Display) -> bool {
    message.to_string().to_lowercase().contains("lock")
}
