# Research: Browser and Favorites Workflow Polish

**Feature**: `006-browser-workflow-polish` | **Date**: 2026-11-06

## Decision 1: Per-pane selection anchor and modifier context

**Decision**: Keep selection in each pane as the existing ordered set of item paths plus a new anchor path. `DirectoryList` reports which modifier was used (plain, Shift, or Cmd/Ctrl) instead of collapsing every modifier into one `additive` boolean. A Shift selection computes the range from the anchor to the target in the currently visible order; a Cmd/Ctrl selection toggles one item and updates the anchor.

**Rationale**: This matches standard macOS Finder and platform file-manager behavior, works with the existing path-based selection arrays, and keeps range semantics correct after filtering or sorting.

**Alternatives considered**:
- Store a numeric index anchor: rejected because sorting and filtering change indices and would break the selection.
- Keep a single `additive` boolean: rejected because Shift and Cmd/Ctrl have different semantics.
- Reuse the selected path as the anchor: rejected because adding an individual file would unexpectedly move a later Shift range.

## Decision 2: Direction-aware client-side sorting

**Decision**: Add a `sortDirection` value (`asc` or `desc`) beside the existing sort key. Column headers become buttons that set the column and toggle direction when the same column is activated again. Sorting remains client-side over the loaded listing. The existing toolbar sort control remains as a keyboard-accessible fallback and updates the same sort state without forcing a direction change.

**Rationale**: The listing is already streamed and virtualized in the UI; sorting locally avoids new network traffic and preserves instant navigation. Keeping the toolbar control avoids removing an existing accessible path.

**Alternatives considered**:
- Re-query the server on each sort: rejected because it would add latency and break instant sorting.
- Remove the toolbar sort control: rejected because it is not requested and headers alone are less discoverable for keyboard users.
- Store direction only in the toolbar: rejected because the requested interaction is header-based.

## Decision 3: Favorite prefill must resolve relative working paths

**Decision**: Fix new-favorite creation by resolving the active tab's working paths before passing them to the favorite form. The tab defaults for local and remote paths can be `.`; those values are not absolute and fail favorite-path validation. The prefill flow will use the existing path-resolution command to turn the current pane locations into accessible absolute paths, and will omit a path when it cannot be resolved. Direct creation with a valid absolute remote path already works in the core and will be protected by a regression test.

**Rationale**: The failure is caused by prefill data, not by the core favorite model. Resolving at the edge preserves the existing rule that stored paths are absolute and avoids weakening validation.

**Alternatives considered**:
- Allow `.` as a stored remote or local path: rejected because it is ambiguous across servers and reconnects.
- Require the user to retype the remote path every time: rejected because it defeats save-as-favorite.
- Change the remote path validation to accept relative paths: rejected because saved paths are restored as absolute locations.

## Decision 4: Local path browsing uses the existing native dialog

**Decision**: Add a browse button next to the local-path field that opens the existing native folder chooser at the current value when it is an absolute directory, otherwise at the user's home directory. Cancelling leaves the field unchanged. Remote paths remain text fields because remote-tree browsing is outside this feature.

**Rationale**: The dialog plugin is already a dependency and is already used for SSH key selection, so no new dependency or permission is needed.

**Alternatives considered**:
- Build a custom in-app folder browser: rejected as unnecessary complexity.
- Add remote browsing to the same button: rejected because remote path selection has different connection and permission semantics.

## Decision 5: Favorite dialogs use a constrained body with fixed actions

**Decision**: Give the favorite dialog a viewport-constrained maximum height, place its fields in a scrollable body, and keep the action bar outside that scroll area. The edit wrapper must not add unconstrained height above or below the form.

**Rationale**: The current form grows with every credential and policy field, pushing Save and Cancel off-screen on short windows. A fixed action area keeps the primary actions reachable regardless of validation messages or credential controls.

**Alternatives considered**:
- Shrink the fields: rejected because it harms readability and touch targets.
- Collapse advanced fields: possible later, but not required and would hide existing options.
- Rely on the browser window scrolling: rejected because the app shell intentionally prevents page-level scrolling.

## Decision 6: Linked navigation uses relative path transitions

**Decision**: Represent a navigation transition as a relative move: how many parent segments are removed and which child segments are appended. When linked, apply the same relative move to the other pane. Probe the target location before navigating; if the counterpart does not exist or is inaccessible, leave that pane unchanged. The link control is session-scoped, does not force an immediate synchronization, and mirrors every navigation action including parent, child, breadcrumb, typed path, history, tabs, and favorites.

**Rationale**: Absolute local and remote paths do not share a root. Relative movement is the only rule that works for arbitrary navigation while preserving the user's parallel directory structure. Probing before navigating avoids showing spurious errors in the other pane.

