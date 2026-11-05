//! Recursive local operations, conflict behavior, and symlink handling.

use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;

fn run(home: &TempDir, args: &[&str]) -> (Value, i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp");
    let value = serde_json::from_slice(&output.stdout).unwrap_or(Value::Null);
    (
        value,
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn recursive_local_delete_preserves_symlink_targets() {
    let home = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let nested = root.path().join("a").join("b");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("file.txt"), b"data").unwrap();
    let outside = TempDir::new().unwrap();
    std::fs::write(outside.path().join("keep.txt"), b"keep").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path().join("keep.txt"), root.path().join("link")).unwrap();

    let (value, code, stderr) = run(
        &home,
        &[
            "--json",
            "delete",
            root.path().to_str().unwrap(),
            "--local",
            "--recursive",
            "--yes",
        ],
    );
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(value["ok"], true);
    assert!(!root.path().exists());
    assert!(outside.path().join("keep.txt").exists());
}

#[test]
fn local_mkdir_and_rename_are_idempotent_safe() {
    let home = TempDir::new().unwrap();
    let root = TempDir::new().unwrap();
    let target = root.path().join("one").join("two");
    let (created, code, _) = run(
        &home,
        &["--json", "mkdir", target.to_str().unwrap(), "--local"],
    );
    assert_eq!(code, 0);
    assert_eq!(
        created["data"]["created"],
        target.to_string_lossy().as_ref()
    );
    let renamed = root.path().join("renamed");
    let (value, code, _) = run(
        &home,
        &[
            "--json",
            "rename",
            target.to_str().unwrap(),
            renamed.to_str().unwrap(),
            "--local",
        ],
    );
    assert_eq!(code, 0, "{value}");
    assert!(renamed.exists());
}

#[test]
fn missing_remote_profile_fails_validation_before_connecting() {
    let home = TempDir::new().unwrap();
    let (_value, code, _) = run(
        &home,
        &["--json", "upload", "/tmp/file", "/remote", "--recursive"],
    );
    assert_eq!(code, 10);
}

#[test]
fn no_wait_without_a_host_fails_with_guidance_and_starts_nothing() {
    let home = TempDir::new().unwrap();
    let (value, code, _) = run(
        &home,
        &[
            "--json",
            "--profile",
            "missing",
            "upload",
            "/tmp/file",
            "/remote",
            "--no-wait",
        ],
    );
    assert_eq!(code, 4);
    assert_eq!(value["error"]["code"], "not_found");
    assert!(!home.path().join("run").join("host.lock").exists());
}
