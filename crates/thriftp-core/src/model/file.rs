use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ownership {
    pub owner: String,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolCapabilities {
    pub resume: bool,
    pub permissions: bool,
    pub ownership: bool,
    pub hidden_files: bool,
    pub symbolic_links: bool,
    pub server_side_rename: bool,
    pub server_side_delete: bool,
    pub max_path_length: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub entry_type: EntryType,
    pub size: Option<u64>,
    pub modified_at: Option<DateTime<Utc>>,
    pub permissions: Option<String>,
    pub ownership: Option<Ownership>,
    pub hidden: bool,
    pub symlink_target: Option<String>,
    pub capabilities: ProtocolCapabilities,
}

impl FileEntry {
    pub fn directory(
        name: impl Into<String>,
        path: impl Into<String>,
        capabilities: ProtocolCapabilities,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            entry_type: EntryType::Directory,
            size: None,
            modified_at: None,
            permissions: None,
            ownership: None,
            hidden: false,
            symlink_target: None,
            capabilities,
        }
    }
}
