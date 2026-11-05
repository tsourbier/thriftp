use std::collections::VecDeque;
use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::model::EntryType;
use crate::protocol::RemoteFs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SymlinkPolicy {
    #[default]
    Skip,
    Follow,
    Recreate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedEntry {
    /// Path relative to the transfer root, using `/` separators.
    pub relative_path: String,
    pub entry_type: EntryType,
    pub size: Option<u64>,
    pub target: Option<String>,
}

/// Plan an upload by walking a local file or directory.
pub fn plan_upload(root: &Path, policy: SymlinkPolicy) -> CoreResult<Vec<PlannedEntry>> {
    let metadata = std::fs::symlink_metadata(root)
        .map_err(|error| CoreError::NotFound(format!("{}: {error}", root.display())))?;
    let mut plan = Vec::new();
    let name = root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("upload")
        .to_owned();
    if metadata.file_type().is_symlink() {
        match policy {
            SymlinkPolicy::Skip => return Ok(Vec::new()),
            SymlinkPolicy::Recreate => {
                plan.push(PlannedEntry {
                    relative_path: name,
                    entry_type: EntryType::Symlink,
                    size: None,
                    target: std::fs::read_link(root)
                        .ok()
                        .map(|target| target.display().to_string()),
                });
                return Ok(plan);
            }
            SymlinkPolicy::Follow => {}
        }
    }
    let metadata = if metadata.file_type().is_symlink() {
        std::fs::metadata(root)
            .map_err(|error| CoreError::internal(format!("could not follow symlink: {error}")))?
    } else {
        metadata
    };
    if metadata.is_file() {
        plan.push(PlannedEntry {
            relative_path: name,
            entry_type: EntryType::File,
            size: Some(metadata.len()),
            target: None,
        });
        return Ok(plan);
    }
    walk_local(root, "", policy, &mut plan)?;
    Ok(plan)
}

fn walk_local(
    directory: &Path,
    prefix: &str,
    policy: SymlinkPolicy,
    plan: &mut Vec<PlannedEntry>,
) -> CoreResult<()> {
    let mut entries: Vec<_> = std::fs::read_dir(directory)
        .map_err(|error| CoreError::internal(format!("could not read directory: {error}")))?
        .collect::<Result<_, _>>()
        .map_err(|error| CoreError::internal(format!("could not read directory: {error}")))?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().into_owned();
        let relative = if prefix.is_empty() {
            file_name
        } else {
            format!("{prefix}/{file_name}")
        };
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| CoreError::internal(format!("could not stat path: {error}")))?;
        if metadata.file_type().is_symlink() {
            match policy {
                SymlinkPolicy::Skip => continue,
                SymlinkPolicy::Recreate => {
                    plan.push(PlannedEntry {
                        relative_path: relative,
                        entry_type: EntryType::Symlink,
                        size: None,
                        target: std::fs::read_link(&path)
                            .ok()
                            .map(|target| target.display().to_string()),
                    });
                    continue;
                }
                SymlinkPolicy::Follow => {}
            }
        }
        let metadata = if metadata.file_type().is_symlink() {
            std::fs::metadata(&path).map_err(|error| {
                CoreError::internal(format!("could not follow symlink: {error}"))
            })?
        } else {
            metadata
        };
        if metadata.is_dir() {
            plan.push(PlannedEntry {
                relative_path: relative.clone(),
                entry_type: EntryType::Directory,
                size: None,
                target: None,
            });
            walk_local(&path, &relative, policy, plan)?;
        } else {
            plan.push(PlannedEntry {
                relative_path: relative,
                entry_type: EntryType::File,
                size: Some(metadata.len()),
                target: None,
            });
        }
    }
    Ok(())
}

/// Plan a download by walking the remote tree breadth-first.
pub async fn plan_download(
    remote: &mut dyn RemoteFs,
    root: &str,
    policy: SymlinkPolicy,
) -> CoreResult<Vec<PlannedEntry>> {
    let root_entry = remote.stat(root).await?;
    if root_entry.entry_type == EntryType::Symlink {
        return Ok(match policy {
            SymlinkPolicy::Skip => Vec::new(),
            SymlinkPolicy::Recreate => vec![PlannedEntry {
                relative_path: root_entry.name,
                entry_type: EntryType::Symlink,
                size: None,
                target: root_entry.symlink_target,
            }],
            SymlinkPolicy::Follow => Vec::new(),
        });
    }
    if root_entry.entry_type == EntryType::File {
        return Ok(vec![PlannedEntry {
            relative_path: root_entry.name,
            entry_type: EntryType::File,
            size: root_entry.size,
            target: None,
        }]);
    }

    let mut plan = Vec::new();
    let mut queue = VecDeque::from([(root.to_owned(), String::new())]);
    while let Some((path, prefix)) = queue.pop_front() {
        for entry in remote.list_dir(&path).await? {
            let relative = if prefix.is_empty() {
                entry.name.clone()
            } else {
                format!("{prefix}/{}", entry.name)
            };
            if entry.entry_type == EntryType::Symlink {
                match policy {
                    SymlinkPolicy::Skip => continue,
                    SymlinkPolicy::Recreate => {
                        plan.push(PlannedEntry {
                            relative_path: relative,
                            entry_type: EntryType::Symlink,
                            size: None,
                            target: entry.symlink_target.clone(),
                        });
                        continue;
                    }
                    SymlinkPolicy::Follow => {}
                }
            }
            let entry = if entry.entry_type == EntryType::Symlink {
                remote.stat(&entry.path).await.unwrap_or(entry)
            } else {
                entry
            };
            if entry.entry_type == EntryType::Directory {
                plan.push(PlannedEntry {
                    relative_path: relative.clone(),
                    entry_type: EntryType::Directory,
                    size: None,
                    target: None,
                });
                queue.push_back((entry.path, relative));
            } else {
                plan.push(PlannedEntry {
                    relative_path: relative,
                    entry_type: EntryType::File,
                    size: entry.size,
                    target: None,
                });
            }
        }
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_directories_before_their_files() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::create_dir(directory.path().join("nested")).unwrap();
        std::fs::write(directory.path().join("a.txt"), b"a").unwrap();
        std::fs::write(directory.path().join("nested/b.txt"), b"bb").unwrap();

        let plan = plan_upload(directory.path(), SymlinkPolicy::Skip).unwrap();
        let paths: Vec<_> = plan
            .iter()
            .map(|entry| entry.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec!["a.txt", "nested", "nested/b.txt"]);
        assert_eq!(plan[2].size, Some(2));
    }

    #[test]
    fn skips_symlinks_by_default() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("target.txt"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            directory.path().join("target.txt"),
            directory.path().join("link.txt"),
        )
        .unwrap();

        let plan = plan_upload(directory.path(), SymlinkPolicy::Skip).unwrap();
        assert!(!plan.iter().any(|entry| entry.relative_path == "link.txt"));
    }
}
