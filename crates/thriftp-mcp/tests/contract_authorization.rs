//! Live authorization, confirmation, expiry, and audit coverage.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use base64::Engine;
use chrono::{Duration, Utc};
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use serde_json::{Value, json};
use tempfile::TempDir;
use thriftp_core::automation::{
    AuditEvent, AuditStore, AutomationStore, base64_encode, fingerprint,
};
use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol};
use thriftp_core::repository::{AppState, MetadataStore};
use thriftp_storage_json::JsonMetadataStore;
use uuid::Uuid;

struct Mcp {
    child: Child,
    stdout: BufReader<std::process::ChildStdout>,
}

impl Mcp {
    fn start(home: &TempDir) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
            .env("THRIFTP_HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn mcp");
        let stdout = BufReader::new(child.stdout.take().expect("stdout"));
        Self { child, stdout }
    }

    fn call(&mut self, value: Value) -> Value {
        let stdin = self.child.stdin.as_mut().expect("stdin");
        writeln!(stdin, "{value}").expect("write");
        stdin.flush().expect("flush");
        let mut line = String::new();
        self.stdout.read_line(&mut line).expect("read");
        serde_json::from_str(&line).expect("json")
    }

    fn authenticate(&mut self, signing: &SigningKey, fingerprint: &str) -> String {
        self.call(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
        let challenge = self.call(json!({
            "jsonrpc":"2.0","id":2,"method":"tools/call",
            "params":{"name":"thriftp.automation.challenge","arguments":{"fingerprint":fingerprint}}
        }));
        let nonce = challenge["result"]["structuredContent"]["nonce"]
            .as_str()
            .unwrap()
            .to_owned();
        let signature = base64::engine::general_purpose::STANDARD
            .encode(signing.sign(nonce.as_bytes()).to_bytes());
        let auth = self.call(json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"thriftp.automation.authenticate","arguments":{
                "fingerprint":fingerprint,"nonce":nonce,"signature":signature
            }}
        }));
        auth["result"]["structuredContent"]["sessionToken"]
            .as_str()
            .expect("session token")
            .to_owned()
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Fixture {
    home: TempDir,
    signing: SigningKey,
    fingerprint: String,
    profile_id: Uuid,
}

fn fixture(operations: Vec<&str>, expires_at: Option<chrono::DateTime<Utc>>) -> Fixture {
    let home = TempDir::new().unwrap();
    let signing = SigningKey::generate(&mut OsRng);
    let public_key = base64_encode(signing.verifying_key().as_bytes());
    let fingerprint = fingerprint(&public_key).unwrap();
    let profile = ConnectionProfile::new(
        "test",
        Protocol::Sftp,
        "example.com",
        22,
        "tester",
        AuthMethod::Password,
        30,
    );
    let metadata = AppState {
        connection_profiles: vec![profile.clone()],
        ..Default::default()
    };
    JsonMetadataStore::new(home.path().join("metadata.json"))
        .save(&metadata)
        .unwrap();
    let scope = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let request = store.enroll(&public_key, "agent").unwrap();
    store
        .approve(
            request.id,
            vec![profile.id],
            vec![(
                "local".to_owned(),
                None,
                scope.path().to_string_lossy().into_owned(),
            )],
            operations.into_iter().map(str::to_owned).collect(),
            expires_at,
            "continue",
            "cli",
        )
        .unwrap();
    Fixture {
        home,
        signing,
        fingerprint,
        profile_id: profile.id,
    }
}

#[test]
fn revocation_from_another_process_is_observed_before_the_next_call() {
    let fixture = fixture(vec!["connections.list"], None);
    let mut mcp = Mcp::start(&fixture.home);
    let token = mcp.authenticate(&fixture.signing, &fixture.fingerprint);
    let allowed = mcp.call(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"thriftp.connections.list","arguments":{
            "_meta":{"dev.thriftp/sessionToken":token}
        }}
    }));
    assert_eq!(allowed["result"]["isError"], false);

    let mut other = AutomationStore::open(fixture.home.path().join("automation.json")).unwrap();
    other.revoke(&fixture.fingerprint).unwrap();

    let denied = mcp.call(json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"thriftp.connections.list","arguments":{
            "_meta":{"dev.thriftp/sessionToken":token}
        }}
    }));
    assert_eq!(
        denied["result"]["structuredContent"]["error"]["code"],
        "grant_revoked"
    );
}

#[test]
fn expired_grant_is_rejected_before_the_next_operation() {
    let fixture = fixture(
        vec!["connections.list"],
        Some(Utc::now() - Duration::seconds(5)),
    );
    let mut mcp = Mcp::start(&fixture.home);
    let response = mcp.call(json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"thriftp.automation.challenge","arguments":{
            "fingerprint":fixture.fingerprint
        }}
    }));
    assert_eq!(
        response["result"]["structuredContent"]["error"]["code"],
        "grant_expired"
    );
    let _ = &fixture.signing;
}

#[test]
fn destructive_calls_require_a_policy_in_headless_mode() {
    let fixture = fixture(vec!["files.delete"], None);
    let mut mcp = Mcp::start(&fixture.home);
    let token = mcp.authenticate(&fixture.signing, &fixture.fingerprint);
    let blocked = mcp.call(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"thriftp.files.delete","arguments":{
            "profileId": fixture.profile_id,
            "path": "/srv/app/file",
            "_meta":{"dev.thriftp/sessionToken":token}
        }}
    }));
    assert_eq!(
        blocked["result"]["structuredContent"]["error"]["code"],
        "confirmation_required"
    );

    let mut other = AutomationStore::open(fixture.home.path().join("automation.json")).unwrap();
    let grant = other.grant_for(&fixture.fingerprint).unwrap().clone();
    other
        .add_policy(
            grant.id,
            vec!["files.delete".to_owned()],
            grant.scope_ids.clone(),
            None,
            false,
        )
        .unwrap();

    let after_policy = mcp.call(json!({
        "jsonrpc":"2.0","id":5,"method":"tools/call",
        "params":{"name":"thriftp.files.delete","arguments":{
            "profileId": fixture.profile_id,
            "path": "/srv/app/file",
            "_meta":{"dev.thriftp/sessionToken":token}
        }}
    }));
    // The policy bypassed confirmation; the remote scope then denied the path.
    assert_eq!(
        after_policy["result"]["structuredContent"]["error"]["code"],
        "scope_denied"
    );
}

#[test]
fn audit_records_include_target_and_client_identity() {
    let fixture = fixture(vec!["connections.get"], None);
    let mut mcp = Mcp::start(&fixture.home);
    let token = mcp.authenticate(&fixture.signing, &fixture.fingerprint);
    mcp.call(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{"name":"thriftp.connections.get","arguments":{
            "profileId": fixture.profile_id,
            "_meta":{"dev.thriftp/sessionToken":token}
        }}
    }));
    let store = AuditStore::open(fixture.home.path().join("audit"));
    let events: Vec<AuditEvent> = store.list(usize::MAX).unwrap();
    let event = events
        .iter()
        .find(|event| event.operation == "thriftp.connections.get")
        .expect("audit record");
    assert_eq!(
        event.client_fingerprint.as_deref(),
        Some(fixture.fingerprint.as_str())
    );
    assert!(event.target.is_some());
    assert!(event.duration_ms.is_some());
}