**Alternatives considered**:
- Mirror only up and child navigation: rejected by the clarification answer, which selected all navigation actions.
- Use saved favorite paths for favorites and tabs: rejected because it does not cover breadcrumbs, typed paths, or history and can move the other pane unexpectedly.
- Navigate first and revert on failure: rejected because it causes visible loading and error flicker.

## Decision 7: Derive per-connection queue statistics from session timestamps

**Decision**: Compute per-connection statistics from transfers that started after the active connection's `startedAt` and completed successfully. Count completed files, sum completed bytes, and calculate average speed as total completed bytes divided by total completed-transfer elapsed time. Extend the transfer state event with authoritative `startedAt` and `completedAt` values so live statistics do not depend on client clock guesses. Statistics are display-only and reset when the connection session changes.

**Rationale**: The transfer records already carry the required identities, timestamps, and byte counts. Deriving statistics avoids new persisted state and naturally resets when a new connection session begins. Filtering by start time prevents a transfer that began in an earlier session from being attributed to a reconnect.

**Alternatives considered**:
- Persist lifetime statistics per saved server: rejected by the clarification answer (reset per session).
- Add a backend statistics table: rejected because it duplicates data already present in transfer records and introduces new state.
- Count failed or cancelled partial bytes: rejected because the spec counts transferred files and completed throughput.

## Decision 8: Extend inspector metadata with permission detail

**Decision**: Extend the inspector metadata contract with the numeric mode and parsed User/Group/World Read/Write/Execute bits, while reusing the existing octal string and adding the symbolic summary. Permission parsing and rendering remain in the core so the UI does not duplicate bit rules.

**Rationale**: The inspector currently receives only an octal string. Returning a normalized permission detail keeps the bit mapping testable in Rust and gives the UI a stable shape for checkboxes and summaries.

**Alternatives considered**:
- Parse and render everything in Svelte: rejected because bit-level permission rules are domain behavior and should be tested with the core.
- Only expose the octal string: rejected because it would make the UI reimplement permission semantics.

## Decision 9: Recursive permission changes are a confirmed core operation

**Decision**: Add a core recursive permission operation that walks all descendant folders and files without following symbolic links. It applies the requested mode where permitted, continues after individual failures, and returns a report of changed, skipped, and failed items. The Tauri command requires an explicit `confirmed` flag; the UI collects confirmation before calling it. Local permissions remain read-only.

**Rationale**: Permission changes can be destructive and recursive traversal belongs with the existing permission logic. The confirmation flag makes accidental invocation through a UI bug fail closed, and a report preserves visibility of partial success.

**Alternatives considered**:
- Loop through entries in the Svelte component: rejected because it would duplicate protocol traversal and make failure reporting inconsistent.
- Stop at the first failure: rejected because it hides successful changes and makes retries unclear.
- Follow symlinks: rejected by the clarification answer and by least-privilege safety.

## Decision 10: Remote pane owns the Disconnect control

**Decision**: Move the Disconnect action from the session-picker line into the remote pane heading beside the remote title and server name. The session-picker keeps the active-session selector and Save as favorite action. The pane receives a disconnect callback scoped to the selected connection.

**Rationale**: The requested control belongs with the remote context, and the session line is visually distant from the server title. Keeping Save as favorite in the session line preserves the existing quick action.

**Alternatives considered**:
- Duplicate Disconnect in both places: rejected as duplicate controls with the same meaning.
- Put Disconnect inside the remote toolbar: rejected because the toolbar is for directory navigation and filtering.

## Decision 11: Remove back/forward controls without removing history state

**Decision**: Remove the back and forward buttons from both pane toolbars and remove the now-unused pointer handlers. Keep the internal navigation history used by tab restoration and existing navigation state; no replacement pointer control is added, as accepted in the spec.

**Rationale**: This is the requested cleanup. Keeping history state avoids breaking tab restoration and any future keyboard shortcut work.

**Alternatives considered**:
- Replace them with a menu: rejected as unrequested scope.
- Remove history state entirely: rejected because it would affect tab restoration and navigation bookkeeping.

## Decision 12: Preserve per-pane scroll containment

**Decision**: Keep the app shell at viewport height with overflow hidden, keep each pane header and toolbar fixed, and let each pane's directory viewport own its scrollbar. The selection and sorting work must not introduce a shared scroll container or page-level scrollbar.

**Rationale**: This feature adds controls and statistics above and below the browser area; without the established containment, those additions could reintroduce whole-page scrolling or synchronize both panes accidentally.

**Alternatives considered**:
- Use a single scroll container for both panes: rejected because it breaks independent pane scrolling.
- Let the window scroll: rejected because navigation would move out of view.
