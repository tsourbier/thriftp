# Workspace UI Contract: Browser and Favorites Workflow Polish

**Feature**: `006-browser-workflow-polish` | **Date**: 2026-11-06

This contract defines interaction behavior for the changed desktop surfaces. Existing accessibility, focus-trap, theme, reduced-motion, and error-redaction rules remain in force.

## Selection

### Modifier behavior

| Input | Result |
|-------|--------|
| Plain click/activation | Replace selection with the target and set it as the anchor |
| Shift-click/activation | Select the continuous range from the anchor to the target in the current visible order |
| Cmd-click on macOS | Toggle the target, preserve other selections, and set it as the anchor |
| Ctrl-click on Windows/Linux | Toggle the target, preserve other selections, and set it as the anchor |

Rules:

- If there is no valid anchor, Shift behaves like a plain selection.
- Range selection includes only rows currently visible after filtering and sorting.
- Files, directories, symlinks, and other entries are all selectable.
- Changing sort order or filtering preserves selected paths that still exist.
- The local and remote panes keep independent selections and anchors.

### Range and refresh behavior

- The anchor is a path, not an index.
- If a refresh removes the anchor, the next Shift action establishes a new anchor from the target.
- If a refresh removes a selected path, that path is dropped from the selection.
- Selection never re-fetches the listing.

## Column Sorting

- The Name, Size, and Modified column headers are interactive buttons with an accessible name.
- Activating a different column sorts that column ascending.
- Activating the active column toggles between ascending and descending.
- The active column and direction are visible in the header and exposed through `aria-sort`.
- The existing toolbar Sort control remains available and updates the same column without resetting the current direction.
- Local and remote panes keep independent sort keys and directions.
- The sort state persists with the tab view settings; new tabs use the preference default.

## Favorite Dialogs

- The local-path field is a text input with an adjacent **Browse** button.
- Browse opens the native folder chooser at the current absolute local path when it is an existing directory, otherwise at the user's home directory.
- Choosing a folder fills the field; cancelling leaves the previous value untouched.
- The remote path stays a text input and keeps absolute-path validation.
- The dialog content scrolls inside a height-constrained body.
- The Save, Cancel, Test, and other action buttons remain outside the scroll region and are reachable at 900 × 600.
- Validation and credential messages appear inside the scrollable body without pushing actions off-screen.
- Save-as-favorite resolves the active tab's `.` working paths before opening the form; unresolved paths are omitted.

## Linked Navigation

- The remote session line contains a **Link** control when unlinked and an **Unlink** control when linked.
- Link state is session-scoped and resets when the remote session ends.
- Turning link on or off does not change either pane's current folder.
- Linked navigation mirrors every navigation action:
  - parent-directory action;
  - opening a child directory;
  - breadcrumb ancestor jump;
  - typed path;
  - history back/forward;
  - tab activation;
  - favorite activated from the pane.
- Mirroring uses the relative move: the same number of parent steps and the same child segments.
- The counterpart is resolved before navigation. If it does not exist, is unreadable, or the other pane is disconnected, the other pane stays unchanged.
- The originating pane never waits for or fails because of the mirror operation.
- A mirrored navigation must not trigger another mirror.

## Queue Statistics

- The queue panel shows a statistics block for each active connection.
- Each block displays:
  - completed file count;
  - completed volume in a human-readable unit;
  - average completed-transfer speed.
- Statistics are isolated by connection profile and by the current connection session.
- A newly connected session shows zero values until a transfer completes.
- A completed transfer updates its connection's statistics immediately from the state event.
- Failed, cancelled, interrupted, pending, paused, and active transfers do not contribute.
- A transfer that started before the current connection session does not contribute to the new session.
- Volume formatting scales through B, KB, MB, and GB.
- Average speed is shown as unavailable when no completed transfer has a positive duration.
- Statistics are never persisted and disappear when the session ends.

## Inspector Permissions

- The permissions section shows three rows: User, Group, World.
- Each row has Read, Write, Execute controls with accessible names.
- The section also shows the octal value and symbolic recap such as `rw-r--r--`.
- Toggling a control updates the draft octal and symbolic summaries immediately.
- **Apply** commits the draft for a remote item when the server supports permissions.
- For a remote directory, **Apply to enclosed…** opens a confirmation dialog explaining that all descendant folders and files will change and symbolic links will not be followed.
- Cancelling the confirmation changes nothing.
- Confirming calls the recursive command and reports changed, skipped, and failed items.
- Local items are read-only and show the available permission detail without editing controls.
- If permissions are unavailable, the controls are disabled and an explanation is shown.
- **Copy path** copies the inspected remote path exactly and shows an inline “Path copied” confirmation for at least two seconds; a clipboard failure is reported without hiding the path.

## Remote Pane Controls

- The remote pane heading shows `Remote · <server name>` and a **Disconnect** button beside it when connected.
- Disconnect uses the existing selected-session disconnect behavior.
- No Disconnect button appears when the remote pane has no active session.
- The session line retains the active-session selector and Save as favorite.
- Back and forward buttons are absent from both pane toolbars.
- Parent-directory, refresh, breadcrumbs, filter, hidden-file, and sort controls remain available.

## Scroll and Layout

- The application shell remains viewport-height with no page-level scrollbar.
- Pane headings, toolbars, and the session line do not scroll with the listing.
- Each local and remote directory listing owns its own scrollbar.
- Local and remote panes remain equal in height.
- The queue panel remains below the dual panes and collapsible.
- Opening a dialog or inspector does not cause the workspace to scroll.

## Accessibility

- Column headers expose `aria-sort` and are keyboard operable.
- Selection state remains exposed through listbox option state.
- The link state is announced through the control label and pressed/expanded state.
- Queue statistics are exposed as readable text, not color-only indicators.
- Permission controls have explicit labels for user, group, world, and action.
- Confirmation dialogs trap focus and restore it on close.
- All new controls have visible focus.
