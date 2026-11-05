---
description: "Task list for the Server Favorites Manager"
---

# Tasks: Server Favorites Manager

**Input**: Design documents from `/specs/003-server-favorites/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Required. The specification's success criteria and the project constitution require automated core, adapter, IPC, frontend, and security tests.

**Organization**: Tasks are grouped by user story so each story can be implemented, tested, and demonstrated independently. Four stories are P1 and share the same favorites foundation; the editing story is P2.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Which user story the task belongs to (`US1`–`US5`)
- Every task includes an exact file path under the Cargo workspace, `apps/desktop`, `docs/`, or the feature directory

## Path Conventions

- Rust workspace: `Cargo.toml`, `crates/<crate>/src/`, `crates/<crate>/tests/`, `crates/<crate>/benches/`
- Desktop shell: `apps/desktop/src-tauri/src/`, `apps/desktop/src/`
- Documentation: `docs/`
- Feature artifacts: `specs/003-server-favorites/`

**Path note**: The existing `crates/thriftp-core/src/favorites.rs`, `apps/desktop/src-tauri/src/commands/favorites.rs`, and `apps/desktop/src/components/favorites/` implement **directory favorites**. This feature adds new `server_favorites` Rust modules and `server-favorites` frontend modules; do not overwrite or repurpose the directory-favorites implementation.

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm dependencies, test layout, CI coverage, documentation, and the new module namespace.

- [X] T001 Add or confirm server-favorites dependencies in `Cargo.toml`, `crates/thriftp-core/Cargo.toml`, `crates/thriftp-credentials-keyring/Cargo.toml`, `crates/thriftp-storage-json/Cargo.toml`, `apps/desktop/src-tauri/Cargo.toml`, and `apps/desktop/package.json`: `serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `keyring`, `zeroize`, `tracing`, Vitest, and Testing Library.
- [X] T002 [P] Create the server-favorites test layout and fixtures in `crates/thriftp-core/tests/server_favorites_create.rs`, `crates/thriftp-core/tests/server_favorites_organize.rs`, `crates/thriftp-core/tests/server_favorites_connect.rs`, `crates/thriftp-core/tests/server_favorites_credentials.rs`, `crates/thriftp-core/tests/server_favorites_edit.rs`, `crates/thriftp-storage-json/tests/server_favorites_store.rs`, `crates/thriftp-credentials-keyring/tests/credential_store.rs`, `apps/desktop/src-tauri/tests/server_favorites_commands.rs`, and `apps/desktop/src/components/server-favorites/server-favorites.test.ts`.
- [X] T003 [P] Update CI in `.github/workflows/ci.yml` to run the server-favorites Rust and Vitest suites on macOS, Linux, and Windows, and gate native credential-store smoke tests on backend availability; keep the shared `001` containerized protocol stage separate and optional.
- [X] T004 [P] Create the server-favorites documentation skeleton in `docs/server-favorites.md` and add a link from `README.md`.
- [X] T005 [P] Create the core module skeleton in `crates/thriftp-core/src/server_favorites/mod.rs` and export it from `crates/thriftp-core/src/lib.rs`, keeping the existing `favorites` module unchanged for directory favorites.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared models, persistence, credential references, sorting, errors, services, and test harnesses required before any user story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T006 Implement the server-favorite models in `crates/thriftp-core/src/model/server_favorite.rs` and export them from `crates/thriftp-core/src/model/mod.rs`: `ServerFavorite` (`id` UUID; `connectionProfileId` UUID unique; `pinned` boolean default false; `lastUsedAt` nullable UTC; `credentialPolicy` `save` or `ask`; `credentialRefId` UUID required when policy is `save` and null when `ask`; `createdAt` and `updatedAt` UTC) and `FavoritePreferences` (`sortKey` `name`, `protocol`, `host`, or `lastUsed`; `pinnedFirst` true in the initial release).
- [X] T007 Extend `crates/thriftp-core/src/model/credential.rs` and `crates/thriftp-core/src/model/profile.rs` so `CredentialRef` has an independent UUID and identity fields (`protocol`, `host`, `port`, `username`, `authMethod`), plus a matching helper; quote the rules “must match every referencing connection profile” and “changing any identity field invalidates the previous reference.” Update `CredentialRef::for_profile` so it does not reuse the profile ID as the credential reference ID.
- [X] T008 [P] Add the server-favorites repository boundary in `crates/thriftp-core/src/server_favorites/repository.rs` and extend `AppState` in `crates/thriftp-core/src/repository.rs` with `server_favorites: Vec<ServerFavorite>` and `favorite_preferences: FavoritePreferences`; preserve existing directory-favorite and connection data.
- [X] T009 [P] Implement versioned JSON persistence for server favorites in `crates/thriftp-storage-json/src/server_favorites.rs` and export it from `crates/thriftp-storage-json/src/lib.rs`: `schemaVersion`, unique IDs, temporary-file write + flush + atomic rename, backup of the previous valid document, `0600` on Unix, ordered migrations, and recovery from backup or safe empty state on corruption. Add adapter tests in `crates/thriftp-storage-json/tests/server_favorites_store.rs` for atomic replace, backup recovery, migration, corrupt JSON, and no-secret persistence.
- [X] T010 [P] Verify and extend the native credential adapter in `crates/thriftp-credentials-keyring/src/lib.rs` for independent `CredentialRef` IDs, `Available`/`Locked`/`Unavailable`/`Denied` availability, and `NotFound` mapping; no plaintext fallback is permitted.
- [X] T011 [P] Implement the favorites comparator in `crates/thriftp-core/src/server_favorites/sorting.rs` and wire it from `crates/thriftp-core/src/sorting.rs`: pinned entries first, then display name ascending case-insensitive by default, or protocol/host/last-used, with protocol, host, and username as stable tie-breakers.
- [X] T012 [P] Extend stable error mapping and redaction in `crates/thriftp-core/src/error.rs` and `crates/thriftp-core/src/observability.rs` for `validation`, `not_found`, `conflict`, `credential_store_unavailable`, `credential_missing`, `credential_rejected`, `connection_failed`, `permission_denied`, and `internal`; never include secret values or raw backend text.
- [X] T013 [P] Implement reusable credential retrieval and session-only handling in `crates/thriftp-core/src/credentials/service.rs` and `crates/thriftp-core/src/connections.rs`: retrieve saved secrets by reference, distinguish missing/rejected/unavailable, zeroize session-only secrets, and update `lastUsedAt` only on success.
- [X] T014 Implement the server-favorites application service skeleton in `crates/thriftp-core/src/server_favorites/service.rs` with repository and credential-service injection, validation hooks, sorting, and observable operations.
- [X] T015 Implement Tauri state and command registration for server favorites in `apps/desktop/src-tauri/src/lib.rs` and `apps/desktop/src-tauri/src/commands/mod.rs`, adding a new `server_favorites` command module without changing the existing directory-favorites module.
- [X] T016 [P] Implement frontend IPC types, client methods, and Svelte stores in `apps/desktop/src/lib/server-favorites/types.ts`, `apps/desktop/src/lib/server-favorites/api.ts`, and `apps/desktop/src/lib/server-favorites/stores.ts`, and extend `apps/desktop/src/lib/ipc/client.ts` as needed.
- [X] T017 [P] Implement core test fakes in `crates/thriftp-core/tests/support/mod.rs` and `crates/thriftp-core/tests/support/fakes.rs`: in-memory server-favorites repository, fake credential store with availability modes, fake connection tester, and secret-sentinel helpers.
- [X] T018 [P] Implement the Tauri command test harness in `apps/desktop/src-tauri/tests/support/mod.rs` with temporary application-data directories, typed payload builders, and error/redaction assertions.
- [X] T019 [P] Add frontend test utilities and keyboard/focus helpers in `apps/desktop/src/lib/server-favorites/test-utils.ts` and reuse `apps/desktop/src/lib/accessibility/focus.ts` for dialog focus return.
- [X] T020 Add `tracing` spans and redaction fields for create, list, connect, credential, edit, and delete operations in `crates/thriftp-core/src/server_favorites/service.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`.

