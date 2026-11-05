# Contract: Favorites Tauri IPC

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

This contract defines the interface between the Svelte UI and the Rust core through Tauri commands and events. Command names are shown in snake_case, matching Rust/Tauri conventions; payload fields use camelCase JSON.

## Conventions

- All commands return a typed result or a structured error.
- Secrets are accepted only by commands that need them and are never returned to the UI.
- All identifiers are UUID strings.
- Timestamps are ISO-8601 UTC strings.
- The UI never writes favorites data directly to disk.
- The core validates all inputs; the UI must not rely on validation alone.

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

SortKey =
  | "name"
  | "protocol"
  | "host"
  | "lastUsed"

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
}

FavoritePreferences = {
  sortKey: SortKey
  pinnedFirst: boolean
}

CredentialImpact = {
  credentialRefId: string
  affectedFavorites: Array<{ id: string; displayName: string }>
  isShared: boolean
}

ReusableCredential = {
  credentialRefId: string
  protocol: Protocol
  host: string
  port: number
  username: string
  authMethod: AuthMethod
  affectedFavoriteCount: number
}

ConnectionState = {
  connectionId: string
  favoriteId: string
  state: "connecting" | "connected" | "disconnecting" | "error"
  error?: { code: string; message: string }
}

FavoritesError = {
  code:
    | "validation"
    | "not_found"
    | "conflict"
    | "credential_store_unavailable"
    | "credential_missing"
    | "credential_rejected"
    | "connection_failed"
    | "permission_denied"
    | "internal"
  message: string
  field?: string
  retryable?: boolean
}
```

## Commands

### `favorites_list`

- **Input**: none
- **Output**: `ServerFavoriteView[]`
- **Notes**: Returns every saved connection in the order defined by the persisted preference. The core owns the comparator: pinned entries first, then the selected sort key, with deterministic tie-breakers.

### `favorites_get`

- **Input**: `{ id: string }`
- **Output**: `ServerFavoriteView`
- **Errors**: `not_found`

### `favorites_create`

- **Input**:
  ```text
  {
    profile: ConnectionProfileInput
    credentialPolicy: CredentialPolicy
    pinned?: boolean
    credentialRefId?: string  // explicit reuse; identity fields must match
    secret?: string        // only when credentialPolicy = "save" and no credentialRefId; never returned
    secretKind?: "password" | "privateKey" | "passphrase"
  }
  ```
- **Output**: `ServerFavoriteView`
- **Behavior**:
  - Creates the Connection Profile and its one Server Favorite.
  - With `credentialPolicy = "save"`, requires either a new secret or an explicit `credentialRefId` whose identity fields match the profile.
  - With `credentialPolicy = "ask"`, ignores or rejects any supplied secret or reference.
  - Never writes a secret to the JSON document.
- **Errors**: `validation`, `conflict`, `credential_store_unavailable`, `credential_missing`

### `favorites_update`

- **Input**:
  ```text
  {
    id: string
    profile: ConnectionProfileInput
    credentialPolicy: CredentialPolicy
    pinned?: boolean
    credentialRefId?: string  // explicit reuse; identity fields must match
    secret?: string
    secretKind?: "password" | "privateKey" | "passphrase"
  }
  ```
- **Output**: `ServerFavoriteView`
- **Behavior**:
  - Changing protocol, host, port, username, or auth method invalidates the previous credential reference.
  - When invalidated, the user must supply a new secret, explicitly reuse an existing compatible reference, or explicitly clear the saved credential.
  - Switching `credentialPolicy` to `"ask"` removes the favorite's credential reference.
  - Updating a shared credential applies to all favorites that reference it and lists them before confirmation.
- **Errors**: `validation`, `not_found`, `conflict`, `credential_store_unavailable`, `credential_rejected`

### `favorites_delete`

- **Input**: `{ id: string }`
- **Output**:
  ```text
  {
    deleted: true
    orphanedCredentialRefId?: string
    credentialImpact?: CredentialImpact
  }
  ```
- **Behavior**:
  - Deletes the saved Connection Profile and Server Favorite.
  - Does not disconnect an active connection.
  - If the deleted favorite was the last reference to a credential, returns `orphanedCredentialRefId` and `credentialImpact` so the UI can ask whether to delete or keep the credential.
- **Errors**: `not_found`

### `credentials_delete`

- **Input**: `{ credentialRefId: string; confirmed: true }`
- **Output**: `{ deleted: true }`
- **Behavior**:
  - Deletes the secret from the OS credential store only when no favorite references it and the user confirmed.
  - Rejects if the reference is still in use.
- **Errors**: `conflict`, `not_found`, `credential_store_unavailable`

### `favorites_set_pinned`

- **Input**: `{ id: string; pinned: boolean }`
- **Output**: `ServerFavoriteView`
- **Errors**: `not_found`

### `favorites_set_group`

- **Input**: `{ id: string; groupLabel: string | null }`
- **Output**: `ServerFavoriteView`
- **Behavior**:
  - Assigns or removes the single-level group label for one favorite.
  - Assigning a new label creates the group implicitly.
  - Renaming a label for all assigned favorites uses `favorites_rename_group`.
- **Errors**: `validation`, `not_found`

### `favorites_rename_group`

- **Input**: `{ oldLabel: string; newLabel: string }`
- **Output**: `{ updated: ServerFavoriteView[] }`
- **Behavior**:
  - Renames the group label on every favorite that uses `oldLabel`.
  - Favorites remain intact and keep their other settings.
- **Errors**: `validation`, `not_found`

### `favorites_get_preferences`

- **Input**: none
- **Output**: `FavoritePreferences`

### `favorites_set_preferences`

- **Input**: `FavoritePreferences`
- **Output**: `FavoritePreferences`
- **Errors**: `validation`

### `favorites_test_connection`

- **Input**:
  ```text
  {
    profile: ConnectionProfileInput
    secret?: string
    secretKind?: "password" | "privateKey" | "passphrase"
  }
  ```
- **Output**: `{ ok: boolean; error?: { code: string; message: string } }`
- **Behavior**:
  - Tests the pending settings.
  - Does not persist the profile or any secret.
  - Does not update `lastUsedAt`.
- **Errors**: `validation`, `connection_failed`, `credential_rejected`

### `favorites_connect`

- **Input**: `{ id: string; secret?: string; secretKind?: "password" | "privateKey" | "passphrase" }`
- **Output**: `ConnectionState`
- **Behavior**:
  - Starts a connection using saved settings and credentials.
  - If credentials are missing or rejected, returns `credential_missing` or `credential_rejected`; the UI may prompt and retry with a session-only `secret`.
  - A session-only secret is never persisted and is discarded when the connection attempt or session ends.
  - Updates `lastUsedAt` only after a successful connection.
- **Errors**: `not_found`, `credential_missing`, `credential_rejected`, `connection_failed`

### `connections_disconnect`

- **Input**: `{ connectionId: string }`
- **Output**: `{ disconnected: true }`
- **Errors**: `not_found`

### `connections_get_profile`

- **Input**: `{ connectionId: string }`
- **Output**: `{ connectionId: string; favoriteId?: string; profile: ConnectionProfileInput; state: "connecting" | "connected" | "disconnecting" | "error" }`
- **Behavior**:
  - Returns the current settings of an active connection so the UI can pre-fill the save-as-favorite flow.
  - Does not return credentials.
- **Errors**: `not_found`

### `credentials_get_impact`

- **Input**: `{ favoriteId: string }`
- **Output**: `CredentialImpact`
- **Behavior**:
  - Returns the favorites that would be affected by updating or removing the credential.
  - Used before update/removal confirmation.
- **Errors**: `not_found`

### `credentials_list_reusable`

- **Input**: none
- **Output**: `ReusableCredential[]`
- **Behavior**:
  - Lists credential references that may be explicitly reused, with their identity fields and the number of favorites currently using them.
  - Does not return secrets.
  - The UI must offer reuse only when the new favorite's identity fields match.
- **Errors**: `credential_store_unavailable`

### `credentials_remove`

- **Input**: `{ favoriteId: string; confirmed: true }`
- **Output**: `{ removed: true; impact: CredentialImpact }`
- **Behavior**:
  - Removes the favorite's credential reference and keeps the favorite.
  - Sets the favorite's credential policy to `"ask"`.
  - When shared, lists affected favorites before removal.
  - When it is the last reference, the UI must obtain explicit confirmation before the underlying secret is deleted.
- **Errors**: `not_found`, `credential_store_unavailable`, `conflict`

## Events

### `favorites:changed`

```text
{ favoriteId: string; reason: "created" | "updated" | "deleted" | "pinned" | "groupChanged" | "credentialChanged" }
```

### `connection:state`

```text
ConnectionState
```

### `credentials:changed`

```text
{ credentialRefId: string; affectedFavoriteIds: string[]; reason: "updated" | "removed" | "deleted" }
```

## Security Rules

- Secret fields are accepted only by `favorites_create`, `favorites_update`, `favorites_test_connection`, and `credentials_*` commands that require them.
- No command returns a secret.
- Error payloads and events never contain secret values.
- The UI must treat `credential_missing`, `credential_rejected`, and `credential_store_unavailable` as distinct states with different recovery actions.
- The UI must not cache secrets in persistent frontend storage.
- Destructive commands require explicit confirmation from the UI before invocation.
