pub mod commands;
pub mod error;
pub mod lifecycle;
pub mod views;

use std::collections::HashMap;
use std::sync::Arc;

use tauri::{Emitter, Manager};
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::CredentialService;
use thriftp_core::host::HostClient;
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;

use commands::browser::BrowserListings;
use commands::transfers::TransferManager;
use thriftp_core::favorites::DirectoryFavoriteService;
use thriftp_core::preferences::PreferencesService;
use thriftp_core::preferences::tabs::TabService;
use thriftp_core::repository::{MetadataStore, TransferStore};
use thriftp_core::server_favorites::ServerFavoriteService;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};

/// Shared Tauri-managed state for the desktop application.
pub struct AppState {
    /// Active connection service and profiles.
    pub connections: Arc<tokio::sync::Mutex<ConnectionService>>,
    /// Native credential-store adapter.
    pub credentials: CredentialService,
    /// Browser listing registry.
    pub listings: Arc<BrowserListings>,
    /// Transfer manager state.
    pub transfers: Arc<tokio::sync::Mutex<TransferManager>>,
    /// Directory-favorites service.
    pub favorites: tokio::sync::Mutex<DirectoryFavoriteService>,
    /// Restored tab service.
    pub tabs: tokio::sync::Mutex<TabService>,
    /// Application preferences service.
    pub preferences: tokio::sync::Mutex<PreferencesService>,
    /// Temporary preview paths keyed by handle.
    pub previews: std::sync::Mutex<HashMap<String, std::path::PathBuf>>,
    /// Persistence boundary for non-secret metadata.
    pub metadata_store: Arc<dyn MetadataStore>,
    /// Server-favorites application service.
    pub server_favorites: ServerFavoriteService,
    /// Embedded host/MCP automation runtime.
    pub automation: tokio::sync::Mutex<commands::automation::AutomationRuntime>,
    /// Optional shared transfer host.
    pub transfer_host: tokio::sync::Mutex<Option<thriftp_core::host::HostRuntime>>,
}

impl AppState {
    /// Persist profiles, favorites, tabs, and preference defaults.
    pub async fn persist_metadata(&self) -> thriftp_core::CoreResult<()> {
        let profiles = self.connections.lock().await.list_profiles();
        let favorites = self.favorites.lock().await.snapshot();
        let tabs = self.tabs.lock().await.list();
        let preference = self.preferences.lock().await.get();
        let existing = self.metadata_store.load().unwrap_or_default();
        let state = thriftp_core::repository::AppState {
            connection_profiles: profiles.clone(),
            directory_favorites: favorites,
            credential_refs: existing.credential_refs,
            preference,
            tabs,
            server_favorites: existing.server_favorites,
            favorite_preferences: existing.favorite_preferences,
            workspace_layout: existing.workspace_layout,
            connection_history: existing.connection_history,
        };
        self.metadata_store.save(&state)
    }
}

