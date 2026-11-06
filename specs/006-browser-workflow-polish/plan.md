# Implementation Plan: Browser and Favorites Workflow Polish

**Branch**: `006-browser-workflow-polish` | **Date**: 2026-11-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/006-browser-workflow-polish/spec.md`

## Summary

Polish the daily browsing and transfer workflow across six areas: reliable favorite creation/editing, standard desktop multi-selection and column sorting, linked local/remote navigation, per-connection queue statistics, safe detailed permission editing, and remote-pane control cleanup.

The change preserves the existing architecture boundary. `thriftp-core` owns sort-direction defaults, permission-bit normalization, symbolic rendering, and recursive permission traversal. `thriftp-storage-json` only carries the additive sort-direction field. Tauri commands remain thin adapters and expose recursive permission changes plus richer inspector metadata and transfer timestamps. Svelte components own selection, sorting interaction, dialog behavior, linked-pane navigation, queue statistics presentation, and the inspector UI. No new runtime dependency is required.

## Technical Context

**Language/Version**: Rust 1.99.1 (edition 2024, MSRV 1.90.0); TypeScript 6.0.3; Svelte 5.57.3; Tauri 2.12.x; Node 26.13.0

**Primary Dependencies**: Existing `tauri` 2, `@tauri-apps/api` 2.12, `@tauri-apps/plugin-dialog` 2.8, Svelte 5 runes, Vite 8, Vitest 5 with Testing Library; existing Rust crates `serde`, `serde_json`, `chrono`, `uuid`, `tracing`, `thiserror`; `thriftp-core`, `thriftp-storage-json`, `thriftp-credentials-keyring`. No new runtime dependency is planned. Clipboard access uses the platform webview clipboard API.

**Storage**: The existing versioned metadata document (`metadata.json`) through `JsonMetadataStore` remains the only desktop persistence. The additive `sortDirection` field is added to `ViewSettings` and `Preference` with a serde default of ascending, so existing documents load unchanged. Queue statistics and pane link state are runtime-only and reset with the connection session. Recursive permission results are returned to the UI and are not persisted. Credentials remain in the OS credential store.

**Testing**:
- Rust: `cargo test --workspace --locked`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`; core tests for permission-bit parsing/rendering, recursive traversal, symlink skipping, failure reporting, and sort-direction defaults.
- Tauri: command contract tests for `browser_chmod_recursive`, extended inspector metadata, transfer state timestamps, and unchanged favorite create/update payloads.
- Frontend: `cd apps/desktop && npm test`, `npm run lint`, `npm run format`, `npm run build`; Vitest tests for modifier selection, range selection, header sort toggles, favorite local-path browse, favorite dialog reachability, linked navigation, per-connection statistics, permission checkboxes, recursive confirmation, path copy, and control placement.
- Manual: `quickstart.md` scenarios on macOS, Linux, and Windows, including Cmd/Ctrl modifier behavior and the 900 × 600 minimum window.

**Target Platform**: macOS, Linux, and Windows desktop; supported minimum window 900 × 600 logical pixels

**Project Type**: Desktop application (Tauri 2 shell + Rust core + Svelte/TypeScript UI)

**Performance Goals**:
- Selection and sort changes are local and must not re-fetch directory listings.
- Sorting a 10,000-entry listing must remain interactive; the existing virtualization and bounded listing stream are preserved.
- Queue statistics derive from the in-memory transfer list and update within one render after a completion event.
- Recursive permission traversal is asynchronous, yields to the runtime between directory operations, and does not block the UI thread.
- Favorite browsing and dialog resizing remain immediate; no network or disk work occurs on pointer movement.

**Constraints**:
- `thriftp-core` MUST remain Tauri-independent and MUST build and test without Tauri types.
- No protocol, credential-store, host-key, CLI, or MCP behavior changes.
- Permission editing remains remote-only; local permissions are read-only.
- Recursive permission changes require explicit confirmation, apply to all descendants, and never follow symbolic links outside the selected directory.
- Linked navigation mirrors every navigation action by the same relative movement and never blocks the originating pane.
- The feature must not reintroduce page-level scrolling; each pane's directory listing owns its scrollbar.
- No private-key material, credentials, or secrets are exposed by clipboard, logs, errors, or statistics.

