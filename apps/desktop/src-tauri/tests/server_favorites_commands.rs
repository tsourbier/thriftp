mod support;

use async_trait::async_trait;
use support::{TestApp, assert_no_secret, desktop_code, favorite_input, seed_dependent_data};
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::Secret;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    AuthMethod, CredentialPolicy, FavoritePreferences, FavoriteSortKey, FileEntry, Protocol,
    ProtocolCapabilities,
};
use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};

struct FakeRemoteFs;

#[async_trait]
impl RemoteFs for FakeRemoteFs {
    async fn connect(
        &mut self,
        profile: &thriftp_core::model::ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        if profile.host == "certificate.invalid" {
            return Err(CoreError::CertificateInvalid(
                "the certificate could not be verified".to_owned(),
            ));
        }
        match credential {
            None => Err(CoreError::CredentialMissing),
            Some(secret) if secret.expose() == b"rejected" => Err(CoreError::CredentialRejected),
            Some(_) => Ok(()),
        }
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }

    async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }

    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
        Err(CoreError::NotFound(path.to_owned()))
    }

    async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
        Ok(())
    }

    async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
        Ok(())
    }
}

struct FakeFactory;

impl RemoteFsFactory for FakeFactory {
    fn create(&self, _protocol: Protocol) -> Box<dyn RemoteFs> {
        Box::new(FakeRemoteFs)
    }
}

#[test]
fn favorites_list_get_update_pin_group_rename_and_preferences_contract() {
    let app = TestApp::new();
    let alpha = app
        .create(favorite_input("alpha"), CredentialPolicy::Ask, None, None)
        .unwrap();
    let beta = app
        .create(favorite_input("beta"), CredentialPolicy::Ask, None, None)
        .unwrap();

    assert_eq!(app.service.list().unwrap().len(), 2);
    assert_eq!(app.service.get(alpha.id).unwrap().display_name, "alpha");

    let mut renamed = favorite_input("alpha-renamed");
    renamed.group_label = Some("Production".to_owned());
    let updated = app
        .service
        .update(
            alpha.id,
            renamed,
            CredentialPolicy::Ask,
            None,
            None,
            &app.credentials,
        )
        .unwrap();
    assert_eq!(updated.display_name, "alpha-renamed");
    assert_eq!(updated.group_label.as_deref(), Some("Production"));

    assert!(app.service.set_pinned(beta.id, true).unwrap().pinned);
    assert_eq!(
        app.service
            .set_group(beta.id, Some("Production".to_owned()))
            .unwrap()
            .group_label
            .as_deref(),
        Some("Production")
    );
    let renamed = app.service.rename_group("Production", "Prod").unwrap();
    assert!(
        renamed
            .iter()
            .all(|favorite| { favorite.group_label.as_deref() == Some("Prod") })
    );
    app.service.set_group(beta.id, None).unwrap();

    let preferences = FavoritePreferences {
        sort_key: FavoriteSortKey::Host,
        pinned_first: true,
    };
    assert_eq!(
        app.service.set_preferences(preferences).unwrap().sort_key,
        FavoriteSortKey::Host
    );
    let listed = app.service.list().unwrap();
    assert!(listed[0].pinned);

    assert_eq!(
        desktop_code(&app.service.get(uuid::Uuid::new_v4()).unwrap_err()),
        "not_found"
    );
}

#[test]
fn favorites_delete_removes_dependent_metadata_without_touching_unrelated_data() {
    let app = TestApp::new();
    let alpha = app
        .create(favorite_input("alpha"), CredentialPolicy::Ask, None, None)
        .unwrap();
    let beta = app
        .create(favorite_input("beta"), CredentialPolicy::Ask, None, None)
        .unwrap();
    seed_dependent_data(
        &app,
        alpha.connection_profile_id,
        beta.connection_profile_id,
    );

    app.service.delete(alpha.id).unwrap();

    let state = app.state();
    assert_eq!(state.server_favorites.len(), 1);
    assert_eq!(
        state.server_favorites[0].connection_profile_id,
        beta.connection_profile_id
    );
    assert_eq!(state.connection_profiles.len(), 1);
    assert_eq!(state.connection_profiles[0].id, beta.connection_profile_id);
    assert_eq!(state.directory_favorites.len(), 1);
    assert_eq!(
        state.directory_favorites[0].connection_profile_id,
        beta.connection_profile_id
    );
    assert_eq!(
        state
            .tabs
            .iter()
            .filter(|tab| tab.connection_profile_id.is_some())
            .count(),
        1
    );
    assert_eq!(
        state.tabs[0].connection_profile_id,
        Some(beta.connection_profile_id)
    );
}

