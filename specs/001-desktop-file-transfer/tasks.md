---

description: "Task list for the Thriftp desktop file transfer client implementation"
---

# Tasks: Thriftp Desktop File Transfer Client

**Input**: Design documents from `/specs/001-desktop-file-transfer/`

**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Tests are included because the feature specification and constitution require automated tests. The task order follows the plan's quality gates; tests for each story are listed before that story's implementation.

**Organization**: Tasks are grouped by user story so each increment can be implemented, tested, and demonstrated independently.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel with other `[P]` tasks in the same phase (different files, no incomplete dependencies).
- **[Story]**: Maps the task to a user story (`US1`–`US7`). Setup, Foundational, and Polish tasks have no story label.
- Every task includes an exact file path or directory.

## Path Conventions

- Rust crates: `crates/<crate>/src/...`, `crates/<crate>/tests/...`
- Tauri shell: `apps/desktop/src-tauri/src/...`
- Svelte UI: `apps/desktop/src/...`
- Container harness: `docker/protocol/`, `docker/ftp/`, `docker/sftp/`
- Shared scripts: `scripts/`
- Cross-crate tests: `tests/integration/`, `tests/e2e/`
- Documentation: `docs/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the workspace, app shells, test harness, and CI foundations.

- [X] T001 Create the Cargo workspace and crate skeletons in `Cargo.toml`, `crates/thriftp-core/Cargo.toml`, `crates/thriftp-protocols/Cargo.toml`, `crates/thriftp-credentials-keyring/Cargo.toml`, and `crates/thriftp-storage-json/Cargo.toml`.
- [X] T002 Initialize the Tauri 2 + Svelte 5 + TypeScript app in `apps/desktop/package.json`, `apps/desktop/vite.config.ts`, `apps/desktop/src-tauri/Cargo.toml`, and `apps/desktop/src/`.
- [X] T003 [P] Configure Rust formatting and linting in `./rustfmt.toml`, `./clippy.toml`, and `./Cargo.toml` workspace lint settings.
- [X] T004 [P] Configure frontend linting, formatting, and Vitest in `apps/desktop/eslint.config.js`, `apps/desktop/.prettierrc`, and `apps/desktop/vitest.config.ts`.
- [X] T005 [P] Configure CI stages for unit, protocol-integration, platform, frontend, and build in `.github/workflows/ci.yml`.
- [X] T006 [P] Create the Docker Compose protocol harness with pinned images and health checks in `docker/protocol/docker-compose.yml`.
- [X] T007 [P] Create FTP/FTPS container definitions and TLS certificate fixtures in `docker/ftp/Dockerfile`, `docker/ftp/vsftpd.conf`, and `docker/protocol/certs/`.
- [X] T008 [P] Create the SFTP container definition and SSH host-key fixtures in `docker/sftp/Dockerfile`, `docker/sftp/sshd_config`, and `docker/sftp/keys/`.
- [X] T009 [P] Create `scripts/test-integration.sh` with runtime detection, compose startup, health wait, endpoint export, test execution, teardown, and a clear skip path when no container runtime exists.
- [X] T010 [P] Configure Criterion, integration-test directories, and test naming under `crates/thriftp-core/benches/`, `crates/thriftp-protocols/tests/`, and `tests/integration/`.
- [X] T011 [P] Configure dependency pinning, lockfile enforcement, dependency auditing, and release SBOM generation in `Cargo.toml`, `apps/desktop/package.json`, and `.github/workflows/ci.yml`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared models, traits, storage, credential access, IPC scaffolding, and test support required by every story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T012 Implement the core error taxonomy and user-facing error codes in `crates/thriftp-core/src/error.rs`.
- [X] T013 [P] Create `ConnectionProfile` and `DirectoryFavorite` models in `crates/thriftp-core/src/model/profile.rs` and `crates/thriftp-core/src/model/favorite.rs` with constraints: displayName 1–200 characters; groupLabel 1–100; host 1–255; port 1–65535; timeoutSeconds 1–300; protocol `ftp|ftpsExplicit|ftpsImplicit|sftp`; authMethod `password|sshPrivateKey`; DirectoryFavorite label 1–200 with a canonical path. `ServerFavorite` is defined in `003`, not here.
- [X] T014 [P] Create `CredentialRef`, `FileEntry`, and `BrowsingSession` models in `crates/thriftp-core/src/model/credential.rs`, `crates/thriftp-core/src/model/file.rs`, and `crates/thriftp-core/src/model/browsing.rs` with constraints: CredentialRef never contains a secret and binds protocol/host/port/username/authMethod; FileEntry entryType `file|directory|symlink|other` with nullable size/date/permissions/ownership; BrowsingSession keeps history, historyIndex, viewSettings, selectedItems, and restored state.
- [X] T015 [P] Create `Transfer`, `PartialTransferData`, `TransferQueue`, `TransferHistory`, `Preference`, `ActiveConnection`, and `DirectoryListing` models in `crates/thriftp-core/src/model/transfer.rs`, `queue.rs`, `preference.rs`, and `listing.rs` with constraints: transfer states `pending|active|paused|completed|failed|cancelled|interrupted`; progress never exceeds known total; concurrency default 4; bandwidth null means unlimited; history limit 1,000; partial data linked to exactly one transfer; theme `system|light|dark`; restoreTabs default true; previewMode `auto|native|internal`; listing states `loading|partial|complete|error`.
- [X] T016 [P] Implement the `CredentialStore` trait and zeroizing `Secret` type in `crates/thriftp-core/src/credentials/traits.rs`.
- [X] T017 [P] Implement the native `keyring` adapter in `crates/thriftp-credentials-keyring/src/lib.rs` with fail-closed availability, get, set, and delete behavior.
- [X] T018 [P] Implement the `RemoteFs` trait and `ProtocolCapabilities` in `crates/thriftp-protocols/src/traits.rs`.
- [X] T019 [P] Implement the JSON metadata and transfer-state stores with schema versions, atomic writes, backups, and migrations in `crates/thriftp-storage-json/src/lib.rs`, `favorites.rs`, and `transfers.rs`.
- [X] T020 Implement core validation and credential identity binding in `crates/thriftp-core/src/validation.rs`.
- [X] T021 [P] Implement deterministic sorting and comparator helpers for favorites, browser entries, and queue ordering in `crates/thriftp-core/src/sorting.rs`.
- [X] T022 [P] Implement structured `tracing` setup and secret redaction in `crates/thriftp-core/src/observability.rs`.
- [X] T023 Implement Tauri bootstrap, state wiring, command registration, and event scaffolding in `apps/desktop/src-tauri/src/lib.rs` and `apps/desktop/src-tauri/src/main.rs`.
- [X] T024 [P] Implement the typed Svelte IPC client and base stores in `apps/desktop/src/lib/ipc/client.ts` and `apps/desktop/src/lib/ipc/stores.ts`.
- [X] T025 [P] Implement test fakes for `RemoteFs`, `CredentialStore`, and temporary JSON storage in `crates/thriftp-core/tests/support/`.
- [X] T026 [P] Implement container endpoint fixture helpers for protocol integration tests in `crates/thriftp-protocols/tests/support/` and `tests/integration/support/`.
- [X] T027 Implement preference defaults and persistence in `crates/thriftp-core/src/preferences/service.rs`.
- [X] T028 Document test classes, local commands, and container requirements in `docs/testing.md`.

**Checkpoint**: Foundation ready. US1 can start; dependent stories may begin as their prerequisites complete.

---

## Phase 3: User Story 1 - Connect securely to a remote server (Priority: P1) 🎯 MVP

**Goal**: Users can create, save, and use FTP, explicit FTPS, implicit FTPS, and SFTP connection profiles with native credential storage and verification.

**Independent Test**: Create one profile per protocol, connect with password and SSH key where supported, restart the app, and reconnect without re-entering non-secret settings; verify TLS and host-key failures are visible.

### Tests for User Story 1

- [X] T029 [P] [US1] Add connection IPC contract tests for profile CRUD, connect, disconnect, test, and error mapping in `apps/desktop/src-tauri/tests/connections_contract.rs`.
- [X] T030 [P] [US1] Add protocol integration tests for FTP, explicit FTPS, implicit FTPS, SFTP password/key auth, TLS failure, and host-key failure in `crates/thriftp-protocols/tests/connection_integration.rs`.

### Implementation for User Story 1

- [X] T031 [P] [US1] Implement the FTP/FTPS adapter in `crates/thriftp-protocols/src/ftp.rs` with explicit/implicit modes, rustls validation, and plain-FTP marking.
- [X] T032 [P] [US1] Implement the SFTP adapter in `crates/thriftp-protocols/src/sftp.rs` with password/key auth and known-hosts verification.
- [X] T033 [US1] Implement the connection service for profile CRUD, connect, disconnect, and test in `crates/thriftp-core/src/connections/service.rs`.
- [X] T034 [US1] Implement credential binding, lookup, update, removal, and fail-closed errors in `crates/thriftp-core/src/credentials/service.rs`.
- [X] T035 [US1] Expose the connection-profile and credential interfaces consumed by the `003` server-favorites slice in `crates/thriftp-core/src/connections/service.rs` and `crates/thriftp-core/src/credentials/service.rs`.
- [X] T036 [US1] Implement Tauri connection commands in `apps/desktop/src-tauri/src/commands/connections.rs`.
- [X] T037 [US1] Implement the connection editor UI in `apps/desktop/src/components/connections/ConnectionEditor.svelte`.
- [X] T038 [US1] Implement connection status and error UI in `apps/desktop/src/components/connections/ConnectionStatus.svelte`.
- [X] T039 [US1] Implement plain-FTP warning and explicit verification override UI in `apps/desktop/src/components/connections/SecurityPrompt.svelte`.
- [X] T040 [US1] Implement credential-store unavailable, missing, and rejected credential flows in `apps/desktop/src/components/connections/CredentialPrompt.svelte`.
- [X] T041 [US1] Implement multiple connections, reconnect, and successful-connection last-used updates in `crates/thriftp-core/src/connections/service.rs`.
- [X] T042 [US1] Add connection/credential `tracing` spans with redaction in `crates/thriftp-core/src/connections/service.rs` and `crates/thriftp-core/src/credentials/service.rs`.
- [X] T043 [US1] Add unit tests for profile persistence, reconnect, FTPS modes, and credential failures in `crates/thriftp-core/tests/connections.rs`.

**Checkpoint**: US1 is the MVP.

---

## Phase 4: User Story 2 - Browse local and remote files in a dual-pane workspace (Priority: P1)

**Goal**: Users can browse local and remote directories in two panels, inspect metadata, sort, refresh, perform file operations, and preview files.

**Independent Test**: Open both panels, navigate nested directories, sort and refresh, toggle hidden files, create/rename/delete entries, and open a native or fallback preview.

### Tests for User Story 2

- [X] T044 [P] [US2] Add IPC contract tests for streaming local/remote listings and file operations in `apps/desktop/src-tauri/tests/browser_contract.rs`.
- [X] T045 [P] [US2] Add UI tests for dual-pane layout, sorting, hidden files, loading states, and preview fallback in `apps/desktop/src/components/browser/browser.test.ts`.

### Implementation for User Story 2

- [X] T046 [P] [US2] Implement the local filesystem browser in `crates/thriftp-core/src/browser/local.rs`.
- [X] T047 [P] [US2] Implement the remote browser over `RemoteFs` in `crates/thriftp-core/src/browser/remote.rs`.
- [X] T048 [US2] Implement streaming listing batches, cancellation, first-entry timing, and complete-listing state in `crates/thriftp-core/src/browser/listing.rs`.
- [X] T049 [US2] Implement stat, mkdir, rename, and delete operations in `crates/thriftp-core/src/browser/operations.rs`.
- [X] T050 [US2] Implement local OS-trash deletion and remote permanent-delete warnings in `crates/thriftp-core/src/browser/delete.rs`.
- [X] T051 [US2] Implement Tauri browser commands and listing events in `apps/desktop/src-tauri/src/commands/browser.rs`.
- [X] T052 [US2] Implement the dual-pane layout and active-panel behavior in `apps/desktop/src/components/browser/DualPane.svelte`.
- [X] T053 [US2] Implement the virtualized directory list, first-entry display, and loading indicator in `apps/desktop/src/components/browser/DirectoryList.svelte`.
- [X] T054 [US2] Implement the location bar, parent navigation, refresh, sort, hidden-file, and filter controls in `apps/desktop/src/components/browser/BrowserToolbar.svelte`.
- [X] T055 [US2] Implement browser keyboard shortcuts and focus management in `apps/desktop/src/lib/keyboard/browser.ts`.
- [X] T056 [P] [US2] Implement the native preview Tauri command in `apps/desktop/src-tauri/src/commands/previews.rs`.
- [X] T057 [P] [US2] Implement the internal text/image preview fallback with size limits in `apps/desktop/src/components/previews/InternalPreview.svelte`.
- [X] T058 [US2] Implement remote preview consent and bounded-download behavior in `crates/thriftp-core/src/previews/service.rs`.
- [X] T059 [US2] Add local/remote listing performance instrumentation in `crates/thriftp-core/src/browser/metrics.rs`.
- [X] T060 [US2] Add tests for listing performance, metadata, hidden files, sorting, deletion, and preview fallback in `crates/thriftp-core/tests/browser.rs`.

**Checkpoint**: US2 works independently for local browsing and with US1 for remote browsing.

---

## Phase 5: User Story 3 - Transfer files and directories with clear progress and control (Priority: P1)

**Goal**: Users can stream uploads and downloads, transfer multiple files and directories, pause/resume/cancel, handle conflicts and symlinks, and see accurate progress.

**Independent Test**: Upload/download a file, a multi-file selection, and a nested directory; verify progress, pause/resume, cancellation, conflict choices, symlink behavior, and constant memory for large files.

### Tests for User Story 3

- [X] T061 [P] [US3] Add protocol transfer integration tests for uploads, downloads, directories, conflicts, and symlinks in `crates/thriftp-protocols/tests/transfer_integration.rs`.
- [X] T062 [P] [US3] Add transfer-engine unit tests for chunking, progress, pause, cancel, and O(1) memory in `crates/thriftp-core/tests/transfers.rs`.

### Implementation for User Story 3

- [X] T063 [P] [US3] Implement chunked stream copy and progress accounting in `crates/thriftp-core/src/transfers/stream.rs`.
- [X] T064 [P] [US3] Implement local and remote reader/writer adapters in `crates/thriftp-core/src/transfers/io.rs`.
- [X] T065 [US3] Implement the transfer lifecycle and state machine in `crates/thriftp-core/src/transfers/engine.rs`.
- [X] T066 [US3] Implement recursive traversal and preserve-or-skip symbolic-link policy in `crates/thriftp-core/src/transfers/recursive.rs`.
- [X] T067 [US3] Implement conflict policy and apply-to-all callbacks in `crates/thriftp-core/src/transfers/conflict.rs`.
- [X] T068 [US3] Implement individual pause/resume and progress/state events in `crates/thriftp-core/src/transfers/control.rs`.
- [X] T069 [US3] Implement partial-file naming, metadata, and resume verification in `crates/thriftp-core/src/transfers/resume.rs`.
- [X] T070 [US3] Implement Tauri transfer commands and events in `apps/desktop/src-tauri/src/commands/transfers.rs`.
- [X] T071 [US3] Implement transfer progress, state, and completion/failure notifications in `apps/desktop/src/components/transfers/TransferProgress.svelte`.
- [X] T072 [US3] Implement conflict resolution UI with overwrite, skip, rename, and apply-to-all in `apps/desktop/src/components/transfers/ConflictDialog.svelte`.
- [X] T073 [US3] Implement drag-and-drop transfer initiation in `apps/desktop/src/components/browser/DualPane.svelte`.
- [X] T074 [US3] Implement transfer speed, ETA, and state labels in `apps/desktop/src/components/transfers/TransferList.svelte`.
- [X] T075 [US3] Add tests for large-file memory, progress, cancellation, conflicts, and symlink handling in `crates/thriftp-core/tests/transfers.rs`.

**Checkpoint**: US3 works after US1/US2 and delivers the core transfer workflow.

---

## Phase 6: User Story 4 - Manage and control all transfers from a dedicated queue (Priority: P2)

**Goal**: Users can view, pause/resume, cancel, reorder, prioritize, and bandwidth-limit transfers from a persistent queue.

**Independent Test**: Queue several transfers, change concurrency, pause/resume the queue, set a bandwidth limit, reorder priorities, cancel one/all, and verify status while navigating.

### Tests for User Story 4

- [X] T076 [P] [US4] Add queue tests for priority, concurrency, pause, bandwidth, and cancel-all behavior in `crates/thriftp-core/tests/queue.rs`.
- [X] T077 [P] [US4] Add queue UI tests for progress, status, controls, and reordering in `apps/desktop/src/components/queue/queue.test.ts`.

### Implementation for User Story 4

- [X] T078 [US4] Implement the scheduler, priority ordering, and concurrency semaphore in `crates/thriftp-core/src/queue/scheduler.rs`.
- [X] T079 [US4] Implement the global bandwidth token bucket in `crates/thriftp-core/src/queue/bandwidth.rs`.
- [X] T080 [US4] Implement queue pause/resume, reorder, priority, and cancel-all service behavior in `crates/thriftp-core/src/queue/service.rs`.
- [X] T081 [US4] Implement queue/history persistence and checkpoint scheduling in `crates/thriftp-storage-json/src/transfers.rs` and `crates/thriftp-core/src/queue/persistence.rs`.
- [X] T082 [US4] Implement Tauri queue commands in `apps/desktop/src-tauri/src/commands/queue.rs`.
- [X] T083 [US4] Implement the queue manager UI with per-transfer/global progress and controls in `apps/desktop/src/components/queue/QueueManager.svelte`.
- [X] T084 [US4] Add tests for ordering, queue pause, bandwidth enforcement, and persistence in `crates/thriftp-core/tests/queue.rs`.

**Checkpoint**: US4 works after US3 and adds scheduling/control.

---

## Phase 7: User Story 5 - Organize work with tabs, favorites, and navigation history (Priority: P2)

**Goal**: Users can use tabs, restore them on launch, save server/directory favorites, and navigate back/forward.

**Independent Test**: Open multiple tabs, navigate each independently, save favorites, restart, and verify tab/favorite restoration; move back and forward through history.

### Tests for User Story 5

- [X] T085 [P] [US5] Add tab/favorites/navigation unit tests in `crates/thriftp-core/tests/favorites_tabs.rs`.
- [X] T086 [P] [US5] Add UI tests for tabs, favorites sidebar/list, and save-as-favorite flow in `apps/desktop/src/components/favorites/favorites.test.ts`.

### Implementation for User Story 5

- [X] T087 [US5] Implement tab state persistence and restore-on-launch behavior in `crates/thriftp-core/src/preferences/tabs.rs`.
- [X] T088 [US5] Implement the directory favorite service in `crates/thriftp-core/src/favorites/service.rs`; server favorites remain owned by `003`.
- [X] T089 [US5] Implement navigation history, back, and forward in `crates/thriftp-core/src/browser/history.rs`.
- [X] T090 [US5] Implement Tauri directory-favorites commands in `apps/desktop/src-tauri/src/commands/favorites.rs`; server favorites are owned by `003`.
- [X] T091 [US5] Implement the tabs UI and directory favorites sidebar/list in `apps/desktop/src/components/favorites/FavoritesSidebar.svelte` and `apps/desktop/src/components/tabs/TabBar.svelte`.
- [X] T092 [US5] Implement the save-current-remote-directory-as-directory-favorite flow in `apps/desktop/src/components/favorites/SaveDirectoryFavoriteDialog.svelte`.
- [X] T093 [US5] Implement restore-tabs preference and disconnected restored-tab state in `apps/desktop/src/lib/tabs/restore.ts`.
- [X] T094 [US5] Add tests for tab restoration, favorites persistence, and navigation history in `crates/thriftp-core/tests/favorites_tabs.rs`.

**Checkpoint**: US5 works after US1/US2 and adds organization/navigation.

---

## Phase 8: User Story 7 - Recover interrupted transfers after restart (Priority: P2)

**Goal**: Users can restart the app and find queue/history intact, interrupted transfers resumable, and resume safety verified.

**Independent Test**: Interrupt a transfer, restart, confirm queue/history and interrupted state, resume, then change the source and confirm resume is refused.

### Tests for User Story 7

- [X] T095 [P] [US7] Add recovery tests for interrupted/pending states, resume verification, partial files, and history eviction in `crates/thriftp-core/tests/recovery.rs`.
- [X] T096 [P] [US7] Add recovery UI tests for resume/restart/cancel and partial-file actions in `apps/desktop/src/components/transfers/recovery.test.ts`.

### Implementation for User Story 7

- [X] T097 [US7] Implement startup recovery state mapping in `crates/thriftp-core/src/queue/recovery.rs`.
- [X] T098 [US7] Implement transfer history persistence, 1,000-record eviction, and user clear in `crates/thriftp-storage-json/src/transfers.rs`.
- [X] T099 [US7] Implement resume verification against source size/modification time and destination offset in `crates/thriftp-core/src/transfers/resume.rs`.
- [X] T100 [US7] Implement retain/discard partial-file actions in `crates/thriftp-core/src/transfers/partials.rs`.
- [X] T101 [US7] Implement the recovery UI for interrupted transfers and partial files in `apps/desktop/src/components/transfers/RecoveryPanel.svelte`.
- [X] T102 [US7] Implement the normal-quit wait/cancel flow in `apps/desktop/src-tauri/src/lifecycle.rs` and `apps/desktop/src/lib/lifecycle/quit.ts`.
- [X] T103 [US7] Add tests for changed-source refusal, restart/cancel, partial-file handling, and history eviction in `crates/thriftp-core/tests/recovery.rs`.

**Checkpoint**: US7 works after US3/US4 and completes the reliability loop.

---

## Phase 9: User Story 6 - Inspect and manage file metadata and permissions (Priority: P3)

**Goal**: Users can inspect file metadata, view ownership, and modify Unix permissions where supported.

**Independent Test**: Open the inspector on local and remote entries, modify permissions on a supporting server, and verify unsupported operations are explained gracefully.

### Tests for User Story 6

- [X] T104 [P] [US6] Add metadata/permission tests in `crates/thriftp-core/tests/metadata.rs`.
- [X] T105 [P] [US6] Add inspector UI tests in `apps/desktop/src/components/inspector/inspector.test.ts`.

### Implementation for User Story 6

- [X] T106 [US6] Implement stat metadata, permissions, and ownership mapping in `crates/thriftp-core/src/browser/metadata.rs`.
- [X] T107 [US6] Implement chmod and unsupported-capability handling in `crates/thriftp-core/src/browser/permissions.rs`.
- [X] T108 [US6] Implement the inspector Tauri command in `apps/desktop/src-tauri/src/commands/inspector.rs`.
- [X] T109 [US6] Implement the file information inspector UI in `apps/desktop/src/components/inspector/Inspector.svelte`.
- [X] T110 [US6] Add tests for permissions, ownership, and unsupported operations in `crates/thriftp-core/tests/metadata.rs`.

**Checkpoint**: US6 works after US2 and adds metadata/permission management.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Benchmarks, observability, accessibility, documentation, security review, and full validation.

- [X] T111 [P] Add Criterion benchmarks for protocol parsing, listing batches, and the transfer chunk loop in `crates/thriftp-core/benches/`, `crates/thriftp-protocols/benches/`, and `crates/thriftp-core/benches/transfer.rs`.
- [X] T112 [P] Audit and complete structured tracing/redaction across core, protocols, and Tauri in `crates/thriftp-core/src/observability.rs` and `apps/desktop/src-tauri/src/lib.rs`.
- [X] T113 [P] Complete the accessibility and keyboard pass across the UI in `apps/desktop/src/lib/accessibility/` and `apps/desktop/src/components/`.
- [X] T114 [P] Write desktop, security, and architecture documentation in `docs/desktop.md`, `docs/security.md`, and `docs/architecture.md`.
- [X] T115 [P] Verify CI stages, container harness documentation, and local commands in `.github/workflows/ci.yml` and `docs/testing.md`.
- [X] T116 Run `scripts/test-integration.sh` and the `quickstart.md` scenarios; fix all failures.
- [X] T117 Perform the security review for plaintext secrets, TLS/host-key verification, trash behavior, SBOM, pinned dependencies, lockfiles, no telemetry endpoints, no account dependency, and no unnecessary user-data collection.
- [X] T118 Validate local/remote 10,000-entry performance, 4 concurrent transfers, O(1) memory, and 500 favorites.
- [X] T119 Run the full macOS/Linux/Windows build and test matrix, then update `README.md` and `docs/`.
- [X] T120 [P] Document privacy defaults, no-account operation, and no mandatory telemetry in `docs/security.md` and `docs/desktop.md`.
- [X] T121 [P] Add an integration test proving the core workflow works with no account, subscription, cloud service, or remote authentication service in `tests/integration/offline_workflow.rs`.
- [X] T122 [P] Add a privacy test asserting that no telemetry or user-data transmission is enabled by default in `crates/thriftp-core/tests/privacy.rs`.
- [X] T123 [P] Apply the system/light/dark theme across the UI and add theme tests in `apps/desktop/src/lib/theme/apply.ts` and `apps/desktop/src/lib/theme/theme.test.ts`.
- [X] T124 [P] Add timing assertions for connection readiness (<60 s), cancellation (<2 s), and queue pause (<2 s) in `tests/integration/timing.rs` using a controlled clock/test harness.
- [X] T125 [P] Add an end-to-end core-workflow test for connect, browse, upload, and download without protocol documentation in `tests/e2e/core_workflow.spec.ts`.
- [X] T126 [P] Add a contributor build validation script in `scripts/validate-contributor-build.sh` and document it in `docs/testing.md`.
- [X] T127 [P] Add the MIT `./LICENSE`, `./CONTRIBUTING.md`, and `./SECURITY.md` at the repository root.
- [X] T128 [P] Add a dependency-license inventory and CI check in `scripts/check-licenses.sh` and `.github/workflows/ci.yml`.
- [X] T129 [P] Document the public repository and release process in `README.md` and `docs/release.md`.
- [X] T130 [P] Add UI acknowledgement-latency tests for browser, queue, and connection actions in `apps/desktop/src/lib/performance/acknowledgement.test.ts`; use fake timers and assert visible feedback within 100 ms.
- [X] T131 [P] Add a clean-room reproducible-build check that builds twice and compares artifact hashes/SBOM in `scripts/verify-reproducible-build.sh` and `.github/workflows/ci.yml`, documented in `docs/testing.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: Depends on Foundational; MVP.
- **US2 (Phase 4)**: Depends on Foundational and US1 for remote browsing; local browsing can start after Foundational.
- **US3 (Phase 5)**: Depends on US1 and US2.
- **US4 (Phase 6)**: Depends on US3.
- **US5 (Phase 7)**: Depends on US1 and US2; can run in parallel with US4 after those are complete.
- **US7 (Phase 8)**: Depends on US3 and US4.
- **US6 (Phase 9)**: Depends on US2; can run in parallel with US4/US5/US7.
- **Polish (Phase 10)**: Depends on the desired user stories.

