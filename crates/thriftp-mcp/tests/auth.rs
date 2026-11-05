use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand_core::OsRng;
use serde_json::{Value, json};
use tempfile::TempDir;
use thriftp_core::automation::{AutomationStore, base64_encode, fingerprint};
use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol};
use thriftp_core::repository::MetadataStore;
use thriftp_storage_json::JsonMetadataStore;

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
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn enrollment_grant_and_authenticated_tool_call() {
    let home = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
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
    let metadata = thriftp_core::repository::AppState {
        connection_profiles: vec![profile.clone()],
        ..Default::default()
    };
    JsonMetadataStore::new(home.path().join("metadata.json"))
        .save(&metadata)
        .unwrap();

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
            vec!["connections.list".to_owned()],
            None,
            "continue",
            "cli",
        )
        .unwrap();

    let mut mcp = Mcp::start(&home);
    mcp.call(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    let challenge = mcp.call(json!({
        "jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"thriftp.automation.challenge","arguments":{"fingerprint":fingerprint}}
    }));
    let nonce = challenge["result"]["structuredContent"]["nonce"]
        .as_str()
        .unwrap()
        .to_owned();
    let signature = signing.sign(nonce.as_bytes());
    let signature = base64::engine::general_purpose::STANDARD.encode(signature.to_bytes());
    let auth = mcp.call(json!({
        "jsonrpc":"2.0","id":3,"method":"tools/call",
        "params":{"name":"thriftp.automation.authenticate","arguments":{
            "fingerprint":fingerprint,
            "nonce":nonce,
            "signature":signature
        }}
    }));
    let token = auth["result"]["structuredContent"]["sessionToken"]
        .as_str()
        .expect("session token")
        .to_owned();

    let listed = mcp.call(json!({
        "jsonrpc":"2.0","id":4,"method":"tools/call",
        "params":{
            "name":"thriftp.connections.list",
            "arguments":{"_meta":{"dev.thriftp/sessionToken":token}}
        }
    }));
    let profiles = listed["result"]["structuredContent"].as_array().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0]["displayName"], "test");
}

#[test]
fn missing_session_is_rejected() {
    let home = TempDir::new().unwrap();
    let mut store =
        thriftp_core::automation::AutomationStore::open(home.path().join("automation.json"))
            .unwrap();
    store.enable("standalone").unwrap();
    let mut mcp = Mcp::start(&home);
    let response = mcp.call(json!({
        "jsonrpc":"2.0","id":1,"method":"tools/call",
        "params":{"name":"thriftp.connections.list","arguments":{}}
    }));
    assert_eq!(response["result"]["isError"], true);
    assert_eq!(
        response["result"]["structuredContent"]["error"]["code"],
        "client_not_enrolled"
    );
}
