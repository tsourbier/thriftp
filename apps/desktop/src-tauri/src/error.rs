use serde::Serialize;
use thriftp_core::error::CoreError;

/// Serializable error payload shared with the Svelte UI.
///
/// Messages are deliberately user-facing and redacted: raw protocol
/// diagnostics are logged locally instead of being sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retryable: Option<bool>,
    /// SHA-256 fingerprint presented by the server for host-key failures.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl DesktopError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            field: None,
            retryable: None,
            fingerprint: None,
        }
    }

    #[must_use]
    pub fn from_core(error: &CoreError) -> Self {
        let message = match error {
            CoreError::Validation(message) | CoreError::InvalidArgument(message) => message.clone(),
            CoreError::NotFound(_) => "The requested item could not be found.".to_owned(),
            CoreError::Conflict(message) => message.clone(),
            CoreError::CredentialMissing => {
                "A credential is required to connect to this server.".to_owned()
            }
            CoreError::CredentialRejected => {
                "The server rejected the credential. Check it and try again.".to_owned()
            }
            CoreError::CredentialStoreUnavailable(_) => {
                "The operating-system credential store is unavailable.".to_owned()
            }
            CoreError::AuthFailed(_) => {
                "Authentication failed. Check the credential and that the server accepts this key."
                    .to_owned()
            }
            CoreError::CertificateInvalid(_) => {
                "The server certificate could not be verified.".to_owned()
            }
            CoreError::HostKeyUnknown { .. } => {
                "The server host key is not trusted. Verify its fingerprint before continuing."
                    .to_owned()
            }
            CoreError::HostKeyChanged { .. } => {
                "The server host key has changed. This may indicate interception.".to_owned()
            }
            CoreError::Network(_) => "The server could not be reached.".to_owned(),
            CoreError::Timeout(_) => "The operation timed out.".to_owned(),
            CoreError::PermissionDenied(_) => "Permission denied by the server.".to_owned(),
            CoreError::UnsupportedCapability(message)
            | CoreError::UnsupportedProtocolVersion(message) => message.clone(),
            CoreError::Cancelled => "The operation was cancelled.".to_owned(),
            CoreError::Interrupted => "The transfer was interrupted.".to_owned(),
            CoreError::McpDisabled => "MCP is disabled.".to_owned(),
            CoreError::ClientNotEnrolled => "This client is not enrolled.".to_owned(),
            CoreError::GrantMissing => "The authorization grant is missing.".to_owned(),
            CoreError::GrantExpired => "The authorization grant has expired.".to_owned(),
            CoreError::GrantRevoked => "The authorization grant was revoked.".to_owned(),
            CoreError::ScopeDenied => "The operation is outside the granted scope.".to_owned(),
            CoreError::PathAmbiguous => "The path is ambiguous.".to_owned(),
            CoreError::ConfirmationRequired => "Confirmation is required.".to_owned(),
            CoreError::ConfirmationExpired => "The confirmation has expired.".to_owned(),
            CoreError::AuditUnavailable(_) => "The audit store is unavailable.".to_owned(),
            CoreError::HostUnavailable(_) => "The transfer host is unavailable.".to_owned(),
            CoreError::HostLost => "The transfer host was lost.".to_owned(),
            CoreError::Internal(_) => "An unexpected error occurred.".to_owned(),
        };
        if matches!(error, CoreError::Internal(_)) {
            tracing::warn!(error = %error, "redacted internal error returned to the UI");
        }
        Self {
            code: error.code().to_owned(),
            message,
            field: error.field(),
            retryable: Some(error.retryable()),
            fingerprint: error.host_key_fingerprint().map(str::to_owned),
        }
    }
}

impl From<CoreError> for DesktopError {
    fn from(error: CoreError) -> Self {
        Self::from_core(&error)
    }
}

#[cfg(test)]
mod tests {
    use super::DesktopError;
    use thriftp_core::error::CoreError;

    #[test]
    fn redacts_internal_diagnostics() {
        let error = DesktopError::from(CoreError::internal("socket 10.0.0.1 leaked"));
        assert_eq!(error.code, "internal");
        assert_eq!(error.message, "An unexpected error occurred.");
        assert!(!error.message.contains("10.0.0.1"));
    }

    #[test]
    fn exposes_host_key_fingerprint() {
        let error = DesktopError::from(CoreError::HostKeyUnknown {
            fingerprint: "abc123".to_owned(),
            detail: "unknown key".to_owned(),
        });
        assert_eq!(error.code, "host_key_unknown");
        assert_eq!(error.fingerprint.as_deref(), Some("abc123"));
    }

    #[test]
    fn keeps_distinct_recovery_codes() {
        assert_eq!(
            DesktopError::from(CoreError::CredentialMissing).code,
            "credential_missing"
        );
        assert_eq!(
            DesktopError::from(CoreError::CredentialRejected).code,
            "credential_rejected"
        );
        assert_eq!(
            DesktopError::from(CoreError::HostKeyChanged {
                fingerprint: "abc".to_owned(),
                detail: "line".to_owned(),
            })
            .code,
            "host_key_changed"
        );
        assert_eq!(
            DesktopError::from(CoreError::CertificateInvalid("tls".to_owned())).code,
            "certificate_invalid"
        );
    }

    #[test]
    fn exposes_validation_field() {
        let error = DesktopError::from(CoreError::validation("port must be 1-65535"));
        assert_eq!(error.field.as_deref(), Some("port"));
        assert_eq!(error.code, "validation");
    }
}
