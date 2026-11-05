//! Cross-process disconnect requests for MCP clients.
//!
//! The CLI and desktop cannot directly terminate sessions held by a separate
//! MCP process. They append a request to a small persisted queue which the MCP
//! server drains before its next authorization decision.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectRequest {
    pub fingerprint: String,
    pub requested_at: chrono::DateTime<chrono::Utc>,
    pub request_id: Uuid,
}

/// Path of the pending disconnect queue.
#[must_use]
pub fn default_path() -> PathBuf {
    crate::automation::data_dir()
        .join("run")
        .join("disconnects.json")
}

/// Queue a disconnect request for a client fingerprint.
pub fn request(fingerprint: &str) -> CoreResult<DisconnectRequest> {
    request_at(&default_path(), fingerprint)
}

/// Queue a disconnect request in an explicit file.
pub fn request_at(path: &std::path::Path, fingerprint: &str) -> CoreResult<DisconnectRequest> {
    let request = DisconnectRequest {
        fingerprint: fingerprint.to_owned(),
        requested_at: chrono::Utc::now(),
        request_id: Uuid::new_v4(),
    };
    let mut requests = load(path)?;
    requests.push(request.clone());
    save(path, &requests)?;
    Ok(request)
}

/// Drain all pending disconnect requests.
pub fn take_requests() -> CoreResult<Vec<DisconnectRequest>> {
    take_requests_at(&default_path())
}

/// Drain pending requests from an explicit file.
pub fn take_requests_at(path: &std::path::Path) -> CoreResult<Vec<DisconnectRequest>> {
    let requests = load(path)?;
    if path.is_file() {
        std::fs::remove_file(path).map_err(|error| {
            CoreError::internal(format!("cannot clear disconnect requests: {error}"))
        })?;
    }
    Ok(requests)
}

fn load(path: &std::path::Path) -> CoreResult<Vec<DisconnectRequest>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)
        .map_err(|error| CoreError::internal(format!("disconnect read: {error}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::internal(format!("disconnect parse: {error}")))
}

fn save(path: &std::path::Path, requests: &[DisconnectRequest]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| CoreError::internal(format!("disconnect directory: {error}")))?;
    }
    let bytes = serde_json::to_vec_pretty(requests)
        .map_err(|error| CoreError::internal(format!("disconnect serialize: {error}")))?;
    std::fs::write(path, bytes)
        .map_err(|error| CoreError::internal(format!("disconnect write: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_queued_and_drained() {
        let home = tempfile::TempDir::new().unwrap();
        let path = home.path().join("disconnects.json");
        request_at(&path, "fingerprint-a").unwrap();
        request_at(&path, "fingerprint-b").unwrap();
        let requests = take_requests_at(&path).unwrap();
        assert_eq!(requests.len(), 2);
        assert!(take_requests_at(&path).unwrap().is_empty());
    }
}
