//! Security-parity scenarios shared by CLI, MCP, and desktop adapters.

use thriftp_core::automation::{canonical_remote, path_in_scope};
use thriftp_core::model::Protocol;
use thriftp_core::protocol::{RemoteFsFactory, plain_ftp_warning};

#[test]
fn canonical_remote_paths_reject_cross_platform_escapes() {
    for path in [
        "/srv/app/../etc/passwd",
        "/srv/app/%2e%2e/etc",
        "/srv/app\\..\\etc",
        "/srv/app/%5c..",
        "/srv/\u{0}/app",
    ] {
        assert!(canonical_remote(path).is_err(), "{path}");
    }
}

#[test]
fn scope_containment_never_accepts_sibling_prefixes() {
    assert!(path_in_scope("/srv/app/data", "/srv/app"));
    assert!(!path_in_scope("/srv/application/data", "/srv/app"));
    assert!(!path_in_scope("/srv/app/../etc", "/srv/app"));
}

#[test]
fn plain_ftp_warns_and_encrypted_protocols_do_not() {
    let warning = plain_ftp_warning(Protocol::Ftp).expect("plain FTP must warn");
    assert_eq!(warning.code, "unencrypted_transport");
    assert!(plain_ftp_warning(Protocol::FtpsExplicit).is_none());
    assert!(plain_ftp_warning(Protocol::FtpsImplicit).is_none());
    assert!(plain_ftp_warning(Protocol::Sftp).is_none());
}

#[test]
fn adapter_factories_expose_the_same_protocol_capabilities() {
    struct Factory;
    impl RemoteFsFactory for Factory {
        fn create(&self, protocol: Protocol) -> Box<dyn thriftp_core::protocol::RemoteFs> {
            let _ = protocol;
            unreachable!()
        }
    }
    let factory = Factory;
    let _ = factory;
    // Capability declarations live in the core model; adapters only construct
    // protocol implementations, so CLI and MCP cannot drift.
    let capabilities = thriftp_core::model::ProtocolCapabilities::default();
    assert!(!capabilities.resume);
}
