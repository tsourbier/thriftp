# Feature Specification: Browser and Favorites Workflow Polish

**Feature Branch**: `006-browser-workflow-polish`

**Created**: 2026-11-06

**Status**: Draft

**Input**: User description: "I need to introduce a number of UI changes / cleanup to start with: queue statistics per connection, Shift/Cmd multi-selection, favorite dialog local-path browsing, new favorite failures with a remote path, favorite dialog height, Disconnect placement, linked pane navigation, detailed file permissions with recursive apply and path copy, removal of back/forward buttons, and sortable column headers."

## Clarifications

### Session 2026-11-06

- Q: Should per-connection transfer statistics reset when a connection ends, or persist across reconnects and app restarts? → A: Reset per session.
- Q: Which navigation actions should linked panes mirror? → A: All navigation actions.
- Q: When a linked pane navigates via a breadcrumb, typed path, history entry, tab, or favorite, how should the other pane choose its destination? → A: Mirror the relative move.
- Q: When applying a directory permission change to enclosed items, how deep should the change go? → A: All descendants, without following symbolic links.
- Q: Should the new permission controls edit local files and folders as well as remote ones? → A: Remote editing only; local permissions are read-only.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Reliable favorite creation and editing (Priority: P1)

A user creates and edits server favorites without losing work. A remote path can be saved on a new favorite, the local path can be chosen with a folder browser instead of typed manually, and the editor always keeps its actions reachable.

**Why this priority**: Favorite creation is currently blocked by a defect and the editor can become unusable on smaller windows. Fixing these issues restores a core workflow before any other polish matters.

**Independent Test**: Create a new favorite with a remote path, browse to a local path, edit the favorite, and complete the flow in a window at the supported minimum size.

**Acceptance Scenarios**:

1. **Given** the new-favorite form, **when** the user supplies a valid remote path and saves, **then** the favorite is created successfully with the remote path stored.
2. **Given** the local-path field, **when** the user activates the adjacent browse button, **then** a native folder chooser opens at the current or default local location and the chosen folder fills the field.
3. **Given** the folder chooser is cancelled, **when** the user returns to the form, **then** the previous local-path value remains unchanged.
4. **Given** the favorite editor at the supported minimum window size, **when** the form content exceeds the available height, **then** the content scrolls within the dialog and the save and cancel actions remain visible and operable.
5. **Given** a validation error in the editor, **when** the error is shown, **then** the error is visible without preventing the user from reaching the form actions.

---

### User Story 2 - Select and sort files efficiently (Priority: P1)

A user selects several files using standard desktop modifier behavior and sorts any listing by clicking column headers.

**Why this priority**: Selection and sorting are fundamental browser interactions used by nearly every transfer workflow.

**Independent Test**: In a listing with many files, select a continuous range with Shift, add and remove individual selections with Cmd on macOS or Ctrl on Windows/Linux, then sort by name, size, and modified date in both directions.

**Acceptance Scenarios**:

1. **Given** no previous selection, **when** the user selects a file without a modifier, **then** only that file is selected.
2. **Given** an anchor file, **when** the user Shift-selects another file, **then** every file between the anchor and the target in the current listing order is selected.
3. **Given** existing selections, **when** the user Cmd-selects on macOS or Ctrl-selects on Windows/Linux, **then** that file is added when unselected or removed when selected, and the other selections are preserved.
4. **Given** a sorted or filtered listing, **when** the user selects a range, **then** the range follows the order currently shown to the user.
5. **Given** a listing, **when** the user activates the Name header, **then** the listing sorts by name and the indicator shows ascending; activating it again shows descending.
6. **Given** a listing, **when** the user activates the Size or Modified header, **then** the listing sorts by that column and toggles between ascending and descending on repeated activation.
7. **Given** selected files, **when** the sort order changes, **then** the same files remain selected.

---

### User Story 3 - Link navigation between local and remote panes (Priority: P2)

A user links the local and remote panes so every navigation action is mirrored through parallel directory structures, not just moving up or opening a folder.

**Why this priority**: Linked navigation removes repetitive work for users whose local and remote trees mirror each other.

**Independent Test**: Link the panes, move up, open matching folders, jump through breadcrumbs, history, typed paths, tabs, and favorites, verify every relative move is mirrored when possible, then unlink and confirm each pane moves independently.

**Acceptance Scenarios**:

1. **Given** an active remote session, **when** the user views the remote session line, **then** it shows a clearly labeled Link or Unlink control reflecting the current linked state.
2. **Given** linked panes, **when** the user moves up one directory in either pane and the other pane can also move up, **then** both panes move up one level.
3. **Given** linked panes, **when** the user opens a directory in one pane and a directory with the same name exists in the other pane's current folder, **then** the other pane opens that same-named directory.
4. **Given** linked panes, **when** the user jumps to an ancestor through breadcrumbs, **then** the other pane performs the same number of parent steps when possible.
5. **Given** linked panes, **when** the user navigates through a typed path, history entry, tab, or favorite, **then** the other pane mirrors the same relative movement when the corresponding location exists, or remains unchanged otherwise.
6. **Given** linked panes, **when** the mirrored location does not exist, cannot be read, or the other pane is disconnected, **then** the originating pane still navigates and the other pane remains usable without showing a blocking error.
7. **Given** linked panes, **when** the user unlinks them, **then** subsequent navigation affects only the pane the user acts on.
8. **Given** the user activates the link control, **when** the state changes, **then** the current folders are not forcibly synchronized; mirroring begins with the next navigation action.

---

### User Story 4 - Understand transfer activity per connection (Priority: P2)

A user sees concise queue statistics for each active connection: how many files transferred, how much data moved, and the average transfer speed.

**Why this priority**: Connection-level statistics make transfer progress and performance understandable without inspecting individual rows.

**Independent Test**: Run several successful, failed, and cancelled transfers across two connections and verify each connection reports its own completed-file count, volume, and average speed.

**Acceptance Scenarios**:

1. **Given** an active connection, **when** the queue panel is visible, **then** it shows that connection's number of successfully transferred files, total transferred volume, and average speed.
2. **Given** a completed transfer, **when** its file finishes, **then** the connection's file count, volume, and average speed update without requiring a refresh.
3. **Given** more than one active connection, **when** transfers complete on each, **then** each connection displays only its own statistics.
4. **Given** a failed or cancelled transfer, **when** statistics are calculated, **then** it is excluded from the completed-file count and average speed, and its partial volume is not counted as a completed transfer.
5. **Given** a connection with no completed transfers, **when** the queue is shown, **then** its statistics display a clear zero or empty state instead of a misleading value.
6. **Given** a connection session ends and the same server is connected again, **when** the queue is shown, **then** the statistics for the new session start at zero.
7. **Given** a volume total, **when** it is displayed, **then** it is presented in an appropriate byte unit that can scale through KB, MB, and GB.

---

### User Story 5 - Inspect and change file permissions safely (Priority: P3)

A user opens file information and sees permissions broken down by owner, group, and world as checkboxes alongside the octal value and symbolic summary. Directories can apply a permission change to their contents after confirmation, and the inspected path can be copied.

**Why this priority**: Permission editing is an advanced workflow. It should be explicit, understandable, and protected against accidental recursive changes.

**Independent Test**: Inspect files and directories with different modes, toggle permissions, apply a single-file change, apply a directory change recursively through confirmation, cancel the confirmation, and copy the path.

**Acceptance Scenarios**:

1. **Given** the information panel for an item with permissions, **when** it opens, **then** it shows User, Group, and World rows, each with Read, Write, and Execute states.
2. **Given** the permission state, **when** it is displayed, **then** the octal value and the symbolic recap such as `rw-r--r--` match the checkboxes.
3. **Given** editable permissions, **when** the user toggles checkboxes, **then** the octal and symbolic summaries update before the change is applied.
4. **Given** a remote single file with supported permission changes, **when** the user applies a modified mode, **then** the stored permissions update and the information panel reflects the new mode.
5. **Given** a remote directory with supported permission changes, **when** the user chooses to apply the mode to enclosed items, **then** a confirmation dialog explains that all descendant folders and files will change and that symbolic links will not be followed.
6. **Given** the enclosed-items confirmation, **when** the user cancels, **then** no permissions are changed.
7. **Given** the enclosed-items confirmation, **when** the user confirms, **then** the selected mode is applied recursively to every descendant folder and file without following symbolic links, and failures are reported without hiding successful changes.
8. **Given** a remote server or item that cannot change permissions, **when** the panel is shown, **then** the permission controls are disabled and an explanation is provided.
9. **Given** the information panel for a remote item, **when** the user activates the copy-path control, **then** the remote path is copied to the system clipboard and an inline “Path copied” confirmation remains visible for at least two seconds.
10. **Given** a local item, **when** the information panel is shown, **then** the detailed permission display is available in read-only form where the platform exposes it and no permission-editing or recursive-apply controls are offered.

---

### User Story 6 - Keep remote session controls in the right place (Priority: P3)

A user sees the disconnect action beside the remote pane title and works with a simpler toolbar that no longer duplicates history navigation.

