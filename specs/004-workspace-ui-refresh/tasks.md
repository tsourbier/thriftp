# Tasks: Workspace UI Refresh and Dual-Pane Connection Hub

**Input**: Design documents from `/specs/004-workspace-ui-refresh/`
**Prerequisites**: `plan.md`, `spec.md`, `research.md`, `data-model.md`, `contracts/`, `quickstart.md`

**Tests**: Required. The specification's SC-014 requires new automated coverage for the workspace layout, hub tabs, palette actions, favorite path restoration, key discovery, breadcrumbs, and sidebar search, and the constitution requires automated tests for significant features. Test tasks are therefore included in every user story phase.

**Organization**: Tasks are grouped by user story so each story can be implemented and tested independently. Tests come first within each story phase.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks).
- **[Story]**: Which user story this task belongs to.
- Every task names an exact file path.

## Path Conventions

- Rust core: `crates/thriftp-core/src/`
- JSON persistence: `crates/thriftp-storage-json/src/` and `crates/thriftp-storage-json/tests/`
- Tauri commands and contract tests: `apps/desktop/src-tauri/src/` and `apps/desktop/src-tauri/tests/`
- Svelte UI and Vitest tests: `apps/desktop/src/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Configure the supported window bounds and create shared test scaffolding used by multiple stories.

- [X] T001 Set the supported minimum window size to 900 × 600 logical pixels by adding `"minWidth": 900` and `"minHeight": 600` to the window entry in `apps/desktop/src-tauri/tauri.conf.json`.
- [X] T002 [P] Create a reusable typed Tauri `invoke` mock helper in `apps/desktop/src/lib/test-utils/ipc.ts` that supports per-command handlers, records calls, and resets between tests.
- [X] T003 [P] Create shared frontend fixtures in `apps/desktop/src/lib/test-utils/fixtures.ts` for `WorkspaceLayout` (`splitRatio: 0.5`, `sidebarVisible: true`, all sections `true`, `hubTab: "favorites"`), `ServerFavoriteView` with and without local/remote paths, `ConnectionHistoryEntry`, `SshKeyCandidate`, `SshKeyValidation`, tabs, and `PathResolution`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared models, persistence shape, migration, IPC types, and design tokens required by every user story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T004 [P] Create `WorkspaceLayout` and `HubTab` in `crates/thriftp-core/src/model/workspace.rs` with defaults `splitRatio: 0.5`, `sidebarVisible: true`, all sidebar sections `true`, and `hubTab: "favorites"`; derive serde camelCase serialization.
- [X] T005 [P] Create `ConnectionHistoryEntry` in `crates/thriftp-core/src/model/history.rs` with fields `favoriteId: UUID | null`, `connectionProfileId: UUID`, `displayName`, `protocol`, `host`, `port`, `username`, and `lastUsedAt`; derive serde camelCase serialization.
- [X] T006 Update `crates/thriftp-core/src/model/mod.rs` to declare and re-export `workspace` and `history` models.
- [X] T007 Update `crates/thriftp-core/src/repository.rs` so `AppState` includes `#[serde(default)] workspace_layout: WorkspaceLayout` and `#[serde(default)] connection_history: Vec<ConnectionHistoryEntry>`.
- [X] T008 Update `crates/thriftp-storage-json/src/lib.rs` to set `SCHEMA_VERSION = 2`, and update `crates/thriftp-storage-json/src/server_favorites.rs` so versions `0 -> 1 -> 2` and `1 -> 2` are accepted while unknown future versions still fail closed.
- [X] T009 [P] Add persistence tests in `crates/thriftp-storage-json/tests/schema_v2.rs` proving a version-0 document, a version-1 document without the new fields, and a version-2 document all load; version-1 data gains defaults; unknown future versions fail; and the atomic save still writes a backup.
- [X] T010 Update `apps/desktop/src-tauri/src/lib.rs` so `AppState` construction and `persist_metadata()` preserve `workspace_layout` and `connection_history` from the metadata store until the dedicated services are added.
- [X] T011 [P] Extend `apps/desktop/src/lib/ipc/types.ts` with `HubTab`, `SidebarSectionKey`, `WorkspaceLayout`, `ConnectionHistoryEntry`, `SshKeyCandidate`, `SshKeyValidation`, `PathSource`, `PathResolution`, and `localPath`/`remotePath` on the existing server-favorite input/view types.
- [X] T012 [P] Extend `apps/desktop/src/lib/ipc/client.ts` with typed wrappers for `workspaceGet`, `workspaceSet`, `connectionHistoryList`, `connectionHistoryClear`, `sshKeysList`, `sshKeysValidate`, and `browserResolvePath`, following the existing `call<T>` and `IpcError` pattern.
- [X] T013 Update `apps/desktop/src/app.css` with the shared design tokens from `research.md` Decision 10: semantic surface, text, border, accent, danger, warning, success, focus-ring, spacing, radius, and elevation tokens; light/dark values; `:focus-visible` treatment; and `prefers-reduced-motion` rules.

