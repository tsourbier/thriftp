//! Tauri automation commands: MCP lifecycle, grants, sessions, audit, and retention.
//!
//! Commands stay thin and delegate to the shared core automation and host
//! services. The desktop never owns a second transfer implementation.

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;
use thriftp_core::automation::retention::{RetentionPolicy, apply_retention};
use thriftp_core::automation::{AuditStore, AutomationStore};
use thriftp_core::credentials::CredentialStore;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::host::{HostClient, HostRuntime, HostServices, read_host_status};
use thriftp_core::repository::MetadataStore;
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};
use uuid::Uuid;

use crate::AppState;

/// Long-lived embedded MCP lifecycle state.
#[derive(Default)]
pub struct AutomationRuntime {
    pub host: Option<HostRuntime>,
    pub store: Option<AutomationStore>,
    pub embedded_mcp: Option<thriftp_mcp::server::LoopbackHandle>,
}

impl AutomationRuntime {
    fn store(&mut self) -> CoreResult<&mut AutomationStore> {
        if self.store.is_none() {
            self.store = Some(AutomationStore::open_default()?);
        }
        self.store
            .as_mut()
            .ok_or_else(|| CoreError::internal("automation store"))
    }
}

fn audit(operation: &str, result: &str) -> CoreResult<()> {
    let store = AuditStore::open_default()?;
    let mut event = thriftp_core::automation::AuditEvent {
        seq: 0,
        timestamp: chrono::Utc::now(),
        interface: "desktop".to_owned(),
        mode: "embedded".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: operation.to_owned(),
        target: None,
        result: result.to_owned(),
        error_code: None,
        duration_ms: None,
        details: None,
        prev_hash: String::new(),
        hash: String::new(),
    };
    store.append(&mut event)
}

fn host_services() -> CoreResult<HostServices> {
    let home = thriftp_core::automation::data_dir();
    Ok(HostServices {
        metadata: Arc::new(JsonMetadataStore::new(home.join("metadata.json")))
            as Arc<dyn MetadataStore>,
        transfers: Arc::new(JsonTransferStore::new(home.join("transfers.json"))),
        credentials: Arc::new(KeyringCredentialStore::new()) as Arc<dyn CredentialStore>,
        factory: Arc::new(ProtocolFactory),
    })
}

#[tauri::command]
pub async fn automation_status(state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let (enabled, mode, clients, grants, scopes, pending_requests) = {
        let store = runtime.store().map_err(|error| error.to_string())?;
        (
            store.state.settings.mcp_enabled,
            store.state.settings.mcp_mode.clone(),
            store.state.clients.clone(),
            store.state.grants.clone(),
            store.state.directory_scopes.clone(),
            store.pending(),
        )
    };
    let run_dir = thriftp_core::automation::data_dir().join("run");
    let host_role = if HostClient::is_available(&run_dir) {
        if runtime.host.as_ref().is_some_and(HostRuntime::is_host) {
            "host"
        } else {
            "observer"
        }
    } else {
        "none"
    };
    Ok(json!({
        "enabled": enabled,
        "mode": mode,
        "hostRole": host_role,
        "clients": clients,
        "grants": grants,
        "scopes": scopes,
        "pendingRequests": pending_requests,
        "sessions": load_sessions(),
        "contractVersion": "1.0.0",
        "protocolVersions": ["2026-07-28", "2025-11-25"],
    }))
}

fn load_sessions() -> Value {
    let path = thriftp_core::automation::data_dir()
        .join("run")
        .join("mcp-sessions.json");
    std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_else(|| json!([]))
}

#[tauri::command]
pub async fn automation_enable(mode: String, state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    {
        let store = runtime.store().map_err(|error| error.to_string())?;
        store.enable(&mode).map_err(|error| error.to_string())?;
    }
    if runtime.host.is_none() {
        let services = host_services().map_err(|error| error.to_string())?;
        let host = HostRuntime::acquire(&mode, services).map_err(|error| error.to_string())?;
        runtime.host = Some(host);
    }
    if (mode == "embedded" || mode == "both") && runtime.embedded_mcp.is_none() {
        let handle =
            thriftp_mcp::server::spawn_loopback(None, None).map_err(|error| error.to_string())?;
        runtime.embedded_mcp = Some(handle);
    }
    drop(runtime);
    audit("mcp.enable", "allowed").map_err(|error| error.to_string())?;
    automation_status(state).await
}

#[tauri::command]
pub async fn automation_disable(state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    if let Some(store) = runtime.store.as_mut() {
        store.disable().map_err(|error| error.to_string())?;
    }
    if let Some(host) = runtime.host.take() {
        host.shutdown();
    }
    if let Some(mut mcp) = runtime.embedded_mcp.take() {
        mcp.shutdown();
    }
    drop(runtime);
    audit("mcp.disable", "allowed").map_err(|error| error.to_string())?;
    automation_status(state).await
}

