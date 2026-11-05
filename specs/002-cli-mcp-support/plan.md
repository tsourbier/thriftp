# Implementation Plan: CLI and MCP Automation Interfaces

**Branch**: `002-cli-mcp-support` | **Date**: 2026-11-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `/specs/002-cli-mcp-support/spec.md`

## Summary

Add two first-class, headless automation surfaces to Thriftp: a scriptable `thriftp` CLI and a local-only Model Context Protocol (MCP) server, both thin adapters over the existing `thriftp-core` connection, browsing, transfer, queue, and credential services. The CLI provides deterministic JSON output, stable exit codes, secure stdin-only secret input, directory operations, transfer lifecycle control, and MCP administration. The MCP server is disabled by default, requires explicit per-client enrollment and scoped grants, enforces canonical path scopes for local and remote operations (remote scopes are profile-bound; local scopes are profile-independent upload/download endpoints), requires confirmation or a scoped pre-approval policy for destructive actions, and writes a durable audit record for every operation, failing closed when the audit record cannot be written. A single-owner **transfer host** coordinates the desktop application, CLI, and MCP server through the shared persisted queue and a local-only control channel, so all surfaces observe and control the same transfers without an always-running daemon. Protocol integration tests reuse the shared `001` Docker Compose harness on macOS Docker Desktop and Linux CI; Docker is a development/CI tool only and never a runtime dependency of the CLI or MCP server. Planning resolves the specification's deferred decisions: local IPC for process coordination, Ed25519-based client identity with explicit enrollment, and dual-era MCP support for `2026-07-28` (primary) and `2025-11-25` (legacy). The plan assumes the `thriftp-core`, `thriftp-storage-json`, and native credential adapter described by `001` and `003` are implemented first; it extends them without duplicating protocol or credential logic.

## Technical Context

**Language/Version**: Rust stable, edition 2024, MSRV 1.88.0 (required by `rmcp` 3.8); TypeScript 5.x and Svelte 5.x only for the desktop automation control surface. This raises the workspace MSRV above the 1.85.1 recorded in `001`/`003`; those plans remain valid except for that line, and the workspace toolchain file becomes the single source of truth.

**Primary Dependencies**: `clap` 4.5 with derive for the CLI; Tokio 1.x; `rmcp` 3.8.0 with server, stdio, and Streamable HTTP server support; `ed25519-dalek` 2.x, `sha2` 0.10, `base64` 0.22, and `getrandom` for client identity and audit hash chains; `serde`, `serde_json`, and `schemars` for contracts; `uuid`, `chrono`, `thiserror`, `tracing`, `tracing-subscriber`, and `zeroize` from the existing workspace; `fs2` for cross-platform advisory locks; `interprocess` 2.x for Unix domain sockets and Windows named pipes; `trash` for local deletion; the existing `keyring`-backed credential adapter; `assert_cmd`, `predicates`, `insta`, `jsonschema`, and `tempfile` as dev-dependencies

**Storage**: Existing shared `favorites.json` (connection profiles and credential references) and `transfers.json` (queue, active transfers, partial data, history) from `001`/`003`; new versioned `automation.json` for MCP settings, enrolled clients, enrollment requests, grants, directory scopes, and approval policies; append-only `audit/YYYY-MM-DD.jsonl` segments with hash chaining; ephemeral `run/host.lock`, `run/host.json`, `run/host.token`, and the control endpoint under the OS application-data directory. No secrets are persisted outside the native OS credential store.

**Testing**: `cargo test` Docker-free unit, contract, multi-process, and platform tests; `assert_cmd`/`predicates`/`insta` CLI contract and snapshot tests with stdout/stderr separation assertions; `jsonschema` validation of CLI JSON and MCP tool schemas; an in-process `rmcp` client harness for tool, authorization, version-negotiation, destructive-confirmation, disabled-by-default, remote-client-rejection, client-disconnect-policy, audit-failure, and audit-tamper tests; parity tests that run identical authorization, audit, denial, and transfer scenarios across standalone MCP, embedded MCP, CLI, and desktop; security-parity tests for the native credential store, certificate validation, host-key rejection, plain-FTP warning, and OS permissions; multi-process tests that spawn real CLI/MCP binaries to exercise host ownership, delegation, host loss, and stale-lock recovery; protocol integration tests through the shared `001` Docker Compose harness on macOS Docker Desktop and Linux CI, with Windows Docker Desktop optional; separate CI stages for unit/contract, protocol integration, multi-process, platform, and build

