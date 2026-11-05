//! Disabled-by-default, loopback-only binding, and origin rejection.

use std::process::{Command, Stdio};

use tempfile::TempDir;
use thriftp_mcp::transport::{validate_bind, validate_origin};

#[test]
fn disabled_by_default_refuses_to_start() {
    let home = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
        .env("THRIFTP_HOME", home.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("thriftp mcp enable"));
}

#[test]
fn wildcard_and_non_loopback_binds_are_rejected() {
    for bind in ["0.0.0.0:0", "[::]:0", "192.0.2.10:8383"] {
        assert!(validate_bind(bind).is_err(), "{bind}");
    }
    assert!(validate_bind("127.0.0.1:8383").is_ok());
}

#[test]
fn remote_host_and_origin_headers_are_rejected() {
    assert!(validate_origin(Some("https://agent.example"), Some("127.0.0.1:9")).is_err());
    assert!(validate_origin(None, Some("agent.example:9")).is_err());
    assert!(validate_origin(Some("http://localhost:5173"), Some("localhost:9")).is_ok());
}
