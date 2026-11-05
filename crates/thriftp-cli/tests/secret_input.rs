//! Stdin-only secret input and forbidden secret environment variables.

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::Value;
use tempfile::TempDir;

fn run_with_stdin(home: &TempDir, args: &[&str], stdin: &[u8]) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn thriftp");
    child.stdin.as_mut().unwrap().write_all(stdin).unwrap();
    child.wait_with_output().unwrap()
}

fn create_args(name: &str) -> Vec<&str> {
    vec![
        "--json",
        "profiles",
        "create",
        "--name",
        name,
        "--protocol",
        "sftp",
        "--host",
        "example.com",
        "--port",
        "22",
        "--username",
        "tester",
    ]
}

#[test]
fn forbidden_secret_environment_variables_are_rejected() {
    let home = TempDir::new().unwrap();
    for variable in ["THRIFTP_PASSWORD", "THRIFTP_KEY_PASSPHRASE"] {
        let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
            .env("THRIFTP_HOME", home.path())
            .env(variable, "sentinel-secret")
            .args(["--json", "mcp", "status"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(10));
        let serialized = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!serialized.contains("sentinel-secret"));
    }
}

#[test]
fn empty_stdin_secret_is_a_credential_error() {
    let home = TempDir::new().unwrap();
    let mut args = create_args("empty-secret");
    args.push("--password-stdin");
    let output = run_with_stdin(&home, &args, b"");
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
    assert!(stderr.contains("credential"), "{stderr}");
}

#[test]
fn set_credential_requires_an_explicit_stdin_mode() {
    let home = TempDir::new().unwrap();
    let created = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(create_args("no-stdin-mode"))
        .output()
        .unwrap();
    let profile: Value = serde_json::from_slice(&created.stdout).unwrap();
    let id = profile["data"]["id"].as_str().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(["--json", "profiles", "set-credential", id])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn missing_stdin_never_prompts_on_a_tty() {
    let home = TempDir::new().unwrap();
    let mut args = create_args("no-prompt");
    args.push("--key-passphrase-stdin");
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(&args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let serialized = String::from_utf8_lossy(&output.stdout).to_lowercase();
    assert!(!serialized.contains("password:"));
}

#[test]
fn operational_commands_accept_the_global_stdin_credential() {
    let home = TempDir::new().unwrap();
    let created = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args([
            "--json",
            "profiles",
            "create",
            "--name",
            "local",
            "--protocol",
            "sftp",
            "--host",
            "127.0.0.1",
            "--port",
            "9",
            "--username",
            "tester",
            "--timeout",
            "1",
        ])
        .output()
        .unwrap();
    let profile: Value = serde_json::from_slice(&created.stdout).unwrap();
    let id = profile["data"]["id"].as_str().unwrap();
    let output = run_with_stdin(
        &home,
        &["--json", "--password-stdin", "profiles", "test", id],
        b"one-shot-secret\n",
    );
    // The connection is refused, but the credential was accepted rather than
    // missing (exit 3) and no prompt was shown.
    assert_eq!(output.status.code(), Some(4));
    let serialized = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!serialized.contains("one-shot-secret"));
}