### User Story Dependencies

- **US1 (P1)**: No story dependencies beyond Foundational.
- **US2 (P1)**: Remote browsing integrates with US1; local browsing is independently testable.
- **US3 (P1)**: Requires US1 and US2.
- **US4 (P2)**: Requires US3.
- **US5 (P2)**: Requires US1 and US2.
- **US7 (P2)**: Requires US3 and US4.
- **US6 (P3)**: Requires US2.

### Within Each User Story

- Tests are listed before implementation; ensure the relevant contract/integration test exists and fails before the implementation tasks.
- Models and shared utilities precede services.
- Services precede Tauri commands and UI wiring.
- Core behavior precedes UI integration.

---

## Parallel Opportunities

### Setup and Foundational

- T003–T011 can run in parallel after T001/T002.
- T013–T022 can run in parallel once their shared files exist; T012 and T019 should land before services that depend on them.
- T024–T026 can run in parallel with T023.

### US1

```text
T029 connection contract tests
T030 protocol connection integration tests
T031 FTP/FTPS adapter
T032 SFTP adapter
```

### US2

```text
T044 browser IPC contract tests
T045 browser UI tests
T046 local browser
T047 remote browser
T056 native preview command
T057 internal preview fallback
```

### US3

```text
T061 protocol transfer integration tests
T062 transfer-engine unit tests
T063 chunked stream copy
T064 local/remote reader/writer adapters
```

