//! Newline-delimited JSON-RPC 2.0 wire types for the local control channel.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{CoreError, CoreResult};

/// Maximum size of one control-channel message (1 MiB).
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

pub const JSONRPC_VERSION: &str = "2.0";

/// Stable JSON-RPC error codes used by the host.
pub mod codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const UNAUTHORIZED: i32 = -32001;
    pub const HOST_LOST: i32 = -32002;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub auth: Option<String>,
    #[serde(default)]
    pub params: Value,
}

impl RpcRequest {
    /// Parse one line, enforcing the 1 MiB cap and the JSON-RPC version.
    pub fn parse(line: &str) -> CoreResult<Self> {
        if line.len() > MAX_MESSAGE_BYTES {
            return Err(CoreError::InvalidArgument(
                "control message exceeds the 1 MiB limit".to_owned(),
            ));
        }
        let request: Self = serde_json::from_str(line)
            .map_err(|error| CoreError::InvalidArgument(format!("malformed request: {error}")))?;
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(CoreError::InvalidArgument(
                "unsupported JSON-RPC version".to_owned(),
            ));
        }
        Ok(request)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl RpcError {
    #[must_use]
    pub fn from_core(error: &CoreError) -> Self {
        let code = match error {
            CoreError::NotFound(_) => codes::INVALID_PARAMS,
            CoreError::InvalidArgument(_) | CoreError::Validation(_) => codes::INVALID_PARAMS,
            CoreError::HostLost => codes::HOST_LOST,
            CoreError::HostUnavailable(_) => codes::HOST_LOST,
            CoreError::UnsupportedCapability(_) => codes::METHOD_NOT_FOUND,
            _ => codes::INTERNAL_ERROR,
        };
        Self {
            code,
            message: crate::observability::redact_text(&error.to_string()),
            data: Some(serde_json::json!({"code": error.code()})),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    #[must_use]
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            result: Some(result),
            error: None,
        }
    }

    #[must_use]
    pub fn failure(id: Option<Value>, error: RpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id,
            result: None,
            error: Some(error),
        }
    }

    pub fn encode(&self) -> CoreResult<String> {
        serde_json::to_string(self)
            .map_err(|error| CoreError::internal(format!("control serialization: {error}")))
    }
}

/// Constant-time comparison for the per-instance host token.
#[must_use]
pub fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let a = left.get(index).copied().unwrap_or(0);
        let b = right.get(index).copied().unwrap_or(0);
        difference |= usize::from(a ^ b);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_and_malformed_messages() {
        assert!(RpcRequest::parse(&"x".repeat(MAX_MESSAGE_BYTES + 1)).is_err());
        assert!(RpcRequest::parse("not json").is_err());
        assert!(RpcRequest::parse(r#"{"jsonrpc":"1.0","method":"x"}"#).is_err());
        assert!(RpcRequest::parse(r#"{"jsonrpc":"2.0","method":"host.status"}"#).is_ok());
    }

    #[test]
    fn constant_time_comparison_is_length_safe() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }
}
