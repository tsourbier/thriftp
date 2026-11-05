use serde::Serialize;

use crate::error::{CoreError, CoreResult};
use crate::model::FileEntry;
use crate::protocol::RemoteFs;

/// Normalized metadata shown by the inspector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileMetadata {
    pub entry: FileEntry,
    pub permissions_supported: bool,
    pub ownership_supported: bool,
}

pub fn metadata_from_entry(entry: &FileEntry) -> FileMetadata {
    let capabilities = &entry.capabilities;
    FileMetadata {
        entry: entry.clone(),
        permissions_supported: capabilities.permissions,
        ownership_supported: capabilities.ownership,
    }
}

pub async fn metadata_remote(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<FileMetadata> {
    let entry = remote.stat(path).await?;
    Ok(metadata_from_entry(&entry))
}

/// Render an octal permission string such as `755` as `rwxr-xr-x`.
#[must_use]
pub fn symbolic_permissions(octal: &str) -> Option<String> {
    let value = u32::from_str_radix(octal, 8).ok()?;
    if value > 0o7777 {
        return None;
    }
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    Some(
        bits.iter()
            .map(|(mask, character)| if value & mask != 0 { *character } else { '-' })
            .collect(),
    )
}

pub fn parse_octal_mode(value: &str) -> CoreResult<u32> {
    let trimmed = value.trim().trim_start_matches("0o");
    let mode = u32::from_str_radix(trimmed, 8)
        .map_err(|_| CoreError::validation("permission mode must be an octal value"))?;
    if mode > 0o777 {
        return Err(CoreError::validation(
            "permission mode must be between 000 and 777",
        ));
    }
    Ok(mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_symbolic_permissions() {
        assert_eq!(symbolic_permissions("755").as_deref(), Some("rwxr-xr-x"));
        assert_eq!(symbolic_permissions("644").as_deref(), Some("rw-r--r--"));
        assert_eq!(symbolic_permissions("invalid"), None);
    }

    #[test]
    fn parses_octal_modes() {
        assert_eq!(parse_octal_mode("755").unwrap(), 0o755);
        assert_eq!(parse_octal_mode("0o600").unwrap(), 0o600);
        assert!(parse_octal_mode("999").is_err());
        assert!(parse_octal_mode("1000").is_err());
    }
}
