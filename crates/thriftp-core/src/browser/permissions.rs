use std::collections::VecDeque;

use serde::Serialize;

use crate::error::{CoreError, CoreResult};
use crate::model::{EntryType, FileEntry};
use crate::protocol::RemoteFs;

/// One item that could not be changed during a recursive permission update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecursivePermissionFailure {
    pub path: String,
    pub message: String,
}

/// Outcome of applying one permission mode to a remote directory tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecursivePermissionResult {
    pub root: String,
    pub changed: usize,
    pub skipped: usize,
    pub failed: Vec<RecursivePermissionFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Change remote permissions, failing clearly when the protocol cannot.
pub async fn chmod_remote(
    remote: &mut dyn RemoteFs,
    path: &str,
    mode: u32,
) -> CoreResult<FileEntry> {
    if mode > 0o777 {
        return Err(CoreError::validation(
            "permission mode must be between 000 and 777",
        ));
    }
    if !remote.capabilities().permissions {
        return Err(CoreError::UnsupportedCapability(
            "this server does not support permission changes".to_owned(),
        ));
    }
    remote.chmod(path, mode).await?;
    remote.stat(path).await
}

/// Apply one permission mode to every descendant of a remote directory.
///
/// The selected directory itself is not included. Symbolic links are skipped
/// and per-item failures are reported without stopping the traversal.
pub async fn chmod_remote_recursive(
    remote: &mut dyn RemoteFs,
    path: &str,
    mode: u32,
) -> CoreResult<RecursivePermissionResult> {
    if mode > 0o777 {
        return Err(CoreError::validation(
            "permission mode must be between 000 and 777",
        ));
    }
    if !remote.capabilities().permissions {
        return Err(CoreError::UnsupportedCapability(
            "this server does not support permission changes".to_owned(),
        ));
    }
    let root_entry = remote.stat(path).await?;
    if root_entry.entry_type != EntryType::Directory {
        return Err(CoreError::validation(
            "recursive permission changes require a directory",
        ));
    }

    let mut pending = VecDeque::from([path.to_owned()]);
    let mut changed = 0;
    let mut skipped = 0;
    let mut failed = Vec::new();

    while let Some(directory) = pending.pop_front() {
        let entries = match remote.list_dir(&directory).await {
            Ok(entries) => entries,
            Err(error) => {
                failed.push(RecursivePermissionFailure {
                    path: directory.clone(),
                    message: error.to_string(),
                });
                continue;
            }
        };

        for entry in entries {
            if entry.entry_type == EntryType::Symlink {
                skipped += 1;
                continue;
            }
            match remote.chmod(&entry.path, mode).await {
                Ok(()) => changed += 1,
                Err(error) => failed.push(RecursivePermissionFailure {
                    path: entry.path.clone(),
                    message: error.to_string(),
                }),
            }
            if entry.entry_type == EntryType::Directory {
                pending.push_back(entry.path);
            }
        }
    }

    Ok(RecursivePermissionResult {
        root: path.to_owned(),
        changed,
        skipped,
        failed,
        message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::ProtocolCapabilities;
    use async_trait::async_trait;

    struct Locked;

    #[async_trait]
    impl RemoteFs for Locked {
        async fn connect(
            &mut self,
            _profile: &crate::model::ConnectionProfile,
            _credential: Option<crate::credentials::Secret>,
        ) -> CoreResult<()> {
            Ok(())
        }
        async fn disconnect(&mut self) -> CoreResult<()> {
            Ok(())
        }
        fn capabilities(&self) -> ProtocolCapabilities {
            ProtocolCapabilities {
                permissions: false,
                ..ProtocolCapabilities::default()
            }
        }
        async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
            Ok(Vec::new())
        }
        async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
            Err(CoreError::NotFound(path.to_owned()))
        }
        async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
            Ok(())
        }
    }

    struct Tree {
        fail: Option<String>,
    }

    fn entry(path: &str, entry_type: crate::model::EntryType) -> FileEntry {
        FileEntry {
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            path: path.to_owned(),
            entry_type,
            size: None,
            modified_at: None,
            permissions: Some("644".to_owned()),
            ownership: None,
            hidden: false,
            symlink_target: None,
            capabilities: ProtocolCapabilities {
                permissions: true,
                ..ProtocolCapabilities::default()
            },
        }
    }

    #[async_trait]
    impl RemoteFs for Tree {
        async fn connect(
            &mut self,
            _profile: &crate::model::ConnectionProfile,
            _credential: Option<crate::credentials::Secret>,
        ) -> CoreResult<()> {
            Ok(())
        }
        async fn disconnect(&mut self) -> CoreResult<()> {
            Ok(())
        }
        fn capabilities(&self) -> ProtocolCapabilities {
            ProtocolCapabilities {
                permissions: true,
                ..ProtocolCapabilities::default()
            }
        }
        async fn list_dir(&mut self, path: &str) -> CoreResult<Vec<FileEntry>> {
            let entries = match path {
                "/root" => vec![
                    entry("/root/file.txt", crate::model::EntryType::File),
                    entry("/root/sub", crate::model::EntryType::Directory),
                    entry("/root/link", crate::model::EntryType::Symlink),
                ],
                "/root/sub" => vec![entry("/root/sub/nested.txt", crate::model::EntryType::File)],
                _ => Vec::new(),
            };
            Ok(entries)
        }
        async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
            if path == "/root" {
                return Ok(entry("/root", crate::model::EntryType::Directory));
            }
            Err(CoreError::NotFound(path.to_owned()))
        }
        async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
            Ok(())
        }
        async fn chmod(&mut self, path: &str, _mode: u32) -> CoreResult<()> {
            if self.fail.as_deref() == Some(path) {
                return Err(CoreError::PermissionDenied(path.to_owned()));
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn recursive_chmod_skips_symlinks_and_walks_descendants() {
        let mut remote = Tree { fail: None };
        let result = chmod_remote_recursive(&mut remote, "/root", 0o700)
            .await
            .unwrap();
        assert_eq!(result.changed, 3);
        assert_eq!(result.skipped, 1);
        assert!(result.failed.is_empty());
    }

    #[tokio::test]
    async fn recursive_chmod_reports_partial_failures() {
        let mut remote = Tree {
            fail: Some("/root/sub/nested.txt".to_owned()),
        };
        let result = chmod_remote_recursive(&mut remote, "/root", 0o700)
            .await
            .unwrap();
        assert_eq!(result.changed, 2);
        assert_eq!(result.failed.len(), 1);
        assert_eq!(result.failed[0].path, "/root/sub/nested.txt");
    }

    #[tokio::test]
    async fn unsupported_permissions_fail_before_the_protocol_call() {
        let mut remote = Locked;
        let error = chmod_remote(&mut remote, "/file", 0o644).await.unwrap_err();
        assert!(matches!(error, CoreError::UnsupportedCapability(_)));
    }

    #[tokio::test]
    async fn invalid_modes_are_rejected() {
        let mut remote = Locked;
        assert!(chmod_remote(&mut remote, "/file", 0o1000).await.is_err());
    }
}