**Why this priority**: These are focused layout cleanups that reduce visual clutter after the core workflows are reliable.

**Independent Test**: Connect to a server, disconnect from the remote pane header, and verify both pane toolbars no longer show back or forward controls.

**Acceptance Scenarios**:

1. **Given** an active remote session, **when** the remote pane header is shown, **then** the Disconnect control appears next to the remote title and server name.
2. **Given** an active remote session, **when** the user activates Disconnect, **then** the existing disconnect behavior runs and the control is no longer offered once disconnected.
3. **Given** either pane toolbar, **when** it is displayed, **then** back and forward buttons are not shown.
4. **Given** the back and forward controls are removed, **when** the user navigates with the remaining controls, **then** location, parent, refresh, filter, hidden-file, and sort behavior remain available.

---

### Edge Cases

- A new favorite is saved with a remote path that is valid but the local path is blank; only the supplied path is stored.
- A local folder chosen through the browser is later moved, deleted, or becomes unreadable; the existing favorite fallback behavior applies on the next connection.
- The native folder chooser is unavailable; the local path remains editable by text.
- The favorite editor is opened on a very short window; all content and actions remain reachable through scrolling.
- Shift-selection starts before any anchor exists; the first selection establishes the anchor.
- A range selection spans directories or files; all item types in the visible range are selected according to the existing selection model.
- The listing changes because of a refresh, filter, or sort while a range is being selected; the selection remains by item identity where the item still exists.
- The other pane is disconnected or showing the connection hub when linked navigation is active; link state does not block the active pane.
- A linked typed path, history entry, tab, or favorite has no corresponding location in the other pane; the originating pane navigates and the other pane remains unchanged.
- A linked history action is at the beginning or end of the other pane's history; that pane remains where it is.
- A linked favorite or tab has only one saved path; the other pane applies the same relative move when possible, otherwise it remains unchanged.
- The same-named directory exists only in a different case on a case-insensitive local file system; the existing platform path semantics determine the match.
- The connection is disconnected while queued transfers exist; statistics remain accurate for completed work and do not leak into another connection.
- A transfer completes with an unknown total size; volume still counts transferred bytes and average speed uses elapsed transfer time.
- A directory permission change encounters a file the server refuses to change; the outcome reports which items failed and preserves successful changes.
- A directory contains nested folders and symbolic links; the recursive change reaches every descendant folder and file but does not follow links outside the selected directory.
- A populated directory is made non-traversable by the applied mode; the user has been warned by the confirmation and the result reports failures.
- The remote server does not expose symbolic or numeric permissions; the information panel explains the limitation instead of inventing values.
- A local item's permissions are displayed read-only; no permission-editing or recursive-apply controls appear.
- Clipboard access is unavailable; the copy-path action reports that copying failed without hiding the path.
- Removing back and forward buttons leaves no pointer alternative for history navigation; this is accepted as the requested cleanup.

## Requirements *(mandatory)*

### Functional Requirements

**Selection and sorting**

- **FR-001**: A plain selection MUST replace the current selection with the activated item.
- **FR-002**: Shift-selection MUST select the continuous range from the current selection anchor to the activated item, using the order currently displayed to the user.
- **FR-003**: Cmd-selection on macOS and Ctrl-selection on Windows and Linux MUST add an unselected item or remove a selected item without clearing the other selections.
- **FR-004**: The selection anchor MUST update when the user makes a non-modifier selection or an individual modifier selection.
- **FR-005**: Range and individual selection MUST apply to the current listing only and MUST preserve selected items by identity across refreshes and re-sorts when those items are still present.
- **FR-006**: Activating the Name, Size, or Modified column header MUST sort that pane by the activated column.
- **FR-007**: Re-activating the active sort column MUST toggle between ascending and descending order.
- **FR-008**: The active sort column and direction MUST be visibly indicated in the column header.
- **FR-009**: Sorting MUST be independent for the local and remote panes.
- **FR-010**: Selecting a range in a filtered or sorted listing MUST include only the rows currently visible between the anchor and the target.

**Favorites**

- **FR-011**: Creating a new favorite with a valid remote path MUST succeed and persist the remote path.
- **FR-012**: The local-path field in the favorite editor MUST provide both text entry and an adjacent browse control.
- **FR-013**: The browse control MUST open the native folder chooser at the current local-path value when it is an existing directory, otherwise at the user's home directory.
- **FR-014**: Choosing a folder MUST populate the local-path field; cancelling the chooser MUST leave the existing value unchanged.
- **FR-015**: The favorite editor MUST constrain its height to the available window space, scroll its content, and keep save and cancel actions reachable at the supported minimum window size.
- **FR-016**: Validation and save errors MUST remain visible without pushing the editor actions out of reach.
- **FR-017**: The remote path MUST remain editable by text and MUST retain its existing optional behavior.

