# Data Model: Thriftp Desktop File Transfer Client

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

## Entity Overview

```text
ConnectionProfile 1 ─── 0..* DirectoryFavorite
ConnectionProfile 1 ─── 0..1 ServerFavorite (defined in 003)
ConnectionProfile 1 ─── 0..* BrowsingSession/Tab
ConnectionProfile 0..* ─── 0..* CredentialRef
CredentialRef 0..1 ─── 1 Credential (OS credential store)
ConnectionProfile 1 ─── 0..* Transfer
Transfer 0..1 ─── 1 PartialTransferData
TransferQueue 1 ─── 0..* Transfer
TransferHistory 1 ─── 0..* completed Transfer snapshots
BrowsingSession 1 ─── 0..* DirectoryListing
ConnectionProfile 0..* ─── 0..* ActiveConnection (runtime)
```

## Entities

### ConnectionProfile

A saved configuration for reaching a server. This feature extends its use but does not create a parallel model.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Stable identifier |
| `displayName` | string | Yes | 1–200 characters after trimming |
| `groupLabel` | string or null | No | Optional, single-level, 1–100 characters |
| `protocol` | enum | Yes | `ftp`, `ftpsExplicit`, `ftpsImplicit`, `sftp` |
| `host` | string | Yes | 1–255 characters |
| `port` | integer | Yes | 1–65535 |
| `username` | string | Yes | Protocol account name |
| `authMethod` | enum | Yes | `password`, `sshPrivateKey` |
| `timeoutSeconds` | integer | Yes | 1–300; default 30 |
| `sshKeyRef` | string or null | Conditional | Required for SFTP private-key auth; opaque protected reference, never key material |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Rules**: FTP/FTPS require password authentication; SFTP supports password and private-key authentication. No secret is stored in the profile.

### ServerFavorite (owned by `003`)

Server favorites are defined in `specs/003-server-favorites/data-model.md`. Feature `001` provides the shared `ConnectionProfile` and `CredentialRef` interfaces they use, but does not redefine the server-favorite record.

### DirectoryFavorite

A saved remote directory associated with a Connection Profile. This remains distinct from a Server Favorite.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Stable identifier |
| `connectionProfileId` | UUID | Yes | References a Connection Profile |
| `path` | string | Yes | Canonical remote path |
| `label` | string | Yes | User-visible name, 1–200 characters |
| `sortOrder` | integer | Yes | Ordering within the profile's directory favorites |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Rules**: Directory favorites are not server favorites; they do not create a connection profile. A missing profile makes the directory favorite unavailable but does not delete it silently.

### CredentialRef

A non-secret reference to a credential held in the OS credential store.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Keyring account |
| `protocol` | enum | Yes | Identity binding |
| `host` | string | Yes | Identity binding |
| `port` | integer | Yes | Identity binding |
| `username` | string | Yes | Identity binding |
| `authMethod` | enum | Yes | Identity binding |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Rules**: Never contains a secret. May be shared only when identity fields match. Changing identity fields invalidates the reference for that favorite. Shared updates apply to every referencing favorite.

### Credential (external)

Held in the OS credential store under service `dev.thriftp.credentials` and account `CredentialRef.id`.

| Field | Notes |
|-------|-------|
| Service | Fixed string |
| Account | CredentialRef UUID |
| Secret | Zeroized password, key passphrase, or protected key material |
| Kind | `password`, `privateKey`, or `passphrase` |

**Rules**: Never serialized, never returned to the UI, never logged.

### BrowsingSession / Tab

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Tab identifier |
| `connectionProfileId` | UUID or null | Null for local-only tabs |
| `currentPath` | string | Local or remote path |
| `history` | string[] | Back/forward navigation history |
| `historyIndex` | integer | Current position |
| `viewSettings` | object | Sort, filter, hidden-file visibility |
| `selectedItems` | string[] | Selection state; may be cleared on restore |
| `createdAt` | timestamp | UTC |
| `restored` | boolean | Whether the tab was restored at launch |

