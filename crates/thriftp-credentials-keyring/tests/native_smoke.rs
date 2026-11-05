use thriftp_core::credentials::{CredentialStore, Secret};
use thriftp_core::model::{AuthMethod, CredentialRef, Protocol};
use thriftp_credentials_keyring::KeyringCredentialStore;
use uuid::Uuid;

#[test]
fn native_store_round_trip_when_enabled() {
    if std::env::var("THRIFTP_NATIVE_KEYRING").as_deref() != Ok("1") {
        return;
    }
    let store = KeyringCredentialStore::new();
    let reference = CredentialRef {
        id: Uuid::new_v4(),
        protocol: Protocol::Sftp,
        host: "native-smoke.test".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    store
        .set(&reference, Secret::new(b"sentinel-value".to_vec()))
        .expect("set native credential");
    assert!(
        store
            .get(&reference)
            .expect("get native credential")
            .is_some()
    );
    store.delete(&reference).expect("delete native credential");
}
