use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn run(home: &TempDir, args: &[&str]) -> (Value, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    let stderr = String::from_utf8(output.stderr).expect("stderr");
    let value: Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|error| panic!("invalid JSON: {error}\nstdout={stdout}\nstderr={stderr}"));
    (value, stderr)
}

#[test]
fn json_status_has_stable_envelope() {
    let home = TempDir::new().unwrap();
    let (value, stderr) = run(&home, &["--json", "mcp", "status"]);
    assert_eq!(value["schemaVersion"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["exitCode"], 0);
    assert_eq!(value["data"]["enabled"], false);
    assert!(stderr.is_empty(), "stderr should be empty: {stderr}");
}

#[test]
fn creates_and_lists_profiles() {
    let home = TempDir::new().unwrap();
    let (created, _) = run(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "example",
            "--protocol",
            "sftp",
            "--host",
            "example.com",
            "--port",
            "22",
            "--username",
            "tester",
        ],
    );
    assert_eq!(created["ok"], true);
    let (listed, _) = run(&home, &["--json", "profiles", "list"]);
    assert_eq!(listed["ok"], true);
    assert_eq!(listed["data"].as_array().unwrap().len(), 1);
    assert_eq!(listed["data"][0]["displayName"], "example");
}

#[test]
fn browses_local_directory_with_json_envelope() {
    let home = TempDir::new().unwrap();
    let data = TempDir::new().unwrap();
    std::fs::write(data.path().join("alpha.txt"), b"alpha").unwrap();
    let (value, stderr) = run(
        &home,
        &["--json", "browse", data.path().to_str().unwrap(), "--local"],
    );
    assert_eq!(value["ok"], true);
    assert_eq!(value["data"]["entries"].as_array().unwrap().len(), 1);
    assert!(stderr.is_empty());
}

#[test]
fn mcp_enable_and_disable_are_explicit() {
    let home = TempDir::new().unwrap();
    let (enabled, _) = run(&home, &["--json", "mcp", "enable"]);
    assert_eq!(enabled["data"]["enabled"], true);
    let (disabled, _) = run(&home, &["--json", "mcp", "disable"]);
    assert_eq!(disabled["data"]["enabled"], false);
}

#[test]
fn rejects_secret_environment_variables() {
    let home = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .env("THRIFTP_PASSWORD", "sentinel")
        .args(["--json", "mcp", "status"])
        .output()
        .expect("run thriftp");
    assert_eq!(output.status.code(), Some(10));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("sentinel"));
}

#[test]
fn mcp_client_approval_flow_is_visible() {
    use thriftp_core::automation::{AutomationStore, base64_encode};

    let home = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
    let (created, _) = run(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "agent-target",
            "--protocol",
            "sftp",
            "--host",
            "example.com",
            "--port",
            "22",
            "--username",
            "tester",
        ],
    );
    assert_eq!(created["ok"], true);

    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    let request = store.enroll(&base64_encode(&[4u8; 32]), "agent").unwrap();

    let (pending, _) = run(&home, &["--json", "mcp", "clients", "pending"]);
    assert_eq!(pending["data"].as_array().unwrap().len(), 1);

    let (approved, _) = run(
        &home,
        &[
            "--json",
            "mcp",
            "clients",
            "approve",
            &request.id.to_string(),
            "--profile",
            "agent-target",
            "--local-dir",
            scope.path().to_str().unwrap(),
            "--operations",
            "connections.list",
            "--disconnect-policy",
            "cancel",
            "--yes",
        ],
    );
    assert_eq!(approved["ok"], true);
    assert_eq!(approved["data"]["disconnectPolicy"], "cancel");

    let (clients, _) = run(&home, &["--json", "mcp", "clients", "list"]);
    assert_eq!(clients["data"].as_array().unwrap().len(), 1);
    assert_eq!(clients["data"][0]["state"], "active");
}

fn raw(home: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp")
}

#[test]
fn exit_codes_distinguish_usage_validation_conflict_and_not_found() {
    let home = TempDir::new().unwrap();
    let usage = raw(&home, &["not-a-command"]);
    assert_eq!(usage.status.code(), Some(2));

    let validation = raw(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "bad",
            "--protocol",
            "sftp",
            "--host",
            "",
            "--port",
            "22",
            "--username",
            "user",
        ],
    );
    assert_eq!(validation.status.code(), Some(10));

    let conflict = raw(&home, &["--json", "delete", "/tmp/does-not-matter"]);
    assert_eq!(conflict.status.code(), Some(6));

    let missing = raw(&home, &["--json", "profiles", "show", "missing-profile"]);
    assert_eq!(missing.status.code(), Some(4));
}

#[test]
fn version_reports_contract_and_protocol_revisions() {
    let home = TempDir::new().unwrap();
    let output = raw(&home, &["--version"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("automation contract"));
    assert!(text.contains("2026-07-28"));
}

#[test]
fn documented_commands_and_exit_codes_are_present() {
    let docs = include_str!("../../../docs/cli.md");
    for command in [
        "profiles", "browse", "stat", "upload", "download", "mkdir", "rename", "delete",
        "transfer", "queue", "config", "mcp",
    ] {
        assert!(docs.contains(command), "missing command docs: {command}");
    }
    for code in std::iter::once(0).chain(2..=13) {
        assert!(
            docs.contains(&format!("| {code} |")),
            "missing exit code {code}"
        );
    }
}

#[test]
fn error_output_is_redacted() {
    let home = TempDir::new().unwrap();
    let (value, stderr) = run(&home, &["--json", "profiles", "show", "missing"]);
    assert_eq!(value["ok"], false);
    let serialized = value.to_string();
    assert!(!serialized.contains("password"));
    assert!(!serialized.contains("secret"));
    assert!(!stderr.contains("password"));
    assert!(!stderr.contains("secret"));
}