**Checkpoint**: Core models, persistence, IPC types, and visual tokens are ready. User stories can now proceed.

---

## Phase 3: User Story 1 - A clean, always-dual-pane workspace (Priority: P1) 🎯 MVP

**Goal**: The workspace always shows the local file system on the left and the right-side destination or placeholder on the right, with a collapsible far-left sidebar, a resizable split, remembered layout, shared styling, themes, and keyboard-accessible controls.

**Independent Test**: Launch with and without a connection, resize the divider and sidebar, switch themes, restart, and confirm both panes remain usable and the split, sidebar state, and hub tab are restored.

### Tests for User Story 1

- [X] T014 [P] [US1] Add Rust unit tests in `crates/thriftp-core/src/workspace.rs` for default layout, clamping `splitRatio` to `0.20..=0.80`, filling missing sidebar section keys with `true`, ignoring unknown keys, and falling back to `favorites` for an unknown hub tab.
- [X] T015 [P] [US1] Add Tauri contract tests in `apps/desktop/src-tauri/tests/workspace_contract.rs` for `workspace_get` defaults, `workspace_set` normalization, camelCase payloads, and persistence through the metadata store.
- [X] T016 [P] [US1] Add Vitest tests in `apps/desktop/src/lib/workspace/layout.test.ts` for ratio clamping, effective sidebar visibility at narrow widths without overwriting `sidebarVisible`, keyboard split steps, and hub-tab normalization.
- [X] T017 [P] [US1] Add Vitest component tests in `apps/desktop/src/components/browser/dual-pane.test.ts` proving the local pane is always on the left, the right pane is always rendered, the split separator exposes `aria-valuenow`, and Arrow/Home/End/Enter change or reset the ratio.

### Implementation for User Story 1

- [X] T018 [US1] Implement `WorkspaceService` in `crates/thriftp-core/src/workspace.rs` with read normalization and save validation for `splitRatio` (`0.20..=0.80`), section defaults, and `hubTab`.
- [X] T019 [US1] Declare `pub mod workspace;` in `crates/thriftp-core/src/lib.rs`.
- [X] T020 [US1] Create `workspace_get` and `workspace_set` Tauri commands in `apps/desktop/src-tauri/src/commands/workspace.rs`; return the normalized persisted layout and a typed `DesktopError` on failure.
- [X] T021 [US1] Register the workspace commands in `apps/desktop/src-tauri/src/lib.rs` and persist the layout atomically through `persist_metadata()`.
- [X] T022 [US1] Implement pure layout helpers in `apps/desktop/src/lib/workspace/layout.ts`: `clampSplit`, `effectiveSidebarVisible`, `applySplitKey`, and `normalizeHubTab`.
- [ ] T023 [US1] Extract the local/remote pane markup from `apps/desktop/src/components/browser/DualPane.svelte` into `apps/desktop/src/components/browser/FilePane.svelte`, preserving drag/drop, keyboard, listing, toolbar, selection, and error behavior.
- [X] T024 [US1] Rebuild `apps/desktop/src/components/browser/DualPane.svelte` as a CSS-grid split with 320 px pane minimums, a focusable `role="separator"`, pointer drag, Arrow/Home/End/Enter keyboard control, a visible reset control, and commit-on-release.
- [X] T025 [US1] Create `apps/desktop/src/components/workspace/WorkspaceSidebar.svelte` as a docked, collapsible shell with a labeled collapse control, a labeled reopen control, and section placeholders for Saved Servers, Directory Favorites, and Automation.
- [X] T026 [US1] Rework `apps/desktop/src/App.svelte` to load `workspaceGet` on mount, render the sidebar before the local pane, render `DualPane` with the local left pane and a right pane, and save committed layout changes with a short debounce and a non-blocking error notice.
- [X] T027 [US1] Add sidebar-first responsive behavior in `apps/desktop/src/App.svelte` and `apps/desktop/src/app.css`: at narrow widths the effective sidebar collapses before either pane drops below 320 px, and the stored `sidebarVisible` preference is preserved.
- [ ] T028 [US1] Update `apps/desktop/src/components/browser/browser.test.ts` and related imports for the `FilePane` extraction, and add assertions that focus is visible and the shell uses theme tokens rather than hard-coded colors.

