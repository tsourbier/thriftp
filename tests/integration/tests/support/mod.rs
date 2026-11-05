//! Shared fixtures for cross-crate integration tests.

#![allow(dead_code)]

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

pub fn profile(protocol: Protocol, host: &str, port: u16) -> ConnectionProfile {
    ConnectionProfile::new(
        "integration",
        protocol,
        host,
        port,
        "tester",
        AuthMethod::Password,
        30,
    )
}
