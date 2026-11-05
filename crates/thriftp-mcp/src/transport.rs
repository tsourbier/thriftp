//! Local-only transports for the MCP server.
//!
//! Standalone mode uses stdio with stdout reserved for protocol messages.
//! Embedded mode uses a loopback-only Streamable HTTP endpoint with a
//! per-launch bearer token. Wildcard and non-loopback binds are rejected.

use std::net::SocketAddr;

use thriftp_core::error::{CoreError, CoreResult};

pub const SUPPORTED_REVISIONS: [&str; 2] = ["2026-07-28", "2025-11-25"];

/// Validate a requested bind address: only loopback addresses are permitted.
pub fn validate_bind(bind: &str) -> CoreResult<SocketAddr> {
    let address: SocketAddr = bind.parse().map_err(|_| {
        CoreError::invalid_argument(format!("invalid bind address {bind}; loopback is required"))
    })?;
    if !address.ip().is_loopback() {
        return Err(CoreError::invalid_argument(
            "MCP may only bind to a loopback address".to_owned(),
        ));
    }
    Ok(address)
}

/// Validate HTTP `Host` and `Origin` headers against the loopback boundary.
pub fn validate_origin(origin: Option<&str>, host: Option<&str>) -> CoreResult<()> {
    if let Some(host) = host
        && !is_loopback_authority(host)
    {
        return Err(CoreError::PermissionDenied(
            "non-loopback Host header rejected".to_owned(),
        ));
    }
    if let Some(origin) = origin {
        let authority = origin
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(origin);
        let authority = authority.split('/').next().unwrap_or(authority);
        if !is_loopback_authority(authority) {
            return Err(CoreError::PermissionDenied(
                "non-loopback Origin header rejected".to_owned(),
            ));
        }
    }
    Ok(())
}

/// Whether an authority is a loopback host with an optional port.
pub fn is_loopback_authority(authority: &str) -> bool {
    let host = if authority.starts_with('[') {
        authority
            .split_once(']')
            .map(|(host, _)| format!("{host}]"))
            .unwrap_or_else(|| authority.to_owned())
    } else {
        authority.split(':').next().unwrap_or(authority).to_owned()
    };
    host == "localhost"
        || host == "[::1]"
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

/// Generate a per-launch bearer token.
#[must_use]
pub fn new_bearer_token() -> String {
    format!("{}{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
}

/// Run the loopback HTTP transport.
pub fn run_loopback(port: Option<u16>, token_file: Option<std::path::PathBuf>) -> CoreResult<()> {
    crate::server::run_loopback(port, token_file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_wildcard_and_remote_binds() {
        assert!(validate_bind("0.0.0.0:0").is_err());
        assert!(validate_bind("[::]:0").is_err());
        assert!(validate_bind("192.168.1.10:9000").is_err());
        assert!(validate_bind("127.0.0.1:0").is_ok());
        assert!(validate_bind("[::1]:0").is_ok());
    }

    #[test]
    fn rejects_remote_origins_and_hosts() {
        assert!(validate_origin(Some("https://evil.example"), Some("127.0.0.1:9000")).is_err());
        assert!(validate_origin(None, Some("evil.example:9000")).is_err());
        assert!(validate_origin(Some("http://localhost:5173"), Some("localhost:9000")).is_ok());
        assert!(validate_origin(Some("http://127.0.0.1:5173"), Some("127.0.0.1:9000")).is_ok());
    }

    #[test]
    fn loopback_authority_handles_ipv6() {
        assert!(is_loopback_authority("[::1]:9000"));
        assert!(!is_loopback_authority("[2001:db8::1]:9000"));
    }
}
