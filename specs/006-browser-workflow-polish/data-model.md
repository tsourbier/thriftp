# Data Model: Browser and Favorites Workflow Polish

**Feature**: `006-browser-workflow-polish` | **Date**: 2026-11-06

## Entity Overview

```text
ConnectionProfile 1 ─── * Transfer
ConnectionProfile 1 ─── 0..1 ConnectionSession
ConnectionSession 1 ─── * ConnectionTransferStatistics   (derived, runtime only)
Pane 1 ─── 1 SelectionState
Pane 1 ─── 1 SortState
Pane 2 1 ─── 1 PaneLinkState                              (runtime only)
ServerFavorite 1 ─── 0..1 LocalPath
ServerFavorite 1 ─── 0..1 RemotePath
FileMetadata 1 ─── 0..1 PermissionDetail
PermissionDetail 1 ─── 1 PermissionSet
RecursivePermissionRequest 1 ─── 1 PermissionSet
RecursivePermissionResult 1 ─── * PermissionFailure
```

All persisted data remains non-secret. Credentials, private-key material, passphrases, and file contents are outside this model.

## Persisted Additions

### SortState (stored with view settings)

Sorting remains local to a pane but the chosen direction is retained alongside the existing sort key.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `column` | `name` \| `size` \| `date` \| `type` | Yes | Existing sort key |
| `direction` | `asc` \| `desc` | Yes | NEW: defaults to `asc` for existing data |

**Validation and compatibility**:
- Missing `direction` deserializes as `asc`, so existing tabs and preferences load without migration failure.
- Only `name`, `size`, and `date` are exposed as clickable column headers; `type` remains available only through the existing sort control.
- Selecting a new column starts in ascending order; re-selecting the active column toggles direction.
- Sorting never changes the selected path set or the selection anchor.

**State transitions**:

```text
Header(column != current) ──► column = selected, direction = asc ──► persist view settings
Header(column == current) ──► direction = inverse(direction) ────► persist view settings
Toolbar sort(column) ───────► column = selected, direction retained ─► persist view settings
```

### Preference (extended)

The default sort direction for newly created tabs.

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `sortDirection` | `asc` \| `desc` | No | `asc` | Additive to the existing preference document |

**Compatibility**: Missing values default to `asc`. No schema version bump is required.

## Runtime-Only Entities

### SelectionState

The selection model for one pane.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `selectedPaths` | string[] | Yes | Ordered set of selected item paths |
| `anchorPath` | string or null | No | Anchor for Shift ranges |
| `activePath` | string or null | No | Visually focused/primary item |

**Rules**:
- A plain activation replaces `selectedPaths` with the activated path and sets the anchor and active path.
- Shift activation selects the continuous range from the anchor to the target in the current visible order, without including filtered-out entries.
- Cmd/Ctrl activation toggles the target and updates the anchor and active path.
- If the anchor no longer exists in the current listing, Shift activation behaves like a plain selection and establishes a new anchor.
- Sorting or filtering preserves selections by path; missing paths are removed on the next authoritative listing update.
- A range can include files, directories, symlinks, and other entries; no type is excluded from normal selection.

**State transitions**:

```text
No selection ──plain──► one selected + anchor
Anchor + target ──shift──► contiguous range + active target
Existing selection + target ──cmd/ctrl──► toggled target + anchor
Sort/filter ──► same paths retained where still visible
```

### PaneLinkState

Whether navigation in one pane mirrors the other.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `linked` | boolean | Yes | Session-scoped; default false |

**Rules**:
- Link state is owned by the active remote session and resets when the session ends.
- Turning link on or off never changes either pane's current path immediately.
- When linked, every user navigation action computes a relative transition from the source pane's current path to its destination.
- The relative transition consists of an `up` count and a list of child segments.
- If the other pane is disconnected or the resulting location cannot be resolved, that pane remains unchanged.
- Mirroring never triggers a second mirror operation.

**State transitions**:

```text
Disconnected ──► linked = false
Connected ──unlink──► linked = false
Connected ──link──► linked = true; paths unchanged
Linked navigation ──relative move──► counterpart navigates or stays
```

### ConnectionSession (runtime reference)

The active remote session used as the statistics boundary.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `connectionId` | UUID | Yes | Active session identifier |
| `connectionProfileId` | UUID | Yes | Saved connection identity |
| `startedAt` | timestamp | Yes | Session start; statistics ignore older completions |

**Rules**:
- A session is created by a successful connection and ends on disconnect.
- Reconnecting creates a new session with a new start time.
- The session is runtime-only; it is not written to metadata.

### ConnectionTransferStatistics

