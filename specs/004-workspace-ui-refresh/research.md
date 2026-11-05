# Phase 0 Research: Workspace UI Refresh and Dual-Pane Connection Hub

**Feature**: `004-workspace-ui-refresh` | **Date**: 2026-11-05

All technical unknowns from the plan's Technical Context are resolved below. No `NEEDS CLARIFICATION` markers remain.

## Decision 1: Where workspace layout and connection history are persisted

**Decision**: Extend the existing versioned metadata document (`metadata.json`) with two additive `AppState` fields: `workspace_layout` and `connection_history`. Bump the JSON schema version from 1 to 2 and extend the ordered migration ladder so versions 0 and 1 load as version 2. Use `#[serde(default)]` on every new field so a version-1 document without them still loads. Persist through the existing `MetadataStore` and `JsonMetadataStore` atomic-write path.

**Rationale**: Layout and history are small, local, non-secret metadata and already belong to the same lifecycle as preferences, tabs, and favorites. Reusing the existing store preserves the single atomic write, backup file, restrictive permissions, and migration mechanism. It also avoids introducing `localStorage` (which would split state across the Rust and web layers) or a new store dependency. Additive defaults keep rollback to an older build from destroying a newer document.

**Alternatives considered**:
- Browser `localStorage`: rejected because layout and history would be invisible to the core, CLI/MCP reuse, and the existing backup/migration mechanism; it also makes cross-platform tests harder.
- A new `workspace.json` store: rejected as unnecessary fragmentation for a few hundred bytes of state.
- Recomputing history from favorite `lastUsedAt` values: rejected because history must survive deletion of a favorite, must deduplicate by saved server, and must be independently clearable.
- SQLite: rejected as a heavier dependency for a tiny local dataset.

## Decision 2: Always-dual-pane composition and the disconnected right pane

**Decision**: Keep one `DualPane` shell that always renders a local pane on the left and a right pane on the right. Extract the current pane markup into `FilePane.svelte`. The right pane renders the remote file listing when a connection is active and `ConnectionHub.svelte` when no connection is active. The hub owns Favorites and History tabs, while the sidebar remains available in both states. The remote pane never moves to the left and the local pane is never hidden.

**Rationale**: The current implementation already renders both panes, but the right side becomes a plain empty message and the sidebar only appears after a connection. Extracting a pane keeps the split, keyboard handling, drag-and-drop, and listing behavior in one place while letting the right side swap between hub and listing. This satisfies the core invariant with minimal structural churn.

**Alternatives considered**:
- A separate disconnected screen: rejected because it breaks the always-dual-pane requirement and loses the local browser.
- Swapping the hub into the left pane: rejected because the local pane must remain local.
- Keeping the hub in a modal: rejected because the right pane must be the startup and reconnection surface.

## Decision 3: Pane split interaction and persistence

**Decision**: Model the split as a `splitRatio` between 0.2 and 0.8 (clamped), rendered with CSS Grid `grid-template-columns: minmax(320px, {ratio}fr) 4px minmax(320px, {1-ratio}fr)`. Implement the divider as a focusable `role="separator"` with `aria-orientation="horizontal"`, `aria-valuemin`, `aria-valuemax`, and `aria-valuenow`. Pointer drag updates presentation state only and commits on pointer release; ArrowLeft/ArrowRight adjust by 2%, Home/End clamp, and Enter or a reset button restores 50/50. Persist the committed ratio through `workspace_set` with a short debounce. At widths below the point where two 320 px panes plus the sidebar fit, the sidebar visually collapses first; the user's saved visibility preference is not overwritten.

**Rationale**: CSS Grid keeps the ratio declarative and avoids manual width math. A fixed 4 px gutter is easy to hit and separates pane backgrounds. Enforcing a 320 px minimum per pane at the 900 px supported minimum leaves room for the divider and keeps both panes usable. Keyboard controls satisfy FR-003 without a pointer. Committing on release avoids a write on every pointer move.

**Alternatives considered**:
- Storing pixel widths: rejected because window resizing then requires rescaling and can violate minimums.
- Using a third-party split-pane component: rejected under YAGNI and the dependency-review rule; the behavior is small and testable.
- Writing the layout on every pointer move: rejected because it adds unnecessary disk I/O during a drag.

