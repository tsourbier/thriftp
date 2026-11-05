use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{CoreError, CoreResult};
use crate::model::PartialTransferData;
use crate::recovery::verify_resume;

pub const PARTIAL_SUFFIX: &str = ".thriftp-part";
pub const METADATA_SUFFIX: &str = ".thriftp-part.json";

/// Path of the partial file written next to the final destination.
#[must_use]
pub fn partial_path(destination: &Path) -> PathBuf {
    let mut value = destination.as_os_str().to_owned();
    value.push(PARTIAL_SUFFIX);
    PathBuf::from(value)
}

#[must_use]
pub fn metadata_path(destination: &Path) -> PathBuf {
    let mut value = destination.as_os_str().to_owned();
    value.push(METADATA_SUFFIX);
    PathBuf::from(value)
}

#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn new_partial(
    transfer_id: Uuid,
    local_path: Option<&Path>,
    remote_path: Option<&str>,
    offset: u64,
    source_size: Option<u64>,
    source_modified_at: Option<DateTime<Utc>>,
) -> PartialTransferData {
    let now = Utc::now();
    PartialTransferData {
        id: Uuid::new_v4(),
        transfer_id,
        local_path: local_path.map(|path| path.display().to_string()),
        remote_path: remote_path.map(str::to_owned),
        offset,
        source_size,
        source_modified_at,
        created_at: now,
        updated_at: now,
    }
}

pub fn write_metadata(destination: &Path, partial: &PartialTransferData) -> CoreResult<()> {
    let bytes = serde_json::to_vec_pretty(partial).map_err(|error| {
        CoreError::internal(format!("could not encode resume metadata: {error}"))
    })?;
    std::fs::write(metadata_path(destination), bytes)
        .map_err(|error| CoreError::internal(format!("could not write resume metadata: {error}")))
}

pub fn read_metadata(destination: &Path) -> CoreResult<PartialTransferData> {
    let bytes = std::fs::read(metadata_path(destination)).map_err(|error| {
        CoreError::NotFound(format!(
            "resume metadata for {}: {error}",
            destination.display()
        ))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::internal(format!("could not parse resume metadata: {error}")))
}

/// Remove the partial data and its metadata sidecar.
pub fn remove_artifacts(destination: &Path) -> CoreResult<()> {
    for path in [partial_path(destination), metadata_path(destination)] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(CoreError::internal(format!(
                    "could not remove partial data: {error}"
                )));
            }
        }
    }
    Ok(())
}

/// Verify that a partial transfer can safely resume.
pub fn verify(
    partial: &PartialTransferData,
    source_size: Option<u64>,
    source_modified_at: Option<DateTime<Utc>>,
    destination_offset: u64,
) -> CoreResult<()> {
    verify_resume(partial, source_size, source_modified_at, destination_offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_artifacts_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("report.pdf");
        let partial = new_partial(
            Uuid::new_v4(),
            Some(&destination),
            Some("/remote/report.pdf"),
            128,
            Some(256),
            None,
        );

        write_metadata(&destination, &partial).unwrap();
        let loaded = read_metadata(&destination).unwrap();
        assert_eq!(loaded.offset, 128);
        assert_eq!(loaded.source_size, Some(256));

        std::fs::write(partial_path(&destination), b"partial").unwrap();
        remove_artifacts(&destination).unwrap();
        assert!(!partial_path(&destination).exists());
        assert!(!metadata_path(&destination).exists());
    }
}
