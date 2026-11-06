mod support;

use support::PermissionTreeRemote;
use thriftp_core::browser::permissions::chmod_remote_recursive;

#[tokio::test]
async fn recursive_permissions_cover_descendants_and_skip_symlinks() {
    let mut remote = PermissionTreeRemote::new(None);
    let result = chmod_remote_recursive(&mut remote, "/root", 0o700)
        .await
        .unwrap();
    assert_eq!(result.changed, 3);
    assert_eq!(result.skipped, 1);
    assert!(result.failed.is_empty());
}

#[tokio::test]
async fn recursive_permissions_report_partial_failures() {
    let mut remote = PermissionTreeRemote::new(Some("/root/sub/nested.txt".to_owned()));
    let result = chmod_remote_recursive(&mut remote, "/root", 0o700)
        .await
        .unwrap();
    assert_eq!(result.changed, 2);
    assert_eq!(result.failed.len(), 1);
    assert_eq!(result.failed[0].path, "/root/sub/nested.txt");
}
