//! End-to-end CLI automation smoke scenario.

#[path = "../support/mod.rs"]
mod support;

use tempfile::TempDir;
use thriftp_core::automation::AutomationStore;

#[test]
fn cli_harness_runs_the_same_core_state_as_the_desktop() {
    let home = TempDir::new().unwrap();
    let mut store = AutomationStore::open(home.path().join("automation.json")).unwrap();
    store.enable("standalone").unwrap();
    let reopened = AutomationStore::open(home.path().join("automation.json")).unwrap();
    assert!(reopened.state.settings.mcp_enabled);
}
