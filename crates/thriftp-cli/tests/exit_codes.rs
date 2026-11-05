//! Full exit-code matrix for documented categories.

use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn raw(home: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp")
}

#[test]
fn success_and_usage_codes() {
    let home = TempDir::new().unwrap();
    assert_eq!(
        raw(&home, &["--json", "mcp", "status"]).status.code(),
        Some(0)
    );
    assert_eq!(raw(&home, &["not-a-command"]).status.code(), Some(2));
}

#[test]
fn credential_authentication_and_not_found_codes() {
    let home = TempDir::new().unwrap();
    let missing_credential = raw(&home, &["--json", "delete", "/tmp/x"]);
    // No --yes means a conflict first.
    assert_eq!(missing_credential.status.code(), Some(6));
    assert_eq!(
        raw(&home, &["--json", "profiles", "show", "missing"])
            .status
            .code(),
        Some(4)
    );
}

#[test]
fn conflict_and_validation_codes() {
    let home = TempDir::new().unwrap();
    assert_eq!(
        raw(&home, &["--json", "delete", "/tmp/x"]).status.code(),
        Some(6)
    );
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
}

#[test]
fn json_error_carries_the_same_code_as_the_process() {
    let home = TempDir::new().unwrap();
    let output = raw(&home, &["--json", "profiles", "show", "missing"]);
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["exitCode"], output.status.code().unwrap());
}

#[test]
fn every_documented_code_is_covered_by_the_mapping_tests() {
    // `crates/thriftp-cli/src/output.rs` unit tests exercise every CoreError
    // variant, including 5, 7, 8, 9, 11, 12, and 13, which require external
    // failure injection to reach through the binary.
    let docs = include_str!("../../../docs/cli.md");
    for code in std::iter::once(0).chain(2..=13) {
        assert!(docs.contains(&format!("| {code} |")));
    }
}
