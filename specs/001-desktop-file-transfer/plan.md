# Implementation Plan: Thriftp Desktop File Transfer Client

**Branch**: `001-desktop-file-transfer` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/001-desktop-file-transfer/spec.md`

## Summary

Build the initial Thriftp desktop client: a Rust core for connection management, local/remote browsing, streaming transfers, queue control, persistence, and security; protocol adapters for FTP, explicit/implicit FTPS, and SFTP; a Tauri 2 shell; and a Svelte/TypeScript dual-pane UI. The client supports saved connection profiles and directory favorites; server favorites are delivered by feature `003`. It also supports secure OS credential storage, resumable transfers, pause/resume, queue pause/resume, global bandwidth limiting, prioritization, conflict resolution, native previews with fallback, tabs restored on launch, and persisted queue/history with interrupted-transfer recovery. The shared metadata store and credential abstraction introduced by the Server Favorites feature (`003`) are reused. Protocol integration tests use one Docker Compose harness shared by macOS Docker Desktop and Linux CI; Docker is a development/CI tool only and never an application runtime dependency. The specification records the resolved product decisions: accessibility targets WCAG 2.2 AA where applicable, and transfer history retains the 1,000 most recent records with a user-clear action.

## Technical Context

**Language/Version**: Rust stable (edition 2024, Rust 1.85.1 or newer); TypeScript 5.x; Svelte 5.x

**Primary Dependencies**: Tauri 2.x; Tokio 1.x; `suppaftp` 12.x with `native-tls` for FTP/FTPS and the `deprecated` feature for implicit FTPS; the accepted TLS-stack deviation is recorded in `docs/adr/0001-ftp-tls-stack.md`; `russh` and `russh-sftp` for SFTP; `keyring`; `serde`/`serde_json`; `uuid`; `chrono`; `thiserror`; `tracing`; `zeroize`; `trash`; `criterion`; Svelte; Vite; Vitest with Testing Library; optional `tauri-driver` for end-to-end smoke tests

**Storage**: Shared metadata JSON (`favorites.json`, extended from `003`) for connection profiles, server favorites, directory favorites, preferences, and tab state; `transfers.json` for queue and capped transfer history with periodic checkpoints; passwords, private keys, and passphrases only in the native OS credential store through `CredentialStore`

**Testing**: `cargo test` for Docker-free unit tests; protocol integration tests against Dockerized FTP, explicit FTPS, implicit FTPS, and SFTP services through the same Docker Compose harness on macOS via Docker Desktop and in Linux CI; platform-specific tests that do not require a container runtime; Tauri IPC tests; Vitest component tests; optional end-to-end smoke tests; CI reports unit, container-integration, and platform-specific test classes as separate stages

**Target Platform**: macOS, Linux, and Windows desktop

**Project Type**: Desktop application (Tauri 2 + Svelte)

**Performance Goals**: Local directory with 10,000 entries shows first entries in under 1 second and completes in under 5 seconds; remote directory with 10,000 entries over a controlled server at up to 50 ms round-trip latency shows first entries within 2 seconds of the first server response and completes within 30 seconds; at least 4 concurrent transfers by default; constant (`O(1)`) transfer memory regardless of file size; UI input response under 100 ms during browsing and transfers

**Constraints**: No plaintext secrets; no custom vault or master password; no silent certificate or host-key bypass; offline-first; no cloud or cross-device sync; desktop-only initial release; core independent of Tauri; protocol implementations isolated behind stable interfaces; CLI/MCP reuse through the shared core; accessibility target WCAG 2.2 AA where applicable plus native platform accessibility; SBOM generation for releases; pinned dependencies and CI actions; committed and enforced lockfiles; Docker-compatible container runtimes are a development/CI-only dependency for protocol integration tests and MUST NOT be a runtime dependency of the Thriftp application; absence of a container runtime MUST NOT block unit tests or platform-specific tests

**Scale/Scope**: At least 10,000 entries per directory; at least 4 concurrent transfers; 1,000 retained transfer-history records; at least 500 server favorites; FTP, explicit FTPS, implicit FTPS, and SFTP; macOS, Linux, and Windows

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Product Vision | Dual-pane transfer client focused on simplicity, reliability, performance, and polish | PASS |
| II. Architecture | Rust core, Tauri 2 runtime, Svelte/TypeScript UI, core independent of Tauri, protocol adapters behind traits, async work, O(1) transfers, benchmarks, tracing, cohesive modules | PASS |
| III. Security | Native credential store, no plaintext, certificate/host-key verification, no silent bypass, least privilege, SBOM/pinned dependencies/lockfiles | PASS |
| IV. User Experience | Native dual-pane interaction, drag and drop, previews with fallback, themes, accessibility, no proprietary assets | PASS |
| V. Quality | Automated tests, Docker-free unit tests, containerized protocol integration tests shared by macOS and Linux CI, separate platform-specific tests, Clippy, `rustfmt`, documented interfaces, explicit errors, benchmark coverage | PASS |
| VI. Open-Source | MIT license, documentation, transparent dependencies | PASS |
| VII. AI-Assisted Development | Specification and acceptance criteria are authoritative, tests required, YAGNI applied | PASS |
| VIII. Automation & Integration | Domain logic and protocol abstractions live in the shared core; CLI/MCP work is a separate feature (`002`) and must reuse the core, not duplicate protocol logic | PASS |

No gate violations. No complexity exceptions are required.

## Project Structure

### Documentation (this feature)

```text
specs/001-desktop-file-transfer/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 validation guide
├── contracts/           # Phase 1 interface contracts
│   ├── desktop-ipc.md
│   ├── protocol-adapter.md
│   ├── transfer-engine.md
│   └── credential-store.md
└── tasks.md             # Phase 2 output (/speckit.tasks, not created here)
```

### Source Code (repository root)

```text
Cargo.toml                       # Cargo workspace

