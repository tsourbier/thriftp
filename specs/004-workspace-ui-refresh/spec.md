# Feature Specification: Workspace UI Refresh and Dual-Pane Connection Hub

**Feature Branch**: `004-workspace-ui-refresh`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "I need to clean the UI and add a few features." Clarified scope: command palette (Cmd/Ctrl+K), breadcrumb path bar, adjustable split panes, a unified searchable sidebar, SFTP private-key discovery from the platform SSH directory with a browse button, an always-visible dual-pane workspace (local on the left, remote on the right, with a favorites and connection-history hub when disconnected), and server favorites that persist both a local and a remote path.

## Clarifications

### Session 2026-11-05

- Q: Where should the unified sidebar sit relative to the two file panes? → A: A collapsible far-left sidebar, before the local pane.
- Q: What should the connection history record? → A: Successful connects only, with one entry per saved server and an updated last-used time.
- Q: Should breadcrumbs replace the always-visible path field, or coexist with it? → A: Replace it; editable breadcrumbs are the default location display.
- Q: How broad should the command palette's action coverage be? → A: Navigation plus common safe actions; destructive and transfer-queue controls stay in their dedicated surfaces.
- Q: When a saved favorite path can no longer be opened, where should that pane go? → A: The nearest existing ancestor, then the home or login directory.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - A clean, always-dual-pane workspace (Priority: P1)

A user opens Thriftp and immediately sees a coherent workspace: the local file system on the left and the remote file system on the right, with consistent styling and without panels competing for attention. The two panes can be resized and the chosen layout is remembered.

**Why this priority**: The dual-pane workspace is the core product surface. Cleaning up the layout and making it predictable is the foundation every other improvement builds on.

**Independent Test**: Launch the application with and without an active connection, resize the pane divider and sidebar, switch themes, restart, and confirm both panes remain usable and the layout is restored.

**Acceptance Scenarios**:

1. **Given** the application is running, **when** the user views the workspace, **then** the local file system is always displayed in the left pane and the remote destination or connection hub is always displayed in the right pane, with the unified sidebar docked to the far left of both panes.
2. **Given** no remote connection is active, **when** the workspace loads, **then** the left pane still shows local files and the right pane shows the connection hub instead of an empty or unusable listing.
3. **Given** an active remote connection, **when** the user browses either pane, **then** the left pane remains local and the right pane remains remote; the panes never swap roles.
4. **Given** the pane divider, **when** the user drags it, **then** both panes resize within sensible minimum and maximum widths and the new proportions are saved.
5. **Given** a saved layout, **when** the application restarts, **then** the pane proportions, sidebar collapsed state, and last connection-hub tab are restored.
6. **Given** the user collapses the sidebar, **when** the window is narrow, **then** the sidebar collapses before either file pane is hidden or compressed below its minimum usable width.
7. **Given** the user switches between system, light, and dark themes, **when** the theme changes, **then** all refreshed surfaces update immediately and preserve readable contrast.
8. **Given** the user navigates the workspace by keyboard only, **when** focus moves through toolbars, panes, sidebar, and dialogs, **then** every interactive control is reachable and shows a visible focus indicator.

---

### User Story 2 - Connect or reconnect from the right-pane hub (Priority: P1)

A user without an active connection uses the right pane as a connection hub. They can choose a saved server from Favorites, review recent connections in History, or create a new connection without leaving the workspace.

**Why this priority**: The right pane is otherwise wasted when disconnected. Turning it into the primary reconnection surface makes the always-dual-pane layout useful from the first moment.

**Independent Test**: Start with no connection, open the hub, browse Favorites and History, connect to a saved server, disconnect, and confirm the hub returns with the previously selected tab.

**Acceptance Scenarios**:

