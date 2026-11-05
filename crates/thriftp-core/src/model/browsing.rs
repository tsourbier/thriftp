use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortKey {
    Name,
    Size,
    Date,
    Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewSettings {
    pub sort_key: SortKey,
    pub show_hidden: bool,
    pub filter: Option<String>,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            sort_key: SortKey::Name,
            show_hidden: false,
            filter: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowsingSession {
    pub id: Uuid,
    pub connection_profile_id: Option<Uuid>,
    pub current_path: String,
    #[serde(default)]
    pub local_path: Option<String>,
    pub history: Vec<String>,
    pub history_index: usize,
    pub view_settings: ViewSettings,
    pub selected_items: Vec<String>,
    #[serde(default)]
    pub local_selected_items: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub restored: bool,
}

impl BrowsingSession {
    pub fn new(connection_profile_id: Option<Uuid>, path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            id: Uuid::new_v4(),
            connection_profile_id,
            current_path: path.clone(),
            local_path: Some(".".to_owned()),
            history: vec![path],
            history_index: 0,
            view_settings: ViewSettings::default(),
            selected_items: Vec::new(),
            local_selected_items: Vec::new(),
            created_at: Utc::now(),
            restored: false,
        }
    }
}
