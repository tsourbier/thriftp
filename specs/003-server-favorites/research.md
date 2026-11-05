# Phase 0 Research: Server Favorites Manager

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

All technical unknowns from the plan's Technical Context are resolved below.

## Decision 1: Credential storage backend

**Decision**: Use the `keyring` crate behind a `CredentialStore` trait implemented by `thriftp-credentials-keyring`. Use macOS Keychain, Windows Credential Manager, and Linux Secret Service through that adapter.

**Rationale**: The constitution and feature specification require native OS credential storage and forbid a custom vault or plaintext fallback. `keyring` provides a single Rust interface over the required native backends and keeps platform-specific code in one adapter. If a backend is unavailable or locked, the adapter returns a typed error and the feature fails closed.

**Alternatives considered**:
- Platform-specific FFI for each OS: maximum control, but three implementations and more unsafe/platform code to audit.
- Custom encrypted file with a Thriftp master password: rejected because the specification forbids a custom vault unless separately justified and approved.
- Browser/localStorage storage: rejected because it is not a native credential store and exposes secrets to the UI layer.

## Decision 2: Non-secret metadata persistence

**Decision**: Store favorites, group labels, pinned state, preferences, last-used timestamps, and credential-reference metadata in a versioned JSON document in the OS application-data directory. Use atomic replace (temporary file, flush, rename), restrictive file permissions, a schema version, and explicit migrations.

**Rationale**: The dataset is small (at least 500 entries is still modest), the data is non-secret, and JSON keeps the store inspectable and dependency-light. Credentials remain in the OS store, so the JSON file never contains secrets. Atomic replacement prevents partially written files after a crash. A repository trait keeps the adapter replaceable if the data model outgrows JSON.

**Alternatives considered**:
- SQLite: robust transactions and queries, but a heavier dependency for a small local list; can be adopted later behind the repository trait.
- Tauri store plugin: convenient, but less control over atomicity, migrations, file permissions, and reuse by non-Tauri CLI/MCP adapters.
- Direct in-place JSON writes: rejected because a crash can corrupt the file.

## Decision 3: Core and adapter boundary

**Decision**: Put domain models, validation, sorting, credential-reference rules, and application services in `thriftp-core`. Implement storage and keyring access in separate adapter crates. Tauri commands only translate IPC payloads and call core services.

**Rationale**: The constitution requires the core to build and test without Tauri and to remain reusable by future CLI/MCP interfaces. Thin Tauri commands keep platform/framework concerns out of business logic and make the core testable with fakes.

**Alternatives considered**:
- Business logic inside Tauri commands: violates the core-independence principle and makes CLI/MCP reuse harder.
- A single crate containing core, Tauri, and platform code: simpler initially, but violates module boundaries and complicates testing.

## Decision 4: Credential reference and sharing model

**Decision**: Represent a saved secret by an opaque UUID `CredentialRef` with identity metadata (protocol, host, port, username, authentication method). Store the secret in the OS credential store under a fixed service name and the reference UUID as the account. Each favorite with saved credentials gets its own reference by default; users may explicitly reuse a reference when identity fields match. Updating a shared reference updates all favorites that use it and lists them before confirmation. Deleting a favorite removes its reference; the secret is deleted only when no references remain and the user confirms.

**Rationale**: This satisfies the specification's shared-credential safety, identity-bound reuse, and explicit deletion behavior. An opaque reference avoids encoding secrets or relying on reversible identifiers, and the fixed service/account scheme gives a deterministic lookup without storing the secret in JSON.

**Alternatives considered**:
- Per-favorite credentials only: simpler, but cannot satisfy the shared-reference deletion requirement.
- Deterministic reference derived from identity fields: convenient, but changing an identity field would either break the reference or silently point at the old secret.
- Username or host as the keyring account: collision-prone and leaks identity into the store key.

## Decision 5: Credential-store availability and failure behavior

**Decision**: Probe availability before save/retrieval. If the backend is unavailable, locked, or denied, return a typed error, explain it in the UI, and never write a plaintext fallback. If unavailable while saving, persist non-secret favorite settings without credentials and tell the user. If unavailable while connecting, allow session-only credentials only after a clear warning, keep them in zeroized memory, and discard them at session end.

**Rationale**: The specification requires fail-safe behavior and forbids silent downgrade. This preserves usability for a single session without creating a secret at rest.

