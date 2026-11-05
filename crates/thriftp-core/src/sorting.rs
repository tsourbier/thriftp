use std::cmp::Ordering;

use crate::model::{
    ConnectionProfile, DirectoryFavorite, FileEntry, SortKey, Transfer, TransferState,
};

pub fn sort_profiles(profiles: &mut [ConnectionProfile]) {
    profiles.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
            .then_with(|| format!("{:?}", a.protocol).cmp(&format!("{:?}", b.protocol)))
            .then_with(|| a.host.cmp(&b.host))
            .then_with(|| a.username.cmp(&b.username))
    });
}

pub fn sort_directory_favorites(favorites: &mut [DirectoryFavorite]) {
    favorites.sort_by(|a, b| {
        a.sort_order
            .cmp(&b.sort_order)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
}

pub fn sort_file_entries(entries: &mut [FileEntry], key: SortKey) {
    entries.sort_by(|a, b| {
        let primary = match key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
            SortKey::Date => a.modified_at.cmp(&b.modified_at),
            SortKey::Type => format!("{:?}", a.entry_type).cmp(&format!("{:?}", b.entry_type)),
        };
        primary.then_with(|| a.name.cmp(&b.name))
    });
}

pub fn transfer_priority_cmp(a: &Transfer, b: &Transfer) -> Ordering {
    let rank = |state: TransferState| match state {
        TransferState::Active => 0,
        TransferState::Paused => 1,
        TransferState::Pending => 2,
        _ => 3,
    };
    rank(a.state)
        .cmp(&rank(b.state))
        .then_with(|| format!("{:?}", b.priority).cmp(&format!("{:?}", a.priority)))
        .then_with(|| a.order.cmp(&b.order))
        .then_with(|| a.created_at.cmp(&b.created_at))
}
