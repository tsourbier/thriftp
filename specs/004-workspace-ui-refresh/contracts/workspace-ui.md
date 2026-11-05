# Contract: Workspace UI Behavior

**Feature**: `004-workspace-ui-refresh` | **Date**: 2026-11-05

This contract defines the user-facing behavior of the refreshed workspace. It is implementation-agnostic about component internals but fixes the layout invariants, keyboard map, focus rules, and visible states that automated tests and manual validation rely on.

## Shell Layout

At all times in the primary workspace:

```text
┌──────────────────────────────────────────────────────────────────────┐
│ Title bar: product name · Preferences · New favorite · New connection│
├───────────────┬────────────────────────┬─────────────────────────────┤
│ Unified       │ Local pane             │ Right pane                  │
│ sidebar       │ (always local)         │ (remote listing OR hub)     │
│ (collapsible) │                        │                             │
│               │                        │                             │
└───────────────┴────────────────────────┴─────────────────────────────┘
│ Transfer queue (existing, collapsible surface)                      │
└──────────────────────────────────────────────────────────────────────┘
```

Invariants:

1. The local pane is always the left file pane and always browses local files.
2. The right pane is always the right file pane. It shows the remote listing when a connection is active and the connection hub when not.
3. The unified sidebar is docked at the far left, before the local pane, and is collapsible.
4. Neither file pane may be hidden or moved to the other side.
5. The supported minimum window is 900 × 600 logical pixels. Each file pane keeps a usable minimum width of 320 logical pixels. The sidebar collapses before either pane is compressed below that minimum.
6. The transfer queue remains reachable and visibly updates while the hub is displayed; it is not part of the palette.

## Title Bar and Primary Actions

- Shows the product name.
- Provides Preferences, New favorite, and New connection actions.
- Primary actions remain reachable without horizontal scrolling at the minimum window size.
- The command palette is the keyboard-first equivalent of these actions; it does not replace the visible controls for pointer users.

## Unified Sidebar

- Sections, in order: **Saved Servers**, **Directory Favorites**, **Automation**.
- A single search field filters saved servers and directory favorites by name, host, protocol, group, or path. Automation status is not filtered by the server/favorite query.
- Each section header toggles collapse/expand and exposes `aria-expanded`.
- The sidebar header exposes a labeled collapse control; when collapsed, a labeled reopen control remains visible.
- Search with no matches shows an empty state and a one-action clear-search control.
- Saved-server activation connects or focuses the existing connection using the same behavior as the hub.
- Directory-favorite activation navigates the appropriate pane, subject to the same connection requirement as elsewhere.
- Automation status and controls open from the Automation section; they do not occupy a permanent workspace area.
- Empty sections explain the next action.
- At narrow widths, the effective sidebar collapses without overwriting the persisted `sidebarVisible` preference. Reopening it at a narrow width uses an overlay so neither pane drops below its minimum.

## Pane Split

- The divider is a focusable `role="separator"` with `aria-orientation="horizontal"`, an accessible name, and `aria-valuemin`/`aria-valuemax`/`aria-valuenow` expressed as percentages.
- Pointer drag resizes both panes within the minimums. The layout commits on pointer release.
- Keyboard on the separator:
  - `ArrowLeft` / `ArrowRight`: adjust the split by 2%.
  - `Home` / `End`: move to the corresponding limit.
  - `Enter`: reset to 50/50.
- A visible reset control is also available.
- The committed ratio is restored on restart.
- Resizing never triggers a directory listing reload or a transfer operation.

## Connection Hub (Disconnected Right Pane)

- Tabs: **Favorites** and **History**; the last selected tab is restored.
- Favorites tab:
  - Search field filters by name, host, protocol, group, and stored remote path.
  - Each row shows enough detail to distinguish duplicates: display name, protocol, host, and username; the saved remote path is visible without opening the editor.
  - Actions: connect/focus, edit, delete with confirmation, pin/unpin, create.
  - Empty state explains how to create the first server favorite.
- History tab:
  - Newest first, at most one entry per saved server, capped at 50.
  - Each row shows display name, protocol, host, last-used time, and a connect action.
  - Empty state explains that recent successful connections appear after the first connection.
  - Clear history requires confirmation; cancelling leaves history unchanged.
  - A connect action for a deleted server fails with a clear non-blocking message and does not remove the history entry.
- A successful connection replaces the hub with the remote listing. Disconnecting returns to the hub with the last selected tab.
- Active transfers remain visible and controllable while the hub is shown.

## Breadcrumbs

- Both panes show breadcrumbs from the root to the current folder whenever a location exists.
- Breadcrumbs replace the always-visible path input as the default location display.
- Activating a segment navigates that pane to the segment's folder.
- Activating the current folder, or pressing `Ctrl/Cmd+L`, enters editable mode with the current path selected.
- In editable mode: `Enter` navigates; `Escape` cancels and restores the breadcrumb display.
- An invalid or inaccessible submitted path leaves the pane at its current folder and shows one actionable notice.
- Long paths collapse middle segments into an overflow control; root and current remain visible. Every segment remains reachable by keyboard and pointer and exposes its full path via `aria-label`/`title`.
- The disconnected hub has no remote location, so its breadcrumb bar is hidden.
- Back, forward, up, refresh, sort, and filter controls remain in the pane toolbar.

