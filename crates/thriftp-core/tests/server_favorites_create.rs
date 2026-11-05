mod support;

use support::MemoryCredentialStore;
use thriftp_core::credentials::CredentialService;
use thriftp_core::model::{AuthMethod, CredentialPolicy, Protocol, ServerFavorite};
use thriftp_core::repository::{MemoryStore, MetadataStore};
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

#[test]
fn create_and_reload_from_store() {
    let store: std::sync::Arc<dyn MetadataStore> = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(std::sync::Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let created = service
        .create(
            input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    assert_eq!(created.display_name, "alpha");
    let reloaded = service.list().unwrap();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(reloaded[0].id, created.id);
}

#[test]
fn invalid_profile_is_rejected() {
    let service = ServerFavoriteService::new(std::sync::Arc::new(MemoryStore::default()));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let mut bad = input("bad");
    bad.host = String::new();
    assert!(
        service
            .create(bad, CredentialPolicy::Ask, None, None, &credentials)
            .is_err()
    );
}

#[test]
fn persisted_state_has_one_favorite_per_profile() {
    let store: std::sync::Arc<dyn MetadataStore> = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(std::sync::Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    for name in ["alpha", "beta"] {
        service
            .create(input(name), CredentialPolicy::Ask, None, None, &credentials)
            .unwrap();
    }
    let state = store.load().unwrap();
    assert_eq!(
        state.connection_profiles.len(),
        state.server_favorites.len()
    );
}

#[test]
fn backfills_existing_profiles_and_deduplicates_favorites() {
    let store: std::sync::Arc<dyn MetadataStore> = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(std::sync::Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let profile = thriftp_core::model::ConnectionProfile::new(
        "legacy",
        Protocol::Sftp,
        "legacy.example",
        22,
        "tester",
        AuthMethod::Password,
        30,
    );
    let duplicate = ServerFavorite::new(profile.id, CredentialPolicy::Ask, None);
    let mut state = store.load().unwrap();
    state.connection_profiles.push(profile);
    state.server_favorites.push(duplicate.clone());
    state.server_favorites.push(duplicate);
    store.save(&state).unwrap();

    assert_eq!(service.ensure_favorites().unwrap(), 0);
    let state = store.load().unwrap();
    assert_eq!(state.server_favorites.len(), 1);

    let extra = thriftp_core::model::ConnectionProfile::new(
        "external",
        Protocol::Ftp,
        "external.example",
        21,
        "tester",
        AuthMethod::Password,
        30,
    );
    let mut state = store.load().unwrap();
    state.connection_profiles.push(extra.clone());
    store.save(&state).unwrap();
    assert_eq!(service.ensure_favorites().unwrap(), 1);
    let state = store.load().unwrap();
    assert!(
        state
            .server_favorites
            .iter()
            .any(|favorite| favorite.connection_profile_id == extra.id)
    );
    let _ = credentials;
}