1. **Given** no active connection, **when** the right pane shows the hub, **then** the hub offers a Favorites tab and a History tab.
2. **Given** the Favorites tab, **when** the user searches or selects a saved server, **then** the matching favorites are shown and the selected server can be connected, edited, deleted, or pinned.
3. **Given** the Favorites tab with no saved servers, **when** it is displayed, **then** an empty state explains how to create the first server favorite.
4. **Given** the History tab, **when** the user views it, **then** recent successful connections appear newest first, one entry per saved server, with their display name, protocol, host, last-used time, and a connect action.
5. **Given** the History tab with no entries, **when** it is displayed, **then** an empty state explains that recent successful connections will appear after the first connection.
6. **Given** a connection chosen from the hub, **when** it succeeds, **then** the right pane switches to the remote file listing and the hub state is retained for the next disconnect.
7. **Given** an active connection, **when** the user disconnects, **then** the right pane returns to the hub with the last selected tab.
8. **Given** history entries, **when** the user clears history, **then** the entries are removed after confirmation without deleting saved servers, favorites, or credentials.
9. **Given** active transfers while the user is disconnected, **when** the hub is shown, **then** transfer progress remains visible and unaffected.

---

### User Story 3 - Find anything with the command palette (Priority: P1)

A user presses Cmd/Ctrl+K and searches for a saved server, a directory favorite, an open tab, or a common safe action. Results appear instantly and can be run with the keyboard.

**Why this priority**: A polished desktop client must support fast, keyboard-driven navigation. The palette turns a growing set of panels and actions into one predictable entry point.

**Independent Test**: Open the palette with no connection and with several saved servers and tabs, search by partial name, run a connection, switch a tab, open Preferences, and dismiss with Escape.

**Acceptance Scenarios**:

1. **Given** any primary workspace state, **when** the user presses Cmd+K on macOS or Ctrl+K elsewhere, **then** the command palette opens with focus in the search field.
2. **Given** an open palette, **when** the user types a partial server name, host, favorite name, or action label, **then** matching results are grouped by category and update as the user types.
3. **Given** matching results, **when** the user presses the arrow keys and Enter, **then** the highlighted result runs and the palette closes.
4. **Given** the palette is open, **when** the user presses Escape, **then** the palette closes without running an action and focus returns to where it was.
5. **Given** a saved server result, **when** it is run, **then** the user connects to that server using the existing connection and credential behavior.
6. **Given** a directory favorite result, **when** it is run, **then** the corresponding pane navigates to that path when its server is connected, or the user is told what is required.
7. **Given** an open tab result, **when** it is run, **then** that tab becomes active.
8. **Given** an action result that is unavailable in the current context, **when** the results are shown, **then** the action is visibly disabled with a reason rather than failing silently.
9. **Given** a destructive action or a transfer-queue control, **when** the palette is open, **then** it does not appear as a palette result and remains available only from its dedicated surface.
10. **Given** no search text, **when** the palette opens, **then** it shows recent or suggested destinations and actions rather than an empty list.
11. **Given** no result matches, **when** the user finishes typing, **then** a clear empty state appears and the query remains editable.

---

### User Story 4 - Save and restore local and remote paths in a favorite (Priority: P1)

A user saves a server favorite together with the local folder and remote folder they were working in. Reconnecting restores both folders so they can resume work immediately.

**Why this priority**: Favorites currently restore the server but not the working context. Persisting both paths removes repeated navigation and makes favorites genuinely useful for recurring workflows.

**Independent Test**: From an active session in non-default folders, save a favorite, verify both path fields are pre-filled, disconnect, reconnect from the favorite, and confirm both panes return to the saved folders.

**Acceptance Scenarios**:

1. **Given** an active connection with the local pane in one folder and the remote pane in another, **when** the user saves the server as a favorite, **then** both current paths are pre-filled in the favorite form.
2. **Given** a favorite form, **when** the user edits, clears, or leaves a path blank, **then** each path is optional and the favorite can still be saved.
3. **Given** a favorite with both paths, **when** the user connects, **then** the local pane opens the saved local path and the remote pane opens the saved remote path.
4. **Given** a favorite whose saved remote path no longer exists or is inaccessible, **when** the user connects, **then** the connection succeeds at the nearest accessible ancestor folder, falls back to the server's default login folder if no ancestor is accessible, and shows a non-blocking notice explaining the fallback.
5. **Given** a favorite whose saved local path no longer exists or is inaccessible, **when** the user connects, **then** the remote connection still succeeds, the local pane opens the nearest accessible ancestor folder or the local home folder, and a non-blocking notice explains the fallback.
6. **Given** a favorite with only one path saved, **when** the user connects, **then** the saved path is restored and the other pane uses its existing default behavior.
7. **Given** a favorite with saved paths, **when** the user edits and saves new paths, **then** future connections use the updated paths without affecting the active connection.
8. **Given** a favorite with saved paths, **when** the user views it in the favorites list, **then** the stored remote path is visible without opening the editor.