## Decision 4: Unified sidebar structure and narrow-window behavior

**Decision**: Replace the current connected-only `FavoritesSidebar` and the separate always-visible server-favorites panel with `WorkspaceSidebar.svelte`, docked before the local pane. It has three labeled, independently collapsible sections: Saved Servers, Directory Favorites, and Automation. A single search field filters saved servers and directory favorites by name, host, protocol, group, and path. Section collapsed states and overall visibility persist in `WorkspaceLayout`. On narrow windows the sidebar auto-collapses when the workspace cannot fit the sidebar plus two minimum-width panes, and a labeled button reopens it as an overlay without shrinking either pane below its minimum. Automation status and controls open from the sidebar section instead of occupying a permanent bottom strip.

**Rationale**: One sidebar directly satisfies FR-044 through FR-049 and removes duplicate navigation surfaces. Deriving the effective collapsed state from available width rather than overwriting the saved preference means a user who reopens the window gets their preferred layout back. An overlay on narrow windows keeps the two panes usable.

**Alternatives considered**:
- Keeping both the old favorites sidebar and the new hub: rejected because it duplicates directory favorites and creates competing navigation.
- A tabbed sidebar: rejected because the spec asks for clearly labeled sections and remembered collapse state.
- Hiding a pane when narrow: rejected by FR-001 and FR-008.

## Decision 5: Breadcrumb display, editing, and overflow

**Decision**: Replace the always-visible path `<input>` in `BrowserToolbar` with `BreadcrumbBar.svelte`. A pure helper `lib/browser/breadcrumbs.ts` splits POSIX and Windows-style absolute paths into segments while preserving the root/prefix, and produces navigation targets. Clicking a segment navigates; clicking the current folder toggles edit mode; Ctrl/Cmd+L also enters edit mode; Enter submits and Escape cancels. On submit, the pane attempts navigation and keeps its current folder on failure with an actionable notice. When space is insufficient, root and current segments remain visible and middle segments move into an overflow menu; every segment keeps its full path in `aria-label`/`title`. The bar is hidden when the pane has no location (disconnected remote hub).

**Rationale**: A separate pure helper makes path edge cases (trailing slashes, roots, drive letters, escaped characters) unit-testable without mounting a component. Reusing the existing navigate function keeps history and error behavior consistent with the rest of the pane. The overflow menu is a standard pattern and keeps all segments reachable by keyboard.

**Alternatives considered**:
- Keeping the path field and adding breadcrumbs above it: rejected by the clarification that breadcrumbs replace the default location field, and it wastes vertical space.
- Rendering every segment always: rejected because long paths overflow and hide the current folder.
- Parsing remote paths with a platform-specific library: rejected because remote paths follow the server's POSIX convention, not the client OS.

## Decision 6: Command palette scope, data source, and keyboard model

**Decision**: Implement the palette entirely in the Svelte layer with `lib/commands/registry.ts` and `CommandPalette.svelte`. Build entries on open from existing stores: saved servers (connect), directory favorites (navigate), open tabs (activate), and a fixed safe-action list (new connection, new favorite, Preferences, refresh active pane, toggle hidden files). Group results by category, score case-insensitive prefix/substring matches against label, host, username, protocol, group, and path, and sort by score then label. No search text shows recent history entries and suggested actions. Unavailable actions remain visible with a disabled state and a reason. Destructive actions (delete, remove credential, clear history, cancel-all) and transfer-queue controls are excluded. Cmd+K on macOS and Ctrl+K elsewhere open the palette from the primary workspace; Arrow keys move, Enter runs, Escape dismisses. The palette uses the existing focus-trap/restore pattern and never opens on top of another modal.

**Rationale**: All data already exists in stores; a backend command would add latency and a second source of truth. Synchronous filtering over at most 200 entries comfortably meets the 100 ms target and avoids debounce complexity. A fixed safe-action allowlist enforces FR-020 at the registry boundary rather than relying on each component. Excluding the palette while another modal is open satisfies the edge case without stacking focus traps.

**Alternatives considered**:
- A Rust-side search command: rejected because it adds IPC latency and duplicates store state for no measurable benefit at this scale.
- A general fuzzy-search dependency: rejected under YAGNI; a small deterministic scorer is easier to test and tune.
- Including transfer controls: explicitly rejected by the specification.
- A global registry singleton with side effects: rejected in favor of a pure function over current state.

