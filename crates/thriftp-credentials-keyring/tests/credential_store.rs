use std::error::Error;

use thriftp_core::credentials::{
    CredentialAvailability, CredentialStore, CredentialStoreError, Secret,
};
use thriftp_core::model::{AuthMethod, CredentialRef, Protocol};
use thriftp_credentials_keyring::{KeyringCredentialStore, map_keyring_error};

fn reference(name: &str) -> CredentialRef {
    CredentialRef {
        id: uuid::Uuid::new_v4(),
        protocol: Protocol::Sftp,
        host: format!("{name}.example"),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

fn backend_error(message: &'static str) -> Box<dyn Error + Send + Sync> {
    Box::new(std::io::Error::other(message))
}

#[test]
fn availability_probe_reports_a_known_state_without_panicking() {
    let store = KeyringCredentialStore::new();
    match store.availability() {
        Ok(
            CredentialAvailability::Available
            | CredentialAvailability::Locked
            | CredentialAvailability::Unavailable
            | CredentialAvailability::Denied,
        ) => {}
        Err(CredentialStoreError::Backend(_)) => {}
        Err(error) => panic!("unexpected availability error: {error}"),
    }
}

#[test]
fn maps_locked_unavailable_denied_missing_and_invalid_references() {
    assert_eq!(
        map_keyring_error(keyring::Error::NoEntry),
        CredentialStoreError::NotFound
    );
    assert_eq!(
        map_keyring_error(keyring::Error::NoStorageAccess(backend_error(
            "keychain locked"
        ))),
        CredentialStoreError::Locked
    );
    assert_eq!(
        map_keyring_error(keyring::Error::NoStorageAccess(backend_error(
            "access denied by policy"
        ))),
        CredentialStoreError::Denied
    );
    assert_eq!(
        map_keyring_error(keyring::Error::PlatformFailure(backend_error(
            "secret service unavailable"
        ))),
        CredentialStoreError::Unavailable
    );
    assert_eq!(
        map_keyring_error(keyring::Error::Invalid(
            "account".to_owned(),
            "empty".to_owned()
        )),
        CredentialStoreError::InvalidReference
    );
}

#[test]
fn missing_entry_and_invalid_reference_behavior_when_available() {
    let store = KeyringCredentialStore::new();
    if store.availability() != Ok(CredentialAvailability::Available) {
        return;
    }
    let missing = reference("missing");
    assert!(store.get(&missing).unwrap().is_none());
    assert_eq!(
        store.delete(&missing).unwrap_err(),
        CredentialStoreError::NotFound
    );
}

#[test]
fn put_get_update_delete_round_trip_when_available() {
    if std::env::var("THRIFTP_NATIVE_KEYRING").as_deref() != Ok("1") {
        return;
    }
    let store = KeyringCredentialStore::new();
    if store.availability() != Ok(CredentialAvailability::Available) {
        return;
    }
    let reference = reference("round-trip");
    store
        .set(&reference, Secret::new(b"first-value".to_vec()))
        .unwrap();
    let first = store.get(&reference).unwrap().expect("stored secret");
    assert_eq!(first.expose(), b"first-value");

    store
        .set(&reference, Secret::new(b"second-value".to_vec()))
        .unwrap();
    let second = store.get(&reference).unwrap().expect("updated secret");
    assert_eq!(second.expose(), b"second-value");

    store.delete(&reference).unwrap();
    assert!(store.get(&reference).unwrap().is_none());
}
