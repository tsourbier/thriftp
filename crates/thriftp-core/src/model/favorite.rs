use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryFavorite {
    pub id: Uuid,
    pub connection_profile_id: Uuid,
    pub path: String,
    pub label: String,
    pub sort_order: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DirectoryFavorite {
    pub fn new(
        connection_profile_id: Uuid,
        path: impl Into<String>,
        label: impl Into<String>,
        sort_order: i64,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            connection_profile_id,
            path: path.into(),
            label: label.into(),
            sort_order,
            created_at: now,
            updated_at: now,
        }
    }
}