## Decision 7: Connection history model and recording point

**Decision**: Add a core `ConnectionHistoryService` backed by `AppState.connection_history`. Record only after a successful connect in `connections_connect` and `favorites_connect`; never record failed or rejected attempts. Deduplicate by the saved server identity (favorite ID when present, otherwise connection-profile ID), keep the newest `lastUsedAt`, and cap the list at 50. Store a denormalized non-secret display snapshot (display name, protocol, host, port, username, profile ID, optional favorite ID) so deleted servers remain explainable. Clear only through `connection_history_clear` with `confirmed: true`; clearing history does not touch favorites, tabs, or credentials.

**Rationale**: Recording in the command boundary guarantees that only successful connections are retained and keeps protocol logic out of the UI. A denormalized snapshot lets the History tab remain useful after a favorite is deleted, while a connect action to a missing server fails with the existing `not_found` error. The 50-entry cap matches the spec assumption and bounds document size.

**Alternatives considered**:
- Recording in the connection service: rejected because the service is shared with CLI/MCP and this feature must not change automation behavior.
- Deriving history from favorites: rejected because deletion would erase history and independent clearing would be impossible.
- Storing failures with error details: rejected by FR-013 and the privacy/security rule.

## Decision 8: Favorite local/remote path storage and restoration

**Decision**: Add `local_path: Option<String>` and `remote_path: Option<String>` to `ServerFavorite` (and to the input/view projections) with `#[serde(default)]`. Validate that non-empty values are absolute; trim whitespace; treat blank as absent. Prefill both from the active tab when saving from a session. On connect, resolve each saved path through a core `navigation` helper and a new `browser_resolve_path` command:
- Local: use filesystem metadata to walk from the saved path up to the nearest existing readable directory; fall back to the user home directory.
- Remote: use the active connection's `stat`/listing capability to walk ancestors; fall back to the server's login directory (currently `.`).
Return `{ requestedPath, path, fallback, notice? }`. The UI opens each pane at the resolved path and shows at most one non-blocking notice per pane. Editing paths affects only future connections. Favorites with no paths keep the pre-existing behavior.

**Rationale**: Storing paths on the favorite (rather than the connection profile) keeps them part of the user-facing favorite and avoids changing CLI/MCP profile semantics. Resolution belongs in the core because ancestor walking, permission checks, and login-directory fallback are rules, not presentation. Returning a structured resolution lets the UI show exactly one actionable notice. Additive defaults preserve old favorites.

**Alternatives considered**:
- Storing paths on `ConnectionProfile`: rejected because it would expose them through CLI/MCP profile payloads and change automation contracts.
- Blindly setting the pane path: rejected because a stale path would leave the pane in an error state and violate FR-027.
- Resolving in the UI only: rejected because remote resolution needs the protocol connection and would duplicate core rules.
- Storing relative paths: explicitly out of scope in the spec assumptions.

## Decision 9: SFTP private-key discovery and native browsing

**Decision**: Add a core `ssh_keys` module that returns `SshKeyCandidate { fileName, path }` from the platform SSH directory: `~/.ssh` on macOS/Linux and `%USERPROFILE%\.ssh` on Windows, resolved from `HOME`/`USERPROFILE` with an injectable base for tests. Only conventional private-key file names are offered (`id_rsa`, `id_dsa`, `id_ecdsa`, `id_ed25519`, `id_ed25519_sk`, `id_ecdsa_sk`, and conventional `*.pem`/`*.key` names) while `.pub`, `known_hosts`, `config`, `authorized_keys`, sockets, and unrelated files are excluded. Do not read key contents; discovery reads directory entries and file metadata only. Prefill only when exactly one candidate exists. Add `ssh_keys_list` and `ssh_keys_validate` commands. The key field uses `@tauri-apps/plugin-dialog` `open()` with `defaultPath` set to the platform SSH directory; the chosen path fills the field. Manual entry remains available, and validation reports missing, unreadable, directory, or public-key paths without echoing file contents. Authentication still requires explicit key-auth selection and connection confirmation.

