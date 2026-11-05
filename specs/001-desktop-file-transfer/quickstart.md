# Quickstart Validation: Thriftp Desktop File Transfer Client

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

This guide defines runnable validation scenarios. It is a validation guide, not an implementation task list.

## Prerequisites

- Rust stable toolchain and Cargo.
- Node.js 20+ and pnpm.
- Tauri 2 desktop prerequisites for the host platform.
- A usable native credential store: macOS Keychain, Windows Credential Manager, or Linux Secret Service.
- Docker Desktop on macOS (the primary development environment) or a compatible Docker runtime on Linux/CI; optional on Windows with Docker Desktop. A container runtime is required only for containerized protocol integration tests, not for unit tests, platform-specific tests, or running Thriftp.
- Disposable test accounts only; never production credentials.

## Setup

```bash
cargo build --workspace
pnpm install
pnpm tauri dev
```

Start the containerized protocol test environment and run protocol integration tests:

```bash
scripts/test-integration.sh
```

The script starts `docker/protocol/docker-compose.yml`, waits for FTP, FTPS, and SFTP health checks, exports the test endpoints, runs the same Rust integration suite used in CI, and tears the containers down. On macOS this uses Docker Desktop; on Linux CI it uses the runner's Docker runtime. Windows developers with Docker Desktop can use the same script.

