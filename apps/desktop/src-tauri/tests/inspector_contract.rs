use thriftp_core::model::{EntryType, FileEntry, ProtocolCapabilities};

use thriftp_desktop_lib::commands::inspector::prepare_metadata;

fn entry(permissions: Option<&str>, permission_support: bool) -> FileEntry {
    FileEntry {
        name: "notes.txt".to_owned(),
        path: "/home/user/notes.txt".to_owned(),
        entry_type: EntryType::File,
        size: Some(128),
        modified_at: None,
        permissions: permissions.map(str::to_owned),
        ownership: None,
        hidden: false,
        symlink_target: None,
        capabilities: ProtocolCapabilities {
            permissions: permission_support,
            ..ProtocolCapabilities::default()
        },
    }
}

#[test]
fn local_metadata_forces_permission_detail_read_only() {
    let metadata = prepare_metadata(&entry(Some("644"), true), true);
    let detail = metadata.permission_detail.expect("permission detail");
    assert!(!detail.editable);
    assert_eq!(
        detail.reason.as_deref(),
        Some("Local permissions are read-only.")
    );
    assert_eq!(detail.mode, Some(0o644));
}

#[test]
fn remote_metadata_mirrors_server_permission_capability() {
    let editable = prepare_metadata(&entry(Some("644"), true), false);
    assert!(editable.permission_detail.expect("detail").editable);
    assert!(editable.permissions_supported);

    let read_only = prepare_metadata(&entry(Some("644"), false), false);
    let detail = read_only.permission_detail.expect("detail");
    assert!(!detail.editable);
    assert!(detail.reason.is_none());
    assert!(!read_only.permissions_supported);
}

#[test]
fn missing_or_invalid_permissions_are_explained_without_inventing_bits() {
    assert!(
        prepare_metadata(&entry(None, true), false)
            .permission_detail
            .is_none()
    );

    let invalid = prepare_metadata(&entry(Some("not-octal"), true), false)
        .permission_detail
        .expect("unavailable detail");
    assert!(invalid.mode.is_none());
    assert!(invalid.symbolic.is_none());
    assert!(!invalid.editable);
    assert!(invalid.reason.is_some());
    assert!(!invalid.user.read && !invalid.group.write && !invalid.world.execute);
}
