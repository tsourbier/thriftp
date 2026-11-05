//! Performance guard for sorting and rendering large server-favorite lists.

use chrono::Utc;
use criterion::{Criterion, criterion_group, criterion_main};
use thriftp_core::model::{
    AuthMethod, CredentialPolicy, FavoritePreferences, FavoriteSortKey, Protocol,
};
use thriftp_core::server_favorites::ServerFavoriteView;
use thriftp_core::server_favorites::sorting::sort_views;
use uuid::Uuid;

fn favorite(index: usize) -> ServerFavoriteView {
    let now = Utc::now();
    ServerFavoriteView {
        id: Uuid::new_v4(),
        connection_profile_id: Uuid::new_v4(),
        display_name: format!("server-{index:03}"),
        group_label: Some(format!("group-{}", index % 8)),
        protocol: match index % 4 {
            0 => Protocol::Ftp,
            1 => Protocol::FtpsExplicit,
            2 => Protocol::FtpsImplicit,
            _ => Protocol::Sftp,
        },
        host: format!("host-{:03}.example", (index * 37) % 500),
        port: 22,
        username: format!("user-{}", index % 12),
        auth_method: if index.is_multiple_of(5) {
            AuthMethod::SshPrivateKey
        } else {
            AuthMethod::Password
        },
        timeout_seconds: 30,
        ssh_key_ref: None,
        pinned: index.is_multiple_of(11),
        last_used_at: (index.is_multiple_of(3)).then_some(now),
        credential_policy: CredentialPolicy::Ask,
        has_saved_credential: false,
        created_at: now,
        updated_at: now,
        credential_warning: None,
    }
}

fn sort_500_favorites(c: &mut Criterion) {
    let mut views: Vec<_> = (0..500).map(favorite).collect();
    c.bench_function("sort_500_server_favorites", |b| {
        b.iter(|| {
            views.reverse();
            sort_views(
                &mut views,
                &FavoritePreferences {
                    sort_key: FavoriteSortKey::Name,
                    pinned_first: true,
                },
            );
        });
    });
}

#[test]
fn sorting_500_favorites_stays_below_100_ms() {
    let mut views: Vec<_> = (0..500).map(favorite).collect();
    let started = Instant::now();
    for sort_key in [
        FavoriteSortKey::Name,
        FavoriteSortKey::Protocol,
        FavoriteSortKey::Host,
        FavoriteSortKey::LastUsed,
    ] {
        views.reverse();
        sort_views(
            &mut views,
            &FavoritePreferences {
                sort_key,
                pinned_first: true,
            },
        );
    }
    assert!(
        started.elapsed() < Duration::from_millis(100),
        "sorting 500 favorites took {:?}",
        started.elapsed()
    );
}

criterion_group!(benches, sort_500_favorites);
criterion_main!(benches);
