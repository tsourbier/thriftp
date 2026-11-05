# Implementation Plan: Workspace UI Refresh and Dual-Pane Connection Hub

**Branch**: `004-workspace-ui-refresh` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/004-workspace-ui-refresh/spec.md`

## Summary

Refresh the Thriftp desktop workspace around an always-visible dual-pane shell: local files on the left, remote files or a connection hub on the right, and one collapsible far-left sidebar for saved servers, directory favorites, and automation. Add Cmd/Ctrl+K command palette navigation, breadcrumb path bars with editable mode, a draggable and keyboard-adjustable pane split, connection history, favorite local/remote path restoration, and SFTP private-key discovery from the platform SSH directory.

The change is split along the existing architecture boundary. New domain models and rules (workspace layout, connection history, favorite path fields, path fallback, SSH key candidates) live in `thriftp-core`; the versioned JSON adapter persists non-secret state; Tauri commands stay thin adapters; Svelte 5 components own presentation, keyboard interaction, and focus behavior. No new runtime dependency is required beyond the existing `@tauri-apps/plugin-dialog`, and no secret, new telemetry, cloud service, or storage mechanism is introduced.

## Technical Context

**Language/Version**: Rust stable (edition 2024, Rust 1.88.0+); TypeScript 5.x; Svelte 5.57.x; Tauri 2.x; Node 22+

**Primary Dependencies**: Existing `tauri` 2, `@tauri-apps/api` 2.9, `@tauri-apps/plugin-dialog` 2.8, Svelte 5 runes, Vite 6, Vitest 2 with Testing Library; existing Rust crates `serde`, `serde_json`, `chrono`, `uuid`, `tracing`, `thiserror`; `thriftp-core`, `thriftp-storage-json`, `thriftp-credentials-keyring`. No new runtime dependency is planned.

**Storage**: Versioned metadata document (`metadata.json`) through `JsonMetadataStore`, written atomically with restrictive permissions, for workspace layout, connection history, and the new optional favorite paths. Passwords, private keys, and passphrases remain only in the OS credential store (macOS Keychain, Windows Credential Manager, Linux Secret Service). No `localStorage`, custom vault, or new secret store is introduced.

**Testing**:
- Rust: `cargo test --workspace --locked` and `cargo clippy --workspace --all-targets -- -D warnings`; core unit tests for layout validation, history deduplication/cap/clear, favorite path validation, path fallback, and SSH key discovery using temporary directories and injected home paths; JSON migration tests for the bumped schema; Tauri command contract tests for the new payloads and redaction.
- Frontend: `cd apps/desktop && npm test`, `npm run lint`, `npm run format`, `npm run build`; Vitest component tests for breadcrumbs, palette, hub tabs, split keyboard control, sidebar search/collapse, key suggestions, and favorite path prefill; existing keyboard and theme tests remain green.
- Manual: `quickstart.md` scenarios on macOS, Linux, and Windows; native file chooser checked where available, with manual path entry as the fallback.

**Target Platform**: macOS, Linux, and Windows desktop; supported minimum window 900 × 600 logical pixels

**Project Type**: Desktop application (Tauri 2 shell + Rust core + Svelte/TypeScript UI)

**Performance Goals**:
- Command palette and sidebar filtering update within 100 ms for 200 saved servers and directory favorites.
- Pane drag and layout changes remain smooth (target 60 fps, no network or disk work per pointer move).
- Directory navigation remains unchanged; listing virtualization and O(1) streaming transfers are not touched.
- Key discovery completes within 1 second for a normal SSH directory; the directory scan is bounded and never reads key material into memory.
- Workspace layout load and save are single small JSON reads/writes; saving is debounced and must not block browsing.

**Constraints**:
- `thriftp-core` MUST remain Tauri-independent and MUST build and test without Tauri types.
- No protocol behavior, transfer semantics, CLI behavior, or MCP behavior changes; existing security rules remain in force.
- No private-key contents or passphrases in the UI, logs, history, errors, or JSON metadata.
- Command palette excludes destructive actions and transfer-queue controls.
- All refreshed controls are keyboard reachable with visible focus; dialogs and the palette trap and restore focus.
- The refreshed surfaces use shared design tokens; no new hard-coded theme-dependent colors.
- No mandatory telemetry, cloud dependency, account, or subscription.

**Scale/Scope**: Two file panes plus one sidebar and one splitter; 7 user stories; 55 functional requirements; at least 200 saved servers and directory favorites; connection history capped at the 50 most recent servers with one entry per saved server; FTP, FTPS, and SFTP unchanged.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Product Vision | Improves navigation, reconnection, and daily transfer workflow without scope outside the stated vision | PASS |
| II. Architecture | Rust core retains domain rules; Tauri commands are thin; Svelte 5 owns presentation; async/network work stays off the UI thread; no new dependency without need | PASS |
| III. Security | Favorite paths are non-secret; history explicitly excludes credentials, key material, and file data; key discovery never reads or displays key contents; credential store and host-key rules unchanged | PASS |
| IV. User Experience | Always-visible local/remote panes, native dialog use, keyboard shortcuts, theme adaptation, reduced-motion support, and no proprietary assets | PASS |
| V. Quality | Core, persistence, command-contract, and component tests are defined; Clippy/rustfmt/frontend lint/build remain gates; errors stay explicit and secret-free | PASS |
| VI. Open-Source | MIT project; no account, cloud, or mandatory telemetry; documentation updated | PASS |
| VII. AI-Assisted Development | Spec and acceptance criteria remain authoritative; plan and tests precede implementation; YAGNI applied to components and state | PASS |
| VIII. Automation & Integration | Shared core models remain reusable; CLI/MCP behavior and output are explicitly out of scope and unchanged | PASS |

No gate violations. No complexity exceptions are required.

## Project Structure

### Documentation (this feature)

```text
specs/004-workspace-ui-refresh/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 validation guide
├── contracts/
│   ├── desktop-ipc.md   # Tauri command and event contract
│   └── workspace-ui.md  # UI behavior, keyboard, focus, and layout contract
└── tasks.md             # Phase 2 output (/speckit.tasks, not created here)
```

### Source Code (repository root)

```text
crates/
├── thriftp-core/
│   ├── src/
│   │   ├── model/
│   │   │   ├── server_favorite.rs     # Extend with optional localPath/remotePath
│   │   │   ├── workspace.rs           # NEW: WorkspaceLayout, HubTab, sidebar sections
│   │   │   ├── history.rs             # NEW: ConnectionHistoryEntry
│   │   │   ├── ssh_key.rs             # NEW: SshKeyCandidate
│   │   │   └── mod.rs                 # Re-export new models
│   │   ├── server_favorites/mod.rs    # Persist and project favorite paths
│   │   ├── workspace.rs               # NEW: layout validation and normalization
│   │   ├── history.rs                 # NEW: bounded, deduplicated history service
│   │   ├── ssh_keys.rs                # NEW: platform SSH directory and candidate discovery
│   │   ├── navigation.rs              # NEW: accessible-ancestor path resolution
│   │   ├── repository.rs              # Add layout/history to AppState
│   │   └── lib.rs                     # Register new modules
│   └── tests/                         # Core unit and migration-facing tests
├── thriftp-storage-json/
│   └── src/
│       ├── lib.rs                     # Schema version bump and additive loading
│       └── server_favorites.rs        # Ordered migration ladder
└── thriftp-credentials-keyring/       # Unchanged