**Scale/Scope**: Two panes, one queue panel, one inspector dialog, and favorite create/edit dialogs; 6 user stories; 54 functional requirements; existing 10,000-entry listing target; per-connection statistics for a small number of simultaneous connections; no new persisted statistics or link state.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Product Vision | Directly improves everyday browsing, selection, favorites, and transfer understanding without expanding into unrelated product scope | PASS |
| II. Architecture | Core owns permission and sort rules; Tauri commands stay thin; Svelte owns interaction and presentation; recursive traversal remains asynchronous and off the UI thread | PASS |
| III. Security | Permission changes require explicit confirmation; symlinks are not followed; local permission editing stays read-only; clipboard contains only a path, never a secret; credential and host-key rules are unchanged | PASS |
| IV. User Experience | Standard Cmd/Ctrl selection, visible sort state, linked navigation, reachable dialogs, fixed toolbars, and independent pane scrolling follow native desktop conventions | PASS |
| V. Quality | Core, persistence, command-contract, and component tests are defined for every changed behavior; Clippy, rustfmt, frontend lint, and production build remain gates | PASS |
| VI. Open-Source | No account, cloud service, telemetry, or new dependency; behavior remains local and documented | PASS |
| VII. AI-Assisted Development | Spec, clarification answers, and acceptance criteria remain authoritative; YAGNI applies to runtime-only state and helper modules | PASS |
| VIII. Automation & Integration | CLI and MCP interfaces and security behavior are explicitly out of scope and unchanged | PASS |

No gate violations. No complexity exceptions are required.

## Project Structure

### Documentation (this feature)

```text
specs/006-browser-workflow-polish/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 validation guide
├── contracts/
│   ├── desktop-ipc.md   # Tauri command and event contract
│   └── workspace-ui.md  # Selection, sorting, linked navigation, queue, and inspector UI contract
└── tasks.md             # Phase 2 output (/speckit.tasks, not created here)
```

### Source Code (repository root)

```text
crates/
├── thriftp-core/
│   ├── src/
│   │   ├── model/
│   │   │   ├── browsing.rs            # Add SortDirection and view-settings field
│   │   │   ├── preference.rs          # Add persisted default sort direction
│   │   │   └── transfer.rs            # Existing timestamps reused for statistics
│   │   ├── browser/
│   │   │   ├── metadata.rs            # Permission detail parsing/rendering
│   │   │   └── permissions.rs         # Recursive chmod with report and symlink policy
│   │   └── lib.rs                     # Re-export new permission/sort behavior
│   └── tests/                         # Permission and sort-default tests
└── thriftp-storage-json/
    └── src/
        ├── lib.rs                     # Additive defaults for existing metadata documents
        └── tests/                     # Round-trip and legacy-loading tests

apps/desktop/
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   │   ├── browser.rs             # Add browser_chmod_recursive
│   │   │   ├── inspector.rs           # Return permission detail
│   │   │   └── transfers.rs           # Include timestamps in transfer:state
│   │   ├── views.rs                   # Typed recursive-chmod result and event payloads
│   │   └── lib.rs                     # Register new command
│   └── tests/                         # Command contract tests
└── src/
    ├── App.svelte                     # Link state, disconnect placement, stats wiring
    ├── app.css                        # Existing design tokens and scroll containment
    ├── components/
    │   ├── browser/
    │   │   ├── DirectoryList.svelte   # Selection modifiers, sort headers, virtualization
    │   │   ├── DualPane.svelte        # Linked navigation, pane header disconnect, sort state
    │   │   └── BrowserToolbar.svelte  # Remove back/forward buttons
    │   ├── server-favorites/
    │   │   ├── SaveFavoriteDialog.svelte  # Local browse, constrained height
    │   │   └── EditFavoriteDialog.svelte  # Reachable actions in edit mode
    │   ├── queue/
    │   │   └── QueueManager.svelte    # Per-connection statistics
    │   └── inspector/
    │       └── Inspector.svelte       # Permission matrix, recursive apply, copy path
    └── lib/
        ├── browser/
        │   ├── view.ts                # Direction-aware sorting
        │   ├── selection.ts           # NEW: anchor/range/toggle selection helpers
        │   └── linked-navigation.ts   # NEW: relative-move calculation
        ├── favorites/path-browser.ts  # NEW: native local-folder browse helper
        ├── transfers/stats.ts         # NEW: per-session connection statistics
        └── ipc/                       # Extend types and client wrappers
```

