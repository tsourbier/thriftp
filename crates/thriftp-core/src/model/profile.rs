use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Protocol {
    Ftp,
    FtpsExplicit,
    FtpsImplicit,
    Sftp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthMethod {
    Password,
    SshPrivateKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub id: Uuid,
    pub display_name: String,
    pub group_label: Option<String>,
    pub protocol: Protocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth_method: AuthMethod,
    pub timeout_seconds: u32,
    pub ssh_key_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_used_at: Option<DateTime<Utc>>,
}

impl ConnectionProfile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display_name: impl Into<String>,
        protocol: Protocol,
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        auth_method: AuthMethod,
        timeout_seconds: u32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            display_name: display_name.into(),
            group_label: None,
            protocol,
            host: host.into(),
            port,
            username: username.into(),
            auth_method,
            timeout_seconds,
            ssh_key_ref: None,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }
}
