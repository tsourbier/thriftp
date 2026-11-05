use std::path::Path;

use crate::error::{CoreError, CoreResult};
use crate::model::EntryType;
use crate::protocol::RemoteFs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteOutcome {
    MovedToTrash,
    PermanentlyDeleted,
}

/// Delete a local path through the operating-system trash.
pub fn delete_local(path: &Path) -> CoreResult<DeleteOutcome> {
    if !path.exists() && std::fs::symlink_metadata(path).is_err() {
        return Err(CoreError::NotFound(path.display().to_string()));
    }
    trash::delete(path)
        .map_err(|error| CoreError::internal(format!("could not move path to trash: {error}")))?;
    Ok(DeleteOutcome::MovedToTrash)
}

/// Delete a remote path permanently.
///
/// Remote deletion cannot be undone, so callers must pass explicit
/// confirmation from the user interface.
pub async fn delete_remote(
    remote: &mut dyn RemoteFs,
    path: &str,
    entry_type: EntryType,
    confirmed: bool,
) -> CoreResult<DeleteOutcome> {
    if !confirmed {
        return Err(CoreError::Conflict(
            "remote deletion is permanent and requires confirmation".to_owned(),
        ));
    }
    match entry_type {
        EntryType::Directory => remote.delete_dir(path).await?,
        _ => remote.delete_file(path).await?,
    }
    Ok(DeleteOutcome::PermanentlyDeleted)
}
