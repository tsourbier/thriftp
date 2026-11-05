//! Local-only MCP server over stdio.

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde_json::{Value, json};
use thriftp_core::automation::{
    AuditEvent, AuditStore, AutomationStore, base64_decode, canonical_local, canonical_remote,
    path_in_scope,
};
use thriftp_core::browser;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{CredentialService, CredentialStore, Secret};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::host::protocol::{self, RpcError, RpcRequest, RpcResponse};
use thriftp_core::host::{HostRuntime, HostServices};
use thriftp_core::model::{
    ConnectionProfile, CredentialRef, Transfer, TransferDirection, TransferState,
};
use thriftp_core::protocol::plain_ftp_warning;
use thriftp_core::repository::{MetadataStore, TransferStore};
use thriftp_core::transfers::TransferControl;
use thriftp_core::transfers::engine::{execute_download, execute_upload};
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};
use uuid::Uuid;

const CONTRACT_VERSION: &str = "1.0.0";

#[derive(Clone)]
struct Session {
    token: String,
    fingerprint: String,
    display_name: String,
    revision: u64,
    epoch: u64,
    expires_at: DateTime<Utc>,
    owned_transfer_ids: Vec<Uuid>,
}

#[derive(Default)]
struct State {
    store: Option<AutomationStore>,
    audit: Option<AuditStore>,
    challenges: HashMap<String, (String, String, DateTime<Utc>)>,
    sessions: HashMap<String, Session>,
    host: Option<HostRuntime>,
    audit_blocked: Option<String>,
}

impl State {
    fn store(&mut self) -> CoreResult<&mut AutomationStore> {
        if self.store.is_none() {
            self.store = Some(AutomationStore::open_default()?);
        }
        self.store
            .as_mut()
            .ok_or_else(|| CoreError::internal("automation store"))
    }

    /// Reload persisted automation state so cross-process revocation,
    /// disablement, and expiry take effect before the next authorization.
    fn reload(&mut self) -> CoreResult<()> {
        self.store = Some(AutomationStore::open_default()?);
        Ok(())
    }

    /// Prepare a transport: run retention, load state, and require MCP enabled.
    fn prepare(&mut self) -> CoreResult<()> {
        self.reload()?;
        self.audit_blocked = thriftp_core::automation::maintain_audit()
            .err()
            .map(|error| error.to_string());
        let enabled = self
            .store
            .as_ref()
            .is_some_and(|store| store.state.settings.mcp_enabled);
        if !enabled {
            return Err(CoreError::McpDisabled);
        }
        Ok(())
    }

    fn enabled(&self) -> bool {
        self.store
            .as_ref()
            .is_some_and(|store| store.state.settings.mcp_enabled)
    }

    fn audit_store(&mut self) -> CoreResult<&AuditStore> {
        if self.audit.is_none() {
            self.audit = Some(AuditStore::open_default()?);
        }
        self.audit
            .as_ref()
            .ok_or_else(|| CoreError::internal("audit store"))
    }

    /// Acquire or connect to the transfer host so MCP tool calls do not write
    /// transfer state directly when another host exists.
    fn ensure_host(&mut self) -> CoreResult<&mut HostRuntime> {
        if self.host.is_none() {
            let home = thriftp_core::automation::data_dir();
            self.host = Some(HostRuntime::acquire(
                "standalone",
                HostServices {
                    metadata: Arc::new(JsonMetadataStore::new(home.join("metadata.json"))),
                    transfers: Arc::new(JsonTransferStore::new(home.join("transfers.json"))),
                    credentials: Arc::new(KeyringCredentialStore::new())
                        as Arc<dyn CredentialStore>,
                    factory: Arc::new(ProtocolFactory),
                },
            )?);
        }
        self.host
            .as_mut()
            .ok_or_else(|| CoreError::internal("host state"))
    }

    /// Delegate a control method when another process owns the host. Returns
    /// `None` when this MCP server is the host and may act directly.
    fn delegate(&mut self, method: &str, params: Value) -> CoreResult<Option<Value>> {
        let runtime = self.ensure_host()?;
        if runtime.is_host() {
            return Ok(None);
        }
        let client = runtime
            .client()
            .ok_or_else(|| CoreError::HostUnavailable("control channel unavailable".to_owned()))?;
        Ok(Some(client.call(method, params)?))
    }

    fn audit(
        &mut self,
        operation: &str,
        result: &str,
        session: Option<&Session>,
        error: Option<&str>,
    ) -> CoreResult<()> {
        self.audit_detailed(operation, result, session, error, None, None, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn audit_detailed(
        &mut self,
        operation: &str,
        result: &str,
        session: Option<&Session>,
        error: Option<&str>,
        target: Option<Value>,
        details: Option<Value>,
        duration_ms: Option<u64>,
    ) -> CoreResult<()> {
        let display_name = session
            .map(|session| session.display_name.clone())
            .or_else(|| {
                self.store.as_ref().and_then(|store| {
                    session.and_then(|session| {
                        store
                            .state
                            .clients
                            .iter()
                            .find(|client| client.fingerprint == session.fingerprint)
                            .map(|client| client.display_name.clone())
                    })
                })
            });
        let store = self.audit_store()?;
        let mut event = AuditEvent {
            seq: 0,
            timestamp: Utc::now(),
            interface: "mcp".to_owned(),
            mode: "standalone".to_owned(),
            client_fingerprint: session.map(|session| session.fingerprint.clone()),
            client_display_name: display_name,
            session_id: session.map(|session| session.token.clone()),
            operation: operation.to_owned(),
            target,
            result: result.to_owned(),
            error_code: error.map(str::to_owned),
            duration_ms,
            details,
            prev_hash: String::new(),
            hash: String::new(),
        };
        store.append(&mut event)
    }

    fn process_disconnect_requests(&mut self) -> CoreResult<()> {
        let requests = thriftp_core::automation::disconnect::take_requests()?;
        if requests.is_empty() {
            return Ok(());
        }
        let mut transfers = load_transfers()?;
        let mut changed = false;
        for request in requests {
            let sessions: Vec<Session> = self
                .sessions
                .values()
                .filter(|session| session.fingerprint == request.fingerprint)
                .cloned()
                .collect();
            for session in sessions {
                let policy = self
                    .store
                    .as_ref()
                    .and_then(|store| store.grant_for(&session.fingerprint))
                    .map_or_else(
                        || "continue".to_owned(),
                        |grant| grant.disconnect_policy.clone(),
                    );
                if policy == "cancel" {
                    for transfer in &mut transfers.transfers {
                        if session.owned_transfer_ids.contains(&transfer.id)
                            && matches!(
                                transfer.state,
                                TransferState::Pending | TransferState::Active
                            )
                        {
                            transfer.state = TransferState::Cancelled;
                            transfer.updated_at = Utc::now();
                            changed = true;
                        }
                    }
                }
                self.audit_detailed(
                    "mcp.clients.disconnect",
                    "allowed",
                    Some(&session),
                    None,
                    None,
                    Some(json!({"disconnectPolicy": policy})),
                    None,
                )?;
                self.sessions.remove(&session.token);
            }
        }
        if changed {
            save_transfers(&transfers)?;
        }
        self.persist_sessions()
    }

    fn disconnect_all(&mut self) -> CoreResult<()> {
        let sessions = std::mem::take(&mut self.sessions);
        let mut transfers = load_transfers()?;
        let mut changed = false;
        for session in sessions.into_values() {
            let policy = self
                .store
                .as_ref()
                .and_then(|store| store.grant_for(&session.fingerprint))
                .map_or_else(
                    || "continue".to_owned(),
                    |grant| grant.disconnect_policy.clone(),
                );
            if policy == "cancel" {
                for transfer in &mut transfers.transfers {
                    if session.owned_transfer_ids.contains(&transfer.id)
                        && matches!(
                            transfer.state,
                            TransferState::Pending | TransferState::Active
                        )
                    {
                        transfer.state = TransferState::Cancelled;
                        transfer.updated_at = Utc::now();
                        changed = true;
                    }
                }
            }
            self.audit_detailed(
                "mcp.session.disconnect",
                "allowed",
                Some(&session),
                None,
                None,
                Some(json!({"disconnectPolicy": policy})),
                None,
            )?;
        }
        if changed {
            save_transfers(&transfers)?;
        }
        self.persist_sessions()
    }

    fn persist_sessions(&self) -> CoreResult<()> {
        let path = thriftp_core::automation::data_dir()
            .join("run")
            .join("mcp-sessions.json");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| CoreError::internal(error.to_string()))?;
        }
        let sessions: Vec<Value> = self
            .sessions
            .iter()
            .map(|(token, session)| {
                json!({
                    "token": token,
                    "fingerprint": session.fingerprint,
                    "displayName": session.display_name,
                    "grantRevision": session.revision,
                    "revocationEpoch": session.epoch,
                    "expiresAt": session.expires_at,
                    "disconnected": false,
                })
            })
            .collect();
        let bytes = serde_json::to_vec_pretty(&sessions)
            .map_err(|error| CoreError::internal(error.to_string()))?;
        std::fs::write(path, bytes).map_err(|error| CoreError::internal(error.to_string()))
    }
}

