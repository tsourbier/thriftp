use chrono::Utc;
use thriftp_core::model::{ConnectionHistoryEntry, HubTab, Protocol, WorkspaceLayout};
use thriftp_core::repository::AppState;
use uuid::Uuid;

#[test]
fn workspace_and_history_are_non_secret() {
    let state = AppState {
        workspace_layout: WorkspaceLayout {
            split_ratio: 0.5,
            sidebar_visible: true,
            sidebar_sections: Default::default(),
            hub_tab: HubTab::History,
        },
        connection_history: vec![ConnectionHistoryEntry {
            favorite_id: Some(Uuid::new_v4()),
            connection_profile_id: Uuid::new_v4(),
            display_name: "Production".to_owned(),
            protocol: Protocol::Sftp,
            host: "example.test".to_owned(),
            port: 22,
            username: "deploy".to_owned(),
            last_used_at: Utc::now(),
        }],
        ..AppState::default()
    };

    let json = serde_json::to_string(&state).expect("serialize");
    for forbidden in [
        "password",
        "passphrase",
        "PRIVATE KEY",
        "secret",
        "privateKey",
    ] {
        assert!(
            !json.contains(forbidden),
            "workspace metadata must not contain {forbidden}"
        );
    }
}