**Target Platform**: macOS, Linux, and Windows desktop and headless hosts

**Project Type**: CLI binary, local MCP server binary/library, and a desktop automation control surface, all over a shared Rust core

**Performance Goals**: CLI cold start and `--help` under 250 ms; first directory entries in under 1 second locally and within 2 seconds of the first server response remotely; 10,000-entry listings remain streamed and bounded in memory; MCP tool dispatch overhead under 50 ms p95 excluding network and disk I/O; durable audit append under 10 ms p95 on a local SSD; transfer-host control round trip under 20 ms on the same machine; transfer memory remains `O(1)` regardless of file size

**Constraints**: MCP disabled by default and local-only (stdio for standalone, loopback for embedded); no remote network clients; no undocumented background daemon; exactly one transfer host at a time; no secrets in arguments, environment variables, output, logs, errors, or audit records; no Tauri dependency in the CLI, MCP server, or core; no favorites, arbitrary shell, or server-administration tools; all MCP operations fail closed without a durable audit record; canonical path scope enforcement for local and remote operations; dual-era MCP compatibility with `2026-07-28` and `2025-11-25`; Docker-compatible container runtimes are a development/CI-only dependency for shared protocol integration tests and MUST NOT be a runtime dependency of the CLI or MCP server; missing container runtimes skip only the protocol-integration stage

**Scale/Scope**: At least 500 connection profiles; 10,000 entries per directory; at least 4 concurrent transfers; at least 100 enrolled MCP clients and 1,000 grants/scopes; at least 1,000,000 audit records retained across the default 90-day window; macOS, Linux, and Windows

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Gate | Status |
|-----------|------|--------|
| I. Product Vision | Adds first-class CLI and MCP automation to the file transfer product without expanding into unrelated capabilities | PASS |
| II. Architecture | Rust core; CLI and MCP are thin adapters; no Tauri or protocol logic in adapters; async; streaming `O(1)` transfers; cohesive `automation` and `host` modules; justified dependencies; structured tracing | PASS |
| III. Security | Native credential store only; stdin-only secrets; no secret logging; same certificate/host-key rules; explicit MCP enrollment and least-privilege grants; local-only transport; audit fail-closed; SBOM/pinned dependencies/lockfiles | PASS |
| IV. User Experience | Human-readable CLI output and help; machine-readable output for automation; desktop control surface for grants, sessions, and audit; no proprietary assets | PASS |
| V. Quality | Automated CLI/MCP contract tests; Docker-free unit, contract, multi-process, and platform tests; shared containerized protocol integration tests on macOS Docker Desktop and Linux CI; separate CI stages; Clippy; `rustfmt`; documented public interfaces; explicit errors; benchmarks for scope validation, audit append, and serialization | PASS |
| VI. Open-Source | MIT license; complete CLI/MCP documentation; transparent dependencies; no cloud requirement | PASS |
| VII. AI-Assisted Development | Specification is authoritative; acceptance criteria and tests are defined; YAGNI applied; architecture decisions documented | PASS |
| VIII. Automation & Integration | This feature implements the principle directly: first-class CLI and MCP, shared core, security parity, explicit authorization, auditability, versioning, and contract tests | PASS |

No gate violations. No complexity exceptions are required.

## Project Structure

### Documentation (this feature)

```text
specs/002-cli-mcp-support/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 validation guide
├── contracts/           # Phase 1 interface contracts
│   ├── cli.md
│   ├── mcp-tools.md
│   ├── automation-authorization.md
│   └── transfer-host.md
├── security-review.md   # Required security review record
├── validation.md        # Quickstart validation record with timing evidence
└── tasks.md             # Phase 2 output (/speckit.tasks)
```

### Source Code (repository root)