pub fn run_stdio() -> CoreResult<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut state = State::default();
    state.prepare()?;
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| CoreError::internal(error.to_string()))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(trimmed) {
            Ok(value) => value,
            Err(error) => {
                let response = json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":format!("parse error: {error}")}});
                writeln!(stdout, "{response}")
                    .map_err(|error| CoreError::internal(error.to_string()))?;
                stdout
                    .flush()
                    .map_err(|error| CoreError::internal(error.to_string()))?;
                continue;
            }
        };
        let id = request.get("id").cloned();
        let method = request
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let result = dispatch(&mut state, method, &params);
        if let Some(id) = id {
            let response = match result {
                Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
                Err(error) => {
                    json!({
                        "jsonrpc":"2.0",
                        "id":id,
                        "error": crate::errors::rpc_error(&error)
                    })
                }
            };
            writeln!(stdout, "{response}")
                .map_err(|error| CoreError::internal(error.to_string()))?;
            stdout
                .flush()
                .map_err(|error| CoreError::internal(error.to_string()))?;
        }
    }
    let _ = state.disconnect_all();
    Ok(())
}

fn dispatch(state: &mut State, method: &str, params: &Value) -> CoreResult<Value> {
    if let Some(version) = params
        .get("_meta")
        .and_then(|meta| meta.get("io.modelcontextprotocol/protocolVersion"))
        .and_then(Value::as_str)
        && version != "2026-07-28"
        && version != "2025-11-25"
    {
        return Err(crate::errors::unsupported_version(version));
    }
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2025-11-25",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "thriftp-mcp", "version": env!("CARGO_PKG_VERSION")}
        })),
        "server/discover" => Ok(json!({
            "protocolVersions": ["2026-07-28", "2025-11-25"],
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "thriftp-mcp", "version": env!("CARGO_PKG_VERSION")},
            "dev.thriftp/automationContractVersion": CONTRACT_VERSION
        })),
        "tools/list" => Ok(json!({"tools": tool_catalog()})),
        "tools/call" => tools_call(state, params),
        "ping" => Ok(json!({})),
        _ => Err(CoreError::UnsupportedCapability(format!(
            "unknown method {method}"
        ))),
    }
}

fn tools_call(state: &mut State, params: &Value) -> CoreResult<Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::validation("tool name is required"))?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let result = call_tool(state, name, &arguments);
    if let Err(error) = &result
        && let Err(audit_error) = state.audit(name, "failed", None, Some(error.code()))
    {
        let payload = json!({
            "schemaVersion": CONTRACT_VERSION,
            "ok": false,
            "data": null,
            "warnings": [],
            "error": {
                "code": "audit_unavailable",
                "message": audit_error.to_string(),
                "retryable": true,
                "details": {}
            }
        });
        return Ok(tool_result(payload, true));
    }
    match result {
        Ok(value) => Ok(tool_result(value, false)),
        Err(error) => Ok(tool_result(
            json!({
                "schemaVersion": CONTRACT_VERSION,
                "ok": false,
                "data": null,
                "warnings": [],
                "error": {
                    "code": error.code(),
                    "message": error.to_string(),
                    "retryable": error.retryable(),
                    "details": {}
                }
            }),
            true,
        )),
    }
}

fn tool_result(value: Value, is_error: bool) -> Value {
    let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_owned());
    json!({
        "content": [{"type": "text", "text": text}],
        "structuredContent": value,
        "isError": is_error
    })
}

fn call_tool(state: &mut State, name: &str, args: &Value) -> CoreResult<Value> {
    state.reload()?;
    state.process_disconnect_requests()?;
    if let Err(error) = thriftp_core::automation::maintain_audit() {
        state.audit_blocked = Some(error.to_string());
    }
    if let Some(message) = state.audit_blocked.clone() {
        let _ = state.audit(name, "blocked", None, Some("audit_unavailable"));
        return Err(CoreError::AuditUnavailable(message));
    }
    if !state.enabled() {
        state.audit(name, "denied", None, Some("mcp_disabled"))?;
        return Err(CoreError::McpDisabled);
    }
    match name {
        "thriftp.automation.status" => {
            state.audit("mcp.automation.status", "allowed", None, None)?;
            let store = state.store()?;
            Ok(json!({
                "enabled": store.status()["enabled"],
                "mode": store.status()["mode"],
                "contractVersion": CONTRACT_VERSION,
                "protocolVersions": ["2026-07-28", "2025-11-25"]
            }))
        }
        "thriftp.automation.enroll" => {
            let public_key = required_str(args, "publicKey")?;
            let display_name = required_str(args, "displayName")?;
            let request = state.store()?.enroll(public_key, display_name)?;
            state.audit("mcp.automation.enroll", "allowed", None, None)?;
            Ok(json!({
                "requestId": request.id,
                "state": request.state,
                "expiresAt": request.expires_at
            }))
        }
        "thriftp.automation.enrollment_status" => {
            let request_id = args.get("requestId").and_then(Value::as_str);
            let store = state.store()?;
            if let Some(request_id) = request_id {
                let id = Uuid::parse_str(request_id)
                    .map_err(|_| CoreError::validation("invalid requestId"))?;
                let request = store
                    .state
                    .enrollment_requests
                    .iter()
                    .find(|request| request.id == id)
                    .ok_or_else(|| CoreError::NotFound("enrollment request".to_owned()))?;
                let value = json!({"state": request.state, "grantId": request.grant_id});
                state.audit("mcp.automation.enrollment_status", "allowed", None, None)?;
                Ok(value)
            } else {
                let value = json!({"pending": store.pending()});
                state.audit("mcp.automation.enrollment_status", "allowed", None, None)?;
                Ok(value)
            }
        }
        "thriftp.automation.challenge" => {
            let fingerprint = required_str(args, "fingerprint")?;
            let store = state.store()?;
            let Some(grant) = store.grant_for(fingerprint) else {
                if store.state.grants.iter().any(|grant| {
                    grant.client_fingerprint == fingerprint && grant.state == "revoked"
                }) {
                    state.audit(
                        "mcp.automation.challenge",
                        "denied",
                        None,
                        Some("grant_revoked"),
                    )?;
                    return Err(CoreError::GrantRevoked);
                }
                return Err(CoreError::GrantMissing);
            };
            if grant
                .expires_at
                .is_some_and(|expires_at| Utc::now() > expires_at)
            {
                state.audit(
                    "mcp.automation.challenge",
                    "denied",
                    None,
                    Some("grant_expired"),
                )?;
                return Err(CoreError::GrantExpired);
            }
            let challenge_id = Uuid::new_v4().to_string();
            let nonce = Uuid::new_v4().to_string();
            let expires_at = Utc::now() + Duration::seconds(60);
            state.challenges.insert(
                fingerprint.to_owned(),
                (challenge_id.clone(), nonce.clone(), expires_at),
            );
            state.audit("mcp.automation.challenge", "allowed", None, None)?;
            Ok(json!({"challengeId": challenge_id, "nonce": nonce, "expiresAt": expires_at}))
        }
        "thriftp.automation.authenticate" => {
            let fingerprint = required_str(args, "fingerprint")?;
            let nonce = required_str(args, "nonce")?;
            let signature_b64 = required_str(args, "signature")?;
            let (challenge_id, expected_nonce, expires_at) = state
                .challenges
                .get(fingerprint)
                .cloned()
                .ok_or(CoreError::Conflict("challenge_required".to_owned()))?;
            if Utc::now() > expires_at || expected_nonce != nonce {
                return Err(CoreError::Conflict("challenge_expired".to_owned()));
            }
            let (grant, client, grant_view) = {
                let store = state.store()?;
                let grant = store
                    .grant_for(fingerprint)
                    .cloned()
                    .ok_or(CoreError::GrantMissing)?;
                let client = store
                    .clients()
                    .into_iter()
                    .find(|client| client.fingerprint == fingerprint)
                    .ok_or(CoreError::ClientNotEnrolled)?;
                let grant_view = store.grant_view(fingerprint);
                (grant, client, grant_view)
            };
            verify_signature(&client.public_key, nonce.as_bytes(), signature_b64)?;
            if grant
                .expires_at
                .is_some_and(|expires_at| Utc::now() > expires_at)
            {
                state.audit(
                    "mcp.automation.authenticate",
                    "denied",
                    None,
                    Some("grant_expired"),
                )?;
                return Err(CoreError::GrantExpired);
            }
            let token = Uuid::new_v4().to_string();
            let session = Session {
                token: token.clone(),
                fingerprint: fingerprint.to_owned(),
                display_name: client.display_name.clone(),
                revision: grant.revision,
                epoch: grant.revocation_epoch,
                expires_at: Utc::now() + Duration::hours(8),
                owned_transfer_ids: Vec::new(),
            };
            state.sessions.insert(token.clone(), session.clone());
            state.challenges.remove(fingerprint);
            let _ = challenge_id;
            state.persist_sessions()?;
            state.audit(
                "mcp.automation.authenticate",
                "allowed",
                Some(&session),
                None,
            )?;
            Ok(json!({
                "sessionToken": token,
                "expiresAt": session.expires_at,
                "grant": grant_view
            }))
        }
        _ => {
            let session = require_session(state, args)?;
            let operation = name.trim_start_matches("thriftp.");
            let (grant, grant_view) = {
                let store = state.store()?;
                let grant = store.grant_for(&session.fingerprint).cloned();
                let revoked = store.state.grants.iter().any(|grant| {
                    grant.client_fingerprint == session.fingerprint && grant.state == "revoked"
                });
                let view = store.grant_view(&session.fingerprint);
                let grant = grant.ok_or(if revoked {
                    CoreError::GrantRevoked
                } else {
                    CoreError::GrantMissing
                })?;
                (grant, view)
            };
            if grant.revocation_epoch != session.epoch || grant.revision != session.revision {
                state.sessions.remove(&session.token);
                state.persist_sessions()?;
                state.audit(name, "denied", Some(&session), Some("grant_revoked"))?;
                return Err(CoreError::GrantRevoked);
            }
            if Utc::now() > session.expires_at {
                state.sessions.remove(&session.token);
                state.persist_sessions()?;
                state.audit(name, "denied", Some(&session), Some("client_not_enrolled"))?;
                return Err(CoreError::ClientNotEnrolled);
            }
            if grant
                .expires_at
                .is_some_and(|expires_at| Utc::now() > expires_at)
            {
                state.audit(name, "denied", Some(&session), Some("grant_expired"))?;
                return Err(CoreError::GrantExpired);
            }
            if name == "thriftp.automation.session" {
                return Ok(grant_view.unwrap_or_else(|| json!({})));
            }
            if !grant.operations.iter().any(|allowed| allowed == operation) {
                state.audit(name, "denied", Some(&session), Some("scope_denied"))?;
                return Err(CoreError::ScopeDenied);
            }
            let target = audit_target(args);
            if is_destructive(name) {
                let destructive_target = if name == "thriftp.directories.rename" {
                    args.get("fromPath").and_then(Value::as_str)
                } else {
                    args.get("path").and_then(Value::as_str)
                };
                check_destructive(
                    state,
                    &grant,
                    Some(session.token.as_str()),
                    args,
                    operation,
                    destructive_target,
                )?;
                // The durable authorization record must precede execution.
                state.audit_detailed(
                    name,
                    "allowed",
                    Some(&session),
                    None,
                    target.clone(),
                    Some(json!({"phase": "pre-execution"})),
                    None,
                )?;
            }
            let started = std::time::Instant::now();
            let result = call_authenticated(state, name, args, &grant);
            let duration = started.elapsed().as_millis() as u64;
            if let Ok(value) = &result {
                record_session_transfers(state, &session.token, value);
            }
            match &result {
                Ok(_) => state.audit_detailed(
                    name,
                    "completed",
                    Some(&session),
                    None,
                    target,
                    None,
                    Some(duration),
                )?,
                Err(error) => state.audit_detailed(
                    name,
                    "failed",
                    Some(&session),
                    Some(error.code()),
                    target,
                    None,
                    Some(duration),
                )?,
            }
            result
        }
    }
}

