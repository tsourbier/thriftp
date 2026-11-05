# Contract: Desktop Tauri IPC

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

This contract defines the interface between the Svelte UI and the Rust core through Tauri commands and events. Server-favorite commands are shared with `003`; this document defines the desktop connection, browsing, transfer, queue, preview, preference, and directory-favorite surfaces.

## Conventions

- Command names use snake_case (Tauri convention); payload fields use camelCase JSON.
- All identifiers are UUID strings; timestamps are ISO-8601 UTC.
- Secrets are accepted only by commands that need them and are never returned to the UI.
- The UI never writes state files directly.
- The core validates all inputs; UI validation is advisory.
- Long-running operations return an identifier and stream results through events.

## Shared Types

```text
Protocol = "ftp" | "ftpsExplicit" | "ftpsImplicit" | "sftp"
AuthMethod = "password" | "sshPrivateKey"
TransferDirection = "upload" | "download"
TransferState = "pending" | "active" | "paused" | "completed" | "failed" | "cancelled" | "interrupted"
Priority = "low" | "normal" | "high"
SortKey = "name" | "size" | "date" | "type"
Theme = "system" | "light" | "dark"

ConnectionProfileInput = {
  displayName: string
  groupLabel?: string | null
  protocol: Protocol
  host: string
  port: number
  username: string
  authMethod: AuthMethod
  timeoutSeconds: number
  sshKeyRef?: string | null
}

ConnectionView = {
  connectionId: string
  profileId: string
  state: "connecting" | "connected" | "disconnecting" | "error"
  error?: DesktopError
}

FileEntryView = {
  name: string
  path: string
  entryType: "file" | "directory" | "symlink" | "other"
  size: number | null
  modifiedAt: string | null
  permissions: string | null
  ownership: { owner: string; group?: string } | null
  hidden: boolean
  symlinkTarget?: string | null
  capabilities: ProtocolCapabilities
}

TransferView = {
  id: string
  direction: TransferDirection
  profileId: string
  sourcePath: string
  destinationPath: string
  totalBytes: number | null
  transferredBytes: number
  state: TransferState
  priority: Priority
  order: number
  speedBytesPerSecond: number | null
  estimatedRemainingSeconds: number | null
  resumable: boolean
  error?: DesktopError
}

QueueView = {
  paused: boolean
  concurrencyLimit: number
  bandwidthLimitBytesPerSecond: number | null
  activeTransferIds: string[]
  orderedTransferIds: string[]
}

Preferences = {
  theme: Theme
  showHiddenFiles: boolean
  defaultConcurrency: number
  bandwidthLimitBytesPerSecond: number | null
  restoreTabs: boolean
  previewMode: "auto" | "native" | "internal"
  sortKey: SortKey
  reducedMotion: boolean
}

TabView = {
  id: string
  profileId: string | null
  currentPath: string
  history: string[]
  historyIndex: number
  sortKey: SortKey
  showHiddenFiles: boolean
}

DesktopError = {
  code:
    | "validation"
    | "not_found"
    | "auth_failed"
    | "credential_store_unavailable"
    | "credential_missing"
    | "credential_rejected"
    | "certificate_invalid"
    | "host_key_unknown"
    | "host_key_changed"
    | "network"
    | "timeout"
    | "permission_denied"
    | "conflict"
    | "cancelled"
    | "unsupported_capability"
    | "internal"
  message: string
  field?: string
  retryable?: boolean
}
```

## Connection Commands

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `connections_list` | none | `ConnectionProfileView[]` | Includes saved profiles and server favorites |
| `connections_create` | `{ profile, secret?, secretKind? }` | `ConnectionProfileView` | Delegates to the shared favorites service; secrets never returned |
| `connections_update` | `{ id, profile, secret?, secretKind? }` | `ConnectionProfileView` | Identity changes invalidate credential reuse |
| `connections_delete` | `{ id }` | `{ deleted: true; credentialImpact?: CredentialImpact }` | Does not disconnect active connections |
| `connections_connect` | `{ profileId, secret?, secretKind? }` | `ConnectionView` | Session-only secrets are not persisted |
| `connections_disconnect` | `{ connectionId }` | `{ disconnected: true }` | |
| `connections_get_profile` | `{ connectionId }` | `{ profile, state }` | Supports save-as-favorite |
| `connections_test` | `{ profile, secret?, secretKind? }` | `{ ok: boolean; error?: DesktopError }` | No persistence, no last-used update |

