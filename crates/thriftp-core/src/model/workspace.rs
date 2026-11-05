//! Workspace presentation state shared by the desktop shell and future clients.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize};

/// Disconnected right-pane tab.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HubTab {
    /// Saved-server favorites.
    #[default]
    Favorites,
    /// Recent successful connections.
    History,
}

impl<'de> Deserialize<'de> for HubTab {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Ok(match value.as_str() {
            "history" => Self::History,
            _ => Self::Favorites,
        })
    }
}

/// Known sidebar section keys.
pub const SIDEBAR_SECTION_KEYS: [&str; 3] = ["servers", "directoryFavorites", "automation"];

/// Persisted workspace presentation preferences.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceLayout {
    /// Left pane share of the split, clamped to `0.20..=0.80`.
    #[serde(default = "default_split_ratio")]
    pub split_ratio: f64,
    /// User preference for the sidebar visibility.
    #[serde(default = "default_true")]
    pub sidebar_visible: bool,
    /// Visibility of each known sidebar section.
    #[serde(default = "default_sidebar_sections")]
    pub sidebar_sections: BTreeMap<String, bool>,
    /// Last selected disconnected-hub tab.
    #[serde(default)]
    pub hub_tab: HubTab,
}

fn default_split_ratio() -> f64 {
    0.5
}

fn default_true() -> bool {
    true
}

fn default_sidebar_sections() -> BTreeMap<String, bool> {
    SIDEBAR_SECTION_KEYS
        .iter()
        .map(|key| ((*key).to_owned(), true))
        .collect()
}

impl Default for WorkspaceLayout {
    fn default() -> Self {
        Self {
            split_ratio: default_split_ratio(),
            sidebar_visible: true,
            sidebar_sections: default_sidebar_sections(),
            hub_tab: HubTab::Favorites,
        }
    }
}

impl WorkspaceLayout {
    /// Return a normalized layout that can safely be persisted or returned to the UI.
    #[must_use]
    pub fn normalized(self) -> Self {
        let split_ratio = if self.split_ratio.is_finite() {
            self.split_ratio.clamp(0.20, 0.80)
        } else {
            default_split_ratio()
        };
        let sidebar_sections = SIDEBAR_SECTION_KEYS
            .iter()
            .map(|key| {
                (
                    (*key).to_owned(),
                    self.sidebar_sections.get(*key).copied().unwrap_or(true),
                )
            })
            .collect();
        Self {
            split_ratio,
            sidebar_visible: self.sidebar_visible,
            sidebar_sections,
            hub_tab: self.hub_tab,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{HubTab, WorkspaceLayout};
    use std::collections::BTreeMap;

    #[test]
    fn defaults_are_complete() {
        let layout = WorkspaceLayout::default();
        assert_eq!(layout.split_ratio, 0.5);
        assert!(layout.sidebar_visible);
        assert_eq!(layout.sidebar_sections.len(), 3);
        assert!(layout.sidebar_sections.values().all(|value| *value));
        assert_eq!(layout.hub_tab, HubTab::Favorites);
    }

    #[test]
    fn normalization_clamps_and_fills_sections() {
        let mut sections = BTreeMap::new();
        sections.insert("servers".to_owned(), false);
        sections.insert("unknown".to_owned(), false);
        let layout = WorkspaceLayout {
            split_ratio: 0.95,
            sidebar_visible: false,
            sidebar_sections: sections,
            hub_tab: HubTab::History,
        }
        .normalized();
        assert_eq!(layout.split_ratio, 0.80);
        assert!(!layout.sidebar_visible);
        assert_eq!(layout.sidebar_sections.len(), 3);
        assert_eq!(layout.sidebar_sections.get("servers"), Some(&false));
        assert_eq!(
            layout.sidebar_sections.get("directoryFavorites"),
            Some(&true)
        );
        assert!(!layout.sidebar_sections.contains_key("unknown"));
        assert_eq!(layout.hub_tab, HubTab::History);
    }

    #[test]
    fn non_finite_ratio_falls_back() {
        let layout = WorkspaceLayout {
            split_ratio: f64::NAN,
            ..WorkspaceLayout::default()
        }
        .normalized();
        assert_eq!(layout.split_ratio, 0.5);
    }

    #[test]
    fn unknown_hub_tab_deserializes_to_favorites() {
        let layout: WorkspaceLayout = serde_json::from_str(
            r#"{"splitRatio":0.4,"sidebarVisible":true,"sidebarSections":{},"hubTab":"future"}"#,
        )
        .expect("layout");
        assert_eq!(layout.hub_tab, HubTab::Favorites);
    }
}
