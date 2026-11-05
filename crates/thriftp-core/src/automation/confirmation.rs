//! Cross-process single-use confirmations for destructive MCP operations.
//!
//! The MCP server runs in its own process, so a user-run CLI or desktop
//! confirmation must be persisted where the server can consume it. Entries are
//! single-use, expiring, and bound to the grant, session, operation, and target
//! where supplied.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

/// One persisted confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredConfirmation {
    pub id: Uuid,
    pub grant_id: Uuid,
    pub session_id: Option<String>,
    pub operation: String,
    pub target: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// Default confirmation store path.
#[must_use]
pub fn default_path() -> PathBuf {
    crate::automation::data_dir()
        .join("run")
        .join("confirmations.json")
}

/// Issue a confirmation for a grant, operation, and target.
pub fn issue(
    grant_id: Uuid,
    operation: &str,
    target: Option<&str>,
    ttl_seconds: i64,
) -> CoreResult<StoredConfirmation> {
    issue_at(&default_path(), grant_id, operation, target, ttl_seconds)
}

/// Issue a confirmation into an explicit store file.
pub fn issue_at(
    path: &Path,
    grant_id: Uuid,
    operation: &str,
    target: Option<&str>,
    ttl_seconds: i64,
) -> CoreResult<StoredConfirmation> {
    if ttl_seconds <= 0 {
        return Err(CoreError::invalid_argument(
            "confirmation TTL must be positive",
        ));
    }
    let now = Utc::now();
    let confirmation = StoredConfirmation {
        id: Uuid::new_v4(),
        grant_id,
        session_id: None,
        operation: operation.to_owned(),
        target: target.map(str::to_owned),
        created_at: now,
        expires_at: now + Duration::seconds(ttl_seconds),
        used_at: None,
    };
    let mut entries = load(path)?;
    entries.push(confirmation.clone());
    save(path, &entries)?;
    Ok(confirmation)
}

/// Consume a matching confirmation exactly once.
pub fn consume(
    id: Uuid,
    grant_id: Uuid,
    session_id: Option<&str>,
    operation: &str,
    target: Option<&str>,
) -> CoreResult<()> {
    consume_at(
        &default_path(),
        id,
        grant_id,
        session_id,
        operation,
        target,
        Utc::now(),
    )
}

/// Consume a confirmation from an explicit store file.
#[allow(clippy::too_many_arguments)]
pub fn consume_at(
    path: &Path,
    id: Uuid,
    grant_id: Uuid,
    session_id: Option<&str>,
    operation: &str,
    target: Option<&str>,
    now: DateTime<Utc>,
) -> CoreResult<()> {
    let mut entries = load(path)?;
    let entry = entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or_else(|| CoreError::NotFound("confirmation".to_owned()))?;
    if entry.grant_id != grant_id
        || entry.operation != operation
        || entry.target.as_deref() != target
        || entry
            .session_id
            .as_deref()
            .is_some_and(|bound| Some(bound) != session_id)
    {
        return Err(CoreError::ConfirmationRequired);
    }
    if entry.used_at.is_some() || now > entry.expires_at {
        return Err(CoreError::ConfirmationExpired);
    }
    entry.used_at = Some(now);
    save(path, &entries)?;
    Ok(())
}

/// Remove expired and used confirmations.
pub fn prune(path: &Path, now: DateTime<Utc>) -> CoreResult<()> {
    let mut entries = load(path)?;
    entries.retain(|entry| entry.used_at.is_none() && now <= entry.expires_at);
    save(path, &entries)
}

fn load(path: &Path) -> CoreResult<Vec<StoredConfirmation>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let bytes = std::fs::read(path)
        .map_err(|error| CoreError::internal(format!("confirmation read: {error}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::internal(format!("confirmation parse: {error}")))
}

fn save(path: &Path, entries: &[StoredConfirmation]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| CoreError::internal(format!("confirmation directory: {error}")))?;
    }
    let bytes = serde_json::to_vec_pretty(entries)
        .map_err(|error| CoreError::internal(format!("confirmation serialize: {error}")))?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)
        .map_err(|error| CoreError::internal(format!("confirmation write: {error}")))?;
    std::fs::rename(&temporary, path)
        .map_err(|error| CoreError::internal(format!("confirmation replace: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_is_single_use_and_target_bound() {
        let home = tempfile::TempDir::new().unwrap();
        let path = home.path().join("confirmations.json");
        let grant = Uuid::new_v4();
        let confirmation = issue_at(&path, grant, "files.delete", Some("/srv/a"), 60).unwrap();
        assert_eq!(
            consume_at(
                &path,
                confirmation.id,
                grant,
                None,
                "files.delete",
                Some("/srv/b"),
                Utc::now()
            )
            .unwrap_err(),
            CoreError::ConfirmationRequired
        );
        consume_at(
            &path,
            confirmation.id,
            grant,
            None,
            "files.delete",
            Some("/srv/a"),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(
            consume_at(
                &path,
                confirmation.id,
                grant,
                None,
                "files.delete",
                Some("/srv/a"),
                Utc::now()
            )
            .unwrap_err(),
            CoreError::ConfirmationExpired
        );
    }
}
