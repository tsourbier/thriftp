mod support;

use std::time::{Duration, Instant};

use thriftp_core::favorites::DirectoryFavoriteService;
use thriftp_core::model::DirectoryFavorite;
use uuid::Uuid;

#[test]
fn local_listing_of_ten_thousand_entries_stays_interactive() {
    let directory = tempfile::tempdir().unwrap();
    for index in 0..10_000 {
        std::fs::write(directory.path().join(format!("file-{index:05}")), b"x").unwrap();
    }
    let started = Instant::now();
    let entries = thriftp_core::browser::list_local(directory.path()).unwrap();
    let elapsed = started.elapsed();
    assert_eq!(entries.len(), 10_000);
    assert!(
        elapsed < Duration::from_secs(10),
        "listing took {elapsed:?}"
    );
}

#[tokio::test]
async fn four_transfers_run_concurrently_with_bounded_buffers() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.bin");
    std::fs::write(&source, vec![7_u8; 512 * 1024]).unwrap();
    let started = Instant::now();
    let mut handles = Vec::new();
    for index in 0..4 {
        let source = source.clone();
        let destination = directory.path().join(format!("destination-{index}.bin"));
        handles.push(tokio::spawn(async move {
            thriftp_core::transfers::io::copy_local(&source, &destination, 0, |_| true)
                .await
                .unwrap()
        }));
    }
    for handle in handles {
        assert_eq!(handle.await.unwrap(), 512 * 1024);
    }
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn five_hundred_favorites_remain_usable() {
    let profile = Uuid::new_v4();
    let mut service = DirectoryFavoriteService::default();
    for index in 0..500 {
        service
            .create(DirectoryFavorite::new(
                profile,
                format!("/path/{index}"),
                format!("Favorite {index}"),
                index,
            ))
            .unwrap();
    }
    let started = Instant::now();
    let favorites = service.list(Some(profile));
    assert_eq!(favorites.len(), 500);
    assert!(started.elapsed() < Duration::from_secs(2));
}
