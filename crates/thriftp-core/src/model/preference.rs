use serde::{Deserialize, Serialize};

use super::browsing::{SortDirection, SortKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewMode {
    Auto,
    Native,
    Internal,
}

/// User preferences persisted in the versioned metadata document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preference {
    /// Active color theme.
    pub theme: Theme,
    /// Whether new listings include hidden entries by default.
    pub show_hidden_files: bool,
    /// Default number of concurrent transfers.
    pub default_concurrency: usize,
    /// Optional global bandwidth ceiling in bytes per second.
    pub bandwidth_limit_bytes_per_second: Option<u64>,
    /// Whether tabs are restored on the next launch.
    pub restore_tabs: bool,
    /// Preferred preview surface for supported files.
    pub preview_mode: PreviewMode,
    /// Default sort column for new tabs.
    pub sort_key: SortKey,
    /// Default sort direction for new tabs; missing values deserialize as ascending.
    #[serde(default)]
    pub sort_direction: SortDirection,
    /// Whether the interface minimizes animation.
    pub reduced_motion: bool,
}

impl Default for Preference {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            show_hidden_files: false,
            default_concurrency: 4,
            bandwidth_limit_bytes_per_second: None,
            restore_tabs: true,
            preview_mode: PreviewMode::Auto,
            sort_key: SortKey::Name,
            sort_direction: SortDirection::Asc,
            reduced_motion: false,
        }
    }
}
