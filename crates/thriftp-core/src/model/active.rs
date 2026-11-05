use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profile::Protocol;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveConnection {
    pub id: Uuid,
    pub connection_profile_id: Uuid,
    pub protocol: Protocol,
    pub state: String,
    pub started_at: DateTime<Utc>,
    pub error: Option<String>,
}

impl ActiveConnection {
    pub fn new(connection_profile_id: Uuid, protocol: Protocol) -> Self {
        Self {
            id: Uuid::new_v4(),
            connection_profile_id,
            protocol,
            state: "connecting".to_owned(),
            started_at: Utc::now(),
            error: None,
        }
    }
}
