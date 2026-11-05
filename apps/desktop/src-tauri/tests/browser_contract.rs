use chrono::Utc;
use thriftp_core::browser::listing::{ListingBatch, ListingState};
use thriftp_core::error::CoreError;
use thriftp_core::model::{EntryType, FileEntry, ProtocolCapabilities};

use thriftp_desktop_lib::commands::browser::{DirectoryErrorPayload, batch_payload};
use thriftp_desktop_lib::error::DesktopError;

fn entry() -> FileEntry {
    FileEntry {
        name: "report.txt".to_owned(),
        path: "/remote/report.txt".to_owned(),
        entry_type: EntryType::File,
        size: Some(42),
        modified_at: Some(Utc::now()),
        permissions: Some("644".to_owned()),
        ownership: None,
        hidden: false,
        symlink_target: None,
        capabilities: ProtocolCapabilities::default(),
    }
}

#[test]
fn batch_payload_matches_the_streaming_contract() {
    let now = Utc::now();
    let payload = batch_payload(
        "listing-1",
        ListingBatch {
            entries: vec![entry()],
            state: ListingState::Complete,
            first_entry_at: Some(now),
            completed_at: Some(now),
        },
    );
    let json = serde_json::to_value(payload).unwrap();

    assert_eq!(json["listingId"], "listing-1");
    assert_eq!(json["state"], "complete");
    assert_eq!(json["entries"][0]["entryType"], "file");
    assert_eq!(json["entries"][0]["size"], 42);
    assert_eq!(json["entries"][0]["name"], "report.txt");
    assert!(json.get("firstEntryAt").is_some());
    assert!(json.get("completedAt").is_some());
    assert!(json["entries"][0].get("capabilities").is_some());
}

#[test]
fn partial_batches_omit_timestamps_until_available() {
    let payload = batch_payload(
        "listing-2",
        ListingBatch {
            entries: Vec::new(),
            state: ListingState::Partial,
            first_entry_at: None,
            completed_at: None,
        },
    );
    let json = serde_json::to_value(payload).unwrap();
    assert!(json.get("firstEntryAt").is_none());
    assert!(json.get("completedAt").is_none());
}

#[test]
fn listing_errors_use_the_desktop_error_contract() {
    let payload = DirectoryErrorPayload {
        listing_id: "listing-3".to_owned(),
        error: DesktopError::from(CoreError::PermissionDenied("nope".to_owned())),
    };
    let json = serde_json::to_value(payload).unwrap();
    assert_eq!(json["listingId"], "listing-3");
    assert_eq!(json["error"]["code"], "permission_denied");
    assert_eq!(json["error"]["retryable"], false);
}
