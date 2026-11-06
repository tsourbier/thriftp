# Desktop IPC Contract: Browser and Favorites Workflow Polish

**Feature**: `006-browser-workflow-polish` | **Date**: 2026-11-06

This document covers only new or changed desktop IPC shapes. Existing connection, credential, favorite, transfer, tab, and preferences contracts remain unchanged unless noted.

## Browser Commands

### `browser_chmod`

Unchanged. Applies a single remote item's permission mode and returns the refreshed `FileEntryView`.

### `browser_chmod_recursive`

Applies a permission mode to every descendant folder and file under a remote directory.

| Field | Value |
|-------|-------|
| Command | `browser_chmod_recursive` |
| Input | `{ connectionId, path, mode, confirmed }` |
| Output | `RecursivePermissionResult` |

```ts
type RecursivePermissionResult = {
  root: string;
  changed: number;
  skipped: number;
  failed: Array<{ path: string; message: string }>;
  message?: string;
};
```

Rules:

- `confirmed` MUST be `true`; false is rejected with a `conflict` error and changes nothing.
- `mode` is an integer from `0o000` through `0o777`.
- The target must be a remote directory on an active session.
- Traversal applies the mode to all descendant folders and files.
- Symbolic links are not followed and contribute to `skipped`.
- Per-item failures are returned in `failed`; successful changes are retained.
- Permission support is checked before traversal; an unsupported server returns `unsupported_capability`.
- The command never changes local files.

### `browser_resolve_path`

Unchanged. The favorite prefill flow reuses it to convert active-tab working paths into accessible absolute paths before validation.

### `browser_probe_directory`

Checks whether a local or remote directory can be opened for listing before linked navigation commits a mirrored move.

| Field | Value |
|-------|-------|
| Command | `browser_probe_directory` |
| Input | `{ source, path, connectionId? }` |
| Output | `boolean` |

Rules:

- `source` is `local` or `remote`; `connectionId` is required for remote probes.
- The command performs no navigation and returns `false` when the directory cannot be read.
- Linked navigation uses the probe so an unreadable counterpart stays unchanged.

## Inspector Contract

### `inspector_get`

Input is unchanged: `{ source, connectionId?, path }`.

Output is extended with a `permissionDetail` field:

```ts
type FileMetadata = {
  entry: FileEntryView;
  permissionsSupported: boolean;
  ownershipSupported: boolean;
  permissionDetail: PermissionDetail | null;
};

type PermissionDetail = {
  mode: number | null;
  symbolic: string | null;
  user: PermissionBits;
  group: PermissionBits;
  world: PermissionBits;
  editable: boolean;
  reason?: string;
};

type PermissionBits = {
  read: boolean;
  write: boolean;
  execute: boolean;
};
```

Rules:

- `permissionDetail` is `null` only when no permission information is available at all.
- For local items, `editable` is always `false` and `reason` explains that local permissions are read-only.
- For remote items, `editable` mirrors the server's permission capability.
- `mode` and `symbolic` are consistent with the four permission bits.
- The field is additive; older frontends that ignore it continue to work.

## Transfer Event Contract

### `transfer:state`

The event payload adds authoritative timing and connection information so the queue can derive per-session statistics without guessing.

```ts
type TransferStateEvent = {
  transferId: string;
  connectionProfileId: string;
  direction: "upload" | "download";
  state:
    | "pending"
    | "active"
    | "paused"
    | "completed"
    | "failed"
    | "cancelled"
    | "interrupted";
  transferredBytes: number;
  totalBytes?: number | null;
  error?: DesktopError;
  startedAt?: string | null;
  completedAt?: string | null;
};
```

Rules:

- `transferredBytes` is authoritative for the completed transfer, so statistics do not depend on the last throttled progress sample.
- `totalBytes` is present when the backend knows the transfer size.
- `startedAt` is present when the backend has marked the transfer active.
- `completedAt` is present when the backend has recorded a terminal completion.
- Timestamps are RFC 3339 UTC strings.
- The added fields are optional for compatibility; the current frontend must tolerate their absence in old recordings.
- Statistics count only `completed` transfers whose `startedAt` is at or after the active connection's `startedAt`.
- Failed, cancelled, and interrupted transfers are not counted.

## Favorite Contract

### `favorites_create` / `favorites_update`

The commands and payloads remain unchanged. The fix for a new favorite with a remote path happens before submission:

1. Resolve the active tab's local and remote working paths through `browser_resolve_path`.
2. Omit any path that cannot be resolved.
3. Validate that remaining paths are absolute.
4. Submit the existing `ServerFavoriteInput` with the resolved paths.

`favorites_create` continues to reject relative paths with a `validation` error. Core behavior for a valid absolute remote path is unchanged and covered by a regression test.

## Error Rules

- Permission commands never return raw protocol diagnostics or secrets.
- Recursive permission failures identify the affected path and a safe reason.
- Clipboard failures are handled in the UI and are not an IPC error.
- Existing redaction, credential, certificate, and host-key rules apply unchanged.
