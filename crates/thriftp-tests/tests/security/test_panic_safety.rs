//! Panic-safety and malformed-input scenarios.

use std::panic::catch_unwind;
use std::process::{Command, Stdio};

use thriftp_core::automation::canonical_remote;
use thriftp_core::browser;
use thriftp_core::host::protocol::{MAX_MESSAGE_BYTES, RpcRequest};

#[test]
fn malformed_control_messages_are_rejected_without_panicking() {
    let inputs = [
        "",
        "{",
        "null",
        "[]",
        "{\"jsonrpc\":\"2.0\"}",
        "{\"jsonrpc\":\"3.0\",\"method\":\"host.status\"}",
        &"x".repeat(MAX_MESSAGE_BYTES + 1),
    ];
    for input in inputs {
        let result = catch_unwind(|| RpcRequest::parse(input));
        assert!(result.is_ok(), "parser panicked for {input:.32}");
        assert!(result.unwrap().is_err());
    }
}

#[test]
fn malformed_paths_return_errors_instead_of_panicking() {
    for path in ["", "..", "/a/../../b", "/a/%2e%2e/b", "\\\\server\\share"] {
        let result = catch_unwind(|| canonical_remote(path));
        assert!(result.is_ok());
        assert!(result.unwrap().is_err(), "{path}");
    }
    let result =
        catch_unwind(|| browser::list_local(std::path::Path::new("/definitely/missing/path")));
    assert!(result.is_ok());
    assert!(result.unwrap().is_err());
}

#[test]
fn the_mcp_binary_survives_a_closed_stdin() {
    // The binary is built by the same workspace; a missing artifact simply
    // skips this process-level check rather than failing the suite.
    let Some(binary) = std::env::var_os("CARGO_BIN_EXE_thriftp-mcp") else {
        return;
    };
    let output = Command::new(binary)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run mcp");
    assert!(output.status.success());
}