### US4

```text
T076 queue unit tests
T077 queue UI tests
T078 scheduler
T079 bandwidth token bucket
```

### US5

```text
T085 tab/favorites/navigation tests
T086 favorites UI tests
T087 tab persistence
T089 navigation history
```

### US7

```text
T095 recovery tests
T096 recovery UI tests
T097 startup recovery mapping
T098 history persistence
```

### US6

```text
T104 metadata/permission tests
T105 inspector UI tests
T106 metadata mapping
T107 chmod/permission handling
```

### Polish

T111–T115 and T120–T131 can run in parallel; T116–T119 are sequential validation gates.

---

## Implementation Strategy

### MVP First (US1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: US1.
4. **STOP and VALIDATE**: Run `crates/thriftp-core/tests/connections.rs`, `apps/desktop/src-tauri/tests/connections_contract.rs`, and the protocol connection integration tests.
5. Demo secure FTP/FTPS/SFTP connection management.

### Incremental Delivery

1. Setup + Foundational → workspace and shared core ready.
2. US1 → secure connection MVP.
3. US2 → dual-pane browsing and previews.
4. US3 → streaming transfers and conflict handling.
5. US4 → queue scheduling, bandwidth, and priority.
6. US5 → tabs, favorites, and navigation.
7. US7 → restart/recovery and resume safety.
8. US6 → metadata and permissions.
9. Polish → benchmarks, accessibility, docs, security, and full validation.

