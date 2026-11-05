//! MCP automation identity, grants, canonical scopes, and audit.
//!
//! The module is Tauri-independent. Persistence lives in a single versioned
//! JSON document; audit records live in append-only daily JSONL segments.

pub mod approval;
pub mod confirmation;
pub mod disconnect;
pub mod retention;
pub mod service;

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};

pub const SERVICE_NAME: &str = "dev.thriftp.credentials";
pub const AUTOMATION_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_RETENTION_DAYS: u32 = 90;
pub const DEFAULT_NOTICE_DAYS: u32 = 7;
pub const DEFAULT_AUDIT_MAX_BYTES: u64 = 1024 * 1024 * 1024;

/// Resolve the per-user application data directory, honoring `THRIFTP_HOME`.
#[must_use]
pub fn data_dir() -> PathBuf {
    if let Ok(home) = std::env::var("THRIFTP_HOME") {
        return PathBuf::from(home);
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("Thriftp");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Thriftp");
        }
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("thriftp");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("thriftp");
    }
    std::env::temp_dir().join("thriftp")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationSettings {
    pub mcp_enabled: bool,
    pub mcp_mode: String,
    pub loopback_port: Option<u16>,
    pub audit_retention_days: u32,
    pub audit_notice_days: u32,
    pub audit_max_bytes: Option<u64>,
    pub enrollment_request_ttl_seconds: i64,
    pub confirmation_ttl_seconds: i64,
    pub max_pending_enrollment_requests: usize,
    pub max_pending_enrollment_requests_per_fingerprint: usize,
    pub max_enrolled_clients: usize,
    pub updated_at: DateTime<Utc>,
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            mcp_enabled: false,
            mcp_mode: "standalone".to_owned(),
            loopback_port: None,
            audit_retention_days: DEFAULT_RETENTION_DAYS,
            audit_notice_days: DEFAULT_NOTICE_DAYS,
            audit_max_bytes: Some(DEFAULT_AUDIT_MAX_BYTES),
            enrollment_request_ttl_seconds: 600,
            confirmation_ttl_seconds: 120,
            max_pending_enrollment_requests: 10,
            max_pending_enrollment_requests_per_fingerprint: 3,
            max_enrolled_clients: 100,
            updated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClient {
    pub id: Uuid,
    pub fingerprint: String,
    pub public_key: String,
    pub display_name: String,
    pub state: String,
    pub enrolled_at: Option<DateTime<Utc>>,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnrollmentRequest {
    pub id: Uuid,
    pub fingerprint: String,
    pub public_key: String,
    pub display_name: String,
    pub state: String,
    pub requested_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub decided_at: Option<DateTime<Utc>>,
    pub decided_by: Option<String>,
    pub grant_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryScope {
    pub id: Uuid,
    pub grant_id: Uuid,
    pub kind: String,
    pub profile_id: Option<Uuid>,
    pub canonical_path: String,
    pub recursive: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalPolicy {
    pub id: Uuid,
    pub grant_id: Uuid,
    pub scope_ids: Vec<Uuid>,
    pub operations: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub single_use: bool,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationGrant {
    pub id: Uuid,
    pub client_fingerprint: String,
    pub revision: u64,
    pub state: String,
    pub profile_ids: Vec<Uuid>,
    pub scope_ids: Vec<Uuid>,
    pub operations: Vec<String>,
    pub disconnect_policy: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revocation_epoch: u64,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationState {
    pub schema_version: u32,
    pub state_revision: u64,
    pub settings: AutomationSettings,
    pub clients: Vec<McpClient>,
    pub enrollment_requests: Vec<EnrollmentRequest>,
    pub grants: Vec<AuthorizationGrant>,
    pub directory_scopes: Vec<DirectoryScope>,
    pub approval_policies: Vec<ApprovalPolicy>,
}

impl Default for AutomationState {
    fn default() -> Self {
        Self {
            schema_version: AUTOMATION_SCHEMA_VERSION,
            state_revision: 1,
            settings: AutomationSettings::default(),
            clients: Vec::new(),
            enrollment_requests: Vec::new(),
            grants: Vec::new(),
            directory_scopes: Vec::new(),
            approval_policies: Vec::new(),
        }
    }
}

pub struct AutomationStore {
    path: PathBuf,
    pub state: AutomationState,
}

impl AutomationStore {
    pub fn open_default() -> CoreResult<Self> {
        Self::open(data_dir().join("automation.json"))
    }

    pub fn open(path: impl Into<PathBuf>) -> CoreResult<Self> {
        let path = path.into();
        let state = if path.is_file() {
            let bytes = fs::read(&path).map_err(io_error)?;
            serde_json::from_slice(&bytes).map_err(|error| {
                CoreError::internal(format!("failed to parse automation state: {error}"))
            })?
        } else {
            AutomationState::default()
        };
        Ok(Self { path, state })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save(&mut self) -> CoreResult<()> {
        self.state.state_revision = self.state.state_revision.saturating_add(1);
        self.state.settings.updated_at = Utc::now();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let bytes = serde_json::to_vec_pretty(&self.state).map_err(|error| {
            CoreError::internal(format!("failed to serialize automation: {error}"))
        })?;
        let temporary = self.path.with_extension("tmp");
        fs::write(&temporary, bytes).map_err(io_error)?;
        fs::rename(&temporary, &self.path).map_err(io_error)
    }

    pub fn status(&self) -> serde_json::Value {
        serde_json::json!({
            "enabled": self.state.settings.mcp_enabled,
            "mode": self.state.settings.mcp_mode,
            "clients": self.state.clients.len(),
            "pendingRequests": self.state.enrollment_requests.iter().filter(|r| r.state == "pending").count(),
            "activeGrants": self.state.grants.iter().filter(|g| g.state == "active").count(),
        })
    }

    pub fn enable(&mut self, mode: &str) -> CoreResult<()> {
        if mode != "standalone" && mode != "embedded" && mode != "both" {
            return Err(CoreError::validation(
                "mode must be standalone, embedded, or both",
            ));
        }
        self.state.settings.mcp_enabled = true;
        self.state.settings.mcp_mode = mode.to_owned();
        self.save()
    }

    pub fn disable(&mut self) -> CoreResult<()> {
        self.state.settings.mcp_enabled = false;
        self.save()
    }

    pub fn enroll(
        &mut self,
        public_key: &str,
        display_name: &str,
    ) -> CoreResult<EnrollmentRequest> {
        if self
            .state
            .enrollment_requests
            .iter()
            .filter(|r| r.state == "pending")
            .count()
            >= self.state.settings.max_pending_enrollment_requests
        {
            return Err(CoreError::Conflict(
                "too many pending enrollment requests".to_owned(),
            ));
        }
        let fingerprint = fingerprint(public_key)?;
        let request = EnrollmentRequest {
            id: Uuid::new_v4(),
            fingerprint: fingerprint.clone(),
            public_key: public_key.to_owned(),
            display_name: display_name.to_owned(),
            state: "pending".to_owned(),
            requested_at: Utc::now(),
            expires_at: Utc::now()
                + chrono::Duration::seconds(self.state.settings.enrollment_request_ttl_seconds),
            decided_at: None,
            decided_by: None,
            grant_id: None,
        };
        self.state.enrollment_requests.push(request.clone());
        self.save()?;
        Ok(request)
    }

    pub fn pending(&self) -> Vec<EnrollmentRequest> {
        self.state
            .enrollment_requests
            .iter()
            .filter(|request| request.state == "pending")
            .cloned()
            .collect()
    }

    pub fn deny(&mut self, request_id: Uuid) -> CoreResult<()> {
        let request = self
            .state
            .enrollment_requests
            .iter_mut()
            .find(|request| request.id == request_id)
            .ok_or_else(|| CoreError::NotFound("enrollment request".to_owned()))?;
        if request.state != "pending" {
            return Err(CoreError::Conflict(
                "enrollment request is no longer pending".to_owned(),
            ));
        }
        request.state = "denied".to_owned();
        request.decided_at = Some(Utc::now());
        request.decided_by = Some("cli".to_owned());
        self.save()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn approve(
        &mut self,
        request_id: Uuid,
        profile_ids: Vec<Uuid>,
        scopes: Vec<(String, Option<Uuid>, String)>,
        operations: Vec<String>,
        expires_at: Option<DateTime<Utc>>,
        disconnect_policy: &str,
        approved_by: &str,
    ) -> CoreResult<AuthorizationGrant> {
        if profile_ids.is_empty() {
            return Err(CoreError::validation("at least one profile is required"));
        }
        if operations.is_empty() {
            return Err(CoreError::validation("at least one operation is required"));
        }
        if disconnect_policy != "continue" && disconnect_policy != "cancel" {
            return Err(CoreError::validation(
                "disconnect policy must be continue or cancel",
            ));
        }
        let request = self
            .state
            .enrollment_requests
            .iter_mut()
            .find(|request| request.id == request_id)
            .ok_or_else(|| CoreError::NotFound("enrollment request".to_owned()))?;
        if request.state != "pending" {
            return Err(CoreError::Conflict(
                "enrollment request is no longer pending".to_owned(),
            ));
        }
        let now = Utc::now();
        let client_id = Uuid::new_v4();
        let grant_id = Uuid::new_v4();
        let mut scope_ids = Vec::new();
        for (kind, profile_id, path) in scopes {
            let canonical = if kind == "local" {
                canonical_local(Path::new(&path))?
            } else {
                canonical_remote(&path)?
            };
            let scope = DirectoryScope {
                id: Uuid::new_v4(),
                grant_id,
                kind,
                profile_id,
                canonical_path: canonical,
                recursive: true,
                created_at: now,
            };
            scope_ids.push(scope.id);
            self.state.directory_scopes.push(scope);
        }
        if self
            .state
            .directory_scopes
            .iter()
            .all(|scope| scope.grant_id != grant_id)
        {
            return Err(CoreError::validation(
                "at least one directory scope is required",
            ));
        }
        let grant = AuthorizationGrant {
            id: grant_id,
            client_fingerprint: request.fingerprint.clone(),
            revision: 1,
            state: "active".to_owned(),
            profile_ids,
            scope_ids,
            operations,
            disconnect_policy: disconnect_policy.to_owned(),
            created_at: now,
            updated_at: now,
            expires_at,
            revoked_at: None,
            revocation_epoch: 0,
            last_used_at: None,
        };
        self.state.grants.push(grant.clone());
        if let Some(client) = self
            .state
            .clients
            .iter_mut()
            .find(|client| client.fingerprint == request.fingerprint)
        {
            client.state = "active".to_owned();
            client.enrolled_at = Some(now);
            client.updated_at = now;
        } else {
            self.state.clients.push(McpClient {
                id: client_id,
                fingerprint: request.fingerprint.clone(),
                public_key: request.public_key.clone(),
                display_name: request.display_name.clone(),
                state: "active".to_owned(),
                enrolled_at: Some(now),
                last_seen_at: None,
                revoked_at: None,
                created_at: now,
                updated_at: now,
            });
        }
        request.state = "approved".to_owned();
        request.decided_at = Some(now);
        request.decided_by = Some(approved_by.to_owned());
        request.grant_id = Some(grant_id);
        self.save()?;
        Ok(grant)
    }

    pub fn grant_for(&self, fingerprint: &str) -> Option<&AuthorizationGrant> {
        self.state
            .grants
            .iter()
            .find(|grant| grant.client_fingerprint == fingerprint && grant.state == "active")
    }

    pub fn scopes_for(&self, grant_id: Uuid) -> Vec<DirectoryScope> {
        self.state
            .directory_scopes
            .iter()
            .filter(|scope| scope.grant_id == grant_id)
            .cloned()
            .collect()
    }

    pub fn revoke(&mut self, fingerprint: &str) -> CoreResult<()> {
        let mut found = false;
        for grant in &mut self.state.grants {
            if grant.client_fingerprint == fingerprint && grant.state == "active" {
                grant.state = "revoked".to_owned();
                grant.revoked_at = Some(Utc::now());
                grant.revocation_epoch += 1;
                found = true;
            }
        }
        if !found {
            return Err(CoreError::NotFound("active grant".to_owned()));
        }
        for client in &mut self.state.clients {
            if client.fingerprint == fingerprint {
                client.state = "revoked".to_owned();
                client.revoked_at = Some(Utc::now());
            }
        }
        self.save()
    }

    pub fn clients(&self) -> Vec<McpClient> {
        self.state.clients.clone()
    }

    pub fn grant_view(&self, fingerprint: &str) -> Option<serde_json::Value> {
        let grant = self.grant_for(fingerprint)?;
        Some(serde_json::json!({
            "grantId": grant.id,
            "revision": grant.revision,
            "epoch": grant.revocation_epoch,
            "expiresAt": grant.expires_at,
            "profiles": grant.profile_ids,
            "operations": grant.operations,
            "disconnectPolicy": grant.disconnect_policy,
            "scopes": self.scopes_for(grant.id),
        }))
    }

    /// Add a scoped pre-approval policy to an active grant.
    pub fn add_policy(
        &mut self,
        grant_id: Uuid,
        operations: Vec<String>,
        scope_ids: Vec<Uuid>,
        expires_at: Option<DateTime<Utc>>,
        single_use: bool,
    ) -> CoreResult<ApprovalPolicy> {
        if operations.is_empty() {
            return Err(CoreError::validation(
                "at least one pre-approved operation is required",
            ));
        }
        let grant = self
            .state
            .grants
            .iter()
            .find(|grant| grant.id == grant_id && grant.state == "active")
            .ok_or_else(|| CoreError::NotFound("active grant".to_owned()))?;
        let policy = ApprovalPolicy {
            id: Uuid::new_v4(),
            grant_id: grant.id,
            scope_ids,
            operations,
            created_at: Utc::now(),
            expires_at,
            revoked_at: None,
            single_use,
            last_used_at: None,
        };
        self.state.approval_policies.push(policy.clone());
        self.save()?;
        Ok(policy)
    }

    /// Check a policy and record its use when it applies.
    pub fn policy_allows(&mut self, grant_id: Uuid, operation: &str) -> CoreResult<bool> {
        let now = Utc::now();
        let Some(policy) = self.state.approval_policies.iter_mut().find(|policy| {
            policy.grant_id == grant_id
                && policy.revoked_at.is_none()
                && policy.expires_at.is_none_or(|expires| now <= expires)
                && policy.operations.iter().any(|allowed| allowed == operation)
                && !(policy.single_use && policy.last_used_at.is_some())
        }) else {
            return Ok(false);
        };
        policy.last_used_at = Some(now);
        self.save()?;
        Ok(true)
    }

    /// Revoke every policy attached to a grant.
    pub fn revoke_policies(&mut self, grant_id: Uuid) {
        let now = Utc::now();
        for policy in &mut self.state.approval_policies {
            if policy.grant_id == grant_id {
                policy.revoked_at = Some(now);
            }
        }
    }
}

pub fn fingerprint(public_key_b64: &str) -> CoreResult<String> {
    let bytes = base64_decode(public_key_b64)?;
    if bytes.len() != 32 {
        return Err(CoreError::validation("Ed25519 public key must be 32 bytes"));
    }
    let digest = Sha256::digest(&bytes);
    Ok(hex(&digest))
}

pub fn new_challenge() -> (String, String) {
    let nonce = Uuid::new_v4().to_string();
    let challenge_id = Uuid::new_v4().to_string();
    (challenge_id, nonce)
}

pub fn canonical_local(path: &Path) -> CoreResult<String> {
    let canonical = fs::canonicalize(path).map_err(io_error)?;
    if !canonical.is_absolute() {
        return Err(CoreError::validation("local path must be absolute"));
    }
    Ok(canonical.to_string_lossy().into_owned())
}

pub fn canonical_remote(path: &str) -> CoreResult<String> {
    if !path.starts_with('/') {
        return Err(CoreError::validation("remote path must be absolute"));
    }
    if path.contains('\\') || path.contains('\0') {
        return Err(CoreError::validation(
            "remote path contains an invalid separator",
        ));
    }
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("%2e") || lowered.contains("%2f") || lowered.contains("%5c") {
        return Err(CoreError::validation("encoded traversal is not allowed"));
    }
    if path.split('/').any(|part| part == "..") {
        return Err(CoreError::validation("traversal is not allowed"));
    }
    let mut parts = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::RootDir | Component::Prefix(_) => {}
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(CoreError::validation("traversal is not allowed"));
            }
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
        }
    }
    Ok(format!("/{}", parts.join("/")))
}

#[must_use]
pub fn path_in_scope(path: &str, scope: &str) -> bool {
    if path.contains('\\') || path.split('/').any(|part| part == "..") {
        return false;
    }
    let lowered = path.to_ascii_lowercase();
    if lowered.contains("%2e") || lowered.contains("%2f") || lowered.contains("%5c") {
        return false;
    }
    if scope == "/" {
        return path.starts_with('/');
    }
    path == scope || path.starts_with(&format!("{scope}/"))
}

pub fn base64_decode(value: &str) -> CoreResult<Vec<u8>> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|_| CoreError::validation("invalid base64"))
}

#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[must_use]
pub fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

/// Run startup/daily audit maintenance: retention, storage limits, and notice.
///
/// Callers should invoke this at startup. A returned `audit_unavailable` error
/// means no safe deletion was possible and MCP operations must fail closed.
pub fn maintain_audit() -> CoreResult<retention::RetentionReport> {
    let marker = data_dir().join("run").join("audit-maintained");
    if marker
        .metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|elapsed| elapsed < Duration::from_secs(24 * 60 * 60))
    {
        return Ok(retention::RetentionReport::default());
    }
    let settings = AutomationStore::open_default()?.state.settings;
    let audit = AuditStore::open_default()?;
    let report = retention::apply_retention(
        &audit,
        retention::RetentionPolicy {
            retention_days: settings.audit_retention_days,
            notice_days: settings.audit_notice_days,
            max_bytes: settings.audit_max_bytes,
        },
        Utc::now(),
    )?;
    if let Some(parent) = marker.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&marker, Utc::now().to_rfc3339());
    Ok(report)
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::internal(format!("I/O error: {error}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEvent {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub interface: String,
    pub mode: String,
    pub client_fingerprint: Option<String>,
    pub client_display_name: Option<String>,
    pub session_id: Option<String>,
    pub operation: String,
    pub target: Option<serde_json::Value>,
    pub result: String,
    pub error_code: Option<String>,
    pub duration_ms: Option<u64>,
    pub details: Option<serde_json::Value>,
    pub prev_hash: String,
    pub hash: String,
}

pub struct AuditStore {
    directory: PathBuf,
}

/// Metadata for one append-only audit segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditSegment {
    pub path: PathBuf,
    pub date: String,
    pub bytes: u64,
    pub sealed: bool,
}

struct AuditLock {
    path: PathBuf,
}

impl AuditLock {
    fn acquire(directory: &Path) -> CoreResult<Self> {
        let path = directory.join("audit.lock");
        for _ in 0..200 {
            match OpenOptions::new().create_new(true).write(true).open(&path) {
                Ok(mut file) => {
                    let _ = writeln!(file, "{}", std::process::id());
                    return Ok(Self { path });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    // A stale lock from a crashed writer is removed after two
                    // seconds; normal appends hold the lock for microseconds.
                    if fs::metadata(&path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .and_then(|modified| modified.elapsed().ok())
                        .is_some_and(|elapsed| elapsed > Duration::from_secs(2))
                    {
                        let _ = fs::remove_file(&path);
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    return Err(CoreError::audit_unavailable(format!(
                        "audit lock is unavailable: {error}"
                    )));
                }
            }
        }
        Err(CoreError::audit_unavailable(
            "timed out waiting for the audit lock",
        ))
    }
}

impl Drop for AuditLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn audit_io_error(error: std::io::Error) -> CoreError {
    CoreError::audit_unavailable(format!("audit I/O error: {error}"))
}

impl AuditStore {
    pub fn open_default() -> CoreResult<Self> {
        Ok(Self {
            directory: data_dir().join("audit"),
        })
    }

    pub fn open(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn append(&self, event: &mut AuditEvent) -> CoreResult<()> {
        fs::create_dir_all(&self.directory).map_err(|error| {
            CoreError::audit_unavailable(format!("audit directory is unavailable: {error}"))
        })?;
        let _guard = AuditLock::acquire(&self.directory)?;
        let date = event.timestamp.format("%Y-%m-%d").to_string();
        let path = self.directory.join(format!("{date}.jsonl"));
        let (seq, prev_hash) = self.last_record_for(&path)?;
        event.seq = seq + 1;
        event.prev_hash = prev_hash;
        event.hash = String::new();
        event.hash = hash_event(event)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                CoreError::audit_unavailable(format!("audit segment is unavailable: {error}"))
            })?;
        let line = serde_json::to_vec(event)
            .map_err(|error| CoreError::internal(format!("audit serialization: {error}")))?;
        file.write_all(&line)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| {
                CoreError::audit_unavailable(format!(
                    "audit record could not be persisted: {error}"
                ))
            })
    }

    /// Return the append-only segment files in chronological order.
    pub fn segments(&self) -> CoreResult<Vec<AuditSegment>> {
        if !self.directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut paths: Vec<PathBuf> = fs::read_dir(&self.directory)
            .map_err(audit_io_error)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        paths.sort();
        let mut segments = Vec::new();
        for path in paths {
            let bytes = fs::metadata(&path).map_err(audit_io_error)?.len();
            let date = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_owned();
            segments.push(AuditSegment {
                path,
                date,
                bytes,
                sealed: true,
            });
        }
        Ok(segments)
    }

    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }

    /// Read the retention checkpoint that anchors verification after old
    /// segments are deliberately deleted.
    #[must_use]
    pub fn read_checkpoint(&self) -> Option<(u64, String)> {
        let bytes = fs::read(self.directory.join("audit.checkpoint")).ok()?;
        let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
        Some((
            value.get("seq")?.as_u64()?,
            value.get("hash")?.as_str()?.to_owned(),
        ))
    }

    /// Persist a checkpoint before deleting old segments.
    pub fn write_checkpoint(&self, seq: u64, hash: &str) -> CoreResult<()> {
        fs::create_dir_all(&self.directory).map_err(audit_io_error)?;
        let bytes = serde_json::to_vec(&serde_json::json!({"seq": seq, "hash": hash}))
            .map_err(|error| CoreError::internal(format!("audit checkpoint: {error}")))?;
        fs::write(self.directory.join("audit.checkpoint"), bytes).map_err(audit_io_error)
    }

    fn last_record_for(&self, current_path: &Path) -> CoreResult<(u64, String)> {
        // Chain across segments: use the newest existing segment at or before
        // the current date so a new day never resets the hash chain.
        let mut candidates: Vec<PathBuf> = fs::read_dir(&self.directory)
            .map_err(audit_io_error)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .filter(|path| path <= current_path)
            .collect();
        candidates.sort();
        for path in candidates.iter().rev() {
            let contents = fs::read_to_string(path).map_err(audit_io_error)?;
            if let Some(last) = contents.lines().rfind(|line| !line.trim().is_empty()) {
                let event: AuditEvent = serde_json::from_str(last).map_err(|error| {
                    CoreError::audit_unavailable(format!("audit parse: {error}"))
                })?;
                if let Some((checkpoint_seq, checkpoint_hash)) = self.read_checkpoint()
                    && checkpoint_seq > event.seq
                {
                    return Ok((checkpoint_seq, checkpoint_hash));
                }
                return Ok((event.seq, event.hash));
            }
        }
        Ok(self
            .read_checkpoint()
            .unwrap_or_else(|| (0, "0".repeat(64))))
    }

    pub fn verify(&self) -> CoreResult<bool> {
        let (mut previous, mut expected_seq) = self
            .read_checkpoint()
            .map_or_else(|| ("0".repeat(64), 1), |(seq, hash)| (hash, seq + 1));
        let mut entries: Vec<_> = fs::read_dir(&self.directory)
            .map_err(io_error)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        entries.sort();
        for path in entries {
            let contents = fs::read_to_string(&path).map_err(io_error)?;
            for line in contents.lines().filter(|line| !line.trim().is_empty()) {
                let event: AuditEvent = serde_json::from_str(line)
                    .map_err(|error| CoreError::internal(format!("audit parse: {error}")))?;
                if event.seq != expected_seq || event.prev_hash != previous {
                    return Ok(false);
                }
                let mut copy = event.clone();
                copy.hash.clear();
                if hash_event(&copy)? != event.hash {
                    return Ok(false);
                }
                previous = event.hash;
                expected_seq += 1;
            }
        }
        Ok(true)
    }

    pub fn list(&self, limit: usize) -> CoreResult<Vec<AuditEvent>> {
        let mut events = Vec::new();
        if !self.directory.is_dir() {
            return Ok(events);
        }
        let mut entries: Vec<_> = fs::read_dir(&self.directory)
            .map_err(io_error)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
            .collect();
        entries.sort();
        for path in entries {
            let contents = fs::read_to_string(&path).map_err(io_error)?;
            for line in contents.lines().filter(|line| !line.trim().is_empty()) {
                if let Ok(event) = serde_json::from_str(line) {
                    events.push(event);
                }
            }
        }
        if events.len() > limit {
            events = events.split_off(events.len() - limit);
        }
        Ok(events)
    }
}

fn hash_event(event: &AuditEvent) -> CoreResult<String> {
    let bytes = serde_json::to_vec(event)
        .map_err(|error| CoreError::internal(format!("audit hash: {error}")))?;
    Ok(hex(&Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_hex() {
        let key = base64_encode(&[7u8; 32]);
        let first = fingerprint(&key).unwrap();
        let second = fingerprint(&key).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[test]
    fn scope_containment_uses_boundaries() {
        assert!(path_in_scope("/srv/app/file", "/srv/app"));
        assert!(!path_in_scope("/srv/app2/file", "/srv/app"));
        assert!(!path_in_scope("/srv/app/../etc", "/srv/app"));
    }

    #[test]
    fn remote_canonicalization_rejects_traversal() {
        assert!(canonical_remote("/srv/app/../etc").is_err());
        assert!(canonical_remote("/srv/%2e%2e/etc").is_err());
        assert_eq!(
            canonical_remote("/srv/app/./file").unwrap(),
            "/srv/app/file"
        );
    }

    fn base64_encode(bytes: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }
}