fn audit_target(args: &Value) -> Option<Value> {
    for key in [
        "profileId",
        "path",
        "fromPath",
        "toPath",
        "transferId",
        "requestId",
    ] {
        if let Some(value) = args.get(key) {
            return Some(json!({key: value}));
        }
    }
    None
}

fn is_destructive(name: &str) -> bool {
    matches!(
        name,
        "thriftp.directories.rename" | "thriftp.directories.delete" | "thriftp.files.delete"
    )
}

fn call_authenticated(
    state: &mut State,
    name: &str,
    args: &Value,
    grant: &thriftp_core::automation::AuthorizationGrant,
) -> CoreResult<Value> {
    match name {
        "thriftp.automation.session" => {
            let store = state.store()?;
            Ok(store
                .grant_view(&grant.client_fingerprint)
                .unwrap_or_else(|| json!({})))
        }
        "thriftp.connections.list" => {
            let metadata = load_metadata()?;
            let profiles: Vec<_> = metadata
                .connection_profiles
                .into_iter()
                .filter(|profile| grant.profile_ids.contains(&profile.id))
                .collect();
            Ok(json!(profiles))
        }
        "thriftp.connections.get" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            if !grant.profile_ids.contains(&profile_id) {
                return Err(CoreError::ScopeDenied);
            }
            let metadata = load_metadata()?;
            let profile = metadata
                .connection_profiles
                .into_iter()
                .find(|profile| profile.id == profile_id)
                .ok_or_else(|| CoreError::NotFound("profile".to_owned()))?;
            let warnings = warnings_for_profile(&profile);
            Ok(with_warnings(json!(profile), warnings))
        }
        "thriftp.browse.list" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let path = required_str(args, "path")?;
            check_remote_scope(state, grant, profile_id, path)?;
            let profile = granted_profile(grant, profile_id)?;
            let secret = stored_secret(&profile)?;
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            let entries = block_on(browser::list_remote(&mut **guard, path))??;
            let warnings = warnings_for_profile(&profile);
            Ok(with_warnings(
                json!({"path": path, "entries": entries, "nextCursor": null}),
                warnings,
            ))
        }
        "thriftp.files.stat" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let path = required_str(args, "path")?;
            check_remote_scope(state, grant, profile_id, path)?;
            let profile = granted_profile(grant, profile_id)?;
            let secret = stored_secret(&profile)?;
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            let entry = block_on(browser::stat_remote(&mut **guard, path))??;
            let warnings = warnings_for_profile(&profile);
            Ok(with_warnings(json!(entry), warnings))
        }
        "thriftp.directories.create" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let path = required_str(args, "path")?;
            check_remote_scope(state, grant, profile_id, path)?;
            let profile = granted_profile(grant, profile_id)?;
            let secret = stored_secret(&profile)?;
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            block_on(browser::create_dir_remote(&mut **guard, path))??;
            check_remote_scope(state, grant, profile_id, path)?;
            Ok(json!({"created": path}))
        }
        "thriftp.directories.rename" | "thriftp.files.delete" | "thriftp.directories.delete" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let path = required_str(
                args,
                if name == "thriftp.directories.rename" {
                    "fromPath"
                } else {
                    "path"
                },
            )?;
            check_remote_scope(state, grant, profile_id, path)?;
            let profile = granted_profile(grant, profile_id)?;
            let secret = stored_secret(&profile)?;
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            if name == "thriftp.directories.rename" {
                let to = required_str(args, "toPath")?;
                check_remote_scope(state, grant, profile_id, to)?;
                block_on(browser::rename_remote(&mut **guard, path, to))??;
                check_remote_scope(state, grant, profile_id, to)?;
                Ok(json!({"renamed": true, "from": path, "to": to}))
            } else {
                let entry = block_on(browser::stat_remote(&mut **guard, path))??;
                block_on(browser::delete_remote(
                    &mut **guard,
                    path,
                    entry.entry_type,
                    true,
                ))??;
                Ok(json!({"deleted": path, "permanent": true}))
            }
        }
        "thriftp.transfers.list" => {
            if let Some(result) = state.delegate("transfers.list", json!({"limit": 1_000}))? {
                return Ok(result);
            }
            let transfers = load_transfers()?.transfers;
            let warnings: Vec<Value> = transfers
                .iter()
                .filter_map(|transfer| {
                    load_metadata()
                        .ok()?
                        .connection_profiles
                        .into_iter()
                        .find(|profile| profile.id == transfer.connection_profile_id)
                        .map(|profile| warnings_for_profile(&profile))
                })
                .flatten()
                .collect();
            Ok(json!({"transfers": transfers, "warnings": warnings}))
        }
        "thriftp.transfers.status" => {
            let id = parse_uuid(required_str(args, "transferId")?)?;
            if let Some(result) = state.delegate("transfers.status", json!({"transferId": id}))? {
                return Ok(result);
            }
            let transfer = load_transfers()?
                .transfers
                .into_iter()
                .find(|transfer| transfer.id == id)
                .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
            let warnings = load_metadata()
                .ok()
                .and_then(|metadata| {
                    metadata
                        .connection_profiles
                        .into_iter()
                        .find(|profile| profile.id == transfer.connection_profile_id)
                })
                .map_or_else(Vec::new, |profile| warnings_for_profile(&profile));
            Ok(with_warnings(json!(transfer), warnings))
        }
        "thriftp.queue.get" => {
            if let Some(result) = state.delegate("queue.get", json!({}))? {
                return Ok(result);
            }
            Ok(json!(load_transfers()?.queue))
        }
        "thriftp.queue.set_concurrency" => {
            let value = args
                .get("concurrency")
                .and_then(Value::as_u64)
                .unwrap_or(4)
                .max(1) as usize;
            if let Some(result) =
                state.delegate("queue.setConcurrency", json!({"concurrency": value}))?
            {
                return Ok(result);
            }
            let mut transfers = load_transfers()?;
            transfers.queue.concurrency_limit = value;
            save_transfers(&transfers)?;
            Ok(json!({"concurrency": value}))
        }
        "thriftp.queue.set_bandwidth" => {
            let value = args.get("bytesPerSecond").and_then(Value::as_u64);
            if let Some(result) =
                state.delegate("queue.setBandwidth", json!({"bytesPerSecond": value}))?
            {
                return Ok(result);
            }
            let mut transfers = load_transfers()?;
            transfers.queue.bandwidth_limit_bytes_per_second = value;
            save_transfers(&transfers)?;
            Ok(json!({"bandwidthLimitBytesPerSecond": value}))
        }
        "thriftp.transfers.upload" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let remote_dir = required_str(args, "remoteDir")?;
            let local_paths = args
                .get("localPaths")
                .and_then(Value::as_array)
                .ok_or_else(|| CoreError::validation("localPaths is required"))?;
            for path in local_paths {
                let path = path
                    .as_str()
                    .ok_or_else(|| CoreError::validation("invalid local path"))?;
                check_local_scope(state, grant, path)?;
            }
            let profile = granted_profile(grant, profile_id)?;
            let warnings = warnings_for_profile(&profile);
            let mut delegated = Vec::new();
            let mut delegated_any = false;
            for source in local_paths {
                revalidate_mcp_grant(state, grant)?;
                let params = json!({
                    "direction": "upload",
                    "profileId": profile_id,
                    "sources": [source],
                    "destination": remote_dir,
                    "recursive": args.get("recursive").and_then(Value::as_bool).unwrap_or(false),
                    "conflictPolicy": args.get("conflictPolicy").and_then(Value::as_str).unwrap_or("fail"),
                    "priority": args.get("priority").and_then(Value::as_str).unwrap_or("normal"),
                    "wait": args.get("wait").and_then(Value::as_bool).unwrap_or(true),
                    "timeoutMs": args.get("timeoutMs").and_then(Value::as_u64),
                    "execute": true
                });
                if let Some(result) = state.delegate("transfers.submit", params)? {
                    delegated_any = true;
                    if let Some(items) = result.get("transfers").and_then(Value::as_array) {
                        delegated.extend(items.iter().cloned());
                    }
                } else {
                    delegated_any = false;
                    break;
                }
            }
            if delegated_any {
                return Ok(with_warnings(json!({"transfers": delegated}), warnings));
            }
            let secret = stored_secret(&profile)?;
            let recursive = args
                .get("recursive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let conflict = args
                .get("conflictPolicy")
                .and_then(Value::as_str)
                .unwrap_or("fail")
                .to_owned();
            let wait = args.get("wait").and_then(Value::as_bool).unwrap_or(true);
            let timeout_ms = args.get("timeoutMs").and_then(Value::as_u64);
            let priority = parse_priority(
                args.get("priority")
                    .and_then(Value::as_str)
                    .unwrap_or("normal"),
            );
            let mut records = Vec::new();
            if !wait {
                for path in local_paths {
                    let path = PathBuf::from(path.as_str().unwrap_or_default());
                    let destination = join_remote(
                        remote_dir,
                        &path.file_name().unwrap_or_default().to_string_lossy(),
                    );
                    let mut transfer = Transfer::new(
                        TransferDirection::Upload,
                        profile.id,
                        path.display().to_string(),
                        destination,
                        None,
                    );
                    transfer.priority = priority;
                    transfer.state = TransferState::Pending;
                    records.push(transfer);
                }
                record_transfers(&records)?;
                return Ok(with_warnings(
                    json!({"transfers": records, "state": "pending"}),
                    warnings,
                ));
            }
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            for path in local_paths {
                revalidate_mcp_grant(state, grant)?;
                let path = PathBuf::from(path.as_str().unwrap_or_default());
                if path.is_dir() {
                    if !recursive {
                        return Err(CoreError::Conflict(
                            "directory upload requires recursive".to_owned(),
                        ));
                    }
                    block_on(upload_directory_mcp(
                        state,
                        grant,
                        &profile,
                        &mut **guard,
                        &path,
                        remote_dir,
                        &conflict,
                        priority,
                        timeout_ms,
                        &mut records,
                    ))??;
                } else {
                    let name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let Some(destination) = block_on(resolve_mcp_remote_destination(
                        &mut **guard,
                        remote_dir,
                        &name,
                        &conflict,
                    ))??
                    else {
                        continue;
                    };
                    if let Ok(Ok(existing)) =
                        block_on(browser::stat_remote(&mut **guard, &destination))
                    {
                        check_remote_entry_scope(
                            state,
                            grant,
                            profile.id,
                            &destination,
                            &existing,
                        )?;
                    }
                    let mut transfer = Transfer::new(
                        TransferDirection::Upload,
                        profile.id,
                        path.display().to_string(),
                        destination,
                        None,
                    );
                    transfer.priority = priority;
                    block_on(with_timeout(
                        execute_upload(
                            &mut **guard,
                            &mut transfer,
                            &path,
                            &TransferControl::new(),
                            None,
                        ),
                        timeout_ms,
                    ))???;
                    records.push(transfer);
                }
            }
            record_transfers(&records)?;
            Ok(with_warnings(json!({"transfers": records}), warnings))
        }
        "thriftp.transfers.download" => {
            let profile_id = parse_uuid(required_str(args, "profileId")?)?;
            let local_dir = required_str(args, "localDir")?;
            check_local_scope(state, grant, local_dir)?;
            let remote_paths = args
                .get("remotePaths")
                .and_then(Value::as_array)
                .ok_or_else(|| CoreError::validation("remotePaths is required"))?;
            for path in remote_paths {
                let path = path
                    .as_str()
                    .ok_or_else(|| CoreError::validation("invalid remote path"))?;
                check_remote_scope(state, grant, profile_id, path)?;
            }
            let profile = granted_profile(grant, profile_id)?;
            let warnings = warnings_for_profile(&profile);
            let mut delegated = Vec::new();
            let mut delegated_any = false;
            for source in remote_paths {
                revalidate_mcp_grant(state, grant)?;
                let params = json!({
                    "direction": "download",
                    "profileId": profile_id,
                    "sources": [source],
                    "destination": local_dir,
                    "recursive": args.get("recursive").and_then(Value::as_bool).unwrap_or(false),
                    "conflictPolicy": args.get("conflictPolicy").and_then(Value::as_str).unwrap_or("fail"),
                    "priority": args.get("priority").and_then(Value::as_str).unwrap_or("normal"),
                    "wait": args.get("wait").and_then(Value::as_bool).unwrap_or(true),
                    "timeoutMs": args.get("timeoutMs").and_then(Value::as_u64),
                    "execute": true
                });
                if let Some(result) = state.delegate("transfers.submit", params)? {
                    delegated_any = true;
                    if let Some(items) = result.get("transfers").and_then(Value::as_array) {
                        delegated.extend(items.iter().cloned());
                    }
                } else {
                    delegated_any = false;
                    break;
                }
            }
            if delegated_any {
                return Ok(with_warnings(json!({"transfers": delegated}), warnings));
            }
            let secret = stored_secret(&profile)?;
            let recursive = args
                .get("recursive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let conflict = args
                .get("conflictPolicy")
                .and_then(Value::as_str)
                .unwrap_or("fail")
                .to_owned();
            let wait = args.get("wait").and_then(Value::as_bool).unwrap_or(true);
            let timeout_ms = args.get("timeoutMs").and_then(Value::as_u64);
            let priority = parse_priority(
                args.get("priority")
                    .and_then(Value::as_str)
                    .unwrap_or("normal"),
            );
            let mut records = Vec::new();
            if !wait {
                for path in remote_paths {
                    let path = path
                        .as_str()
                        .ok_or_else(|| CoreError::validation("invalid remote path"))?;
                    let local = PathBuf::from(local_dir).join(
                        path.rsplit('/')
                            .find(|part| !part.is_empty())
                            .unwrap_or("download"),
                    );
                    let mut transfer = Transfer::new(
                        TransferDirection::Download,
                        profile.id,
                        path.to_owned(),
                        local.display().to_string(),
                        None,
                    );
                    transfer.priority = priority;
                    transfer.state = TransferState::Pending;
                    records.push(transfer);
                }
                record_transfers(&records)?;
                return Ok(with_warnings(
                    json!({"transfers": records, "state": "pending"}),
                    warnings,
                ));
            }
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = block_on(connections.connect(profile.id, secret))??;
            let session = connections.session(active.id)?;
            let mut guard = block_on(session.lock())?;
            for path in remote_paths {
                revalidate_mcp_grant(state, grant)?;
                let path = path
                    .as_str()
                    .ok_or_else(|| CoreError::validation("invalid remote path"))?;
                check_remote_scope(state, grant, profile_id, path)?;
                let entry = block_on(browser::stat_remote(&mut **guard, path))??;
                if entry.entry_type == thriftp_core::model::EntryType::Directory {
                    if !recursive {
                        return Err(CoreError::Conflict(
                            "directory download requires recursive".to_owned(),
                        ));
                    }
                    block_on(download_directory_mcp(
                        state,
                        grant,
                        &profile,
                        &mut **guard,
                        path,
                        Path::new(local_dir),
                        &conflict,
                        priority,
                        timeout_ms,
                        &mut records,
                    ))??;
                } else {
                    check_remote_entry_scope(state, grant, profile_id, path, &entry)?;
                    let Some(local) = resolve_mcp_local_destination(
                        Path::new(local_dir),
                        &entry.name,
                        &conflict,
                    )?
                    else {
                        continue;
                    };
                    check_local_destination_scope(state, grant, &local)?;
                    let mut transfer = Transfer::new(
                        TransferDirection::Download,
                        profile.id,
                        path.to_owned(),
                        local.display().to_string(),
                        entry.size,
                    );
                    transfer.priority = priority;
                    block_on(with_timeout(
                        execute_download(
                            &mut **guard,
                            &mut transfer,
                            &local,
                            &TransferControl::new(),
                            None,
                        ),
                        timeout_ms,
                    ))???;
                    records.push(transfer);
                }
            }
            record_transfers(&records)?;
            Ok(with_warnings(json!({"transfers": records}), warnings))
        }
        "thriftp.transfers.pause" | "thriftp.transfers.resume" | "thriftp.transfers.cancel" => {
            let id = parse_uuid(required_str(args, "transferId")?)?;
            let method = match name {
                "thriftp.transfers.pause" => "transfers.pause",
                "thriftp.transfers.resume" => "transfers.resume",
                _ => "transfers.cancel",
            };
            if let Some(result) = state.delegate(method, json!({"transferId": id}))? {
                return Ok(result);
            }
            let mut transfers = load_transfers()?;
            let state = match name {
                "thriftp.transfers.pause" => TransferState::Paused,
                "thriftp.transfers.resume" => TransferState::Pending,
                _ => TransferState::Cancelled,
            };
            let transfer = transfers
                .transfers
                .iter_mut()
                .find(|transfer| transfer.id == id)
                .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
            transfer.state = state;
            save_transfers(&transfers)?;
            Ok(json!({"transferId": id, "state": state}))
        }
        "thriftp.transfers.priority" => {
            let id = parse_uuid(required_str(args, "transferId")?)?;
            let priority = required_str(args, "priority")?;
            if let Some(result) = state.delegate(
                "transfers.priority",
                json!({"transferId": id, "priority": priority}),
            )? {
                return Ok(result);
            }
            let mut transfers = load_transfers()?;
            let transfer = transfers
                .transfers
                .iter_mut()
                .find(|transfer| transfer.id == id)
                .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
            transfer.priority = match priority {
                "low" => thriftp_core::model::Priority::Low,
                "high" => thriftp_core::model::Priority::High,
                _ => thriftp_core::model::Priority::Normal,
            };
            save_transfers(&transfers)?;
            Ok(json!({"transferId": id, "priority": priority}))
        }
        "thriftp.transfers.reorder" => {
            let ids = args
                .get("transferIds")
                .and_then(Value::as_array)
                .ok_or_else(|| CoreError::validation("transferIds is required"))?;
            if let Some(result) =
                state.delegate("transfers.reorder", json!({"transferIds": ids}))?
            {
                return Ok(result);
            }
            let mut transfers = load_transfers()?;
            transfers.queue.ordered_transfer_ids = ids
                .iter()
                .filter_map(Value::as_str)
                .filter_map(|value| Uuid::parse_str(value).ok())
                .collect();
            save_transfers(&transfers)?;
            Ok(json!({"orderedTransferIds": transfers.queue.ordered_transfer_ids}))
        }
        _ => Err(CoreError::UnsupportedCapability(format!(
            "unknown tool {name}"
        ))),
    }
}

