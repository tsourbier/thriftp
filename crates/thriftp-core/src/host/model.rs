//! Transfer-host metadata and shared state types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role a process plays relative to the transfer host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostRole {
    /// This process owns the queue and control endpoint.
    Host,
    /// This process delegates to another host.
    Client,
    /// This process only observes and never writes transfer state.
    Observer,
}

impl HostRole {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Host => "host",
            Self::Client => "client",
            Self::Observer => "observer",
        }
    }
}

/// Published metadata for the current host instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub instance_id: Uuid,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub version: String,
    pub state: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub transport: String,
    #[serde(default)]
    pub token_hash: String,
    #[serde(default)]
    pub owned_transfer_ids: Vec<Uuid>,
}

/// Snapshot reported by `host.status`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostSnapshot {
    pub instance_id: Uuid,
    pub pid: u32,
    pub started_at: DateTime<Utc>,
    pub version: String,
    pub mode: String,
    pub role: String,
    pub owned_transfer_count: usize,
}
