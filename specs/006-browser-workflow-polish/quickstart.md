# Quickstart Validation Guide: Browser and Favorites Workflow Polish

**Feature**: `006-browser-workflow-polish` | **Date**: 2026-11-06

This guide proves the feature works end to end. It references [data-model.md](./data-model.md), [desktop-ipc.md](./contracts/desktop-ipc.md), and [workspace-ui.md](./contracts/workspace-ui.md) rather than duplicating their rules. Implementation details belong in `tasks.md` and the implementation phase.

## Prerequisites

- Rust 1.99.1 with `cargo`, `rustfmt`, and `clippy`.
- Node 26.13.0 and npm.
- Tauri platform prerequisites for the desktop shell.
- A reachable FTP, FTPS, or SFTP test server for transfer and permission scenarios.
- Two active connections for the per-connection statistics scenario.
- A remote account that can change permissions on a disposable directory for recursive permission tests.
- Optional Docker only for the existing protocol integration suite; this feature's tests do not require it.

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

- Core tests cover permission-bit parsing/rendering and recursive traversal with symlink skipping and per-item failures.
- Storage tests confirm old metadata without `sortDirection` loads with ascending defaults.
- Tauri tests cover the recursive chmod contract, extended inspector metadata, and transfer timestamps.
- Frontend tests cover modifier selection, sort toggles, favorite path resolution/browsing, dialog reachability, linked navigation, queue statistics, permission controls, recursive confirmation, copy path, and control placement.
- Lint, format, and the production build exit successfully.

## Run the Desktop Application

```bash
cd apps/desktop
npx tauri dev
```

Use a disposable saved server and test directories. Never run recursive permission tests against a directory whose contents you cannot replace.

## Scenario 1: Favorites Create and Edit Reliably (P1)

1. Connect to a saved server and leave both panes at their default working locations.
2. Choose **Save as favorite**.
3. Confirm the form shows absolute local and remote paths rather than `.` values.
4. Add a valid absolute remote path and save.
5. Confirm the favorite is created and the remote path is shown in the favorites list.
6. Edit the favorite and activate the local-path **Browse** button.
7. Choose a folder and confirm it fills the field; reopen Browse and cancel, confirming the value is unchanged.
8. Resize the window to 900 × 600, open the editor, and add enough content or validation messages to exceed the visible area.
9. Scroll the form body and confirm Save, Cancel, and Test remain reachable at all times.
10. Save the edit and confirm the updated path appears on the next connection.

