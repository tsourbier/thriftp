//! Shared end-to-end helpers.

#![allow(dead_code)]

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thriftp_core::host::{HostRuntime, HostServices};
use thriftp_core::repository::{TransferState, TransferStore};
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};

/// A spawned process holding the transfer-host lock.
pub struct Worker {
    pub child: Child,
    pub home: PathBuf,
}

impl Worker {
    pub fn spawn(home: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_host_worker"))
            .env("THRIFTP_HOME", home)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn host worker");
        let stdout = child.stdout.take().expect("stdout");
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let _ = reader.read_line(&mut line);
        // Keep the pipe alive for the process lifetime.
        std::mem::forget(reader);
        Self {
            child,
            home: home.to_path_buf(),
        }
    }

    pub fn wait_until_ready(home: &Path) -> bool {
        HostRuntime::is_host_ready(&home.join("run"))
    }

    pub fn home(&self) -> &Path {
        &self.home
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Services backed by JSON stores in a temporary application-data directory.
pub fn services(home: &Path) -> HostServices {
    HostServices {
        metadata: Arc::new(JsonMetadataStore::new(home.join("metadata.json"))),
        transfers: Arc::new(JsonTransferStore::new(home.join("transfers.json"))),
        credentials: Arc::new(KeyringCredentialStore::new()),
        factory: Arc::new(ProtocolFactory),
    }
}

pub fn transfer_store(home: &Path) -> JsonTransferStore {
    JsonTransferStore::new(home.join("transfers.json"))
}

pub fn save_transfers(home: &Path, state: &TransferState) {
    transfer_store(home).save(state).expect("save transfers");
}

pub fn wait_for<F: FnMut() -> bool>(mut predicate: F, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Shared CLI harness for spawned-binary scenarios.
pub mod cli_harness {
    use std::path::PathBuf;
    use std::process::{Command, Output, Stdio};

    #[must_use]
    pub fn binary() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/thriftp")
    }

    pub fn run(home: &std::path::Path, args: &[&str]) -> Output {
        Command::new(binary())
            .env("THRIFTP_HOME", home)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("run thriftp")
    }

    pub fn run_json(home: &std::path::Path, args: &[&str]) -> serde_json::Value {
        let output = run(home, args);
        serde_json::from_slice(&output.stdout).expect("json output")
    }
}

/// Shared MCP stdio harness.
pub mod mcp_harness {
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::process::{Child, Command, Stdio};

    use serde_json::Value;

    pub struct McpSession {
        child: Child,
        reader: BufReader<std::process::ChildStdout>,
    }

    #[must_use]
    pub fn binary() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/debug/thriftp-mcp")
    }

    impl McpSession {
        pub fn start(home: &std::path::Path) -> Self {
            let binary = binary();
            let mut child = Command::new(binary)
                .env("THRIFTP_HOME", home)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("spawn thriftp-mcp");
            let reader = BufReader::new(child.stdout.take().expect("stdout"));
            Self { child, reader }
        }

        pub fn call(&mut self, value: Value) -> Value {
            let stdin = self.child.stdin.as_mut().expect("stdin");
            writeln!(stdin, "{value}").expect("write");
            stdin.flush().expect("flush");
            let mut line = String::new();
            self.reader.read_line(&mut line).expect("read");
            serde_json::from_str(&line).expect("json response")
        }
    }

    impl Drop for McpSession {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
