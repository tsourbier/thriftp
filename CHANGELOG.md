# Changelog

## 2026-11-06 — Browser and favorites workflow polish

### Scope

- Added native Shift-range and Cmd/Ctrl toggle selection plus sortable Name, Size, and Modified headers.
- Added keyboard Shift/Cmd/Ctrl selection, restored per-tab sort/filter settings, and exact completed-volume accounting in queue statistics.
- Added linked local/remote navigation with relative-move mirroring for every navigation action, including a readability probe before the counterpart moves.
- Added per-connection queue statistics for completed files, completed volume, and average speed.
- Added detailed permission editing, confirmed recursive permission changes, and path copying to the file inspector.
- Moved Disconnect beside the remote title and removed back/forward buttons from pane toolbars.
- Added local-folder browsing and reachable actions to favorite dialogs, and resolved active-tab paths before saving a favorite.

### Compatibility and security

- Local permissions remain read-only; recursive remote changes require explicit confirmation and never follow symbolic links.
- Queue statistics and pane-link state are session-only.
- No new telemetry, account, cloud dependency, or secret storage path was introduced.

## 2026-11-05 — Dependency and toolchain upgrade

### Scope

- Upgraded the Rust workspace, Tauri desktop stack, frontend toolchain, CI actions, and protocol test containers to current stable releases.
- Pinned Rust to 1.99.1 and Node.js to 26.13.0 LTS.
- Pinned CI actions to verified commit SHAs.
- Regenerated `Cargo.lock`, `apps/desktop/src-tauri/Cargo.lock`, and `apps/desktop/package-lock.json`.

### User action

- Install Node.js 26.13.0+ and Rust 1.90.0+ (the repository pins 1.99.1).
- Run `npm ci` in `apps/desktop` to install the regenerated frontend dependency graph.

### Compatibility and security

- No intentional user-facing behavior change.
- CLI, MCP, desktop IPC, protocol, credential, transfer, favorites, and saved-data contracts remain compatible.
- No new telemetry, account, cloud dependency, or plaintext secret path.
- TypeScript 7 is deferred by exception `EX-001`; the residual low-severity `deepmerge` advisory is covered by `EX-002`.
