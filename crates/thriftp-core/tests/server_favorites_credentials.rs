mod support;

use std::sync::Arc;

use support::MemoryCredentialStore;
use thriftp_core::credentials::{CredentialService, Secret};
use thriftp_core::model::{AuthMethod, CredentialPolicy, Protocol};
use thriftp_core::repository::{MemoryStore, MetadataStore};
use thriftp_core::server_favorites::{ServerFavoriteInput, ServerFavoriteService};

fn input(name: &str) -> ServerFavoriteInput {
    ServerFavoriteInput {
        display_name: name.to_owned(),
        group_label: None,
        protocol: Protocol::Sftp,
        host: "shared.example".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
    }
}

#[test]
fn shared_credentials_are_listed_removed_and_deleted_explicitly() {
    let service = ServerFavoriteService::new(Arc::new(MemoryStore::default()));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let first = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let ref_id = service.impact(first.id).unwrap().credential_ref_id;
    let second = service
        .create(
            input("beta"),
            CredentialPolicy::Save,
            Some(ref_id),
            None,
            &credentials,
        )
        .unwrap();

    let impact = service.impact(second.id).unwrap();
    assert!(impact.is_shared);
    assert_eq!(impact.affected_favorites.len(), 2);
    assert_eq!(service.reusable().unwrap()[0].affected_favorite_count, 2);

    let removed = service.remove_credential(first.id, &credentials).unwrap();
    assert!(removed.is_shared);
    let remaining = service.get(first.id).unwrap();
    assert_eq!(remaining.credential_policy, CredentialPolicy::Ask);
    assert!(!remaining.has_saved_credential);

    service.remove_credential(second.id, &credentials).unwrap();
    service
        .delete_credential(ref_id, true, &credentials)
        .unwrap();
    assert!(service.reusable().unwrap().is_empty());
}

#[test]
fn editing_one_shared_favorite_keeps_the_other_reference_usable() {
    let store = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(store);
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let first = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"shared".to_vec())),
            &credentials,
        )
        .unwrap();
    let ref_id = service.impact(first.id).unwrap().credential_ref_id;
    let second = service
        .create(
            input("beta"),
            CredentialPolicy::Save,
            Some(ref_id),
            None,
            &credentials,
        )
        .unwrap();

    let mut moved = input("alpha");
    moved.host = "moved.example".to_owned();
    service
        .update(
            first.id,
            moved,
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"replacement".to_vec())),
            &credentials,
        )
        .unwrap();

    let (_, old_secret) = service.connection_target(second.id, &credentials).unwrap();
    assert!(old_secret.is_some());
    let (_, new_secret) = service.connection_target(first.id, &credentials).unwrap();
    assert!(new_secret.is_some());
    assert_eq!(
        service.impact(second.id).unwrap().affected_favorites.len(),
        1
    );
}

#[test]
fn unavailable_store_saves_non_secret_settings_with_a_warning() {
    let store = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(store.clone());
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::unavailable()));
    let view = service
        .create(
            input("offline"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"never-stored".to_vec())),
            &credentials,
        )
        .unwrap();
    assert_eq!(view.credential_policy, CredentialPolicy::Ask);
    assert!(!view.has_saved_credential);
    assert_eq!(
        view.credential_warning
            .as_ref()
            .map(|warning| warning.code.as_str()),
        Some("credential_store_unavailable")
    );
    let state = store.load().unwrap();
    assert_eq!(state.server_favorites.len(), 1);
    assert_eq!(state.connection_profiles.len(), 1);
    assert!(state.credential_refs.is_empty());
}

#[test]
fn updating_a_shared_credential_from_a_reusing_favorite_updates_every_reference() {
    let store = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(store.clone());
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let first = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"first".to_vec())),
            &credentials,
        )
        .unwrap();
    let ref_id = service.impact(first.id).unwrap().credential_ref_id;
    let second = service
        .create(
            input("beta"),
            CredentialPolicy::Save,
            Some(ref_id),
            None,
            &credentials,
        )
        .unwrap();

    service
        .update(
            second.id,
            input("beta"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"second".to_vec())),
            &credentials,
        )
        .unwrap();

    let state = store.load().unwrap();
    assert_eq!(state.credential_refs.len(), 1);
    assert_eq!(state.server_favorites[0].credential_ref_id, Some(ref_id));
    assert_eq!(state.server_favorites[1].credential_ref_id, Some(ref_id));
    assert!(
        state.credential_refs[0].matches_profile(
            state
                .connection_profiles
                .iter()
                .find(|profile| profile.id == second.connection_profile_id)
                .unwrap()
        )
    );
}

#[test]
fn reconcile_profiles_invalidates_external_identity_changes() {
    let store = std::sync::Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(store.clone());
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let favorite = service
        .create(
            input("external"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"old-secret".to_vec())),
            &credentials,
        )
        .unwrap();

    let mut state = store.load().unwrap();
    state
        .connection_profiles
        .iter_mut()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .unwrap()
        .username = "changed".to_owned();
    store.save(&state).unwrap();

    assert_eq!(service.reconcile_profiles(&credentials).unwrap(), 1);
    let state = store.load().unwrap();
    assert_eq!(
        state.server_favorites[0].credential_policy,
        CredentialPolicy::Ask
    );
    assert!(state.server_favorites[0].credential_ref_id.is_none());
    assert!(state.credential_refs.is_empty());
}
