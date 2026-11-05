//! Stable MCP protocol and domain error mapping.

use serde_json::{Value, json};
use thriftp_core::error::CoreError;

use crate::transport::SUPPORTED_REVISIONS;

/// JSON-RPC code used for unsupported protocol revisions.
pub const UNSUPPORTED_PROTOCOL_VERSION: i32 = -32022;

/// Build the structured unsupported-version error.
#[must_use]
pub fn unsupported_version(version: &str) -> CoreError {
    CoreError::UnsupportedProtocolVersion(format!(
        "{version}; supported revisions: {}",
        SUPPORTED_REVISIONS.join(", ")
    ))
}

/// Map a core error to a JSON-RPC error object.
#[must_use]
pub fn rpc_error(error: &CoreError) -> Value {
    let code = if matches!(error, CoreError::UnsupportedProtocolVersion(_)) {
        UNSUPPORTED_PROTOCOL_VERSION
    } else {
        -32603
    };
    json!({
        "code": code,
        "message": thriftp_core::observability::redact_text(&error.to_string()),
        "data": {"code": error.code()}
    })
}

/// Map a core error to the tool-result error envelope.
#[must_use]
pub fn tool_error(error: &CoreError) -> Value {
    json!({
        "schemaVersion": "1.0.0",
        "ok": false,
        "data": null,
        "warnings": [],
        "error": {
            "code": error.code(),
            "message": thriftp_core::observability::redact_text(&error.to_string()),
            "retryable": error.retryable(),
            "details": {}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_versions_list_supported_revisions() {
        let error = unsupported_version("1900-01-01");
        let value = rpc_error(&error);
        assert_eq!(value["code"], UNSUPPORTED_PROTOCOL_VERSION);
        assert!(value["message"].as_str().unwrap().contains("2026-07-28"));
    }

    #[test]
    fn tool_errors_are_redacted() {
        let error = CoreError::Internal("password=hunter2".to_owned());
        assert!(!tool_error(&error).to_string().contains("hunter2"));
    }
}
