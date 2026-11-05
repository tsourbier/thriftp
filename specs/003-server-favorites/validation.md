# Quickstart Validation: Server Favorites Manager

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

This document records execution of the automated checks and the scenarios in
[quickstart.md](./quickstart.md).

Phase 9 convergence added automated coverage for create/save UI wiring, active-
connection backfill, unavailable-store warnings, shared-credential updates,
last-reference credential prompts, explicit credential reuse/removal, IPC result
wrappers and events, persistence backup/permissions/migrations, protocol labels,
and cross-platform CI. Phase 10 convergence added external-profile credential
reconciliation, contract-document alignment, complete credential event payloads,
theme-aware dialogs, and atomic connection creation.

## Status

- Automated core, storage, credential-mapping, desktop IPC, and frontend
  validation: **PASS on macOS**.
- Native credential save/retrieve/update/delete on macOS: **not verified in this
  headless session** (see [Native credential store](#native-credential-store)).
- Linux and Windows execution: **not performed in this session**. The CI
  `platform` matrix runs the workspace tests on `macos-latest` and
  `windows-latest`; the native credential smoke test remains opt-in.

## Environment

| Item | Value |
|------|-------|
| Host platform | macOS (Darwin, arm64) |
| Rust | stable toolchain, edition 2024 |
| Node.js | 20+ |
| Frontend test runner | Vitest + jsdom |
| Credential backend | macOS Keychain (headless shell session) |

## Commands executed

| Check | Command | Result |
|-------|---------|--------|
| Formatting | `cargo fmt --all -- --check` | PASS |
| Workspace Clippy | `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| Desktop Clippy | `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings` | PASS |
| Workspace tests | `cargo test --workspace -q` | PASS |
| Desktop IPC tests | `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --tests` | PASS (33 tests) |
| Rustdoc gate | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p thriftp-core -p thriftp-storage-json -p thriftp-credentials-keyring` | PASS |
| Desktop rustdoc gate | `RUSTDOCFLAGS="-D warnings" cargo doc --manifest-path apps/desktop/src-tauri/Cargo.toml --no-deps` | PASS |
| Frontend build | `npm run build` | PASS |
| Frontend tests | `npm test` | PASS (76 tests) |

## Scenario coverage

| # | Scenario | Automated evidence | Status |
|---|----------|--------------------|--------|
| 1 | Create a favorite without saved credentials | `server_favorites_create`, `save-favorite.test.ts` | PASS |
| 2 | Save and retrieve a password | `server_favorites_create`, `server_favorites_credentials`, `credentials.test.ts` | PASS (logic); native retrieval pending |
| 3 | SSH private-key handling | `server_favorites_edit`, `server_favorites_connect`, `edit.test.ts` | PASS |
| 4 | Quick connect and last-used ordering | `server_favorites_connect`, `server_favorites_organize`, `connect.test.ts` | PASS |
| 5 | Edit identity fields | `server_favorites_edit`, `server_favorites_commands`, `edit.test.ts` | PASS |
| 6 | Shared credential update | `server_favorites_credentials`, `credentials.test.ts` | PASS |
| 7 | Delete a favorite with a shared credential | `server_favorites_credentials`, `server_favorites_commands` | PASS |
| 8 | Delete the last credential reference | `server_favorites_credentials`, `credentials.test.ts` | PASS |
| 9 | Pin, unpin, and sort | `server_favorites_organize`, `favorites-list.test.ts` | PASS |
| 10 | Group labels | `server_favorites_organize`, `favorites-list.test.ts` | PASS |
| 11 | Active connection independence | `server_favorites_edit`, `server_favorites_commands` | PASS |
| 12 | Credential store unavailable | `credential_store.rs`, `server_favorites_connect`, `credentials.test.ts` | PASS |
| 13 | Connection test before save | `server_favorites_edit`, `server_favorites_commands`, `edit.test.ts` | PASS |
| 14 | Corrupt or interrupted metadata write | `server_favorites_store` | PASS for parse failure; backup recovery remains covered by adapter behavior |
| 15 | Keyboard and accessibility | `favorites-list.test.ts`, `lib/accessibility/focus.ts` tests | PASS |
| 16 | Large favorites list | `server_favorites_performance`, `server_favorites_sort` bench, `performance.test.ts` | PASS |

## Native credential store

The availability probe returned `Available` on this macOS host. A direct
round-trip in the headless shell did **not** persist:

- `set` returned `Ok(())`;
- `get` returned `None`;
- `delete` returned `NotFound`.

This indicates the current shell session has no usable interactive Keychain
context, so the platform-gated native smoke test could not provide evidence
here. No plaintext fallback was used; the adapter failed closed.

Run the native smoke test on an interactive workstation:

```bash
# macOS: unlock Keychain and run from a signed/interactive terminal
THRIFTP_NATIVE_KEYRING=1 cargo test -p thriftp-credentials-keyring --test native_smoke

# Linux: with a Secret Service provider running (GNOME Keyring or KWallet)
THRIFTP_NATIVE_KEYRING=1 cargo test -p thriftp-credentials-keyring --test native_smoke

# Windows: with Credential Manager available for the current user
$env:THRIFTP_NATIVE_KEYRING = "1"
cargo test -p thriftp-credentials-keyring --test native_smoke
```

## Cross-platform remaining evidence (T083)

The following evidence is still required before T083 can be closed:

1. macOS: complete one real save/retrieve/update/delete in an interactive
   Keychain session.
2. Linux: run the automated suite and one native save/retrieve/update/delete
   with a Secret Service provider running.
3. Windows: run the automated suite and one native
   save/retrieve/update/delete with Credential Manager available.

No credentials used during validation were production secrets. Test values are
never written to configuration, logs, errors, diagnostics, or telemetry.
