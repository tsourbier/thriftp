use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::profile::{AuthMethod, ConnectionProfile, Protocol};

/// Namespace used to derive a stable credential-reference id from a profile.
///
/// The id is deliberately independent from the connection-profile id: it is a
/// value in its own UUID namespace so a leaked or reused profile id cannot be
/// mistaken for a credential reference id.
const CREDENTIAL_REFERENCE_NAMESPACE: Uuid =
    Uuid::from_u128(0x6a0f_c4d2_9c31_4b6e_8d79_2a15_5f4c_e7b3);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRef {
    pub id: Uuid,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CredentialRef {
    /// Credential identity bound to a connection profile.
    ///
    /// A credential reference **must match every referencing connection
    /// profile** on protocol, host, port, username, and authentication method.
    /// **Changing any identity field invalidates the previous reference**, so
    /// the derived id includes those fields: the old reference is no longer
    /// produced and its secret is not reused for the edited profile.
    ///
    /// The reference id is an independent UUID (derived with the UUID v5
    /// namespace above); it is never the connection-profile id.
    #[must_use]
    pub fn for_profile(profile: &ConnectionProfile) -> Self {
        let identity = format!(
            "{}|{}|{}|{}|{}|{}",
            profile.id,
            protocol_key(profile.protocol),
            profile.host,
            profile.port,
            profile.username,
            auth_method_key(profile.auth_method)
        );
        let now = Utc::now();
        Self {
            id: Uuid::new_v5(&CREDENTIAL_REFERENCE_NAMESPACE, identity.as_bytes()),
            protocol: profile.protocol,
            host: profile.host.clone(),
            port: profile.port,
            username: profile.username.clone(),
            auth_method: profile.auth_method,
            created_at: now,
            updated_at: now,
        }
    }

    /// Returns whether this reference may serve the supplied profile.
    ///
    /// This implements the rule that a credential reference must match every
    /// referencing connection profile on all identity-defining fields.
    #[must_use]
    pub fn matches_profile(&self, profile: &ConnectionProfile) -> bool {
        self.protocol == profile.protocol
            && self.host == profile.host
            && self.port == profile.port
            && self.username == profile.username
            && self.auth_method == profile.auth_method
    }
}

fn protocol_key(protocol: Protocol) -> &'static str {
    match protocol {
        Protocol::Ftp => "ftp",
        Protocol::FtpsExplicit => "ftps-explicit",
        Protocol::FtpsImplicit => "ftps-implicit",
        Protocol::Sftp => "sftp",
    }
}

fn auth_method_key(auth_method: AuthMethod) -> &'static str {
    match auth_method {
        AuthMethod::Password => "password",
        AuthMethod::SshPrivateKey => "ssh-private-key",
    }
}
