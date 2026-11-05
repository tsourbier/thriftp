# Data Model: Workspace UI Refresh and Dual-Pane Connection Hub

**Feature**: `004-workspace-ui-refresh` | **Date**: 2026-11-05

## Entity Overview

```text
ConnectionProfile 1 ─── 1 ServerFavorite
ServerFavorite 1 ─── 0..1 WorkspacePath (localPath)
ServerFavorite 1 ─── 0..1 WorkspacePath (remotePath)
ServerFavorite 1 ─── 0..1 ConnectionHistoryEntry   (history survives deletion)
ConnectionProfile 1 ─── 0..1 ConnectionHistoryEntry
WorkspaceLayout 1 ─── 1 ConnectionHubTab
WorkspaceLayout 1 ─── * SidebarSectionState
SshKeyCandidate 0..* ─── 0..1 selected key path     (runtime only)
CommandPaletteEntry 0..*                              (runtime only)
NavigationLocation 1 ─── * BreadcrumbSegment          (runtime only)
```

All persisted entities are non-secret. Credentials, private-key material, and passphrases remain outside this model in the OS credential store.

## Persisted Entities

### ServerFavorite (extended)

The existing saved-server record extended with optional working paths. Every saved Connection Profile still has exactly one Server Favorite.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Stable favorite identifier |
| `connectionProfileId` | UUID | Yes | References the saved Connection Profile |
| `pinned` | boolean | Yes | Presentation only; default false |
| `lastUsedAt` | timestamp or null | No | Most recent successful connection, UTC |
| `credentialPolicy` | `save` \| `ask` | Yes | Existing behavior unchanged |
| `credentialRefId` | UUID or null | Conditional | Existing credential reference |
| `localPath` | string or null | No | NEW: optional absolute local folder to restore |
| `remotePath` | string or null | No | NEW: optional absolute remote folder to restore |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Projection fields**: `displayName`, `groupLabel`, `protocol`, `host`, `port`, `username`, `authMethod`, `timeoutSeconds`, and `sshKeyRef` continue to come from the Connection Profile. `hasSavedCredential`, `credentialPolicy`, and `credentialWarning` keep their existing meaning.

**Validation rules**:
- `localPath` and `remotePath` are optional; empty or whitespace-only values normalize to `null`.
- Non-null paths MUST be absolute. Relative paths are rejected with a `validation` error and the field name.
- Paths are stored verbatim after trimming; no expansion of `~`, environment variables, or remote shell syntax.
- Paths are limited to the existing metadata document's practical string size; extremely long paths are stored and displayed with breadcrumb truncation rather than rejected.
- Paths contain no credentials, tokens, or key material.
- Editing paths does not change `connectionProfileId`, credentials, or the active connection.
- Deleting a favorite deletes its paths with the favorite. History may retain a display snapshot.

**Compatibility**: Missing fields deserialize as `null`, so favorites created before this feature behave exactly as before.

### WorkspaceLayout

The remembered presentation of the workspace.

| Field | Type | Required | Default | Notes |
|-------|------|----------|---------|-------|
| `splitRatio` | number | Yes | `0.5` | Left-pane share; clamped to `0.20..=0.80` |
| `sidebarVisible` | boolean | Yes | `true` | User's preferred sidebar visibility |
| `sidebarSections` | object of booleans | Yes | All `true` | Keys: `servers`, `directoryFavorites`, `automation` |
| `hubTab` | `favorites` \| `history` | Yes | `favorites` | Last selected disconnected-hub tab |

**Validation rules**:
- `splitRatio` is finite and clamped to `0.20..=0.80` on read and write.
- Unknown `sidebarSections` keys are ignored; missing known keys default to `true`.
- Unknown `hubTab` values fall back to `favorites` with no error, so a future build's value does not block startup.
- The effective sidebar visibility is `sidebarVisible && workspaceWideEnough`; auto-collapse does not overwrite the stored preference.
- Layout changes do not affect tabs, favorites, transfers, or credentials.

