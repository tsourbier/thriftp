//! Plain-FTP warnings are emitted consistently for MCP results.

use thriftp_core::model::Protocol;
use thriftp_core::protocol::plain_ftp_warning;

#[test]
fn plain_ftp_warning_is_machine_readable() {
    let warning = plain_ftp_warning(Protocol::Ftp).expect("warning");
    assert_eq!(warning.code, "unencrypted_transport");
    assert!(!warning.message.is_empty());
}

#[test]
fn encrypted_protocols_never_warn() {
    assert!(plain_ftp_warning(Protocol::FtpsExplicit).is_none());
    assert!(plain_ftp_warning(Protocol::FtpsImplicit).is_none());
    assert!(plain_ftp_warning(Protocol::Sftp).is_none());
}
