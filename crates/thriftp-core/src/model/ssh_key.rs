//! Discovered SSH private-key metadata.
//!
//! Discovery never reads or stores key contents.

use serde::{Deserialize, Serialize};

/// A private-key candidate found in the platform SSH directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyCandidate {
    /// File name shown to the user.
    pub file_name: String,
    /// Full path used as the key reference.
    pub path: String,
}
