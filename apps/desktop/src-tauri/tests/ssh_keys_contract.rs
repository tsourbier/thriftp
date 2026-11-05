use std::fs;

use tempfile::TempDir;
use thriftp_core::ssh_keys::{list_candidates, validate_key_path};

#[test]
fn ssh_key_results_never_expose_key_contents() {
    let home = TempDir::new().unwrap();
    let key = home.path().join("id_ed25519");
    fs::write(&key, "PRIVATE-KEY-CONTENT").unwrap();

    let listing = list_candidates(home.path());
    assert!(listing.exists);
    assert!(listing.readable);
    assert_eq!(listing.candidates.len(), 1);
    let serialized = serde_json::to_string(&listing).unwrap();
    assert!(!serialized.contains("PRIVATE-KEY-CONTENT"));

    let validation = validate_key_path(&key.display().to_string()).unwrap();
    assert!(validation.valid);
    assert!(!format!("{validation:?}").contains("PRIVATE-KEY-CONTENT"));
}

#[test]
fn public_keys_are_rejected_without_reading_contents() {
    let home = TempDir::new().unwrap();
    let public = home.path().join("id_ed25519.pub");
    fs::write(&public, "PUBLIC-KEY-CONTENT").unwrap();
    let result = validate_key_path(&public.display().to_string()).unwrap();
    assert!(!result.valid);
    assert_eq!(result.code.as_deref(), Some("public_key"));
    assert!(!format!("{result:?}").contains("PUBLIC-KEY-CONTENT"));
}
