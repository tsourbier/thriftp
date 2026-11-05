//! Shared-state visibility across host and control clients.

#[path = "../support/mod.rs"]
mod support;

use support::{Worker, services};
use tempfile::TempDir;
use thriftp_core::host::{HostClient, HostRuntime};
use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol};
use thriftp_core::repository::MetadataStore;
use thriftp_storage_json::JsonMetadataStore;

#[test]
fn control_client_observes_the_same_queue_and_transfers() {
    let home = TempDir::new().unwrap();
    let profile = ConnectionProfile::new(
        "agent",
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

    let worker = Worker::spawn(home.path());
    let mut client = HostClient::connect_at(home.path().join("run")).unwrap();
    let status = client.status().unwrap();
    assert_eq!(status["role"], "host");

    let submitted = client
        .submit(serde_json::json!({
            "direction": "upload",
            "profileId": profile.id,
            "sources": ["/local/file"],
            "destination": "/remote/file",
            "recursive": false,
            "conflictPolicy": "fail",
            "priority": "normal",
            "execute": false
        }))
        .unwrap();
    assert_eq!(submitted["state"], "pending");
    let listed = client.list_transfers(None).unwrap();
    assert_eq!(listed["transfers"].as_array().unwrap().len(), 1);

    let observer =
        HostRuntime::acquire_at(home.path().join("run"), "standalone", services(home.path()))
            .unwrap();
    assert!(!observer.is_host());
    drop(worker);
}
