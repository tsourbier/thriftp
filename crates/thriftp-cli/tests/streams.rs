//! stdout/stderr separation, quiet/verbose modes, and broken pipes.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

fn raw(home: &TempDir, args: &[&str], stdin: Option<&[u8]>) -> std::process::Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_thriftp"));
    command
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().expect("spawn");
    if let Some(stdin) = stdin {
        child.stdin.as_mut().unwrap().write_all(stdin).unwrap();
    }
    child.wait_with_output().unwrap()
}

#[test]
fn json_success_writes_one_document_to_stdout_only() {
    let home = TempDir::new().unwrap();
    let output = raw(&home, &["--json", "mcp", "status"], None);
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schemaVersion"], 1);
}

#[test]
fn json_failure_writes_stdout_only_and_human_failure_writes_stderr() {
    let home = TempDir::new().unwrap();
    let json = raw(&home, &["--json", "profiles", "show", "missing"], None);
    assert!(!json.status.success());
    assert!(json.stderr.is_empty());
    let value: Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["ok"], false);

    let human = raw(&home, &["profiles", "show", "missing"], None);
    assert!(!human.status.success());
    assert!(human.stdout.is_empty());
    assert!(String::from_utf8_lossy(&human.stderr).contains("error:"));
}

#[test]
fn quiet_suppresses_success_output() {
    let home = TempDir::new().unwrap();
    let output = raw(&home, &["--quiet", "mcp", "status"], None);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
}

#[test]
fn verbose_diagnostics_go_to_stderr() {
    let home = TempDir::new().unwrap();
    let output = raw(&home, &["--verbose", "mcp", "status"], None);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("diagnostic:"));
    assert!(!String::from_utf8_lossy(&output.stdout).contains("diagnostic:"));
}

#[test]
fn a_closed_stdout_pipe_exits_with_code_seven() {
    let home = TempDir::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(["--json", "mcp", "status"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(7));
}
