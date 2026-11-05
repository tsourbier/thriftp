# Data Model: Server Favorites Manager

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

## Entity Overview

```text
ConnectionProfile 1 ─── 1 ServerFavorite
ServerFavorite 0..1 ─── 0..1 CredentialRef
CredentialRef 0..* ─── 0..* ServerFavorite   (explicit reuse, matching identity only)
CredentialRef 0..1 ─── 1 Credential         (secret held in OS credential store)
ServerFavorite 0..* ─── 0..* ActiveConnection (runtime only)
ConnectionProfile 0..1 ─── 1 FavoriteGroupLabel (derived, no separate record)
```

## Entities

### ConnectionProfile (reused)

The existing saved-server configuration. This feature extends its use but does not create a parallel model.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Stable identifier |
| `displayName` | string | Yes | User-defined name, 1–200 characters after trimming |
| `groupLabel` | string or null | No | Optional, single-level, 1–100 characters after trimming |
| `protocol` | enum | Yes | `ftp`, `ftpsExplicit`, `ftpsImplicit`, `sftp` |
| `host` | string | Yes | 1–255 characters |
| `port` | integer | Yes | 1–65535 |
| `username` | string | Yes | Protocol-defined account name |
| `authMethod` | enum | Yes | `password`, `sshPrivateKey` |
| `timeoutSeconds` | integer | Yes | 1–300; default 30 |
| `sshKeyRef` | string or null | Conditional | Required for SFTP private-key authentication; opaque reference to protected key material, never the key itself |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Validation rules**:
- FTP and FTPS require password authentication.
- SFTP supports password and private-key authentication.
- Private-key authentication requires a non-empty `sshKeyRef` and must not store key material in the profile.
- Display names may repeat across profiles.
- The Connection Profile never stores a password, private key, passphrase, or other secret.

### ServerFavorite

The favorites-facing record for a saved connection. Every saved Connection Profile has exactly one Server Favorite.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Stable identifier |
| `connectionProfileId` | UUID | Yes | Unique; references a Connection Profile |
| `pinned` | boolean | Yes | Presentation only; default false |
| `lastUsedAt` | timestamp or null | No | Most recent successful connection, UTC |
| `credentialPolicy` | enum | Yes | `save` or `ask` |
| `credentialRefId` | UUID or null | Conditional | Required when policy is `save`; null when policy is `ask` |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Derived/presented fields** (not separately persisted):
- `displayName`, `groupLabel`, `protocol`, `host`, `port`, and `username` come from the referenced Connection Profile.
- `hasSavedCredential` is true when `credentialPolicy = save` and the referenced secret exists or is expected to exist.
- `pinnedFirst` ordering is applied by the favorites comparator.

**Validation rules**:
- Exactly one Server Favorite per Connection Profile.
- `credentialPolicy = save` requires a `credentialRefId`.
- `credentialPolicy = ask` has no `credentialRefId`.
- Pinning does not change connection settings or credentials.
- Deleting the favorite deletes the Connection Profile; active connections remain.

### CredentialRef

A non-secret identifier linking favorites to secret material in the OS credential store.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Opaque reference; used as the credential-store account |
| `protocol` | enum | Yes | Must match every referencing connection profile |
| `host` | string | Yes | Must match every referencing connection profile |
| `port` | integer | Yes | Must match every referencing connection profile |
| `username` | string | Yes | Must match every referencing connection profile |
| `authMethod` | enum | Yes | Must match every referencing connection profile |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Relationships**:
- Zero or more Server Favorites may reference a CredentialRef when their identity fields match.
- A CredentialRef may exist with zero favorites when the user chooses to keep the credential after deleting the last referencing favorite.
- An orphaned CredentialRef is not shown in the favorites list but remains available for explicit reuse.

**Validation rules**:
- A CredentialRef never contains a secret.
- A favorite may reference a CredentialRef only when protocol, host, port, username, and auth method match.
- Changing any identity field invalidates the previous reference for that favorite.
- Updating a shared CredentialRef updates the secret for every referencing favorite.
- Splitting a shared reference requires an explicit user action.

