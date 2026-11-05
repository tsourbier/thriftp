//! Multi-process ownership and stale-lock scenarios.

#[path = "../support/mod.rs"]
mod support;

use std::time::Duration;

use support::{Worker, save_transfers, services};
use tempfile::TempDir;
use thriftp_core::host::{HostClient, HostRuntime};

#[test]
fn exactly_one_process_acquires_ownership() {
    let home = TempDir::new().unwrap();
    let first = Worker::spawn(home.path());
    assert!(HostClient::is_available(&home.path().join("run")));

    let mut second = Worker::spawn(home.path());
    let mut third = Worker::spawn(home.path());
    assert!(
        HostRuntime::is_host_ready(&home.path().join("run")),
        "the original host must stay alive"
    );
    second.child.wait().unwrap();
    third.child.wait().unwrap();

    drop((first, second, third));
    assert!(support::wait_for(
        || !HostRuntime::is_host_ready(&home.path().join("run")),
        Duration::from_secs(5)
    ));
}

#[test]
fn stale_lock_is_cleaned_by_the_next_host() {
    let home = TempDir::new().unwrap();
    {
        let worker = Worker::spawn(home.path());
        assert!(HostRuntime::is_host_ready(&home.path().join("run")));
        drop(worker);
    }
    // The worker was killed without a clean release; the next owner recovers.
    let recovered =
        HostRuntime::acquire_at(home.path().join("run"), "standalone", services(home.path()))
            .expect("acquire after host loss");
    assert!(recovered.is_host());
    recovered.shutdown();
}

#[test]
fn a_live_owner_is_never_stolen() {
    let home = TempDir::new().unwrap();
    let worker = Worker::spawn(home.path());
    let observer =
        HostRuntime::acquire_at(home.path().join("run"), "standalone", services(home.path()))
            .unwrap();
    assert!(!observer.is_host());
    assert!(HostRuntime::is_host_ready(&home.path().join("run")));
    drop(worker);
    save_transfers(home.path(), &Default::default());
}