**Rationale**: A filename allowlist is deterministic, avoids parsing secret material, and satisfies "supported conventional private keys" without exposing contents. Injecting the home directory makes tests portable. The dialog plugin is already installed and permitted, so no new dependency or capability is needed. Returning only names and paths keeps the security boundary clear.

**Alternatives considered**:
- Reading the first bytes to detect PEM/OpenSSH headers: rejected because it reads key material into the UI-facing path and adds little value over a conventional-name allowlist.
- Using the SSH agent or generating keys: explicitly out of scope.
- Scanning every file in `~/.ssh`: rejected because it would suggest config files, public keys, and sockets.
- Building a custom file chooser: rejected; the native dialog is already available and more accessible.

## Decision 10: Design tokens, visual states, and accessibility

**Decision**: Extend `app.css` into a small shared token set for color, spacing, typography, borders, radii, elevation, focus rings, and motion. Use semantic tokens (`--surface`, `--surface-raised`, `--text`, `--text-muted`, `--border`, `--accent`, `--danger`, `--warning`, `--success`, `--focus-ring`) with light and dark values, plus `prefers-reduced-motion` to disable non-essential transitions. Refactor refreshed components to use tokens and shared state classes rather than hard-coded colors. Preserve the existing `data-theme` application and system/light/dark choices. Add visible `:focus-visible` treatment, accessible names to icon-only controls, and `aria-live` for status/fallback notices.

**Rationale**: The current styles mix `Canvas`, `GrayText`, and hard-coded hex values. A token layer directly supports FR-005, FR-006, FR-007, and SC-009, and it makes the contrast review mechanical. Reusing the existing theme application avoids a theme rewrite. Reduced-motion support is already a stored preference; the CSS media query enforces it in the browser layer.

**Alternatives considered**:
- A component library: rejected as a large dependency and visual-identity change beyond the spec.
- Per-component CSS variables: rejected because it fragments the visual system and makes contrast review harder.
- A new theme engine: rejected; the existing `data-theme` plus `color-scheme` is sufficient.

## Decision 11: Layout and history schema migration

**Decision**: Keep backward compatibility additive. Set `SCHEMA_VERSION = 2`; migrate `0 -> 1 -> 2` and `1 -> 2`. Version 2 adds `workspaceLayout` and `connectionHistory` plus optional favorite path fields. Because every new field has a serde default, loading a version-1 document succeeds; saving upgrades it to version 2. Reject unknown future versions as today.

**Rationale**: The existing migration ladder is ordered and fail-closed. A version bump documents the shape change without requiring a data rewrite, and serde defaults protect users who downgrade temporarily. The change is testable with fixtures for versions 0, 1, and 2.

**Alternatives considered**:
- Keeping version 1: rejected because the document shape changes materially and future migrations would lose the boundary.
- A destructive rewrite: rejected because it risks user data and violates the compatibility requirement.

## Decision 12: Verification strategy

**Decision**: Verify at four layers:
1. `thriftp-core` unit tests with temporary homes and the existing fakes: layout clamping, history dedupe/cap/clear, favorite path validation, ancestor fallback, SSH candidate filtering, and palette-independent rules.
2. `thriftp-storage-json` tests for version 0/1/2 loading, additive defaults, and atomic save.
3. `apps/desktop/src-tauri/tests` command contract tests for camelCase payloads, redaction, history side effects, workspace persistence, and path resolution errors.
4. `apps/desktop/src/**/*.test.ts` Vitest tests for breadcrumbs, palette grouping/keyboard/disabled actions, hub tabs, split keyboard control, sidebar filtering/collapse, key suggestions, and favorite path prefill. `quickstart.md` covers the manual cross-platform and native-dialog checks.

**Rationale**: This maps each requirement to the lowest layer that can prove it and keeps tests Docker-free. The existing containerized protocol suite remains a separate stage and is not required by this feature. Frontend tests run in jsdom, so native dialog behavior is verified manually while the selection logic is unit-tested with a mocked dialog module.

**Alternatives considered**:
- End-to-end browser automation of the Tauri window: valuable but not currently part of the repository's CI; deferred to the manual quickstart.
- Only manual validation: rejected by the constitution's automated-test requirement.
- Testing palette search in Rust: rejected because the registry is presentation state and has no backend consumer.
