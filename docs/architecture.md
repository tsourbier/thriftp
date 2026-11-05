# Architecture

Thriftp is split into a Tauri-independent Rust core, protocol adapters, native
integrations, versioned JSON persistence, and a Tauri/Svelte desktop shell.

## Crates

| Crate | Responsibility |
|-------|----------------|
| `thriftp-core` | Domain models, validation, connection and credential services, browsing, transfer engine, queue, recovery, preferences, favorites, previews, repository traits |
| `thriftp-protocols` | `RemoteFs` adapters for FTP, explicit FTPS, implicit FTPS, and SFTP |
| `thriftp-credentials-keyring` | Native operating-system credential store adapter (`keyring`) |
| `thriftp-storage-json` | Versioned, atomic JSON metadata and transfer-state stores |
| `apps/desktop/src-tauri` | Tauri commands, events, lifecycle, and application state |
| `apps/desktop/src` | Svelte 5 dual-pane UI and typed IPC client |

The dependency direction is always inward: `thriftp-core` never depends on
Tauri, Svelte, keyring, or JSON storage. Tauri commands stay thin and delegate to
core services.

## Protocol boundary

All protocol behavior is behind `RemoteFs`:

```text
connect / disconnect / capabilities
list_dir / stat / mkdir / rename / delete_file / delete_dir / chmod
download_to / upload_from
```

Transfer methods stream in bounded chunks and call a progress callback after
every chunk. Returning `false` from the callback stops at a chunk boundary so the
engine can pause or cancel without buffering the whole file.

FTP/FTPS uses `suppaftp`; SFTP uses `russh` and `russh-sftp`. FTP servers that do
not implement `MLSD` fall back to parsing `LIST`. The TLS stack deviation from
the plan is recorded in [ADR 0001](adr/0001-ftp-tls-stack.md).

## Transfer engine

- `transfers/stream.rs`: bounded chunk copy and progress accounting.
- `transfers/io.rs`: local reader/writer adapters.
- `transfers/engine.rs`: lifecycle, pause/cancel, and error mapping.
- `transfers/recursive.rs`: directory planning and symlink policy.
- `transfers/conflict.rs`: overwrite/skip/rename and apply-to-all.
- `transfers/control.rs`: shared atomics and speed/ETA estimation.
- `transfers/resume.rs`: partial naming, metadata, and verification.
- `recovery.rs`: retained/discarded partial state and startup recovery.

The Tauri shell starts workers, emits `transfer:progress` and `transfer:state`,
and persists queue snapshots.

## Persistence

Metadata and transfers are written through `MetadataStore` and `TransferStore`.
The JSON adapter checks a schema version, rejects unsupported versions, and uses
an atomic write (temporary file + rename). Secrets are never stored in JSON.

## Dependency governance

Dependency changes are recorded in `specs/005-dependency-upgrade/inventory.json`,
verified through `evidence.json`, and validated by
`scripts/validate-dependency-records.py`. CI actions are pinned to verified
commit SHAs. Time-bound exceptions live in `exceptions.json` and remain visible
to reviewers.

## Security invariants

- Secrets live only in zeroizing memory or the native credential store.
- Errors sent to the UI are redacted; raw diagnostics stay in local logs.
- TLS certificates and SSH host keys are verified by default.
- Plain FTP requires an explicit warning and confirmation.
- Remote deletes require explicit confirmation; local deletes use the OS trash.

## Workspace state

Workspace layout, connection history, and optional favorite working paths are
non-secret metadata in the versioned JSON document (schema version 2). They are
owned by `thriftp-core` services, mapped by thin Tauri commands, and projected by
the Svelte workspace components. No new runtime service, database, or secret
store is introduced.