```text
Cargo.toml                       # Cargo workspace (MSRV 1.88)

crates/
├── thriftp-core/                # Tauri-independent domain and application logic
│   ├── src/
│   │   ├── connections/
│   │   ├── credentials/
│   │   ├── browser/
│   │   ├── transfers/
│   │   ├── queue/
│   │   ├── favorites/
│   │   ├── previews/
│   │   ├── preferences/
│   │   ├── automation/          # New: MCP identity, grants, scopes, approvals, audit
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── enrollment.rs
│   │   │   ├── grants.rs
│   │   │   ├── scope.rs
│   │   │   ├── approval.rs
│   │   │   ├── audit.rs
│   │   │   ├── retention.rs
│   │   │   └── service.rs
│   │   └── host/                # New: single-owner transfer host and control protocol
│   │       ├── mod.rs
│   │       ├── model.rs
│   │       ├── lock.rs
│   │       ├── protocol.rs
│   │       ├── server.rs
│   │       └── client.rs
│   └── tests/
├── thriftp-protocols/           # Existing FTP/FTPS/SFTP adapters
├── thriftp-credentials-keyring/ # Existing native credential-store adapter
├── thriftp-storage-json/        # Existing JSON adapters; extended for automation and audit
│   └── src/
│       ├── lib.rs
│       ├── favorites.rs
│       ├── transfers.rs
│       ├── automation.rs
│       └── audit.rs
├── thriftp-cli/                 # New: `thriftp` binary, thin CLI adapter
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/
│   │   │   ├── profiles.rs
│   │   │   ├── browse.rs
│   │   │   ├── files.rs
│   │   │   ├── transfers.rs
│   │   │   ├── queue.rs
│   │   │   ├── mcp.rs
│   │   │   └── audit.rs
│   │   ├── output.rs
│   │   ├── exit.rs
│   │   └── secret_input.rs
│   └── tests/
├── thriftp-mcp/                 # New: `thriftp-mcp` binary + embedded server library
│   ├── src/
│   │   ├── lib.rs
│   │   ├── server.rs
│   │   ├── auth.rs
│   │   ├── transport.rs
│   │   ├── errors.rs
│   │   └── tools/
│   │       ├── connections.rs
│   │       ├── browse.rs
│   │       ├── files.rs
│   │       ├── transfers.rs
│   │       ├── queue.rs
│   │       └── automation.rs
│   └── tests/
└── thriftp-tests/               # New: shared end-to-end CLI, MCP, host, security tests
    └── tests/

apps/
└── desktop/
    ├── src-tauri/
    │   └── src/
    │       └── commands/
    │           └── automation.rs # Embedded MCP lifecycle, grants, sessions, audit
    └── src/
        ├── lib/
        │   └── automation/
        │       ├── api.ts
        │       ├── stores.ts
        │       └── types.ts
        └── components/
            └── automation/

docs/
├── cli.md                       # Commands, options, exit codes, output streams, examples
├── mcp.md                       # Enrollment, grants, tools, transports, compatibility
├── automation-security.md       # Scope enforcement, approvals, audit, fail-closed rules
├── transfer-host.md             # Process ownership and shared-state model
└── compatibility.md             # CLI/MCP versions and deprecation policy

```

Shared end-to-end scenarios live in `crates/thriftp-tests/tests/`:

```text
crates/thriftp-tests/tests/
├── cli/                         # End-to-end CLI contract scenarios
├── mcp/                         # MCP client contract scenarios
├── host/                        # Multi-process ownership and handoff scenarios
├── integration/                 # Shared 001 Docker Compose protocol scenarios via scripts/test-integration.sh
└── security/                    # Secret redaction, security parity, and panic-safety scenarios
```

**Structure Decision**: The workspace keeps `thriftp-core` independent of Tauri and of any specific surface. New `automation` and `host` modules own MCP identity, authorization, scopes, approvals, audit orchestration, transfer-host ownership, and the local control protocol. `thriftp-cli` and `thriftp-mcp` are thin adapters that translate their protocols into core service calls; neither contains protocol or business logic. `thriftp-storage-json` gains automation and audit adapters behind core traits. The `thriftp-tests` crate hosts shared multi-process, parity, security, and end-to-end scenarios. The desktop application only adds a control surface and the embedded MCP lifecycle; it does not own automation rules. This satisfies the constitution's shared-core, security-parity, and thin-adapter requirements and avoids a second implementation of file transfer behavior.