---

### User Story 5 - Discover and browse SSH private keys for SFTP (Priority: P1)

A user configuring SFTP key authentication is guided to their existing private keys. The platform SSH directory is used by default, discovered keys are offered as choices, and a key button opens a file browser at that location.

**Why this priority**: Key-based SFTP is common and error-prone when users must remember file paths. Secure, obvious key discovery reduces failed connections without weakening credential handling.

**Independent Test**: Configure an SFTP connection on a machine with keys in the platform SSH directory and on one without it; confirm discovered keys are suggested, the browse button opens the correct folder, manual entry still works, and no key contents appear in the interface or logs.

**Acceptance Scenarios**:

1. **Given** the user selects SSH private-key authentication for SFTP, **when** the key field is shown, **then** the default location is the platform SSH directory (`~/.ssh` on macOS and Linux, the user profile `.ssh` directory on Windows).
2. **Given** conventional private keys exist in that directory, **when** the key field is shown, **then** the discovered keys are listed as selectable suggestions by file name.
3. **Given** exactly one conventional private key is discovered, **when** the user opens the key field, **then** that key is pre-filled as the suggested reference.
4. **Given** multiple conventional private keys are discovered, **when** the user opens the key field, **then** no key is chosen silently and the user must pick one or browse.
5. **Given** a key field, **when** the user activates the key icon button (🔑) beside it, **then** the native file chooser opens in the platform SSH directory and the chosen file path fills the key field after confirmation.
6. **Given** the native file chooser is unavailable, **when** the user activates the key button, **then** the user can still type or paste a key path manually and receives clear guidance.
7. **Given** a key path that does not exist, is unreadable, or is not a supported private key, **when** the user saves or tests the connection, **then** an actionable message identifies the problem without revealing key contents.
8. **Given** a discovered key, **when** it is suggested, **then** the application does not authenticate with it until the user explicitly selects key authentication and confirms the connection.
9. **Given** any key flow, **when** the user reviews the interface, logs, and saved configuration, **then** no private-key contents or passphrases are displayed or stored in plaintext.

---

### User Story 6 - Navigate with breadcrumbs in both panes (Priority: P2)

A user sees and navigates the full location path as clickable breadcrumbs in both panes, and can switch to typing a full path when that is faster.

**Why this priority**: Breadcrumbs make location obvious at a glance and reduce reliance on typing full paths, completing the navigation cleanup.

**Independent Test**: Navigate deep into both local and remote trees, click a breadcrumb segment, switch to editable mode, type a valid and invalid path, and confirm correct navigation and errors.

**Acceptance Scenarios**:

1. **Given** either pane at any folder, **when** the breadcrumb bar is displayed, **then** it shows the path from the root to the current folder as ordered segments.
2. **Given** a breadcrumb segment, **when** the user activates it, **then** that pane navigates to that ancestor folder.
3. **Given** a breadcrumb bar, **when** the user clicks the current folder or uses the edit shortcut, **then** the breadcrumbs switch to a selected text field, Enter navigates, and Escape cancels.
4. **Given** a typed path that is invalid or inaccessible, **when** the user submits it, **then** the pane stays at its current folder and an actionable message is shown.
5. **Given** a path too long for the available width, **when** the breadcrumb is displayed, **then** middle segments collapse into an overflow control while the root and current folder remain visible.
6. **Given** keyboard or screen-reader navigation, **when** the user moves through breadcrumbs, **then** segments are reachable, labeled, and expose their full path.
7. **Given** the remote pane is showing the connection hub, **when** breadcrumbs would otherwise appear, **then** they are hidden or disabled because there is no remote location yet.

---

### User Story 7 - Use one searchable sidebar for saved servers, directory favorites, and automation (Priority: P2)

