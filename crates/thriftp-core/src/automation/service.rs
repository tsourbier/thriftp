//! Automation authorization service shared by the CLI, MCP, and desktop.
//!
//! The service owns the cross-process revision/epoch checks, session lifetime,
//! revocation propagation, audit-before-result ordering, and fail-closed
//! behaviour when the audit store cannot persist a record.

use std::collections::HashMap;
use std::sync::Mutex;

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use super::{AuditEvent, AuditStore, AuthorizationGrant, AutomationStore, McpClient};
use crate::error::{CoreError, CoreResult};

/// Runtime state for one authenticated automation session.
#[derive(Debug, Clone)]
pub struct ServiceSession {
    pub token: String,
    pub fingerprint: String,
    pub grant_id: Uuid,
    pub revision: u64,
    pub epoch: u64,
    pub expires_at: DateTime<Utc>,
    pub disconnected: bool,
    pub disconnect_policy: String,
    pub owned_transfer_ids: Vec<Uuid>,
}

/// Shared authorization pipeline for the CLI, MCP, and desktop control surfaces.
pub struct AutomationService {
    store: AutomationStore,
    audit: AuditStore,
    sessions: Mutex<HashMap<String, ServiceSession>>,
}

impl AutomationService {
    #[must_use]
    pub fn new(store: AutomationStore, audit: AuditStore) -> Self {
        Self {
            store,
            audit,
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn open_default() -> CoreResult<Self> {
        Ok(Self::new(
            AutomationStore::open_default()?,
            AuditStore::open_default()?,
        ))
    }

    #[must_use]
    pub fn store(&self) -> &AutomationStore {
        &self.store
    }

    /// Reload persisted state when another process advanced `stateRevision`.
    pub fn reconcile(&mut self) -> CoreResult<bool> {
        let path = self.store.path().to_path_buf();
        let reloaded = AutomationStore::open(path)?;
        if reloaded.state.state_revision > self.store.state.state_revision {
            self.store = reloaded;
            self.terminate_stale_sessions();
            return Ok(true);
        }
        Ok(false)
    }

    fn terminate_stale_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for session in sessions.values_mut() {
                let grant = self
                    .store
                    .state
                    .grants
                    .iter()
                    .find(|grant| grant.id == session.grant_id);
                if grant.is_none_or(|grant| {
                    grant.state != "active"
                        || grant.revision != session.revision
                        || grant.revocation_epoch != session.epoch
                }) {
                    session.disconnected = true;
                }
            }
        }
    }

    pub fn enable(&mut self, mode: &str) -> CoreResult<()> {
        self.store.enable(mode)?;
        self.audit("mcp.enable", "allowed", None, None)
    }

    pub fn disable(&mut self) -> CoreResult<()> {
        self.store.disable()?;
        self.terminate_all_sessions();
        self.audit("mcp.disable", "allowed", None, None)
    }

    fn terminate_all_sessions(&self) {
        if let Ok(mut sessions) = self.sessions.lock() {
            for session in sessions.values_mut() {
                session.disconnected = true;
            }
        }
    }

    /// Open a session after a successful challenge/signature exchange.
    pub fn open_session(
        &mut self,
        token: impl Into<String>,
        fingerprint: &str,
        ttl_hours: i64,
    ) -> CoreResult<ServiceSession> {
        if !self.store.state.settings.mcp_enabled {
            return Err(CoreError::McpDisabled);
        }
        let grant = self
            .store
            .grant_for(fingerprint)
            .cloned()
            .ok_or(CoreError::GrantMissing)?;
        if grant.expires_at.is_some_and(|expires| Utc::now() > expires) {
            return Err(CoreError::GrantExpired);
        }
        let session = ServiceSession {
            token: token.into(),
            fingerprint: fingerprint.to_owned(),
            grant_id: grant.id,
            revision: grant.revision,
            epoch: grant.revocation_epoch,
            expires_at: Utc::now() + Duration::hours(ttl_hours.clamp(1, 24)),
            disconnected: false,
            disconnect_policy: grant.disconnect_policy,
            owned_transfer_ids: Vec::new(),
        };
        self.sessions
            .lock()
            .map_err(|_| CoreError::internal("session lock poisoned"))?
            .insert(session.token.clone(), session.clone());
        self.audit(
            "mcp.automation.authenticate",
            "allowed",
            Some(&session),
            None,
        )?;
        Ok(session)
    }

