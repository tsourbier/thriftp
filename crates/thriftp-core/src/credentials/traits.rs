use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::model::CredentialRef;

#[derive(Zeroize, ZeroizeOnDrop)]
pub struct Secret(Vec<u8>);

impl Secret {
    pub fn new(value: impl Into<Vec<u8>>) -> Self {
        Self(value.into())
    }

    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialAvailability {
    Available,
    Locked,
    Unavailable,
    Denied,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum CredentialStoreError {
    #[error("credential store is unavailable")]
    Unavailable,
    #[error("credential store is locked")]
    Locked,
    #[error("credential access was denied")]
    Denied,
    #[error("credential was not found")]
    NotFound,
    #[error("credential reference is invalid")]
    InvalidReference,
    #[error("credential backend error: {0}")]
    Backend(String),
}

pub trait CredentialStore: Send + Sync {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError>;
    fn get(&self, reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError>;
    fn set(&self, reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError>;
    fn delete(&self, reference: &CredentialRef) -> Result<(), CredentialStoreError>;
}