#[tokio::test]
async fn favorites_connect_and_disconnect_contract_records_success_only() {
    let app = TestApp::new();
    let favorite = app
        .create(
            favorite_input("alpha"),
            CredentialPolicy::Save,
            None,
            Some("connect-sentinel"),
        )
        .unwrap();
    let (profile, stored) = app
        .service
        .connection_target(favorite.id, &app.credentials)
        .unwrap();
    assert!(stored.is_some());

    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, stored).await.unwrap();
    app.service.mark_used(favorite.id).unwrap();
    assert!(app.service.get(favorite.id).unwrap().last_used_at.is_some());

    let view = thriftp_desktop_lib::views::ConnectionView::from(active.clone());
    let text = assert_no_secret(&view, "connect-sentinel");
    assert!(text.contains("\"connectionId\""));
    assert!(text.contains("\"profileId\""));
    assert!(text.contains("\"state\":\"connecting\""));

    connections.disconnect(active.id).await.unwrap();
    assert!(connections.active_connections().is_empty());

    let error = app.service.get(uuid::Uuid::new_v4()).unwrap_err();
    assert_eq!(desktop_code(&error), "not_found");
}

#[test]
fn credentials_impact_reuse_remove_and_delete_contract() {
    let app = TestApp::new();
    let first = app
        .create(
            favorite_input("first"),
            CredentialPolicy::Save,
            None,
            Some("first-sentinel"),
        )
        .unwrap();
    let impact = app.service.impact(first.id).unwrap();
    let ref_id = impact.credential_ref_id;
    assert!(!impact.is_shared);

    let second = app
        .create(
            favorite_input("second"),
            CredentialPolicy::Save,
            Some(ref_id),
            None,
        )
        .unwrap();
    let shared = app.service.impact(second.id).unwrap();
    assert!(shared.is_shared);
    assert_eq!(shared.affected_favorites.len(), 2);

    let reusable = app.service.reusable().unwrap();
    assert_eq!(reusable.len(), 1);
    assert_eq!(reusable[0].affected_favorite_count, 2);

    let removed = app
        .service
        .remove_credential(first.id, &app.credentials)
        .unwrap();
    assert!(removed.is_shared);
    let first_view = app.service.get(first.id).unwrap();
    assert_eq!(first_view.credential_policy, CredentialPolicy::Ask);
    assert!(!first_view.has_saved_credential);

    assert_eq!(
        desktop_code(
            &app.service
                .delete_credential(ref_id, false, &app.credentials)
                .unwrap_err()
        ),
        "conflict"
    );
    assert_eq!(
        desktop_code(
            &app.service
                .delete_credential(ref_id, true, &app.credentials)
                .unwrap_err()
        ),
        "conflict"
    );

    app.service
        .remove_credential(second.id, &app.credentials)
        .unwrap();
    app.service
        .delete_credential(ref_id, true, &app.credentials)
        .unwrap();
    assert!(app.service.reusable().unwrap().is_empty());
}

#[test]
fn favorites_update_requires_a_new_credential_when_identity_changes() {
    let app = TestApp::new();
    let favorite = app
        .create(
            favorite_input("alpha"),
            CredentialPolicy::Save,
            None,
            Some("old-secret"),
        )
        .unwrap();

    let mut host_changed = favorite_input("alpha");
    host_changed.host = "moved.example".to_owned();
    assert_eq!(
        desktop_code(
            &app.service
                .update(
                    favorite.id,
                    host_changed.clone(),
                    CredentialPolicy::Save,
                    None,
                    None,
                    &app.credentials,
                )
                .unwrap_err()
        ),
        "credential_missing"
    );

    let updated = app
        .service
        .update(
            favorite.id,
            host_changed,
            CredentialPolicy::Save,
            None,
            Some(Secret::new(b"new-secret".to_vec())),
            &app.credentials,
        )
        .unwrap();
    assert_eq!(updated.host, "moved.example");
    assert_eq!(app.state().credential_refs.len(), 1);

    let cleared = app
        .service
        .update(
            favorite.id,
            favorite_input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &app.credentials,
        )
        .unwrap();
    assert_eq!(cleared.credential_policy, CredentialPolicy::Ask);
    assert!(!cleared.has_saved_credential);
}

#[tokio::test]
async fn favorites_test_connection_never_persists_or_updates_last_used() {
    let app = TestApp::new();
    let favorite = app
        .create(favorite_input("alpha"), CredentialPolicy::Ask, None, None)
        .unwrap();
    let (profile, _) = app
        .service
        .connection_target(favorite.id, &app.credentials)
        .unwrap();
    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    connections
        .test(profile.id, Some(Secret::new(b"session".to_vec())))
        .await
        .unwrap();

    let state = app.state();
    assert!(state.credential_refs.is_empty());
    assert!(state.server_favorites[0].last_used_at.is_none());
    assert_eq!(state.server_favorites.len(), 1);
}

