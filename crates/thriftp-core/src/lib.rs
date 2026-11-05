//! Tauri-independent domain and application logic for Thriftp.
//!
//! This crate must build and test without Tauri or any desktop framework.

pub mod automation;
pub mod browser;
pub mod connections;
pub mod credentials;
pub mod error;
pub mod favorites;
pub mod history;
pub mod host;
pub mod model;
pub mod navigation;
pub mod observability;
pub mod preferences;
pub mod previews;
pub mod protocol;
pub mod queue;
pub mod recovery;
pub mod repository;
pub mod server_favorites;
pub mod sorting;
pub mod ssh_keys;
pub mod transfers;
pub mod validation;
pub mod workspace;

pub use error::{CoreError, CoreResult};
