//! Single-owner transfer host, local control channel, and shared-state recovery.
//!
//! The host owns the transfer queue while it runs. Other processes connect to
//! the local-only control endpoint as control clients. No daemon is started.

pub mod client;
pub mod lock;
pub mod model;
pub mod protocol;
pub mod runtime;
pub mod server;

pub use client::{HostClient, HostEventStream};
pub use lock::HostLock;
pub use model::{HostRole, HostStatus};
pub use runtime::HostRuntime;
pub use server::{HostServer, HostServices, read_host_status};