**Checkpoint**: Foundation ready. US1, US2, US3, and US4 can start, subject to their stated dependencies.

---

## Phase 3: User Story 1 — Save a server as a favorite (Priority: P1) 🎯 MVP

**Goal**: Create a favorite from scratch or from an active connection, persist its non-secret settings, and optionally save credentials.

**Independent Test**: Create a favorite from scratch and from an active connection, restart the application, and confirm both appear with the correct protocol, host, port, username, authentication method, and credential policy.

### Tests for User Story 1 (REQUIRED)

- [X] T021 [P] [US1] Add core tests for create-from-scratch, create-from-active-connection, validation, persistence reload, and one-favorite-per-profile in `crates/thriftp-core/tests/server_favorites_create.rs`.
- [X] T022 [P] [US1] Add IPC contract tests for `favorites_create` and `connections_get_profile` in `apps/desktop/src-tauri/tests/server_favorites_create.rs`, including validation errors and no secret leakage.
- [X] T023 [P] [US1] Add frontend tests for the save form, credential policy choice, pre-filled active-connection settings, and empty state in `apps/desktop/src/components/server-favorites/save-favorite.test.ts`.

### Implementation for User Story 1

- [X] T024 [US1] Implement `ServerFavoriteService::create` and validation in `crates/thriftp-core/src/server_favorites/service.rs`: display name 1–200 characters after trimming, group label 1–100, host 1–255, port 1–65535, timeout 1–300 with default 30, FTP/FTPS password-only auth, SFTP password or private-key auth, a non-empty `sshKeyRef` when private-key auth is selected, no key material stored in the profile, and exactly one favorite per profile.
- [X] T025 [US1] Implement the `favorites_create` command in `apps/desktop/src-tauri/src/commands/server_favorites.rs` and register it in `apps/desktop/src-tauri/src/commands/mod.rs`.
- [X] T026 [US1] Implement active-connection prefill via `connections_get_profile` in `apps/desktop/src-tauri/src/commands/connections.rs` and the save-as-favorite flow in `apps/desktop/src/lib/server-favorites/api.ts`.
- [X] T027 [P] [US1] Implement the favorites list store and create action in `apps/desktop/src/lib/server-favorites/stores.ts` and `apps/desktop/src/lib/server-favorites/api.ts`.
- [X] T028 [US1] Implement `apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte` with display name, protocol, host, port, username, authentication method, timeout, FTPS mode, SSH key reference, credential policy, and pinning.
- [X] T029 [US1] Wire startup load and save through `crates/thriftp-storage-json/src/server_favorites.rs` in `apps/desktop/src-tauri/src/lib.rs`, preserving existing directory favorites and preferences.
- [X] T030 [US1] Add tracing and redaction around create/save operations in `crates/thriftp-core/src/server_favorites/service.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`.