fn require_session(state: &State, args: &Value) -> CoreResult<Session> {
    let token = args
        .get("_meta")
        .and_then(|meta| meta.get("dev.thriftp/sessionToken"))
        .and_then(Value::as_str)
        .or_else(|| args.get("sessionToken").and_then(Value::as_str))
        .ok_or(CoreError::ClientNotEnrolled)?;
    state
        .sessions
        .get(token)
        .cloned()
        .ok_or(CoreError::ClientNotEnrolled)
}

fn verify_signature(public_key: &str, message: &[u8], signature: &str) -> CoreResult<()> {
    let key_bytes = base64_decode(public_key)?;
    let key_array: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| CoreError::validation("invalid public key"))?;
    let key = VerifyingKey::from_bytes(&key_array)
        .map_err(|_| CoreError::validation("invalid public key"))?;
    let signature_bytes = base64_decode(signature)?;
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| CoreError::validation("invalid signature"))?;
    key.verify(message, &signature)
        .map_err(|_| CoreError::Conflict("signature_invalid".to_owned()))
}

fn required_str<'a>(args: &'a Value, key: &str) -> CoreResult<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CoreError::validation(format!("{key} is required")))
}

fn parse_uuid(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| CoreError::validation("invalid UUID"))
}

fn load_metadata() -> CoreResult<thriftp_core::repository::AppState> {
    JsonMetadataStore::new(thriftp_core::automation::data_dir().join("metadata.json")).load()
}

