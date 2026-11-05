//! Tauri command adapters.
//!
//! Commands stay thin and delegate to the Tauri-independent core services.

pub mod automation;
pub mod browser;
pub mod connections;
pub mod favorites;
pub mod history;
pub mod inspector;
pub mod preferences;
pub mod previews;
pub mod server_favorites;
pub mod ssh_keys;
pub mod tabs;
pub mod transfers;
pub mod workspace;
