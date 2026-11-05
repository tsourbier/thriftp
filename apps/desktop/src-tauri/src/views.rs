use serde::Serialize;
use thriftp_core::model::{ActiveConnection, ConnectionProfile, Protocol};
use thriftp_core::protocol::{TransportWarning, plain_ftp_warning};

use crate::error::DesktopError;

/// Serialized form of an active connection for the desktop IPC contract.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    pub connection_id: String,
    pub profile_id: String,
    pub protocol: Protocol,
    pub state: String,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DesktopError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport_warning: Option<TransportWarning>,
    /// Favorite that produced this connection, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_id: Option<String>,
}

impl From<ActiveConnection> for ConnectionView {
    fn from(connection: ActiveConnection) -> Self {
        let transport_warning = plain_ftp_warning(connection.protocol);
        Self {
            connection_id: connection.id.to_string(),
            profile_id: connection.connection_profile_id.to_string(),
            protocol: connection.protocol,
            state: connection.state,
            started_at: connection.started_at.to_rfc3339(),
            error: None,
            transport_warning,
            favorite_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletedView {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DisconnectedView {
    pub disconnected: bool,
}

/// Result of explicitly trusting an SSH host key.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedHostKeyView {
    /// Always true after the trust decision is stored.
    pub trusted: bool,
    /// Normalized SHA-256 fingerprint that was stored.
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestView {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<DesktopError>,
}

/// Profile payload returned by create/update commands.
pub type ProfileView = ConnectionProfile;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingHandle {
    pub listing_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelledView {
    pub cancelled: bool,
}

/// Result of deleting a server favorite.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteDeleteView {
    /// Always true after a successful deletion.
    pub deleted: bool,
    /// Credential reference left without any favorite, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orphaned_credential_ref_id: Option<String>,
    /// Remaining favorites affected by the credential, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_impact: Option<thriftp_core::server_favorites::CredentialImpact>,
}

/// Result of removing one favorite's credential reference.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialRemoveView {
    /// Always true after a successful removal.
    pub removed: bool,
    /// Favorites that referenced the credential.
    pub impact: thriftp_core::server_favorites::CredentialImpact,
}

/// Result of renaming a favorite group label.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameGroupView {
    /// Favorites whose label was updated.
    pub updated: Vec<thriftp_core::server_favorites::ServerFavoriteView>,
}

/// Profile payload for the save-as-favorite prefill flow.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveProfileView {
    /// Active connection identifier.
    pub connection_id: String,
    /// Favorite already linked to this profile, when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite_id: Option<String>,
    /// Current connection-profile settings.
    pub profile: ConnectionProfile,
}

/// Event payload emitted when favorites change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesChangedEvent {
    /// Favorite identifier.
    pub favorite_id: String,
    /// Reason for the change.
    pub reason: String,
}

/// Event payload emitted when credentials change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialsChangedEvent {
    /// Credential reference identifier.
    pub credential_ref_id: String,
    /// Favorites that reference the credential.
    pub affected_favorite_ids: Vec<String>,
    /// Reason for the change.
    pub reason: String,
}