fn load_transfers() -> CoreResult<thriftp_core::repository::TransferState> {
    JsonTransferStore::new(thriftp_core::automation::data_dir().join("transfers.json")).load()
}

fn save_transfers(state: &thriftp_core::repository::TransferState) -> CoreResult<()> {
    JsonTransferStore::new(thriftp_core::automation::data_dir().join("transfers.json")).save(state)
}

fn record_transfers(records: &[Transfer]) -> CoreResult<()> {
    let mut state = load_transfers()?;
    for transfer in records {
        state.transfers.push(transfer.clone());
        state.history.push(transfer.clone());
    }
    save_transfers(&state)
}

fn record_session_transfers(state: &mut State, token: &str, value: &Value) {
    let Some(transfers) = value.get("transfers").and_then(Value::as_array) else {
        return;
    };
    let ids: Vec<Uuid> = transfers
        .iter()
        .filter_map(|transfer| transfer.get("id").and_then(Value::as_str))
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    if let Some(session) = state.sessions.get_mut(token) {
        session.owned_transfer_ids.extend(ids);
    }
}

fn revalidate_mcp_grant(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
) -> CoreResult<()> {
    state.reload()?;
    if !state.enabled() {
        return Err(CoreError::McpDisabled);
    }
    let store = state.store()?;
    let current = store
        .grant_for(&grant.client_fingerprint)
        .ok_or(CoreError::GrantRevoked)?;
    if current.revision != grant.revision || current.revocation_epoch != grant.revocation_epoch {
        return Err(CoreError::GrantRevoked);
    }
    if current
        .expires_at
        .is_some_and(|expires_at| Utc::now() > expires_at)
    {
        return Err(CoreError::GrantExpired);
    }
    Ok(())
}