## Phase 0: Research

All technical unknowns and open architectural decisions are resolved in [research.md](./research.md), including the transfer process model, MCP client identity proofing, supported MCP protocol revisions, local IPC, audit durability, and cross-platform testing.

## Phase 1: Design

- [data-model.md](./data-model.md) defines automation, authorization, audit, transfer-host, CLI result, and persistence entities with validation rules and state transitions.
- [contracts/cli.md](./contracts/cli.md) defines the command tree, global options, JSON envelope, exit codes, stream conventions, secret input, and version output.
- [contracts/mcp-tools.md](./contracts/mcp-tools.md) defines protocol versions, transports, discovery, the pre-authorization bootstrap, tool catalog, schemas, errors, and long-running transfer behavior.
- [contracts/automation-authorization.md](./contracts/automation-authorization.md) defines enrollment, Ed25519 identity proofing, grants, canonical path scopes, destructive approvals, revocation, and audit.
- [contracts/transfer-host.md](./contracts/transfer-host.md) defines host ownership, the local control channel, JSON-RPC methods, authentication, host loss, and shared-state semantics.
- [quickstart.md](./quickstart.md) defines end-to-end validation scenarios for both interfaces, authorization, audit, host coordination, and failure modes.

## Post-Design Constitution Check

Re-evaluated after Phase 1 design:

- **Core independence**: All connection, browsing, transfer, queue, credential, automation, and host logic lives in `thriftp-core`; the CLI, MCP server, and Tauri commands are adapters.
- **No parallel protocol logic**: FTP/FTPS/SFTP continue to implement `RemoteFs`; the new surfaces call the existing transfer engine and protocol adapters.
- **Streaming and memory**: CLI and MCP transfers use the same chunked engine; MCP returns transfer IDs for long operations and never buffers file data; directory listings stream and paginate.
- **Security parity**: Both surfaces use `CredentialStore`, certificate and host-key verification, plain-FTP unencrypted-transport warnings, and least privilege. Secrets are stdin-only, zeroized, and never serialized. MCP is local-only, disabled by default, explicitly enrolled, scoped, and audited.
- **Security review and documentation gates**: An explicit security review covers MCP identity, scopes, approvals, audit, tokens, and secret handling. Public core, CLI, and MCP APIs are documented with rustdoc and a `missing_docs` or `cargo doc` gate. Recoverable-error paths are audited for panics and `unwrap`/`expect` use.
- **Auditability**: Every MCP operation, including denials, produces a hash-chained, fsynced audit record before a result is returned; destructive operations never proceed without durable audit.
- **Process model**: A single transfer host owns execution; desktop, CLI, and MCP share state through a local authenticated control channel; no always-running daemon and no silent completion after process exit.
- **Compatibility**: CLI and MCP contracts are versioned; MCP supports `2026-07-28` and `2025-11-25`; unsupported versions receive a clear compatibility error.
- **Quality**: Contract tests, schema validation, destructive-confirmation tests, disabled-by-default and remote-client tests, standalone/embedded and desktop/CLI/MCP parity tests, security-parity tests, audit-tamper tests, multi-process tests, fault injection, panic/unwrap audits, Docker-free unit/contract tests, shared containerized protocol integration tests on macOS Docker Desktop and Linux CI, separate CI stages, cross-platform CI, and benchmarks are defined.
- **Containerized integration testing**: Docker is used only for shared protocol tests; it is never a CLI/MCP runtime dependency, and a missing runtime skips only the protocol-integration stage without masking unit, contract, multi-process, or platform results.
- **Supply chain**: Dependencies are pinned; the MSRV moves to 1.88 for `rmcp` 3.8; SBOM, audits, and lockfile enforcement remain required.
- **Scope discipline**: Favorites, remote MCP, shell tools, and an always-running daemon remain out of scope.

No gate violations. No complexity exceptions are required.

## Complexity Tracking

No constitution violations or complexity exceptions were identified. The new CLI and MCP crates are justified as thin, independently testable adapters required by the constitution. The `automation` and `host` modules are cohesive domain components, not organizational-only layers: they own security-critical authorization, audit, and cross-process coordination that cannot live in either adapter without duplication.
