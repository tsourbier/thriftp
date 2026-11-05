//! End-to-end MCP discovery smoke scenario.

#[path = "../support/mod.rs"]
mod support;

use serde_json::json;
use tempfile::TempDir;

#[test]
fn mcp_lists_tools_before_enrollment() {
    if !support::mcp_harness::binary().exists() {
        return;
    }
    let home = TempDir::new().unwrap();
    let mut store =
        thriftp_core::automation::AutomationStore::open(home.path().join("automation.json"))
            .unwrap();
    store.enable("standalone").unwrap();
    let mut session = support::mcp_harness::McpSession::start(home.path());
    let response = session.call(json!({
        "jsonrpc":"2.0","id":1,"method":"tools/list","params":{}
    }));
    let tools = response["result"]["tools"].as_array().unwrap();
    assert!(
        tools
            .iter()
            .any(|tool| tool["name"] == "thriftp.automation.enroll")
    );
}