fn remote_link_target(base_path: &str, target: &str) -> CoreResult<String> {
    if target.starts_with('/') {
        canonical_remote(target)
    } else {
        let parent = base_path.rsplit_once('/').map_or("/", |(parent, _)| parent);
        canonical_remote(&format!("{parent}/{target}"))
    }
}

fn check_remote_entry_scope(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    profile_id: Uuid,
    path: &str,
    entry: &thriftp_core::model::FileEntry,
) -> CoreResult<()> {
    if entry.entry_type != thriftp_core::model::EntryType::Symlink {
        return Ok(());
    }
    let target = entry
        .symlink_target
        .as_deref()
        .ok_or(CoreError::PathAmbiguous)?;
    let resolved = remote_link_target(path, target)?;
    let scopes = state.store()?.scopes_for(grant.id);
    let allowed = scopes.iter().any(|scope| {
        scope.kind == "remote"
            && scope.profile_id == Some(profile_id)
            && path_in_scope(&resolved, &scope.canonical_path)
    });
    if allowed {
        Ok(())
    } else {
        Err(CoreError::ScopeDenied)
    }
}

async fn resolve_mcp_remote_destination(
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    directory: &str,
    name: &str,
    policy: &str,
) -> CoreResult<Option<String>> {
    let candidate = join_remote(directory, name);
    let existing = remote.stat(&candidate).await.ok();
    if existing.is_none() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            for suffix in 1..1_000 {
                let candidate = join_remote(directory, &format!("{name}.{suffix}"));
                if remote.stat(&candidate).await.is_err() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict("no free destination name".to_owned()))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {candidate}"
        ))),
        other => Err(CoreError::invalid_argument(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn resolve_mcp_local_destination(
    directory: &Path,
    name: &str,
    policy: &str,
) -> CoreResult<Option<PathBuf>> {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            for suffix in 1..1_000 {
                let candidate = directory.join(format!("{name}.{suffix}"));
                if !candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict("no free destination name".to_owned()))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {}",
            candidate.display()
        ))),
        other => Err(CoreError::invalid_argument(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn check_local_destination_scope(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    path: &Path,
) -> CoreResult<()> {
    if !path.exists() {
        return Ok(());
    }
    let canonical = canonical_local(path)?;
    check_local_scope(state, grant, &canonical)
}

async fn with_timeout<F: std::future::Future>(
    future: F,
    timeout_ms: Option<u64>,
) -> CoreResult<F::Output> {
    match timeout_ms {
        Some(milliseconds) => {
            tokio::time::timeout(std::time::Duration::from_millis(milliseconds), future)
                .await
                .map_err(|_| CoreError::Timeout("MCP transfer timed out".to_owned()))
        }
        None => Ok(future.await),
    }
}

#[allow(clippy::too_many_arguments)]
async fn upload_directory_mcp(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    profile: &ConnectionProfile,
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    local: &Path,
    remote_dir: &str,
    conflict: &str,
    priority: thriftp_core::model::Priority,
    timeout_ms: Option<u64>,
    records: &mut Vec<Transfer>,
) -> CoreResult<()> {
    for entry in std::fs::read_dir(local).map_err(|error| CoreError::internal(error.to_string()))? {
        revalidate_mcp_grant(state, grant)?;
        let entry = entry.map_err(|error| CoreError::internal(error.to_string()))?;
        let path = entry.path();
        if path
            .symlink_metadata()
            .is_ok_and(|metadata| metadata.file_type().is_symlink())
        {
            check_local_scope(state, grant, &path.display().to_string())?;
            eprintln!("warning: skipping symbolic link {}", path.display());
            continue;
        }
        if path.is_dir() {
            let target = join_remote(remote_dir, &entry.file_name().to_string_lossy());
            let _ = remote.mkdir(&target).await;
            Box::pin(upload_directory_mcp(
                state, grant, profile, remote, &path, &target, conflict, priority, timeout_ms,
                records,
            ))
            .await?;
        } else {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(destination) =
                resolve_mcp_remote_destination(remote, remote_dir, &name, conflict).await?
            else {
                continue;
            };
            if let Ok(existing) = remote.stat(&destination).await {
                check_remote_entry_scope(state, grant, profile.id, &destination, &existing)?;
            }
            let mut transfer = Transfer::new(
                TransferDirection::Upload,
                profile.id,
                path.display().to_string(),
                destination,
                None,
            );
            transfer.priority = priority;
            with_timeout(
                execute_upload(remote, &mut transfer, &path, &TransferControl::new(), None),
                timeout_ms,
            )
            .await??;
            records.push(transfer);
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn download_directory_mcp(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    profile: &ConnectionProfile,
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    remote_dir: &str,
    local_dir: &Path,
    conflict: &str,
    priority: thriftp_core::model::Priority,
    timeout_ms: Option<u64>,
    records: &mut Vec<Transfer>,
) -> CoreResult<()> {
    let target = local_dir.join(remote_dir.rsplit('/').next().unwrap_or("download"));
    std::fs::create_dir_all(&target).map_err(|error| CoreError::internal(error.to_string()))?;
    for entry in remote.list_dir(remote_dir).await? {
        revalidate_mcp_grant(state, grant)?;
        if entry.entry_type == thriftp_core::model::EntryType::Directory {
            Box::pin(download_directory_mcp(
                state,
                grant,
                profile,
                remote,
                &entry.path,
                &target,
                conflict,
                priority,
                timeout_ms,
                records,
            ))
            .await?;
        } else if entry.entry_type != thriftp_core::model::EntryType::Symlink {
            let Some(local_path) = resolve_mcp_local_destination(&target, &entry.name, conflict)?
            else {
                continue;
            };
            check_local_destination_scope(state, grant, &local_path)?;
            let mut transfer = Transfer::new(
                TransferDirection::Download,
                profile.id,
                entry.path.clone(),
                local_path.display().to_string(),
                entry.size,
            );
            transfer.priority = priority;
            with_timeout(
                execute_download(
                    remote,
                    &mut transfer,
                    &local_path,
                    &TransferControl::new(),
                    None,
                ),
                timeout_ms,
            )
            .await??;
            records.push(transfer);
        } else {
            check_remote_entry_scope(state, grant, profile.id, &entry.path, &entry)?;
            eprintln!("warning: skipping symbolic link {}", entry.path);
        }
    }
    Ok(())
}

fn stored_secret(profile: &ConnectionProfile) -> CoreResult<Option<Secret>> {
    let reference = CredentialRef::for_profile(profile);
    CredentialService::new(Box::new(KeyringCredentialStore::new())).get(&reference)
}

fn granted_profile(
    grant: &thriftp_core::automation::AuthorizationGrant,
    profile_id: Uuid,
) -> CoreResult<ConnectionProfile> {
    if !grant.profile_ids.contains(&profile_id) {
        return Err(CoreError::ScopeDenied);
    }
    load_metadata()?
        .connection_profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| CoreError::NotFound("profile".to_owned()))
}

fn check_destructive(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    session_token: Option<&str>,
    args: &Value,
    operation: &str,
    target: Option<&str>,
) -> CoreResult<()> {
    if let Some(confirmation_id) = args.get("confirmationId").and_then(Value::as_str) {
        let confirmation_id = Uuid::parse_str(confirmation_id)
            .map_err(|_| CoreError::validation("invalid confirmationId"))?;
        let path = thriftp_core::automation::confirmation::default_path();
        return thriftp_core::automation::confirmation::consume_at(
            &path,
            confirmation_id,
            grant.id,
            session_token,
            operation,
            target,
            Utc::now(),
        );
    }
    if state.store()?.policy_allows(grant.id, operation)? {
        state.audit("policy_used", "allowed", None, Some(operation))?;
        return Ok(());
    }
    Err(CoreError::ConfirmationRequired)
}

fn warnings_for_profile(profile: &ConnectionProfile) -> Vec<Value> {
    plain_ftp_warning(profile.protocol)
        .and_then(|warning| serde_json::to_value(warning).ok())
        .into_iter()
        .collect()
}

fn with_warnings(mut value: Value, warnings: Vec<Value>) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert("warnings".to_owned(), json!(warnings));
    }
    value
}

fn check_remote_scope(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    profile_id: Uuid,
    path: &str,
) -> CoreResult<()> {
    let canonical = canonical_remote(path)?;
    let scopes = state.store()?.scopes_for(grant.id);
    let allowed = scopes.iter().any(|scope| {
        scope.kind == "remote"
            && scope.profile_id == Some(profile_id)
            && path_in_scope(&canonical, &scope.canonical_path)
    });
    if allowed {
        Ok(())
    } else {
        Err(CoreError::ScopeDenied)
    }
}

fn check_local_scope(
    state: &mut State,
    grant: &thriftp_core::automation::AuthorizationGrant,
    path: &str,
) -> CoreResult<()> {
    let canonical = canonical_local(std::path::Path::new(path))?;
    let scopes = state.store()?.scopes_for(grant.id);
    let allowed = scopes
        .iter()
        .any(|scope| scope.kind == "local" && path_in_scope(&canonical, &scope.canonical_path));
    if allowed {
        Ok(())
    } else {
        Err(CoreError::ScopeDenied)
    }
}

fn join_remote(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

fn parse_priority(value: &str) -> thriftp_core::model::Priority {
    match value {
        "low" => thriftp_core::model::Priority::Low,
        "high" => thriftp_core::model::Priority::High,
        _ => thriftp_core::model::Priority::Normal,
    }
}

fn tool_catalog() -> Value {
    let mut tools = Vec::new();
    tools.push(tool(
        "thriftp.connections.list",
        "List granted connection profiles",
        json!({}),
        &[],
        false,
    ));
    tools.push(tool(
        "thriftp.connections.get",
        "Get one granted profile",
        json!({"profileId": {"type": "string", "format": "uuid"}}),
        &["profileId"],
        false,
    ));
    tools.push(tool(
        "thriftp.browse.list",
        "List a remote directory",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "path": {"type": "string"},
            "cursor": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 1000}
        }),
        &["profileId", "path"],
        false,
    ));
    tools.push(tool(
        "thriftp.files.stat",
        "Inspect a remote file",
        json!({"profileId": {"type": "string", "format": "uuid"}, "path": {"type": "string"}}),
        &["profileId", "path"],
        false,
    ));
    tools.push(tool(
        "thriftp.directories.create",
        "Create a remote directory",
        json!({"profileId": {"type": "string", "format": "uuid"}, "path": {"type": "string"}}),
        &["profileId", "path"],
        false,
    ));
    tools.push(tool(
        "thriftp.directories.rename",
        "Rename a remote path",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "fromPath": {"type": "string"},
            "toPath": {"type": "string"},
            "confirmationId": {"type": "string", "format": "uuid"}
        }),
        &["profileId", "fromPath", "toPath"],
        true,
    ));
    tools.push(tool(
        "thriftp.directories.delete",
        "Delete a remote directory",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "path": {"type": "string"},
            "recursive": {"type": "boolean"},
            "confirmationId": {"type": "string", "format": "uuid"}
        }),
        &["profileId", "path"],
        true,
    ));
    tools.push(tool(
        "thriftp.files.delete",
        "Delete a remote file",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "path": {"type": "string"},
            "confirmationId": {"type": "string", "format": "uuid"}
        }),
        &["profileId", "path"],
        true,
    ));
    tools.push(tool(
        "thriftp.transfers.upload",
        "Upload local files or directories",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "remoteDir": {"type": "string"},
            "localPaths": {"type": "array", "items": {"type": "string"}, "minItems": 1},
            "recursive": {"type": "boolean"},
            "conflictPolicy": {"type": "string", "enum": ["fail", "overwrite", "skip", "rename"]},
            "priority": {"type": "string", "enum": ["low", "normal", "high"]},
            "wait": {"type": "boolean"},
            "timeoutMs": {"type": "integer", "minimum": 0},
            "disconnectPolicy": {"type": "string", "enum": ["continue", "cancel"]}
        }),
        &["profileId", "remoteDir", "localPaths"],
        false,
    ));
    tools.push(tool(
        "thriftp.transfers.download",
        "Download remote files or directories",
        json!({
            "profileId": {"type": "string", "format": "uuid"},
            "localDir": {"type": "string"},
            "remotePaths": {"type": "array", "items": {"type": "string"}, "minItems": 1},
            "recursive": {"type": "boolean"},
            "conflictPolicy": {"type": "string", "enum": ["fail", "overwrite", "skip", "rename"]},
            "priority": {"type": "string", "enum": ["low", "normal", "high"]},
            "wait": {"type": "boolean"},
            "timeoutMs": {"type": "integer", "minimum": 0},
            "disconnectPolicy": {"type": "string", "enum": ["continue", "cancel"]}
        }),
        &["profileId", "localDir", "remotePaths"],
        false,
    ));
    tools.push(tool(
        "thriftp.transfers.list",
        "List transfers",
        json!({
            "state": {"type": "string", "enum": ["pending", "active", "paused", "completed", "failed", "cancelled", "interrupted"]},
            "cursor": {"type": "string"},
            "limit": {"type": "integer", "minimum": 1, "maximum": 1000}
        }),
        &[],
        false,
    ));
    tools.push(tool(
        "thriftp.transfers.status",
        "Get transfer status",
        json!({"transferId": {"type": "string", "format": "uuid"}}),
        &["transferId"],
        false,
    ));
    for (name, description) in [
        ("thriftp.transfers.pause", "Pause a transfer"),
        ("thriftp.transfers.resume", "Resume a transfer"),
        ("thriftp.transfers.cancel", "Cancel a transfer"),
    ] {
        tools.push(tool(
            name,
            description,
            json!({"transferId": {"type": "string", "format": "uuid"}}),
            &["transferId"],
            false,
        ));
    }
    tools.push(tool(
        "thriftp.transfers.priority",
        "Set transfer priority",
        json!({
            "transferId": {"type": "string", "format": "uuid"},
            "priority": {"type": "string", "enum": ["low", "normal", "high"]}
        }),
        &["transferId", "priority"],
        false,
    ));
    tools.push(tool(
        "thriftp.transfers.reorder",
        "Reorder queued transfers",
        json!({"transferIds": {"type": "array", "items": {"type": "string", "format": "uuid"}}}),
        &["transferIds"],
        false,
    ));
    tools.push(tool(
        "thriftp.queue.get",
        "Get queue settings",
        json!({}),
        &[],
        false,
    ));
    tools.push(tool(
        "thriftp.queue.set_concurrency",
        "Set queue concurrency",
        json!({"concurrency": {"type": "integer", "minimum": 1, "maximum": 64}}),
        &["concurrency"],
        false,
    ));
    tools.push(tool(
        "thriftp.queue.set_bandwidth",
        "Set global bandwidth in bytes per second",
        json!({"bytesPerSecond": {"type": ["integer", "null"], "minimum": 1}}),
        &["bytesPerSecond"],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.status",
        "Automation status",
        json!({}),
        &[],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.enroll",
        "Submit an enrollment request",
        json!({
            "publicKey": {"type": "string"},
            "displayName": {"type": "string", "minLength": 1, "maxLength": 100}
        }),
        &["publicKey", "displayName"],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.enrollment_status",
        "Get enrollment status",
        json!({"requestId": {"type": "string", "format": "uuid"}}),
        &[],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.challenge",
        "Request a session challenge",
        json!({"fingerprint": {"type": "string"}}),
        &["fingerprint"],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.authenticate",
        "Authenticate a session",
        json!({
            "fingerprint": {"type": "string"},
            "nonce": {"type": "string"},
            "signature": {"type": "string"}
        }),
        &["fingerprint", "nonce", "signature"],
        false,
    ));
    tools.push(tool(
        "thriftp.automation.session",
        "Describe the current session",
        json!({}),
        &[],
        false,
    ));
    json!(tools)
}