**State transitions**:

```text
Split drag/resize ──► draft ratio in UI ──► commit on pointer release/key ──► workspace_set
Sidebar toggle ────► sidebarVisible ─────────────────────────────────────────► workspace_set
Section toggle ────► sidebarSections[key] ───────────────────────────────────► workspace_set
Hub tab select ────► hubTab ─────────────────────────────────────────────────► workspace_set
Disconnect ────────► right pane shows hub with hubTab ───────────────────────► no write required
```

### ConnectionHistoryEntry

A local record of a recent successful connection. It is a non-secret snapshot and is independent of the current favorite list.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `favoriteId` | UUID or null | No | Saved favorite used for the connection; null for legacy/profile-only connects |
| `connectionProfileId` | UUID | Yes | Saved server identity used for deduplication |
| `displayName` | string | Yes | Snapshot at connection time |
| `protocol` | enum | Yes | `ftp`, `ftpsExplicit`, `ftpsImplicit`, `sftp` |
| `host` | string | Yes | Snapshot |
| `port` | integer | Yes | Snapshot |
| `username` | string | Yes | Snapshot |
| `lastUsedAt` | timestamp | Yes | Time of the most recent successful connection |

**Validation and lifecycle rules**:
- Exactly one entry per saved server: deduplicate by `favoriteId` when present, otherwise by `connectionProfileId`.
- A newer successful connection replaces the older timestamp and snapshot for that server.
- Failed, rejected, cancelled, and host-key/certificate-failed attempts are never recorded.
- The list is capped at 50 entries; when the cap is exceeded, remove the oldest `lastUsedAt` entries first.
- Entries contain no credentials, passphrases, key contents, file data, failure details, or transport options.
- `clearHistory(confirmed: true)` removes every entry. It does not disconnect, delete favorites, delete credentials, close tabs, or affect transfers.
- Deleting a favorite does not delete its history entry; a connect action for a deleted server fails with the existing `not_found` error and a clear message.
- `lastUsedAt` here is a snapshot; the favorite's own `lastUsedAt` remains the authoritative value for favorite sorting.

**State transitions**:

```text
Successful connect ──► upsert entry ──► cap at 50 ──► persist metadata
Failed connect ──────► no change
Clear (confirmed) ───► list = [] ──────────────────► persist metadata
Favorite deleted ────► entry retained for display, connect fails not_found
```

## Runtime-Only Entities

### SshKeyCandidate

A supported private-key file discovered in the platform SSH directory. Key contents are never part of this entity.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `fileName` | string | Yes | Display name, for example `id_ed25519` |
| `path` | string | Yes | Full path used as the key reference |

**Rules**:
- Platform directory: `~/.ssh` on macOS/Linux, `%USERPROFILE%\.ssh` on Windows.
- Conventional private-key names only; public keys (`.pub`), `known_hosts`, `config`, `authorized_keys`, sockets, and unrelated files are excluded.
- Discovery reads directory entries and file metadata only; it never reads or returns key contents.
- Exactly one candidate may be pre-filled; multiple candidates require explicit selection or browsing.
- A selected path is validated for existence, file type, and readability. Public-key and directory paths are rejected with actionable messages.
- The selected path is stored in the existing `sshKeyRef` profile field, not in a new secret store.
- Selecting a candidate does not authenticate; key authentication still requires explicit method selection and connection confirmation.

### CommandPaletteEntry

A searchable destination or safe action, derived at runtime from existing stores. Not persisted.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | string | Yes | Stable entry key, for example `server:<uuid>` |
| `label` | string | Yes | Primary display text |
| `category` | `servers` \| `favorites` \| `tabs` \| `actions` | Yes | Grouping |
| `subtitle` | string or null | No | Protocol/host/username, path, or shortcut |
| `keywords` | string[] | Yes | Searchable terms: host, username, group, protocol, path, aliases |
| `enabled` | boolean | Yes | Contextual availability |
| `disabledReason` | string or null | Conditional | Required when `enabled` is false |
| `action` | tagged action | Yes | Connect server, navigate favorite, activate tab, or run safe action |

