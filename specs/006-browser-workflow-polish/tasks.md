---
description: "Task list for Browser and Favorites Workflow Polish"
---

# Tasks: Browser and Favorites Workflow Polish

**Input**: Design documents from `/specs/006-browser-workflow-polish/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/

**Tests**: Included because the plan and constitution require automated coverage for every changed behavior.

**Organization**: Tasks are grouped by user story so each story can be implemented and validated independently.

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Confirm the baseline and add shared test fixtures.

- [X] T001 Run the baseline checks in `docs/testing.md` from the repository root and confirm the starting workspace, Rust tests, frontend tests, lint, and build are green.
- [X] T002 [P] Add shared fixtures for large listings, transfer timestamps, and permission modes in `apps/desktop/src/lib/test-utils/fixtures.ts`.
- [X] T003 [P] Add a fake remote permission tree with nested folders, files, symlinks, and failure injection in `crates/thriftp-core/tests/support/mod.rs`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared contracts and models that every affected story relies on.

**⚠️ CRITICAL**: Complete this phase before starting the user story phases.

- [X] T004 [P] Extend `apps/desktop/src/lib/ipc/types.ts` with `SortDirection`, `Preferences.sortDirection`, `PermissionDetail`, `RecursivePermissionResult`, and transfer-state timestamp fields.
- [X] T005 [P] Add the `chmodRecursive` wrapper in `apps/desktop/src/lib/ipc/client.ts`.
- [X] T006 [P] Add the `sortDirection` field with an ascending serde default to `crates/thriftp-core/src/model/browsing.rs` and `crates/thriftp-core/src/model/preference.rs`.
- [X] T007 [P] Extend command test support for recursive permission results and transfer event payloads in `apps/desktop/src-tauri/tests/support/mod.rs`.

**Checkpoint**: Shared contracts and fixtures are ready. User stories can begin.

---

## Phase 3: User Story 1 - Reliable favorite creation and editing (Priority: P1) 🎯 MVP

**Goal**: A remote path can be saved on a new favorite, the local path can be browsed, and dialog actions remain reachable.

**Independent Test**: Create a new favorite with a remote path, browse to a local folder, edit the favorite, and complete the flow at 900 × 600.

### Tests for User Story 1

- [X] T008 [P] [US1] Add a core regression test for favorite creation with an absolute remote path in `crates/thriftp-core/tests/server_favorites_create.rs`.
- [X] T009 [P] [US1] Add a prefill test proving `.` working paths are resolved or omitted in `apps/desktop/src/lib/server-favorites/api.test.ts`.
- [X] T010 [P] [US1] Add dialog tests for local-path browse, cancel preservation, validation messages, and reachable actions in `apps/desktop/src/components/server-favorites/save-favorite.test.ts`.
- [X] T011 [P] [US1] Add a Tauri contract test for `favorites_create` and `favorites_update` with `remotePath` in `apps/desktop/src-tauri/tests/server_favorites_create.rs`.

### Implementation for User Story 1

- [X] T012 [US1] Add `resolveFavoritePrefillPaths` using the existing path resolver in `apps/desktop/src/lib/server-favorites/api.ts`.
- [X] T013 [US1] Wire resolved local/remote prefill paths into the save-as-favorite flow in `apps/desktop/src/App.svelte`.
- [X] T014 [P] [US1] Add a native local-folder browse helper in `apps/desktop/src/lib/favorites/path-browser.ts`, starting at the current path when it is an existing directory and otherwise at the user's home directory.
- [X] T015 [US1] Add the Browse button and local-path wiring in `apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte`.
- [X] T016 [US1] Constrain the dialog body and keep actions outside the scroll area in `apps/desktop/src/components/server-favorites/SaveFavoriteDialog.svelte` and `apps/desktop/src/components/server-favorites/EditFavoriteDialog.svelte`.

**Checkpoint**: Favorites can be created with remote paths, local paths can be browsed, and dialog actions remain reachable.

---

## Phase 4: User Story 2 - Select and sort files efficiently (Priority: P1)

**Goal**: Shift and Cmd/Ctrl selection work like native desktop file managers, and column headers sort in both directions.

**Independent Test**: Select ranges and individual files in a large listing, then sort by Name, Size, and Modified in both directions and verify selections survive.

### Tests for User Story 2

- [X] T017 [P] [US2] Add selection helper tests for plain, Shift, Cmd/Ctrl, missing anchor, filtered ranges, and selection persistence across refresh and re-sort in `apps/desktop/src/lib/browser/selection.test.ts`.
- [X] T018 [P] [US2] Add direction-aware sorting tests in `apps/desktop/src/lib/browser/view.test.ts`.
- [X] T019 [P] [US2] Add column-header interaction, keyboard activation, `aria-sort`, and focus-indicator tests in `apps/desktop/src/components/browser/directory-list.test.ts`.
- [X] T020 [P] [US2] Add legacy metadata loading tests proving missing `sortDirection` defaults to ascending in `crates/thriftp-storage-json/tests/schema_v2.rs`.
- [X] T021 [P] [US2] Add a 10,000-entry sorting and selection performance test in `apps/desktop/src/lib/browser/view.test.ts`.

### Implementation for User Story 2

- [X] T022 [P] [US2] Implement the selection anchor, range, and toggle helpers in `apps/desktop/src/lib/browser/selection.ts`.
- [X] T023 [US2] Make sorting direction-aware in `apps/desktop/src/lib/browser/view.ts`.
- [X] T024 [US2] Add sort-direction defaults to new tabs, consume `Preferences.sortDirection`, and update tab view persistence with a regression test in `apps/desktop/src/lib/tabs/restore.ts`, `apps/desktop/src/lib/tabs/restore.test.ts`, and `apps/desktop/src/App.svelte`.
- [X] T025 [US2] Convert Name, Size, and Modified headers to sort buttons with visible indicators in `apps/desktop/src/components/browser/DirectoryList.svelte`.
- [X] T026 [US2] Add pane-level anchor and sort-direction state, range selection, and selected-path preservation across refresh and re-sort in `apps/desktop/src/components/browser/DualPane.svelte`.
- [X] T027 [US2] Keep the toolbar Sort control synchronized without resetting direction in `apps/desktop/src/components/browser/BrowserToolbar.svelte`.
- [X] T028 [US2] Propagate sort direction through active-tab view settings in `apps/desktop/src/App.svelte`.

**Checkpoint**: Native modifier selection and sortable headers work independently in each pane.

---

## Phase 5: User Story 3 - Link navigation between local and remote panes (Priority: P2)

**Goal**: Linking the panes mirrors every navigation action by relative movement where the counterpart exists.

**Independent Test**: Link the panes and exercise parent, child, breadcrumb, typed path, history, tab, and favorite navigation; verify mirroring and graceful no-ops.

### Tests for User Story 3

- [X] T029 [P] [US3] Add relative-transition tests for parent steps, child segments, unrelated paths, and no-op counterparts in `apps/desktop/src/lib/browser/linked-navigation.test.ts`.
- [X] T030 [P] [US3] Add linked-pane interaction tests for up, child, breadcrumb, history, tab, and favorite mirrors, plus no-immediate-sync when Link/Unlink is toggled and no bypass for disconnected or unauthorized counterparts, in `apps/desktop/src/components/browser/dual-pane.test.ts`.
- [X] T031 [P] [US3] Add accessibility tests for the Link/Unlink label and state in `apps/desktop/src/lib/accessibility/workspace-a11y.test.ts`.

### Implementation for User Story 3

- [X] T032 [P] [US3] Implement relative up/down segment calculation in `apps/desktop/src/lib/browser/linked-navigation.ts`.
- [X] T033 [US3] Add session-scoped link state and the Link/Unlink control to the remote session line in `apps/desktop/src/App.svelte` without changing either current path when the state changes.
- [X] T034 [US3] Pass linked state and mirror callbacks into `apps/desktop/src/components/browser/DualPane.svelte`.
- [X] T035 [US3] Implement counterpart probing and relative mirrored navigation for parent, child, breadcrumb, typed path, history, tab, and favorite actions in `apps/desktop/src/components/browser/DualPane.svelte`; saved paths are translated as relative moves, and the existing connection, permission, and security guards are preserved so a disconnected or unauthorized counterpart is left unchanged.
- [X] T036 [US3] Reset link state when the active session changes or disconnects in `apps/desktop/src/App.svelte`.

**Checkpoint**: Linked navigation mirrors relative moves without blocking the originating pane.

---

## Phase 6: User Story 4 - Understand transfer activity per connection (Priority: P2)

**Goal**: The queue shows completed-file count, completed volume, and average speed for each active connection session.

**Independent Test**: Complete, fail, and cancel transfers across two connections and verify each connection's isolated session statistics.

### Tests for User Story 4

- [X] T037 [P] [US4] Add statistics tests for completed-only counting, byte totals, average speed, multiple connections, and session resets in `apps/desktop/src/lib/transfers/stats.test.ts`.
- [X] T038 [P] [US4] Add a Tauri contract test for transfer-state timestamps and connection identity in `apps/desktop/src-tauri/tests/transfers_contract.rs`.
- [X] T039 [P] [US4] Add a test proving statistics update from the in-memory transfer list without an extra listing fetch in `apps/desktop/src/lib/transfers/stats.test.ts`.

### Implementation for User Story 4

- [X] T040 [US4] Implement per-session statistics derivation and byte formatting in `apps/desktop/src/lib/transfers/stats.ts`.
- [X] T041 [US4] Add `connectionProfileId`, `direction`, `startedAt`, and `completedAt` to the transfer-state payload in `apps/desktop/src-tauri/src/commands/transfers.rs`.
- [X] T042 [US4] Update the transfer event handler and pass active connections plus statistics into `apps/desktop/src/App.svelte`.
- [X] T043 [US4] Add the per-connection statistics block to `apps/desktop/src/components/queue/QueueManager.svelte`.
- [X] T044 [US4] Ensure statistics ignore transfers started before the current session and reset on reconnect in `apps/desktop/src/lib/transfers/stats.ts`.

**Checkpoint**: Each connection has accurate, isolated, session-scoped statistics.

---

## Phase 7: User Story 5 - Inspect and change file permissions safely (Priority: P3)

**Goal**: The inspector shows detailed permissions, supports remote edits and confirmed recursive apply, and copies paths.

**Independent Test**: Inspect files and directories, edit remote permissions, confirm/cancel recursive apply, verify local read-only behavior, and copy a path.

### Tests for User Story 5

- [X] T045 [P] [US5] Add permission-bit parsing, symbolic rendering, and metadata-detail tests in `crates/thriftp-core/src/browser/metadata.rs`.
- [X] T046 [P] [US5] Add recursive chmod tests for descendants, symlink skipping, partial failures, and unsupported servers in `crates/thriftp-core/tests/permissions_recursive.rs`, using the fixtures from `crates/thriftp-core/tests/support/mod.rs`.
- [X] T047 [P] [US5] Add a Tauri contract test for `browser_chmod_recursive` and the required `confirmed` flag in `apps/desktop/src-tauri/tests/browser_contract.rs`.
- [X] T048 [P] [US5] Add inspector tests for the permission matrix, keyboard toggling, focus trapping in recursive confirmation, local read-only state, and copy-path feedback in `apps/desktop/src/components/inspector/inspector.test.ts`.

### Implementation for User Story 5

- [X] T049 [P] [US5] Add permission-detail and bit utilities in `crates/thriftp-core/src/browser/metadata.rs`.
- [X] T050 [US5] Implement recursive remote chmod with changed/skipped/failed reporting in `crates/thriftp-core/src/browser/permissions.rs`.
- [X] T051 [US5] Add and register `browser_chmod_recursive` and its typed result in `apps/desktop/src-tauri/src/commands/browser.rs`, `apps/desktop/src-tauri/src/views.rs`, and `apps/desktop/src-tauri/src/lib.rs`.
- [X] T052 [US5] Return permission detail from `inspector_get` in `apps/desktop/src-tauri/src/commands/inspector.rs` and verify the existing frontend IPC type wiring.
- [X] T053 [US5] Implement the permission matrix, apply behavior, recursive confirmation, and local read-only display in `apps/desktop/src/components/inspector/Inspector.svelte`.
- [X] T054 [US5] Add copy-path with keyboard activation, visible focus, an inline “Path copied” confirmation lasting at least two seconds, and clipboard-failure feedback in `apps/desktop/src/components/inspector/Inspector.svelte`.

**Checkpoint**: Remote permissions are editable with safe recursive confirmation; local permissions remain read-only; paths copy correctly.

---

## Phase 8: User Story 6 - Keep remote session controls in the right place (Priority: P3)

**Goal**: Disconnect sits beside the remote title, and back/forward buttons are gone from both toolbars.

**Independent Test**: Connect, disconnect from the remote heading, and confirm both toolbars have no back/forward controls while remaining navigation still works.

### Tests for User Story 6

- [X] T055 [P] [US6] Add a DualPane test for the remote-header Disconnect control and callback in `apps/desktop/src/components/browser/dual-pane.test.ts`.
- [X] T056 [P] [US6] Add a BrowserToolbar test proving back/forward buttons are absent, remaining controls are keyboard operable, and focus order is intact in `apps/desktop/src/components/browser/browser-toolbar.test.ts`.

### Implementation for User Story 6

- [X] T057 [US6] Add the `ondisconnect` prop and remote-header Disconnect button in `apps/desktop/src/components/browser/DualPane.svelte`, and remove the duplicate from the session line in `apps/desktop/src/App.svelte`.
- [X] T058 [US6] Remove back/forward buttons and their unused handlers from `apps/desktop/src/components/browser/BrowserToolbar.svelte` and `apps/desktop/src/components/browser/DualPane.svelte`.

**Checkpoint**: Disconnect is in the remote header and the toolbars are simplified.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Documentation, validation, performance, and security/accessibility review.

- [X] T059 [P] Update `docs/desktop.md` and `CHANGELOG.md` with the new selection, sorting, linked navigation, statistics, permissions, and control behavior.
- [X] T060 Run the full Rust and frontend validation commands from `docs/testing.md`.
- [X] T061 Execute every scenario in `specs/006-browser-workflow-polish/quickstart.md` on macOS, Linux, and Windows where available.
- [X] T062 Verify 10,000-entry sorting/selection responsiveness and that no listing re-fetch occurs during local sorting.
- [X] T063 Review recursive permission confirmation, local read-only behavior, clipboard contents, and statistics for secret leakage and least privilege.
- [X] T064 [P] Add keyboard-only and focus tests for selection, sort headers, favorite browsing, permission controls, copy path, and remote controls in `apps/desktop/src/lib/accessibility/workspace-a11y.test.ts`.
- [X] T065 [P] Add an automated layout regression test for equal local/remote pane heights and scrolling confined to each directory listing in `apps/desktop/src/components/browser/dual-pane.test.ts`.
- [X] T066 Final spec/docs synchronization and mark the feature tasks complete.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies.
- **Foundational (Phase 2)**: Depends on Setup and blocks all user stories.
- **User Stories (Phases 3–8)**: Depend on Foundational. They are independently testable, but shared files create practical ordering constraints.
- **Polish (Phase 9)**: Depends on all selected stories.

### User Story Dependencies

- **US1 (P1)**: Independent after Foundational.
- **US2 (P1)**: Depends on T006 and the shared IPC types.
- **US3 (P2)**: Depends on the shared IPC types; shares `App.svelte` and `DualPane.svelte` with US2 and US6.
- **US4 (P2)**: Depends on the transfer event contract in `types.ts`; shares `App.svelte` and `QueueManager.svelte` with other stories.
- **US5 (P3)**: Depends on `types.ts`, `client.ts`, and the inspector contract; independent of the browser interaction stories.
- **US6 (P3)**: Depends on Foundational; shares `App.svelte` and `DualPane.svelte`, so sequence after US2 and US3 for conflict-free execution.

### Within Each User Story

- Write the listed tests first and confirm they fail before implementation.
- Implement shared/core helpers before component wiring.
- Complete and validate one story before moving to the next priority when working sequentially.

### Parallel Opportunities

- Setup tasks T002 and T003 can run in parallel.
- Foundational tasks T004–T007 can run in parallel, with T005/T007 following the type changes.
- US1 tests T008–T010 and T016 can run in parallel.
- US2 tests T017–T021 can run in parallel.
- US5 tests T045–T048 can run in parallel.
- After Foundational, US1 and US5 can proceed in parallel with the browser interaction work.

---

## Parallel Example: User Story 2

```text
Task: "Add selection helper tests in apps/desktop/src/lib/browser/selection.test.ts"
Task: "Add direction-aware sorting tests in apps/desktop/src/lib/browser/view.test.ts"
Task: "Add column-header interaction tests in apps/desktop/src/components/browser/directory-list.test.ts"
Task: "Add legacy metadata loading tests in crates/thriftp-storage-json/tests/schema_v2.rs"
```

---

## Parallel Example: User Story 5

```text
Task: "Add permission detail tests in crates/thriftp-core/src/browser/metadata.rs"
Task: "Add recursive chmod tests in crates/thriftp-core/src/browser/permissions.rs"
Task: "Add the recursive chmod command contract test in apps/desktop/src-tauri/tests/browser_contract.rs"
Task: "Add inspector UI tests in apps/desktop/src/components/inspector/inspector.test.ts"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1 and Phase 2.
2. Complete User Story 1.
3. Validate favorite creation with a remote path, local-folder browsing, and dialog reachability.
4. Stop and demo the MVP before starting the remaining stories.

