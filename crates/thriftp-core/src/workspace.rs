//! Workspace layout application service.

use std::sync::Arc;

use crate::error::CoreResult;
use crate::model::WorkspaceLayout;
use crate::repository::{AppState, MetadataStore};

/// Read and persist the singleton workspace layout.
pub struct WorkspaceService {
    store: Arc<dyn MetadataStore>,
}

impl WorkspaceService {
    /// Create a service backed by the supplied metadata store.
    pub fn new(store: Arc<dyn MetadataStore>) -> Self {
        Self { store }
    }

    /// Return the normalized persisted layout, or defaults.
    pub fn get(&self) -> CoreResult<WorkspaceLayout> {
        Ok(self.store.load()?.workspace_layout.normalized())
    }

    /// Normalize and persist the supplied layout.
    pub fn set(&self, layout: WorkspaceLayout) -> CoreResult<WorkspaceLayout> {
        let mut state = self.store.load()?;
        let normalized = layout.normalized();
        state.workspace_layout = normalized.clone();
        self.store.save(&state)?;
        Ok(normalized)
    }

    /// Replace the layout on an existing state value without persisting.
    ///
    /// This is used by adapters that already own an atomic metadata write.
    pub fn apply(state: &mut AppState, layout: WorkspaceLayout) -> WorkspaceLayout {
        let normalized = layout.normalized();
        state.workspace_layout = normalized.clone();
        normalized
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Arc;

    use super::WorkspaceService;
    use crate::model::{HubTab, WorkspaceLayout};
    use crate::repository::MemoryStore;

    #[test]
    fn defaults_are_returned_for_empty_store() {
        let service = WorkspaceService::new(Arc::new(MemoryStore::default()));
        let layout = service.get().expect("layout");
        assert_eq!(layout, WorkspaceLayout::default());
    }

    #[test]
    fn set_normalizes_and_persists() {
        let store = Arc::new(MemoryStore::default());
        let service = WorkspaceService::new(store.clone());
        let layout = WorkspaceLayout {
            split_ratio: 0.99,
            sidebar_visible: false,
            sidebar_sections: BTreeMap::new(),
            hub_tab: HubTab::History,
        };
        let saved = service.set(layout).expect("saved");
        assert_eq!(saved.split_ratio, 0.8);
        assert_eq!(saved.hub_tab, HubTab::History);
        assert_eq!(service.get().expect("get"), saved);
    }
}
