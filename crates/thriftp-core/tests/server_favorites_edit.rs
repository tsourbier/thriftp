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
        host: "edit.example".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
    }
}

#[test]
fn auth_method_and_protocol_changes_require_compatible_credentials() {
    let service = ServerFavoriteService::new(Arc::new(MemoryStore::default()));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let view = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();

    let mut ssh = input("alpha");
    ssh.auth_method = AuthMethod::SshPrivateKey;
    ssh.ssh_key_ref = Some("/keys/id_ed25519".to_owned());
    assert!(
        service
            .update(
                view.id,
                ssh.clone(),
                CredentialPolicy::Save,
                None,
                None,
                &credentials
            )
            .is_err()
    );
    let updated = service
        .update(
            view.id,
            ssh,
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"passphrase".to_vec())),
            &credentials,
        )
        .unwrap();
    assert_eq!(updated.auth_method, AuthMethod::SshPrivateKey);

    let mut ftp = input("alpha");
    ftp.protocol = Protocol::Ftp;
    let updated = service
        .update(
            view.id,
            ftp,
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"new-secret".to_vec())),
            &credentials,
        )
        .unwrap();
    assert_eq!(updated.protocol, Protocol::Ftp);

    let cleared = service
        .update(
            view.id,
            input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    assert_eq!(cleared.credential_policy, CredentialPolicy::Ask);
    assert!(!cleared.has_saved_credential);
}

#[test]
fn private_key_authentication_requires_a_key_reference() {
    let service = ServerFavoriteService::new(Arc::new(MemoryStore::default()));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let mut ssh = input("alpha");
    ssh.auth_method = AuthMethod::SshPrivateKey;
    ssh.ssh_key_ref = None;
    assert!(
        service
            .create(ssh, CredentialPolicy::Ask, None, None, &credentials)
            .is_err()
    );
}

#[tokio::test]
async fn test_connection_does_not_persist_and_active_connections_survive_delete() {
    use support::FakeFactory;
    use thriftp_core::connections::ConnectionService;
    use thriftp_core::repository::MetadataStore;

    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let view = service
        .create(
            input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    let profile = store
        .load()
        .unwrap()
        .connection_profiles
        .into_iter()
        .find(|profile| profile.id == view.connection_profile_id)
        .unwrap();

    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    connections
        .test(profile.id, Some(Secret::new(b"session".to_vec())))
        .await
        .unwrap();
    assert_eq!(service.list().unwrap().len(), 1);

    let active = connections
        .connect(profile.id, Some(Secret::new(b"session".to_vec())))
        .await
        .unwrap();
    service.delete(view.id).unwrap();
    assert_eq!(connections.active_connections().len(), 1);
    assert_eq!(connections.active_connections()[0].id, active.id);
}

#[test]
fn syncing_an_external_profile_edit_invalidates_stale_credentials() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let favorite = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let mut profile = store
        .load()
        .unwrap()
        .connection_profiles
        .iter()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .cloned()
        .unwrap();
    profile.host = "moved.example".to_owned();
    let view = service.sync_profile(profile, &credentials).unwrap();
    assert_eq!(view.host, "moved.example");
    assert_eq!(view.credential_policy, CredentialPolicy::Ask);
    assert!(!view.has_saved_credential);
    assert!(store.load().unwrap().credential_refs.is_empty());
}
