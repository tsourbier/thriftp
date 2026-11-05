//! Envelope snapshots for success and failure categories.

use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn run(home: &TempDir, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp");
    serde_json::from_slice(&output.stdout).expect("json")
}

fn snapshot() -> Value {
    serde_json::from_str(include_str!("snapshots/envelope.json")).unwrap()
}

#[test]
fn success_envelope_matches_the_snapshot_shape() {
    let home = TempDir::new().unwrap();
    let value = run(&home, &["--json", "mcp", "status"]);
    let expected = snapshot();
    for key in expected["envelopeKeys"].as_array().unwrap() {
        assert!(value.get(key.as_str().unwrap()).is_some(), "missing {key}");
    }
    for key in expected["metaKeys"].as_array().unwrap() {
        assert!(value["meta"].get(key.as_str().unwrap()).is_some());
    }
    assert_eq!(value["warnings"], serde_json::json!([]));
}

#[test]
fn failure_envelope_matches_the_snapshot_shape() {
    let home = TempDir::new().unwrap();
    let value = run(&home, &["--json", "profiles", "show", "missing"]);
    let expected = snapshot();
    for key in expected["envelopeKeys"].as_array().unwrap() {
        assert!(value.get(key.as_str().unwrap()).is_some(), "missing {key}");
    }
    for key in expected["errorKeys"].as_array().unwrap() {
        assert!(value["error"].get(key.as_str().unwrap()).is_some());
    }
    assert_eq!(value["exitCode"], 4);
}