crates/
├── thriftp-core/                # Domain and application logic; no Tauri dependency
│   ├── src/
│   │   ├── connections/
│   │   ├── credentials/
│   │   ├── browser/
│   │   ├── transfers/
│   │   ├── queue/
│   │   ├── favorites/
│   │   ├── previews/
│   │   └── preferences/
│   └── tests/
├── thriftp-protocols/           # FTP/FTPS/SFTP adapters behind RemoteFs
│   ├── src/
│   │   ├── traits.rs
│   │   ├── ftp.rs
│   │   └── sftp.rs
│   └── tests/
├── thriftp-credentials-keyring/ # Native credential-store adapter
│   └── src/lib.rs
└── thriftp-storage-json/        # Metadata and transfer-state persistence
    └── src/lib.rs

apps/
└── desktop/
    ├── src-tauri/               # Thin Tauri IPC adapter
    │   ├── src/
    │   │   ├── main.rs
    │   │   ├── lib.rs
    │   │   └── commands/
    │   │       ├── connections.rs
    │   │       ├── browser.rs
    │   │       ├── transfers.rs
    │   │       ├── queue.rs
    │   │       ├── favorites.rs
    │   │       ├── previews.rs
    │   │       ├── inspector.rs
    │   │       └── preferences.rs
    │   └── Cargo.toml
    └── src/                     # Svelte UI
        ├── lib/
        │   ├── connections/
        │   ├── browser/
        │   ├── transfers/
        │   ├── queue/
        │   └── favorites/
        ├── components/
        └── routes/

docker/
├── ftp/                         # FTP/FTPS test server definitions
├── sftp/                        # OpenSSH test server definitions
└── protocol/                    # Shared Docker Compose integration harness

scripts/
└── test-integration.sh          # Starts containers, runs protocol tests, tears down

tests/
├── integration/                 # Cross-crate protocol and persistence tests
└── e2e/                         # Optional desktop smoke tests
```

**Structure Decision**: A Cargo workspace separates the Tauri-independent domain (`thriftp-core`) from protocol adapters (`thriftp-protocols`), the native credential-store adapter (`thriftp-credentials-keyring`), and storage (`thriftp-storage-json`). The Tauri shell is a thin IPC adapter; the Svelte UI owns presentation only. This satisfies the constitution's core-independence, protocol-isolation, and CLI/MCP-reuse requirements. Server-favorite management remains owned by `003`; feature `001` exposes the shared connection-profile, credential, and storage interfaces it consumes.

## Phase 0: Research

All technical unknowns and open product decisions are resolved in [research.md](./research.md).

## Phase 1: Design

- [data-model.md](./data-model.md) defines entities, relationships, validation rules, persistence schemas, and state transitions.
- [contracts/desktop-ipc.md](./contracts/desktop-ipc.md) defines Tauri commands and events for connections, browsing, transfers, queue control, previews, preferences, and tabs.
- [contracts/protocol-adapter.md](./contracts/protocol-adapter.md) defines the `RemoteFs` abstraction and protocol capabilities.
- [contracts/transfer-engine.md](./contracts/transfer-engine.md) defines queue, pause/resume, bandwidth, priority, conflict, and persistence behavior.
- [contracts/credential-store.md](./contracts/credential-store.md) records the shared credential-store contract from `003`.
- [quickstart.md](./quickstart.md) defines end-to-end validation scenarios.

## Post-Design Constitution Check

Re-evaluated after Phase 1 design:

- **Core independence**: Domain models, transfer engine, browsing logic, queue rules, and protocol traits live in `thriftp-core`; Tauri commands are thin adapters.
- **Protocol isolation**: FTP/FTPS/SFTP implement the same `RemoteFs` contract and advertise capabilities; the core has no per-protocol branches.
- **Streaming and memory**: Transfers process fixed-size chunks and maintain constant memory; directory enumeration is streaming/paginated.
- **Security**: Credentials stay in the native store, TLS and host keys are verified by default, overrides are explicit and visible, and secrets are redacted from logs and errors.
- **Observability**: `tracing` spans cover connection, browsing, transfer, queue, and credential operations with contextual IDs and redaction.
- **Quality**: Rust unit tests are Docker-free; protocol integration tests use the same Docker Compose harness on macOS Docker Desktop and Linux CI; platform-specific tests run separately; Tauri IPC tests and frontend tests are planned; CI exposes the test classes as distinct stages.
- **Containerized integration testing**: Container runtimes are development/CI tooling only, never an application runtime dependency; a missing runtime blocks only container integration tests and cannot mask unit or platform test results.
- **Supply chain**: SBOM generation, dependency audits, pinned dependencies/actions, and lockfile enforcement are included.
- **Automation reuse**: Core services and protocol traits are independent of Tauri and reusable by the CLI/MCP feature.
- **Accessibility and retention**: WCAG 2.2 AA is the target where applicable; transfer history is capped at 1,000 records with a clear action.

No gate violations. No complexity exceptions are required.

## Complexity Tracking

No constitution violations or complexity exceptions were identified. The protocol, credential, and storage adapters are justified by the constitution's core-independence, protocol-isolation, and security requirements; they are not organizational-only complexity.
