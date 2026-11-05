# Contract: Desktop Workspace IPC

**Feature**: `004-workspace-ui-refresh` | **Date**: 2026-11-05

This contract defines the interface between the Svelte UI and the Rust core through Tauri commands. Command names are shown in snake_case, matching Rust/Tauri conventions; payload fields use camelCase JSON. It extends, and does not replace, `specs/001-desktop-file-transfer/contracts/desktop-ipc.md` and `specs/003-server-favorites/contracts/favorites-ipc.md`.

## Conventions

- All commands return a typed result or a structured `DesktopError`.
- Secrets are accepted only by commands that need them and are never returned to the UI.
- All identifiers are UUID strings.
- Timestamps are ISO-8601 UTC strings.
- The UI never writes metadata directly to disk.
- The core validates all inputs; the UI must not rely on validation alone.
- New fields are additive and optional. Missing fields must preserve pre-existing behavior.
- Paths are opaque strings. Local paths use the client OS convention; remote paths use the server convention.

## Shared Types

```text
Protocol =
  | "ftp"
  | "ftpsExplicit"
  | "ftpsImplicit"
  | "sftp"

AuthMethod =
  | "password"
  | "sshPrivateKey"

CredentialPolicy =
  | "save"
  | "ask"

HubTab =
  | "favorites"
  | "history"

SidebarSectionKey =
  | "servers"
  | "directoryFavorites"
  | "automation"

WorkspaceLayout = {
  splitRatio: number          // clamped to 0.20..=0.80
  sidebarVisible: boolean
  sidebarSections: Record<SidebarSectionKey, boolean>
  hubTab: HubTab
}

ServerFavoriteInput = {
  displayName: string
  groupLabel?: string | null
  protocol: Protocol
  host: string
  port: number
  username: string
  authMethod: AuthMethod
  timeoutSeconds: number
  sshKeyRef?: string | null
  pinned?: boolean
  localPath?: string | null      // new
  remotePath?: string | null     // new
}

ServerFavoriteView = {
  id: string
  connectionProfileId: string
  displayName: string
  groupLabel: string | null
  protocol: Protocol
  host: string
  port: number
  username: string
  authMethod: AuthMethod
  timeoutSeconds: number
  sshKeyRef?: string | null
  pinned: boolean
  lastUsedAt: string | null
  credentialPolicy: CredentialPolicy
  hasSavedCredential: boolean
  createdAt: string
  updatedAt: string
  credentialWarning?: { code: string; message: string }
  localPath: string | null       // new
  remotePath: string | null      // new
}

ConnectionHistoryEntry = {
  favoriteId: string | null
  connectionProfileId: string
  displayName: string
  protocol: Protocol
  host: string
  port: number
  username: string
  lastUsedAt: string
}

SshKeyCandidate = {
  fileName: string
  path: string
}

SshKeyValidation = {
  valid: boolean
  code?: "missing" | "not_a_file" | "unreadable" | "public_key" | "unsupported"
  message?: string
}

PathSource =
  | "local"
  | "remote"

PathResolution = {
  originalPath: string
  path: string
  fallback: "none" | "ancestor" | "home" | "login"
  notice?: { code: string; message: string }
}

DesktopError = {
  code:
    | "validation"
    | "not_found"
    | "conflict"
    | "permission_denied"
    | "unsupported_capability"
    | "credential_store_unavailable"
    | "credential_missing"
    | "credential_rejected"
    | "connection_failed"
    | "internal"
  message: string
  field?: string
  retryable?: boolean
}
```

## New Commands

### `workspace_get`

- **Input**: none
- **Output**: `WorkspaceLayout`
- **Behavior**: Returns the persisted layout, or defaults when the document has no layout (version 0/1 or first launch). Clamps an out-of-range `splitRatio` and normalizes unknown section keys or hub tabs.
- **Errors**: `internal`

### `workspace_set`

- **Input**: `{ layout: WorkspaceLayout }`
- **Output**: `WorkspaceLayout` (the normalized value that was persisted)
- **Behavior**:
  - Clamps `splitRatio` to `0.20..=0.80`.
  - Fills missing section keys with `true` and ignores unknown keys.
  - Falls back to `favorites` for an unknown `hubTab`.
  - Persists atomically through the metadata store.
  - Never blocks browsing on a write failure; the command still returns a typed error so the UI can show a non-blocking notice.
- **Errors**: `validation`, `internal`

### `connection_history_list`

- **Input**: none
- **Output**: `ConnectionHistoryEntry[]`, newest `lastUsedAt` first
- **Behavior**: Returns at most 50 entries and at most one entry per saved server. Entries survive deletion of the referenced favorite.
- **Errors**: `internal`

### `connection_history_clear`