### Parallel Team Strategy

- One developer owns Setup/Foundational core primitives.
- After Foundational:
  - Developer A: US1 protocol adapters and connection service.
  - Developer B: US2 local browser, listing, and UI shell.
  - Developer C: storage/credential adapters and test harness.
- After US1/US2:
  - Developer A: US3 transfer engine.
  - Developer B: US4 queue and bandwidth.
  - Developer C: US5 tabs/favorites and US6 inspector.
- US7 follows US3/US4.

---

## Notes

- Every task includes a concrete file path or directory.
- Tests are required by the feature specification and constitution; they are not optional in this task list.
- Docker is required only for containerized protocol integration tests and is never an application runtime dependency.
- Server-favorite UX, persistence, and data model are owned by feature `003`; tasks here only expose the shared interfaces.
- `[P]` tasks touch different files and have no incomplete dependencies.
- Validate each story at its checkpoint before moving to the next priority.

---

## Phase 11: Convergence

**Purpose**: Close the gaps between the current implementation and the specification, plan, and constitution.

- [X] T132 CRITICAL: Restore a compiling Tauri desktop shell by exhaustively mapping every `CoreError` variant in `apps/desktop/src-tauri/src/error.rs` and fixing any remaining build/test errors in `apps/desktop/src-tauri` per plan: thin Tauri shell, Constitution V, and SC-001 (partial).
- [X] T133 CRITICAL: Refresh `apps/desktop/src-tauri/Cargo.lock`, make locked builds succeed for the Tauri shell, and add CI jobs that build and run the desktop shell and its IPC contract tests on macOS, Linux, and Windows per Constitution III, Constitution V, and SC-017 (partial).
- [X] T134 CRITICAL: Replace immediate per-transfer worker spawning with a scheduler that honors the concurrency limit, priority and order, queue pause, and the global bandwidth token bucket in `apps/desktop/src-tauri/src/commands/transfers.rs` and `crates/thriftp-core/src/queue.rs` per FR-049, FR-050, FR-051, SC-006, SC-009, SC-010, and Constitution IV (missing).
- [X] T135 CRITICAL: Implement conflict detection and overwrite/skip/rename/apply-to-all resolution in the transfer engine and wire `apps/desktop/src/components/transfers/ConflictDialog.svelte` into the transfer-start flow per FR-044, US3/AC7, and Constitution IV (missing).
- [X] T136 CRITICAL: Surface the unencrypted-transport warning during plain-FTP sessions by including `plain_ftp_warning` in desktop connection views and status and keeping it visible while connected per FR-105 and Constitution III (partial).
- [X] T137 Verify resumable source and destination state with `recovery::verify_resume` or `transfers::resume::verify` before `transfers_resume` continues, and offer restart or cancel when verification fails per FR-059, SC-013, US7/AC3, and US7/AC4 (missing).
- [X] T138 Write downloads through `.thriftp-part` files with sidecar metadata, retain and identify partial data, implement discard and remote cleanup reporting, and persist `partial_transfers` in `transfers.json` per FR-057, FR-058, and US7/AC6 (missing).
- [X] T139 Record terminal transfers in persisted history with 1,000-record eviction and a clear-history command and UI, use the checkpoint scheduler on state transitions, and persist all queue mutations per FR-052, SC-014, T081, T098, and the transfer-engine persistence contract (partial).
- [X] T140 Stream remote directory entries in batches from the protocol adapters instead of awaiting the complete `list_dir` result, and make cancellation abort enumeration, so first entries appear within 2 seconds per FR-023, SC-004, and US2/AC8 (partial).
- [X] T141 Add multi-file selection to `apps/desktop/src/components/browser/DirectoryList.svelte` and pass every selected source to `transfers_start` so multi-file upload and download are supported per FR-034 and US3 (missing).
- [X] T142 Wire `lifecycle_prepare_quit` into the window close path and `apps/desktop/src/lib/lifecycle/quit.ts` so quitting with pending, active, or paused transfers warns and requires wait-for-completion or cancel per FR-055, FR-117, and US4/AC5 (missing).
- [X] T143 Implement consented, bounded remote preview downloads up to 25 MB that render in `apps/desktop/src/components/previews/InternalPreview.svelte`, fall back to metadata for oversized, unknown-size, or binary files, and add the contracted preview close and state handling per FR-076, FR-078, SC-021, and the desktop-ipc preview contract (partial).
- [X] T144 Add `preferences_get` and `preferences_set` commands backed by `PreferencesService`, stop `AppState::persist_metadata` from overwriting stored preferences with `Preference::default()`, and apply restoreTabs, theme, hidden-file, sort, concurrency, and preview preferences in the UI per FR-017, FR-087, FR-095, and the desktop-ipc preferences contract (missing).
- [X] T145 Add back and forward navigation controls and keyboard shortcuts backed by `crates/thriftp-core/src/browser/history.rs` and the active tab history per FR-082 and US5/AC4 (missing).
- [X] T146 Allow SFTP private-key connections without a passphrase by making the credential optional for key authentication and validating key-only connections per FR-004 and US1/AC2 (partial).
- [X] T147 Recreate symbolic links where the protocol and destination support it and emit a visible skipped-link report otherwise, without following links, in recursive upload and download per FR-033 and US3/AC8 (partial).
- [X] T148 Persist and restore each tab's local and remote path, sort, hidden-file, filter, and selection state; show a retry action for disconnected restored tabs; and support reopening closed tabs per FR-085, FR-086, US5/AC2, US5/AC6, and the restored-tab edge case (partial).
- [X] T149 Show aggregate queue progress and add a dismiss action for completed, failed, and cancelled transfers in `apps/desktop/src/components/queue/QueueManager.svelte` and `apps/desktop/src/components/transfers/TransferList.svelte` per FR-064 and FR-068 (partial).
- [X] T150 Add directory-favorite reorder and edit controls to `apps/desktop/src/components/favorites/FavoritesSidebar.svelte` and persist ordering per FR-080 (partial).
- [X] T151 Add a duplicate action for saved connection profiles in `apps/desktop/src/components/connections/ConnectionStatus.svelte` and the connection commands per FR-001 (missing).
- [X] T152 Replace browser `prompt` and `confirm` usage for file operations with platform-native dialogs and complete shortcut coverage for common operations per FR-089 and FR-030 (partial).
- [X] T153 Use the capability-checked `browser::permissions::chmod_remote` path in `apps/desktop/src-tauri/src/commands/inspector.rs` and explain unsupported permission or ownership changes without failing navigation per FR-072 and FR-019 (partial).
- [X] T154 Align FTP and FTPS TLS with the plan's `suppaftp` plus `tokio-rustls-ring` stack, or record an approved plan amendment before keeping `native-tls` per plan: TLS stack decision (contradicts).
- [X] T155 Notify users in-app or through a system notification when a transfer completes or fails per FR-040 (partial).
- [X] T156 Make `scripts/verify-reproducible-build.sh` compare compiled artifact hashes in addition to lockfile and metadata and document the check per plan: reproducible build and T131 (partial).
- [X] T157 Throttle progress events to at most every 100 ms or 1 MB, whichever comes first, and emit the active state transition immediately per the transfer-engine progress contract (partial).

