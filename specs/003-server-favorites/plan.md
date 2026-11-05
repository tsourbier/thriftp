# Implementation Plan: Server Favorites Manager

**Branch**: `003-server-favorites` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/003-server-favorites/spec.md`

## Summary

Implement a desktop-only Server Favorites Manager on top of the existing connection-profile model. Favorites store non-secret connection metadata in a versioned local JSON store, keep credentials exclusively in the operating system credential store through a `CredentialStore` abstraction, and expose operations to the Svelte UI through thin Tauri commands. All saved server connections appear in the favorites view and may be pinned, grouped with the existing connection-profile group label, sorted, edited, connected, and deleted. Deleting a favorite deletes the saved connection, leaves active sessions running, and deletes credentials only when no other favorite references them and the user confirms. Shared credential updates apply to every referencing favorite after listing them. Credential-store failures fail closed with no plaintext fallback. This feature adds no protocol-server integration tests; the shared `001` Docker Compose protocol suite remains a separate CI stage, and Docker is never a runtime dependency of the application.

## Technical Context

**Language/Version**: Rust stable (edition 2024, Rust 1.85.1 or newer); TypeScript 5.x; Svelte 5.x

**Primary Dependencies**: Tauri 2.x; `serde`/`serde_json`; `uuid`; `chrono`; `thiserror`; `keyring` for native OS credential storage; `zeroize`; `tracing`; Svelte; Vite; Vitest with Testing Library

**Storage**: Versioned JSON document for non-secret favorites, groups, preferences, and credential-reference metadata in the OS application-data directory, written atomically with restrictive permissions; passwords, private keys, and passphrases only in the OS credential store (macOS Keychain, Windows Credential Manager, Linux Secret Service)

**Testing**: `cargo test` for Docker-free core unit and integration tests using fake credential and repository implementations; Tauri command tests; Vitest component tests; platform-specific native credential-store smoke tests; cross-platform CI matrix; no protocol-server integration tests in this feature; shared `001` containerized protocol integration tests remain a separate CI stage

**Target Platform**: macOS, Linux, and Windows desktop

**Project Type**: Desktop application (Tauri 2 + Svelte)

**Performance Goals**: The favorites view renders and sorts 500 entries in under 100 ms on a typical professional workstation; credential lookup completes in under 1 second in normal conditions; credential and connection work never blocks the UI thread

**Constraints**: Offline-first; no plaintext secrets; no custom vault or master password; fail closed when the OS credential store is unavailable; no cloud or cross-device sync; desktop-only initial release; core independent of Tauri; data model reusable by future CLI/MCP interfaces; CI must run formatting, Clippy, tests, dependency audits, SBOM generation for releases, and builds on all supported platforms; dependencies and CI actions pinned; lockfiles committed and enforced; this feature's tests do not require a container runtime, and Docker-compatible runtimes used by the separate `001` protocol suite are never an application runtime dependency

**Scale/Scope**: At least 500 server favorites; one-level group labels; FTP, explicit FTPS, implicit FTPS, and SFTP; macOS, Linux, and Windows

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Product Vision | Improves quick reconnect and reliability without unrelated scope | PASS |
| II. Architecture | Rust core, Tauri 2 runtime, Svelte/TypeScript UI, core independent of Tauri, async work, justified dependencies, cohesive modules | PASS |
| III. Security | Native credential store only, no plaintext, no secret logging, certificate/host-key parity, SBOM/pinned dependencies/lockfiles | PASS |
| IV. User Experience | Dedicated favorites view, native conventions, accessibility, theme awareness, no proprietary assets | PASS |
| V. Quality | Automated Docker-free unit/integration/component tests; platform-specific credential-store tests; separate shared containerized protocol stage from `001`; cross-platform CI; Clippy; `rustfmt`; documented public interfaces; explicit errors | PASS |
| VI. Open-Source | MIT license, documentation, transparent dependencies | PASS |
| VII. AI-Assisted Development | Specification and acceptance criteria are authoritative, tests required, YAGNI applied | PASS |
| VIII. Automation & Integration | Favorites domain logic lives in the shared core and remains reusable; management through CLI/MCP is explicitly deferred by the spec and CLI/MCP specification | PASS |

No gate violations. No complexity exceptions are required.

## Project Structure

### Documentation (this feature)

```text
specs/003-server-favorites/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 validation guide
├── contracts/           # Phase 1 interface contracts
│   ├── favorites-ipc.md
│   └── credential-store.md
└── tasks.md             # Phase 2 output (/speckit.tasks, not created here)
```

### Source Code (repository root)

```text
Cargo.toml                       # Cargo workspace

