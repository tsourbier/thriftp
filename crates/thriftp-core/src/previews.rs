//! Preview policy for local and remote files.
//!
//! Previewing must never download an unbounded remote file or open a file the
//! user has not consented to download.

pub mod service;

pub use service::{
    MAX_INLINE_BYTES, MAX_REMOTE_BYTES, PreviewContent, PreviewMode, PreviewPlan, classify,
    plan_local, plan_remote,
};