### Credential (external)

Not persisted in application configuration.

| Field | Type | Notes |
|-------|------|-------|
| Service | fixed string | `dev.thriftp.credentials` |
| Account | string | CredentialRef UUID |
| Secret | opaque bytes | Password, key passphrase, or protected key material; held by OS credential store |
| Kind | enum | `password` or `privateKey` or `passphrase` |

**Rules**:
- Never serialized to JSON.
- Never returned to the Svelte UI.
- Wrapped in a zeroizing type in Rust and dropped as soon as possible.
- Never included in logs, errors, diagnostics, crash reports, or telemetry.

### FavoriteGroupLabel

A derived grouping label, not a separate persisted entity.

- Stored as `ConnectionProfile.groupLabel`.
- Single-level only.
- No empty group records; a group exists while at least one favorite uses its label.
- Renaming a label updates all assigned favorites.
- Removing the label leaves favorites ungrouped and intact.

### ActiveConnection (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Runtime session identifier |
| `favoriteId` | UUID | Favorite used to start the connection |
| `state` | enum | `connecting`, `connected`, `disconnecting`, `error` |
| `startedAt` | timestamp | UTC |
| `error` | error or null | Redacted, user-facing error |

**Rules**:
- Not persisted across application restarts.
- Editing or deleting the favorite does not change an existing ActiveConnection.
- New connections use the latest saved settings.
- Last-used is updated only on a successful connection.

### FavoritePreference

| Field | Type | Notes |
|-------|------|-------|
| `sortKey` | enum | `name`, `protocol`, `host`, `lastUsed` |
| `pinnedFirst` | boolean | Always true in the initial release; stored for forward compatibility |

**Rules**:
- Default `sortKey = name`.
- Last-used sorting is most recent first; other sorts are ascending.
- Preferences persist across restarts.

## Persistence Schema

JSON document in the OS application-data directory. No secret material is permitted.

```json
{
  "schemaVersion": 1,
  "connectionProfiles": [],
  "favorites": [],
  "credentialRefs": [],
  "preferences": {
    "sortKey": "name",
    "pinnedFirst": true
  }
}
```

**Storage rules**:
- Unique: `connectionProfiles.id`, `favorites.id`, `favorites.connectionProfileId`, `credentialRefs.id`.
- Write through a temporary file in the same directory, flush, and atomically rename.
- Keep a backup of the previous valid document.
- Use restrictive permissions: `0600` on Unix; the platform application-data directory otherwise.
- Include `schemaVersion` and run ordered migrations on startup.
- On corrupt data, recover from the backup or a safe empty state without deleting unrelated valid data.
- Credentials are never written to this file.

## State Transitions

### Server Favorite

```text
Created → Saved
Saved → Pinned / Unpinned
Saved → Edited
Saved → Credential attached
Saved → Credential removed
Saved → Deleted → (Connection Profile removed)
```

### Credential Reference

```text
Created → Referenced
Referenced → Shared (explicit reuse with matching identity)
Referenced → Updated (affects all referencing favorites)
Referenced → Unlinked (favorite deleted)
Unlinked → Deleted (only after last reference and confirmation)
Unlinked → Retained as orphan (user chose to keep)
```

### Active Connection

```text
Idle → Connecting → Connected → Disconnecting → Disconnected
                  ↘ Error
```

Active connections are independent of favorite edits and deletion.

## Invariants

1. Every saved Connection Profile has exactly one Server Favorite.
2. A CredentialRef identity matches every referencing Connection Profile.
3. No secret appears in the JSON document or any serializable model.
4. A shared credential update applies to all referencing favorites.
5. Deleting the last reference prompts before credential deletion.
6. Pinning affects display order only.
7. Group labels are single-level and derived from the Connection Profile.
8. Editing or deleting a favorite never interrupts an active connection.
9. Failed connections and connection tests do not update `lastUsedAt`.
10. The core model and services have no Tauri dependency and are reusable by future CLI/MCP interfaces.
