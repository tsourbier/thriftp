mod support;

use std::sync::Arc;

use support::{FakeFactory, MemoryCredentialStore};
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{CredentialService, Secret};
use thriftp_core::error::CoreError;
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, CredentialPolicy, CredentialRef, Protocol,
};
use thriftp_core::repository::{MemoryStore, MetadataStore};
use thriftp_core::server_favorites::{ServerFavoriteInput, ServerFavoriteService};

fn input(name: &str) -> ServerFavoriteInput {
    ServerFavoriteInput {
        display_name: name.to_owned(),
        group_label: None,
        protocol: Protocol::Sftp,
        host: "connect.example".to_owned(),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
    }
}

async fn connect_profile(
    profile: &ConnectionProfile,
    credential: Option<Secret>,
) -> Result<thriftp_core::model::ActiveConnection, CoreError> {
    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    connections.connect(profile.id, credential).await
}

#[tokio::test]
async fn saved_credentials_connect_and_last_used_updates_only_on_success() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let saved = service
        .create(
            input("alpha"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();

    let (profile, secret) = service.connection_target(saved.id, &credentials).unwrap();
    assert!(secret.is_some());
    connect_profile(&profile, secret).await.unwrap();
    service.mark_used(saved.id).unwrap();
    assert!(service.get(saved.id).unwrap().last_used_at.is_some());

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
    assert!(connect_profile(&profile, None).await.is_err());
    assert!(service.get(ask.id).unwrap().last_used_at.is_none());
}

#[tokio::test]
async fn missing_and_rejected_credentials_fail_without_touching_last_used() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));

    let missing = service
        .create(
            input("missing"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"secret".to_vec())),
            &credentials,
        )
        .unwrap();
    let state = store.load().unwrap();
    let reference = state.credential_refs[0].clone();
    credentials.delete(&reference).unwrap();
    let (profile, secret) = service.connection_target(missing.id, &credentials).unwrap();
    assert!(secret.is_none());
    assert!(matches!(
        connect_profile(&profile, secret).await,
        Err(CoreError::CredentialMissing)
    ));
    assert!(service.get(missing.id).unwrap().last_used_at.is_none());

    let rejected = service
        .create(
            input("rejected"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"rejected".to_vec())),
            &credentials,
        )
        .unwrap();
    let (profile, secret) = service
        .connection_target(rejected.id, &credentials)
        .unwrap();
    assert!(matches!(
        connect_profile(&profile, secret).await,
        Err(CoreError::CredentialRejected)
    ));
    assert!(service.get(rejected.id).unwrap().last_used_at.is_none());
}

#[tokio::test]
async fn unavailable_locked_and_denied_stores_fail_closed() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let available = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let favorite = service
        .create(
            input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &available,
        )
        .unwrap();

    let mut state = store.load().unwrap();
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .unwrap()
        .clone();
    let reference = CredentialRef::for_profile(&profile);
    state.credential_refs.push(reference.clone());
    let stored = state
        .server_favorites
        .iter_mut()
        .find(|item| item.id == favorite.id)
        .unwrap();
    stored.credential_policy = CredentialPolicy::Save;
    stored.credential_ref_id = Some(reference.id);
    store.save(&state).unwrap();

    for mode in [
        MemoryCredentialStore::unavailable(),
        MemoryCredentialStore::locked(),
        MemoryCredentialStore::denied(),
    ] {
        let credentials = CredentialService::new(Box::new(mode));
        match service.connection_target(favorite.id, &credentials) {
            Err(CoreError::CredentialStoreUnavailable(_)) => {}
            Ok(_) => panic!("expected an unavailable-store error"),
            Err(_) => panic!("expected an unavailable-store error"),
        }
    }
}

