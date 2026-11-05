//! Destructive confirmation and pre-approval policy behavior.

use chrono::{Duration, Utc};
use thriftp_core::automation::approval::ApprovalManager;
use thriftp_core::error::CoreError;
use uuid::Uuid;

#[test]
fn confirmation_is_single_use() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    let confirmation = manager
        .request(
            grant,
            Some("session"),
            "files.delete",
            Some("/srv/app/a"),
            120,
        )
        .unwrap();
    manager
        .consume(
            confirmation.id,
            grant,
            Some("session"),
            "files.delete",
            Some("/srv/app/a"),
        )
        .unwrap();
    assert_eq!(
        manager
            .consume(
                confirmation.id,
                grant,
                Some("session"),
                "files.delete",
                Some("/srv/app/a")
            )
            .unwrap_err(),
        CoreError::ConfirmationExpired
    );
}

#[test]
fn confirmation_expires_and_is_bound_to_the_target() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    let now = Utc::now();
    let confirmation = manager
        .request_at(
            grant,
            Some("session"),
            "directories.delete",
            Some("/srv/app"),
            now,
            120,
        )
        .unwrap();
    assert_eq!(
        manager
            .consume_at(
                confirmation.id,
                grant,
                Some("session"),
                "directories.delete",
                Some("/srv/other"),
                now + Duration::seconds(1)
            )
            .unwrap_err(),
        CoreError::ConfirmationRequired
    );
    assert_eq!(
        manager
            .consume_at(
                confirmation.id,
                grant,
                Some("session"),
                "directories.delete",
                Some("/srv/app"),
                now + Duration::seconds(121)
            )
            .unwrap_err(),
        CoreError::ConfirmationExpired
    );
}

#[test]
fn headless_callers_fail_closed() {
    let manager = ApprovalManager::new();
    assert_eq!(
        manager.require_or_deny(false).unwrap_err(),
        CoreError::ConfirmationRequired
    );
    assert!(manager.require_or_deny(true).is_ok());
}

#[test]
fn pre_approval_policy_is_scoped_and_records_use() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    let policy = manager
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
    manager.revoke_policies(grant);
    assert!(!manager.policy_allows(grant, "directories.delete", Some("/srv/app/cache")));
    let _ = policy;
}

#[test]
fn single_use_policy_cannot_be_reused() {
    let manager = ApprovalManager::new();
    let grant = Uuid::new_v4();
    manager
        .add_policy(grant, vec!["files.delete".to_owned()], None, None, true)
        .unwrap();
    assert!(manager.policy_allows(grant, "files.delete", Some("/a")));
    assert!(!manager.policy_allows(grant, "files.delete", Some("/b")));
}