**Alternatives considered**:
- Fail the entire favorite save: safe but unnecessarily blocks non-secret configuration.
- Persist an encrypted local cache: forbidden by the no-custom-vault constraint.
- Silently use in-memory credentials without disclosure: rejected because the user must understand persistence behavior.

## Decision 6: Error taxonomy and redaction

**Decision**: Use typed errors with stable codes for validation, not-found, conflict, credential-store-unavailable, credential-missing, credential-rejected, connection-failed, authorization/denied, and internal failures. Map them to user-facing messages at the Tauri boundary. Never include secret values in error payloads, logs, or diagnostics.

**Rationale**: Stable error codes make acceptance tests objective and let the UI choose appropriate recovery actions. Redaction is required by the constitution and specification.

**Alternatives considered**:
- Free-form strings: easy initially, but hard to test and easy to leak secrets.
- Returning backend error text directly: may contain environment details or secrets; rejected.

## Decision 7: Favorites ordering and pinning

**Decision**: Persist a `pinned` flag and sort preferences. The comparator places pinned entries first, then applies the selected sort: display name ascending (case-insensitive) by default, or protocol, host, or last used (most recent first). Stable tie-breakers are protocol, host, and username. Pinning changes presentation only.

**Rationale**: The specification requires predictable sorting and optional pinning. A deterministic comparator makes tests stable.

**Alternatives considered**:
- Store a manual global order: more complex and can conflict with sorting.
- Sort only by name: fails the explicit sort requirements.
- Pin by moving to a separate section: adds UI complexity; pinned-first within one list is simpler.

## Decision 8: Group labels

**Decision**: Reuse the existing optional `groupLabel` on the Connection Profile. No separate group entity is created. Group labels exist only while at least one favorite uses them, are single-level, and removing the last assignment makes the affected favorites ungrouped.

**Rationale**: The desktop specification already defines an optional group label for organizing connections. Reusing it avoids a duplicate grouping model and satisfies the "optional groups without unnecessary complexity" requirement.

**Alternatives considered**:
- Separate favorite-group records: supports empty groups, but duplicates the profile group label and adds lifecycle rules.
- Nested folders: explicitly out of scope.

## Decision 9: Testing strategy

**Decision**: Test core services with in-memory repository and fake credential-store implementations. Test adapters against temporary directories and, where the OS backend is available, against the native store. Test Tauri commands at the IPC boundary. Test Svelte components with Vitest and Testing Library. Run formatting, linting, Docker-free unit/integration tests, and builds in a macOS/Linux/Windows CI matrix. Use real-keychain smoke tests only where the runner provides a usable backend. This feature has no protocol-server integration tests; the shared `001` Docker Compose protocol suite is a separate CI stage and is never required by this feature's tests.

**Rationale**: Most behavior can be tested deterministically without real secrets or containers. Native credential-store tests remain necessary to catch platform integration issues, but must not make CI depend on a logged-in desktop session. Keeping the protocol integration stage separate prevents Docker from becoming a prerequisite for favorites tests.

**Alternatives considered**:
- Real keychain for every test: flaky on headless CI and may require user interaction.
- End-to-end tests only: too slow and gives poor failure localization.
- No adapter tests: fails to catch native-store integration regressions.

## Decision 10: Frontend state and accessibility

**Decision**: Use Svelte stores and a typed IPC client; do not add an external state-management library. Implement keyboard navigation for the list, explicit focus return after dialogs, ARIA labels for protocol and credential state, visible focus, and theme-aware styling.

**Rationale**: The favorites view is small and local to the desktop app; Svelte stores are sufficient. Accessibility and keyboard operation are explicit requirements, and avoiding an extra state library follows YAGNI.

**Alternatives considered**:
- External store library: unnecessary dependency for a small feature.
- Ad hoc component state only: harder to keep list, dialogs, and connection state consistent.

## Decision 11: Schema migration and crash safety

**Decision**: Include `schemaVersion` in the JSON document, write through a temporary file in the same directory, flush and atomically rename, and keep a backup of the previous valid document. Implement migrations as ordered functions from the stored version to the current version. On startup, validate the document and recover to the backup or a safe empty state if it is corrupt, without losing other favorites.

**Rationale**: The specification requires persistence and safe recovery from interrupted writes. Atomic replacement and versioning prevent silent data loss.