apps/desktop/
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs                     # Register workspace/history/key/path commands; persist layout
│   │   ├── commands/
│   │   │   ├── workspace.rs           # NEW: workspace_get/set
│   │   │   ├── history.rs             # NEW: connection_history_list/clear
│   │   │   ├── ssh_keys.rs            # NEW: ssh_keys_list/validate
│   │   │   ├── browser.rs             # Add path resolution command
│   │   │   ├── connections.rs         # Record successful connections in history
│   │   │   └── server_favorites.rs    # Record favorite connects; return path fields
│   │   └── views.rs                   # Add typed IPC views
│   ├── tauri.conf.json                # minWidth/minHeight = 900 × 600
│   └── tests/                         # Command contract and persistence tests
└── src/
    ├── App.svelte                     # Workspace orchestration, shortcuts, modal stack
    ├── app.css                        # Shared design tokens, focus, motion, theme states
    ├── components/
    │   ├── workspace/                 # NEW: shell pieces
    │   │   ├── WorkspaceSidebar.svelte
    │   │   ├── ConnectionHub.svelte
    │   │   ├── HistoryList.svelte
    │   │   └── CommandPalette.svelte
    │   ├── browser/
    │   │   ├── DualPane.svelte        # Split layout and pane composition
    │   │   ├── FilePane.svelte        # NEW: extracted single pane
    │   │   ├── BreadcrumbBar.svelte   # NEW: breadcrumbs and editable mode
    │   │   └── BrowserToolbar.svelte  # Navigation/sort/filter only
    │   ├── connections/
    │   │   └── SshKeyField.svelte     # NEW: discovery, suggestions, key button
    │   └── server-favorites/          # Add local/remote path fields to dialogs and list
    └── lib/
        ├── browser/breadcrumbs.ts     # NEW: path segmentation, collapse, validation helpers
        ├── commands/registry.ts       # NEW: palette categories, availability, search
        ├── workspace/layout.ts        # NEW: split/sidebar/hub state helpers
        ├── ssh-keys/api.ts            # NEW: typed IPC wrappers
        └── ipc/                       # Extend types and client