**Checkpoint**: US1 is independently usable and testable as the always-dual-pane shell.

---

## Phase 4: User Story 2 - Connect or reconnect from the right-pane hub (Priority: P1)

**Goal**: When disconnected, the right pane is a Favorites/History hub. Successful connects replace it with the remote listing; disconnecting returns to the remembered tab; history is local, bounded, deduplicated, and clearable.

**Independent Test**: Start with no connection, browse Favorites and History, connect to a saved server, disconnect, and confirm the hub returns with the previously selected tab; verify failed connects are not recorded and history clearing is safe.

### Tests for User Story 2

- [X] T029 [P] [US2] Add Rust unit tests in `crates/thriftp-core/src/history.rs` for successful-only recording, deduplication by `favoriteId` (or `connectionProfileId` when no favorite), newest-first ordering, the 50-entry cap, confirmed clear, and entries surviving favorite deletion.
- [X] T030 [P] [US2] Add Tauri contract tests in `apps/desktop/src-tauri/tests/history_contract.rs` proving successful `connections_connect`/`favorites_connect` record history, failed connects do not, `connection_history_clear` requires `confirmed: true`, and no secret appears in any entry.
- [X] T031 [P] [US2] Add Vitest tests in `apps/desktop/src/components/workspace/connection-hub.test.ts` for Favorites/History tabs, empty states, last-tab restoration, newest-first history rows, connect actions, clear confirmation, and transfer-visibility compatibility.

### Implementation for User Story 2

- [X] T032 [US2] Implement `ConnectionHistoryService` in `crates/thriftp-core/src/history.rs`: `record_success`, `list`, and `clear(confirmed)` with one entry per saved server, newest-first order, and a cap of 50.
- [X] T033 [US2] Declare `pub mod history;` in `crates/thriftp-core/src/lib.rs`.
- [X] T034 [US2] Record successful connections in `apps/desktop/src-tauri/src/commands/connections.rs` and `apps/desktop/src-tauri/src/commands/server_favorites.rs`, persist the updated metadata after each successful record, and record nothing on any failure path.
- [X] T035 [US2] Create `connection_history_list` and `connection_history_clear` commands in `apps/desktop/src-tauri/src/commands/history.rs`, register them in `apps/desktop/src-tauri/src/lib.rs`, and persist after a successful record or clear.
- [X] T036 [US2] Create `apps/desktop/src/lib/workspace/history.ts` with a typed store and load/refresh/clear helpers for the History tab.
- [X] T037 [US2] Implement `apps/desktop/src/components/workspace/ConnectionHub.svelte` with Favorites and History tabs, remembered `hubTab`, searchable favorites, duplicate-disambiguating detail, stored remote path display, and accessible empty states.
- [X] T038 [US2] Implement `apps/desktop/src/components/workspace/HistoryList.svelte` with newest-first rows, display name, protocol, host, last-used time, connect action, and a confirmed clear-history action.
- [X] T039 [US2] Integrate the hub into `apps/desktop/src/App.svelte`: show it in the right pane when `selectedConnection` is null, replace it with the remote listing on success, return to it on disconnect, refresh history after successful connects, and persist the selected tab.
- [X] T040 [US2] Ensure `QueueManager`/transfer progress remains visible and controllable while the hub is shown in `apps/desktop/src/App.svelte`, and add hub/history styles to `apps/desktop/src/components/workspace/ConnectionHub.svelte`.

**Checkpoint**: US2 works independently and completes the disconnected workspace surface.

---

## Phase 5: User Story 3 - Find anything with the command palette (Priority: P1)

**Goal**: Cmd/Ctrl+K opens a keyboard-first palette that searches saved servers, directory favorites, open tabs, and safe actions, grouped and contextual, without exposing destructive or queue controls.

