# AGENTS.md

Guidance for AI coding agents working in this repository. Read this file before
making changes. The [project constitution](.specify/memory/constitution.md) is
authoritative; if anything here conflicts with it, the constitution wins.

## What this project is

Thriftp is a cross-platform desktop file transfer client for FTP, FTPS, and
SFTP. The same Rust core powers three surfaces:

- the Tauri 2 + Svelte 5 desktop app,
- the `thriftp` command-line interface,
- the local `thriftp-mcp` Model Context Protocol server.

## Read first

1. [`.specify/memory/constitution.md`](.specify/memory/constitution.md) — project principles and quality gates.
2. The active feature under `specs/` (the path is in `.specify/feature.json`).
   Read its `spec.md`, `plan.md`, and `tasks.md` before implementing.
3. [`docs/architecture.md`](docs/architecture.md), [`docs/security.md`](docs/security.md),
   and [`docs/testing.md`](docs/testing.md).
4. [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Repository layout

| Path | Responsibility |
|------|----------------|
| `crates/thriftp-core` | Domain models, validation, browsing, transfers, queue, recovery, repository traits |
| `crates/thriftp-protocols` | `RemoteFs` adapters for FTP, FTPS, and SFTP |
| `crates/thriftp-credentials-keyring` | Native OS credential store adapter |
| `crates/thriftp-storage-json` | Versioned, atomic JSON metadata and transfer stores |
| `crates/thriftp-cli` | `thriftp` binary; a thin adapter over the core |
| `crates/thriftp-mcp` | `thriftp-mcp` binary and embedded server; a thin adapter over the core |
| `crates/thriftp-tests` | Shared cross-crate, multi-process, security, and end-to-end tests |
| `apps/desktop/src-tauri` | Tauri commands, events, lifecycle, and app state |
| `apps/desktop/src` | Svelte 5 UI and typed IPC client |
| `tests/integration` | Containerized protocol integration tests |
| `specs/` | Feature specifications, plans, contracts, and task lists |

## Setup

Prerequisites: the Rust toolchain pinned by `rust-toolchain.toml` (1.99.1),
Node.js 26.13.0+, the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/),
and `npm` for the desktop UI. Docker is only needed for protocol integration tests.

```bash
cd apps/desktop
npm ci
npx tauri dev        # launch the desktop app
```

## Validation

Run the checks relevant to your change. The full pre-merge set:

```bash
# Rust workspace (does not include apps/desktop/src-tauri)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

# Tauri shell (separate crate, not a workspace member)
cargo test --locked --manifest-path apps/desktop/src-tauri/Cargo.toml

# Desktop frontend
cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build

# Protocol integration tests (requires a Docker-compatible runtime; skips cleanly without one)
scripts/test-integration.sh
```

`scripts/validate-contributor-build.sh` performs a lighter build-only check.

## Development workflow

1. Work from a written specification. Significant changes belong in a
   `specs/NNN-name/` directory with `spec.md`, `plan.md`, and `tasks.md`.
   The Spec Kit commands are in `.opencode/commands/` (`speckit.specify`,
   `speckit.plan`, `speckit.tasks`, `speckit.implement`, and related).
2. Keep `specs/`, `docs/`, and the implementation synchronized. If behavior
   changes, update the relevant documentation in the same change.
3. Add or update tests with every behavior change. Run the relevant tests
   before reporting a task complete.
4. Keep changes focused. YAGNI applies; do not add speculative abstractions or
   unrelated refactors.
5. Do not edit `Cargo.lock` or `package-lock.json` except for a deliberate
   dependency change, and record dependency changes in the inventory under
   `specs/005-dependency-upgrade/`.

## Architecture constraints

- `thriftp-core` MUST NOT depend on Tauri, Svelte, JSON storage, or the keyring.
  It must build and test without Tauri.
- All protocol behavior belongs behind the `RemoteFs` interface. Adding a
  protocol must not require changes to core modules.
- Tauri commands, the CLI, and the MCP server are thin adapters. Put business
  logic in core services, not in command handlers or tool definitions.
- Transfers stream in bounded chunks with O(1) memory. Never buffer a whole
  file, and never block the UI thread with network, filesystem, or CPU-heavy work.
- Persist metadata and transfer state only through `MetadataStore` and
  `TransferStore`. Writes must be atomic and schema-versioned.
- Network and application events go through `tracing`; respect secret redaction.
- `unsafe` code is forbidden in the workspace.
- Public Rust APIs must be documented. Production code must not panic on
  recoverable errors.

## Security invariants

- Never store secrets in plaintext, and never put passwords, private keys, or
  tokens in arguments, environment variables, logs, errors, fixtures, or JSON.
  Use zeroizing memory or the native credential store.
- TLS certificates and SSH host keys are verified by default. Any override must
  be explicit, user-visible, and scoped to the session.
- Plain FTP must warn before and during use.
- MCP is disabled by default, local-only, and requires explicit enrollment and
  authorization. Destructive MCP operations require confirmation or an approved
  policy and must be audited.
- Remote deletes require explicit confirmation; local deletes use the OS trash.
- Keep dependencies and CI actions pinned. Do not add a dependency without a
  clear justification and a license/security review.

## Style

- Rust: edition 2024, `rustfmt` (max width 100), clippy with `-D warnings`.
  Follow the module layout and naming of the surrounding code.
- Frontend: Svelte 5 and TypeScript. Prettier uses double quotes, no trailing
  commas, and 90-character lines. ESLint must pass.
- Tests live next to the code they cover (`*.test.ts` for frontend, `tests/`
  and inline `#[cfg(test)]` modules for Rust).
- Do not commit generated output such as `target/`, `dist/`, `node_modules/`,
  logs, or local environment files.

## Common pitfalls

- `cargo test --workspace` does not test the Tauri shell; run the
  `apps/desktop/src-tauri` command separately.
- Docker is optional for unit tests. Never make unit tests depend on it.
- The desktop build requires the Tauri platform prerequisites; a plain
  `cargo build --workspace` does not build the desktop binary.
- Check `.specify/feature.json` for the active feature instead of assuming the
  highest-numbered spec directory.
