//! Destructive-operation confirmations and scoped pre-approval policies.
//!
//! Confirmations are single-use, expiring, and bound to one session, grant,
//! operation, and target. Policies are scoped pre-approvals that record
//! `policy_used` audit events when exercised. Headless callers never receive an
//! implicit confirmation.

use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

/// One pending confirmation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmationRequest {
    pub id: Uuid,
    pub grant_id: Uuid,
    pub session_id: Option<String>,
    pub operation: String,
    pub target: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

/// A scoped pre-approval policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPolicy {
    pub id: Uuid,
    pub grant_id: Uuid,
    pub operations: Vec<String>,
    pub target_prefix: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub single_use: bool,
    pub last_used_at: Option<DateTime<Utc>>,
}

/// Coordinates single-use confirmations and scoped pre-approval policies.
#[derive(Default)]
pub struct ApprovalManager {
    confirmations: Mutex<Vec<ConfirmationRequest>>,
    policies: Mutex<Vec<ApprovalPolicy>>,
}

impl ApprovalManager {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a single-use confirmation bound to a grant and optional session.
    pub fn request(
        &self,
        grant_id: Uuid,
        session_id: Option<&str>,
        operation: &str,
        target: Option<&str>,
        ttl_seconds: i64,
    ) -> CoreResult<ConfirmationRequest> {
        self.request_at(
            grant_id,
            session_id,
            operation,
            target,
            Utc::now(),
            ttl_seconds,
        )
    }

    pub fn request_at(
        &self,
        grant_id: Uuid,
        session_id: Option<&str>,
        operation: &str,
        target: Option<&str>,
        now: DateTime<Utc>,
        ttl_seconds: i64,
    ) -> CoreResult<ConfirmationRequest> {
        if ttl_seconds <= 0 {
            return Err(CoreError::invalid_argument(
                "confirmation TTL must be positive",
            ));
        }
        let confirmation = ConfirmationRequest {
            id: Uuid::new_v4(),
            grant_id,
            session_id: session_id.map(str::to_owned),
            operation: operation.to_owned(),
            target: target.map(str::to_owned),
            created_at: now,
            expires_at: now + Duration::seconds(ttl_seconds),
            used_at: None,
        };
        self.confirmations
            .lock()
            .map_err(|_| CoreError::internal("approval lock poisoned"))?
            .push(confirmation.clone());
        Ok(confirmation)
    }

    /// Consume a confirmation exactly once. Any binding mismatch is rejected.
    pub fn consume(
        &self,
        confirmation_id: Uuid,
        grant_id: Uuid,
        session_id: Option<&str>,
        operation: &str,
        target: Option<&str>,
    ) -> CoreResult<()> {
        self.consume_at(
            confirmation_id,
            grant_id,
            session_id,
            operation,
            target,
            Utc::now(),
        )
    }

    pub fn consume_at(
        &self,
        confirmation_id: Uuid,
        grant_id: Uuid,
        session_id: Option<&str>,
        operation: &str,
        target: Option<&str>,
        now: DateTime<Utc>,
    ) -> CoreResult<()> {
        let mut confirmations = self
            .confirmations
            .lock()
            .map_err(|_| CoreError::internal("approval lock poisoned"))?;
        let confirmation = confirmations
            .iter_mut()
            .find(|confirmation| confirmation.id == confirmation_id)
            .ok_or_else(|| CoreError::NotFound("confirmation".to_owned()))?;
        if confirmation.grant_id != grant_id
            || confirmation.operation != operation
            || confirmation.session_id.as_deref() != session_id
            || confirmation.target.as_deref() != target
        {
            return Err(CoreError::ConfirmationRequired);
        }
        if confirmation.used_at.is_some() {
            return Err(CoreError::ConfirmationExpired);
        }
        if now > confirmation.expires_at {
            return Err(CoreError::ConfirmationExpired);
        }
        confirmation.used_at = Some(now);
        Ok(())
    }

    /// Add a scoped pre-approval policy.
    pub fn add_policy(
        &self,
        grant_id: Uuid,
        operations: Vec<String>,
        target_prefix: Option<&str>,
        expires_at: Option<DateTime<Utc>>,
        single_use: bool,
    ) -> CoreResult<ApprovalPolicy> {
        if operations.is_empty() {
            return Err(CoreError::invalid_argument(
                "a policy requires at least one operation",
            ));
        }
        let policy = ApprovalPolicy {
            id: Uuid::new_v4(),
            grant_id,
            operations,
            target_prefix: target_prefix.map(str::to_owned),
            created_at: Utc::now(),
            expires_at,
            revoked_at: None,
            single_use,
            last_used_at: None,
        };
        self.policies
            .lock()
            .map_err(|_| CoreError::internal("approval lock poisoned"))?
            .push(policy.clone());
        Ok(policy)
    }

