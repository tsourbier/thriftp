mod support;

use support::{
    TestApp, assert_no_secret, assert_redacted, decode_favorite_payload, desktop_code,
    favorite_input, favorite_payload,
};
use thriftp_core::error::CoreError;
use thriftp_core::model::{CredentialPolicy, Protocol};

#[test]
fn favorites_create_accepts_camel_case_payload_and_returns_a_secret_free_view() {
    let app = TestApp::new();
    let input = decode_favorite_payload(favorite_payload("Alpha"));
    let view = app
        .create(input, CredentialPolicy::Save, None, Some("create-sentinel"))
        .unwrap();

    assert_eq!(view.display_name, "Alpha");
    assert!(view.has_saved_credential);
    let text = assert_no_secret(&view, "create-sentinel");
    assert!(text.contains("\"displayName\":\"Alpha\""));
    assert!(text.contains("\"credentialPolicy\":\"save\""));
    assert!(text.contains("\"hasSavedCredential\":true"));

    let persisted = app.state();
    assert_eq!(persisted.server_favorites.len(), 1);
    assert_eq!(persisted.connection_profiles.len(), 1);
    assert_eq!(persisted.credential_refs.len(), 1);
}

#[test]
fn favorites_create_and_update_persist_remote_paths() {
    let app = TestApp::new();
    let mut input = favorite_input("with-path");
    input.remote_path = Some("/srv/www".to_owned());
    let created = app.create(input, CredentialPolicy::Ask, None, None).unwrap();
    assert_eq!(created.remote_path.as_deref(), Some("/srv/www"));

    let mut updated = favorite_input("with-path");
    updated.remote_path = Some("/srv/app".to_owned());
    app.service
        .update(
            created.id,
            updated,
            CredentialPolicy::Ask,
            None,
            None,
            app.credentials(),
        )
        .unwrap();
    assert_eq!(
        app.service.get(created.id).unwrap().remote_path.as_deref(),
        Some("/srv/app")
    );
}

#[test]
fn favorites_create_rejects_invalid_profiles_with_a_stable_validation_code() {
    let app = TestApp::new();
    let mut input = favorite_input("bad");
    input.host.clear();
    let error = app
        .create(input, CredentialPolicy::Ask, None, None)
        .unwrap_err();
    let desktop = thriftp_desktop_lib::error::DesktopError::from(error);
    assert_eq!(desktop.code, "validation");
    assert_eq!(desktop.field.as_deref(), Some("host"));
}

#[test]
fn favorites_create_requires_a_secret_or_reference_for_saved_credentials() {
    let app = TestApp::new();
    let error = app
        .create(
            favorite_input("missing"),
            CredentialPolicy::Save,
            None,
            None,
        )
        .unwrap_err();
    assert_eq!(desktop_code(&error), "credential_missing");
    assert!(app.state().server_favorites.is_empty());
}

#[test]
fn favorites_create_never_persists_a_plaintext_secret() {
    let app = TestApp::new();
    app.create(
        favorite_input("secure"),
        CredentialPolicy::Save,
        None,
        Some("plaintext-sentinel"),
    )
    .unwrap();

    let persisted = std::fs::read_to_string(app.metadata_path()).unwrap();
    assert!(!persisted.contains("plaintext-sentinel"));
    assert!(persisted.contains("credentialRefId"));
}

#[test]
fn connections_get_profile_returns_a_redacted_camel_case_profile() {
    let app = TestApp::new();
    let view = app
        .create(favorite_input("profile"), CredentialPolicy::Ask, None, None)
        .unwrap();
    let state = app.store.load().unwrap();
    let profile = state
        .connection_profiles
        .iter()
        .find(|profile| profile.id == view.connection_profile_id)
        .cloned()
        .unwrap();

    assert_eq!(profile.protocol, Protocol::Sftp);
    assert_eq!(profile.display_name, "profile");
    let text = serde_json::to_string(&profile).unwrap();
    assert!(text.contains("\"displayName\":\"profile\""));
    assert!(text.contains("\"authMethod\":\"password\""));
    assert!(!text.contains("password\":\""));
}

#[test]
fn commands_redact_internal_diagnostics() {
    let error = CoreError::internal("backend leaked socket 10.0.0.7");
    let desktop = thriftp_desktop_lib::error::DesktopError::from(error);
    assert_redacted(&desktop, "10.0.0.7");
    assert_eq!(desktop.code, "internal");
    assert_eq!(desktop.message, "An unexpected error occurred.");
}

#[test]
fn favorite_views_keep_timeout_and_ssh_key_reference() {
    use thriftp_core::model::AuthMethod;

    let app = TestApp::new();
    let mut input = favorite_input("ssh");
    input.auth_method = AuthMethod::SshPrivateKey;
    input.ssh_key_ref = Some("~/.ssh/id_ed25519".to_owned());
    input.timeout_seconds = 45;
    let view = app
        .create(input, CredentialPolicy::Ask, None, None)
        .unwrap();

    assert_eq!(view.timeout_seconds, 45);
    assert_eq!(view.ssh_key_ref.as_deref(), Some("~/.ssh/id_ed25519"));
    let text = serde_json::to_string(&view).unwrap();
    assert!(text.contains("\"timeoutSeconds\":45"));
    assert!(text.contains("\"sshKeyRef\":\"~/.ssh/id_ed25519\""));
}
