mod support;

use std::sync::Arc;

use support::MemoryCredentialStore;
use thriftp_core::credentials::{CredentialService, Secret};
use thriftp_core::model::{
    AuthMethod, CredentialPolicy, FavoritePreferences, FavoriteSortKey, Protocol,
};
use thriftp_core::repository::MemoryStore;
use thriftp_core::server_favorites::{ServerFavoriteInput, ServerFavoriteService};

fn input(name: &str) -> ServerFavoriteInput {
    ServerFavoriteInput {
        display_name: name.to_owned(),
        group_label: None,
        protocol: Protocol::Sftp,
        host: "example.test".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
    }
}

fn service() -> (ServerFavoriteService, CredentialService) {
    (
        ServerFavoriteService::new(Arc::new(MemoryStore::default())),
        CredentialService::new(Box::new(MemoryCredentialStore::available())),
    )
}

#[test]
fn creates_lists_pins_and_groups_favorites() {
    let (service, credentials) = service();
    let view = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    assert_eq!(view.display_name, "alpha");
    assert!(view.has_saved_credential);

    let second = service
        .create(
            input("beta"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    service.set_pinned(second.id, true).unwrap();
    service
        .set_group(second.id, Some("Production".to_owned()))
        .unwrap();

    let list = service.list().unwrap();
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].display_name, "beta");
    assert_eq!(list[0].group_label.as_deref(), Some("Production"));

    let preferences = FavoritePreferences {
        sort_key: FavoriteSortKey::Name,
        pinned_first: false,
    };
    service.set_preferences(preferences.clone()).unwrap();
    assert_eq!(service.preferences().unwrap(), preferences);
}

#[test]
fn update_invalidates_identity_bound_credentials() {
    let (service, credentials) = service();
    let view = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let mut changed = input("alpha");
    changed.username = "other".to_owned();
    let result = service.update(
        view.id,
        changed,
        CredentialPolicy::Save,
        None,
        None,
        &credentials,
    );
    assert!(result.is_err());
}

#[test]
fn delete_reports_shared_credential_impact() {
    let (service, credentials) = service();
    let first = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let reference = service.impact(first.id).unwrap();
    assert!(!reference.is_shared);
    let impact = service.delete(first.id).unwrap();
    assert_eq!(impact.credential_ref_id, reference.credential_ref_id);
    assert!(service.list().unwrap().is_empty());
}

#[test]
fn shared_credential_can_be_reused_for_matching_identity() {
    let (service, credentials) = service();
    let first = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let impact = service.impact(first.id).unwrap();
    let reusable = service.reusable().unwrap();
    assert_eq!(reusable.len(), 1);
    assert_eq!(reusable[0].affected_favorite_count, 1);
    let second = service
        .create(
            input("beta"),
            CredentialPolicy::Save,
            Some(impact.credential_ref_id),
            None,
            &credentials,
        )
        .unwrap();
    let shared = service.impact(second.id).unwrap();
    assert!(shared.is_shared);
    assert_eq!(shared.affected_favorites.len(), 2);
}

#[test]
fn persisted_state_never_contains_secret_material() {
    use thriftp_core::repository::MetadataStore;

    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"sentinel-secret-value".to_vec())),
            &credentials,
        )
        .unwrap();
    let persisted = serde_json::to_string(&store.load().unwrap()).unwrap();
    assert!(!persisted.contains("sentinel-secret-value"));
    assert!(!persisted.contains("privateKey"));
}

#[test]
fn invalid_input_returns_validation_instead_of_panicking() {
    let (service, credentials) = service();
    let mut bad = input(" ");
    bad.host = String::new();
    let error = service
        .create(bad, CredentialPolicy::Ask, None, None, &credentials)
        .unwrap_err();
    assert_eq!(error.code(), "validation");
}

#[test]
fn connection_target_returns_saved_credential_for_favorite() {
    let (service, credentials) = service();
    let view = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"saved-secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let (profile, secret) = service.connection_target(view.id, &credentials).unwrap();
    assert_eq!(profile.id, view.connection_profile_id);
    assert!(secret.is_some());

    let ask = service
        .create(
            input("beta"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    let (_, secret) = service.connection_target(ask.id, &credentials).unwrap();
    assert!(secret.is_none());
}
