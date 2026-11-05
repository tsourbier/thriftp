//! Stable CLI JSON envelope and stream discipline.

use std::io::Write;

use serde_json::{Value, json};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::observability::redact_text;

pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const AUTOMATION_CONTRACT_VERSION: &str = "1.0.0";
pub const MCP_PROTOCOL_VERSIONS: &str = "2026-07-28, 2025-11-25";

pub fn envelope(command: &str, result: CoreResult<Value>) -> (Value, i32) {
    match result {
        Ok(mut data) => {
            let warnings = hoist_warnings(&mut data);
            (
                json!({
                    "schemaVersion": 1,
                    "command": command,
                    "ok": true,
                    "exitCode": 0,
                    "data": data,
                    "warnings": warnings,
                    "error": null,
                    "meta": meta(),
                }),
                0,
            )
        }
        Err(error) => {
            let code = exit_code(&error);
            (
                json!({
                    "schemaVersion": 1,
                    "command": command,
                    "ok": false,
                    "exitCode": code,
                    "data": null,
                    "warnings": [],
                    "error": {
                        "code": error.code(),
                        "message": redact(&error.to_string()),
                        "retryable": error.retryable(),
                        "details": {},
                    },
                    "meta": meta(),
                }),
                code,
            )
        }
    }
}

/// Extract an object's `warnings` array into the envelope-level warnings field.
fn hoist_warnings(data: &mut Value) -> Value {
    let Some(object) = data.as_object_mut() else {
        return json!([]);
    };
    object
        .remove("warnings")
        .filter(Value::is_array)
        .unwrap_or_else(|| json!([]))
}

pub fn meta() -> Value {
    json!({
        "cliVersion": CLI_VERSION,
        "automationContractVersion": AUTOMATION_CONTRACT_VERSION,
        "mcpProtocolVersions": MCP_PROTOCOL_VERSIONS,
        "hostMode": "local",
    })
}

pub fn exit_code(error: &CoreError) -> i32 {
    error.cli_exit_code()
}

pub fn redact(message: &str) -> String {
    redact_text(message)
}

/// Emit the result with exactly one JSON document on stdout and diagnostics on
/// stderr. Returns the process exit code.
pub fn emit(value: &Value, json: bool, quiet: bool, verbose: bool) -> i32 {
    let code = value.get("exitCode").and_then(Value::as_i64).unwrap_or(13) as i32;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    if json {
        let encoded = match serde_json::to_string_pretty(value) {
            Ok(encoded) => encoded,
            Err(error) => {
                eprintln!("error: could not serialize result: {error}");
                return 13;
            }
        };
        if write_document(&mut handle, &encoded).is_err() {
            return 7;
        }
    } else if code == 0 {
        if !quiet && write_document(&mut handle, &human(value)).is_err() {
            return 7;
        }
    } else if !quiet && let Some(error) = value.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("failed");
        eprintln!("error: {message}");
    }
    if !quiet && let Some(warnings) = value.get("warnings").and_then(Value::as_array) {
        for warning in warnings {
            let code = warning
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("warning");
            let message = warning
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default();
            eprintln!("warning: {code}: {message}");
        }
    }
    if verbose {
        eprintln!(
            "diagnostic: command={} exitCode={code}",
            value
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
        );
    }
    code
}

fn write_document(handle: &mut impl Write, text: &str) -> std::io::Result<()> {
    handle.write_all(text.as_bytes())?;
    if !text.ends_with('\n') {
        handle.write_all(b"\n")?;
    }
    handle.flush()
}

#[must_use]
pub fn human(value: &Value) -> String {
    let data = value.get("data").unwrap_or(value);
    if let Some(object) = data.as_object() {
        let mut output = String::new();
        for (key, value) in object {
            output.push_str(&format!("{key}: {}\n", render(value)));
        }
        output
    } else {
        format!("{}\n", render(data))
    }
}

fn render(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => "-".to_owned(),
        Value::Array(items) => items.iter().map(render).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_maps_to_a_documented_exit_code() {
        let errors = [
            CoreError::Validation("x".to_owned()),
            CoreError::NotFound("x".to_owned()),
            CoreError::Conflict("x".to_owned()),
            CoreError::CredentialMissing,
            CoreError::CredentialRejected,
            CoreError::CredentialStoreUnavailable("x".to_owned()),
            CoreError::Network("x".to_owned()),
            CoreError::PermissionDenied("x".to_owned()),
            CoreError::Cancelled,
            CoreError::Interrupted,
            CoreError::McpDisabled,
            CoreError::ScopeDenied,
            CoreError::ConfirmationRequired,
            CoreError::UnsupportedCapability("x".to_owned()),
            CoreError::Internal("x".to_owned()),
        ];
        for error in errors {
            let code = exit_code(&error);
            assert!((2..=13).contains(&code), "{} -> {code}", error.code());
        }
    }

    #[test]
    fn warnings_move_to_the_envelope() {
        let (value, code) = envelope(
            "browse",
            Ok(json!({"entries": [], "warnings": [{"code": "unencrypted_transport"}]})),
        );
        assert_eq!(code, 0);
        assert_eq!(value["warnings"][0]["code"], "unencrypted_transport");
        assert!(value["data"].get("warnings").is_none());
    }

    #[test]
    fn error_envelopes_are_redacted() {
        let (value, code) = envelope(
            "profiles",
            Err(CoreError::internal("password=hunter2 leaked")),
        );
        assert_eq!(code, 13);
        assert!(!value.to_string().contains("hunter2"));
    }
}