crates/
├── thriftp-core/                # Domain and application logic; no Tauri dependency
│   ├── src/
│   │   ├── favorites/
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── repository.rs
│   │   │   └── service.rs
│   │   ├── credentials/
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   └── service.rs
│   │   └── connections/
│   │       ├── mod.rs
│   │       ├── model.rs
│   │       └── service.rs
│   └── tests/
├── thriftp-credentials-keyring/ # CredentialStore adapter for native OS stores
│   └── src/lib.rs
└── thriftp-storage-json/        # FavoritesRepository adapter for versioned JSON
    └── src/lib.rs

apps/
└── desktop/
    ├── src-tauri/               # Thin Tauri IPC adapter
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── lib.rs
    │   │   └── commands/
    │   │       ├── favorites.rs
    │   │       ├── credentials.rs
    │   │       └── connections.rs
    │   └── Cargo.toml
    └── src/                     # Svelte UI
        ├── lib/
        │   └── favorites/
        │       ├── api.ts
        │       ├── stores.ts
        │       └── types.ts
        ├── components/
        │   └── favorites/
        └── routes/
```

**Structure Decision**: A Cargo workspace separates the Tauri-independent domain (`thriftp-core`) from platform adapters (`thriftp-credentials-keyring`, `thriftp-storage-json`) and the desktop shell (`apps/desktop`). Tauri commands are thin adapters that translate IPC requests into core service calls. This keeps the core testable and reusable by future CLI/MCP interfaces, as required by the constitution and the CLI/MCP specification.

## Phase 0: Research

All technical unknowns are resolved in [research.md](./research.md).

## Phase 1: Design

- [data-model.md](./data-model.md) defines entities, relationships, validation rules, persistence shape, and state transitions.
- [contracts/favorites-ipc.md](./contracts/favorites-ipc.md) defines the Tauri command contract used by the Svelte UI.
- [contracts/credential-store.md](./contracts/credential-store.md) defines the credential-store abstraction and fail-closed behavior.
- [quickstart.md](./quickstart.md) defines end-to-end validation scenarios.

## Post-Design Constitution Check

Re-evaluated after Phase 1 design:

- **Core independence**: `thriftp-core` owns models, validation, sorting, credential-reference rules, and services; Tauri commands remain thin adapters.
- **Security**: Credentials are held only in the native OS store through `CredentialStore`; no secret is serializable, logged, returned to the UI, or written to JSON. Store failures fail closed.
- **Privacy**: No telemetry or cloud transmission is introduced.
- **Quality**: Docker-free contract tests, adapter tests, IPC tests, and cross-platform CI are defined; the shared `001` containerized protocol integration suite is a separate stage and is not required by this feature.
- **Containerized integration testing**: This feature's tests require no container runtime; Docker is never an application runtime dependency.
- **Observability**: `tracing` spans cover favorites and credential operations with contextual fields and no secret values.
- **Supply chain**: Dependency audits, SBOM generation, pinned dependencies, and lockfile enforcement are included in the plan constraints.
- **Automation reuse**: The core service interface and data model remain reusable by future CLI/MCP work; no management surface is added in this release.

No gate violations. No complexity exceptions are required.

## Complexity Tracking

No constitution violations or complexity exceptions were identified. The three-crate split is justified by the constitution's requirement that the core remain independent of Tauri and reusable by CLI/MCP; it is not organizational-only complexity.