**Rules**:
- Destructive actions and transfer-queue controls are never registered.
- Empty query shows recent history entries plus suggested safe actions.
- No matches show an empty state while the query remains editable.
- Running an entry closes the palette and returns focus to the previously focused control.
- An action that becomes unavailable while open is shown disabled with a reason or fails with an actionable message.

### NavigationLocation and BreadcrumbSegment

The current folder shown in a pane and its ordered path representation. Runtime only.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `path` | string | Yes | Current folder path |
| `segments` | BreadcrumbSegment[] | Yes | Ordered root to current |
| `editableValue` | string | Yes | Draft used in editable mode |
| `hasLocation` | boolean | Yes | False for the disconnected remote hub |

| BreadcrumbSegment field | Type | Required | Notes |
|---------|------|----------|-------|
| `label` | string | Yes | Display label, root label for the root |
| `path` | string | Yes | Full path to navigate to |
| `isCurrent` | boolean | Yes | Marks the last segment |
| `fullPath` | string | Yes | Exposed to assistive technology |

**Rules**:
- Both panes show breadcrumbs whenever they have a location.
- Activating a segment navigates that pane to the segment's path.
- The current segment activates editable mode; Enter navigates, Escape restores the previous display.
- Invalid or inaccessible paths leave the pane at its current folder and show one actionable notice.
- Long paths collapse middle segments into an overflow control while root, current, and every segment's full path remain accessible.
- The disconnected hub has no location, so its breadcrumb bar is hidden.

## Relationships and Invariants

- One Connection Profile has exactly one Server Favorite. The existing backfill behavior in `ServerFavoriteService::ensure_favorites` remains the invariant enforcer.
- A Server Favorite may have zero, one, or two saved paths. Each path is independent.
- A Connection History Entry references a saved server by `connectionProfileId` and optionally `favoriteId`; it does not own or lock either record.
- A Workspace Layout is a singleton for the desktop installation.
- A Discovered SSH Key is not stored; only the chosen path is stored on the Connection Profile.
- Command Palette entries and Navigation Locations are projections of existing state and are never written to metadata.
- No persisted entity contains a password, passphrase, private-key contents, token, file data, or failure detail.

## Persistence Shape (version 2)

The metadata document remains a single camelCase JSON object with `schemaVersion` plus the flattened `AppState`. New keys are additive:

```text
{
  "schemaVersion": 2,
  "connectionProfiles": [...],
  "directoryFavorites": [...],
  "credentialRefs": [...],
  "preference": {...},
  "tabs": [...],
  "serverFavorites": [
    {
      ...existing fields,
      "localPath": "/Users/example/Projects",   // optional
      "remotePath": "/srv/www"                  // optional
    }
  ],
  "favoritePreferences": {...},
  "workspaceLayout": {
    "splitRatio": 0.5,
    "sidebarVisible": true,
    "sidebarSections": {
      "servers": true,
      "directoryFavorites": true,
      "automation": false
    },
    "hubTab": "favorites"
  },
  "connectionHistory": [
    {
      "favoriteId": "...",
      "connectionProfileId": "...",
      "displayName": "Production",
      "protocol": "sftp",
      "host": "example.com",
      "port": 22,
      "username": "deploy",
      "lastUsedAt": "2026-11-05T12:00:00Z"
    }
  ]
}
```

Schema history:

| Version | Change |
|---------|--------|
| 0 | Pre-versioned documents; shape-compatible |
| 1 | Explicit `schemaVersion` and server-favorites metadata |
| 2 | Additive `workspaceLayout`, `connectionHistory`, and optional favorite paths |

Version 1 documents load with defaults for all new fields. Version 0 migrates through 1 to 2. Unknown future versions fail closed.
