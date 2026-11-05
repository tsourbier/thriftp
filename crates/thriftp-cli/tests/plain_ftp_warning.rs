//! Plain-FTP warnings appear on profile and transfer results.

use std::process::Command;

use serde_json::Value;
use tempfile::TempDir;
use thriftp_core::model::{Transfer, TransferDirection};
use thriftp_core::repository::{TransferState, TransferStore};
use thriftp_storage_json::JsonTransferStore;

fn run(home: &TempDir, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp");
    serde_json::from_slice(&output.stdout).expect("json")
}

fn run_raw(home: &TempDir, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_thriftp"))
        .env("THRIFTP_HOME", home.path())
        .args(args)
        .output()
        .expect("run thriftp")
}

#[test]
fn plain_ftp_profile_show_emits_an_unencrypted_transport_warning() {
    let home = TempDir::new().unwrap();
    let created = run(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "plain",
            "--protocol",
            "ftp",
            "--host",
            "example.com",
            "--port",
            "21",
            "--username",
            "tester",
        ],
    );
    assert_eq!(
        created["warnings"][0]["code"], "unencrypted_transport",
        "{created}"
    );
    let id = created["data"]["id"].as_str().unwrap();
    let shown = run(&home, &["--json", "profiles", "show", id]);
    assert_eq!(shown["warnings"][0]["code"], "unencrypted_transport");

    let human = run_raw(&home, &["profiles", "show", id]);
    assert!(human.status.success());
    assert!(
        String::from_utf8_lossy(&human.stderr).contains("unencrypted_transport"),
        "{}",
        String::from_utf8_lossy(&human.stderr)
    );
}

#[test]
fn transfer_list_warns_for_plain_ftp_profiles() {
    let home = TempDir::new().unwrap();
    let created = run(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "plain",
            "--protocol",
            "ftp",
            "--host",
            "example.com",
            "--port",
            "21",
            "--username",
            "tester",
        ],
    );
    let profile_id = created["data"]["id"].as_str().unwrap().to_owned();
    let mut state = TransferState::default();
    let mut transfer = Transfer::new(
        TransferDirection::Upload,
        profile_id.parse().unwrap(),
        "/local/file",
        "/remote/file",
        None,
    );
    transfer.state = thriftp_core::model::TransferState::Completed;
    state.transfers.push(transfer);
    JsonTransferStore::new(home.path().join("transfers.json"))
        .save(&state)
        .unwrap();

    let listed = run(&home, &["--json", "transfer", "list"]);
    assert_eq!(listed["warnings"][0]["code"], "unencrypted_transport");
}

#[test]
fn encrypted_profiles_have_no_transport_warning() {
    let home = TempDir::new().unwrap();
    let created = run(
        &home,
        &[
            "--json",
            "profiles",
            "create",
            "--name",
            "secure",
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
    assert_eq!(created["warnings"].as_array().unwrap().len(), 0);
}
