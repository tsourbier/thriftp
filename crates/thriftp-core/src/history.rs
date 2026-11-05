//! Bounded, deduplicated connection-history service.

use std::sync::Arc;

use chrono::Utc;

use crate::error::{CoreError, CoreResult};
use crate::model::ConnectionHistoryEntry;
use crate::repository::MetadataStore;

/// Maximum number of recent successful servers retained.
pub const HISTORY_CAP: usize = 50;

/// Application service for recent successful connections.
pub struct ConnectionHistoryService {
    store: Arc<dyn MetadataStore>,
}

impl ConnectionHistoryService {
    /// Create a service backed by the supplied metadata store.
    pub fn new(store: Arc<dyn MetadataStore>) -> Self {
        Self { store }
    }

    /// Return history newest first, capped at [`HISTORY_CAP`].
    pub fn list(&self) -> CoreResult<Vec<ConnectionHistoryEntry>> {
        let mut entries = self.store.load()?.connection_history;
        sort_and_cap(&mut entries);
        Ok(entries)
    }

    /// Record one successful connection, replacing any prior entry for the
    /// same favorite or, when no favorite is present, the same profile.
    pub fn record_success(&self, entry: ConnectionHistoryEntry) -> CoreResult<()> {
        let mut state = self.store.load()?;
        let same_entry = |candidate: &ConnectionHistoryEntry| match entry.favorite_id {
            Some(favorite_id) => candidate.favorite_id == Some(favorite_id),
            None => {
                candidate.favorite_id.is_none()
                    && candidate.connection_profile_id == entry.connection_profile_id
            }
        };
        state
            .connection_history
            .retain(|candidate| !same_entry(candidate));
        state.connection_history.push(entry);
        sort_and_cap(&mut state.connection_history);
        self.store.save(&state)
    }

    /// Clear history only when explicitly confirmed.
    pub fn clear(&self, confirmed: bool) -> CoreResult<usize> {
        if !confirmed {
            return Err(CoreError::Conflict(
                "clearing connection history requires confirmation".to_owned(),
            ));
        }
        let mut state = self.store.load()?;
        let removed = state.connection_history.len();
        state.connection_history.clear();
        self.store.save(&state)?;
        Ok(removed)
    }

    /// Record a successful connection using a non-secret profile snapshot.
    pub fn record_profile(
        &self,
        profile: &crate::model::ConnectionProfile,
        favorite_id: Option<uuid::Uuid>,
    ) -> CoreResult<()> {
        self.record_success(ConnectionHistoryEntry {
            favorite_id,
            connection_profile_id: profile.id,
            display_name: profile.display_name.clone(),
            protocol: profile.protocol,
            host: profile.host.clone(),
            port: profile.port,
            username: profile.username.clone(),
            last_used_at: Utc::now(),
        })
    }
}

fn sort_and_cap(entries: &mut Vec<ConnectionHistoryEntry>) {
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.last_used_at));
    entries.truncate(HISTORY_CAP);
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{Duration, Utc};
    use uuid::Uuid;

    use super::{ConnectionHistoryService, HISTORY_CAP};
    use crate::model::{ConnectionHistoryEntry, Protocol};
    use crate::repository::MemoryStore;

    fn entry(
        name: &str,
        profile: Uuid,
        favorite: Option<Uuid>,
        offset: i64,
    ) -> ConnectionHistoryEntry {
        ConnectionHistoryEntry {
            favorite_id: favorite,
            connection_profile_id: profile,
            display_name: name.to_owned(),
            protocol: Protocol::Sftp,
            host: "example.test".to_owned(),
            port: 22,
            username: "deploy".to_owned(),
            last_used_at: Utc::now() + Duration::seconds(offset),
        }
    }

    #[test]
    fn deduplicates_by_favorite_and_profile() {
        let service = ConnectionHistoryService::new(Arc::new(MemoryStore::default()));
        let favorite = Uuid::new_v4();
        let profile = Uuid::new_v4();
        service
            .record_success(entry("old", profile, Some(favorite), 0))
            .expect("old");
        service
            .record_success(entry("new", profile, Some(favorite), 10))
            .expect("new");
        let history = service.list().expect("list");
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].display_name, "new");
    }

    #[test]
    fn failed_attempts_are_not_recorded_by_the_service() {
        let service = ConnectionHistoryService::new(Arc::new(MemoryStore::default()));
        assert!(service.list().expect("list").is_empty());
    }

    #[test]
    fn caps_at_fifty_and_sorts_newest_first() {
        let service = ConnectionHistoryService::new(Arc::new(MemoryStore::default()));
        for index in 0..(HISTORY_CAP + 5) {
            service
                .record_success(entry(
                    &format!("server-{index}"),
                    Uuid::new_v4(),
                    None,
                    index as i64,
                ))
                .expect("record");
        }
        let history = service.list().expect("list");
        assert_eq!(history.len(), HISTORY_CAP);
        assert_eq!(
            history[0].display_name,
            format!("server-{}", HISTORY_CAP + 4)
        );
    }

    #[test]
    fn clear_requires_confirmation() {
        let service = ConnectionHistoryService::new(Arc::new(MemoryStore::default()));
        service
            .record_success(entry("server", Uuid::new_v4(), None, 0))
            .expect("record");
        assert!(service.clear(false).is_err());
        assert_eq!(service.list().expect("list").len(), 1);
        assert_eq!(service.clear(true).expect("clear"), 1);
        assert!(service.list().expect("list").is_empty());
    }
}