A user opens a single sidebar to reach saved servers, directory favorites, and automation status, and filters all of them from one search field.

**Why this priority**: Consolidating navigation removes duplicate panels and gives the workspace a single, predictable place for secondary destinations.

**Independent Test**: Populate saved servers and directory favorites, search from the sidebar, collapse and expand sections, open the automation panel, restart, and confirm sidebar state and filtering behavior.

**Acceptance Scenarios**:

1. **Given** the sidebar, **when** the user views it, **then** it presents separate sections for saved servers, directory favorites, and automation status.
2. **Given** the sidebar search field, **when** the user types, **then** saved servers and directory favorites across all sections are filtered by name, host, protocol, group, or path.
3. **Given** a search with no matches, **when** results are shown, **then** a clear empty state appears and the search can be cleared in one action.
4. **Given** a section header, **when** the user collapses or expands it, **then** the section state is remembered across restarts.
5. **Given** the automation section, **when** the user opens it, **then** automation status and controls are available without occupying a permanent area of the main workspace.
6. **Given** a saved server in the sidebar, **when** the user activates it, **then** it connects or focuses the existing connection using the same behavior as the favorites hub.
7. **Given** a directory favorite in the sidebar, **when** the user activates it, **then** the appropriate pane navigates to that path, subject to the same connection requirements as elsewhere.
8. **Given** a narrow window, **when** the sidebar is collapsed, **then** a clearly labeled control reopens it and the search remains available.

---

### Edge Cases

- No saved servers, no directory favorites, and no history: every empty surface explains the next action instead of showing blank space.
- A saved remote path points to a deleted folder, a file, a symlink loop, or a location the account cannot read: connection succeeds, the pane opens the nearest accessible ancestor or the server's login folder, and the user sees one actionable notice.
- A saved local path is on a removed volume, external drive, or protected folder: the remote session is not blocked, the local pane opens the nearest accessible ancestor or the local home folder, and the fallback is explained.
- A favorite's local or remote path contains characters that require escaping or is extremely long: it is stored and displayed without corruption, and breadcrumbs truncate gracefully.
- The pane divider is dragged to either extreme or the window is resized below the preferred minimum: both panes retain a usable minimum width, and the sidebar collapses before a pane does.
- The window is resized while a modal dialog or the command palette is open: the dialog remains usable and focus is not lost.
- The command palette is opened while another modal dialog is active: the palette does not stack on top of the dialog, or it closes the dialog only after explicit user intent.
- A palette query matches many duplicate server names: results show enough connection detail to distinguish them.
- A palette action becomes unavailable while the palette is open (for example, the last connection is closed): the result updates or fails with a clear explanation.
- The platform SSH directory does not exist, is empty, or is not readable: the user sees guidance, can browse elsewhere, and can still enter a path manually.
- The SSH directory contains public keys, config files, agent sockets, or non-key files: only supported private-key candidates are suggested.
- Multiple keys exist with the same file name in different locations: the full path is available before selection.
- A selected private key is encrypted or requires a passphrase: the existing passphrase behavior applies and no passphrase is displayed.
- Connection history becomes large or contains an entry for a server that was later deleted: history remains usable, is bounded, and connections to missing servers fail with a clear explanation.
- The user clears history while connected: the active connection and saved favorites are unaffected.
- A theme change occurs while a dialog or the palette is open: contrast and focus indicators remain correct.
- The user relies on reduced motion or a screen reader: animations are suppressed and all state changes are announced where appropriate.

## Requirements *(mandatory)*

### Functional Requirements

**Workspace shell and visual cleanup**