**Alternatives considered**:
- In-place writes: corruption risk.
- No schema version: future migrations become guesswork.
- Discard corrupt data silently: loses user favorites and violates transparency.

## Decision 12: Secret handling in memory

**Decision**: Wrap secrets in a non-serializable, non-debuggable `Secret` type backed by a zeroizing buffer. Keep secret lifetimes as short as possible, pass them directly to the connection/keyring boundary, and never return them to the frontend or include them in errors.

**Rationale**: Reduces accidental exposure through logs, serialization, and UI state. Required by the constitution's secret-handling rules.

**Alternatives considered**:
- Plain `String`: can leak through debug output and serialization.
- Frontend-held secrets: increases exposure and violates the separation between UI and credential handling.

## Decision 13: Last-used timestamps and connection tests

**Decision**: Update `lastUsedAt` only after a successful connection to the favorite. A connection test does not update `lastUsedAt` and does not persist credentials entered solely for the test. Timestamps are stored in UTC and displayed in local time.

**Rationale**: Makes "last used" sorting predictable and prevents a failed or test attempt from changing the user's list order.

**Alternatives considered**:
- Update on every attempt: failed attempts would distort sorting.
- Update on favorite open: does not reflect actual use.

## Decision 14: Future CLI/MCP reuse

**Decision**: Keep the favorites application service in `thriftp-core` with an interface independent of Tauri. Do not expose server-favorite management through CLI or MCP in the initial release. Document the core service and data model as the reuse point for a future feature.

**Rationale**: The specification and the CLI/MCP specification defer management through those interfaces, while the constitution requires shared core capabilities. Keeping the logic in the core satisfies both without expanding the current scope.

**Alternatives considered**:
- Expose MCP tools now: expands scope and requires a separate authorization design.
- Put favorites logic in the desktop app only: blocks future reuse and violates the shared-core direction.

## Decision 15: Observability and redaction

**Decision**: Use `tracing` spans and structured fields for favorites and credential operations, including operation name, favorite ID, credential-reference ID, outcome, and duration. Never include secret values, key material, or raw backend error text. Logs are local and never transmitted.

**Rationale**: The constitution requires structured tracing for asynchronous and networking layers with strict secret redaction. Contextual IDs make failures diagnosable without exposing user secrets.

**Alternatives considered**:
- Unstructured console logging: harder to filter and easier to leak.
- Logging backend error text: may contain environment or secret details; rejected.
- No observability: fails the constitution's tracing requirement.

## Decision 16: CI and supply-chain gates

**Decision**: CI runs `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo audit`, frontend lint and Vitest suites, and builds on macOS, Linux, and Windows. Test classes are reported separately: Docker-free unit/integration/component tests, platform-specific credential-store tests, and the shared `001` containerized protocol integration stage when applicable. Dependencies and CI actions are pinned, lockfiles are committed and enforced, and an SBOM is generated for official releases.

**Rationale**: The constitution requires cross-platform validation, separate test classes, SBOM generation for releases, pinned dependencies and actions, and committed lockfiles. Keeping the containerized protocol stage separate ensures a missing container runtime cannot block or mask this feature's tests.

**Alternatives considered**:
- Ad hoc local checks only: does not satisfy the constitution's CI gates.
- Unpinned actions or dependencies: supply-chain risk.
- SBOM generated manually at release time only: easy to skip; CI enforcement is safer.

## Resolved Unknowns Summary

| Unknown | Resolution |
|---------|------------|
| Credential backend | Native OS store through `keyring` |
| Metadata storage | Versioned atomic JSON |
| Core/framework boundary | Core crate plus adapters |
| Credential sharing | Opaque UUID references, explicit reuse, update all references |
| Credential-store outage | Fail closed; no plaintext fallback |
| Error model | Typed codes with redacted messages |
| Ordering | Pinned first, then deterministic sort |
| Grouping | Reuse Connection Profile group label |
| Testing | Docker-free unit, adapter, and IPC tests; platform-specific credential-store tests; no container runtime required for this feature |
| Secret lifetime | Zeroized in-memory type; never returned to UI |
| Schema safety | Versioned JSON, atomic writes, backup, migrations |
| Last used | Successful connections only; tests do not update |
| CLI/MCP scope | Core reusable; management deferred |
| Observability | `tracing` with contextual fields and secret redaction |
| CI and supply chain | Pinned deps/actions, lockfiles, audits, SBOM, cross-platform builds, separate shared protocol-integration stage |