    /// Check and record use of a matching pre-approval policy.
    pub fn policy_allows(&self, grant_id: Uuid, operation: &str, target: Option<&str>) -> bool {
        self.policy_allows_at(grant_id, operation, target, Utc::now())
    }

    pub fn policy_allows_at(
        &self,
        grant_id: Uuid,
        operation: &str,
        target: Option<&str>,
        now: DateTime<Utc>,
    ) -> bool {
        let Ok(mut policies) = self.policies.lock() else {
            return false;
        };
        for policy in &mut *policies {
            if policy.grant_id != grant_id
                || policy.revoked_at.is_some()
                || policy.expires_at.is_some_and(|expires| now > expires)
                || !policy.operations.iter().any(|allowed| allowed == operation)
            {
                continue;
            }
            if policy.single_use && policy.last_used_at.is_some() {
                continue;
            }
            if let Some(prefix) = &policy.target_prefix
                && !target.is_some_and(|target| target.starts_with(prefix))
            {
                continue;
            }
            policy.last_used_at = Some(now);
            return true;
        }
        false
    }

    pub fn revoke_policies(&self, grant_id: Uuid) {
        if let Ok(mut policies) = self.policies.lock() {
            for policy in policies
                .iter_mut()
                .filter(|policy| policy.grant_id == grant_id)
            {
                policy.revoked_at = Some(Utc::now());
            }
        }
    }

    /// Purge expired confirmations and policies.
    pub fn cleanup(&self, now: DateTime<Utc>) {
        if let Ok(mut confirmations) = self.confirmations.lock() {
            confirmations.retain(|confirmation| now <= confirmation.expires_at);
        }
        if let Ok(mut policies) = self.policies.lock() {
            policies.retain(|policy| policy.expires_at.is_none_or(|expires| now <= expires));
        }
    }

    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.confirmations
            .lock()
            .map(|items| items.iter().filter(|item| item.used_at.is_none()).count())
            .unwrap_or(0)
    }

    /// Headless operations fail closed instead of prompting.
    pub fn require_or_deny(&self, granted: bool) -> CoreResult<()> {
        if granted {
            Ok(())
        } else {
            Err(CoreError::ConfirmationRequired)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_is_single_use_and_expires() {
        let manager = ApprovalManager::new();
        let grant = Uuid::new_v4();
        let now = Utc::now();
        let confirmation = manager
            .request_at(grant, Some("session"), "files.delete", Some("/a"), now, 120)
            .unwrap();
        manager
            .consume_at(
                confirmation.id,
                grant,
                Some("session"),
                "files.delete",
                Some("/a"),
                now + Duration::seconds(1),
            )
            .unwrap();
        assert_eq!(
            manager
                .consume_at(
                    confirmation.id,
                    grant,
                    Some("session"),
                    "files.delete",
                    Some("/a"),
                    now + Duration::seconds(2),
                )
                .unwrap_err(),
            CoreError::ConfirmationExpired
        );
    }

    #[test]
    fn confirmation_rejects_wrong_session_or_target() {
        let manager = ApprovalManager::new();
        let grant = Uuid::new_v4();
        let confirmation = manager
            .request(grant, Some("session-a"), "files.delete", Some("/a"), 120)
            .unwrap();
        assert_eq!(
            manager
                .consume(
                    confirmation.id,
                    grant,
                    Some("session-b"),
                    "files.delete",
                    Some("/a")
                )
                .unwrap_err(),
            CoreError::ConfirmationRequired
        );
        assert_eq!(
            manager
                .consume(
                    confirmation.id,
                    grant,
                    Some("session-a"),
                    "files.delete",
                    Some("/b")
                )
                .unwrap_err(),
            CoreError::ConfirmationRequired
        );
    }

    #[test]
    fn policy_is_scoped_and_records_use() {
        let manager = ApprovalManager::new();
        let grant = Uuid::new_v4();
        manager
            .add_policy(
                grant,
                vec!["directories.delete".to_owned()],
                Some("/srv/app"),
                None,
                false,
            )
            .unwrap();
        assert!(manager.policy_allows(grant, "directories.delete", Some("/srv/app/cache")));
        assert!(!manager.policy_allows(grant, "directories.delete", Some("/etc")));
        assert!(!manager.policy_allows(grant, "files.delete", Some("/srv/app/cache")));
    }
}