**Checkpoint**: US1 works independently and is the MVP increment.

---

## Phase 4: User Story 2 — Browse and organize favorites (Priority: P1)

**Goal**: Display every favorite in a dedicated view, sort predictably, pin entries, group them, rename them, edit them, and delete them safely.

**Independent Test**: Create several favorites, sort by each supported key, pin one, move one into a group, rename it, and delete another while confirming the remaining favorites and credentials stay intact.

### Tests for User Story 2 (REQUIRED)

- [X] T031 [P] [US2] Add core tests for pinned-first ordering, every sort key, stable tie-breakers, group assignment/rename/removal, large-list ordering, and graceful handling of directory favorites or tabs that reference a deleted profile in `crates/thriftp-core/tests/server_favorites_organize.rs`.
- [X] T032 [P] [US2] Add IPC contract tests for `favorites_list`, `favorites_get`, `favorites_update`, `favorites_delete`, `favorites_set_pinned`, `favorites_set_group`, `favorites_rename_group`, and preferences in `apps/desktop/src-tauri/tests/server_favorites_commands.rs`, including deletion of a profile referenced by directory favorites or open tabs without deleting unrelated data.
- [X] T033 [P] [US2] Add frontend tests for list rendering, sort controls, pinned sections, group labels, rename, delete confirmation, the empty state, and a keyboard-only workflow covering create, navigation, pin, edit, delete, and dialog focus return in `apps/desktop/src/components/server-favorites/favorites-list.test.ts`.

### Implementation for User Story 2

