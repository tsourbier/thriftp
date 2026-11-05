use std::fmt::Write as _;

use serde_json::Value;
use tracing_subscriber::EnvFilter;

/// Replacement text used whenever potentially sensitive material is emitted.
pub const REDACTED: &str = "[redacted]";

/// Initialize structured tracing for the core and application adapters.
///
/// Secrets must never be passed as tracing fields. Callers are responsible for
/// supplying only contextual identifiers and redacted error messages.
pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

/// Redact secret-bearing substrings from a message before it reaches output,
/// logs, errors, or audit records.
#[must_use]
pub fn redact_text(message: &str) -> String {
    let mut output = String::with_capacity(message.len());
    let mut cursor = 0;
    let lowered = message.to_ascii_lowercase();
    let markers = ["password", "passphrase", "secret", "token", "private key"];
    loop {
        let mut best: Option<(usize, &str)> = None;
        for marker in markers {
            if let Some(offset) = lowered[cursor..].find(marker)
                && best.is_none_or(|(best_offset, _)| offset < best_offset)
            {
                best = Some((offset, marker));
            }
        }
        let Some((offset, marker)) = best else {
            break;
        };
        let start = cursor + offset;
        output.push_str(&message[cursor..start]);
        output.push_str(REDACTED);
        let end = start + marker.len();
        let mut value_end = end;
        let bytes = message.as_bytes();
        if value_end < bytes.len() && matches!(bytes[value_end], b'=' | b':') {
            value_end += 1;
            while value_end < bytes.len() && bytes[value_end].is_ascii_whitespace() {
                value_end += 1;
            }
            while value_end < bytes.len()
                && !bytes[value_end].is_ascii_whitespace()
                && !matches!(bytes[value_end], b',' | b';' | b')' | b'"' | b'\'')
            {
                value_end += 1;
            }
        } else if value_end < bytes.len() && !bytes[value_end].is_ascii_whitespace() {
            while value_end < bytes.len()
                && !bytes[value_end].is_ascii_whitespace()
                && !matches!(bytes[value_end], b',' | b';' | b')' | b'"' | b'\'')
            {
                value_end += 1;
            }
        }
        cursor = value_end;
    }
    output.push_str(&message[cursor..]);
    output
}

/// Recursively redact every string in a JSON value.
#[must_use]
pub fn redact_value(value: &Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact_text(text)),
        Value::Array(items) => Value::Array(items.iter().map(redact_value).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), redact_value(value)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Return a stable, secret-free identifier for logs.
#[must_use]
pub fn identifier(value: &str) -> String {
    let mut output = String::with_capacity(value.len().min(16));
    for character in value.chars().take(16) {
        if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
            let _ = write!(output, "{character}");
        } else {
            output.push('_');
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_assignment_and_words() {
        let text = redact_text("password=hunter2 token: abc secret value");
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("abc"));
        assert!(text.contains(REDACTED));
    }

    #[test]
    fn redacts_nested_json() {
        let value = serde_json::json!({"error": "bad password=hunter2", "items": ["token=abc"]});
        let redacted = redact_value(&value).to_string();
        assert!(!redacted.contains("hunter2"));
        assert!(!redacted.contains("abc"));
    }
}