Derived display data for one active connection session.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `connectionProfileId` | UUID | Yes | Connection identity |
| `connectionStartedAt` | timestamp | Yes | Lower bound for included transfers |
| `completedFiles` | integer | Yes | Count of successful completions |
| `completedBytes` | integer | Yes | Sum of completed transfer sizes |
| `averageBytesPerSecond` | integer or null | No | Total bytes divided by total elapsed time |

**Rules**:
- Include a transfer only when it belongs to the active profile, has `state = completed`, and has `startedAt >= connectionStartedAt`.
- Count bytes using the completed transfer's exact transferred bytes; fall back to total bytes only when transferred bytes are unavailable.
- Exclude pending, active, paused, failed, cancelled, and interrupted transfers.
- Exclude transfers that started before the current connection session.
- Average speed is null until at least one completed transfer has a positive elapsed duration.
- Statistics reset when the connection session ends and do not merge across simultaneous connections.
- Values are never persisted and never contain file content or secrets.

**State transitions**:

```text
Connection opens ──► zero values
Transfer completes ──► recompute from completed session transfers
Transfer fails/cancels ──► no contribution
Connection closes ──► statistics discarded
Reconnect ──► new session, values start at zero
```

### PermissionDetail

Permission data returned by the inspector for one item.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `mode` | integer or null | No | Numeric mode, for example `0o644` |
| `symbolic` | string or null | No | For example `rw-r--r--` |
| `user` | PermissionBits | Yes | Owner bits |
| `group` | PermissionBits | Yes | Group bits |
| `world` | PermissionBits | Yes | Other/world bits |
| `editable` | boolean | Yes | True only for remote items with permission support |
| `reason` | string or null | Conditional | Explanation when editing is unavailable |

**PermissionBits**:

| Field | Type | Required |
|-------|------|----------|
| `read` | boolean | Yes |
| `write` | boolean | Yes |
| `execute` | boolean | Yes |

**Rules**:
- Permission detail is consistent with the octal value: user bits map to `0400/0200/0100`, group to `0040/0020/0010`, world to `0004/0002/0001`.
- A missing or unparseable permission value produces an unavailable detail with a reason; it never invents bits.
- Local items are always `editable = false`; remote items are editable only when the connection advertises permission support.
- Toggling a checkbox produces a new draft `PermissionSet`; the displayed octal and symbolic values are recalculated from the draft before applying.
- Applying a mode refreshes the metadata and the permission detail.

### RecursivePermissionRequest

The explicit request to apply a mode to a directory tree.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `connectionId` | UUID | Yes | Active remote session |
| `path` | string | Yes | Remote directory root |
| `mode` | integer | Yes | `0o000`–`0o777` |
| `confirmed` | boolean | Yes | Must be true |

**Rules**:
- The request is rejected when `confirmed` is false.
- The directory must exist and be a directory.
- The mode is validated before traversal.
- Symbolic links are not followed and are reported as skipped.
- The root directory itself is not part of the enclosed set; the UI applies the root mode separately if the user chooses.

### RecursivePermissionResult

The outcome of a recursive permission request.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `root` | string | Yes | Directory that was processed |
| `changed` | integer | Yes | Number of items whose mode was applied |
| `skipped` | integer | Yes | Items skipped, including symlinks |
| `failed` | PermissionFailure[] | Yes | Per-item failures |
| `message` | string or null | No | Summary when useful |

**PermissionFailure**:

| Field | Type | Required |
|-------|------|----------|
| `path` | string | Yes |
| `message` | string | Yes |

**Rules**:
- A failure on one item does not stop traversal.
- Successful changes are preserved and reported even when some items fail.
- Error messages contain paths and actionable permission/transport reasons, never secret material.
- The result is not persisted.

### FavoriteEditorState

The transient draft used by create and edit dialogs.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `displayName` | string | Yes | Existing profile field |
| `protocol` | enum | Yes | Existing protocol |
| `host` | string | Yes | Existing profile field |
| `port` | integer | Yes | Existing profile field |
| `username` | string | Yes | Existing profile field |
| `authMethod` | enum | Yes | Existing behavior |
| `sshKeyRef` | string or null | Conditional | Existing key reference |
| `localPath` | string or null | No | Absolute local path |
| `remotePath` | string or null | No | Absolute server path |
| `credentialPolicy` | `save` \| `ask` | Yes | Existing credential behavior |
| `credentialRefId` | UUID or null | No | Existing reuse behavior |
| `validationError` | string or null | No | Transient |
| `browseError` | string or null | No | Transient folder-browser failure |

**Rules**:
- Empty or whitespace-only paths normalize to null.
- Non-null local and remote paths must be absolute where the platform requires it.
- The local browse button opens at the current value when it is an existing directory; otherwise it opens at the user's home directory and never changes the value on cancel.
- Prefilling from an active tab resolves `.` and other relative working paths before validation.
- The dialog body scrolls while the action bar remains outside the scroll area.
- Validation errors remain visible without moving Save or Cancel out of reach.
