use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use thriftp_core::model::{AuthMethod, ConnectionProfile, Protocol};

pub fn integration_enabled() -> bool {
    std::env::var("THRIFTP_INTEGRATION").is_ok_and(|value| value == "1")
}

pub fn env_port(name: &str, default: u16) -> u16 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

pub fn env_string(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_owned())
}

/// Resolve a fixture path relative to the workspace root.
///
/// Cargo runs integration tests with the crate directory as the working
/// directory, so relative repository paths must be anchored explicitly.
pub fn fixture_path(value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path)
    }
}

pub fn profile(
    protocol: Protocol,
    host: &str,
    port: u16,
    username: &str,
    auth_method: AuthMethod,
    ssh_key_ref: Option<&str>,
) -> ConnectionProfile {
    let mut profile = ConnectionProfile::new(
        "integration",
        protocol,
        host,
        port,
        username,
        auth_method,
        30,
    );
    profile.ssh_key_ref = ssh_key_ref.map(str::to_owned);
    profile
}

pub fn unique_name() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("thriftp-{}-{nanos}", std::process::id())
}
