use crate::model::BrowsingSession;
use crate::repository::AppState;

/// Persist and restore browsing tabs.
#[derive(Default)]
pub struct TabService {
    tabs: Vec<BrowsingSession>,
}

impl TabService {
    #[must_use]
    pub fn from_state(state: &AppState, restore: bool) -> Self {
        let tabs = if restore {
            state
                .tabs
                .iter()
                .cloned()
                .map(|mut tab| {
                    // Restored tabs must start disconnected; reconnecting is an
                    // explicit user action and must not block other tabs.
                    tab.restored = true;
                    tab
                })
                .collect()
        } else {
            Vec::new()
        };
        Self { tabs }
    }

    #[must_use]
    pub fn list(&self) -> Vec<BrowsingSession> {
        self.tabs.clone()
    }

    pub fn save(&mut self, tabs: Vec<BrowsingSession>) {
        self.tabs = tabs;
    }

    pub fn apply_to(&self, state: &mut AppState) {
        state.tabs = self.tabs.clone();
    }

    /// Tabs whose profile is no longer connected should be shown as disconnected.
    #[must_use]
    pub fn disconnected_ids(&self, connected_profiles: &[uuid::Uuid]) -> Vec<uuid::Uuid> {
        self.tabs
            .iter()
            .filter_map(|tab| {
                tab.connection_profile_id
                    .filter(|id| !connected_profiles.contains(id))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn restore_marks_tabs_disconnected_and_respects_the_preference() {
        let profile = Uuid::new_v4();
        let mut state = AppState::default();
        state
            .tabs
            .push(BrowsingSession::new(Some(profile), "/home"));
        let service = TabService::from_state(&state, true);
        assert_eq!(service.list().len(), 1);
        assert!(service.list()[0].restored);
        assert_eq!(service.disconnected_ids(&[]), vec![profile]);

        let service = TabService::from_state(&state, false);
        assert!(service.list().is_empty());
    }

    #[test]
    fn saving_tabs_updates_the_metadata_state() {
        let mut service = TabService::default();
        service.save(vec![BrowsingSession::new(None, "/tmp")]);
        let mut state = AppState::default();
        service.apply_to(&mut state);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].current_path, "/tmp");
    }
}