## Command Palette

### Opening and closing

- `Cmd+K` on macOS and `Ctrl+K` on Linux/Windows open the palette from anywhere in the primary workspace.
- The palette does not open while another modal dialog is active.
- `Escape` closes without running an action and restores focus to the previously focused control.
- Running an entry closes the palette and restores focus in the same way.
- The dialog traps focus while open and has `role="dialog"` and `aria-modal="true"`.

### Search and results

- Focus starts in the search field.
- Results update as the user types.
- Results are grouped by category: Servers, Favorites, Tabs, Actions.
- Duplicate names are distinguishable by subtitle (protocol, host, username, tab title, or favorite path).
- Empty query shows recent history entries and suggested safe actions.
- No matches shows a clear empty state while the query remains editable.
- `ArrowUp`/`ArrowDown` move the highlight; `Enter` runs the highlighted entry.
- Unavailable entries are visible, disabled, and show a reason.

### Allowed actions

- Connect to a saved server.
- Navigate to a directory favorite when its server is connected, or explain what is required.
- Activate an open tab.
- Create a connection.
- Create a favorite.
- Open Preferences.
- Refresh the active pane.
- Toggle hidden files.

### Excluded actions

- Delete, remove credential, clear history, cancel-all, and every other destructive action.
- Transfer-queue controls: pause, resume, cancel, reorder, priority, bandwidth, concurrency.
- These remain only in their dedicated surfaces.

## Keyboard Map (Workspace)

| Shortcut | Context | Action |
|----------|---------|--------|
| `Cmd/Ctrl+K` | Primary workspace | Open command palette |
| `Escape` | Palette/dialog | Close and restore focus |
| `ArrowUp`/`ArrowDown` | Palette | Move highlighted result |
| `Enter` | Palette | Run highlighted result |
| `ArrowLeft`/`ArrowRight` | Split separator | Adjust split by 2% |
| `Home`/`End` | Split separator | Move split to a limit |
| `Enter` | Split separator | Reset split to 50/50 |
| `Ctrl/Cmd+L` | File pane | Edit breadcrumb path |
| `Enter` | Breadcrumb edit | Navigate |
| `Escape` | Breadcrumb edit | Cancel editing |
| `Cmd/Ctrl+H` | File pane | Toggle hidden files (existing) |
| `F5` | File pane | Refresh (existing) |
| `Arrow keys` | File list | Move selection (existing) |
| `Enter` | File list | Open (existing) |
| `Backspace`/`Alt+Left` | File list | Go up (existing) |
| `Delete` | File list | Delete with existing confirmation (existing) |

## Focus and Accessibility

- Every interactive control is reachable by keyboard and has an accessible name.
- Visible focus uses a shared focus-ring token and is never removed without an equivalent indicator.
- Icon-only controls have `aria-label`s.
- Modal dialogs and the palette trap focus, restore focus on close, and do not stack.
- Status and fallback notices use `role="status"` or `aria-live="polite"`; errors use `role="alert"`.
- Breadcrumb segments are labeled with their full path.
- The split separator exposes its current value.
- Section headers expose `aria-expanded` and `aria-controls`.
- Reduced-motion preference suppresses non-essential transitions and animations.
- Theme changes apply immediately without losing focus or dialog state.

## Visual States

Every refreshed surface uses shared tokens and visibly distinct:

- Primary, secondary, and destructive buttons.
- Disabled controls with a reason where relevant.
- Loading/list progress states.
- Validation errors associated with the field.
- Empty states for no favorites, no history, no search matches, and no automation clients.
- Selected, active, hover, and focus states for rows and tabs.
- Warning/notice/error messages with readable contrast in system, light, and dark themes.

No refreshed surface introduces a hard-coded theme-dependent color. All text and interactive elements meet WCAG 2.1 AA contrast.

## Non-Blocking Notices

- Fallback path notices identify the requested path and the resolved path, explain why, and can be dismissed.
- At most one fallback notice is shown per pane per navigation.
- Connection, transfer, and credential errors retain their existing dedicated prompts and messages.
- Notices never include credentials, key contents, raw protocol payloads, or internal paths beyond the user-visible file path.

## Acceptance Mapping

| Story | Primary UI contract |
|-------|---------------------|
| 1. Dual-pane workspace | Shell layout invariants, pane split, theme/focus states |
| 2. Connection hub | Hub tabs, history rules, disconnect/connect transition |
| 3. Command palette | Open/close, search groups, keyboard, allowed/excluded actions |
| 4. Favorite paths | Favorite list path visibility, form fields, connect restoration, fallback notice |
| 5. SSH key discovery | Key field suggestions, exactly-one prefill, browse button, manual fallback, validation messages |
| 6. Breadcrumbs | Display/edit modes, overflow, invalid path behavior, hidden on hub |
| 7. Unified sidebar | Sections, search, collapse persistence, automation access, narrow behavior |
