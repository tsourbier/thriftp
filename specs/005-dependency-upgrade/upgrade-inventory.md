# Upgrade Inventory

The authoritative machine-readable inventory is [inventory.json](./inventory.json). This document summarizes the upgrade batches.

## Batch 1 — Baseline

Baseline captured in [baseline.md](./baseline.md) before any upgrade edit, including the starting commit, toolchains, test results, and audit state.

## Batch 2 — Toolchain, CI, and containers

- Rust 1.88.0 → 1.99.1; workspace MSRV → 1.90.0.
- Node.js 22.23.0 → 26.13.0 engine baseline.
- CI actions pinned to verified commit SHAs.
- FTP test image `alpine:3.20` → `alpine:3.23`.
- SFTP test image `debian:12-slim` → `debian:13-slim`.

## Batch 3 — Rust dependencies

Upgraded utility crates, then SSH/FTP/TLS, then cryptography, then the credential store, then benchmarks. The full list, including versions and reasons, is in [inventory.json](./inventory.json).

## Batch 4 — Frontend tooling

Upgraded Vite, Vitest, ESLint, TypeScript, jsdom, Prettier, and their plugins. TypeScript 7 is deferred with exception `EX-001` because `typescript-eslint` still declares `<6.1.0` support.

## Batch 5 — Desktop runtime

Aligned `tauri`, `tauri-build`, `tauri-plugin-dialog`, `@tauri-apps/api`, and `@tauri-apps/cli` on the Tauri 2 stable line.

## Batch 6 — Cleanup and evidence

Evidence is recorded in [evidence.json](./evidence.json) and summarized in [validation.md](./validation.md). Exceptions are in [exceptions.json](./exceptions.json) and [exceptions.md](./exceptions.md).