- [X] T034 [US2] Implement list/get/sort/group operations in `crates/thriftp-core/src/server_favorites/service.rs` and `crates/thriftp-core/src/server_favorites/sorting.rs`.
- [X] T035 [US2] Implement `favorites_list` and `favorites_get` commands in `apps/desktop/src-tauri/src/commands/server_favorites.rs`.
- [X] T036 [US2] Implement `favorites_update` and `favorites_delete` in `apps/desktop/src-tauri/src/commands/server_favorites.rs`; deletion removes the connection profile and favorite metadata, leaves active connections running, and handles directory favorites or open tabs that reference the deleted profile without deleting unrelated data or interrupting active connections.
- [X] T037 [P] [US2] Implement `favorites_set_pinned`, `favorites_set_group`, and `favorites_rename_group` in `apps/desktop/src-tauri/src/commands/server_favorites.rs`.
- [X] T038 [P] [US2] Implement `favorites_get_preferences` and `favorites_set_preferences` in `apps/desktop/src-tauri/src/commands/server_favorites.rs` and persist them through `crates/thriftp-storage-json/src/server_favorites.rs`.
- [X] T039 [US2] Implement `apps/desktop/src/components/server-favorites/FavoritesList.svelte` and `FavoriteRow.svelte` with display name, protocol indicator, host, username, credential-saved indicator, pin control, and active-connection distinction.
- [X] T040 [P] [US2] Implement group assignment, rename, and ungrouped-state controls in `apps/desktop/src/components/server-favorites/GroupMenu.svelte`.
- [X] T041 [US2] Implement `apps/desktop/src/components/server-favorites/DeleteFavoriteDialog.svelte` with explicit confirmation and a guarantee that active connections are not interrupted.
- [X] T042 [US2] Implement keyboard navigation, visible focus, ARIA labels, theme awareness, and dialog focus return in `apps/desktop/src/components/server-favorites/` and `apps/desktop/src/lib/accessibility/focus.ts`.
- [X] T043 [US2] Integrate the favorites view into `apps/desktop/src/App.svelte` and the existing navigation, clearly separating saved favorites from active connections.
- [X] T044 [US2] Add tracing and redaction for list/edit/delete/group operations in `crates/thriftp-core/src/server_favorites/service.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`.

**Checkpoint**: US1 and US2 work independently.

---

## Phase 5: User Story 3 — Connect quickly to a favorite (Priority: P1)

**Goal**: Connect with one action, use saved credentials when available, prompt only when necessary, and preserve the favorite across connect/disconnect cycles.

**Independent Test**: Connect from a favorite with saved credentials, then with missing credentials, then after a connection failure, and confirm each path gives clear feedback without exposing secrets.

### Tests for User Story 3 (REQUIRED)

- [X] T045 [P] [US3] Add core tests for saved-credential connect, missing/rejected/unavailable credentials, session-only credentials, disconnect/reconnect, last-used success-only updates, active-connection independence, FTPS certificate validation failure, SSH unknown/changed host-key failure, and a missing or moved SSH private-key file that requests a new key reference in `crates/thriftp-core/tests/server_favorites_connect.rs`.
- [X] T046 [P] [US3] Add IPC contract tests for `favorites_connect`, `connections_disconnect`, and `connections_get_profile` in `apps/desktop/src-tauri/tests/server_favorites_commands.rs`.
- [X] T047 [P] [US3] Add frontend tests for one-action connect, credential prompting, failure feedback, and active-state display in `apps/desktop/src/components/server-favorites/connect.test.ts`.

### Implementation for User Story 3

- [X] T048 [US3] Implement `ServerFavoriteService::connect` and credential retrieval in `crates/thriftp-core/src/server_favorites/service.rs`, `crates/thriftp-core/src/credentials/service.rs`, and `crates/thriftp-core/src/connections.rs`.
- [X] T049 [US3] Implement `favorites_connect` and `connections_disconnect` commands in `apps/desktop/src-tauri/src/commands/server_favorites.rs` and `apps/desktop/src-tauri/src/commands/connections.rs`.
- [X] T050 [P] [US3] Implement `apps/desktop/src/components/server-favorites/CredentialPromptDialog.svelte` for missing, rejected, and unavailable credentials, with a clear distinction between saved and session-only secrets.
- [X] T051 [P] [US3] Implement connection state/event handling in `apps/desktop/src/lib/server-favorites/stores.ts` and `apps/desktop/src/lib/ipc/stores.ts`.
- [X] T052 [US3] Implement the one-action connect control and active-connection indicator in `apps/desktop/src/components/server-favorites/FavoriteRow.svelte` and `apps/desktop/src/components/connections/ConnectionStatus.svelte`.
- [X] T053 [US3] Ensure `lastUsedAt` updates only after a successful connection in `crates/thriftp-core/src/server_favorites/service.rs` and persist the change through the repository.
- [X] T054 [US3] Add tracing and redaction to connect/disconnect operations in `crates/thriftp-core/src/server_favorites/service.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`.

**Checkpoint**: US1, US2, and US3 work independently.

---

## Phase 6: User Story 4 — Manage credentials securely (Priority: P1)

**Goal**: Save, update, reuse, remove, and delete credentials while the operating system protects all secret material.