- **FR-001**: The workspace MUST always present two file panes side by side: local on the left and remote on the right. Neither pane may be hidden in favor of the other.
- **FR-002**: When no remote connection exists, the left pane MUST remain a usable local browser and the right pane MUST present the connection hub.
- **FR-003**: The pane divider MUST be draggable, MUST enforce a usable minimum width for both panes, and MUST provide a keyboard-accessible way to adjust or reset the split.
- **FR-004**: The application MUST persist the pane split, sidebar state, and last connection-hub tab across restarts.
- **FR-005**: The interface MUST use a shared visual system for color, spacing, typography, borders, radii, elevation, and focus treatment across every refreshed surface.
- **FR-006**: Refreshed surfaces MUST adapt to system, light, and dark themes without hard-coded colors that break contrast.
- **FR-007**: Primary, secondary, destructive, disabled, loading, and validation states MUST be visually distinct and consistently applied to buttons, fields, menus, lists, and dialogs.
- **FR-008**: The layout MUST keep primary actions reachable without horizontal scrolling at the supported minimum window size; the sidebar MUST collapse before either file pane is compressed below its minimum width.
- **FR-009**: Notices and errors MUST be non-blocking where possible, MUST identify the affected item or path, and MUST NOT expose secrets or raw internal details.

**Connection hub and history**

- **FR-010**: The disconnected right pane MUST offer a Favorites tab and a History tab.
- **FR-011**: The Favorites tab MUST show saved servers with enough detail to distinguish them and MUST support search, connect, create, edit, delete, and pin actions.
- **FR-012**: The History tab MUST show recent successful connections newest first, with at most one entry per saved server, including display name, protocol, host, last-used time, and a connect action. Failed attempts MUST NOT be retained in history.
- **FR-013**: Connection history MUST be stored locally, MUST be bounded to a reasonable recent set, MUST contain at most one entry per saved server, MUST NOT contain credentials, passphrases, key contents, file data, or failure details, and MUST be clearable after confirmation.
- **FR-014**: A successful connection from the hub MUST replace the hub with the remote listing in the right pane; disconnecting MUST return to the hub and preserve the last selected tab.
- **FR-015**: Active transfers MUST remain visible and controllable while the hub is displayed.

**Command palette**

- **FR-016**: Users MUST be able to open a command palette from anywhere in the primary workspace with Cmd+K on macOS and Ctrl+K on other supported platforms.
- **FR-017**: The palette MUST search saved servers, server favorites, directory favorites, open tabs, and common safe application actions: connect, create a connection or favorite, open Preferences, refresh, and toggle hidden files.
- **FR-018**: Results MUST update as the user types, MUST be grouped by category, and MUST support arrow-key navigation, Enter to run, and Escape to dismiss.
- **FR-019**: Results MUST show enough context to distinguish duplicates, such as protocol, host, username, tab title, or favorite path.
- **FR-020**: Actions that are unavailable in the current context MUST be shown as disabled with a reason. Destructive actions and transfer-queue controls MUST NOT be offered in the palette and MUST remain in their dedicated surfaces.
- **FR-021**: Dismissing the palette MUST return focus to the previously focused control.
- **FR-022**: An empty query MUST show recent or suggested destinations and actions, and a query with no matches MUST show a clear empty state.

**Favorites with local and remote paths**

- **FR-023**: A server favorite MUST be able to store an optional local path, an optional remote path, or both.
- **FR-024**: When a favorite is created from an active connection, the form MUST pre-fill the current local and remote pane paths.
- **FR-025**: The favorite editor MUST allow both paths to be viewed, edited, cleared, and left blank without preventing the favorite from being saved.
- **FR-026**: Connecting to a favorite MUST restore each saved path in its corresponding pane.
- **FR-027**: If a saved path cannot be restored, the connection MUST still succeed by opening the nearest accessible ancestor folder, falling back to the home or login directory when no ancestor is accessible. The pane MUST remain usable, and a single actionable notice MUST explain the fallback.
- **FR-028**: Updating a favorite's paths MUST NOT change the active connection and MUST apply to subsequent connections.
- **FR-029**: The favorites list and hub MUST expose the saved remote path so users can identify the working location without opening the editor.

**SFTP key discovery and selection**