**Linked navigation**

- **FR-018**: The remote session line MUST include a Link or Unlink control whose label reflects the current state.
- **FR-019**: When linked, every navigation action in either pane MUST attempt the same relative movement in the other pane: the same parent steps, the same child folder name, the same history direction, and the same descendant segments. The counterpart is resolved from the other pane's current location; saved tab or favorite paths do not override this rule.
- **FR-020**: When linked, opening a directory in one pane MUST open the same-named directory in the other pane when it exists and is accessible.
- **FR-021**: If the other pane cannot resolve or access the mirrored location, the originating pane MUST still complete its navigation and the other pane MUST remain unchanged without a blocking error.
- **FR-022**: Unlinking MUST return both panes to independent navigation immediately.
- **FR-023**: Changing link state MUST NOT force the panes to synchronize their current locations; mirroring applies from the next navigation action.
- **FR-024**: Linked navigation MUST NOT bypass existing connection, permission, or security behavior.

**Queue statistics**

- **FR-025**: The queue MUST display, for each active connection, the number of successfully completed file transfers, the total volume transferred, and the average transfer speed.
- **FR-026**: Statistics MUST be attributed to the connection that owns each transfer and MUST remain isolated between simultaneous connections.
- **FR-027**: A transfer MUST count toward the completed-file total only after it reaches a successful completed state.
- **FR-028**: Failed, cancelled, and interrupted transfers MUST NOT count as completed files and MUST NOT contribute their partial bytes to completed volume or average speed.
- **FR-029**: Average speed MUST represent completed-transfer throughput for the connection and MUST be available once at least one transfer has completed.
- **FR-030**: Statistics MUST update when transfers complete and MUST present a clear zero or empty state before the first completion.
- **FR-031**: Volume MUST be displayed in a human-readable unit that scales through bytes, KB, MB, and GB.
- **FR-032**: Statistics MUST be scoped to the active connection session, MUST reset when that session ends, and MUST NOT be merged across different connections or reconnects.

**File information and permissions**

- **FR-033**: The information panel MUST present permissions as User, Group, and World rows with Read, Write, and Execute states.
- **FR-034**: The panel MUST show the octal value and the symbolic summary alongside the permission rows.
- **FR-035**: Toggling an available permission control MUST update the octal and symbolic summaries before the change is applied.
- **FR-036**: Applying a permission mode to a remote item MUST use the existing permission-change behavior and MUST refresh the displayed information after success.
- **FR-037**: For a remote directory, the panel MUST offer an action to apply the permission mode to enclosed items, with a confirmation dialog that explains the scope.
- **FR-038**: Cancelling the enclosed-items confirmation MUST make no permission changes.
- **FR-039**: Confirming the enclosed-items action MUST apply the mode recursively to every descendant folder and file, MUST NOT follow symbolic links, and MUST report failures without hiding successful changes.
- **FR-040**: Permission-editing and enclosed-items controls MUST be disabled with an explanation when the remote item or server does not support permission changes.
- **FR-041**: Local items MUST show the detailed permission display available from the platform in read-only form and MUST NOT offer permission-editing or recursive-apply controls.
- **FR-042**: The information panel MUST provide a control to copy the item's path; for remote items this MUST copy the remote server path.
- **FR-043**: The copy action MUST show an inline “Path copied” confirmation for at least two seconds and MUST provide an actionable failure message if clipboard access is unavailable.
- **FR-044**: Permission and path displays MUST NOT expose credentials, key material, or other secrets.

**Remote pane controls**

- **FR-045**: The Disconnect control MUST appear next to the remote pane title and server name for an active connection.
- **FR-046**: The Disconnect control MUST invoke the existing disconnect behavior and MUST NOT be shown when no remote connection is active.
- **FR-047**: Both local and remote pane toolbars MUST no longer display back and forward buttons.
- **FR-048**: Removing back and forward buttons MUST NOT remove the remaining parent-directory, refresh, location, filter, hidden-file, or sort controls.
- **FR-049**: The requested control moves and removals MUST preserve existing keyboard access, focus order, and accessible labels for the remaining controls.

**Compatibility and quality**

