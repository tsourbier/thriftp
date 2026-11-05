//! Protocol adapters and interfaces.
//!
//! The `RemoteFs` contract lives in the Tauri-independent core so connection,
//! browsing, and transfer services can depend on it without depending on a
//! concrete protocol implementation.

pub mod ftp;
pub mod sftp;
pub mod traits {
    pub use thriftp_core::protocol::{RemoteFs, RemoteFsFactory};
}

pub use ftp::{FtpAdapter, FtpFactory, parse_mlsd_line};
pub use sftp::{SftpAdapter, key_fingerprint, normalize_fingerprint, trusted_fingerprint};
pub use traits::{RemoteFs, RemoteFsFactory};

use thriftp_core::model::Protocol;

pub struct ProtocolFactory;

impl RemoteFsFactory for ProtocolFactory {
    fn create(&self, protocol: Protocol) -> Box<dyn RemoteFs> {
        match protocol {
            Protocol::Ftp | Protocol::FtpsExplicit | Protocol::FtpsImplicit => {
                Box::new(FtpAdapter::new(protocol))
            }
            Protocol::Sftp => Box::new(SftpAdapter::new()),
        }
    }
}