- **FR-030**: For SFTP key authentication, the key selector MUST default to the platform SSH directory: `~/.ssh` on macOS and Linux, and the equivalent user-profile SSH directory on Windows.
- **FR-031**: The application MUST detect supported conventional private keys in that directory and present them as selectable suggestions by file name.
- **FR-032**: The application MUST pre-fill a discovered key only when exactly one supported private key is found; otherwise it MUST NOT choose a key without explicit user selection.
- **FR-033**: A small key icon button (🔑) MUST appear beside the private-key field and MUST open the native file chooser at the platform SSH directory; the chosen path MUST populate the field.
- **FR-034**: Manual entry and pasting of a key path MUST remain supported when the native file chooser is unavailable or the user prefers it.
- **FR-035**: The application MUST validate the selected key path and MUST report missing, unreadable, and unsupported files with actionable messages that do not reveal key contents.
- **FR-036**: The application MUST NOT authenticate with a discovered key unless the user explicitly selects key authentication and confirms the connection.
- **FR-037**: Private-key contents and passphrases MUST NOT appear in the interface, logs, history, error messages, or saved configuration beyond the existing secure credential handling.

**Breadcrumbs**

- **FR-038**: Both the local and remote panes MUST display breadcrumbs from the root to the current folder whenever a location is available, and the breadcrumb bar MUST replace the existing always-visible path field as the default location display.
- **FR-039**: Activating a breadcrumb segment MUST navigate that pane to the corresponding ancestor folder.
- **FR-040**: Each breadcrumb bar MUST provide an editable path mode, entered by clicking the current folder or using a keyboard shortcut, with Enter to navigate and Escape to cancel.
- **FR-041**: Invalid or inaccessible paths submitted through breadcrumbs MUST leave the pane at its current folder and show an actionable message.
- **FR-042**: Long paths MUST collapse middle segments into an overflow control while preserving access to every segment through keyboard and pointer interaction.
- **FR-043**: Breadcrumbs MUST expose full paths to assistive technology and MUST be hidden or disabled when the pane has no location, such as the disconnected hub.

**Unified sidebar**

- **FR-044**: A single collapsible sidebar MUST be docked at the far left of the workspace, before the local pane, and MUST present saved servers, directory favorites, and automation status in clearly labeled sections.
- **FR-045**: The sidebar MUST provide a search field that filters saved servers and directory favorites by name, host, protocol, group, and path.
- **FR-046**: Section collapsed states and sidebar visibility MUST persist across restarts.
- **FR-047**: Automation status and controls MUST be reachable from the sidebar without occupying a permanent area of the main workspace.
- **FR-048**: Sidebar entries MUST perform the same connect, navigate, edit, and delete behavior as their equivalents elsewhere, including all confirmation and security rules.
- **FR-049**: The sidebar MUST provide empty states and a one-action way to clear an active search.

**Compatibility, security, and accessibility**

- **FR-050**: Existing connection, transfer, credential, host-key, and plain-FTP warning behavior MUST remain unchanged unless this specification explicitly extends it.
- **FR-051**: Existing favorites, tabs, preferences, and history MUST load without data loss; favorites without paths MUST behave as they did before this feature.
- **FR-052**: This feature MUST NOT change the behavior, output, or security rules of the command-line or MCP interfaces.
- **FR-053**: All new and refreshed controls MUST be operable by keyboard, MUST have accessible names, and MUST show a visible focus indicator.
- **FR-054**: Dialogs and the command palette MUST trap focus while open, restore focus on close, and respect reduced-motion preferences.
- **FR-055**: The refreshed interface MUST NOT introduce mandatory telemetry, cloud services, or new secret storage outside the operating system credential store.

### Key Entities *(include if data involved)*

