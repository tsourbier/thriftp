use thriftp_core::automation::{canonical_remote, path_in_scope};

#[test]
fn shared_scope_rules_hold_across_adapters() {
    assert!(path_in_scope("/srv/app/file", "/srv/app"));
    assert!(!path_in_scope("/srv/app2/file", "/srv/app"));
    assert!(canonical_remote("/srv/app/../etc").is_err());
    assert!(canonical_remote("/srv/%2e%2e/etc").is_err());
}