**Rules**: Tabs persist by default and restore on launch unless the user disabled restoration. A restored tab that cannot reconnect shows a disconnected state with retry and does not block other tabs.

### FileEntry

| Field | Type | Notes |
|-------|------|-------|
| `name` | string | Display name |
| `path` | string | Full local or remote path |
| `entryType` | enum | `file`, `directory`, `symlink`, `other` |
| `size` | integer or null | Bytes |
| `modifiedAt` | timestamp or null | UTC |
| `permissions` | string or null | Unix mode or protocol representation |
| `ownership` | object or null | Owner/group where available |
| `hidden` | boolean | Hidden-file status |
| `symlinkTarget` | string or null | Target for display; never followed for recursive transfer |
| `capabilities` | object | Which metadata fields are supported |

### Transfer

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Stable identifier |
| `direction` | enum | `upload`, `download` |
| `connectionProfileId` | UUID | Source/destination profile |
| `sourcePath` | string | Local or remote path |
| `destinationPath` | string | Local or remote path |
| `totalBytes` | integer or null | Null when unknown |
| `transferredBytes` | integer | Last confirmed bytes |
| `state` | enum | `pending`, `active`, `paused`, `completed`, `failed`, `cancelled`, `interrupted` |
| `priority` | enum | `low`, `normal`, `high` |
| `order` | integer | Queue ordering tie-breaker |
| `speedBytesPerSecond` | integer or null | Runtime estimate |
| `estimatedRemainingSeconds` | integer or null | Runtime estimate |
| `resumable` | boolean | Based on protocol/server capability |
| `partialDataId` | UUID or null | Link to PartialTransferData |
| `error` | object or null | Redacted error code and message |
| `createdAt` | timestamp | UTC |
| `updatedAt` | timestamp | UTC |
| `startedAt` | timestamp or null | UTC |
| `completedAt` | timestamp or null | UTC |

**Rules**: Progress never exceeds total when known; completed requires 100%; interrupted/failed preserve last confirmed progress; a paused transfer consumes no transfer bandwidth.

### PartialTransferData

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Stable identifier |
| `transferId` | UUID | Owning transfer |
| `localPath` | string or null | Local temporary/incomplete file |
| `remotePath` | string or null | Remote partial file |
| `offset` | integer | Last confirmed byte position |
| `sourceSize` | integer or null | Size at start |
| `sourceModifiedAt` | timestamp or null | Modification time at start |
| `createdAt` | timestamp | UTC |
| `updatedAt` | timestamp | UTC |

**Rules**: Local partial files are identifiable and never presented as complete; remote cleanup depends on protocol support; resume verification must compare source and destination metadata.

### TransferQueue

A singleton persisted structure.

| Field | Type | Notes |
|-------|------|-------|
| `concurrencyLimit` | integer | Default 4; configurable |
| `bandwidthLimitBytesPerSecond` | integer or null | Null means unlimited |
| `paused` | boolean | Queue-level pause |
| `orderedTransferIds` | UUID[] | Pending/active/paused order |
| `activeTransferIds` | UUID[] | Currently running |
| `lastCheckpointAt` | timestamp | Last persisted progress checkpoint |

### TransferHistory

| Field | Type | Notes |
|-------|------|-------|
| `entries` | TransferSnapshot[] | Completed, failed, cancelled, interrupted records |
| `retentionLimit` | integer | 1,000 records |

**Rules**: New entries evict the oldest beyond 1,000. Users can clear history. History does not include active queue state.

### Preference

| Field | Type | Notes |
|-------|------|-------|
| `theme` | enum | `system`, `light`, `dark` |
| `showHiddenFiles` | boolean | Per-panel default |
| `defaultConcurrency` | integer | Default 4 |
| `bandwidthLimitBytesPerSecond` | integer or null | Null means unlimited |
| `restoreTabs` | boolean | Default true |
| `previewMode` | enum | `native`, `internal`, `auto` |
| `sortKey` | enum | Name, size, date, type |
| `reducedMotion` | boolean | Accessibility preference |