```

**Structure Decision**: Keep the established inward dependency direction. `thriftp-core` owns new models, validation, history bounds, layout normalization, path fallback rules, and key-candidate policy so these can be tested without Tauri and reused by future interfaces. `thriftp-storage-json` owns the additive schema migration. `apps/desktop/src-tauri` maps IPC payloads and performs no protocol logic. `apps/desktop/src` gains focused workspace components rather than growing `App.svelte`; the existing directory-favorites and server-favorites namespaces remain distinct, as established by `003-server-favorites`.

## Phase 0: Research

All technical unknowns are resolved in [research.md](./research.md). Key decisions are: persist workspace and history in the existing versioned metadata document; implement the split as a CSS-grid ratio with pointer and keyboard controls; keep palette and sidebar filtering entirely client-side; discover SSH keys by allowlisted filenames without reading key contents; resolve stale favorite paths through a core ancestor resolver; and derive palette entries from existing stores rather than introducing a second navigation model.

## Phase 1: Design

- [data-model.md](./data-model.md) defines the extended favorite, workspace layout, connection history entry, discovered SSH key, palette entry, and navigation location, including validation and state transitions.
- [contracts/desktop-ipc.md](./contracts/desktop-ipc.md) defines new and changed Tauri commands, payloads, events, errors, and backwards-compatibility rules.
- [contracts/workspace-ui.md](./contracts/workspace-ui.md) defines the shell, splitter, sidebar, hub, breadcrumbs, palette, keyboard map, focus behavior, and empty/loading/error states.
- [quickstart.md](./quickstart.md) defines runnable end-to-end validation scenarios for all seven user stories.

## Post-Design Constitution Check

Re-evaluated after Phase 1 design:

- **Core independence**: New state, history, path-fallback, layout, and key-discovery rules live in `thriftp-core`; Tauri commands translate payloads only. The core still builds without Tauri.
- **Security**: History is a non-secret projection and is explicitly clearable; SSH discovery returns file names and paths only and never reads or logs key contents; credentials remain in the OS store and existing verification paths are untouched.
- **Privacy**: All new state is local. No telemetry, account, cloud, or synchronization is added.
- **UX and accessibility**: The design preserves the dual-pane invariant, defines keyboard equivalents for every pointer interaction, keeps visible focus, and respects reduced motion and theme tokens.
- **Quality**: Each new core service, migration, command, and component has a named automated test layer; the existing protocol/CLI/MCP suites remain unaffected.
- **Performance**: Palette/sidebar filtering is bounded and synchronous over at most 200 entries; dragging changes only presentation state; listing and transfer paths are not modified.

No gate violations. No complexity exceptions are required.

## Complexity Tracking

No constitution violations or complexity exceptions were identified. The added modules are small, cohesive units justified by the core-independence rule and by separating workspace state from existing tab, favorite, and transfer state.
