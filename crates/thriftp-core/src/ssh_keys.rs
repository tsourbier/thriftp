//! Platform SSH-directory discovery and key-path validation.
//!
//! The module reads directory entries and file metadata only. It never reads,
//! returns, or logs private-key contents.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};
use crate::model::SshKeyCandidate;

/// Conventional private-key file names considered during discovery.
pub const CONVENTIONAL_KEY_NAMES: [&str; 4] = ["id_rsa", "id_ed25519", "id_ecdsa", "id_dsa"];

/// Result of scanning the platform SSH directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyListing {
    /// Directory that was scanned.
    pub directory: String,
    /// Whether the directory exists.
    pub exists: bool,
    /// Whether the directory could be read.
    pub readable: bool,
    /// Conventional private-key candidates.
    pub candidates: Vec<SshKeyCandidate>,
}

/// Result of validating a selected key path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyValidation {
    /// Whether the path is usable as a private-key reference.
    pub valid: bool,
    /// Stable validation code when invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// User-facing message when invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl SshKeyValidation {
    /// Return a valid result.
    #[must_use]
    pub fn valid() -> Self {
        Self {
            valid: true,
            code: None,
            message: None,
        }
    }

    /// Return an invalid result.
    #[must_use]
    pub fn invalid(code: &str, message: &str) -> Self {
        Self {
            valid: false,
            code: Some(code.to_owned()),
            message: Some(message.to_owned()),
        }
    }
}

/// Resolve the platform SSH directory.
#[must_use]
pub fn platform_ssh_directory() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    Some(home.join(".ssh"))
}

/// Scan the default platform SSH directory.
pub fn list_default_candidates() -> CoreResult<SshKeyListing> {
    let directory = platform_ssh_directory()
        .ok_or_else(|| CoreError::internal("the platform home directory could not be resolved"))?;
    Ok(list_candidates(&directory))
}

/// Scan a specific SSH directory.
#[must_use]
pub fn list_candidates(directory: &Path) -> SshKeyListing {
    let exists = directory.is_dir();
    let display = directory.display().to_string();
    if !exists {
        return SshKeyListing {
            directory: display,
            exists: false,
            readable: false,
            candidates: Vec::new(),
        };
    }
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(_) => {
            return SshKeyListing {
                directory: display,
                exists: true,
                readable: false,
                candidates: Vec::new(),
            };
        }
    };
    let mut candidates = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !CONVENTIONAL_KEY_NAMES.contains(&name.as_str()) {
                return None;
            }
            let metadata = entry.metadata().ok()?;
            if !metadata.is_file() {
                return None;
            }
            Some(SshKeyCandidate {
                file_name: name,
                path: entry.path().display().to_string(),
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.file_name.cmp(&right.file_name));
    SshKeyListing {
        directory: display,
        exists: true,
        readable: true,
        candidates,
    }
}

/// Validate a selected private-key path without reading its contents.
pub fn validate_key_path(path: &str) -> CoreResult<SshKeyValidation> {
    if path.trim().is_empty() {
        return Err(CoreError::validation("path is required"));
    }
    if path.contains('\0') {
        return Err(CoreError::validation("path contains an invalid character"));
    }
    let candidate = Path::new(path);
    if !candidate.exists() {
        return Ok(SshKeyValidation::invalid(
            "missing",
            "The selected private key file does not exist.",
        ));
    }
    if !candidate.is_file() {
        return Ok(SshKeyValidation::invalid(
            "not_a_file",
            "The selected path is not a file.",
        ));
    }
    if path.ends_with(".pub") {
        return Ok(SshKeyValidation::invalid(
            "public_key",
            "Select a private key, not a public key.",
        ));
    }
    if fs::File::open(candidate).is_err() {
        return Ok(SshKeyValidation::invalid(
            "unreadable",
            "The selected private key file cannot be read.",
        ));
    }
    Ok(SshKeyValidation::valid())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{list_candidates, validate_key_path};

    #[test]
    fn lists_only_conventional_private_keys() {
        let dir = std::env::temp_dir().join(format!("thriftp-ssh-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("dir");
        fs::write(dir.join("id_ed25519"), "private").expect("key");
        fs::write(dir.join("id_rsa.pub"), "public").expect("pub");
        fs::write(dir.join("known_hosts"), "host").expect("known");
        fs::write(dir.join("random"), "random").expect("random");
        let listing = list_candidates(&dir);
        assert!(listing.exists);
        assert!(listing.readable);
        assert_eq!(listing.candidates.len(), 1);
        assert_eq!(listing.candidates[0].file_name, "id_ed25519");
        fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_directory_is_not_an_error() {
        let dir = std::env::temp_dir().join(format!("thriftp-missing-{}", uuid::Uuid::new_v4()));
        let listing = list_candidates(&dir);
        assert!(!listing.exists);
        assert!(!listing.readable);
        assert!(listing.candidates.is_empty());
    }

    #[test]
    fn validates_existing_private_key_and_rejects_public_key() {
        let dir = std::env::temp_dir().join(format!("thriftp-ssh-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).expect("dir");
        let key = dir.join("id_ed25519");
        fs::write(&key, "private").expect("key");
        assert!(
            validate_key_path(&key.display().to_string())
                .expect("valid")
                .valid
        );
        let public = dir.join("id_ed25519.pub");
        fs::write(&public, "public").expect("pub");
        let result = validate_key_path(&public.display().to_string()).expect("result");
        assert!(!result.valid);
        assert_eq!(result.code.as_deref(), Some("public_key"));
        fs::remove_dir_all(dir).ok();
    }
}
