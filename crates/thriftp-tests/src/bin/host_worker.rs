//! Test helper that acquires the transfer-host lock and holds it until killed.

use std::sync::Arc;
use std::time::Duration;

use thriftp_core::host::{HostRuntime, HostServices};
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};

fn main() {
    let home = std::env::var("THRIFTP_HOME").unwrap_or_else(|_| {
        eprintln!("host_worker: THRIFTP_HOME is required");
        std::process::exit(2);
    });
    let home = std::path::PathBuf::from(home);
    let run_dir = home.join("run");
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "standalone".to_owned());
    let services = HostServices {
        metadata: Arc::new(JsonMetadataStore::new(home.join("metadata.json"))),
        transfers: Arc::new(JsonTransferStore::new(home.join("transfers.json"))),
        credentials: Arc::new(KeyringCredentialStore::new()),
        factory: Arc::new(ProtocolFactory),
    };
    match HostRuntime::acquire_at(&run_dir, &mode, services) {
        Ok(runtime) => {
            if !runtime.is_host() {
                println!("observer");
                std::process::exit(3);
            }
            println!("ready");
            // Hold ownership until the test kills the process.
            loop {
                std::thread::sleep(Duration::from_millis(100));
            }
            #[allow(unreachable_code)]
            runtime.shutdown();
        }
        Err(error) => {
            println!("conflict: {error}");
            std::process::exit(3);
        }
    }
}
