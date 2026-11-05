//! Versioning and migrations for the shared favorites metadata document.
//!
//! Server favorites, credential references, and their preferences live inside
//! the versioned metadata document. This module owns the ordered migration
//! ladder so adapters never interpret an unsupported version.

use thriftp_core::error::{CoreError, CoreResult};

use crate::SCHEMA_VERSION;

/// Migrate a stored schema version to the current version.
///
/// Migrations are ordered and additive. Unknown future versions fail closed
/// instead of being interpreted with the wrong shape.
pub fn migrate_version(version: u32) -> CoreResult<u32> {
    match version {
        // Version 0 documents predate the explicit schema field; their state
        // shape is compatible, so the migration only records the new version.
        0 => migrate_version(1),
        // Version 1 gains workspace layout, connection history, and optional
        // favorite paths through serde defaults, so the migration is additive.
        1 => Ok(SCHEMA_VERSION),
        SCHEMA_VERSION => Ok(SCHEMA_VERSION),
        other => Err(CoreError::internal(format!(
            "unsupported metadata schema version {other}"
        ))),
    }
}
