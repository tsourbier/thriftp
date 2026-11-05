use thiserror::Error;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("credential is required")]
    CredentialMissing,
    #[error("credential was rejected")]
    CredentialRejected,
    #[error("credential store is unavailable: {0}")]
    CredentialStoreUnavailable(String),
    #[error("authentication failed: {0}")]
    AuthFailed(String),
    #[error("certificate is invalid: {0}")]
    CertificateInvalid(String),
    #[error("server host key is unknown: {detail}")]
    HostKeyUnknown {
        /// SHA-256 fingerprint presented by the server, when available.
        fingerprint: String,
        /// Redacted diagnostic detail.
        detail: String,
    },
    #[error("server host key has changed: {detail}")]
    HostKeyChanged {
        /// SHA-256 fingerprint presented by the server, when available.
        fingerprint: String,
        /// Redacted diagnostic detail.
        detail: String,
    },
    #[error("network error: {0}")]
    Network(String),
    #[error("operation timed out: {0}")]
    Timeout(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("operation is unsupported: {0}")]
    UnsupportedCapability(String),
    #[error("unsupported protocol version: {0}")]
    UnsupportedProtocolVersion(String),
    #[error("cancelled")]
    Cancelled,
    #[error("transfer interrupted")]
    Interrupted,
    #[error("MCP is disabled")]
    McpDisabled,
    #[error("client is not enrolled")]
    ClientNotEnrolled,
    #[error("grant is missing")]
    GrantMissing,
    #[error("grant has expired")]
    GrantExpired,
    #[error("grant was revoked")]
    GrantRevoked,
    #[error("operation is outside the granted scope")]
    ScopeDenied,
    #[error("path is ambiguous")]
    PathAmbiguous,
    #[error("confirmation is required")]
    ConfirmationRequired,
    #[error("confirmation has expired")]
    ConfirmationExpired,
    #[error("audit store is unavailable: {0}")]
    AuditUnavailable(String),
    #[error("transfer host is unavailable: {0}")]
    HostUnavailable(String),
    #[error("transfer host was lost")]
    HostLost,
    #[error("internal error: {0}")]
    Internal(String),
}