fn tool(
    name: &str,
    description: &str,
    properties: Value,
    required: &[&str],
    destructive: bool,
) -> Value {
    json!({
        "name": name,
        "description": description,
        "annotations": {
            "destructiveHint": destructive,
            "readOnlyHint": !destructive
        },
        "inputSchema": {
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "type": "object",
            "properties": properties,
            "required": required,
            "additionalProperties": false
        },
        "outputSchema": output_schema(name)
    })
}

fn output_schema(name: &str) -> Value {
    let data = match name {
        "thriftp.connections.list" => json!({"type": "array", "items": {"type": "object"}}),
        "thriftp.connections.get" => json!({"type": "object"}),
        "thriftp.browse.list" => json!({
            "type": "object",
            "properties": {
                "path": {"type": "string"},
                "entries": {"type": "array", "items": {"type": "object"}},
                "nextCursor": {"type": ["string", "null"]}
            }
        }),
        "thriftp.files.stat" | "thriftp.directories.create" => json!({"type": "object"}),
        "thriftp.transfers.upload" | "thriftp.transfers.download" => json!({
            "type": "object",
            "properties": {
                "transfers": {"type": "array", "items": {"type": "object"}},
                "state": {"type": "string"}
            }
        }),
        "thriftp.transfers.list" => json!({
            "type": "object",
            "properties": {"transfers": {"type": "array", "items": {"type": "object"}}}
        }),
        "thriftp.transfers.status"
        | "thriftp.transfers.pause"
        | "thriftp.transfers.resume"
        | "thriftp.transfers.cancel"
        | "thriftp.transfers.priority" => json!({"type": "object"}),
        "thriftp.transfers.reorder" => json!({
            "type": "object",
            "properties": {"orderedTransferIds": {"type": "array", "items": {"type": "string"}}}
        }),
        "thriftp.queue.get" | "thriftp.queue.set_concurrency" | "thriftp.queue.set_bandwidth" => {
            json!({"type": "object"})
        }
        _ => json!({"type": ["object", "null"]}),
    };
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "schemaVersion": {"type": "string"},
            "ok": {"type": "boolean"},
            "data": data,
            "warnings": {"type": "array", "items": {"type": "object"}},
            "error": {"type": ["object", "null"]}
        },
        "required": ["schemaVersion", "ok", "data", "warnings", "error"]
    })
}

