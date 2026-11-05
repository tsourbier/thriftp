# Quickstart Validation: Server Favorites Manager

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

This guide defines runnable validation scenarios for the feature. It is a validation guide, not an implementation task list. Implementation details belong in `tasks.md` and the implementation phase.

## Prerequisites

- Rust stable toolchain and Cargo.
- Node.js 20+ and pnpm.
- Tauri 2 desktop prerequisites for the host platform.
- A usable native credential store:
  - macOS: Keychain unlocked.
  - Windows: Credential Manager available for the current user.
  - Linux: Secret Service provider running (for example, GNOME Keyring or KWallet).
- No container runtime is required for this feature's tests. The shared `001` containerized protocol integration suite is run separately when needed.
- Test credentials only; never use production secrets.

## Setup

```bash
cargo build --workspace
pnpm install
pnpm tauri dev
```

Run the automated checks:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo audit
pnpm audit
pnpm test
```

### Test Classes

| Class | Requires a container runtime | How to run |
|-------|------------------------------|------------|
| Core unit/integration | No | `cargo test --workspace` |
| Tauri IPC and component | No | `cargo test` / `pnpm test` |
| Platform-specific credential store | No | Platform-specific smoke test targets |
| Shared protocol integration from `001` | Yes, when run | `scripts/test-integration.sh` (separate stage) |

A missing container runtime must not block this feature's unit, integration, component, or platform-specific tests. Docker is never required to build, package, or run Thriftp.

Build validation:

```bash
pnpm tauri build
```

For an official release, generate the SBOM with the project's release tooling and verify that dependencies and CI actions are pinned and lockfiles are committed.

## Validation Scenarios

### 1. Create a favorite without saved credentials

1. Open the favorites view.
2. Create a favorite with a name, protocol, host, port, username, and no saved credential.
3. Confirm it appears in the list with the protocol and connection details.
4. Select connect.

**Expected**: The application requests the missing credential, connects after entry, and does not persist a secret. The saved favorite remains unchanged unless the user chooses to save the credential.

### 2. Save and retrieve a password

1. Create a favorite with password authentication and choose to save the credential.
2. Connect.
3. Restart the application and connect again.

**Expected**: The second connection retrieves the password from the native credential store without prompting. The favorites JSON document contains only a credential reference.

### 3. SSH private-key handling

1. Create an SFTP favorite using private-key authentication.
2. Save the credential or passphrase as required.
3. Inspect the favorites JSON document.
4. Attempt a connection with an unknown or changed SSH host key, and attempt an FTPS connection with an invalid certificate.
5. Move or delete the referenced private-key file, then reconnect.

**Expected**: The document contains only the protected key reference and credential reference. It contains no private key material, passphrase, or password. The host-key and certificate attempts fail without a silent bypass. The missing or moved key produces a clear error and requests a new key reference without reusing the old one.

### 4. Quick connect and last-used ordering

1. Save a favorite with working credentials.
2. Connect with one action from the list.
3. Sort by last used.

**Expected**: The favorite connects without re-entering settings and moves to the most recent position. A failed connection does not update last used.

### 5. Edit identity fields

1. Save a favorite with a password.
2. Change the host, port, username, or authentication method.
3. Attempt to connect without entering a new credential.
4. Cancel and edit again, providing a new credential.

**Expected**: The old credential is not reused. The application requires a compatible credential or an explicit decision to clear the saved credential.

### 6. Shared credential update

1. Create two favorites that explicitly reuse the same credential for matching identity fields.
2. Update the credential from one favorite.
3. Review the confirmation.

**Expected**: The confirmation lists both affected favorites. After confirmation, both use the new secret. The application does not silently split the credential.

### 7. Delete a favorite with a shared credential

1. Create two favorites sharing a credential.
2. Delete one favorite.
3. Connect with the other.

**Expected**: The deleted favorite and its saved connection are removed. The shared credential remains and the other favorite still connects.

### 8. Delete the last credential reference

1. Delete the only favorite referencing a saved credential.
2. Review the credential-deletion prompt.
3. Test both "keep" and "delete" paths in separate runs.

**Expected**: The application asks explicitly. "Keep" retains the credential for later reuse without showing an orphan in the favorites list. "Delete" removes the secret only after confirmation. Neither path deletes silently.

### 9. Pin, unpin, and sort

1. Create several favorites.
2. Pin two of them.
3. Change the sort key.
4. Restart the application.

**Expected**: Pinned entries appear before unpinned entries, the selected sort applies within each section, and pinned state persists across restarts.

### 10. Group labels

1. Assign several favorites to a group label.
2. Rename the label.
3. Remove the label from the last favorite in the group.

**Expected**: Renaming updates all assigned favorites. Removing the label leaves the favorites intact and ungrouped. No empty group remains.

### 11. Active connection independence

1. Connect to a favorite.
2. Edit the favorite's display name and connection settings.
3. Delete a different favorite while the first connection remains active.

**Expected**: The active connection is not interrupted. New connections use updated settings.

### 12. Credential store unavailable

1. Simulate an unavailable, locked, or denied credential store using the test adapter or by stopping the Linux Secret Service provider.
2. Attempt to save and retrieve a credential.

**Expected**: The application explains the failure and does not create a plaintext fallback. Saving may persist non-secret settings; connecting may use session-only credentials only after a clear warning.

### 13. Connection test before save

1. Open the favorite editor with valid settings.
2. Enter a credential only for the test.
3. Run the test and cancel the editor.
4. Inspect the favorites document and the credential store.

**Expected**: The test reports success or failure. Neither the profile nor the test-only credential is persisted. Last used is unchanged.

### 14. Corrupt or interrupted metadata write

1. Simulate an interrupted write or corrupt the favorites JSON document in a test environment.
2. Start the application.

**Expected**: The application recovers from the backup or a safe empty state, keeps unrelated valid favorites where possible, and reports the recovery without exposing secrets.

### 15. Keyboard and accessibility

1. Using only the keyboard, create a favorite, move focus through the list, pin an entry, open the editor, save, and delete a favorite.
2. Check focus visibility, labels, and dialog focus return.

**Expected**: The complete workflow is operable without a pointer. Focus is visible and returns to the list after dialogs.

### 16. Large favorites list

1. Seed at least 500 favorites.
2. Open the favorites view and sort by each supported key.

**Expected**: The view renders and sorts in under 100 ms on a typical professional workstation and remains interactive.

## Security Verification

Use a unique sentinel value as a test password or passphrase, then verify:

- The sentinel does not appear in the favorites JSON document.
- The sentinel does not appear in application logs, error messages, diagnostics, crash reports, or telemetry.
- The sentinel does appear only in the native credential store entry for the expected credential reference.
- Deleting the credential removes the native entry.
- A missing or corrupt credential reference produces a repairable error without leaking the secret.

## Cross-Platform Checks

Run the automated suite on macOS, Linux, and Windows. On each platform, perform at least one real credential save, retrieve, update, and delete against the native store. Gate real-store tests on the availability of the platform credential service so headless CI remains deterministic.

## Expected Outcomes

- All functional scenarios pass.
- No plaintext secret appears outside the native credential store.
- Favorites, group labels, pinned state, sort preferences, and last-used timestamps persist.
- Active connections survive favorite edits and deletion.
- Shared credentials behave consistently and deletion is explicit.
- The core test suite, frontend tests, Clippy, formatting, and platform builds pass.
- No container runtime is required for this feature's tests; the shared protocol integration suite remains a separate stage.
