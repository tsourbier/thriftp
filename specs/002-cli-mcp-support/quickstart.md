# Quickstart Validation: CLI and MCP Automation Interfaces

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

This guide defines runnable scenarios that prove the feature end-to-end. It intentionally avoids implementation code. Contracts and data shapes are defined in [contracts/cli.md](./contracts/cli.md), [contracts/mcp-tools.md](./contracts/mcp-tools.md), [contracts/automation-authorization.md](./contracts/automation-authorization.md), and [contracts/transfer-host.md](./contracts/transfer-host.md); entities are defined in [data-model.md](./data-model.md).

## Prerequisites

- Rust 1.88 or newer and a working Cargo workspace build.
- A test FTP/FTPS/SFTP server. Use the shared `001` Docker Compose harness through `scripts/test-integration.sh` on macOS with Docker Desktop and in Linux CI; Windows developers with Docker Desktop may use it too. Docker-free contract tests use deterministic fakes.
- A disposable OS user or a test credential-store entry for the native keychain.
- Two terminal sessions for host coordination scenarios.
- A test MCP client that supports `2026-07-28` and, for compatibility checks, `2025-11-25`.
- A writable application-data directory. Use an isolated test directory when running automated checks.

Build and discover the surfaces:

```bash
cargo build -p thriftp-cli -p thriftp-mcp
cargo run -p thriftp-cli -- --help
cargo run -p thriftp-cli -- --version
cargo run -p thriftp-cli -- mcp tools --json
```

**Expected**: Help lists every command and global option. Version output includes the CLI version, automation contract version, and supported MCP revisions. `mcp tools --json` lists the catalog without starting a server.

## Scenario 1: CLI profile, browse, and transfer

1. Create a profile and store its credential from stdin:

```bash
printf '%s\n' "$TEST_PASSWORD" \
  | thriftp profiles create --name test-server --protocol sftp \
      --host 127.0.0.1 --port 2222 --username tester --password-stdin
thriftp profiles test test-server
```

2. Browse and inspect:

```bash
thriftp browse /upload --profile test-server --long
thriftp stat /upload --profile test-server
```

3. Create a remote directory, upload a directory, and download it:

```bash
thriftp mkdir /upload/quickstart --profile test-server
thriftp upload ./fixtures/quickstart /upload/quickstart --profile test-server --recursive
thriftp download /upload/quickstart/quickstart ./downloads --profile test-server --recursive
```

4. Rename and delete with explicit confirmation:

```bash
thriftp rename /upload/quickstart /upload/quickstart-renamed --profile test-server --yes
thriftp delete /upload/quickstart-renamed --profile test-server --recursive --yes
```

**Expected**: Every command exits 0. The download preserves directory structure. `delete` refuses to run without `--yes`. Human output is readable, and warnings appear on stderr.

## Scenario 2: Machine-readable output and exit codes

```bash
thriftp --json browse /upload --profile test-server > result.json 2> progress.log
thriftp --json upload ./fixtures/missing /upload --profile test-server > error.json 2> error.log
```

**Expected**:
- `result.json` contains exactly one document matching the envelope in `contracts/cli.md`, with `ok: true` and `exitCode: 0`.
- `progress.log` may contain human-readable progress and contains no JSON result document.
- The failing upload writes exactly one error document to `error.json`, exits non-zero with the documented code, and writes diagnostics only to `error.log`.
- No secret appears in either file.

Test a destination conflict without a policy:

```bash
thriftp upload ./fixtures/quickstart /upload/quickstart --profile test-server
```

**Expected**: Exit code 6; the existing destination is not overwritten.

Test usage and help:

```bash
thriftp unknown-command
thriftp upload --help
```

**Expected**: The unknown command exits 2 with usage guidance on stderr. Help exits 0 and lists options, defaults, and exit-code notes.

## Scenario 3: Secret handling

```bash
# No stored credential and no stdin flag: fail promptly
thriftp profiles set-credential test-server
# Expected: exit 3, credential_missing, no wait for input

# A forbidden secret environment variable is rejected
THRIFTP_PASSWORD=secret thriftp profiles test test-server
# Expected: exit 10 with guidance; the value never appears in output

# Stdin is the supported path
printf '%s\n' "$TEST_PASSWORD" | thriftp profiles set-credential test-server --password-stdin
```

**Expected**: No secret appears in arguments, environment, process titles, output, logs, errors, or audit records. The stdin value is consumed once and is not echoed.

## Scenario 4: CLI transfer lifecycle and interruption