### ActiveConnection (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Runtime identifier |
| `connectionProfileId` | UUID | Source profile |
| `state` | enum | `connecting`, `connected`, `disconnecting`, `error` |
| `startedAt` | timestamp | UTC |
| `error` | object or null | Redacted |

**Rules**: Not persisted; editing or deleting the profile does not interrupt it.

### DirectoryListing (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `sessionId` | UUID | Tab |
| `path` | string | Listing path |
| `state` | enum | `loading`, `partial`, `complete`, `error` |
| `entries` | FileEntry[] | Entries received so far |
| `firstEntryAt` | timestamp or null | Time to first visible entry |
| `completedAt` | timestamp or null | Time complete |
| `error` | object or null | Redacted |

### ProtocolCapabilities

| Field | Type | Notes |
|-------|------|-------|
| `resume` | boolean | Transfer resumption |
| `permissions` | boolean | View/modify Unix permissions |
| `ownership` | boolean | View ownership |
| `hiddenFiles` | boolean | Hidden-file listing |
| `symbolicLinks` | boolean | Preserve/list links |
| `serverSideRename` | boolean | Rename support |
| `serverSideDelete` | boolean | Delete support |
| `maxPathLength` | integer or null | Protocol limit |

## Persistence

### `favorites.json` (shared metadata, extended from `003`)

```json
{
  "schemaVersion": 1,
  "connectionProfiles": [],
  "directoryFavorites": [],
  "credentialRefs": [],
  "preferences": {},
  "tabs": []
}
```

Feature `003` extends this document with its `favorites` array for server favorites.

### `transfers.json`

```json
{
  "schemaVersion": 1,
  "queue": {
    "concurrencyLimit": 4,
    "bandwidthLimitBytesPerSecond": null,
    "paused": false,
    "orderedTransferIds": [],
    "activeTransferIds": [],
    "lastCheckpointAt": null
  },
  "transfers": [],
  "partialTransfers": [],
  "history": []
}
```

**Storage rules**: No secrets; atomic replace with backup; restrictive permissions; schema migrations; checkpoint active progress every 5 seconds or 1 MB; cap history at 1,000 records; recover to backup or safe state on corruption.

## State Transitions

### Transfer

```text
Pending → Active → Completed
Pending → Active → Paused → Active
Pending → Active → Cancelled
Pending → Active → Failed
Pending → Active → Interrupted (unexpected termination)
Pending → Cancelled
Paused → Cancelled
Interrupted → Active (resume) | Cancelled
```

Resume requires source size/modification-time match and destination offset match; otherwise the user must restart or cancel.

### Queue

```text
Running → Paused → Running
```

Pausing the queue pauses active transfers at chunk boundaries and prevents pending starts.

### Browsing Session / Tab

```text
Open → Disconnected → Open (retry)
Open → Closed
Restored → Open | Disconnected
```

### Connection

```text
Connecting → Connected → Disconnecting → Disconnected
Connecting → Error
```

## Invariants

1. No secret appears in `favorites.json`, `transfers.json`, logs, errors, crash reports, or telemetry.
2. Directory favorite paths are unique per Connection Profile; server-favorite uniqueness is defined by `003`.
3. Every Transfer belongs to a valid queue entry; history entries do not occupy queue slots.
4. Pending queue order is unique and deterministic.
5. History contains at most 1,000 records.
6. Partial transfer data is linked to exactly one transfer.
7. A completed transfer has complete progress; interrupted/failed transfers preserve last confirmed progress.
8. Pausing or cancelling the queue stops active transfers at a safe chunk boundary.
9. Symbolic links are never followed during recursive transfers.
10. Conflict resolution never silently overwrites a destination without an explicit policy.
11. Editing or deleting a favorite/profile never interrupts an active connection.
12. The core and protocol traits have no Tauri dependency and remain reusable by CLI/MCP.