#[tokio::test]
async fn session_only_credentials_are_used_but_never_persisted() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let favorite = service
        .create(
            input("session"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();

    let (profile, stored) = service
        .connection_target(favorite.id, &credentials)
        .unwrap();
    assert!(stored.is_none());
    connect_profile(&profile, Some(Secret::new(b"session-secret".to_vec())))
        .await
        .unwrap();

    let state = store.load().unwrap();
    assert!(state.credential_refs.is_empty());
    assert_eq!(
        state.server_favorites[0].credential_policy,
        CredentialPolicy::Ask
    );
}

#[tokio::test]
async fn disconnected_and_reconnected_sessions_leave_the_favorite_intact() {
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
    let (profile, _) = service
        .connection_target(favorite.id, &credentials)
        .unwrap();

    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    let first = connections
        .connect(profile.id, Some(Secret::new(b"secret".to_vec())))
        .await
        .unwrap();
    let second = connections
        .reconnect(first.id, Some(Secret::new(b"secret".to_vec())))
        .await
        .unwrap();
    assert_eq!(connections.active_connections().len(), 1);
    connections.disconnect(second.id).await.unwrap();
    assert!(connections.active_connections().is_empty());

    let third = connections
        .connect(profile.id, Some(Secret::new(b"secret".to_vec())))
        .await
        .unwrap();
    assert_eq!(connections.active_connections().len(), 1);
    assert_eq!(service.list().unwrap().len(), 1);
    assert_eq!(connections.active_connections()[0].id, third.id);
}

#[tokio::test]
async fn certificate_and_host_key_failures_never_bypass_verification() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));

    let cases = [
        (
            "certificate.invalid",
            Protocol::FtpsImplicit,
            "certificate_invalid",
        ),
        ("hostkey.unknown", Protocol::Sftp, "host_key_unknown"),
        ("hostkey.changed", Protocol::Sftp, "host_key_changed"),
    ];
    for (host, protocol, _code) in cases {
        let mut profile = input(host);
        profile.host = host.to_owned();
        profile.protocol = protocol;
        let favorite = service
            .create(profile, CredentialPolicy::Ask, None, None, &credentials)
            .unwrap();
        let (profile, _) = service
            .connection_target(favorite.id, &credentials)
            .unwrap();
        let error = connect_profile(&profile, Some(Secret::new(b"secret".to_vec())))
            .await
            .unwrap_err();
        match host {
            "certificate.invalid" => {
                assert!(matches!(error, CoreError::CertificateInvalid(_)))
            }
            "hostkey.unknown" => assert!(matches!(error, CoreError::HostKeyUnknown(_))),
            "hostkey.changed" => assert!(matches!(error, CoreError::HostKeyChanged(_))),
            _ => unreachable!(),
        }
        assert!(service.get(favorite.id).unwrap().last_used_at.is_none());
    }
}

#[tokio::test]
async fn a_missing_or_moved_ssh_key_requests_a_new_reference() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let mut ssh = input("keys");
    ssh.host = "key.missing".to_owned();
    ssh.auth_method = AuthMethod::SshPrivateKey;
    ssh.ssh_key_ref = Some("/old/id_ed25519".to_owned());
    let favorite = service
        .create(ssh.clone(), CredentialPolicy::Ask, None, None, &credentials)
        .unwrap();

    let (profile, _) = service
        .connection_target(favorite.id, &credentials)
        .unwrap();
    assert!(matches!(
        connect_profile(&profile, Some(Secret::new(b"passphrase".to_vec()))).await,
        Err(CoreError::AuthFailed(message)) if message.contains("missing")
    ));

    let mut replacement = ssh;
    replacement.host = "connect.example".to_owned();
    replacement.ssh_key_ref = Some("/new/id_ed25519".to_owned());
    service
        .update(
            favorite.id,
            replacement,
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    let state = store.load().unwrap();
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .unwrap();
    assert_eq!(profile.ssh_key_ref.as_deref(), Some("/new/id_ed25519"));
    assert!(state.credential_refs.is_empty());

    let (profile, secret) = service
        .connection_target(favorite.id, &credentials)
        .unwrap();
    assert!(secret.is_none());
    connect_profile(&profile, Some(Secret::new(b"passphrase".to_vec())))
        .await
        .unwrap();
}

#[test]
fn connection_target_rejects_a_stale_identity_reference() {
    let store: Arc<dyn MetadataStore> = Arc::new(MemoryStore::default());
    let service = ServerFavoriteService::new(Arc::clone(&store));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let favorite = service
        .create(
            input("stale"),
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"old-secret".to_vec())),
            &credentials,
        )
        .unwrap();

    let mut state = store.load().unwrap();
    let profile = state
        .connection_profiles
        .iter_mut()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .unwrap();
    profile.host = "moved.example".to_owned();
    store.save(&state).unwrap();

    let (_, secret) = service
        .connection_target(favorite.id, &credentials)
        .unwrap();
    assert!(secret.is_none());
}