#[tauri::command]
pub async fn automation_pending_requests(state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    Ok(json!(store.pending()))
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn automation_approve(
    request_id: String,
    profile_ids: Vec<String>,
    directories: Vec<String>,
    local_directories: Vec<String>,
    operations: Vec<String>,
    expires_at: Option<String>,
    disconnect_policy: Option<String>,
    pre_approve: Vec<String>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    let request_id = Uuid::parse_str(&request_id).map_err(|_| "invalid request id".to_owned())?;
    let profiles: Vec<Uuid> = profile_ids
        .iter()
        .map(|value| Uuid::parse_str(value).map_err(|_| "invalid profile id".to_owned()))
        .collect::<Result<_, _>>()?;
    let mut scopes = Vec::new();
    for directory in directories {
        let (profile, path) = directory
            .split_once(':')
            .ok_or_else(|| "directories must be profile:path".to_owned())?;
        scopes.push((
            "remote".to_owned(),
            Some(Uuid::parse_str(profile).map_err(|_| "invalid profile id".to_owned())?),
            path.to_owned(),
        ));
    }
    for path in local_directories {
        scopes.push(("local".to_owned(), None, path));
    }
    let expires_at = expires_at
        .map(|value| {
            chrono::DateTime::parse_from_rfc3339(&value)
                .map(|value| value.with_timezone(&chrono::Utc))
                .map_err(|_| "invalid expiry".to_owned())
        })
        .transpose()?;
    let policy = disconnect_policy.unwrap_or_else(|| "continue".to_owned());
    let grant = store
        .approve(
            request_id, profiles, scopes, operations, expires_at, &policy, "desktop",
        )
        .map_err(|error| error.to_string())?;
    if !pre_approve.is_empty() {
        store
            .add_policy(
                grant.id,
                pre_approve,
                grant.scope_ids.clone(),
                expires_at,
                false,
            )
            .map_err(|error| error.to_string())?;
    }
    audit("mcp.clients.approve", "allowed").map_err(|error| error.to_string())?;
    Ok(json!(grant))
}

#[tauri::command]
pub async fn automation_deny(
    request_id: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    store
        .deny(Uuid::parse_str(&request_id).map_err(|_| "invalid request id".to_owned())?)
        .map_err(|error| error.to_string())?;
    audit("mcp.clients.deny", "allowed").map_err(|error| error.to_string())?;
    Ok(json!({"denied": true}))
}

#[tauri::command]
pub async fn automation_revoke(
    fingerprint: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    store
        .revoke(&fingerprint)
        .map_err(|error| error.to_string())?;
    audit("mcp.clients.revoke", "allowed").map_err(|error| error.to_string())?;
    Ok(json!({"revoked": true, "fingerprint": fingerprint}))
}

#[tauri::command]
pub async fn automation_disconnect(
    fingerprint: String,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    let grant = store
        .grant_for(&fingerprint)
        .cloned()
        .ok_or_else(|| "grant missing".to_owned())?;
    let request = thriftp_core::automation::disconnect::request(&fingerprint)
        .map_err(|error| error.to_string())?;
    audit("mcp.clients.disconnect", "allowed").map_err(|error| error.to_string())?;
    Ok(json!({
        "disconnected": true,
        "fingerprint": fingerprint,
        "requestId": request.request_id,
        "disconnectPolicy": grant.disconnect_policy
    }))
}

#[tauri::command]
pub async fn automation_issue_confirmation(
    fingerprint: String,
    operation: String,
    target: Option<String>,
    ttl: Option<i64>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    let grant = store
        .grant_for(&fingerprint)
        .cloned()
        .ok_or_else(|| "grant missing".to_owned())?;
    let confirmation = thriftp_core::automation::confirmation::issue(
        grant.id,
        &operation,
        target.as_deref(),
        ttl.unwrap_or(120),
    )
    .map_err(|error| error.to_string())?;
    drop(runtime);
    audit("mcp.confirmations.issue", "allowed").map_err(|error| error.to_string())?;
    Ok(json!({
        "confirmationId": confirmation.id,
        "expiresAt": confirmation.expires_at
    }))
}

#[tauri::command]
pub async fn automation_audit_list(limit: Option<usize>) -> Result<Value, String> {
    let store = AuditStore::open_default().map_err(|error| error.to_string())?;
    Ok(json!(
        store
            .list(limit.unwrap_or(200))
            .map_err(|error| error.to_string())?
    ))
}

#[tauri::command]
pub async fn automation_audit_verify() -> Result<Value, String> {
    let store = AuditStore::open_default().map_err(|error| error.to_string())?;
    Ok(json!({"valid": store.verify().map_err(|error| error.to_string())?}))
}

#[tauri::command]
pub async fn automation_retention(days: u32, state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    store.state.settings.audit_retention_days = days.clamp(1, 3_650);
    store.save().map_err(|error| error.to_string())?;
    Ok(json!({"retentionDays": store.state.settings.audit_retention_days}))
}

#[tauri::command]
pub async fn automation_limit(
    bytes: Option<u64>,
    state: State<'_, AppState>,
) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    store.state.settings.audit_max_bytes = bytes;
    store.save().map_err(|error| error.to_string())?;
    Ok(json!({"auditMaxBytes": bytes}))
}

#[tauri::command]
pub async fn automation_apply_retention(state: State<'_, AppState>) -> Result<Value, String> {
    let mut runtime = state.automation.lock().await;
    let store = runtime.store().map_err(|error| error.to_string())?;
    let settings = store.state.settings.clone();
    let audit = AuditStore::open_default().map_err(|error| error.to_string())?;
    let report = apply_retention(
        &audit,
        RetentionPolicy {
            retention_days: settings.audit_retention_days,
            notice_days: settings.audit_notice_days,
            max_bytes: settings.audit_max_bytes,
        },
        chrono::Utc::now(),
    )
    .map_err(|error| error.to_string())?;
    Ok(json!(report))
}

#[tauri::command]
pub async fn automation_host_status() -> Result<Value, String> {
    let run_dir = thriftp_core::automation::data_dir().join("run");
    match read_host_status(&run_dir) {
        Ok(status) => Ok(json!({
            "instanceId": status.instance_id,
            "pid": status.pid,
            "mode": status.mode,
            "transport": status.transport,
            "role": "observer"
        })),
        Err(_) => Ok(json!({"role": "none"})),
    }
}
