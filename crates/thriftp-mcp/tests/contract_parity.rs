//! Standalone/embedded and cross-surface parity contracts.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

use serde_json::json;
use tempfile::TempDir;
use thriftp_mcp::transport::SUPPORTED_REVISIONS;

fn enable(home: &TempDir) {
    let mut store =
        thriftp_core::automation::AutomationStore::open(home.path().join("automation.json"))
            .unwrap();
    store.enable("standalone").unwrap();
}

fn request(home: &TempDir, value: serde_json::Value) -> serde_json::Value {
    enable(home);
    let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
        .env("THRIFTP_HOME", home.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    writeln!(child.stdin.as_mut().unwrap(), "{value}").unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let _ = child.kill();
    let _ = child.wait();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn discovery_reports_the_same_revisions_for_initialize_and_discover() {
    let home = TempDir::new().unwrap();
    let discover = request(
        &home,
        json!({"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}),
    );
    let versions = discover["result"]["protocolVersions"].as_array().unwrap();
    for revision in SUPPORTED_REVISIONS {
        assert!(versions.iter().any(|value| value == revision));
    }
    let initialize = request(
        &home,
        json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}),
    );
    assert!(
        SUPPORTED_REVISIONS.contains(&initialize["result"]["protocolVersion"].as_str().unwrap())
    );
}

#[test]
fn every_tool_declares_a_2020_12_schema_and_destructive_hint() {
    let home = TempDir::new().unwrap();
    let listed = request(
        &home,
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}),
    );
    for tool in listed["result"]["tools"].as_array().unwrap() {
        assert_eq!(
            tool["inputSchema"]["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            tool["outputSchema"]["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert!(tool["annotations"]["destructiveHint"].is_boolean());
        assert!(tool["inputSchema"]["additionalProperties"] == false);
    }
}