**Independent Test**: Open the palette with no connection and with several saved servers and tabs, search by partial name, run a connection, switch a tab, open Preferences, and dismiss with Escape while confirming focus returns.

### Tests for User Story 3

- [X] T041 [P] [US3] Add Vitest tests in `apps/desktop/src/lib/commands/registry.test.ts` for category grouping, case-insensitive matching on label/host/username/protocol/group/path, disabled actions with reasons, excluded destructive/queue actions, empty-query suggestions, and deterministic ordering.
- [X] T042 [P] [US3] Add Vitest tests in `apps/desktop/src/components/workspace/command-palette.test.ts` for focus in the search field, ArrowUp/ArrowDown/Enter, Escape with focus restoration, no-results empty state, grouped rendering, and disabled-result announcements.

### Implementation for User Story 3

- [X] T043 [US3] Implement the pure palette registry in `apps/desktop/src/lib/commands/registry.ts`: `buildEntries`, `searchEntries`, `recentOrSuggested`, and action descriptors for connect, navigate favorite, activate tab, new connection, new favorite, Preferences, refresh active pane, and toggle hidden files; exclude destructive and transfer-queue actions.
- [X] T044 [US3] Implement `apps/desktop/src/components/workspace/CommandPalette.svelte` with `role="dialog"`, `aria-modal="true"`, search field, grouped results, highlight, disabled reasons, empty state, and the existing focus-trap behavior from `apps/desktop/src/lib/accessibility/focus.ts`.
- [X] T045 [US3] Wire the global `Cmd+K`/`Ctrl+K` shortcut and palette action handlers in `apps/desktop/src/App.svelte`, including the guard that prevents stacking the palette over an open modal.
- [X] T046 [US3] Preserve and restore focus to the previously focused control when the palette closes, and verify unavailable-action behavior when the last connection closes while the palette is open, in `apps/desktop/src/components/workspace/CommandPalette.svelte` and `apps/desktop/src/App.svelte`.
- [X] T047 [US3] Add palette styling to `apps/desktop/src/components/workspace/CommandPalette.svelte` using shared tokens, including selected, disabled, loading, and empty states, and connect the empty-query suggestions to recent history entries.

**Checkpoint**: US3 works independently on top of the existing stores and the US1 shell.

---

## Phase 6: User Story 4 - Save and restore local and remote paths in a favorite (Priority: P1)

**Goal**: A server favorite stores optional absolute local and remote folders; saving from a session prefills both; reconnecting restores both with nearest-ancestor/home/login fallback and one actionable notice.

**Independent Test**: From an active session in non-default folders, save a favorite, verify both paths are prefilled, disconnect, reconnect, and confirm both panes return to the saved folders; repeat with stale paths and confirm fallback without blocking the connection.

### Tests for User Story 4

- [X] T048 [P] [US4] Add Rust tests in `crates/thriftp-core/src/validation.rs` and `crates/thriftp-core/src/navigation.rs` for optional blank paths normalizing to `null`, relative paths rejected with the correct field name, both paths persisting through favorite create/update, and ancestor/home/login fallback behavior.
- [X] T049 [P] [US4] Add Tauri contract tests in `apps/desktop/src-tauri/tests/favorite_paths_contract.rs` for `localPath`/`remotePath` in favorite create/update/view payloads, blank-to-null normalization, absolute-path validation, and `browser_resolve_path` for local and remote sources.
- [X] T050 [P] [US4] Add Vitest tests in `apps/desktop/src/components/server-favorites/favorite-paths.test.ts` for prefilled paths, editable/clearable path fields, saved remote path visibility in lists, and at most one fallback notice per pane.

### Implementation for User Story 4