### Incremental Delivery

1. US1: favorite creation/editing reliability.
2. US2: native selection and sortable headers.
3. US3: linked navigation.
4. US4: per-connection statistics.
5. US5: detailed permissions and path copy.
6. US6: remote control placement and toolbar cleanup.
7. Polish: documentation and full validation.

### Parallel Team Strategy

- Team completes Setup and Foundational together.
- After Foundational: one developer takes US1, one takes US2, one takes US5.
- Add US3 and US4 after the shared `App.svelte`/`DualPane.svelte` and transfer event changes settle.
- Finish with US6 and Polish to avoid repeated merges in the remote header and toolbar.

---

## Notes

- `[P]` means different files and no dependency on an incomplete task.
- `[US#]` maps every story task to the specification.
- Local permissions are read-only; recursive permission changes are remote-only and require confirmation.
- Queue statistics and link state are runtime-only and reset with the connection session.
- Sorting and selection never re-fetch the listing.
- Re-run the quickstart scenarios after any change to the shared `App.svelte`, `DualPane.svelte`, or IPC contracts.

---

## Validation Notes

- 2026-11-06: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked`, `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`, the Tauri shell tests, and the desktop `npm run lint`, `npm run format`, `npm test` (152 tests), and `npm run build` all passed.
- T061: the quickstart behaviors are covered by the automated core, Tauri, and frontend tests and were exercised on macOS during implementation. A manual GUI pass against live FTP/FTPS/SFTP servers on Linux and Windows remains environment-dependent and was not available in this workspace.

---

## Phase 10: Convergence

- [X] T067 CRITICAL — Document `SortDirection`, the new sort-direction fields, and any other feature-added public Rust item so the public API is fully documented, then re-run rustdoc, per Constitution V (contradicts).
- [X] T068 Ensure linked tab and favorite activation performs at most one mirror: do not mirror from both the `remotePath` and `localPath` effects when both paths change together; apply the saved tab/favorite paths from one designated origin and cover tab and favorite activation in `apps/desktop/src/components/browser/dual-pane.test.ts`, per FR-019 (partial).
- [X] T069 Include authoritative transferred bytes in the completion `transfer:state` payload or refresh the completed transfer so queue statistics use exact completed volume when the final progress event was throttled, with a regression test in `apps/desktop/src/lib/transfers/stats.test.ts`, per FR-025 (partial).
- [X] T070 Add Shift-range and Cmd/Ctrl toggle behavior to keyboard navigation in `apps/desktop/src/lib/keyboard/browser.ts`, wire modifiers through `apps/desktop/src/components/browser/DualPane.svelte`, and add keyboard-only selection tests, per SC-013 (partial).
- [X] T071 Apply the active tab's persisted sort key, sort direction, hidden-file, and filter settings when the active tab changes instead of reading them only at pane creation, and add a tab-switch regression test, per workspace-ui Column Sorting (partial).
- [X] T072 Probe that the mirrored counterpart directory is readable, not merely stat-able, before committing the mirrored navigation; leave the other pane unchanged when listing access would fail, with a linked-navigation test, per FR-021 (partial).
- [X] T073 Add a Tauri contract test for `inspector_get` permission detail, including the local `editable = false` reason and the remote capability mapping, per plan Testing (missing).
- [X] T074 Add the remaining interaction and accessibility coverage: linked breadcrumb and history mirroring, favorite dialog validation and reachable actions at the minimum height, and keyboard focus/labels for the new sort headers, link control, permission controls, and copy-path action, per SC-013 (missing).

---

## Phase 11: Convergence

- [X] T075 Return an unavailable `PermissionDetail` with a reason when a permission string is present but unparseable or out of range, instead of returning `None`, and update the inspector/core contract tests so the panel can explain the limitation without inventing bits, per data-model PermissionDetail (partial).

---

## Phase 12: Convergence

- [X] T076 Render unavailable permission detail without inventing values: when `permissionDetail.mode` or `symbolic` is null, hide the derived octal/symbolic summary and zero-filled permission matrix, show the supplied reason, and add an Inspector test for an unparseable permission response, per data-model PermissionDetail (partial).
