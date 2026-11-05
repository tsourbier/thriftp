//! Non-secret connection history records.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::Protocol;

/// A recent successful connection snapshot.
///
/// History intentionally contains no credentials, passphrases, key material,
/// file data, or failure details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionHistoryEntry {
    /// Favorite used for the connection, when applicable.
    #[serde(default)]
    pub favorite_id: Option<Uuid>,
    /// Connection profile used for the connection.
    pub connection_profile_id: Uuid,
    /// Display name snapshot.
    pub display_name: String,
    /// Protocol snapshot.
    pub protocol: Protocol,
    /// Host snapshot.
    pub host: String,
    /// Port snapshot.
    pub port: u16,
    /// Username snapshot.
    pub username: String,
    /// Time of the most recent successful connection.
    pub last_used_at: DateTime<Utc>,
}