- [X] T051 [US4] Extend `crates/thriftp-core/src/model/server_favorite.rs` with `#[serde(default)] local_path: Option<String>` and `#[serde(default)] remote_path: Option<String>`.
- [X] T052 [US4] Extend `ServerFavoriteInput` and `ServerFavoriteView` in `crates/thriftp-core/src/server_favorites/mod.rs`, including `into_profile`/view mapping, and preserve existing clients that omit the fields.
- [X] T053 [US4] Add favorite-path validation in `crates/thriftp-core/src/validation.rs`: blank values become `null`; non-blank values MUST be absolute; relative values return `validation` with `field: "localPath"` or `"remotePath"`; no expansion of `~`, environment variables, or remote shell syntax.
- [X] T054 [US4] Implement `crates/thriftp-core/src/navigation.rs` with a shared nearest-ancestor walker and local-path resolution (existing readable directory, then home) plus a predicate-based remote resolver.
- [X] T055 [US4] Declare `pub mod navigation;` in `crates/thriftp-core/src/lib.rs`.
- [X] T056 [US4] Implement `browser_resolve_path` in `apps/desktop/src-tauri/src/commands/browser.rs`, register it in `apps/desktop/src-tauri/src/lib.rs`, and resolve local paths directly or remote paths through the active session; return `PathResolution` with `fallback: "none" | "ancestor" | "home" | "login"` and at most one redacted notice.
- [X] T057 [US4] Add optional Local path and Remote path fields to `apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte` and `apps/desktop/src/components/server-favorites/EditFavoriteDialog.svelte`, with blank allowed and absolute-path validation messages.
- [X] T058 [US4] Show the stored remote path in favorite rows in `apps/desktop/src/components/server-favorites/FavoritesList.svelte` and in the hub's Favorites tab in `apps/desktop/src/components/workspace/ConnectionHub.svelte`.
- [X] T059 [US4] Update `apps/desktop/src/lib/server-favorites/api.ts` `prefillFavorite` to accept the active tab's local and remote paths and prefill `localPath`/`remotePath`.
- [X] T060 [US4] Wire the favorite connect flow in `apps/desktop/src/App.svelte` to call `browserResolvePath` for each saved path after a successful connect, open each pane at the resolved path, and show at most one non-blocking fallback notice per pane.

**Checkpoint**: US4 restores both paths and handles stale paths without breaking the connection.

---

## Phase 7: User Story 5 - Discover and browse SSH private keys for SFTP (Priority: P1)

**Goal**: SFTP key authentication defaults to the platform SSH directory, offers conventional private-key candidates, prefills only when exactly one exists, and provides a key button that opens the native chooser there while preserving manual entry and secret safety.

**Independent Test**: Configure SFTP on machines with zero, one, and multiple keys in the platform SSH directory; confirm suggestions, exactly-one prefill, browse behavior, manual entry, validation messages, and that no key contents appear in the interface or logs.

### Tests for User Story 5

- [X] T061 [P] [US5] Add Rust tests in `crates/thriftp-core/src/ssh_keys.rs` using injected temporary homes for macOS/Linux (`~/.ssh`) and Windows (`%USERPROFILE%\.ssh`) behavior, conventional-name allowlisting, exclusion of `.pub`/`known_hosts`/`config`/`authorized_keys`/sockets, missing/unreadable directories, and exactly-one prefill.
- [X] T062 [P] [US5] Add Tauri contract tests in `apps/desktop/src-tauri/tests/ssh_keys_contract.rs` for `ssh_keys_list` directory/existence/readability/candidate payloads and `ssh_keys_validate` results for missing, directory, unreadable, public-key, and valid paths without key contents.
- [X] T063 [P] [US5] Add Vitest tests in `apps/desktop/src/components/connections/ssh-key-field.test.ts` for suggestions by file name, exactly-one prefill, multiple-key no-silent-selection, browse-button default path, manual entry fallback, and validation messages.

### Implementation for User Story 5

- [X] T064 [US5] Create `crates/thriftp-core/src/model/ssh_key.rs` with `SshKeyCandidate` and implement `crates/thriftp-core/src/ssh_keys.rs` with platform directory resolution, conventional filename allowlisting, directory metadata checks, and validation codes `missing`, `not_a_file`, `unreadable`, `public_key`, and `unsupported`; never read key contents.
- [X] T065 [US5] Declare and re-export the SSH key model and module in `crates/thriftp-core/src/lib.rs` and `crates/thriftp-core/src/model/mod.rs`.
- [X] T066 [US5] Create `ssh_keys_list` and `ssh_keys_validate` commands in `apps/desktop/src-tauri/src/commands/ssh_keys.rs` and register them in `apps/desktop/src-tauri/src/lib.rs`.
- [X] T067 [US5] Implement `apps/desktop/src/components/connections/SshKeyField.svelte` with the key icon button (🔑), native `open()` from `@tauri-apps/plugin-dialog` defaulting to the platform SSH directory, candidate suggestions by file name, exactly-one prefill, manual entry, and inline validation.
- [X] T068 [US5] Replace the plain SSH key input in `apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte` and `apps/desktop/src/components/connections/ConnectionEditor.svelte` with `SshKeyField.svelte`.
- [X] T069 [US5] Verify and enforce that no key contents or passphrases appear in UI text, logs, history, errors, or `metadata.json`; add regression assertions in `apps/desktop/src/components/connections/ssh-key-field.test.ts` and `apps/desktop/src-tauri/tests/ssh_keys_contract.rs`.