1. Start a large upload in the foreground and note its transfer ID.
2. Terminate the CLI process before completion (for example, `SIGINT` once).
3. Inspect state:

```bash
thriftp --json transfer list
thriftp --json transfer status <transfer-id>
```

**Expected**: The transfer is `interrupted`, not `completed`; the CLI exits 8; the last confirmed offset is preserved; no partial file is presented as complete.

4. Resume:

```bash
thriftp transfer resume <transfer-id>
thriftp transfer wait <transfer-id>
```

**Expected**: Resume verifies source size, modification time, and destination offset first. A mismatch returns exit 6 and requires restart or cancel. A successful resume reaches `completed` with 100% progress.

## Scenario 5: Shared state and host coordination

1. With no desktop running, start a CLI upload without `--no-wait`. It becomes the transfer host.
2. Open the desktop application and confirm it shows the CLI transfer with the same state.
3. Close the desktop application. The transfer continues because the CLI is host.
4. With the desktop running as host, start a CLI upload. Confirm the CLI delegates to the desktop host and both show the same progress.
5. Close the desktop while the delegated transfer is active. Confirm the CLI reports `interrupted` with exit 8 and leaves the transfer resumable; it never reports completion.
6. With no host running, run `thriftp upload ... --no-wait`.

**Expected**: `--no-wait` fails with `host_unavailable` and guidance; no background process is started. Exactly one host exists in each step.

```bash
# Inspect the host from either surface
thriftp --json queue list
thriftp --json transfer list
```

**Expected**: Both surfaces report identical queue limits, order, progress, and states.

## Scenario 6: MCP disabled by default

1. On a fresh test data directory, start the standalone MCP server and attempt `thriftp.automation.status` and an operational tool call.
2. Attempt to configure a wildcard or non-loopback bind through the embedded-mode test configuration surface.

**Expected**: Status reports `mcpEnabled: false`. Operational calls return `mcp_disabled`. No TCP port is opened in standalone mode. The embedded mode refuses wildcard and non-loopback binds. MCP cannot be enabled implicitly.

## Scenario 7: MCP enrollment, authorization, and tools

1. Enable MCP and start the standalone server:

```bash
thriftp mcp enable --mode standalone
thriftp-mcp
```

2. From the test MCP client, call `server/discover`, `tools/list`, and `thriftp.automation.enroll` with a generated public key and display name.
3. In another terminal, review and approve the pending request:

```bash
thriftp mcp clients pending
thriftp mcp clients approve <request-id> \
  --profile test-server --dir test-server:/upload --local-dir ./fixtures \
  --operations connections.list,browse.list,files.stat,transfers.upload,transfers.download,transfers.status \
  --yes
```

4. In the client, complete `challenge` and `authenticate`, then call `thriftp.connections.list`, `thriftp.browse.list` for `/upload`, upload a file, and poll `thriftp.transfers.status`.

**Expected**:
- Discovery reports supported revisions and the contract version.
- Before approval, operational tools return `client_not_enrolled` or `grant_missing`.
- After approval, granted operations succeed and return structured results.
- A request for an ungranted profile returns `scope_denied`.
- A request for `/etc` or a sibling prefix such as `/upload-evil` returns `scope_denied`.
- An upload whose local source is outside `./fixtures`, or a download whose local destination is outside it, returns `scope_denied`.
- Traversal (`/upload/../etc`), encoded traversal (`%2e%2e%2f`), backslash separators, and a symlink that escapes `/upload` are denied and audited.
- An unknown tool returns a clear `invalid_argument` or method error without affecting other sessions.

## Scenario 8: Destructive MCP operations

1. Call `thriftp.files.delete` for a file inside the grant without a confirmation.
2. Approve the pending confirmation in the desktop control surface or CLI, then retry the same call.
3. Create a scoped pre-approval policy for a second file and call delete again.

**Expected**:
- Step 1 returns `confirmation_required`; nothing is deleted.
- The confirmation is single-use, bound to the session, operation, and target, and expires after the configured TTL.
- Step 2 deletes the file and returns `completed`.
- Step 3 proceeds under the policy and produces a `policy_used` audit record.
- In headless mode with no approval channel and no policy, the delete is denied and audited.
- A confirmation for one path cannot authorize another path.

## Scenario 9: Revocation and expiry

1. Authenticate an MCP client and start a long transfer or recursive operation.
2. Revoke the grant while it runs:

```bash
thriftp mcp clients revoke <client-fingerprint> --yes
```