impl CoreError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    pub fn audit_unavailable(message: impl Into<String>) -> Self {
        Self::AuditUnavailable(message.into())
    }

    /// Rebuild an error from a stable contract code, used across the local
    /// control channel and the MCP adapters.
    #[must_use]
    pub fn from_code(code: &str, message: impl Into<String>) -> Self {
        let message = message.into();
        match code {
            "validation" => Self::Validation(message),
            "invalid_argument" => Self::InvalidArgument(message),
            "not_found" => Self::NotFound(message),
            "conflict" => Self::Conflict(message),
            "credential_missing" => Self::CredentialMissing,
            "credential_rejected" => Self::CredentialRejected,
            "credential_store_unavailable" => Self::CredentialStoreUnavailable(message),
            "auth_failed" => Self::AuthFailed(message),
            "certificate_invalid" => Self::CertificateInvalid(message),
            "host_key_unknown" => Self::HostKeyUnknown {
                fingerprint: String::new(),
                detail: message,
            },
            "host_key_changed" => Self::HostKeyChanged {
                fingerprint: String::new(),
                detail: message,
            },
            "network" => Self::Network(message),
            "timeout" => Self::Timeout(message),
            "permission_denied" => Self::PermissionDenied(message),
            "unsupported_capability" => Self::UnsupportedCapability(message),
            "unsupported_protocol_version" => Self::UnsupportedProtocolVersion(message),
            "cancelled" => Self::Cancelled,
            "interrupted" => Self::Interrupted,
            "mcp_disabled" => Self::McpDisabled,
            "client_not_enrolled" => Self::ClientNotEnrolled,
            "grant_missing" => Self::GrantMissing,
            "grant_expired" => Self::GrantExpired,
            "grant_revoked" => Self::GrantRevoked,
            "scope_denied" => Self::ScopeDenied,
            "path_ambiguous" => Self::PathAmbiguous,
            "confirmation_required" => Self::ConfirmationRequired,
            "confirmation_expired" => Self::ConfirmationExpired,
            "audit_unavailable" => Self::AuditUnavailable(message),
            "host_unavailable" => Self::HostUnavailable(message),
            "host_lost" => Self::HostLost,
            _ => Self::Internal(message),
        }
    }

    /// Stable error code exposed by the desktop, CLI, and MCP contracts.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotFound(_) => "not_found",
            Self::Conflict(_) => "conflict",
            Self::CredentialMissing => "credential_missing",
            Self::CredentialRejected => "credential_rejected",
            Self::CredentialStoreUnavailable(_) => "credential_store_unavailable",
            Self::AuthFailed(_) => "auth_failed",
            Self::CertificateInvalid(_) => "certificate_invalid",
            Self::HostKeyUnknown { .. } => "host_key_unknown",
            Self::HostKeyChanged { .. } => "host_key_changed",
            Self::Network(_) => "network",
            Self::Timeout(_) => "timeout",
            Self::PermissionDenied(_) => "permission_denied",
            Self::UnsupportedCapability(_) => "unsupported_capability",
            Self::UnsupportedProtocolVersion(_) => "unsupported_protocol_version",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
            Self::McpDisabled => "mcp_disabled",
            Self::ClientNotEnrolled => "client_not_enrolled",
            Self::GrantMissing => "grant_missing",
            Self::GrantExpired => "grant_expired",
            Self::GrantRevoked => "grant_revoked",
            Self::ScopeDenied => "scope_denied",
            Self::PathAmbiguous => "path_ambiguous",
            Self::ConfirmationRequired => "confirmation_required",
            Self::ConfirmationExpired => "confirmation_expired",
            Self::AuditUnavailable(_) => "audit_unavailable",
            Self::HostUnavailable(_) => "host_unavailable",
            Self::HostLost => "host_lost",
            Self::Internal(_) => "internal",
        }
    }

    /// Stable CLI exit code. Success is `0`; errors use `2`–`13`.
    #[must_use]
    pub fn cli_exit_code(&self) -> i32 {
        match self {
            Self::Validation(_) | Self::InvalidArgument(_) | Self::PathAmbiguous => 10,
            Self::NotFound(_) => 4,
            Self::Conflict(_) => 6,
            Self::CredentialMissing | Self::CredentialRejected | Self::AuthFailed(_) => 3,
            Self::CredentialStoreUnavailable(_) | Self::AuditUnavailable(_) => 11,
            Self::Network(_) | Self::Timeout(_) | Self::CertificateInvalid(_) => 4,
            Self::HostKeyUnknown { .. } | Self::HostKeyChanged { .. } => 5,
            Self::PermissionDenied(_) => 5,
            Self::UnsupportedCapability(_) | Self::UnsupportedProtocolVersion(_) => 12,
            Self::Cancelled => 7,
            Self::Interrupted | Self::HostLost => 8,
            Self::HostUnavailable(_) => 4,
            Self::McpDisabled
            | Self::ClientNotEnrolled
            | Self::GrantMissing
            | Self::GrantExpired
            | Self::GrantRevoked
            | Self::ScopeDenied
            | Self::ConfirmationRequired
            | Self::ConfirmationExpired => 9,
            Self::Internal(_) => 13,
        }
    }

    /// Whether retrying the operation may succeed without user intervention.
    #[must_use]
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_)
                | Self::Timeout(_)
                | Self::CredentialStoreUnavailable(_)
                | Self::AuditUnavailable(_)
                | Self::HostUnavailable(_)
                | Self::Internal(_)
        )
    }

    /// SHA-256 fingerprint presented by the server for host-key failures.
    ///
    /// The value is normalized without the `SHA256:` prefix. It is `None` when
    /// the failure did not carry a server key.
    #[must_use]
    pub fn host_key_fingerprint(&self) -> Option<&str> {
        match self {
            Self::HostKeyUnknown { fingerprint, .. } | Self::HostKeyChanged { fingerprint, .. }
                if !fingerprint.is_empty() =>
            {
                Some(fingerprint)
            }
            _ => None,
        }
    }

    /// Best-effort field name for validation errors.
    #[must_use]
    pub fn field(&self) -> Option<String> {
        let Self::Validation(message) = self else {
            return None;
        };
        message
            .split_whitespace()
            .next()
            .filter(|field| {
                field
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
            })
            .map(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::CoreError;

    #[test]
    fn maps_errors_to_stable_codes() {
        assert_eq!(CoreError::CredentialMissing.code(), "credential_missing");
        assert_eq!(CoreError::CredentialRejected.code(), "credential_rejected");
        assert_eq!(
            CoreError::HostKeyChanged {
                fingerprint: "abc".to_owned(),
                detail: "line 2".to_owned(),
            }
            .code(),
            "host_key_changed"
        );
        assert_eq!(CoreError::Timeout("connect".to_owned()).code(), "timeout");
        assert_eq!(CoreError::internal("boom").code(), "internal");
        assert_eq!(CoreError::ScopeDenied.code(), "scope_denied");
        assert_eq!(CoreError::HostLost.code(), "host_lost");
    }

    #[test]
    fn maps_every_error_to_a_documented_exit_code() {
        let errors = [
            CoreError::Validation("x".to_owned()),
            CoreError::InvalidArgument("x".to_owned()),
            CoreError::NotFound("x".to_owned()),
            CoreError::Conflict("x".to_owned()),
            CoreError::CredentialMissing,
            CoreError::CredentialRejected,
            CoreError::CredentialStoreUnavailable("x".to_owned()),
            CoreError::AuthFailed("x".to_owned()),
            CoreError::CertificateInvalid("x".to_owned()),
            CoreError::HostKeyUnknown {
                fingerprint: "x".to_owned(),
                detail: "x".to_owned(),
            },
            CoreError::HostKeyChanged {
                fingerprint: "x".to_owned(),
                detail: "x".to_owned(),
            },
            CoreError::Network("x".to_owned()),
            CoreError::Timeout("x".to_owned()),
            CoreError::PermissionDenied("x".to_owned()),
            CoreError::UnsupportedCapability("x".to_owned()),
            CoreError::UnsupportedProtocolVersion("x".to_owned()),
            CoreError::Cancelled,
            CoreError::Interrupted,
            CoreError::McpDisabled,
            CoreError::ClientNotEnrolled,
            CoreError::GrantMissing,
            CoreError::GrantExpired,
            CoreError::GrantRevoked,
            CoreError::ScopeDenied,
            CoreError::PathAmbiguous,
            CoreError::ConfirmationRequired,
            CoreError::ConfirmationExpired,
            CoreError::AuditUnavailable("x".to_owned()),
            CoreError::HostUnavailable("x".to_owned()),
            CoreError::HostLost,
            CoreError::Internal("x".to_owned()),
        ];
        for error in errors {
            let code = error.cli_exit_code();
            assert!((2..=13).contains(&code), "{} -> {code}", error.code());
        }
    }

    #[test]
    fn extracts_validation_field() {
        assert_eq!(
            CoreError::validation("port must be 1-65535")
                .field()
                .as_deref(),
            Some("port")
        );
        assert_eq!(CoreError::CredentialMissing.field(), None);
    }

    #[test]
    fn marks_transient_errors_retryable() {
        assert!(CoreError::Network("reset".to_owned()).retryable());
        assert!(CoreError::Timeout("connect".to_owned()).retryable());
        assert!(CoreError::HostUnavailable("down".to_owned()).retryable());
        assert!(!CoreError::CredentialRejected.retryable());
        assert!(
            !CoreError::HostKeyChanged {
                fingerprint: "abc".to_owned(),
                detail: "line 2".to_owned(),
            }
            .retryable()
        );
    }
}
