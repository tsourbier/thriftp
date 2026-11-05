//! Destructive confirmation and pre-approval policy contract tests.

use chrono::{Duration, Utc};
use thriftp_core::automation::approval::ApprovalManager;
use thriftp_core::error::CoreError;
use uuid::Uuid;

#[test]
fn confirmation_ids_are_single_use_and_session_bound() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    let confirmation = manager
        .request(grant, Some("session-a"), "files.delete", Some("/srv/a"), 60)
        .unwrap();
    assert_eq!(
        manager
            .consume(
                confirmation.id,
                grant,
                Some("session-b"),
                "files.delete",
                Some("/srv/a")
            )
            .unwrap_err(),
        CoreError::ConfirmationRequired
    );
    manager
        .consume(
            confirmation.id,
            grant,
            Some("session-a"),
            "files.delete",
            Some("/srv/a"),
        )
        .unwrap();
    assert_eq!(
        manager
            .consume(
                confirmation.id,
                grant,
                Some("session-a"),
                "files.delete",
                Some("/srv/a")
            )
            .unwrap_err(),
        CoreError::ConfirmationExpired
    );
}

#[test]
fn expired_confirmation_is_rejected() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    let now = Utc::now();
    let confirmation = manager
        .request_at(grant, None, "files.delete", None, now, 1)
        .unwrap();
    assert_eq!(
        manager
            .consume_at(
                confirmation.id,
                grant,
                None,
                "files.delete",
                None,
                now + Duration::seconds(2)
            )
            .unwrap_err(),
        CoreError::ConfirmationExpired
    );
}

#[test]
fn policy_use_is_scoped_by_operation_and_target() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    manager
        .add_policy(
            grant,
            vec!["directories.delete".to_owned()],
            Some("/srv/app"),
            None,
            false,
        )
        .unwrap();
    assert!(manager.policy_allows(grant, "directories.delete", Some("/srv/app/cache")));
    assert!(!manager.policy_allows(grant, "directories.delete", Some("/srv/other")));
    assert!(!manager.policy_allows(grant, "files.delete", Some("/srv/app/cache")));
}