**Structure Decision**: Keep the established inward dependency direction. Permission bit rules and recursive traversal live in `thriftp-core`; JSON persistence only gains additive defaulted fields. Tauri commands translate payloads and never implement protocol behavior. Svelte components and small frontend helpers own interaction state (selection anchors, sort toggles, link mirroring, statistics presentation). Runtime-only link and statistics state stays out of the metadata document.

## Phase 0: Research

All technical unknowns are resolved in [research.md](./research.md). Key decisions are:

- Use a per-pane selection anchor plus the visible sorted/filtered list for Shift ranges.
- Add `sortDirection` to existing view preferences with a default, while keeping sorting client-side.
- Fix favorite prefill by resolving the active tab's `.` paths to accessible absolute paths before validation; core creation with an absolute remote path already works.
- Use the existing native dialog plugin for local-path browsing.
- Constrain favorite dialogs with an internal scroll area and sticky action footer.
- Mirror all navigation actions with a relative up/down segment transition; resolve the counterpart before navigating and stay put on failure.
- Derive per-connection statistics from transfer timestamps and the active connection's `startedAt`; extend `transfer:state` with authoritative timestamps.
- Add permission detail and recursive chmod to the core, with an explicit confirmation flag and no symlink following.
- Move Disconnect to the remote pane header and remove back/forward buttons from both toolbars.

## Phase 1: Design

- [data-model.md](./data-model.md) defines sort state, selection state, favorite editor state, pane link state, connection statistics, permission detail, and recursive permission requests/results.
- [contracts/desktop-ipc.md](./contracts/desktop-ipc.md) defines the recursive permission command, extended inspector metadata, transfer timestamp event fields, and unchanged favorite create/update contracts.
- [contracts/workspace-ui.md](./contracts/workspace-ui.md) defines modifier selection, header sorting, linked navigation behavior, favorite dialog layout, queue statistics display, inspector controls, and remote-pane control placement.
- [quickstart.md](./quickstart.md) defines end-to-end validation for all six user stories.

## Post-Design Constitution Check

Re-evaluated after Phase 1 design:

- **Core independence**: Sort defaults, permission detail, and recursive permission traversal live in `thriftp-core`; Tauri commands map payloads and Svelte owns presentation.
- **Security**: Recursive permission changes require a confirmation flag, skip symlinks, and report per-item failures. Local permission changes remain unavailable. Clipboard writes only a path. No secret-bearing data enters statistics, events, or UI.
- **Privacy**: Statistics and link state are runtime-only; no telemetry or new storage is introduced.
- **UX and accessibility**: Column headers are real buttons with sort state exposed; modifier selection follows platform conventions; dialogs keep actions reachable; linked navigation never blocks the active pane; controls keep accessible names and focus order.
- **Quality**: Every new core helper, command payload, dialog behavior, and interaction path has a named automated test layer.
- **Performance**: Sorting and selection remain in-memory over the existing virtualized listing; recursive permission traversal is asynchronous and reports progress without changing transfer streaming or O(1) transfer memory behavior.

No gate violations. No complexity exceptions are required.

## Complexity Tracking

No constitution violations or complexity exceptions were identified. The frontend helpers are small, focused modules; the core additions are confined to permission handling and additive sort defaults.
