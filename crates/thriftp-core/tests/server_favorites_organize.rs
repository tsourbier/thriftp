mod support;

use std::sync::Arc;

use support::MemoryCredentialStore;
use thriftp_core::credentials::CredentialService;
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
        host: format!("{name}.example"),
        port: 22,
        username: "tester".to_owned(),
        auth_method: AuthMethod::Password,
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: false,
    }
}

#[test]
fn pinning_grouping_and_preferences_control_order() {
    let service = ServerFavoriteService::new(Arc::new(MemoryStore::default()));
    let credentials = CredentialService::new(Box::new(MemoryCredentialStore::available()));
    let alpha = service
        .create(
            input("alpha"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    let beta = service
        .create(
            input("beta"),
            CredentialPolicy::Ask,
            None,
            None,
            &credentials,
        )
        .unwrap();
    service.set_pinned(beta.id, true).unwrap();
    service
        .set_group(beta.id, Some("Production".to_owned()))
        .unwrap();
    service.rename_group("Production", "Prod").unwrap();
    service
        .set_preferences(FavoritePreferences {
            sort_key: FavoriteSortKey::Host,
            pinned_first: true,
        })
        .unwrap();

    let list = service.list().unwrap();
    assert_eq!(list[0].display_name, "beta");
    assert_eq!(list[0].group_label.as_deref(), Some("Prod"));
    assert_eq!(list[1].display_name, "alpha");
    assert_eq!(
        service.preferences().unwrap().sort_key,
        FavoriteSortKey::Host
    );
    let _ = alpha;
}