Run automated checks:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace          # unit and non-container tests
cargo audit
pnpm audit
pnpm test
```

Containerized protocol integration tests are run separately by `scripts/test-integration.sh`. Platform-specific tests run on their target operating systems without requiring a container runtime.

Build validation:

```bash
pnpm tauri build
```

Generate the release SBOM with the project's release tooling and verify pinned dependencies, pinned CI actions, and committed lockfiles.

## Test Classes

| Class | Requires a container runtime | How to run |
|-------|------------------------------|------------|
| Unit | No | `cargo test --workspace` |
| Containerized protocol integration | Yes | `scripts/test-integration.sh` |
| Platform-specific | No | Platform test targets on each supported OS |
| Frontend | No | `pnpm test` |

CI MUST report these classes as separate stages. A missing container runtime skips only the containerized integration stage and must not hide unit, frontend, or platform-specific results. Docker is never required to build, package, or run the Thriftp application.

## Validation Scenarios

### 1. Secure connections

1. Create and connect to FTP, explicit FTPS, implicit FTPS, and SFTP profiles.
2. Test SFTP password and private-key authentication.
3. Restart the application and reconnect using stored credentials.

**Expected**: Each supported protocol connects; saved credentials are retrieved from the native store; invalid TLS certificates and unknown or changed SSH host keys produce distinct errors with explicit recovery actions; plain FTP is marked unencrypted.

### 2. Dual-pane browsing

1. Open a local panel and a remote panel.
2. Navigate independently, sort by name/size/date/type, toggle hidden files, refresh, and use the location bar.
3. Resize the window to a small size.

**Expected**: Both panels remain usable; metadata appears where supported; the active panel and independent navigation state are clear.

### 3. File and directory transfers

1. Upload and download a file, a multi-file selection, and a nested directory.
2. Drag files between panels.
3. Verify progress, speed, ETA, completion notification, and final state.

**Expected**: Transfers stream without loading whole files into memory; directory structure and symlink policy are preserved; successful, failed, cancelled, paused, and interrupted states are distinct.

### 4. Pause, resume, queue pause, bandwidth, and priority

1. Pause and resume one active transfer.
2. Pause and resume the entire queue.
3. Set a global bandwidth limit.
4. Reorder queued transfers and change priority.

**Expected**: Paused transfers consume no bandwidth and preserve progress; queue pause prevents new starts; aggregate throughput stays within the limit; the next free slot goes to the highest-priority eligible transfer.

### 5. Interrupted transfers and restart

1. Interrupt a transfer by terminating the application.
2. Restart and inspect the queue and history.
3. Resume the interrupted transfer.
4. Change the source file and attempt resume again.

**Expected**: Active transfers become interrupted with last confirmed progress; pending transfers remain pending and do not auto-start; history persists; resume verifies source size/modification time and destination offset; changed source requires restart or cancel.

### 6. Conflict handling

1. Transfer a file to a destination where it already exists.
2. Test overwrite, skip, rename, and apply-to-all remaining.
3. Repeat a recursive transfer with collisions.

**Expected**: No silent overwrite; the chosen policy applies to the current operation; unrelated browsing and queue activity continue.

### 7. Symbolic links

1. Transfer a directory containing links inside the tree, outside the tree, and a loop.

**Expected**: Links are never followed; they are preserved where supported, otherwise skipped with a visible report; no escape from the selected tree and no infinite recursion.

### 8. Large-directory performance

1. Open a local directory with 10,000 entries.
2. Open a remote directory with 10,000 entries on a controlled server at up to 50 ms round-trip latency.
3. Interact with the UI while each listing loads.

**Expected**: Local first entries appear within 1 second and complete within 5 seconds; remote first entries appear within 2 seconds of the first server response and complete within 30 seconds; the UI remains responsive and indicates that loading continues.

### 9. File operations and deletion

1. Create, rename, and delete local files and directories.
2. Create, rename, and delete remote files and directories.
3. Delete a non-empty local directory and a remote item.

**Expected**: Local deletes go to the OS trash when available; remote deletes require confirmation and warn that they are permanent; unsupported operations explain the limitation.

### 10. Permissions and inspector

1. Inspect a local and remote file.
2. Modify Unix permissions on a supporting server.
3. Attempt permission changes on an unsupported server.

**Expected**: Available metadata appears; unsupported fields are omitted or marked unavailable; permission changes reflect in the listing; unsupported operations degrade gracefully.

### 11. Previews

1. Preview a local text and image file.
2. Preview a remote file with native preview available.
3. Preview a large remote file that would require a full download.

**Expected**: Native preview opens where available; the internal fallback works otherwise; a large remote preview requires explicit consent or shows metadata instead of downloading silently; preview never blocks browsing or transfers.

### 12. Tabs, server favorites, and directory favorites

1. Open several tabs, navigate each, restart, and confirm restoration.
2. Create, edit, connect, pin, and delete server favorites. See [003 quickstart](../003-server-favorites/quickstart.md) for detail.
3. Save and reopen remote directories as directory favorites.

**Expected**: Tabs restore unless disabled; server favorites persist and connect quickly; directory favorites remain associated with their profile; editing or deleting a favorite does not interrupt an active connection.

### 13. Security and credential-store failure

1. Use unique sentinel passwords and key passphrases.
2. Inspect the metadata files, logs, errors, diagnostics, and crash output.
3. Simulate a locked or unavailable credential store.
4. Attempt a TLS failure and an SSH host-key change.

**Expected**: The sentinel appears only in the native credential store; no plaintext fallback exists; credential-store failure is explained; certificate and host-key failures are visible and never silently bypassed.

### 14. Keyboard and accessibility

1. Complete connect, browse, transfer, queue, edit, and delete flows using only the keyboard.
2. Verify visible focus, labels, dialog focus return, and reduced-motion behavior.
3. Check the list with a screen reader.

**Expected**: All primary workflows are keyboard-operable and accessible at the WCAG 2.2 AA target where applicable.

### 15. Cross-platform, containerized integration, and benchmarks

1. Run `cargo test --workspace` on macOS, Linux, and Windows without requiring Docker.
2. Run `scripts/test-integration.sh` on macOS through Docker Desktop and in Linux CI; confirm the same protocol tests and fixtures run in both.
3. Run platform-specific test targets on their target operating systems.
4. Run `criterion` benchmarks for protocol parsing, directory enumeration, and the transfer chunk loop.
5. Verify the release SBOM and pinned dependencies/actions.

**Expected**: All platform builds and Docker-free tests pass; the same containerized protocol suite passes on macOS Docker Desktop and Linux CI; benchmarks meet the regression threshold; the SBOM is generated; no secret appears in benchmark output or logs.

### 16. Container runtime unavailable

1. Stop Docker Desktop or otherwise make the container runtime unavailable.
2. Run `cargo test --workspace` and the frontend tests.
3. Run `scripts/test-integration.sh`.
4. Build and run the application.

**Expected**: Unit, frontend, and platform-specific tests still run; the integration script exits with a clear message explaining that containerized protocol tests were skipped because no compatible runtime is available; the application builds and runs normally because Docker is not a runtime dependency.

## Security Verification

- Use a unique sentinel secret and confirm it never appears in `favorites.json`, `transfers.json`, logs, errors, diagnostics, crash reports, telemetry, or benchmark output.
- Confirm the secret is present only in the native credential-store entry and is removed when the credential is deleted.
- Confirm certificate and host-key overrides are explicit, visible for the session, and audited.
- Confirm local deletion uses OS trash and remote deletion warns that it is permanent.

## Expected Outcomes

- All scenarios pass on supported platforms.
- No plaintext secrets exist outside the native credential store.
- Queue, history, tabs, favorites, and preferences persist and recover correctly.
- Transfers are resumable only after verification, stream with constant memory, and never block the UI.
- Local and remote performance targets are measured separately.
- Docker-free unit and platform tests pass without a container runtime.
- The same containerized protocol integration suite passes on macOS Docker Desktop and Linux CI.
- The core, protocol adapters, Tauri shell, and Svelte UI pass their respective test suites.