---

## Phase 12: Convergence

**Purpose**: Close the remaining gaps found after the Phase 11 implementation pass.

- [X] T158 Implement incremental remote listing for the FTP and FTPS adapters so first entries are emitted before the complete listing, and make cancellation abort enumeration per FR-023, SC-004, and US2/AC8 (partial).
- [X] T159 Integrate `CheckpointScheduler` into active transfer workers and persist progress every 5 seconds or 1 MB so a crash restores the last confirmed offset per FR-053, FR-056, SC-014, and the transfer-engine persistence contract (partial).
- [X] T160 Record cancelled pending and queued transfers in persisted history when cancelling one transfer or all pending transfers per FR-052, FR-063, and SC-014 (partial).
- [X] T161 Emit the active transfer state and queue state immediately when a worker reserves and starts a transfer per the transfer-engine progress/state contract, FR-037, and FR-093 (partial).
- [X] T162 Route local text and image previews through the native operating-system preview where available, with the internal preview as fallback, and honor the preview preference per FR-076, SC-021, and Constitution IV (partial).
- [X] T163 Replace the blocking `std::thread::sleep` bandwidth throttle with an async or cooperative limiter so Tokio workers and UI responsiveness are not blocked per Constitution II and FR-115 (partial).
- [X] T164 Apply `previewMode`, `defaultConcurrency`, and `bandwidthLimitBytesPerSecond` preferences to the active preview path, queue concurrency, and transfer limiter per FR-049, FR-087, and FR-095 (partial).
- [X] T165 Persist per-tab local paths, selection state, and full view settings; add a retry action for disconnected restored tabs and support reopening closed tabs per FR-085, FR-086, US5/AC2, and US5/AC6 (partial).
- [X] T166 Make transfer dismissal persistent by removing dismissed terminal transfers from the backend queue or storing a dismissal marker per FR-068 (partial).
- [X] T167 Make multi-source conflict handling transactional so a conflict does not leave already-enqueued transfers in the manager without dispatch per FR-044 (partial).
- [X] T168 Record source modification time in partial-transfer metadata and verify it before resuming in addition to source size and destination offset per FR-059 and SC-013 (partial).

---

## Phase 13: Convergence

**Purpose**: Close the remaining gap found after the Phase 12 implementation pass.

- [X] T169 Record every transfer cancelled by `queue_cancel_all` in persisted history, including pending and active transfers, per FR-052, FR-063, and SC-014 (partial).

---

## Phase 14: Convergence

**Purpose**: Close the remaining verification and plan-alignment gaps.

- [X] T170 Persist source size and modification time in partial-transfer metadata during active checkpoints, not only on pause or failure, so an unexpected termination can verify resume safety per FR-059, SC-013, and T168 (partial).
- [X] T171 Amend `specs/001-desktop-file-transfer/plan.md` to record the accepted `native-tls` decision or migrate FTP/FTPS to the plan's `tokio-rustls-ring` stack per plan: TLS stack decision and T154 (contradicts).