- **FR-050**: Existing connection, credential, host-key, transfer, queue, and plain-FTP warning behavior MUST remain unchanged unless this specification explicitly changes it.
- **FR-051**: Local and remote panes MUST remain equal in height, and scrolling MUST remain confined to each pane's directory listing.
- **FR-052**: New and changed controls MUST be keyboard operable, have accessible names, and show a visible focus indicator.
- **FR-053**: The feature MUST NOT introduce mandatory telemetry, cloud services, or new secret storage.
- **FR-054**: Existing saved favorites, preferences, and active-session behavior MUST continue to load without data loss.

### Key Entities *(include if data involved)*

- **Listing Selection**: The set of selected items in one pane plus the anchor used for range selection. Selection is identified by item path within the current listing.
- **Sort State**: The column and direction applied to one pane's listing. Local and remote panes each have their own sort state.
- **Favorite Editor State**: The editable favorite fields, including the optional local path chosen by text or folder browser, the optional remote path, validation state, and credential policy.
- **Pane Link State**: Whether the local and remote panes mirror every navigation action by the same relative movement, scoped to the active session.
- **Connection Transfer Statistics**: Per-connection totals for completed file count, completed volume, and average completed-transfer speed.
- **Permission Set**: The User, Group, and World Read, Write, and Execute states, with corresponding octal and symbolic representations.
- **Recursive Permission Request**: A directory, a target permission set, the enclosed-item scope, confirmation state, and the resulting outcome report.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can create a favorite with a remote path on the first attempt in 100% of test runs.
- **SC-002**: The favorite editor's save and cancel actions remain visible and operable in 100% of runs at the supported minimum window size, including when validation errors are shown.
- **SC-003**: A user can select a continuous range and add or remove individual files with the platform modifier in under ten seconds during usability testing.
- **SC-004**: Sorting by Name, Size, and Modified toggles direction correctly in 100% of automated interaction tests, and selected items remain selected after sorting.
- **SC-005**: Linked navigation mirrors every navigation action by the same relative movement in 100% of test cases where the counterpart exists, and never blocks the originating pane when it does not.
- **SC-006**: Per-connection file counts, volumes, and average speeds match calculated fixtures in 100% of automated tests, including mixed completed and failed transfers across two connections.
- **SC-007**: Permission checkboxes, octal values, and symbolic summaries agree for 100% of tested permission modes.
- **SC-008**: Cancelling a recursive permission confirmation changes zero items in 100% of test runs; confirming reports the outcome for every attempted item.
- **SC-009**: Copying a remote path places the exact path on the clipboard in 100% of supported-platform test runs.
- **SC-010**: The Disconnect control is reachable beside the remote title within one action in 100% of connected-session tests.
- **SC-011**: Back and forward buttons are absent from both pane toolbars in 100% of UI tests, while all remaining navigation controls continue to function.
- **SC-012**: Existing automated test suites continue to pass, with new coverage for selection ranges, sorting toggles, favorite save/browse behavior, linked navigation, connection statistics, permission editing, recursive confirmation, path copy, and control placement.
- **SC-013**: Users can complete every changed workflow covered by this specification with the keyboard alone, with visible focus and accessible names at every step.

## Assumptions

- The platform modifier is Cmd on macOS and Ctrl on Windows and Linux; the behavior follows standard desktop file-manager conventions.
- Shift-range selection uses the order currently visible in the pane after filtering and sorting. The anchor is the most recent non-modifier selection or individual modifier selection.
- Selection persists by item path across re-sorts and refreshes when the item still exists; items that disappear from the listing are no longer selected.
- The local-path browse control uses the native folder chooser. Remote paths remain text entry because browsing a remote tree from the dialog is not part of this scope.
- When the local-path field is empty or does not point to an existing directory, the chooser opens at the user's home directory.
- Link state is scoped to the active remote session, is not persisted, and does not immediately force the panes to the same location. When linked, every navigation action mirrors the same relative movement in the other pane; when no corresponding location exists, the other pane stays unchanged.
- Queue statistics are session-scoped per connection and reset when that connection session ends. Only successfully completed file transfers count; directory entries and failed, cancelled, or interrupted transfers do not. Average speed is based on completed-transfer elapsed time, and volume auto-scales through bytes, KB, MB, and GB.
- Permission editing is available for remote items only and uses the existing server permission-change capability. Local permission display is read-only, including the detailed permission breakdown.
- Recursive permission changes apply the selected mode to every descendant folder and file according to what the server permits, do not follow symbolic links into other locations, and report per-item failures.
- The supported minimum window size remains 900 by 600 logical pixels.
- Existing light, dark, and system themes remain in use; no new theme is introduced.
- Removing back and forward buttons is intentional. No replacement pointer control for history navigation is required by this specification.
- The desktop interface is in scope; the command-line and MCP interfaces are unchanged.
