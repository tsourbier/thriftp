//! Deterministic ordering for server favorites.

use crate::model::{FavoritePreferences, FavoriteSortKey};

use super::ServerFavoriteView;

/// Sort server favorites according to the persisted preferences.
///
/// Pinned entries come first when `pinnedFirst` is enabled. The selected key
/// orders each section; protocol, host, and username are stable tie-breakers so
/// the list does not jump between loads.
pub fn sort_views(views: &mut [ServerFavoriteView], preferences: &FavoritePreferences) {
    views.sort_by(|a, b| {
        let pinned = if preferences.pinned_first {
            b.pinned.cmp(&a.pinned)
        } else {
            std::cmp::Ordering::Equal
        };
        let primary = match preferences.sort_key {
            FavoriteSortKey::Name => a
                .display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase()),
            FavoriteSortKey::Protocol => {
                format!("{:?}", a.protocol).cmp(&format!("{:?}", b.protocol))
            }
            FavoriteSortKey::Host => a.host.cmp(&b.host),
            FavoriteSortKey::LastUsed => b.last_used_at.cmp(&a.last_used_at),
        };
        pinned
            .then(primary)
            .then_with(|| format!("{:?}", a.protocol).cmp(&format!("{:?}", b.protocol)))
            .then_with(|| a.host.cmp(&b.host))
            .then_with(|| a.username.cmp(&b.username))
    });
}