- **Input**: `{ confirmed: boolean }`
- **Output**: `{ cleared: true; removed: number }`
- **Behavior**: Removes all history entries only when `confirmed` is exactly `true`. Does not disconnect, delete favorites, remove credentials, close tabs, or affect transfers.
- **Errors**: `conflict` when not confirmed, `internal`

### `ssh_keys_list`

- **Input**: none
- **Output**:
  ```text
  {
    directory: string
    exists: boolean
    readable: boolean
    candidates: SshKeyCandidate[]
  }
  ```
- **Behavior**:
  - Resolves the platform SSH directory: `~/.ssh` on macOS/Linux, `%USERPROFILE%\.ssh` on Windows.
  - Returns conventional private-key candidates only.
  - Never reads or returns key contents.
  - Returns `exists: false` or `readable: false` with an empty candidate list rather than failing when the directory is absent or inaccessible.
- **Errors**: `internal` only for unexpected failures

### `ssh_keys_validate`

- **Input**: `{ path: string }`
- **Output**: `SshKeyValidation`
- **Behavior**:
  - Checks existence, file type (not a directory), readability, and obvious public-key names (`.pub`).
  - Returns a user-correctable `valid: false` result rather than throwing for these cases.
  - Never returns file contents, key material, or passphrases.
- **Errors**: `validation` for an empty path or a path containing NUL; `internal` for unexpected failures

### `browser_resolve_path`

- **Input**:
  ```text
  {
    source: PathSource
    path: string
    connectionId?: string   // required when source = "remote"
  }
  ```
- **Output**: `PathResolution`
- **Behavior**:
  - `source = "local"`: checks the path and walks to the nearest existing, readable ancestor. If none exists, falls back to the local home directory.
  - `source = "remote"`: checks the path through the active connection and walks to the nearest accessible ancestor. If none exists, falls back to the server login directory (currently `.`).
  - Returns `fallback: "none"` when the requested path is usable.
  - Returns at most one redacted `notice` explaining a fallback. The notice identifies the affected path but contains no credentials or raw protocol errors.
  - A missing or disconnected remote connection returns `not_found`.
- **Errors**: `not_found`, `permission_denied`, `unsupported_capability`, `validation`, `internal`

## Changed Commands

### `favorites_create` and `favorites_update`

- **Input addition**: `profile.localPath?: string | null` and `profile.remotePath?: string | null`
- **Output addition**: `localPath` and `remotePath` in `ServerFavoriteView`
- **Behavior**:
  - Blank paths normalize to `null`.
  - Non-blank paths must be absolute; otherwise return `validation` with `field: "localPath"` or `"remotePath"`.
  - Path changes do not touch credentials, active connections, or automation payloads.
  - Existing clients that omit the fields keep the previous value on update and `null` on create.

### `favorites_connect`

- **Behavior addition**: On a successful connection, upsert a `ConnectionHistoryEntry` using the favorite's non-secret identity snapshot and the current time. Record nothing on failure.
- **Output**: existing `ConnectionView` (includes `favoriteId`), unchanged otherwise.
- **Errors**: existing behavior unchanged.

### `connections_connect`

- **Behavior addition**: On a successful connection, upsert a `ConnectionHistoryEntry` keyed by the connection profile. Record nothing on failure.
- **Output**: existing `ConnectionView`, unchanged otherwise.
- **Note**: This is the desktop command. It does not change CLI or MCP connection behavior.

### `connections_update` and `favorites_update`

- **Behavior addition**: Preserve `localPath` and `remotePath` unless the caller explicitly supplies them. Editing connection identity fields must not clear saved paths.

## Events

No new event names are required.

- `connection:state` continues to drive the hub/listing transition.
- `favorites:changed` continues to refresh saved servers and their paths.
- `queue:state`, `transfer:progress`, and `transfer:state` continue to update while the hub is shown.
- `connection_history_list` is queried when the History tab is shown; the UI may refresh it after a successful connection.

## Persistence and Compatibility

- The metadata document becomes schema version 2. Versions 0 and 1 load with defaults for the new fields.
- `ServerFavorite.localPath`, `ServerFavorite.remotePath`, `AppState.workspaceLayout`, and `AppState.connectionHistory` are additive and optional/defaulted.
- Existing `specs/003-server-favorites` payloads remain valid without the new fields.
- The CLI and MCP interfaces expose none of these commands and retain their existing payloads and behavior.
- No command returns key contents, credentials, or failure details for history.

## Verification Expectations

- Contract tests assert camelCase payloads, defaults, validation field names, history deduplication/cap, and the absence of secrets.
- Frontend tests mock `invoke` and assert the exact command names and arguments used by the UI.
- Manual checks cover the native file chooser, which is provided by `@tauri-apps/plugin-dialog` and is not exercised by jsdom.
