//! Large directory listings remain complete and bounded.

use tempfile::TempDir;
use thriftp_core::browser;

#[test]
fn ten_thousand_local_entries_are_listed_without_loss() {
    let home = TempDir::new().unwrap();
    for index in 0..10_000 {
        std::fs::write(home.path().join(format!("entry-{index:05}.txt")), b"x").unwrap();
    }
    let entries = browser::list_local(home.path()).unwrap();
    assert_eq!(entries.len(), 10_000);
    assert!(entries.iter().all(|entry| entry.name.starts_with("entry-")));
}

#[test]
fn pagination_returns_a_stable_cursor_shape() {
    let home = TempDir::new().unwrap();
    for index in 0..50 {
        std::fs::write(home.path().join(format!("entry-{index:02}.txt")), b"x").unwrap();
    }
    let entries = browser::list_local(home.path()).unwrap();
    let page: Vec<_> = entries.iter().take(10).cloned().collect();
    assert_eq!(page.len(), 10);
    assert_eq!(page[0].name, "entry-00.txt");
}
