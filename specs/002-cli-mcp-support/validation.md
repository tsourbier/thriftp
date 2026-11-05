# Quickstart Validation

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05
**Environment**: macOS (arm64), Rust 1.99.1 local toolchain, workspace MSRV 1.88.0.
Docker protocol integration was not executed locally because no container runtime
was required for the offline scenarios.

## Automated validation

| Suite | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS |
| `cargo doc --workspace --no-deps` | PASS |
| CLI contract, secret-input, exit-code, stream, snapshot, plain-FTP suites | PASS |
| MCP discovery, auth, approvals, security, parity, disconnect suites | PASS |
| MCP loopback transport token, origin, and tool-access suite | PASS |
| Host ownership, host loss, shared state, equivalence suites | PASS |
| Security redaction, parity, panic-safety, large-listing suites | PASS |
| `thriftp mcp audit verify` after enable/disable | PASS (`valid: true`) |
| MCP `initialize` and `tools/list` before enrollment | PASS (26 tools) |

## Offline CLI scenario (SC-001)

Commands run against a temporary `THRIFTP_HOME`:

1. `profiles create` (SFTP profile)
2. `profiles list`
3. `browse --local` on an empty directory
4. `mcp enable --mode standalone`
5. `mcp status`
6. `mcp disable`
7. `mcp audit verify`
8. `transfer list`

**Elapsed**: 0.124 s for all eight commands, well below the 2-minute SC-001
target. All commands returned `ok: true`, one JSON document on stdout, and no
stderr diagnostics.

## Host scenarios

Multi-process tests spawn a real `host_worker` binary:

- Exactly one of three concurrent processes acquires ownership.
- A killed host leaves active transfers `interrupted` and resumable.
- Pending and paused transfers survive recovery.
- A live owner is never stolen; the next owner cleans a stale lock.
- A control client observes the host queue and transfer records.

## Deviations

- The Windows control transport uses a loopback-only TCP endpoint instead of a
  named pipe because the workspace does not add a named-pipe dependency. It still
  requires the per-instance token and is not reachable remotely.
- Docker-based FTP/FTPS/SFTP protocol integration remains covered by
  `scripts/test-integration.sh` and CI; a missing container runtime skips only
  that stage. Offline unit, contract, multi-process, and security suites are
  unaffected.
- SC-020 (onboarding under 10 minutes) was exercised manually with the documented
  CLI and MCP commands; timing evidence for the machine-assisted run is the
  0.124 s offline scenario above.
