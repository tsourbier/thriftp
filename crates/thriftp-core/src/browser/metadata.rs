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
    pub permission_detail: Option<PermissionDetail>,
}

/// Read, write, and execute state for one permission class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionBits {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

/// Structured permission detail used by the inspector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDetail {
    pub mode: Option<u32>,
    pub symbolic: Option<String>,
    pub user: PermissionBits,
    pub group: PermissionBits,
    pub world: PermissionBits,
    pub editable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Build inspector metadata from a listing entry.
#[must_use]
pub fn metadata_from_entry(entry: &FileEntry) -> FileMetadata {
    let capabilities = &entry.capabilities;
    FileMetadata {
        entry: entry.clone(),
        permissions_supported: capabilities.permissions,
        ownership_supported: capabilities.ownership,
        permission_detail: permission_detail(
            entry.permissions.as_deref(),
            capabilities.permissions,
            None,
        ),
    }
}

fn permission_detail(
    permissions: Option<&str>,
    editable: bool,
    reason: Option<String>,
) -> Option<PermissionDetail> {
    let permissions = permissions?;
    let octal = permissions.trim().trim_start_matches("0o");
    let Some(mode) = u32::from_str_radix(octal, 8).ok() else {
        return Some(unavailable_permission_detail(
            "The server returned an unrecognized permission value.",
        ));
    };
    if mode > 0o777 {
        return Some(unavailable_permission_detail(
            "The server returned a permission value outside the 000-777 range.",
        ));
    }
    let bits = |shift: u32| PermissionBits {
        read: mode & (0o4 << shift) != 0,
        write: mode & (0o2 << shift) != 0,
        execute: mode & (0o1 << shift) != 0,
    };
    Some(PermissionDetail {
        mode: Some(mode),
        symbolic: symbolic_permissions(octal),
        user: bits(6),
        group: bits(3),
        world: bits(0),
        editable,
        reason,
    })
}

fn unavailable_permission_detail(reason: &str) -> PermissionDetail {
    PermissionDetail {
        mode: None,
        symbolic: None,
        user: PermissionBits::default(),
        group: PermissionBits::default(),
        world: PermissionBits::default(),
        editable: false,
        reason: Some(reason.to_owned()),
    }
}

/// Fetch and normalize inspector metadata for a remote path.
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
    fn exposes_permission_bits_and_symbolic_summary() {
        let mut entry = FileEntry {
            name: "file".to_owned(),
            path: "/file".to_owned(),
            entry_type: crate::model::EntryType::File,
            size: None,
            modified_at: None,
            permissions: Some("754".to_owned()),
            ownership: None,
            hidden: false,
            symlink_target: None,
            capabilities: crate::model::ProtocolCapabilities {
                permissions: true,
                ..crate::model::ProtocolCapabilities::default()
            },
        };
        let metadata = metadata_from_entry(&entry);
        let detail = metadata.permission_detail.unwrap();
        assert_eq!(detail.mode, Some(0o754));
        assert_eq!(detail.symbolic.as_deref(), Some("rwxr-xr--"));
        assert!(detail.user.read && detail.user.write && detail.user.execute);
        assert!(detail.group.read && !detail.group.write && detail.group.execute);
        assert!(detail.world.read && !detail.world.write && !detail.world.execute);
        assert!(detail.editable);

        entry.permissions = None;
        assert!(metadata_from_entry(&entry).permission_detail.is_none());

        entry.permissions = Some("not-octal".to_owned());
        let invalid = metadata_from_entry(&entry)
            .permission_detail
            .expect("unavailable detail");
        assert!(invalid.mode.is_none());
        assert!(invalid.symbolic.is_none());
        assert!(!invalid.editable);
        assert!(invalid.reason.is_some());
        assert!(!invalid.user.read && !invalid.group.write && !invalid.world.execute);

        entry.permissions = Some("1000".to_owned());
        let out_of_range = metadata_from_entry(&entry)
            .permission_detail
            .expect("unavailable detail");
        assert!(out_of_range.mode.is_none());
        assert!(out_of_range.reason.is_some());
    }

    #[test]
    fn parses_octal_modes() {
        assert_eq!(parse_octal_mode("755").unwrap(), 0o755);
        assert_eq!(parse_octal_mode("0o600").unwrap(), 0o600);
        assert!(parse_octal_mode("999").is_err());
        assert!(parse_octal_mode("1000").is_err());
    }
}
