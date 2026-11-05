# Server Favorites

Server favorites reuse the existing connection-profile and credential models. Every saved connection profile has exactly one server favorite.

## Create

Create a favorite from the favorites view with a display name, protocol, host, port, username, authentication method, timeout, and optional group label. You can save credentials securely or choose to be asked at connection time. SSH private-key authentication stores only a protected key reference.

## Organize

The favorites list supports:

- Pinned entries first.
- Sorting by name, protocol, host, or last used.
- Single-level group labels.
- Rename, edit, and delete.

Deleting a favorite removes its saved connection profile and favorite metadata. Active connections remain running. Directory favorites and open tabs that reference the deleted profile are removed without deleting unrelated data.

## Connect

Selecting a favorite connects with its saved settings. If credentials are missing, rejected, or unavailable, the application prompts for new credentials. Session-only credentials are never persisted.

## Credentials

- Saved credentials live only in the native operating system credential store.
- The configuration file stores only a credential reference.
- A credential reference is bound to protocol, host, port, username, and authentication method.
- Shared credential updates list every affected favorite before confirmation.
- Deleting the last favorite that references a credential asks whether to keep or delete the secret.

## Recovery

If a favorite references a missing or corrupt credential, the favorite remains visible with a repair or re-enter-credentials action. Configuration writes are atomic and retain a backup. A corrupt document recovers to the backup or a safe state without exposing secrets.

## Accessibility

The favorites view supports keyboard navigation, visible focus, ARIA labels for protocol and credential state, and dialog focus return.

## Implementation Structure

The implemented layout intentionally keeps the new services cohesive rather than splitting every planned file:

- `crates/thriftp-core/src/server_favorites/mod.rs` owns the application service and credential-reference helpers; deterministic ordering lives in `server_favorites/sorting.rs`.
- `crates/thriftp-storage-json/src/server_favorites.rs` owns schema migrations, while the shared metadata adapter in `lib.rs` owns atomic writes, permissions, and backup recovery.
- Desktop credential commands live beside the favorites commands in `apps/desktop/src-tauri/src/commands/server_favorites.rs`.
- Shared IPC types remain in `apps/desktop/src/lib/ipc/types.ts`, with the favorites API/store modules layered on top.

This is a recorded deviation from the file-by-file layout in `plan.md`; behavior, ownership boundaries, and the Tauri-independent core remain unchanged.

## Local and remote working paths

Server favorites may store optional absolute `localPath` and `remotePath`
values. Saving from an active session pre-fills the current pane paths.
Reconnecting restores both; an unavailable path falls back to the nearest
accessible ancestor and then to home or the server login folder with a single
non-blocking notice. Paths are non-secret and never alter credentials or an
active connection.