**Independent Test**: Save, update, and remove a password and an SSH key reference, simulate an unavailable credential store, and confirm no plaintext secret appears in configuration, logs, errors, or diagnostics.

### Tests for User Story 4 (REQUIRED)

- [X] T055 [P] [US4] Add credential-adapter tests for available, locked, unavailable, denied, missing, and invalid-reference behavior in `crates/thriftp-credentials-keyring/tests/credential_store.rs`.
- [X] T056 [P] [US4] Add core tests for credential reference creation, identity matching, shared update, removal, last-reference prompting, identity invalidation, unavailable-store save of non-secret settings with a warning and no stored secret, and recovery from a missing or corrupt credential reference in `crates/thriftp-core/tests/server_favorites_credentials.rs`.
- [X] T057 [P] [US4] Add IPC contract tests for `credentials_get_impact`, `credentials_list_reusable`, `credentials_remove`, and `credentials_delete` in `apps/desktop/src-tauri/tests/server_favorites_commands.rs`.
- [X] T058 [P] [US4] Add frontend tests for save/update/remove credential flows, shared-impact dialogs, unavailable-store warnings, corrupt-reference repair, and last-reference deletion in `apps/desktop/src/components/server-favorites/credentials.test.ts`.

### Implementation for User Story 4

- [X] T059 [US4] Implement credential-reference creation, identity matching, and rebinding in `crates/thriftp-core/src/server_favorites/service.rs` and `crates/thriftp-core/src/model/credential.rs`.
- [X] T060 [US4] Implement save/update/remove credential operations and shared-impact calculation in `crates/thriftp-core/src/server_favorites/service.rs` and `crates/thriftp-core/src/credentials/service.rs`.
- [X] T061 [US4] Implement `credentials_get_impact`, `credentials_list_reusable`, `credentials_remove`, and `credentials_delete` in `apps/desktop/src-tauri/src/commands/credentials.rs` and register them in `apps/desktop/src-tauri/src/commands/mod.rs`.
- [X] T062 [P] [US4] Implement `apps/desktop/src/components/server-favorites/CredentialImpactDialog.svelte` and `ReuseCredentialDialog.svelte`, listing affected favorites before confirmation.
- [X] T063 [P] [US4] Implement `apps/desktop/src/components/server-favorites/CredentialUnavailableBanner.svelte` for unavailable, locked, and denied stores, including session-only credential warnings, and a repair/re-entry action for a missing or corrupt credential reference that keeps the favorite visible.
- [X] T064 [US4] Implement last-reference keep-versus-delete prompting in `apps/desktop/src/components/server-favorites/DeleteCredentialDialog.svelte` and the corresponding service path.
- [X] T065 [US4] Add secret sentinel/redaction tests for configuration, logs, errors, diagnostics, and events in `crates/thriftp-core/tests/server_favorites_credentials.rs` and `apps/desktop/src-tauri/src/error.rs`.
- [X] T066 [US4] Add platform-gated native credential-store smoke tests in `crates/thriftp-credentials-keyring/tests/native_smoke.rs` for save, retrieve, update, and delete.

**Checkpoint**: US1–US4 work independently and credential behavior is verified.

---

## Phase 7: User Story 5 — Edit connection settings safely (Priority: P2)

**Goal**: Edit favorite settings, invalidate stale credentials when identity fields change, test pending settings without persistence, and leave active connections untouched.

**Independent Test**: Change a favorite's username and authentication method, verify old credentials are not reused, test the connection before saving, and confirm an existing active connection is unaffected.

### Tests for User Story 5 (REQUIRED)

- [X] T067 [P] [US5] Add core tests for identity-field changes, auth-method changes, protocol changes, test-before-save, credential clearing, active-connection independence, and repair/re-entry when an SSH private-key file is missing or moved in `crates/thriftp-core/tests/server_favorites_edit.rs`.
- [X] T068 [P] [US5] Add IPC contract tests for `favorites_update`, `favorites_test_connection`, and `connections_get_profile` in `apps/desktop/src-tauri/tests/server_favorites_commands.rs`.
- [X] T069 [P] [US5] Add frontend tests for the edit form, credential-invalidation prompt, unsaved-changes guard, and test-connection flow in `apps/desktop/src/components/server-favorites/edit.test.ts`.

### Implementation for User Story 5