/// Build and run the desktop application.
pub fn run() {
    thriftp_core::observability::init_tracing();
    let _ = thriftp_core::automation::maintain_audit();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let metadata_store: Arc<dyn MetadataStore> =
                Arc::new(JsonMetadataStore::new(data_dir.join("metadata.json")));
            let transfer_store: Arc<dyn TransferStore> =
                Arc::new(JsonTransferStore::new(data_dir.join("transfers.json")));
            let metadata = metadata_store.load().unwrap_or_default();
            let mut persisted_transfers = transfer_store.load().unwrap_or_default();
            thriftp_core::recovery::recover_transfers(&mut persisted_transfers);

            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(metadata.connection_profiles.clone());
            let mut favorites = DirectoryFavoriteService::default();
            favorites.load(metadata.directory_favorites.clone());
            let mut tabs = TabService::default();
            tabs.save(metadata.tabs.clone());
            let mut preferences = PreferencesService::default();
            preferences.set(metadata.preference.clone());
            let mut transfers = TransferManager::new(
                metadata.preference.default_concurrency,
                Some(transfer_store),
            );
            transfers.load(
                persisted_transfers.queue,
                persisted_transfers.transfers,
                persisted_transfers.partial_transfers,
                persisted_transfers.history,
            );
            transfers
                .queue
                .set_concurrency(metadata.preference.default_concurrency);
            transfers.set_bandwidth(metadata.preference.bandwidth_limit_bytes_per_second);

            let transfer_host = thriftp_core::host::HostRuntime::acquire(
                "desktop",
                thriftp_core::host::HostServices {
                    metadata: Arc::new(JsonMetadataStore::new(data_dir.join("metadata.json"))),
                    transfers: Arc::new(JsonTransferStore::new(data_dir.join("transfers.json"))),
                    credentials: Arc::new(KeyringCredentialStore::new())
                        as Arc<dyn thriftp_core::credentials::CredentialStore>,
                    factory: Arc::new(ProtocolFactory),
                },
            )
            .ok();

            if let Some(runtime) = transfer_host.as_ref()
                && !runtime.is_host()
            {
                let run_dir = data_dir.join("run");
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    if let Ok(mut events) =
                        HostClient::subscribe_at(&run_dir, &["transfers", "queue"])
                    {
                        while let Ok(Some((method, params))) = events.next_event() {
                            match method.as_str() {
                                "event.transferProgress" => {
                                    let _ = handle.emit(
                                        "transfer:progress",
                                        serde_json::json!({
                                            "transferId": params.get("transferId"),
                                            "transferredBytes": params.get("confirmedBytes"),
                                            "totalBytes": params.get("totalBytes"),
                                            "speedBytesPerSecond": params.get("speedBytesPerSecond"),
                                            "estimatedRemainingSeconds": params.get("estimatedRemainingSeconds")
                                        }),
                                    );
                                }
                                "event.transferState" => {
                                    let _ = handle.emit(
                                        "transfer:state",
                                        serde_json::json!({
                                            "transferId": params.get("transferId"),
                                            "state": params.get("newState"),
                                            "error": params.get("errorCode")
                                        }),
                                    );
                                }
                                "event.queueChanged" => {
                                    let _ = handle.emit(
                                        "queue:state",
                                        params.get("queue").cloned().unwrap_or_default(),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                });
            }

            app.manage(AppState {
                connections: Arc::new(tokio::sync::Mutex::new(connections)),
                credentials: CredentialService::new(Box::new(KeyringCredentialStore::new())),
                listings: Arc::new(BrowserListings::default()),
                transfers: Arc::new(tokio::sync::Mutex::new(transfers)),
                favorites: tokio::sync::Mutex::new(favorites),
                tabs: tokio::sync::Mutex::new(tabs),
                preferences: tokio::sync::Mutex::new(preferences),
                previews: std::sync::Mutex::new(HashMap::new()),
                metadata_store: Arc::clone(&metadata_store),
                server_favorites: ServerFavoriteService::new(metadata_store),
                automation: tokio::sync::Mutex::new(
                    commands::automation::AutomationRuntime::default(),
                ),
                transfer_host: tokio::sync::Mutex::new(transfer_host),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::connections::connections_list,
            commands::connections::connections_create,
            commands::connections::connections_update,
            commands::connections::connections_get_profile,
            commands::connections::connections_delete,
            commands::connections::connections_connect,
            commands::connections::connections_trust_host_key,
            commands::connections::connections_reconnect,
            commands::connections::connections_disconnect,
            commands::connections::connections_test,
            commands::browser::browser_open_local,
            commands::browser::browser_open_remote,
            commands::browser::browser_refresh,
            commands::browser::browser_cancel_listing,
            commands::browser::browser_stat,
            commands::browser::browser_mkdir,
            commands::browser::browser_rename,
            commands::browser::browser_delete,
            commands::browser::browser_chmod,
            commands::browser::browser_resolve_path,
            commands::workspace::workspace_get,
            commands::workspace::workspace_set,
            commands::history::connection_history_list,
            commands::history::connection_history_clear,
            commands::ssh_keys::ssh_keys_list,
            commands::ssh_keys::ssh_keys_validate,
            commands::previews::preview_open,
            commands::previews::preview_read,
            commands::previews::preview_close,
            commands::transfers::transfers_start,
            commands::transfers::transfers_list,
            commands::transfers::transfers_get,
            commands::transfers::transfers_pause,
            commands::transfers::transfers_resume,
            commands::transfers::transfers_cancel,
            commands::transfers::transfers_restart,
            commands::transfers::transfers_discard_partial,
            commands::transfers::transfers_clear_history,
            commands::transfers::transfers_dismiss,
            commands::transfers::queue_get,
            commands::transfers::queue_pause,
            commands::transfers::queue_resume,
            commands::transfers::queue_set_concurrency,
            commands::transfers::queue_set_bandwidth,
            commands::transfers::queue_reorder,
            commands::transfers::queue_set_priority,
            commands::transfers::queue_cancel_all,
            lifecycle::lifecycle_prepare_quit,
            commands::favorites::directory_favorites_list,
            commands::favorites::directory_favorites_create,
            commands::favorites::directory_favorites_update,
            commands::favorites::directory_favorites_delete,
            commands::favorites::directory_favorites_reorder,
            commands::tabs::tabs_list,
            commands::tabs::tabs_save,
            commands::tabs::tabs_restore,
            commands::preferences::preferences_get,
            commands::preferences::preferences_set,
            commands::inspector::inspector_get,
            commands::automation::automation_status,
            commands::automation::automation_enable,
            commands::automation::automation_disable,
            commands::automation::automation_pending_requests,
            commands::automation::automation_approve,
            commands::automation::automation_deny,
            commands::automation::automation_revoke,
            commands::automation::automation_disconnect,
            commands::automation::automation_issue_confirmation,
            commands::automation::automation_audit_list,
            commands::automation::automation_audit_verify,
            commands::automation::automation_retention,
            commands::automation::automation_limit,
            commands::automation::automation_apply_retention,
            commands::automation::automation_host_status,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Thriftp desktop application");
}
