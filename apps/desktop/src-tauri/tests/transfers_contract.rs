mod support;

use chrono::{Duration, Utc};
use thriftp_core::model::{TransferDirection, TransferState};

use thriftp_desktop_lib::commands::transfers::transfer_state_payload;

#[test]
fn state_payload_includes_connection_and_timestamps() {
    let started = Utc::now();
    let completed = started + Duration::seconds(5);
    let mut transfer = support::transfer_fixture(TransferDirection::Download, 1024);
    transfer.state = TransferState::Completed;
    transfer.transferred_bytes = 1024;
    transfer.started_at = Some(started);
    transfer.completed_at = Some(completed);

    let json = serde_json::to_value(transfer_state_payload(&transfer, None)).unwrap();

    assert_eq!(json["transferId"], transfer.id.to_string());
    assert_eq!(
        json["connectionProfileId"],
        transfer.connection_profile_id.to_string()
    );
    assert_eq!(json["direction"], "download");
    assert_eq!(json["state"], "completed");
    assert_eq!(json["transferredBytes"], 1024);
    assert_eq!(json["totalBytes"], 1024);
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(json["startedAt"].as_str().unwrap())
            .unwrap()
            .timestamp_millis(),
        started.timestamp_millis()
    );
    assert_eq!(
        chrono::DateTime::parse_from_rfc3339(json["completedAt"].as_str().unwrap())
            .unwrap()
            .timestamp_millis(),
        completed.timestamp_millis()
    );
    assert!(json.get("error").is_none());
}
