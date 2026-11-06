use async_trait::async_trait;
use thriftp_core::browser::metadata::{
    metadata_from_entry, parse_octal_mode, symbolic_permissions,
};
use thriftp_core::browser::permissions::chmod_remote;
use thriftp_core::error::CoreError;
use thriftp_core::model::{
    ConnectionProfile, EntryType, FileEntry, Ownership, ProtocolCapabilities,
};
use thriftp_core::protocol::RemoteFs;

struct FakeRemote {
    permissions: bool,
    mode: Option<u32>,
}

#[async_trait]
impl RemoteFs for FakeRemote {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        _credential: Option<thriftp_core::credentials::Secret>,
    ) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn disconnect(&mut self) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities {
            permissions: self.permissions,
            ownership: true,
            ..ProtocolCapabilities::default()
        }
    }
    async fn list_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<Vec<FileEntry>> {
        Ok(Vec::new())
    }
    async fn stat(&mut self, path: &str) -> thriftp_core::CoreResult<FileEntry> {
        Ok(entry(path))
    }
    async fn mkdir(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn rename(&mut self, _from: &str, _to: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn delete_file(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn delete_dir(&mut self, _path: &str) -> thriftp_core::CoreResult<()> {
        Ok(())
    }
    async fn chmod(&mut self, _path: &str, mode: u32) -> thriftp_core::CoreResult<()> {
        self.mode = Some(mode);
        Ok(())
    }
}

fn entry(path: &str) -> FileEntry {
    FileEntry {
        name: "file.txt".to_owned(),
        path: path.to_owned(),
        entry_type: EntryType::File,
        size: Some(42),
        modified_at: None,
        permissions: Some("644".to_owned()),
        ownership: Some(Ownership {
            owner: "1000".to_owned(),
            group: Some("1000".to_owned()),
        }),
        hidden: false,
        symlink_target: None,
        capabilities: ProtocolCapabilities {
            permissions: true,
            ownership: true,
            ..ProtocolCapabilities::default()
        },
    }
}

#[test]
fn metadata_reports_permissions_and_ownership_support() {
    let metadata = metadata_from_entry(&entry("/tmp/file.txt"));
    assert!(metadata.permissions_supported);
    assert!(metadata.ownership_supported);
    assert_eq!(metadata.entry.size, Some(42));
    assert_eq!(metadata.entry.permissions.as_deref(), Some("644"));
    assert_eq!(metadata.entry.ownership.as_ref().unwrap().owner, "1000");
}

#[test]
fn unparseable_permissions_explain_the_limitation() {
    let mut entry = entry("/tmp/file.txt");
    entry.permissions = Some("not-octal".to_owned());
    let metadata = metadata_from_entry(&entry);
    let detail = metadata.permission_detail.expect("unavailable detail");
    assert!(detail.mode.is_none());
    assert!(detail.symbolic.is_none());
    assert!(!detail.editable);
    assert!(detail.reason.is_some());
}

#[test]
fn permissions_format_and_parse() {
    assert_eq!(symbolic_permissions("640").as_deref(), Some("rw-r-----"));
    assert_eq!(parse_octal_mode("644").unwrap(), 0o644);
    assert!(parse_octal_mode("abc").is_err());
}

#[tokio::test]
async fn chmod_is_applied_when_supported() {
    let mut remote = FakeRemote {
        permissions: true,
        mode: None,
    };
    let updated = chmod_remote(&mut remote, "/file.txt", 0o640).await.unwrap();
    assert_eq!(remote.mode, Some(0o640));
    assert_eq!(updated.name, "file.txt");
}

#[tokio::test]
async fn chmod_is_explained_when_unsupported() {
    let mut remote = FakeRemote {
        permissions: false,
        mode: None,
    };
    let error = chmod_remote(&mut remote, "/file.txt", 0o640)
        .await
        .unwrap_err();
    assert!(matches!(error, CoreError::UnsupportedCapability(_)));
    assert_eq!(remote.mode, None);
}