    /// Validate the session and grant immediately before an operation.
    pub fn authorize(
        &mut self,
        token: &str,
        operation: &str,
        now: DateTime<Utc>,
    ) -> CoreResult<AuthorizationGrant> {
        self.reconcile()?;
        if !self.store.state.settings.mcp_enabled {
            return Err(CoreError::McpDisabled);
        }
        let session = self
            .sessions
            .lock()
            .map_err(|_| CoreError::internal("session lock poisoned"))?
            .get(token)
            .cloned()
            .ok_or(CoreError::ClientNotEnrolled)?;
        if session.disconnected || now > session.expires_at {
            return Err(CoreError::ClientNotEnrolled);
        }
        let grant = self
            .store
            .grant_for(&session.fingerprint)
            .cloned()
            .ok_or(CoreError::GrantRevoked)?;
        if grant.revision != session.revision || grant.revocation_epoch != session.epoch {
            return Err(CoreError::GrantRevoked);
        }
        if grant.expires_at.is_some_and(|expires| now > expires) {
            return Err(CoreError::GrantExpired);
        }
        if !grant.operations.iter().any(|allowed| allowed == operation) {
            self.audit(operation, "denied", Some(&session), Some("scope_denied"))?;
            return Err(CoreError::ScopeDenied);
        }
        if let Ok(mut sessions) = self.sessions.lock()
            && let Some(entry) = sessions.get_mut(token)
        {
            entry.grant_id = grant.id;
        }
        Ok(grant)
    }

    /// Revoke a client, increment its epoch, and disconnect active sessions.
    pub fn revoke(&mut self, fingerprint: &str) -> CoreResult<()> {
        let grant = self
            .store
            .grant_for(fingerprint)
            .cloned()
            .ok_or(CoreError::GrantMissing)?;
        self.store.revoke(fingerprint)?;
        self.store.revoke_policies(grant.id);
        if let Ok(mut sessions) = self.sessions.lock() {
            for session in sessions
                .values_mut()
                .filter(|session| session.fingerprint == fingerprint)
            {
                session.disconnected = true;
            }
        }
        self.audit(
            "mcp.clients.revoke",
            "allowed",
            None,
            Some(&format!("epoch={}", grant.revocation_epoch + 1)),
        )
    }

