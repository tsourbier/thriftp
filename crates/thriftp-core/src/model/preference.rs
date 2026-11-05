use serde::{Deserialize, Serialize};

use super::browsing::SortKey;

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preference {
    pub theme: Theme,
    pub show_hidden_files: bool,
    pub default_concurrency: usize,
    pub bandwidth_limit_bytes_per_second: Option<u64>,
    pub restore_tabs: bool,
    pub preview_mode: PreviewMode,
    pub sort_key: SortKey,
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
            reduced_motion: false,
        }
    }
}
