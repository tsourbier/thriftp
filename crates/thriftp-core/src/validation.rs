use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::model::{AuthMethod, ConnectionProfile, CredentialRef, DirectoryFavorite, Protocol};

/// Trim an optional path and normalize an empty value to `None`.
#[must_use]
pub fn normalize_optional_path(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim().to_owned();
        (!trimmed.is_empty()).then_some(trimmed)
    })
}

/// Validate optional favorite working paths.
///
/// Local paths must be absolute according to the client OS. Remote paths must
/// be server-side absolute POSIX paths. No shell or environment expansion is
/// performed.
pub fn validate_favorite_paths(local: Option<&str>, remote: Option<&str>) -> CoreResult<()> {
    if let Some(local) = local
        && !Path::new(local).is_absolute()
    {
        return Err(CoreError::validation("localPath must be an absolute path"));
    }
    if let Some(remote) = remote
        && !remote.starts_with('/')
    {
        return Err(CoreError::validation(
            "remotePath must be an absolute server path",
        ));
    }
    Ok(())
}

pub fn validate_profile(profile: &ConnectionProfile) -> CoreResult<()> {
    let name = profile.display_name.trim();
    if name.is_empty() || name.chars().count() > 200 {
        return Err(CoreError::validation(
            "displayName must be 1-200 characters",
        ));
    }
    if let Some(group) = &profile.group_label {
        let label = group.trim();
        if label.is_empty() || label.chars().count() > 100 {
            return Err(CoreError::validation(
                "groupLabel must be 1-100 characters when present",
            ));
        }
    }
    if profile.host.trim().is_empty() || profile.host.chars().count() > 255 {
        return Err(CoreError::validation("host must be 1-255 characters"));
    }
    if profile.port == 0 {
        return Err(CoreError::validation("port must be 1-65535"));
    }
    if !(1..=300).contains(&profile.timeout_seconds) {
        return Err(CoreError::validation("timeoutSeconds must be 1-300"));
    }
    match profile.protocol {
        Protocol::Ftp | Protocol::FtpsExplicit | Protocol::FtpsImplicit => {
            if profile.auth_method != AuthMethod::Password {
                return Err(CoreError::validation(
                    "FTP and FTPS require password authentication",
                ));
            }
        }
        Protocol::Sftp => {}
    }
    if profile.auth_method == AuthMethod::SshPrivateKey {
        if profile.protocol != Protocol::Sftp {
            return Err(CoreError::validation(
                "private-key authentication is supported only for SFTP",
            ));
        }
        if profile
            .ssh_key_ref
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(CoreError::validation(
                "sshKeyRef is required for private-key authentication",
            ));
        }
    }
    Ok(())
}

pub fn validate_directory_favorite(favorite: &DirectoryFavorite) -> CoreResult<()> {
    if favorite.path.trim().is_empty() {
        return Err(CoreError::validation("directory favorite path is required"));
    }
    let label = favorite.label.trim();
    if label.is_empty() || label.chars().count() > 200 {
        return Err(CoreError::validation(
            "directory favorite label must be 1-200 characters",
        ));
    }
    Ok(())
}

pub fn validate_credential_ref(reference: &CredentialRef) -> CoreResult<()> {
    if reference.host.trim().is_empty() {
        return Err(CoreError::validation("credential host is required"));
    }
    if reference.port == 0 {
        return Err(CoreError::validation("credential port must be 1-65535"));
    }
    if reference.username.trim().is_empty() {
        return Err(CoreError::validation("credential username is required"));
    }
    Ok(())
}

/// Returns whether a credential reference's identity matches a profile.
///
/// A credential reference must match every referencing connection profile on
/// protocol, host, port, username, and authentication method.
pub fn identity_matches(profile: &ConnectionProfile, reference: &CredentialRef) -> bool {
    reference.matches_profile(profile)
}
