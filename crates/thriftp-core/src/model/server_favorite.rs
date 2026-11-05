use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CredentialPolicy {
    Save,
    Ask,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FavoriteSortKey {
    Name,
    Protocol,
    Host,
    LastUsed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritePreferences {
    pub sort_key: FavoriteSortKey,
    pub pinned_first: bool,
}

impl Default for FavoritePreferences {
    fn default() -> Self {
        Self {
            sort_key: FavoriteSortKey::Name,
            pinned_first: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerFavorite {
    pub id: Uuid,
    pub connection_profile_id: Uuid,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
    pub credential_policy: CredentialPolicy,
    pub credential_ref_id: Option<Uuid>,
    /// Optional absolute local folder to restore on connect.
    #[serde(default)]
    pub local_path: Option<String>,
    /// Optional absolute remote folder to restore on connect.
    #[serde(default)]
    pub remote_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ServerFavorite {
    pub fn new(
        connection_profile_id: Uuid,
        credential_policy: CredentialPolicy,
        credential_ref_id: Option<Uuid>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            connection_profile_id,
            pinned: false,
            last_used_at: None,
            credential_policy,
            credential_ref_id,
            local_path: None,
            remote_path: None,
            created_at: now,
            updated_at: now,
        }
    }
}