**Checkpoint**: US5 works independently and never authenticates or exposes key material during discovery.

---

## Phase 8: User Story 6 - Navigate with breadcrumbs in both panes (Priority: P2)

**Goal**: Both panes show clickable breadcrumbs from root to current folder, support an editable path mode with Enter/Escape, collapse long middle segments, expose full paths to assistive technology, and hide the bar for the disconnected hub.

**Independent Test**: Navigate deep into both trees, click ancestor segments, switch to editable mode, submit valid and invalid paths, resize until segments overflow, and confirm correct navigation, messages, and accessibility.

### Tests for User Story 6

- [X] T070 [P] [US6] Add Vitest tests in `apps/desktop/src/lib/browser/breadcrumbs.test.ts` for POSIX roots, Windows-style prefixes, trailing slashes, escaped/space-containing names, segment labels, and overflow segmentation.
- [X] T071 [P] [US6] Add Vitest tests in `apps/desktop/src/components/browser/breadcrumb-bar.test.ts` for segment activation, current-folder edit mode, Enter/Escape behavior, invalid-path callback, overflow menu reachability, `aria-current`, and full-path labels.

### Implementation for User Story 6

- [X] T072 [US6] Implement `apps/desktop/src/lib/browser/breadcrumbs.ts` with `segmentsForPath`, `navigationTarget`, and `overflowSegments`; support POSIX paths and Windows-style local prefixes while treating remote paths as server-side POSIX paths.
- [X] T073 [US6] Implement `apps/desktop/src/components/browser/BreadcrumbBar.svelte` with ordered segments, current-folder edit mode, Enter/Escape handling, overflow control, `aria-current`, `aria-label`/`title` full paths, and a hidden state when there is no location.
- [X] T074 [US6] Replace the always-visible path input in `apps/desktop/src/components/browser/BrowserToolbar.svelte` with `BreadcrumbBar.svelte`, keeping back/forward/up/refresh/sort/filter controls and wiring navigation through the existing pane `navigate` function.
- [X] T075 [US6] Hide or disable breadcrumbs for the disconnected hub and raise one actionable notice when a submitted path is invalid or inaccessible, leaving the pane at its current folder, in `apps/desktop/src/components/browser/FilePane.svelte` and `apps/desktop/src/components/browser/BreadcrumbBar.svelte`.
- [X] T076 [US6] Add `Ctrl/Cmd+L` to enter breadcrumb edit mode, ensure focus moves into the editable field, and return focus to the breadcrumb display on Escape.

**Checkpoint**: US6 completes navigation cleanup without changing listing behavior.

---

## Phase 9: User Story 7 - Use one searchable sidebar for saved servers, directory favorites, and automation (Priority: P2)

**Goal**: One far-left collapsible sidebar presents Saved Servers, Directory Favorites, and Automation; one search filters servers and favorites by name, host, protocol, group, or path; section state and visibility persist; narrow windows collapse it first.

**Independent Test**: Populate servers and directory favorites, search from the sidebar, collapse and expand sections, open automation, restart, and confirm state and filtering behavior; narrow the window and confirm the sidebar collapses before the panes.

### Tests for User Story 7

- [X] T077 [P] [US7] Add Vitest tests in `apps/desktop/src/lib/workspace/sidebar.test.ts` for filtering by name, host, protocol, group, and path; one-action search clearing; section state persistence; and narrow-width effective collapse.
- [X] T078 [P] [US7] Add Vitest tests in `apps/desktop/src/components/workspace/workspace-sidebar.test.ts` for three labeled sections, `aria-expanded`, collapse/expand, empty states, automation access, saved-server activation, directory-favorite activation, and the reopen control.

### Implementation for User Story 7