    /// Disconnect one session according to the grant's disconnect policy.
    pub fn disconnect(&mut self, token: &str) -> CoreResult<String> {
        let policy = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| CoreError::internal("session lock poisoned"))?;
            let session = sessions
                .get_mut(token)
                .ok_or(CoreError::ClientNotEnrolled)?;
            session.disconnected = true;
            session.disconnect_policy.clone()
        };
        self.audit(
            "mcp.clients.disconnect",
            "allowed",
            None,
            Some(&format!("disconnectPolicy={policy}")),
        )?;
        Ok(policy)
    }

    /// Record a transfer submitted by a session so disconnect policy can act.
    pub fn note_transfer(&mut self, token: &str, transfer_id: Uuid) -> CoreResult<()> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| CoreError::internal("session lock poisoned"))?;
        let session = sessions
            .get_mut(token)
            .ok_or(CoreError::ClientNotEnrolled)?;
        session.owned_transfer_ids.push(transfer_id);
        Ok(())
    }

    /// Apply the disconnect policy to transfers owned by the session.
    ///
    /// `continue` leaves work running or queued; `cancel` cancels the session's
    /// pending and active transfers and preserves their offsets.
    pub fn disconnect_with_transfers(
        &mut self,
        token: &str,
        transfers: &mut crate::repository::TransferState,
    ) -> CoreResult<String> {
        let (policy, owned) = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| CoreError::internal("session lock poisoned"))?;
            let session = sessions
                .get_mut(token)
                .ok_or(CoreError::ClientNotEnrolled)?;
            session.disconnected = true;
            (
                session.disconnect_policy.clone(),
                session.owned_transfer_ids.clone(),
            )
        };
        if policy == "cancel" {
            for transfer in &mut transfers.transfers {
                if owned.contains(&transfer.id)
                    && matches!(
                        transfer.state,
                        crate::model::TransferState::Pending | crate::model::TransferState::Active
                    )
                {
                    transfer.state = crate::model::TransferState::Cancelled;
                    transfer.updated_at = Utc::now();
                }
            }
        }
        self.audit(
            "mcp.clients.disconnect",
            "allowed",
            None,
            Some(&format!("disconnectPolicy={policy}")),
        )?;
        Ok(policy)
    }

    /// Record an operation outcome. Audit failure blocks the result.
    pub fn record(
        &mut self,
        operation: &str,
        session: Option<&ServiceSession>,
        result: &str,
        error: Option<&str>,
    ) -> CoreResult<()> {
        self.audit(operation, result, session, error)
    }

    pub fn clients(&self) -> Vec<McpClient> {
        self.store.clients()
    }

    fn audit(
        &self,
        operation: &str,
        result: &str,
        session: Option<&ServiceSession>,
        error: Option<&str>,
    ) -> CoreResult<()> {
        let mut event = AuditEvent {
            seq: 0,
            timestamp: Utc::now(),
            interface: "automation".to_owned(),
            mode: self.store.state.settings.mcp_mode.clone(),
            client_fingerprint: session.map(|session| session.fingerprint.clone()),
            client_display_name: None,
            session_id: session.map(|session| session.token.clone()),
            operation: operation.to_owned(),
            target: None,
            result: result.to_owned(),
            error_code: error.map(str::to_owned),
            duration_ms: None,
            details: None,
            prev_hash: String::new(),
            hash: String::new(),
        };
        self.audit.append(&mut event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::automation::base64_encode;

    fn service(home: &tempfile::TempDir) -> AutomationService {
        let store = AutomationStore::open(home.path().join("automation.json")).unwrap();
        let audit = AuditStore::open(home.path().join("audit"));
        AutomationService::new(store, audit)
    }

    fn approved(service: &mut AutomationService) -> String {
        service.enable("standalone").unwrap();
        let request = service
            .store
            .enroll(&base64_encode(&[3u8; 32]), "agent")
            .unwrap();
        let scope = tempfile::TempDir::new().unwrap();
        service
            .store
            .approve(
                request.id,
                vec![Uuid::new_v4()],
                vec![(
                    "local".to_owned(),
                    None,
                    scope.path().to_string_lossy().into_owned(),
                )],
                vec!["browse.list".to_owned()],
                None,
                "continue",
                "cli",
            )
            .unwrap();
        service.store.pending().first().map_or_else(
            || request.fingerprint.clone(),
            |pending| pending.fingerprint.clone(),
        )
    }

    #[test]
    fn revocation_invalidates_active_sessions() {
        let home = tempfile::TempDir::new().unwrap();
        let mut service = service(&home);
        let fingerprint = approved(&mut service);
        service.open_session("token", &fingerprint, 8).unwrap();
        assert!(
            service
                .authorize("token", "browse.list", Utc::now())
                .is_ok()
        );
        service.revoke(&fingerprint).unwrap();
        assert_eq!(
            service
                .authorize("token", "browse.list", Utc::now())
                .unwrap_err(),
            CoreError::ClientNotEnrolled
        );
    }

    #[test]
    fn reconcile_reloads_cross_process_revocations() {
        let home = tempfile::TempDir::new().unwrap();
        let mut service = service(&home);
        let fingerprint = approved(&mut service);
        let session = service.open_session("token", &fingerprint, 8).unwrap();
        assert!(
            service
                .authorize(&session.token, "browse.list", Utc::now())
                .is_ok()
        );

        // A second process revokes the grant and advances stateRevision.
        let path = home.path().join("automation.json");
        let mut other = AutomationStore::open(&path).unwrap();
        other.revoke(&fingerprint).unwrap();
        assert!(service.reconcile().unwrap());
        assert_eq!(
            service
                .authorize(&session.token, "browse.list", Utc::now())
                .unwrap_err(),
            CoreError::ClientNotEnrolled
        );
    }
}
