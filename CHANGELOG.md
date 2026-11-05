# Changelog

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