#[tokio::test]
async fn favorites_test_connection_maps_certificate_failures() {
    let app = TestApp::new();
    let mut input = favorite_input("bad-certificate");
    input.protocol = Protocol::FtpsImplicit;
    input.host = "certificate.invalid".to_owned();
    let favorite = app
        .create(input, CredentialPolicy::Ask, None, None)
        .unwrap();
    let (profile, _) = app
        .service
        .connection_target(favorite.id, &app.credentials)
        .unwrap();
    let mut connections = ConnectionService::new(Box::new(FakeFactory));
    connections.load_profiles(vec![profile.clone()]);
    let error = connections
        .test(profile.id, Some(Secret::new(b"secret".to_vec())))
        .await
        .unwrap_err();
    assert_eq!(desktop_code(&error), "certificate_invalid");
    assert!(app.service.get(favorite.id).unwrap().last_used_at.is_none());
}

#[test]
fn ssh_private_key_profiles_require_an_explicit_reference() {
    let app = TestApp::new();
    let mut input = favorite_input("ssh");
    input.auth_method = AuthMethod::SshPrivateKey;
    input.ssh_key_ref = None;
    assert_eq!(
        desktop_code(
            &app.create(input, CredentialPolicy::Ask, None, None)
                .unwrap_err()
        ),
        "validation"
    );
}

#[test]
fn unrelated_state_survives_server_favorite_deletion() {
    let app = TestApp::new();
    let favorite = app
        .create(favorite_input("alpha"), CredentialPolicy::Ask, None, None)
        .unwrap();
    let mut state = app.state();
    state.preference.theme = thriftp_core::model::Theme::Dark;
    app.save_state(&state);
    app.service.delete(favorite.id).unwrap();
    assert_eq!(
        app.state().preference.theme,
        thriftp_core::model::Theme::Dark
    );
}

#[test]
fn credential_profiles_are_bound_to_an_independent_uuid() {
    let app = TestApp::new();
    let favorite = app
        .create(
            favorite_input("alpha"),
            CredentialPolicy::Save,
            None,
            Some("secret"),
        )
        .unwrap();
    let state = app.state();
    let reference = &state.credential_refs[0];
    assert_ne!(reference.id, favorite.connection_profile_id);
    assert_eq!(reference.host, "example.test");
    assert_eq!(reference.username, "tester");
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == favorite.connection_profile_id)
        .unwrap();
    assert!(reference.matches_profile(profile));
    let mut moved = profile.clone();
    moved.host = "moved.example".to_owned();
    assert!(!reference.matches_profile(&moved));
    assert_ne!(
        thriftp_core::model::CredentialRef::for_profile(&moved).id,
        reference.id
    );
}

#[test]
fn favorite_and_credential_result_views_match_the_ipc_contract() {
    use thriftp_desktop_lib::views::{
        ActiveProfileView, CredentialRemoveView, FavoriteDeleteView, RenameGroupView,
    };

    let impact = thriftp_core::server_favorites::CredentialImpact {
        credential_ref_id: uuid::Uuid::new_v4(),
        affected_favorites: Vec::new(),
        is_shared: false,
    };
    let deleted = FavoriteDeleteView {
        deleted: true,
        orphaned_credential_ref_id: Some(impact.credential_ref_id.to_string()),
        credential_impact: Some(impact.clone()),
    };
    let text = serde_json::to_string(&deleted).unwrap();
    assert!(text.contains("\"deleted\":true"));
    assert!(text.contains("\"orphanedCredentialRefId\""));
    assert!(text.contains("\"credentialImpact\""));

    let removed = CredentialRemoveView {
        removed: true,
        impact: impact.clone(),
    };
    let text = serde_json::to_string(&removed).unwrap();
    assert!(text.contains("\"removed\":true"));
    assert!(text.contains("\"impact\""));

    let renamed = RenameGroupView {
        updated: Vec::new(),
    };
    assert_eq!(
        serde_json::to_value(&renamed).unwrap(),
        serde_json::json!({"updated": []})
    );

    let profile = thriftp_core::model::ConnectionProfile::new(
        "active",
        Protocol::Sftp,
        "active.example",
        22,
        "tester",
        AuthMethod::Password,
        30,
    );
    let active = ActiveProfileView {
        connection_id: uuid::Uuid::new_v4().to_string(),
        favorite_id: None,
        profile,
    };
    let text = serde_json::to_string(&active).unwrap();
    assert!(text.contains("\"connectionId\""));
    assert!(text.contains("\"profile\""));
    assert!(!text.contains("favoriteId"));
}