- [X] T079 [US7] Extend `apps/desktop/src/components/workspace/WorkspaceSidebar.svelte` with the single search field, Saved Servers, Directory Favorites, and Automation sections, remembered `sidebarSections`, empty states, one-action clear, and accessible collapse controls.
- [X] T080 [US7] Move automation status and controls behind the sidebar's Automation section in `apps/desktop/src/App.svelte`, removing the permanent `automation-surface` workspace strip while keeping `AutomationPanel` available on demand.
- [X] T081 [US7] Replace the old connected-only `apps/desktop/src/components/favorites/FavoritesSidebar.svelte` usage and the separate server-favorites panel in `apps/desktop/src/App.svelte` with `WorkspaceSidebar.svelte`, wiring the same connect, navigate, edit, delete, pin, and group behavior as the hub.
- [X] T082 [US7] Implement narrow-window overlay behavior for the reopened sidebar so neither pane drops below 320 px, without overwriting the persisted `sidebarVisible` preference.
- [X] T083 [US7] Remove obsolete imports and components from `apps/desktop/src/App.svelte` and `apps/desktop/src/components/favorites/`, and update affected tests so duplicate navigation surfaces do not remain.
- [X] T084 [US7] Verify that sidebar entries share the same confirmation and security rules as their hub/editor equivalents, including delete confirmation and credential handling, in `apps/desktop/src/components/workspace/WorkspaceSidebar.svelte` and `apps/desktop/src/App.svelte`.

**Checkpoint**: US7 delivers the single navigation surface and completes the primary feature scope.

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, performance, accessibility, privacy, and full validation across all stories.

- [X] T085 [P] Update `docs/desktop.md`, `docs/architecture.md`, `docs/server-favorites.md`, and `docs/testing.md` with the refreshed workspace, hub, breadcrumbs, palette, layout/history persistence, favorite paths, SSH key discovery, and new test commands.
- [X] T086 [P] Add performance tests in `apps/desktop/src/lib/commands/registry.test.ts` and `apps/desktop/src/lib/workspace/sidebar.test.ts` proving palette and sidebar filtering stay within 100 ms for 200 saved servers and directory favorites; keep layout drag work presentation-only.
- [X] T087 [P] Audit keyboard reachability, visible focus, `aria-expanded`/`aria-current`/`aria-valuenow`, focus trapping, focus restoration, reduced-motion behavior, and light/dark contrast across all refreshed surfaces in `apps/desktop/src/app.css` and the workspace/browser component tests; fix gaps and add regression assertions.
- [X] T088 [P] Audit `metadata.json`, logs, history entries, error payloads, and rendered text for credentials, passphrases, key contents, and file data; add regression coverage in `crates/thriftp-tests/tests/security/test_workspace_privacy.rs` if any gap exists.
- [X] T089 [P] Remove remaining hard-coded theme-dependent colors from refreshed components and confirm every refreshed surface uses the shared tokens from `apps/desktop/src/app.css`.
- [X] T090 Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace --locked` from the repository root (`Cargo.toml`); resolve all failures.
- [X] T091 Run `npm run lint`, `npm run format`, `npm test`, and `npm run build` in `apps/desktop` (`apps/desktop/package.json`); resolve all failures.
- [ ] T092 Execute every scenario in `specs/004-workspace-ui-refresh/quickstart.md` on macOS, Linux, and Windows where available, including the native file chooser and 900 × 600 layout checks; record any platform-specific deviations.
- [X] T093 [P] Verify version-1 metadata loads without data loss, CLI/MCP commands and payloads are unchanged, and no new telemetry, cloud call, account, or secret store was introduced, using `crates/thriftp-storage-json/tests/schema_v2.rs` and `crates/thriftp-tests/tests/cli/test_automation_smoke.rs`.
- [X] T094 [P] Review `specs/004-workspace-ui-refresh/plan.md`, contracts, and data model against the implementation and update documentation where behavior differs.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; can start immediately.
- **Foundational (Phase 2)**: Depends on Setup; BLOCKS all user stories.
- **User Stories (Phases 3–9)**: Depend on Foundational completion.
- **Polish (Phase 10)**: Depends on all desired user stories.

### User Story Dependencies

- **US1 (P1)**: No dependency after Foundational. Provides the shell, split, and layout persistence used by later stories.
- **US2 (P1)**: Depends on US1 for the right-pane shell and on Foundational history model/persistence. Can be built in parallel with US3–US7 at the service level, but its `App.svelte` integration should follow US1.
- **US3 (P1)**: Depends on US1 for the global shortcut and shell. Uses existing stores plus US2 history suggestions; the registry and component can be developed before the US2 integration.
- **US4 (P1)**: Depends on US1 (panes/notices) and US2 (favorite rows/hub display) for the full flow; core path validation and resolution can proceed independently after Foundational.
- **US5 (P1)**: Independent after Foundational except that its dialog integration touches the same favorite form as US4. Implement US5 after US4 if one developer owns both, or coordinate the file edits.
- **US6 (P2)**: Depends on US1's `FilePane` extraction. Independent of US2–US5 at the component level.
- **US7 (P2)**: Depends on US1's sidebar shell and US2's hub behavior for shared connect/navigate semantics. Replaces the old sidebar surface last.

### Within Each User Story

1. Tests MUST be written and fail before implementation.
2. Models and pure helpers precede services and components.
3. Services precede Tauri commands.
4. Commands precede `App.svelte` integration.
5. Integration precedes story-level manual validation.
6. Complete and validate one story before moving to the next priority for a single developer.

### Recommended Delivery Order

`Setup → Foundational → US1 → US2 → US3 → US4 → US5 → US6 → US7 → Polish`

For parallel staffing after Foundational:

- Developer A: US1, then US6
- Developer B: US2, then US3
- Developer C: US4, then US5
- Developer D: US7 after US1/US2 are available

Avoid parallel edits to `apps/desktop/src/App.svelte`, `apps/desktop/src/app.css`, `apps/desktop/src/lib/ipc/client.ts`, `crates/thriftp-core/src/lib.rs`, `crates/thriftp-core/src/model/mod.rs`, or `apps/desktop/src-tauri/src/lib.rs` without coordinating first.

---

## Parallel Execution Examples

### User Story 1

```text
T014 Rust workspace model/normalization tests
T015 Tauri workspace contract tests
T016 Vitest layout helper tests
T017 Vitest dual-pane split tests
```

### User Story 2

```text
T029 Rust history service tests
T030 Tauri history/connection contract tests
T031 Vitest connection hub tests
```

### User Story 3

```text
T041 Vitest registry/action-availability tests
T042 Vitest palette keyboard/focus tests
```

### User Story 4

```text
T048 Rust path validation/navigation tests
T049 Tauri favorite-path and resolution contract tests
T050 Vitest favorite path form/list tests
```

### User Story 5

```text
T061 Rust SSH key discovery tests
T062 Tauri SSH key contract tests
T063 Vitest SSH key field tests
```

### User Story 6

```text
T070 Vitest breadcrumb helper tests
T071 Vitest breadcrumb component tests
```

### User Story 7

```text
T077 Vitest sidebar filtering/persistence tests
T078 Vitest sidebar component tests
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: User Story 1.
4. **STOP and VALIDATE**: launch with and without a connection, resize, switch themes, restart, and run the US1 tests.
5. Deploy/demo the always-dual-pane shell as the MVP increment.