Expected: new favorites with remote paths succeed; browse works and cancels safely; dialog actions remain reachable. See [workspace-ui.md](./contracts/workspace-ui.md#favorite-dialogs) and [desktop-ipc.md](./contracts/desktop-ipc.md#favorite-contract).

## Scenario 2: Select and Sort Files (P1)

1. Open a directory with at least 50 mixed files and folders.
2. Click a file without modifiers; confirm only it is selected.
3. Shift-click another file; confirm the full range is selected.
4. Cmd-click on macOS or Ctrl-click on Windows/Linux several other files; confirm individual selections toggle while others remain.
5. Filter the listing and repeat a Shift selection; confirm only visible rows are included.
6. Click **Name**; confirm ascending sort and indicator. Click again; confirm descending.
7. Repeat for **Size** and **Modified** in both panes.
8. Confirm selected files remain selected after each sort.
9. Refresh the listing; confirm still-present selections survive and removed entries disappear from the selection.

Expected: standard platform selection and visible, independent column sorting. See [workspace-ui.md](./contracts/workspace-ui.md#selection) and [workspace-ui.md](./contracts/workspace-ui.md#column-sorting).

## Scenario 3: Link Local and Remote Navigation (P2)

1. Connect to a server and create matching local and remote folder trees: `project/a`, `project/b`, and `project/nested/c`.
2. Activate **Link** in the remote session line.
3. Move up from `project/a` on the local side; confirm the remote pane also moves up when possible.
4. Open `project` locally; confirm the remote pane opens `project` if it exists.
5. Use a remote breadcrumb to jump up one or more ancestors; confirm the local pane moves up the same number of levels.
6. Type a path that is a descendant of the current remote folder; confirm the local pane applies the same child segments.
7. Navigate through local and remote history and confirm matching relative history movement when possible.
8. Activate a tab or favorite and confirm the other pane applies the same relative move when the counterpart exists.
9. Navigate to a location with no counterpart; confirm the originating pane still navigates and the other pane stays unchanged without a blocking error.
10. Activate **Unlink** and confirm each pane moves independently afterward.
11. Disconnect and reconnect; confirm link state starts unlinked.

Expected: every navigation action mirrors by relative movement when possible and never blocks the active pane. See [workspace-ui.md](./contracts/workspace-ui.md#linked-navigation).

## Scenario 4: Per-Connection Queue Statistics (P2)

1. Connect to two servers or two profiles simultaneously.
2. Start several transfers on each connection.
3. Let at least two transfers complete on each connection; fail or cancel one on each.
4. Confirm each connection shows only its own completed file count, completed volume, and average speed.
5. Confirm failed and cancelled transfers do not increase counts or volume.
6. Confirm volume formatting changes appropriately between KB, MB, and GB.
7. Disconnect one connection; confirm its statistics disappear.
8. Reconnect the same server; confirm its statistics start at zero.
9. Start a transfer, disconnect, reconnect, and confirm the old transfer does not count toward the new session.

Expected: accurate, isolated, session-scoped statistics. See [workspace-ui.md](./contracts/workspace-ui.md#queue-statistics) and [data-model.md](./data-model.md#connectiontransferstatistics).

## Scenario 5: Inspect and Change Permissions Safely (P3)

1. Open file information for a remote file with known permissions.
2. Confirm User, Group, and World rows show the correct Read, Write, and Execute states.
3. Confirm the octal and symbolic summaries match the checkboxes.
4. Toggle permissions and confirm the summaries update before applying.
5. Apply the mode and confirm the information and listing reflect the change.
6. Open a remote directory and choose **Apply to enclosed…**.
7. Confirm the dialog explains the recursive scope and that symbolic links will not be followed.
8. Cancel; confirm no items changed.
9. Confirm; verify all descendant folders and files changed, symlinks were skipped, and failures were reported without hiding successes.
10. Open a local item; confirm permissions are read-only and no editing controls appear.
11. Activate **Copy path** for a remote item; confirm the exact server path is on the clipboard.
12. Deny clipboard access; confirm an actionable failure message appears while the path remains visible.

Expected: understandable permission detail, safe recursive changes, and exact path copying. See [workspace-ui.md](./contracts/workspace-ui.md#inspector-permissions) and [desktop-ipc.md](./contracts/desktop-ipc.md#inspector-contract).

## Scenario 6: Remote Pane Controls and Scroll Containment (P3)

1. Connect to a server.
2. Confirm **Disconnect** appears beside the remote pane title and server name.
3. Activate it; confirm the session disconnects and the button disappears.
4. Confirm the session line still offers the active-session selector and Save as favorite.
5. Confirm both pane toolbars show no back or forward buttons.
6. Confirm parent-directory, refresh, breadcrumbs, filter, hidden-file, and sort controls still work.
7. Open a directory with many files in each pane and confirm each listing has its own scrollbar.
8. Scroll one pane; confirm the other pane does not move and the pane headers and toolbars remain fixed.
9. Confirm both panes remain equal in height and the queue stays below them and collapsible.

Expected: requested control placement and strictly per-listing scrolling. See [workspace-ui.md](./contracts/workspace-ui.md#remote-pane-controls) and [workspace-ui.md](./contracts/workspace-ui.md#scroll-and-layout).

## Cross-Platform Checks

Run the manual scenarios on macOS, Linux, and Windows where available. Specifically verify:

- Cmd vs Ctrl modifier selection.
- Native local folder chooser behavior and text-entry fallback.
- `Option`/Alt and keyboard focus behavior in dialogs.
- Column sorting and linked navigation with platform path separators.
- Remote permission behavior against the available server.
- Clipboard success and failure behavior.

## Regression Checks

- Existing connection, credential, host-key, transfer, queue, plain-FTP warning, CLI, and MCP behavior is unchanged.
- Existing metadata without sort directions loads with ascending defaults.
- Existing favorites without paths still create, edit, and connect correctly.
- The app shell remains non-scrolling; only directory listings scroll.
- No credentials, private-key material, file contents, or secrets appear in statistics, clipboard values, dialogs, logs, or errors.
