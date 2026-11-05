use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;
use thriftp_core::automation::AutomationStore;

fn enable(home: &TempDir) {
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
}

fn request(home: &TempDir, lines: &[Value]) -> Vec<Value> {
    enable(home);
    let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
        .env("THRIFTP_HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mcp");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        for line in lines {
            writeln!(stdin, "{line}").expect("write request");
        }
    }
    let output = child.wait_with_output().expect("mcp output");
    String::from_utf8(output.stdout)
        .expect("stdout")
        .lines()
        .map(|line| serde_json::from_str(line).expect("json response"))
        .collect()
}

#[test]
fn initialize_and_discover_support_both_revisions() {
    let home = TempDir::new().unwrap();
    let responses = request(
        &home,
        &[
            serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
            serde_json::json!({"jsonrpc":"2.0","id":2,"method":"server/discover","params":{}}),
        ],
    );
    assert_eq!(responses.len(), 2);
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-11-25");
    let versions = responses[1]["result"]["protocolVersions"]
        .as_array()
        .unwrap();
    assert!(versions.iter().any(|value| value == "2026-07-28"));
}

#[test]
fn lists_tools_before_enrollment() {
    let home = TempDir::new().unwrap();
    let responses = request(
        &home,
        &[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})],
    );
    let tools = responses[0]["result"]["tools"].as_array().unwrap();
    assert!(
        tools
            .iter()
            .any(|tool| tool["name"] == "thriftp.connections.list")
    );
    assert!(
        tools
            .iter()
            .any(|tool| tool["name"] == "thriftp.automation.enroll")
    );
}

#[test]
fn disabled_mcp_refuses_to_start() {
    let home = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
        .env("THRIFTP_HOME", home.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("thriftp mcp enable"), "{stderr}");
}

#[test]
fn unsupported_protocol_revision_returns_structured_error() {
    let home = TempDir::new().unwrap();
    let responses = request(
        &home,
        &[serde_json::json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"server/discover",
            "params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"1900-01-01"}}
        })],
    );
    assert_eq!(responses[0]["error"]["code"], -32022);
    assert!(
        responses[0]["error"]["message"]
            .as_str()
            .unwrap()
            .contains("2026-07-28")
    );
}

#[test]
fn documented_tool_catalog_matches_tools_list() {
    let home = TempDir::new().unwrap();
    let responses = request(
        &home,
        &[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}})],
    );
    let docs = include_str!("../../../docs/mcp.md");
    for tool in responses[0]["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        assert!(docs.contains(name), "missing documentation for {name}");
    }
}