### Recommended First Release Increment

US1 + US2. US1 alone proves the shell, but the disconnected right pane becomes genuinely useful only when the hub and history land. Treat US1 as the MVP checkpoint and US1 + US2 as the first user-facing release increment.

### Incremental Delivery

1. Setup + Foundational → shared models, persistence, IPC types, tokens ready.
2. US1 → always-dual-pane shell, split, layout persistence.
3. US2 → hub, history, reconnection surface.
4. US3 → command palette.
5. US4 → favorite local/remote path restoration.
6. US5 → SSH key discovery.
7. US6 → breadcrumbs.
8. US7 → unified sidebar and automation access.
9. Polish → documentation, performance, accessibility, privacy, full validation.

Each increment is independently testable and must not break earlier stories or the existing CLI/MCP/transfer behavior.

---

## Notes

- `[P]` marks tasks that touch different files and have no incomplete dependencies.
- Every user story phase includes its own tests, models/helpers, services/commands, UI, integration, and checkpoint.
- Rust tests may be inline `#[cfg(test)]` modules when they stay in the same file; Tauri contract tests use the existing `apps/desktop/src-tauri/tests/support` harness.
- Frontend tests use Vitest + Testing Library and the shared mocks from `apps/desktop/src/lib/test-utils/`.
- Commit after each task or logical group.
- Verify tests fail before implementing, then pass before moving on.
- Keep `thriftp-core` free of Tauri imports and keep protocol/transfer behavior untouched.
- Run `quickstart.md` before declaring the feature complete.