- [X] T070 [US5] Implement edit validation and identity invalidation in `crates/thriftp-core/src/server_favorites/service.rs` and `crates/thriftp-core/src/model/credential.rs`.
- [X] T071 [US5] Implement the `favorites_update` command and prefill flow in `apps/desktop/src-tauri/src/commands/server_favorites.rs` and `apps/desktop/src/components/server-favorites/EditFavoriteDialog.svelte`.
- [X] T072 [P] [US5] Implement `favorites_test_connection` in `apps/desktop/src-tauri/src/commands/server_favorites.rs` and `crates/thriftp-core/src/server_favorites/service.rs` without persisting the profile or test-only secret and without updating `lastUsedAt`.
- [X] T073 [P] [US5] Implement unsaved-changes, credential-invalidation, and clear-saved-credential UI in `apps/desktop/src/components/server-favorites/EditFavoriteDialog.svelte` and `apps/desktop/src/lib/server-favorites/stores.ts`.
- [X] T074 [US5] Implement protocol-specific edit rules in `crates/thriftp-core/src/server_favorites/service.rs` and `crates/thriftp-core/src/validation.rs`: FTP/FTPS password-only, SFTP password or private key, explicit/implicit FTPS mode changes, and SSH key reference requirements.
- [X] T075 [US5] Add tracing and redaction to edit/test operations in `crates/thriftp-core/src/server_favorites/service.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`.

**Checkpoint**: All five user stories are independently functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

**Purpose**: Performance, security, public API documentation, supply chain, and final validation.