3. Attempt another operation and re-authenticate.
4. Repeat with a grant that expires during an operation.

**Expected**:
- Active sessions terminate.
- The in-flight operation stops at the next file boundary or before the next operation and is audited.
- The next call returns `grant_revoked` or `grant_expired`.
- Re-authentication fails until a new explicit approval creates a new grant.
- Queued work under the revoked grant does not execute.

## Scenario 10: Audit, integrity, and fail-closed behavior

1. Perform allowed, denied, and failed MCP operations.
2. Review and verify the audit trail:

```bash
thriftp mcp audit list --since 2026-11-01T00:00:00Z
thriftp mcp audit verify
```

3. Inject an audit-store failure in the test harness (unwritable directory, disk-full simulation, or permission denial) and attempt an allowed and a destructive operation.
4. Tamper with a test audit record and verify again.

**Expected**:
- Every operation, including denials, has a record with timestamp, client identity, operation, canonical target, result, and redacted error.
- The hash chain verifies cleanly.
- With an unwritable audit store, both operations are blocked with `audit_unavailable`; no destructive action executes.
- Tampering is reported and not repaired.
- No secret, token, private key, or credential appears anywhere in the records.

## Scenario 11: Retention and storage-limit notice

1. Configure a short retention period in an isolated test store.
2. Advance the test clock past the retention boundary.
3. Configure a small storage limit with `thriftp mcp audit limit` and generate enough records to reach it.
4. Make the notice write fail, then make segment deletion fail.

**Expected**: A `retention_notice` is written at least the configured notice period before deletion. No segment is deleted before retention elapses or before the notice is durable. Storage-limit deletion follows the same notice rule and only targets the oldest sealed segments. If no segment can be deleted after notice, new operations are blocked with `audit_unavailable`. Deletion is audited before removal. The CLI and desktop surface the notice.

## Scenario 12: MCP mode parity, concurrency, and compatibility

1. Run the same enrollment, authorization, browse, transfer, denial, and audit scenarios against standalone stdio and embedded loopback modes.
2. Run standalone and embedded servers at the same time, enroll one client through each, and revoke a grant in one while the other has an active session.
3. Connect a client using `2025-11-25` and another using `2026-07-28`.
4. Connect a client using an unsupported revision.

**Expected**:
- Both modes expose the same tools and produce identical authorization, audit, and security outcomes.
- Concurrent servers share grants and audit consistently; a revocation in one is observed by the other no later than the next request or file boundary, and the audit chain remains linear.
- Both supported revisions complete discovery, authentication, and a minimal transfer.
- The unsupported revision receives a structured error listing the supported versions.
- The embedded endpoint is reachable only on `127.0.0.1` and rejects a non-loopback `Host` or `Origin`.
- Stdout in standalone mode contains only protocol messages; logs go to stderr.

## Scenario 13: Documentation walkthrough

From a clean environment, follow `docs/cli.md` and `docs/mcp.md` without reading source code.

**Expected**:
- A new integrator completes a minimal CLI transfer in under 2 minutes.
- A new integrator completes a minimal MCP enrollment and transfer in under 10 minutes.
- Commands, options, exit codes, output streams, tools, schemas, enrollment, grants, audit, and the transfer process model are all documented.

## Automated Validation

Test classes are separated by runtime requirements:

| Class | Requires a container runtime | How to run |
|-------|------------------------------|------------|
| Unit, CLI contract, MCP contract, multi-process host, fault injection | No | `cargo test --workspace` and the contract/host targets below |
| Protocol integration | Yes | `scripts/test-integration.sh` (shared with `001`) |
| Platform-specific | No | Platform test targets on each supported OS |
| Build | No | `pnpm tauri build` / workspace release build |

A missing container runtime skips only the protocol-integration stage. It must not hide unit, contract, multi-process, host, or platform results. Docker is never a runtime dependency of the CLI or MCP server.

Run the Docker-free suite in CI:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p thriftp-cli --test contract
cargo test -p thriftp-mcp --test contract
cargo test -p thriftp-core --test host
cargo test -p thriftp-core --test automation
```

Run the containerized protocol integration suite separately:

```bash
scripts/test-integration.sh
```

**Expected**: All Docker-free tests pass on macOS, Linux, and Windows. The same containerized protocol integration suite passes on macOS through Docker Desktop and in Linux CI. Contract tests validate CLI JSON against the documented schema and MCP tool schemas against the documented catalog. Without a container runtime, only the protocol-integration stage is skipped with a clear message.
