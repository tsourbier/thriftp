# Quickstart Validation Guide: Workspace UI Refresh

**Feature**: `004-workspace-ui-refresh` | **Date**: 2026-11-05

This guide proves the feature works end to end. It references [data-model.md](./data-model.md) and the contracts instead of duplicating their rules. Implementation details belong in `tasks.md` and the implementation phase.

## Prerequisites

- Rust 1.88.0+ with `cargo`, `rustfmt`, and `clippy`.
- Node 22+ and npm.
- Tauri platform prerequisites for the desktop shell.
- Optional: a reachable FTP, FTPS, or SFTP test server with a known key, plus Docker only for the existing protocol integration suite. This feature's tests do not require Docker.
- For SSH key checks: a writable temporary home with (a) no `.ssh`, (b) one conventional private key, and (c) multiple conventional private keys.

## Build and Automated Checks

Run from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked

cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build
```

Expected:

- All Rust tests pass without Tauri or a container runtime.
- New core tests cover layout clamping, history deduplication/cap/clear, favorite path validation, path ancestor fallback, and SSH candidate filtering.
- JSON store tests load schema versions 0, 1, and 2; version-1 documents gain defaults; unknown future versions fail closed.
- Frontend tests cover breadcrumbs, palette grouping and keyboard behavior, hub tabs, split keyboard control, sidebar search/collapse, key suggestions, and favorite path prefill.
- Lint, format, and the production build exit successfully.

## Run the Desktop Application

```bash
cd apps/desktop
npm run dev
```

To exercise the Rust shell directly, use the repository's Tauri development command for the desktop app. Use a temporary app-data directory or a disposable profile if you need a clean first launch.

## Scenario 1: Always-Dual-Pane Workspace and Layout Persistence (P1)

1. Launch with no saved connection.
2. Confirm the sidebar is at the far left, the local pane is next, and the right pane shows the connection hub.
3. Resize the window toward 900 × 600.
4. Confirm the sidebar collapses before either pane drops below a usable width.
5. Drag the divider to roughly 35/65, collapse one sidebar section, select the History hub tab, and restart.
6. Confirm the local pane is still on the left, the right pane still on the right, and the split, sidebar visibility, section state, and hub tab are restored.
7. Switch system/light/dark themes and confirm surfaces update immediately with readable contrast.
8. Tab through the title bar, sidebar, panes, toolbar, and dialogs and confirm a visible focus indicator on every control.

Expected: both panes remain visible and usable; roles never swap; the persisted layout returns after restart. See [workspace-ui.md](./contracts/workspace-ui.md#shell-layout) and [data-model.md](./data-model.md#workspacelayout).

## Scenario 2: Hub Reconnection and History (P1)

1. Start with at least two saved server favorites and no active connection.
2. Open the right-pane hub and switch between Favorites and History.
3. Search Favorites by name, host, and group; connect to a server.
4. Confirm the right pane switches to the remote listing and the left pane remains local.
5. Disconnect and confirm the hub returns with the tab you last selected.
6. Confirm History shows the successful connection newest first with name, protocol, host, last-used time, and a connect action.
7. Make one failed connection attempt and confirm it does not appear in History.
8. Clear History with confirmation and confirm favorites, tabs, credentials, and the active connection are unaffected.
9. While a transfer is running, disconnect and confirm progress remains visible and controllable.

Expected: successful connects only; one entry per saved server; history is bounded and clearable; transfers are unaffected. See [desktop-ipc.md](./contracts/desktop-ipc.md#connection_history_list) and [data-model.md](./data-model.md#connectionhistoryentry).

## Scenario 3: Command Palette (P1)

1. From the primary workspace, press `Cmd+K` on macOS or `Ctrl+K` elsewhere.
2. Confirm focus is in the search field.
3. Type a partial saved-server name, host, favorite name, tab title, and action label.
4. Confirm results are grouped and update as you type.
5. Use arrow keys and `Enter` to connect to a server, switch to a tab, and open Preferences.
6. Run a directory-favorite result while disconnected and confirm you get a clear explanation of what is required.
7. Confirm delete, clear history, remove credential, cancel-all, and queue controls never appear.
8. Confirm an action that is unavailable in the current context is visible, disabled, and has a reason.
9. Open the palette with no query; confirm recent/suggested entries appear. Type a query with no matches; confirm the empty state appears and the query remains editable.
10. Press `Escape`; confirm the palette closes and focus returns to the control that opened it.

Expected: keyboard-only discovery and execution works; destructive and queue actions stay in their dedicated surfaces. See [workspace-ui.md](./contracts/workspace-ui.md#command-palette).

## Scenario 4: Favorite Local and Remote Paths (P1)

1. Connect to a saved server.
2. In the local pane, navigate to a non-default folder. In the remote pane, navigate to a non-default folder.
3. Choose **Save as favorite** and confirm both path fields are pre-filled.
4. Save. Confirm the favorite row shows the stored remote path without opening the editor.
5. Disconnect and reconnect from the favorite.
6. Confirm the local pane opens the saved local path and the remote pane opens the saved remote path.
7. Edit the favorite with one path blank and one path set; save and reconnect. Confirm the blank path uses the existing default behavior.
8. Create a favorite whose saved local path no longer exists and whose saved remote path no longer exists.
9. Reconnect and confirm both panes open the nearest accessible ancestor (or home/login fallback) and exactly one actionable notice per pane explains the fallback.
10. While connected, edit the favorite paths and confirm the active connection does not move; reconnect and confirm the new paths apply.

Expected: both paths restore on connect; stale paths fall back without blocking the connection; updates apply only to later connections. See [data-model.md](./data-model.md#serverfavorite-extended) and [desktop-ipc.md](./contracts/desktop-ipc.md#browser_resolve_path).

## Scenario 5: SFTP Private-Key Discovery (P1)

1. In a temporary home with no `.ssh` directory, create an SFTP favorite, select SSH private-key authentication, and focus the key field.
2. Confirm the field defaults to the platform SSH directory and gives guidance when the directory is missing; confirm manual entry still works.
3. In a temporary home with exactly one conventional private key, focus the key field and confirm that key is pre-filled.
4. In a temporary home with multiple conventional private keys, focus the field and confirm no key is chosen silently and all candidates are listed by file name.
5. Add public keys, `known_hosts`, `config`, `authorized_keys`, and a socket to the directory; confirm none appear as candidates.
6. Activate the key button (🔑) and confirm the native file chooser opens at the platform SSH directory; choose a key and confirm the field is filled.
7. If the native chooser is unavailable, confirm typing/pasting a path still works.
8. Select a missing path, a directory, an unreadable file, and a `.pub` file; confirm each save/test attempt produces an actionable message with no key contents.
9. Review the rendered interface, logs, and `metadata.json`; confirm no private-key contents or passphrases appear.

Expected: discovery is a convenience only; it never authenticates or exposes key material. See [desktop-ipc.md](./contracts/desktop-ipc.md#ssh_keys_list) and [data-model.md](./data-model.md#sshkeycandidate).

## Scenario 6: Breadcrumbs (P2)

1. Navigate deep into the local tree and the remote tree.
2. Confirm both panes show ordered breadcrumb segments from root to current.
3. Activate an ancestor segment in each pane and confirm that pane navigates there.
4. Click the current folder (or press `Ctrl/Cmd+L`), edit the path, press `Enter`, and confirm navigation.
5. Enter an invalid/inaccessible path and confirm the pane stays put and shows one actionable message.
6. Press `Escape` while editing and confirm the breadcrumb display returns without navigating.
7. Resize the window until the path is too long; confirm middle segments collapse into an overflow control while root and current remain visible and every segment remains reachable.
8. Navigate by keyboard and screen reader; confirm segments are labeled and expose their full path.
9. Disconnect and confirm the hub has no breadcrumb bar.

Expected: breadcrumbs replace the path field as the default location display and remain fully navigable. See [workspace-ui.md](./contracts/workspace-ui.md#breadcrumbs).

## Scenario 7: Unified Sidebar (P2)

1. Populate saved servers and directory favorites.
2. Open the sidebar and confirm Saved Servers, Directory Favorites, and Automation sections.
3. Type a query matching a name, host, protocol, group, and path; confirm each filter works across both saved sections.
4. Clear the search in one action and confirm all entries return.
5. Collapse and expand each section, restart, and confirm the states are remembered.
6. Open the Automation section and confirm status and controls are available without a permanent workspace panel.
7. Activate a saved server and a directory favorite; confirm the same connect/navigate behavior as the hub.
8. Narrow the window until the sidebar auto-collapses; confirm a labeled reopen control remains and both panes stay usable.
9. Reopen the sidebar at the narrow width and confirm it overlays rather than compressing the panes below their minimum.

Expected: one predictable navigation surface with persistent state and one-action search clearing. See [workspace-ui.md](./contracts/workspace-ui.md#unified-sidebar).

## Cross-Platform Checks

Run the manual scenarios on macOS, Linux, and Windows where available. Specifically verify:

- `Cmd+K` on macOS and `Ctrl+K` on Linux/Windows.
- `~/.ssh` discovery on macOS/Linux and `%USERPROFILE%\.ssh` on Windows.
- Native file chooser opening at the platform SSH directory.
- Minimum 900 × 600 layout, sidebar-first collapse, and keyboard-only operation.
- Light/dark/system theme contrast and reduced-motion behavior.

## Regression Checks

- Existing FTP/FTPS/SFTP connect, transfer, queue, credential, host-key, and plain-FTP warning behavior is unchanged.
- Existing favorites, tabs, preferences, and version-1 metadata load without data loss.
- CLI and MCP commands, output, and security rules are unchanged.
- No new telemetry, cloud call, account requirement, or secret store appears.
