use tempfile::TempDir;
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, CredentialPolicy, FavoriteSortKey, Protocol, ServerFavorite,
};
use thriftp_core::repository::{AppState, MetadataStore};
use thriftp_storage_json::JsonMetadataStore;

#[test]
fn server_favorites_and_preferences_round_trip() {
    let home = TempDir::new().unwrap();
    let store = JsonMetadataStore::new(home.path().join("metadata.json"));
    let profile = ConnectionProfile::new(
        "example",
        Protocol::Sftp,
        "example.test",
        22,
        "tester",
        AuthMethod::Password,
        30,
    );
    let favorite = ServerFavorite::new(profile.id, CredentialPolicy::Ask, None);
    let mut state = AppState::default();
    state.connection_profiles.push(profile);
    state.server_favorites.push(favorite);
    state.favorite_preferences.sort_key = FavoriteSortKey::Host;
    store.save(&state).unwrap();

    let loaded = store.load().unwrap();
    assert_eq!(loaded.server_favorites.len(), 1);
    assert_eq!(loaded.favorite_preferences.sort_key, FavoriteSortKey::Host);
}

#[test]
fn corrupt_document_returns_an_error_instead_of_panicking() {
    use std::fs;

    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    fs::write(&path, b"{not valid json").unwrap();
    let store = JsonMetadataStore::new(path);
    assert!(store.load().is_err());
}

#[test]
fn corrupt_primary_document_recovers_from_the_previous_valid_backup() {
    use std::fs;

    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut first = AppState::default();
    first.preference.theme = thriftp_core::model::Theme::Dark;
    store.save(&first).unwrap();
    let mut second = AppState::default();
    second.preference.theme = thriftp_core::model::Theme::Light;
    store.save(&second).unwrap();

    fs::write(&path, b"{corrupt").unwrap();
    let recovered = store.load().unwrap();
    assert_eq!(recovered.preference.theme, thriftp_core::model::Theme::Dark);
}

#[test]
fn schema_version_zero_is_migrated() {
    use std::fs;

    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    let mut document = serde_json::to_value(AppState::default()).unwrap();
    document["schemaVersion"] = serde_json::json!(0);
    fs::write(&path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    assert!(store.load().is_ok());
}

#[cfg(unix)]
#[test]
fn metadata_documents_are_written_with_owner_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let home = TempDir::new().unwrap();
    let path = home.path().join("metadata.json");
    let store = JsonMetadataStore::new(path.clone());
    store.save(&AppState::default()).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
