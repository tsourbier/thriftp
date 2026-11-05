//! Streaming transfer engine, conflict handling, and resume support.

pub mod conflict;
pub mod control;
pub mod engine;
pub mod io;
pub mod recursive;
pub mod resume;
pub mod stream;

pub use conflict::{ConflictChoice, ConflictPolicy, ConflictResolver};
pub use control::{ProgressSnapshot, ProgressTracker, TransferControl};
pub use engine::{
    ProgressObserver, TransferRun, ensure_streaming_supported, execute_download, execute_upload,
    execute_upload_to,
};
pub use recursive::{PlannedEntry, SymlinkPolicy, plan_download, plan_upload};
pub use resume::{new_partial, partial_path, remove_artifacts, verify};
pub use stream::{StreamStats, copy_chunked};
