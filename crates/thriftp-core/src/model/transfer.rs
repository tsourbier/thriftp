use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferState {
    Pending,
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Priority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: Uuid,
    pub direction: TransferDirection,
    pub connection_profile_id: Uuid,
    pub source_path: String,
    pub destination_path: String,
    pub total_bytes: Option<u64>,
    pub transferred_bytes: u64,
    pub state: TransferState,
    pub priority: Priority,
    pub order: i64,
    pub speed_bytes_per_second: Option<u64>,
    pub estimated_remaining_seconds: Option<u64>,
    pub resumable: bool,
    pub partial_data_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl Transfer {
    pub fn new(
        direction: TransferDirection,
        connection_profile_id: Uuid,
        source_path: impl Into<String>,
        destination_path: impl Into<String>,
        total_bytes: Option<u64>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            direction,
            connection_profile_id,
            source_path: source_path.into(),
            destination_path: destination_path.into(),
            total_bytes,
            transferred_bytes: 0,
            state: TransferState::Pending,
            priority: Priority::Normal,
            order: 0,
            speed_bytes_per_second: None,
            estimated_remaining_seconds: None,
            resumable: false,
            partial_data_id: None,
            error: None,
            created_at: now,
            updated_at: now,
            started_at: None,
            completed_at: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialTransferData {
    pub id: Uuid,
    pub transfer_id: Uuid,
    pub local_path: Option<String>,
    pub remote_path: Option<String>,
    pub offset: u64,
    pub source_size: Option<u64>,
    pub source_modified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