fn block_on<F: std::future::Future>(future: F) -> CoreResult<F::Output> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| CoreError::internal(format!("could not start async runtime: {error}")))?;
    Ok(runtime.block_on(future))
}

/// Dispatch a method without authentication for benchmarks.
#[doc(hidden)]
pub fn dispatch_for_bench(method: &str, params: &Value) -> CoreResult<Value> {
    let mut state = State::default();
    dispatch(&mut state, method, params)
}

/// Handle for an embedded loopback MCP server.
pub struct LoopbackHandle {
    pub endpoint: String,
    pub token_path: PathBuf,
    shutdown: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl LoopbackHandle {
    /// Stop accepting requests and wait for the server thread to exit.
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

impl Drop for LoopbackHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Start an embedded loopback Streamable HTTP transport.
///
/// The caller owns the returned handle and must keep it alive for the server's
/// lifetime. Tokens are loaded from a protected file or generated and written
/// with `0600` permissions; they are never accepted as process arguments.
pub fn spawn_loopback(port: Option<u16>, token: Option<String>) -> CoreResult<LoopbackHandle> {
    let mut state = State::default();
    state.prepare()?;
    let listener = TcpListener::bind(("127.0.0.1", port.unwrap_or(0))).map_err(|error| {
        CoreError::HostUnavailable(format!("cannot bind loopback MCP endpoint: {error}"))
    })?;
    listener
        .set_nonblocking(true)
        .map_err(|error| CoreError::internal(format!("loopback listener setup: {error}")))?;
    let address = listener
        .local_addr()
        .map_err(|error| CoreError::internal(format!("loopback address: {error}")))?;
    let token = token.unwrap_or_else(crate::transport::new_bearer_token);
    let token_path = write_loopback_token(&token)?;
    eprintln!(
        "thriftp-mcp: loopback endpoint http://{address}/mcp; token file {}",
        token_path.display()
    );
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&shutdown);
    let join = std::thread::Builder::new()
        .name("thriftp-mcp-loopback".to_owned())
        .spawn(move || {
            let mut state = state;
            while !flag.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let _ = handle_http(&mut state, stream, &token);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(StdDuration::from_millis(20));
                    }
                    Err(_) => break,
                }
            }
            let _ = state.disconnect_all();
        })
        .map_err(|error| CoreError::internal(format!("cannot start loopback MCP: {error}")))?;
    Ok(LoopbackHandle {
        endpoint: format!("http://{address}/mcp"),
        token_path,
        shutdown,
        join: Some(join),
    })
}

/// Run the loopback transport until the process exits.
pub fn run_loopback(port: Option<u16>, token_file: Option<PathBuf>) -> CoreResult<()> {
    let token = token_file
        .map(|path| {
            std::fs::read_to_string(&path)
                .map(|value| value.trim().to_owned())
                .map_err(|error| {
                    CoreError::invalid_argument(format!(
                        "could not read MCP token file {}: {error}",
                        path.display()
                    ))
                })
        })
        .transpose()?;
    let handle = spawn_loopback(port, token)?;
    loop {
        std::thread::sleep(StdDuration::from_secs(3_600));
        let _ = &handle;
    }
}

fn write_loopback_token(token: &str) -> CoreResult<PathBuf> {
    let directory = thriftp_core::automation::data_dir().join("run");
    std::fs::create_dir_all(&directory)
        .map_err(|error| CoreError::internal(format!("token directory: {error}")))?;
    let path = directory.join("mcp.token");
    std::fs::write(&path, token)
        .map_err(|error| CoreError::internal(format!("token write: {error}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| CoreError::internal(format!("token permissions: {error}")))?;
    }
    Ok(path)
}

fn handle_http(state: &mut State, mut stream: TcpStream, token: &str) -> CoreResult<()> {
    let _ = stream.set_read_timeout(Some(StdDuration::from_secs(10)));
    let mut reader = std::io::BufReader::new(
        stream
            .try_clone()
            .map_err(|error| CoreError::HostUnavailable(format!("loopback stream: {error}")))?,
    );
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|error| CoreError::HostUnavailable(format!("HTTP request line: {error}")))?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    if method != "POST" || path != "/mcp" {
        return respond(&mut stream, 405, &json!({"error": "method not allowed"}));
    }

    let mut headers = HashMap::new();
    let mut header_bytes = 0usize;
    loop {
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .map_err(|error| CoreError::HostUnavailable(format!("HTTP header: {error}")))?;
        header_bytes += line.len();
        if header_bytes > 64 * 1024 {
            return respond(&mut stream, 431, &json!({"error": "headers too large"}));
        }
        if line == "\r\n" || line == "\n" || line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
    }

    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if length > protocol::MAX_MESSAGE_BYTES {
        return respond(&mut stream, 413, &json!({"error": "message too large"}));
    }
    // Consume the body before rejecting the request so the peer can read the
    // response instead of seeing a connection reset.
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| CoreError::HostUnavailable(format!("HTTP body: {error}")))?;
    let text = String::from_utf8(body)
        .map_err(|_| CoreError::invalid_argument("request body is not UTF-8"))?;

    if crate::transport::validate_origin(
        headers.get("origin").map(String::as_str),
        headers.get("host").map(String::as_str),
    )
    .is_err()
    {
        return respond(
            &mut stream,
            403,
            &json!({"error": "remote origin rejected"}),
        );
    }

    let authorized = headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|value| protocol::constant_time_eq(value, token));
    if !authorized {
        return respond(&mut stream, 401, &json!({"error": "unauthorized"}));
    }

    let response = match RpcRequest::parse(&text) {
        Ok(request) => {
            let id = request.id.clone();
            match dispatch(state, &request.method, &request.params) {
                Ok(result) => RpcResponse::success(id, result),
                Err(error) => RpcResponse::failure(id, to_rpc_error(&error)),
            }
        }
        Err(error) => RpcResponse::failure(None, to_rpc_error(&error)),
    };
    respond(
        &mut stream,
        200,
        &serde_json::to_value(response).unwrap_or_default(),
    )
}

fn to_rpc_error(error: &CoreError) -> RpcError {
    let value = crate::errors::rpc_error(error);
    RpcError {
        code: value.get("code").and_then(Value::as_i64).unwrap_or(-32603) as i32,
        message: value
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("internal error")
            .to_owned(),
        data: value.get("data").cloned(),
    }
}

fn respond(stream: &mut TcpStream, status: u16, body: &Value) -> CoreResult<()> {
    let body = serde_json::to_string(body)
        .map_err(|error| CoreError::internal(format!("HTTP response: {error}")))?;
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        405 => "Method Not Allowed",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .and_then(|()| stream.flush())
    .map_err(|error| CoreError::HostUnavailable(format!("HTTP response: {error}")))
}