- **Workspace Layout**: The remembered presentation of the workspace. Attributes include pane split ratio, sidebar visibility, collapsed sidebar sections, and the last selected connection-hub tab.
- **Server Favorite (extended)**: An existing saved server definition extended with an optional local path and an optional remote path. Credentials remain referenced through the operating system credential store and are not part of this entity.
- **Connection History Entry**: A local record of a recent successful connection. There is at most one entry per saved server. Attributes include the server display name, protocol, host, port, username, source favorite if any, and last-used time. It contains no credentials, passphrases, key contents, file data, or failure details.
- **Discovered SSH Key**: A supported private-key file found in the platform SSH directory. Attributes include display file name, full path, and whether it was selected by the user. Key contents are never part of this entity.
- **Command Palette Entry**: A searchable destination or common safe action. Attributes include label, category, contextual availability, optional shortcut, and the existing behavior it invokes. Destructive and transfer-queue actions are excluded.
- **Navigation Location**: The current folder shown in a pane, represented by an ordered breadcrumb path and an editable path value.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can start the application with no connection and identify the local pane, remote connection hub, Favorites tab, and History tab without documentation in at least 90% of moderated first-use attempts.
- **SC-002**: A returning user can connect to a saved server from the disconnected workspace in no more than two primary actions or five seconds.
- **SC-003**: A user can find and run a saved server, tab, or common safe action through the command palette in ten seconds or less, using the keyboard only.
- **SC-004**: Both file panes remain visible and usable at the supported minimum window size, with no horizontal page scrolling.
- **SC-005**: The pane split, sidebar visibility, and hub tab are restored correctly in 100% of normal restarts during testing.
- **SC-006**: Search results in the command palette and sidebar update within 100 milliseconds for a dataset of 200 saved servers and directory favorites.
- **SC-007**: Every primary workflow covered by this specification can be completed with the keyboard alone, with visible focus at every step.
- **SC-008**: All refreshed text and interactive elements meet WCAG 2.1 AA contrast requirements in light and dark themes, verified by automated checks and manual review.
- **SC-009**: 100% of refreshed surfaces use shared design tokens; a review finds no new hard-coded theme-dependent colors.
- **SC-010**: Connecting to a favorite with valid saved local and remote paths restores both paths in 100% of test cases; invalid paths fall back in 100% of test cases with exactly one actionable notice.
- **SC-011**: On a machine with conventional keys in the platform SSH directory, the key selector surfaces the available keys within one second of choosing key authentication.
- **SC-012**: Activating the key button opens the native file chooser in the platform SSH directory in 100% of supported-platform test runs where a native chooser is available.
- **SC-013**: Automated review finds zero private-key contents, passphrases, or credentials in connection history, logs, or rendered interface text.
- **SC-014**: Existing automated test suites continue to pass, and new automated coverage verifies the workspace layout, hub tabs, palette actions, favorite path restoration, key discovery, breadcrumbs, and sidebar search.
- **SC-015**: Users can clear their connection history in one confirmed action; history contains at most one entry per saved server and never exceeds the configured recent-entry limit.

## Assumptions

- Because the cleanup depth was not explicitly answered, this specification assumes the recommended full layout refresh: reorganizing the shell and applying a shared visual system while preserving existing behavior.
- The selected secondary features are the command palette, breadcrumb path bar, adjustable split panes, and unified searchable sidebar; features not listed here remain unchanged except for visual consistency.
- The unified sidebar and the disconnected right-pane hub coexist: the sidebar provides quick access while working, and the hub is the focused startup and reconnection surface.
- Connection history records successful connections only, keeps one entry per saved server, is local-only, capped at the 50 most recent servers, and is cleared only through an explicit confirmed action. It never contains credentials, secrets, failure details, or file data and is not synchronized or transmitted.
- The platform SSH directory is `~/.ssh` on macOS and Linux and the user profile's `.ssh` directory on Windows. Conventional names such as common RSA, ECDSA, and Ed25519 private keys are considered; public keys, configuration files, and unrelated files are ignored.
- Key discovery is a convenience only. It never generates keys, never uses an SSH agent, and never authenticates without explicit user selection.
- Saved paths are absolute locations. Relative paths, path expansion, and remote shell expansion are out of scope for this feature.
- When a favorite has no saved paths, its behavior matches the pre-existing default: the local pane uses its current or default local location and the remote pane uses the server's default login location. When a saved path is invalid, the pane opens the nearest accessible ancestor and falls back to home or login only if no ancestor is accessible.
- The existing always-visible path field is replaced by breadcrumbs with an editable mode; direct path entry remains available through that mode.
- The supported minimum window size is 900 by 600 logical pixels. Both file panes retain a minimum usable width, and the sidebar collapses first on narrower windows.
- Existing theme choices remain system, light, and dark; no new theme is introduced.
- Command palette actions are limited to navigation and common safe actions that already exist in the desktop interface; destructive actions and transfer-queue controls remain in their dedicated surfaces. The palette adds discovery and speed, not new transfer or protocol behavior.
- The desktop application remains free of mandatory telemetry, accounts, and cloud dependencies. The command-line and MCP interfaces are out of scope.