## Browser Commands

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `browser_open_local` | `{ path }` | `{ listingId }` | Starts streaming local listing |
| `browser_open_remote` | `{ connectionId, path }` | `{ listingId }` | Starts streaming remote listing |
| `browser_refresh` | `{ listingId }` | `{ listingId }` | Re-enumerates the same location |
| `browser_stat` | `{ source, connectionId?, path }` | `FileEntryView` | Source is `local` or `remote` |
| `browser_mkdir` | `{ source, connectionId?, path }` | `FileEntryView` | |
| `browser_rename` | `{ source, connectionId?, from, to }` | `FileEntryView` | |
| `browser_delete` | `{ source, connectionId?, path, confirmed }` | `{ deleted: true }` | Local uses OS trash; remote warns permanent |
| `browser_chmod` | `{ connectionId?, path, mode }` | `FileEntryView` | Remote only; unsupported capability is an error |
| `browser_cancel_listing` | `{ listingId }` | `{ cancelled: true }` | Stops enumeration |

## Transfer and Queue Commands

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `transfers_start` | `{ direction, profileId, sources, destination, conflictPolicy? }` | `{ transferIds }` | Creates queued transfers |
| `transfers_pause` | `{ transferId }` | `TransferView` | Stops at chunk boundary |
| `transfers_resume` | `{ transferId }` | `TransferView` | Verifies resume safety |
| `transfers_cancel` | `{ transferId }` | `TransferView` | Preserves last confirmed progress |
| `transfers_list` | none | `TransferView[]` | Queue and active view |
| `transfers_get` | `{ transferId }` | `TransferView` | |
| `queue_get` | none | `QueueView` | |
| `queue_pause` | none | `QueueView` | Pauses active and prevents starts |
| `queue_resume` | none | `QueueView` | |
| `queue_set_concurrency` | `{ limit }` | `QueueView` | |
| `queue_set_bandwidth` | `{ bytesPerSecond: number | null }` | `QueueView` | Null means unlimited |
| `queue_reorder` | `{ transferIds: string[] }` | `QueueView` | |
| `queue_set_priority` | `{ transferId, priority }` | `QueueView` | |
| `queue_cancel_all` | `{ confirmed: true }` | `QueueView` | Cancels pending and active after confirmation |

## Directory Favorite Commands

Server favorites are defined in `specs/003-server-favorites/contracts/favorites-ipc.md`. The desktop adds:

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `directory_favorites_list` | `{ profileId? }` | `DirectoryFavoriteView[]` | Optionally filtered by profile |
| `directory_favorites_create` | `{ profileId, path, label }` | `DirectoryFavoriteView` | |
| `directory_favorites_update` | `{ id, label?, path? }` | `DirectoryFavoriteView` | |
| `directory_favorites_delete` | `{ id, confirmed: true }` | `{ deleted: true }` | Does not delete the remote directory |
| `directory_favorites_reorder` | `{ profileId, ids: string[] }` | `DirectoryFavoriteView[]` | |

## Preview Commands

| Command | Input | Output | Notes |
|---------|-------|--------|-------|
| `preview_open` | `{ source, connectionId?, path, allowDownload?: boolean }` | `{ previewId, mode: "native" | "internal" | "metadata" }` | Remote full download requires consent |
| `preview_close` | `{ previewId }` | `{ closed: true }` | Releases resources |

## Preference and Tab Commands

| Command | Input | Output |
|---------|-------|--------|
| `preferences_get` | none | `Preferences` |
| `preferences_set` | `Preferences` | `Preferences` |
| `tabs_list` | none | `TabView[]` |
| `tabs_save` | `{ tabs: TabView[] }` | `{ saved: true }` |
| `tabs_restore` | none | `TabView[]` |

## Events

### `connection:state`

```text
ConnectionView
```

### `directory:batch`

```text
{ listingId: string; entries: FileEntryView[]; state: "partial" | "complete"; firstEntryAt?: string; completedAt?: string }
```

### `directory:error`

```text
{ listingId: string; error: DesktopError }
```

### `transfer:progress`

```text
{ transferId: string; transferredBytes: number; totalBytes: number | null; speedBytesPerSecond: number | null; estimatedRemainingSeconds: number | null }
```

### `transfer:state`

```text
{ transferId: string; state: TransferState; error?: DesktopError }
```

### `queue:state`

```text
QueueView
```

### `favorites:changed`

```text
{ favoriteId: string; reason: "created" | "updated" | "deleted" | "pinned" | "groupChanged" | "credentialChanged" }
```

### `preview:state`

```text
{ previewId: string; state: "opening" | "open" | "closed" | "error"; error?: DesktopError }
```

## Security Rules

- Secret fields are accepted only by connection create/update/connect/test and shared credential commands.
- No command or event returns a secret.
- Error payloads never contain secret values or raw protocol diagnostics.
- The UI must not cache secrets in persistent storage.
- The UI must treat certificate, host-key, credential-store, missing-credential, and rejected-credential errors as distinct recovery states.
- Destructive commands require explicit confirmation from the UI.
- Local deletes use OS trash; remote deletes state that they are permanent.
