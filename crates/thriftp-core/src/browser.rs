use std::fs;
use std::path::Path;

use chrono::{DateTime, Utc};

use crate::error::{CoreError, CoreResult};
#[cfg(unix)]
use crate::model::Ownership;
use crate::model::{EntryType, FileEntry, ProtocolCapabilities};
use crate::protocol::RemoteFs;

pub mod delete;
pub mod history;
pub mod listing;
pub mod metadata;
pub mod metrics;
pub mod permissions;

pub use delete::{DeleteOutcome, delete_local, delete_remote};

fn local_capabilities() -> ProtocolCapabilities {
    ProtocolCapabilities {
        permissions: cfg!(unix),
        ownership: cfg!(unix),
        hidden_files: true,
        symbolic_links: true,
        server_side_rename: true,
        server_side_delete: true,
        ..ProtocolCapabilities::default()
    }
}

pub fn list_local(path: &Path) -> CoreResult<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        entries.push(file_entry_from_dir_entry(&entry)?);
    }
    crate::sorting::sort_file_entries(&mut entries, crate::model::SortKey::Name);
    Ok(entries)
}

pub fn stat_local(path: &Path) -> CoreResult<FileEntry> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    Ok(file_entry_from_metadata(
        name,
        path.display().to_string(),
        &metadata,
        path,
    ))
}

pub fn create_dir_local(path: &Path) -> CoreResult<()> {
    fs::create_dir_all(path).map_err(io_error)
}

pub fn rename_local(from: &Path, to: &Path) -> CoreResult<()> {
    fs::rename(from, to).map_err(io_error)
}

pub async fn list_remote(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<Vec<FileEntry>> {
    remote.list_dir(path).await
}

pub async fn stat_remote(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<FileEntry> {
    remote.stat(path).await
}

pub async fn create_dir_remote(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<()> {
    remote.mkdir(path).await
}

pub async fn rename_remote(remote: &mut dyn RemoteFs, from: &str, to: &str) -> CoreResult<()> {
    remote.rename(from, to).await
}

pub async fn delete_remote_file(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<()> {
    remote.delete_file(path).await
}

pub async fn delete_remote_dir(remote: &mut dyn RemoteFs, path: &str) -> CoreResult<()> {
    remote.delete_dir(path).await
}

fn file_entry_from_dir_entry(entry: &fs::DirEntry) -> CoreResult<FileEntry> {
    let metadata = entry.metadata().map_err(io_error)?;
    let path = entry.path();
    let name = entry.file_name().to_string_lossy().into_owned();
    Ok(file_entry_from_metadata(
        name,
        path.display().to_string(),
        &metadata,
        &path,
    ))
}

fn file_entry_from_metadata(
    name: String,
    path: String,
    metadata: &fs::Metadata,
    source_path: &Path,
) -> FileEntry {
    let file_type = metadata.file_type();
    let entry_type = if file_type.is_symlink() {
        EntryType::Symlink
    } else if file_type.is_dir() {
        EntryType::Directory
    } else if file_type.is_file() {
        EntryType::File
    } else {
        EntryType::Other
    };

    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        Some(format!("{:o}", metadata.permissions().mode() & 0o777))
    };
    #[cfg(not(unix))]
    let permissions = None;

    #[cfg(unix)]
    let ownership = {
        use std::os::unix::fs::MetadataExt;
        Some(Ownership {
            owner: metadata.uid().to_string(),
            group: Some(metadata.gid().to_string()),
        })
    };
    #[cfg(not(unix))]
    let ownership = None;

    FileEntry {
        name: name.clone(),
        path,
        entry_type,
        size: (entry_type == EntryType::File).then_some(metadata.len()),
        modified_at: metadata.modified().ok().map(DateTime::<Utc>::from),
        permissions,
        ownership,
        hidden: name.starts_with('.'),
        symlink_target: if entry_type == EntryType::Symlink {
            fs::read_link(source_path)
                .ok()
                .map(|target| target.display().to_string())
        } else {
            None
        },
        capabilities: local_capabilities(),
    }
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::internal(format!("filesystem error: {error}"))
}