- [X] T076 [P] Add a performance test for rendering and sorting at least 500 favorites under 100 ms on a typical workstation in `crates/thriftp-core/benches/server_favorites_sort.rs` and `apps/desktop/src/components/server-favorites/performance.test.ts`.
- [X] T077 [P] Conduct and document the required security review for credential handling, shared references, redaction, fail-closed behavior, and certificate/host-key verification parity in `specs/003-server-favorites/security-review.md`.
- [X] T078 [P] Add rustdoc for new public APIs and enforce a `missing_docs` or `cargo doc` gate in `crates/thriftp-core/src/lib.rs`, `crates/thriftp-storage-json/src/lib.rs`, `crates/thriftp-credentials-keyring/src/lib.rs`, and `apps/desktop/src-tauri/src/lib.rs`.
- [X] T079 [P] Complete `docs/server-favorites.md` with creation, organization, quick connect, credential sharing, unavailable-store behavior, accessibility, and recovery guidance.
- [X] T080 [P] Add panic/`unwrap`/`expect` audit and negative tests for malformed data, corrupt stores, and recoverable credential failures in `crates/thriftp-core/tests/panic_safety.rs`.
- [X] T081 [P] Add SBOM generation, dependency audits, pinned dependency/action verification, and lockfile enforcement in `.github/workflows/ci.yml`, `deny.toml`, and `Cargo.lock`.
- [X] T082 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo audit`, and `pnpm test`; fix all failures in `Cargo.toml`, `crates/`, and `apps/desktop/`.
- [ ] T083 Execute the scenarios in `specs/003-server-favorites/quickstart.md` on macOS, Linux, and Windows, including at least one real credential save/retrieve/update/delete per platform, and record results in `specs/003-server-favorites/validation.md`.
- [X] T084 Remove dead code and unused dependencies, confirm YAGNI, and update `Cargo.toml` and `crates/*/Cargo.toml` before final review.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: Depends on Foundational; delivers the MVP.
- **US2 (Phase 4)**: Depends on US1 for saved favorites; can start after the create path is stable.
- **US3 (Phase 5)**: Depends on US1 and the foundational credential service; can run alongside US2/US4 after US1.
- **US4 (Phase 6)**: Depends on US1 and the foundational credential adapter; can run alongside US2/US3.
- **US5 (Phase 7)**: Depends on US1 and US4 for credential invalidation and test-before-save.
- **Polish (Phase 8)**: Depends on all desired user stories.

### User Story Dependencies

| Story | Priority | Depends on | Independent Test |
|-------|----------|------------|------------------|
| US1 | P1 | Foundational | Create from scratch and from an active connection; restart and verify persistence |
| US2 | P1 | US1 | Sort, pin, group, rename, and delete favorites without losing other entries or credentials |
| US3 | P1 | US1, foundational credential service | Connect with saved, missing, and rejected credentials without exposing secrets |
| US4 | P1 | US1, foundational credential adapter | Save, update, reuse, remove, and delete credentials with fail-closed behavior |
| US5 | P2 | US1, US4 | Edit identity fields, test before saving, and leave active connections untouched |

### Within Each User Story

- Write the listed tests first and confirm they fail.
- Models and public types before services.
- Services before Tauri commands.
- Tauri commands before frontend integration.
- Complete a story and its independent test before moving to the next priority.

### Parallel Opportunities

- Setup tasks marked `[P]` use different files.
- Foundational tasks T008–T013, T016–T019 target different modules and can run together.
- Within US1, T025–T029 touch different layers after the service exists.
- Within US2, T037–T041 target different commands and components.
- Within US3, T050–T052 target credential prompting, connection state, and row integration.
- Within US4, T062–T063 and T066 target separate dialogs and platform tests.
- Within US5, T072–T073 target test-before-save and edit-flow UI.
- Polish performance, security review, rustdoc, docs, panic safety, and supply-chain tasks are independent.

---

## Parallel Example: User Story 1

```bash
# Tests first:
Task: "Core create tests in crates/thriftp-core/tests/server_favorites_create.rs"
Task: "IPC create tests in apps/desktop/src-tauri/tests/server_favorites_create.rs"
Task: "Frontend save-form tests in apps/desktop/src/components/server-favorites/save-favorite.test.ts"

# Then implementation:
Task: "Create service in crates/thriftp-core/src/server_favorites/service.rs"
Task: "favorites_create command in apps/desktop/src-tauri/src/commands/server_favorites.rs"
Task: "Save dialog in apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte"
```

## Parallel Example: User Story 4

```bash
Task: "Credential adapter tests in crates/thriftp-credentials-keyring/tests/credential_store.rs"
Task: "Core credential tests in crates/thriftp-core/tests/server_favorites_credentials.rs"
Task: "IPC credential tests in apps/desktop/src-tauri/tests/server_favorites_commands.rs"
Task: "Credential impact dialog in apps/desktop/src/components/server-favorites/CredentialImpactDialog.svelte"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: US1.
4. Stop and validate create-from-scratch, create-from-active-connection, persistence, and no-plaintext behavior.
5. Deliver the MVP.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → save and persist favorites.
3. US2 → browse, sort, pin, group, rename, delete.
4. US3 → quick connect and credential prompting.
5. US4 → secure credential lifecycle and shared references.
6. US5 → safe editing and test-before-save.
7. Polish → performance, security review, docs, and cross-platform validation.

### Parallel Team Strategy

- Complete Setup + Foundational together.
- After US1, split US2 (list/organization), US3 (connect), and US4 (credentials) across developers.
- US5 follows US1 and US4.
- Polish is shared after all selected stories.

---

## Notes

- `[P]` tasks use different files and have no dependency on an incomplete task.
- `[Story]` labels map every story-phase task to `US1`–`US5`.
- Tests are required by the spec and constitution; do not skip credential, redaction, persistence, or accessibility coverage.
- Never write secrets to configuration, logs, errors, diagnostics, crash reports, telemetry, or test assertions.
- Preserve the existing directory-favorites modules; server favorites use the separate `server_favorites` and `server-favorites` namespaces.
- Docker is never required by this feature; the shared `001` protocol integration stage remains separate.

## Phase 9: Convergence

- [X] T085 [CRITICAL] Add a create-favorite entry point and wire `SaveFavoriteDialog` in `apps/desktop/src/App.svelte`, including an empty-state action, submission through `apps/desktop/src/lib/server-favorites/api.ts`, list refresh, and error handling per FR-001, FR-021, and US1/AC1 (`missing`)
- [X] T086 Add a save-active-connection-as-favorite action in `apps/desktop/src/App.svelte` and align `connections_get_profile` in `apps/desktop/src-tauri/src/commands/connections.rs` with the contract so the form is prefilled with the active connection's current settings per FR-004 and US1/AC3 (`missing`)
- [X] T087 Reconcile the separate saved-connection surface with server favorites: backfill exactly one favorite for every existing, newly created, CLI/MCP-created, edited, or deleted connection profile, and make `connections_create`, `connections_update`, and `connections_delete` preserve favorite metadata, directory favorites, tabs, and shared credentials per FR-007, FR-008, and FR-016 (`contradicts`)
- [X] T088 Implement FR-051 in `crates/thriftp-core/src/server_favorites/mod.rs` and the desktop commands: when the credential store is unavailable while saving with `credentialPolicy = "save"`, persist the favorite's non-secret settings, return a structured warning, and never store the secret (`partial`)
- [X] T089 Update shared credentials through the existing `credentialRefId` when identity fields are unchanged, require `credentials_get_impact` confirmation listing every affected favorite before applying the update, and add tests proving no silent split per FR-047 and SC-018 (`contradicts`)
- [X] T090 Reshape `favorites_delete` and `credentials_remove` to the documented IPC payloads, then wire `DeleteCredentialDialog` in `apps/desktop/src/App.svelte` so deleting the last reference explicitly asks keep-versus-delete and calls `credentials_delete` only after confirmation per FR-049 and SC-009 (`missing`)
- [X] T091 Wire explicit credential reuse and removal into the UI using `ReuseCredentialDialog`, `CredentialImpactDialog`, `credentials_list_reusable`, and `credentials_remove`, including a clear-saved-credential action and shared-impact confirmation per FR-043, FR-046, and US4/AC3 (`missing`)
- [X] T092 Extend `ServerFavoriteView` in `crates/thriftp-core/src/server_favorites/mod.rs`, `apps/desktop/src/lib/ipc/types.ts`, and the IPC contracts with `timeoutSeconds` and `sshKeyRef`, and add round-trip tests proving edit prefill preserves them per FR-003 and FR-005 (`partial`)
- [X] T093 Wire `CredentialUnavailableBanner` repair/re-entry in `apps/desktop/src/App.svelte`, name the affected favorite and credential kind in `CredentialPromptDialog.svelte`, and disable secure saving when the store is unavailable per FR-023 and FR-059 (`partial`)
- [X] T094 Align the remaining IPC surface with `specs/003-server-favorites/contracts/favorites-ipc.md`: `favorites_rename_group` output, `favorites_connect` `ConnectionState` with `favoriteId`, `credentials_remove` output, `pinned`/`secretKind` handling, and `favorites:changed`, `connection:state`, and `credentials:changed` events, with contract tests (`partial`)
- [X] T095 Harden versioned persistence in `crates/thriftp-storage-json/src/lib.rs` and a dedicated `crates/thriftp-storage-json/src/server_favorites.rs`: flush/fsync, `0600` on Unix, backup of the previous valid document, ordered migrations, and backup/safe-empty recovery, plus adapter tests per plan: storage decision, FR-010, and FR-065 (`partial`)
- [X] T096 Extend `.github/workflows/ci.yml` so desktop Rust tests and frontend Vitest run on macOS and Windows as well as Ubuntu, separate from the deferred T083 native credential validation per T003 and Constitution V (`partial`)
- [X] T097 Replace raw protocol enum text with friendly protocol labels and make favorites component colors theme-aware in `apps/desktop/src/components/server-favorites/` per FR-018 and Constitution IV (`partial`)
- [X] T098 Align the implementation layout with the plan (`server_favorites/repository.rs`, `server_favorites/service.rs`, `storage-json/src/server_favorites.rs`, `commands/credentials.rs`, `lib/server-favorites/types.ts`) or record the approved deviation in `docs/server-favorites.md` per the plan's structure decision (`contradicts`)

## Phase 10: Convergence

- [X] T099 Revalidate credential identity in `ServerFavoriteService::connection_target` and reconcile external profile changes so CLI `profile update` invalidates stale references, CLI `profile delete` removes references by derived `CredentialRef` id, and tabs referencing a deleted profile are cleaned per FR-044, FR-045, and FR-069 (`partial`)
- [X] T100 Update `specs/003-server-favorites/contracts/favorites-ipc.md` so `ServerFavoriteView` includes `timeoutSeconds`, `sshKeyRef`, and `credentialWarning`, matching the implemented Rust and TypeScript views per T092 (`partial`)
- [X] T101 Emit complete `credentials:changed.affectedFavoriteIds` sets for shared create, update, and delete operations, and emit the corresponding `favorites:changed` credential events, with contract tests per T094 and `contracts/favorites-ipc.md` (`partial`)
- [X] T102 Apply theme-aware colors and CSS variables to `CredentialImpactDialog.svelte`, `DeleteCredentialDialog.svelte`, `CredentialPromptDialog.svelte`, `ReuseCredentialDialog.svelte`, `EditFavoriteDialog.svelte`, and `SaveFavoriteDialog.svelte` per FR-018 and Constitution IV (`partial`)
- [X] T103 Make `connections_create` fail atomically when the credential store is unavailable, or persist the profile with a structured warning and no stored secret, matching `favorites_create` behavior per FR-051 and T088 (`partial`)
