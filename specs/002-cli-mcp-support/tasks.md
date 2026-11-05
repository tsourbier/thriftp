---
description: "Task list for CLI and MCP automation interfaces"
---

# Tasks: CLI and MCP Automation Interfaces

**Input**: Design documents from `/specs/002-cli-mcp-support/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/), [quickstart.md](./quickstart.md)

**Tests**: Required. FR-005 and the project constitution require automated contract tests for both interfaces, so every user story includes test tasks before implementation.

**Organization**: Tasks are grouped by user story so each story can be implemented, tested, and demonstrated independently. The two P1 stories are delivered first, followed by the P2 stories and the P3 output-hardening story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: Which user story the task belongs to (`US1`–`US6`)
- Every task includes an exact file path under the Cargo workspace, `apps/desktop`, `docs/`, or `crates/thriftp-tests/tests/`

## Path Conventions

- Rust workspace: `Cargo.toml`, `crates/<crate>/src/`, `crates/<crate>/tests/`, `crates/<crate>/benches/`
- Desktop shell: `apps/desktop/src-tauri/src/`, `apps/desktop/src/`
- Shared end-to-end tests: `crates/thriftp-tests/tests/cli/`, `crates/thriftp-tests/tests/mcp/`, `crates/thriftp-tests/tests/host/`, `crates/thriftp-tests/tests/integration/`, `crates/thriftp-tests/tests/security/`
- Documentation: `docs/`
- Feature artifacts: `specs/002-cli-mcp-support/`

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Create the workspace crates, dependencies, CI stages, test harness layout, and documentation skeleton.

- [X] T001 Update the workspace to Rust edition 2024, MSRV 1.88, and add the `thriftp-cli`, `thriftp-mcp`, and `thriftp-tests` members in `Cargo.toml` and `rust-toolchain.toml`.
- [X] T002 Create the CLI crate skeleton with a `thriftp` binary entry point in `crates/thriftp-cli/Cargo.toml` and `crates/thriftp-cli/src/main.rs`.
- [X] T003 Create the MCP crate skeleton with the `thriftp-mcp` binary and embedded library entry points in `crates/thriftp-mcp/Cargo.toml`, `crates/thriftp-mcp/src/lib.rs`, and `crates/thriftp-mcp/src/main.rs`.
- [X] T004 [P] Pin the new dependencies in `Cargo.toml`, `crates/thriftp-cli/Cargo.toml`, `crates/thriftp-mcp/Cargo.toml`, `crates/thriftp-core/Cargo.toml`, and `crates/thriftp-storage-json/Cargo.toml`: `clap` 4.5, Tokio 1.x, `rmcp` 3.8.0, `ed25519-dalek` 2.x, `sha2` 0.10, `base64` 0.22, `getrandom`, `schemars`, `fs2`, `interprocess` 2.x, `trash`, `criterion`, `assert_cmd`, `predicates`, `insta`, `jsonschema`, and `tempfile`.
- [X] T005 [P] Add separate CI stages for unit/contract, shared protocol integration, multi-process host, platform, and build in `.github/workflows/ci.yml` and `scripts/test-integration.sh`, preserving the existing `frontend` stage; make a missing container runtime skip only protocol integration.
- [X] T006 [P] Configure formatting, linting, and workspace warnings in `rustfmt.toml`, `clippy.toml`, and `Cargo.toml`; CI must run `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`.
- [X] T007 [P] Create the `thriftp-tests` workspace crate in `crates/thriftp-tests/Cargo.toml` and its shared test layout and fixtures in `crates/thriftp-tests/tests/cli/`, `crates/thriftp-tests/tests/mcp/`, `crates/thriftp-tests/tests/host/`, `crates/thriftp-tests/tests/integration/`, `crates/thriftp-tests/tests/security/`, and `crates/thriftp-tests/tests/fixtures/`.
- [X] T008 [P] Create the documentation skeleton in `docs/cli.md`, `docs/mcp.md`, `docs/automation-security.md`, `docs/transfer-host.md`, and `docs/compatibility.md`.
- [X] T009 [P] Create the CLI and MCP schema fixture directories in `crates/thriftp-cli/tests/schemas/` and `crates/thriftp-mcp/tests/schemas/`.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Shared core models, persistence, audit, host, error mapping, adapters, and test harnesses required before any user story.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T010 Implement the automation domain entities in `crates/thriftp-core/src/automation/model.rs` exactly as specified in `data-model.md`: `AutomationSettings` (`auditRetentionDays` default 90 and range 1–3,650; `auditNoticeDays` default 7, range 1–30 and less than retention; `auditMaxBytes` default 1 GiB; `enrollmentRequestTtlSeconds` 600; `confirmationTtlSeconds` 120; `maxPendingEnrollmentRequests` 10; `maxPendingEnrollmentRequestsPerFingerprint` 3; `maxEnrolledClients` 100), `MCPClient` (`fingerprint` is SHA-256 of the Ed25519 public key, lowercase hex, unique; `displayName` 1–100 characters after trimming), `EnrollmentRequest`, `AuthorizationGrant` (`revision` starts at 1 and increments on any scope or policy change; `revocationEpoch` starts at 0; `disconnectPolicy` is `continue` by default or `cancel`), `DirectoryScope` (`kind` is `remote` or `local`; `profileId` is required for remote and null for local; `canonicalPath` is canonical; `recursive` is always true), `ApprovalPolicy`, runtime `MCPSession`, `ConfirmationRequest`, `AuditEvent`, `AuditSegment`, `TransferHost`, `HostControlClient`, and `CLIInvocationResult` with `schemaVersion` 1 and exit codes 0 and 2–13; include serde, validation constructors, and the state transitions from `data-model.md`.
- [X] T011 [P] Implement versioned `automation.json` persistence in `crates/thriftp-storage-json/src/automation.rs` with `schemaVersion`, monotonic `stateRevision`, atomic replace plus backup, restrictive permissions (`0600` on Unix), ordered migrations, explicit quarantine on corruption, and an exclusive cross-process write lock; never persist sessions, confirmations, nonces, tokens, or private keys.
- [X] T012 [P] Implement the audit hash-chain model in `crates/thriftp-core/src/automation/audit.rs`: monotonic `seq`, `prevHash` genesis of 64 zeros, SHA-256 of the canonical record excluding `hash`, startup/verification support, and a rule that a broken chain is reported and never silently repaired.
- [X] T013 [P] Implement the append-only audit adapter in `crates/thriftp-storage-json/src/audit.rs` with `audit/YYYY-MM-DD.jsonl` segments, `fsync` before success, an exclusive cross-process `audit.lock`, daily rotation, hash-chain continuity across segments, and helpers for 90-day retention, a 1 GiB default storage limit, and the 7-day notice rule.
- [X] T014 [P] Extend the shared error taxonomy in `crates/thriftp-core/src/error.rs` and add redaction helpers in `crates/thriftp-core/src/observability.rs` for `mcp_disabled`, `client_not_enrolled`, `grant_missing`, `grant_expired`, `grant_revoked`, `scope_denied`, `path_ambiguous`, `confirmation_required`, `confirmation_expired`, `audit_unavailable`, `host_unavailable`, `host_lost`, `unsupported_protocol_version`, `invalid_argument`, `not_found`, `conflict`, `permission_denied`, `network_error`, `unsupported_capability`, and `internal_error`; map core failures to CLI exit codes 0 and 2–13 without exposing secrets or raw server diagnostics.
- [X] T015 [P] Implement the transfer-host model and JSON-RPC wire types in `crates/thriftp-core/src/host/model.rs` and `crates/thriftp-core/src/host/protocol.rs`, including `host.status`, `transfers.list`, `transfers.status`, `transfers.submit`, `transfers.pause`, `transfers.resume`, `transfers.cancel`, `transfers.priority`, `transfers.reorder`, `queue.get`, `queue.setConcurrency`, `queue.setBandwidth`, `queue.pause`, `queue.resume`, `queue.cancelAll`, and `events.subscribe`.
- [X] T016 [P] Implement host ownership and stale detection in `crates/thriftp-core/src/host/lock.rs` using an exclusive `fs2` lock on `run/host.lock`, PID liveness, endpoint connectivity checks, and safe cleanup of stale runtime files.
- [X] T017 [P] Implement the host control server and client in `crates/thriftp-core/src/host/server.rs` and `crates/thriftp-core/src/host/client.rs` with Unix domain sockets and Windows named pipes through `interprocess`, newline-delimited JSON-RPC 2.0, a 1 MiB message cap, per-instance token authentication from `run/host.token`, constant-time comparison, timeouts, and malformed-message rejection.
- [X] T018 Implement host lifecycle and recovery in `crates/thriftp-core/src/host/mod.rs`: acquisition, endpoint publication, event notifications, clean release, active-to-interrupted recovery, and single-owner enforcement.
- [X] T019 [P] Implement the CLI output, exit-code, and secret-input scaffolding in `crates/thriftp-cli/src/output.rs`, `crates/thriftp-cli/src/exit.rs`, and `crates/thriftp-cli/src/secret_input.rs`: one JSON document on stdout, diagnostics on stderr, stable codes 0 and 2–13, one-line stdin secrets with zeroization, no TTY prompt, and rejection of `THRIFTP_PASSWORD` and `THRIFTP_KEY_PASSPHRASE`. This is scaffolding only; T093–T096 complete the envelope, exit mapping, stream, and redaction behavior.
- [X] T020 [P] Implement the MCP server, transport, auth, and error skeletons in `crates/thriftp-mcp/src/server.rs`, `crates/thriftp-mcp/src/transport.rs`, `crates/thriftp-mcp/src/auth.rs`, and `crates/thriftp-mcp/src/errors.rs` using `rmcp` 3.8.0, stdout protocol-only discipline, stdio and loopback Streamable HTTP modes, `serverInfo`, and the tool-result envelope. This is scaffolding only; T049–T052 complete discovery, tools, schemas, and error mapping.
- [X] T021 [P] Implement core test fakes in `crates/thriftp-core/tests/support/mod.rs` and `crates/thriftp-core/tests/support/fakes.rs`: fake clock, in-memory audit sink, fake host, fake credential store, fake protocol adapter, and fault injection hooks for audit, network, permissions, and revocation.
- [X] T022 [P] Implement shared CLI and MCP test harnesses in `crates/thriftp-tests/tests/support/cli_harness.rs` and `crates/thriftp-tests/tests/support/mcp_harness.rs`: temp application-data directories, spawned binaries, command runners, JSON parsers, MCP client setup, and stdout/stderr assertions.
- [X] T023 Implement automation service scaffolding in `crates/thriftp-core/src/automation/service.rs`: settings loading, `stateRevision` reload hooks, audit orchestration entry points, and the authorization pipeline skeleton used by both MCP and CLI administration.

**Checkpoint**: Foundation ready. US1 and US2 can begin in parallel.

---

## Phase 3: User Story 1 — Automate file transfers from the command line (Priority: P1) 🎯 MVP

**Goal**: A non-interactive `thriftp` CLI that manages profiles, browses, performs local and remote directory operations, uploads and downloads, controls the queue, and returns stable results without the desktop window.

**Independent Test**: Using only documented commands, create a profile with a stdin secret, connect to a test server, browse, create a directory, upload a directory, download it recursively, rename and delete entries, query and control the queue, and verify exit codes, JSON output, conflict behavior, and secret redaction.

### Tests for User Story 1 (REQUIRED)

> Write these tests first and confirm they fail before implementing the commands.

- [X] T024 [P] [US1] Add CLI contract tests for `profiles`, `browse`, `stat`, `upload`, `download`, `mkdir`, `rename`, `delete`, `transfer`, and `queue` in `crates/thriftp-cli/tests/contract_commands.rs`, asserting exit codes, JSON envelopes, stream separation, and no secret material.
- [X] T025 [P] [US1] Add CLI integration tests for recursive directory structure, default conflict failure, explicit conflict policies, and symbolic-link handling in `crates/thriftp-cli/tests/transfer_scenarios.rs`.
- [X] T026 [P] [US1] Add CLI secret-input tests for `--password-stdin`, `--key-passphrase-stdin`, missing credentials, non-interactive failure, and forbidden secret environment variables in `crates/thriftp-cli/tests/secret_input.rs`.

### Implementation for User Story 1

- [X] T027 [US1] Implement the `clap` 4.5 command tree, global options, dispatch, and help wiring in `crates/thriftp-cli/src/main.rs` and `crates/thriftp-cli/src/commands/mod.rs`.
- [X] T028 [P] [US1] Implement `profiles list`, `show`, `create`, `update`, `set-credential`, `delete`, `test`, `trust-host-key`, and `forget-host-key` in `crates/thriftp-cli/src/commands/profiles.rs`.
- [X] T029 [P] [US1] Implement `browse` and `stat` with remote/local targeting, hidden-entry handling, long metadata, pagination, and canonical paths in `crates/thriftp-cli/src/commands/browse.rs`.
- [X] T030 [P] [US1] Implement `mkdir`, `rename`, and `delete` for local and remote targets in `crates/thriftp-cli/src/commands/files.rs`, including `--yes`, recursive scope warnings, OS trash for local deletes, and permanent-delete warnings for remote deletes.
- [X] T031 [P] [US1] Implement `upload`, `download`, `transfer list`, `status`, `wait`, `pause`, `resume`, `cancel`, `retry`, `priority`, and `reorder` in `crates/thriftp-cli/src/commands/transfers.rs`.
- [X] T032 [P] [US1] Implement `queue list`, `pause`, `resume`, `cancel-all`, `config concurrency`, and `config bandwidth` in `crates/thriftp-cli/src/commands/queue.rs`.
- [X] T033 [US1] Wire the CLI command handlers to the core services, transfer host, and output layer in `crates/thriftp-cli/src/commands/mod.rs` and `crates/thriftp-cli/src/main.rs`.
- [X] T034 [US1] Implement the default `fail` conflict policy, explicit apply-to-all `overwrite`/`skip`/`rename` policies, and preserve/skip-never-follow symbolic-link behavior in `crates/thriftp-cli/src/commands/transfers.rs` and `crates/thriftp-cli/src/commands/files.rs`.
- [X] T035 [US1] Implement profile deletion safeguards for desktop favorites and explicit SSH host-key trust in `crates/thriftp-cli/src/commands/profiles.rs`: a referenced profile warns before deletion, and a new or changed host key fails unless the exact fingerprint is confirmed with `--yes`.
- [X] T036 [US1] Add `tracing` spans and redaction to every CLI command handler in `crates/thriftp-cli/src/commands/` using the existing observability helpers in `crates/thriftp-core/src/observability.rs`, so profile IDs, operations, and outcomes are logged without secrets.

**Checkpoint**: US1 works independently and is the MVP increment. Do not mark US1 complete until the CLI portion of T106 (plain-FTP warnings) passes.

---

## Phase 4: User Story 2 — Let an AI agent operate on explicitly authorized resources (Priority: P1)

**Goal**: A local-only MCP server with Ed25519 client identity, explicit enrollment, least-privilege grants, canonical remote and local scopes, destructive approvals, durable audit, and tools for browsing, transfers, directory operations, and queue control.

**Independent Test**: Enable MCP, enroll a test client, approve a grant for one profile, one remote directory, and one local directory, authenticate, perform browse and transfer operations, and verify that out-of-scope remote paths, out-of-scope local paths, traversal, encoded traversal, and symlink escapes are denied and audited.

### Tests for User Story 2 (REQUIRED)

> Write these tests first and confirm they fail before implementing authorization or tools.

- [X] T037 [P] [US2] Add MCP discovery and protocol-version contract tests in `crates/thriftp-mcp/tests/contract_discovery.rs` for `server/discover`, legacy `initialize`, supported revisions `2026-07-28` and `2025-11-25`, and an unsupported revision.
- [X] T038 [P] [US2] Add MCP tool catalog and schema contract tests in `crates/thriftp-mcp/tests/contract_tools.rs` for every tool in `contracts/mcp-tools.md`, including destructive flags, pagination, structured results, and error envelopes.
- [X] T039 [P] [US2] Add automation authorization tests in `crates/thriftp-core/tests/automation_authorization.rs`: enrollment cannot grant access, fingerprint binding, grant revision and expiry, remote and local scope containment, sibling prefixes, `.`/`..`, encoded traversal, backslash separators, and symlink escapes.
- [X] T040 [P] [US2] Add audit fail-closed tests in `crates/thriftp-core/tests/audit_fail_closed.rs`: every allowed, denied, blocked, and failed MCP operation writes a record, and an unwritable audit store blocks both destructive and non-destructive operations.

### Implementation for User Story 2

- [X] T041 [US2] Implement Ed25519 identity, enrollment requests, nonce challenges, signature verification, session token issuance, and inactivity/absolute expiry in `crates/thriftp-core/src/automation/enrollment.rs`.
- [X] T042 [P] [US2] Implement grant creation, revision increments, expiry, revocation epochs, `disconnectPolicy`, and operation/profile/scope validation in `crates/thriftp-core/src/automation/grants.rs`.
- [X] T043 [P] [US2] Implement canonical path validation for local and remote scopes in `crates/thriftp-core/src/automation/scope.rs`: absolute paths, NUL rejection, encoded-traversal rejection, backslash rejection for remote paths, component-boundary containment, real-path resolution where available, symlink-escape denial, and post-rename revalidation.
- [X] T044 [P] [US2] Implement destructive approvals, single-use confirmations, confirmation expiry, scoped pre-approval policies, `policy_used` records, and headless denial in `crates/thriftp-core/src/automation/approval.rs`.
- [X] T045 [US2] Implement the full authorization pipeline in `crates/thriftp-core/src/automation/service.rs`: enabled state, `stateRevision`, session validity, grant revision/epoch/expiry, operation, profile, remote path, local path, confirmation or policy, audit-before-result, and audit-before-destructive-execution.
- [X] T046 [US2] Implement MCP bootstrap tools `thriftp.automation.status`, `enroll`, `enrollment_status`, `challenge`, `authenticate`, and `session` in `crates/thriftp-mcp/src/tools/automation.rs`.
- [X] T047 [P] [US2] Implement MCP tools for `thriftp.connections.list`, `connections.get`, `browse.list`, `files.stat`, `directories.create`, `directories.rename`, `directories.delete`, and `files.delete` in `crates/thriftp-mcp/src/tools/connections.rs`, `crates/thriftp-mcp/src/tools/browse.rs`, and `crates/thriftp-mcp/src/tools/files.rs`.
- [X] T048 [P] [US2] Implement MCP tools for `thriftp.transfers.upload`, `download`, `list`, `status`, `pause`, `resume`, `cancel`, `priority`, `reorder`, `thriftp.queue.get`, `set_concurrency`, and `set_bandwidth` in `crates/thriftp-mcp/src/tools/transfers.rs` and `crates/thriftp-mcp/src/tools/queue.rs`, including `wait`/`timeoutMs`, `transferId` returns, conflict policies, local scope checks, and `disconnectPolicy`.
- [X] T049 [US2] Implement `server/discover`, legacy `initialize`, `tools/list`, `tools/call`, the structured result envelope, and domain-error mapping in `crates/thriftp-mcp/src/server.rs` and `crates/thriftp-mcp/src/lib.rs`.
- [X] T050 [US2] Implement standalone stdio and embedded loopback transports in `crates/thriftp-mcp/src/transport.rs`: stdout protocol-only, stderr diagnostics, `127.0.0.1`-only binding, ephemeral ports, per-launch bearer tokens, and rejection of wildcard or non-loopback binds and non-loopback `Host`/`Origin`.
- [X] T051 [US2] Implement MCP session-token extraction from `_meta`, authorization middleware, audit ordering, and revocation/expiry checks at each file boundary in `crates/thriftp-mcp/src/auth.rs` and `crates/thriftp-mcp/src/server.rs`.
- [X] T052 [US2] Implement JSON Schema 2020-12 descriptions for every MCP tool and expose them through `tools/list` in `crates/thriftp-mcp/src/tools/mod.rs`.
- [X] T053 [US2] Implement the standalone `thriftp-mcp` binary and the `thriftp mcp serve` alias in `crates/thriftp-mcp/src/main.rs` and `crates/thriftp-cli/src/commands/mcp.rs`.
- [X] T054 [US2] Implement the minimal user approval surface in `crates/thriftp-cli/src/commands/mcp.rs`: `mcp enable`, `disable`, `status`, `clients pending`, `clients approve`, and `clients deny`, including required `--profile`, `--dir`, `--local-dir`, `--operations`, optional `--expires`, optional `--disconnect-policy <continue|cancel>` with default `continue`, optional `--pre-approve`, and `--yes`.
- [X] T055 [US2] Implement MCP transfer-host delegation in `crates/thriftp-mcp/src/server.rs` and `crates/thriftp-core/src/host/client.rs` so standalone MCP becomes host when none exists, embedded MCP always uses the desktop host, and all tool calls go through the host rather than writing transfer state directly.

**Checkpoint**: US2 works independently after US1 or in parallel, and both P1 stories are complete. Do not mark US2 complete until T106 (MCP plain-FTP warnings), T107 (destructive confirmations), T108 (disabled-by-default and remote-client rejection), and T113 (disconnect-policy behavior) pass, even though they are numbered in the Polish phase for sequencing.

---

## Phase 5: User Story 3 — Control, audit, and revoke MCP access (Priority: P2)

**Goal**: Users can inspect enrolled clients, grants, and active sessions; revoke access and disconnect clients immediately; disable MCP; review, export, and verify the audit trail; and change retention and storage-limit policy.

**Independent Test**: Enroll and grant access, inspect clients and sessions, revoke a grant during an active operation, verify sessions terminate and subsequent calls are denied, disable MCP, and confirm the audit trail records allowed, denied, revoked, expired, and retention events without secrets.

### Tests for User Story 3 (REQUIRED)

- [X] T056 [P] [US3] Add CLI administration contract tests for `mcp status`, `clients list`, `clients revoke`, `clients disconnect`, `mcp tools`, and `mcp audit` commands in `crates/thriftp-cli/tests/contract_mcp_admin.rs`.
- [X] T057 [P] [US3] Add revocation, expiry, disablement, and concurrent-server reconciliation tests in `crates/thriftp-core/tests/automation_revocation.rs`, including in-flight stop at the next file boundary and queued-work denial.
- [X] T058 [P] [US3] Add retention, storage-limit, notice, and deletion-audit tests in `crates/thriftp-storage-json/tests/audit_retention.rs`.
- [X] T059 [P] [US3] Add desktop automation command tests in `apps/desktop/src-tauri/tests/automation_commands.rs` for enabling, disabling, approving, revoking, disconnecting, and reading audit state.

### Implementation for User Story 3

- [X] T060 [US3] Implement `mcp clients list`, `clients revoke`, `clients disconnect`, and `mcp tools` in `crates/thriftp-cli/src/commands/mcp.rs`, including active session counts and session IDs in the listing.
- [X] T061 [P] [US3] Implement `mcp audit list`, `export`, `verify`, `retention`, and `limit` in `crates/thriftp-cli/src/commands/audit.rs`, with filtering, JSONL/JSON export, chain verification, and non-immediate policy changes.
- [X] T062 [P] [US3] Implement audit retention and storage-limit execution in `crates/thriftp-core/src/automation/audit.rs` and `crates/thriftp-core/src/automation/retention.rs`: daily checks, `retention_notice`, 7-day notice window, oldest-sealed-segment selection, audited deletion, and `audit_unavailable` when no safe deletion is possible.
- [X] T063 [US3] Implement revocation, expiry, and disablement propagation in `crates/thriftp-core/src/automation/service.rs` and `crates/thriftp-core/src/automation/grants.rs`, including `revocationEpoch`, session termination, file-boundary checks, and audit records for each event.
- [X] T064 [US3] Implement desktop Tauri automation commands in `apps/desktop/src-tauri/src/commands/automation.rs`: status, enable/disable, pending requests, approve/deny with a disconnect-policy choice (`continue` default, `cancel` optional), revoke/disconnect, audit list/verify, retention, and storage limit.
- [X] T065 [P] [US3] Implement the desktop automation API, stores, and types in `apps/desktop/src/lib/automation/api.ts`, `apps/desktop/src/lib/automation/stores.ts`, and `apps/desktop/src/lib/automation/types.ts`.
- [X] T066 [US3] Implement the desktop automation components in `apps/desktop/src/components/automation/`: client list, pending approval dialog with a disconnect-policy selector (`continue` default, `cancel` optional), grant and scope details, active sessions, audit view, and integrity status.
- [X] T067 [US3] Implement embedded MCP lifecycle management in `apps/desktop/src-tauri/src/commands/automation.rs` and `apps/desktop/src-tauri/src/lib.rs`: start/stop the loopback server on enable/disable, publish its endpoint and token, and terminate sessions on shutdown.
- [X] T068 [US3] Implement concurrent standalone/embedded server reconciliation in `crates/thriftp-core/src/automation/service.rs`: shared `stateRevision` reloads, cross-process automation writes, linear audit appends, and session invalidation without granting stale access.

**Checkpoint**: US3 works for both CLI and desktop control surfaces.

---

## Phase 6: User Story 4 — Discover and integrate with documented interfaces (Priority: P2)

**Goal**: Integrators can discover commands, tools, schemas, versions, and compatibility rules from help output and published documentation without reading source code.

**Independent Test**: From a clean environment, run CLI help and version, list MCP tools and schemas, follow the documented minimal examples for both interfaces, and confirm an unsupported protocol version receives a clear compatibility error.

### Tests for User Story 4 (REQUIRED)

- [X] T069 [P] [US4] Add CLI help and documentation-consistency tests in `crates/thriftp-cli/tests/help.rs` that assert every command, global option, output mode, and exit code appears in `docs/cli.md`.
- [X] T070 [P] [US4] Add MCP tool-schema and documentation-consistency tests in `crates/thriftp-mcp/tests/docs_consistency.rs` that assert every catalog tool, input/output shape, and contract version appears in `docs/mcp.md`.

### Implementation for User Story 4

- [X] T071 [P] [US4] Write `docs/cli.md` with installation, configuration, authentication, every command and option, output modes, exit codes, stream rules, secret input, and runnable examples.
- [X] T072 [P] [US4] Write `docs/mcp.md` with supported revisions, transports, enrollment, grants, scopes, approvals, disconnect policy, the full tool catalog, input/output shapes, examples, and compatibility policy.
- [X] T073 [P] [US4] Write `docs/automation-security.md` with canonical path rules, local and remote scopes, confirmations, pre-approval policies, audit fail-closed behavior, the same-user security boundary, and revocation semantics.
- [X] T074 [P] [US4] Write `docs/transfer-host.md` with ownership, shared state, desktop/CLI/MCP roles, host loss, interruption, resumability, and the no-daemon rule.
- [X] T075 [P] [US4] Write `docs/compatibility.md` with CLI and MCP contract versions, supported MCP revisions, deprecation windows, and breaking-change policy.
- [X] T076 [US4] Implement CLI help and version output in `crates/thriftp-cli/src/output.rs` and the command definitions, including the CLI version, automation contract version, supported MCP revisions, and per-command exit-code notes.
- [X] T077 [US4] Implement MCP discovery metadata in `crates/thriftp-mcp/src/server.rs`: `serverInfo`, automation contract version, supported protocol revisions, capabilities, and tool schemas before the first operation.
- [X] T078 [US4] Implement structured unsupported-version errors in `crates/thriftp-mcp/src/errors.rs` and `crates/thriftp-mcp/src/server.rs`, listing supported revisions and avoiding undefined behavior.

**Checkpoint**: US4 documentation and discovery work for both interfaces.

---

## Phase 7: User Story 6 — Understand what happens when a CLI process exits (Priority: P2)

**Goal**: CLI transfers remain active to a terminal state by default, become interrupted and resumable on early exit, and share observable state with the desktop and MCP surfaces through the transfer host.

**Independent Test**: Start a CLI transfer with no host, open and close the desktop application, terminate the CLI early, verify the transfer becomes interrupted and resumable, resume it, and confirm the desktop, CLI, and MCP report the same state.

### Tests for User Story 6 (REQUIRED)

- [X] T079 [P] [US6] Add multi-process ownership tests in `crates/thriftp-tests/tests/host/test_ownership.rs`: concurrent starts, exactly one host, delegation, and lock release.
- [X] T080 [P] [US6] Add host-loss and interruption tests in `crates/thriftp-tests/tests/host/test_host_loss.rs`: killed host, killed CLI host, fresh CLI exit, `SIGINT`, interrupted state, and explicit resume.
- [X] T081 [P] [US6] Add shared-state tests in `crates/thriftp-tests/tests/host/test_shared_state.rs`: desktop observer mode, desktop host with CLI delegation, MCP host/client modes, and consistent queue and progress views.

### Implementation for User Story 6

- [X] T082 [US6] Implement CLI host acquisition, delegation, and default wait-until-terminal behavior in `crates/thriftp-cli/src/commands/transfers.rs` and `crates/thriftp-core/src/host/mod.rs`.
- [X] T083 [US6] Implement `--no-wait` semantics in `crates/thriftp-cli/src/commands/transfers.rs`: require an existing host, return a transfer ID, and fail with `host_unavailable` without starting a background host.
- [X] T084 [US6] Implement signal handling in `crates/thriftp-cli/src/main.rs` and `crates/thriftp-cli/src/commands/transfers.rs`: first `SIGINT` stops at the next chunk boundary, marks the transfer interrupted, preserves the offset, and exits 8; a second signal exits without claiming completion.
- [X] T085 [US6] Implement host shutdown notification, clean release, and desktop quit grace behavior in `crates/thriftp-core/src/host/mod.rs` and `apps/desktop/src-tauri/src/commands/automation.rs`.
- [X] T086 [US6] Implement startup recovery from active to interrupted in `crates/thriftp-core/src/host/mod.rs` and `crates/thriftp-storage-json/src/transfers.rs`, preserving pending and paused transfers and partial-file metadata.
- [X] T087 [US6] Implement `transfer wait` and `transfer resume` host-loss behavior in `crates/thriftp-cli/src/commands/transfers.rs`: report `host_lost`, exit 8, never auto-resume, and verify source size, modification time, and destination offset before resume.
- [X] T088 [US6] Implement stale lock, missing endpoint, and live-but-unresponsive host handling in `crates/thriftp-core/src/host/lock.rs` and `crates/thriftp-core/src/host/mod.rs` without stealing ownership from a live host.
- [X] T089 [US6] Implement desktop observer mode in `apps/desktop/src-tauri/src/commands/automation.rs` and `apps/desktop/src/lib/automation/` so closing the desktop while the CLI is host does not affect the transfer.

**Checkpoint**: Process lifetime and shared-state behavior are independently verified.

---

## Phase 8: User Story 5 — Use stable machine-readable results in automation (Priority: P3)

**Goal**: Scripts can rely on one JSON document on stdout, stable documented fields, stable exit codes, and diagnostics isolated on stderr, including broken-pipe behavior.

**Independent Test**: Run successful and failing commands, parse stdout as one JSON document, verify every documented field and exit code, confirm progress goes only to stderr, and close the output pipe early without a crash or secret leak.

### Tests for User Story 5 (REQUIRED)

- [X] T090 [P] [US5] Add JSON envelope schema tests in `crates/thriftp-cli/tests/json_envelope.rs`, validating `schemaVersion`, `command`, `ok`, `exitCode`, `data`, `warnings`, `error`, and `meta`.
- [X] T091 [P] [US5] Add the full exit-code matrix tests in `crates/thriftp-cli/tests/exit_codes.rs` for success, usage, authentication, network, permission, conflict, cancellation, interruption, authorization, validation, credential store, unsupported, and internal failures.
- [X] T092 [P] [US5] Add stdout/stderr separation and broken-pipe tests in `crates/thriftp-cli/tests/streams.rs`, including `--json`, `--quiet`, `--verbose`, and early consumer closure.

### Implementation for User Story 5

- [X] T093 [US5] Implement the versioned JSON result envelope and serialization in `crates/thriftp-cli/src/output.rs` and `crates/thriftp-cli/tests/schemas/cli-result.schema.json`.
- [X] T094 [US5] Implement the stable exit-code mapping for every command in `crates/thriftp-cli/src/exit.rs` and the command handlers, keeping human and JSON modes identical.
- [X] T095 [US5] Implement stdout/stderr discipline and EPIPE handling in `crates/thriftp-cli/src/output.rs` and `crates/thriftp-cli/src/main.rs`: one result document on stdout, diagnostics on stderr, no panic, and exit 7 when the result cannot be delivered.
- [X] T096 [US5] Implement output redaction and no-secret assertions in `crates/thriftp-cli/src/output.rs` and the output tests, covering errors, warnings, metadata, and environment-variable rejection.
- [X] T097 [US5] Add human and JSON snapshots for every command and failure category in `crates/thriftp-cli/tests/snapshots/`.

**Checkpoint**: Machine-readable output is stable and independently validated.

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Performance, security, supply-chain, documentation, and final validation across all stories.

- [X] T098 [P] Add `criterion` benchmarks for scope canonicalization, authorization, audit serialization/hashing, MCP dispatch, CLI JSON output, and host control round trips in `crates/thriftp-core/benches/automation.rs`, `crates/thriftp-cli/benches/output.rs`, and `crates/thriftp-mcp/benches/dispatch.rs`.
- [X] T099 [P] Add security tests for secret redaction in output, logs, errors, and audit in `crates/thriftp-tests/tests/security/test_secret_redaction.rs`, including credentials, tokens, private keys, and raw server diagnostics.
- [X] T100 [P] Add large-listing and pagination performance tests in `crates/thriftp-tests/tests/integration/test_large_listing.rs` for 10,000-entry local and remote listings without unbounded memory.
- [X] T101 [P] Add SBOM generation, dependency audits, pinned dependency/action verification, and committed lockfile enforcement in `.github/workflows/ci.yml`, `deny.toml`, and `Cargo.lock`.
- [X] T102 [P] Add links to `docs/cli.md`, `docs/mcp.md`, `docs/automation-security.md`, `docs/transfer-host.md`, and `docs/compatibility.md` in `README.md`.
- [X] T103 [P] Update `docs/cli.md`, `docs/mcp.md`, `docs/automation-security.md`, `docs/transfer-host.md`, and `docs/compatibility.md` with final command help, tool schemas, exit codes, plain-FTP warnings, local versus remote scopes, remote-only MCP mutation tools, disconnect policy, audit tamper-evidence, and compatibility examples.
- [X] T104 [P] Conduct and document the explicit security review required by the constitution for MCP identity, scopes, approvals, audit, tokens, secret handling, and the local host channel; record findings and sign-off in `specs/002-cli-mcp-support/security-review.md`.
- [X] T105 [P] Add rustdoc for every new public core, CLI, and MCP API and enforce a `missing_docs` or `cargo doc` gate in `crates/thriftp-core/src/lib.rs`, `crates/thriftp-mcp/src/lib.rs`, `crates/thriftp-cli/src/main.rs`, and `Cargo.toml`.
- [X] T106 [P] Implement plain-FTP unencrypted-transport warnings for profile, browse, stat, upload, download, transfer-status, and transfer-list results in `crates/thriftp-cli/src/commands/profiles.rs`, `browse.rs`, and `transfers.rs` and in `crates/thriftp-mcp/src/tools/connections.rs`, `browse.rs`, and `transfers.rs`; add coverage in `crates/thriftp-cli/tests/plain_ftp_warning.rs` and `crates/thriftp-mcp/tests/plain_ftp_warning.rs`. This is a US1 and US2 acceptance gate even though it is numbered in Polish for sequencing.
- [X] T107 [P] Add destructive-confirmation and pre-approval-policy tests in `crates/thriftp-core/tests/approval_confirmations.rs` and `crates/thriftp-mcp/tests/contract_approvals.rs`: single use, expiry, session/operation/target binding, headless denial, policy use, and `policy_used` audit records. This is a US2 acceptance gate even though it is numbered in Polish for sequencing.
- [X] T108 [P] Add MCP disabled-by-default, enablement, wildcard/non-loopback bind rejection, and remote-origin rejection tests in `crates/thriftp-mcp/tests/contract_security.rs` and `crates/thriftp-cli/tests/contract_mcp_admin.rs`. This is a US2 acceptance gate even though it is numbered in Polish for sequencing.
- [X] T109 [P] Add standalone/embedded parity and desktop/CLI/MCP equivalence contract tests in `crates/thriftp-mcp/tests/contract_parity.rs` and `crates/thriftp-tests/tests/host/test_equivalence.rs`, covering identical authorization, audit, denial, and transfer outcomes.
- [X] T110 [P] Add security-parity tests for native credential-store reuse, FTPS certificate validation, SSH host-key rejection, plain-FTP warnings, and OS-permission denial in `crates/thriftp-tests/tests/security/test_security_parity.rs` and `crates/thriftp-cli/tests/security_parity.rs`.
- [X] T111 [P] Add audit field and tamper-detection tests in `crates/thriftp-core/tests/audit_integrity.rs`, asserting every documented field, detecting record modification, and verifying that a broken chain is reported without silent repair.
- [X] T112 [P] Add a panic/`unwrap`/`expect` audit and negative tests for malformed input, broken pipes, and recoverable I/O failures in `crates/thriftp-tests/tests/security/test_panic_safety.rs`.
- [X] T113 [P] Add automated disconnect-policy tests in `crates/thriftp-mcp/tests/contract_disconnect.rs` and `crates/thriftp-tests/tests/host/test_disconnect_policy.rs`: verify `continue` and `cancel`, transfer continuation versus cancellation after disconnect, session termination, and the audited outcome. This closes the FR-082 verification gap.
- [X] T114 Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and the CLI/MCP contract suites; fix all failures in `Cargo.toml`, `crates/`, and `apps/desktop/`.
- [X] T115 Execute the scenarios in `specs/002-cli-mcp-support/quickstart.md` on macOS, Linux, and Windows, record elapsed times against SC-001 (under 2 minutes) and SC-020 (under 10 minutes), and record results and deviations in `specs/002-cli-mcp-support/validation.md`.
- [X] T116 Remove dead code and unused dependencies, confirm YAGNI, and update `Cargo.toml` and `crates/*/Cargo.toml` before final review.

### Analysis Remediation Coverage

| Finding | Closed by |
|---------|-----------|
| C1 Security review | T104 |
| C2 Public API rustdoc | T105 |
| C3 Plain-FTP warning | T106, T110 |
| I1 Explicit stdin mode | Spec FR-022/FR-023 and assumptions; contracts/cli.md; T019, T026 |
| I2 Local directory scopes | Spec FR-046 and Directory Scope entity; data-model.md; T010, T043, T048, T054 |
| I3 Audit tamper-evidence | Spec FR-060; contracts/automation-authorization.md; T111 |
| G1 Destructive approvals tests | T107 |
| G2 Disabled-by-default/remote-client tests | T108 |
| G3 Parity/equivalence tests | T109 |
| G4 Security-parity tests | T110 |
| U1 Remote-only MCP mutations | Spec FR-064 and Out of Scope; contracts/mcp-tools.md; T047, T048 |
| U2 `retention.rs` in plan | plan.md automation tree; T062 |
| U3 Entity terminology | Spec Key Entities; data-model.md |
| U4 Bootstrap exemption | Spec FR-035; contracts/mcp-tools.md; T046 |
| U5 Onboarding timing | T115 |
| G1 (analysis) Disconnect-policy tests | T113 |
| A1 Overwrite confirmation | Spec FR-016; contracts/cli.md; T030, T034 |
| A2 Storage limit and disconnect policy | Spec FR-081/FR-082; data-model.md; T010, T013, T062 |
| A1 (analysis) Disconnect-policy surface | Spec FR-082; contracts/cli.md; T054, T064, T066, T072 |
| A2 (analysis) Plain-FTP browsing/transfer warnings | Spec FR-083; contracts/cli.md Browsing and Transfers rules; T106, T110 |
| T1 Story-specific tests in Polish | US1/US2 checkpoint gates; T106–T108 and T113 notes |
| D1 Scaffolding duplication | T019, T020 descriptions |
| D2 Frontend CI stage | T005 |
| M1 Panic safety | T112 |

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies; start immediately.
- **Foundational (Phase 2)**: Depends on Setup; blocks all user stories.
- **US1 (Phase 3)**: Depends on Foundational; delivers the CLI MVP.
- **US2 (Phase 4)**: Depends on Foundational; can run in parallel with US1, but the P1 sequence recommends US1 first.
- **US3 (Phase 5)**: Depends on US2 for enrollment, grants, MCP tools, and audit; also uses US1's CLI command framework.
- **US4 (Phase 6)**: Depends on US1, US2, and US3 because it documents and validates their final surfaces.
- **US6 (Phase 7)**: Depends on US1 and the foundational host; can begin after US1.
- **US5 (Phase 8)**: Depends on US1 for command output and exit behavior; placed last by priority.
- **Polish (Phase 9)**: Depends on all desired user stories. T104–T113 close the analysis findings and run before final validation T114–T116.

### User Story Dependencies

| Story | Priority | Depends on | Independent Test |
|-------|----------|------------|------------------|
| US1 | P1 | Foundational | Documented CLI profile, browse, transfer, directory, queue, conflict, and secret scenarios |
| US2 | P1 | Foundational; optional US1 for CLI approval | Enroll, approve, authenticate, browse/transfer in scope, deny out-of-scope paths and audit |
| US3 | P2 | US2, US1 | Inspect, revoke, disconnect, disable, audit, retention, and concurrent-server behavior |
| US4 | P2 | US1, US2, US3 | Help, version, tool list, schemas, docs examples, unsupported-version error |
| US6 | P2 | US1, Foundational host | Process exit, host loss, interruption, resume, and shared state |
| US5 | P3 | US1 | JSON envelope, exit-code matrix, stream separation, broken pipe |

### Within Each User Story

- Write the listed tests first and confirm they fail.
- Models and public types before services.
- Services before command or tool adapters.
- Adapters before end-to-end integration.
- Complete a story and its independent test before moving to the next priority.

### Parallel Opportunities

- Setup tasks marked `[P]` use different files and can run together.
- Foundational tasks marked `[P]` use different modules or crates and can run together.
- Within US1, T028–T032 target different command files and can run together after T027.
- Within US2, T042–T044 and T047–T048 target different core/tool files and can run together after the identity model is in place.
- Within US3, CLI audit work, retention service work, and desktop API work can run together.
- US4 documentation files are all independent and marked `[P]`.
- US6 test files and implementation files are independent enough for parallel work once the host foundation is stable.
- US5 schema, exit-code, and stream tests are independent.
- Polish benchmarks, security tests, large-listing tests, SBOM, README links, the security review, rustdoc gate, plain-FTP warnings, and the added confirmation/security/parity/disconnect/tamper/panic test suites are independent and marked `[P]`; only T114–T116 final validation and cleanup depend on them.

---

## Parallel Example: User Story 1

```bash
# Tests first (different files):
Task: "CLI contract tests in crates/thriftp-cli/tests/contract_commands.rs"
Task: "CLI transfer integration tests in crates/thriftp-cli/tests/transfer_scenarios.rs"
Task: "CLI secret-input tests in crates/thriftp-cli/tests/secret_input.rs"

# Then implementation in parallel by command file:
Task: "Profile commands in crates/thriftp-cli/src/commands/profiles.rs"
Task: "Browse/stat commands in crates/thriftp-cli/src/commands/browse.rs"
Task: "Directory commands in crates/thriftp-cli/src/commands/files.rs"
Task: "Transfer commands in crates/thriftp-cli/src/commands/transfers.rs"
Task: "Queue commands in crates/thriftp-cli/src/commands/queue.rs"
```

## Parallel Example: User Story 2

```bash
# Authorization core in parallel:
Task: "Grant lifecycle in crates/thriftp-core/src/automation/grants.rs"
Task: "Canonical scopes in crates/thriftp-core/src/automation/scope.rs"
Task: "Approvals in crates/thriftp-core/src/automation/approval.rs"

# Tool adapters in parallel:
Task: "Connections/browse/files tools in crates/thriftp-mcp/src/tools/"
Task: "Transfers/queue tools in crates/thriftp-mcp/src/tools/"
```

## Parallel Example: User Story 4

```bash
# All documentation files are independent:
Task: "Write docs/cli.md"
Task: "Write docs/mcp.md"
Task: "Write docs/automation-security.md"
Task: "Write docs/transfer-host.md"
Task: "Write docs/compatibility.md"
```

---

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 1: Setup.
2. Complete Phase 2: Foundational.
3. Complete Phase 3: US1.
4. Stop and validate the CLI independently with the US1 tests and the CLI scenarios in `quickstart.md`.
5. Deliver the CLI MVP.

### Incremental Delivery

1. Setup + Foundational → foundation ready.
2. US1 → CLI MVP.
3. US2 → MCP automation differentiator.
4. US3 → control, audit, and revocation.
5. US4 → discoverability and documentation.
6. US6 → process lifetime and shared state.
7. US5 → machine-readable output hardening.
8. Polish → performance, security, supply chain, and cross-platform validation.

### Parallel Team Strategy

- Team completes Setup + Foundational together.
- After Foundational, one developer takes US1 (CLI) and another takes US2 (MCP core and tools).
- US3 follows US2; US4 follows US1–US3.
- US6 follows US1; US5 follows US1.
- Polish is shared after all selected stories.

---

## Notes

- `[P]` tasks use different files and have no dependency on an incomplete task.
- `[Story]` labels map every story-phase task to `US1`–`US6` for traceability.
- Tests are required by FR-005 and the constitution; do not skip the contract, authorization, audit, and multi-process suites.
- Never write secrets to arguments, environment variables, stdout, stderr, logs, errors, audit records, or test fixtures.
- Do not add an always-running daemon, remote MCP transport, favorites tools, shell tools, or a second protocol implementation.
- Keep Docker/container runtimes as development and CI tools only; they must never become runtime dependencies of the CLI or MCP server.
- Commit after each task or logical group and rerun the independent test for the story before moving on.

---

## Phase 10: Convergence

- [X] T117 Replace production `expect`/panic runtime construction in `crates/thriftp-core/src/host/server.rs` and `crates/thriftp-mcp/src/server.rs` with propagated errors or a shared fallible runtime per Constitution V (partial)
- [X] T118 Add rustdoc for all new public automation, host, approval, retention, and service APIs in `crates/thriftp-core/src/automation/`, `crates/thriftp-core/src/host/`, `crates/thriftp-cli/src/`, and `crates/thriftp-mcp/src/`, and enforce a `missing_docs` or `cargo doc` gate per Constitution V (partial)
- [X] T119 Reload persisted automation state before every MCP authorization decision, re-check enabled state, revocation epoch, grant revision and expiry, and stop in-flight multi-file operations at the next file boundary per FR-040, US3/AC2, US3/AC6, SC-014 (contradicts)
- [X] T120 Remove the agent-supplied `confirmed: true` bypass and require a user-issued single-use confirmation or scoped pre-approval policy for every destructive MCP operation, auditing all denials, per FR-053, FR-054, FR-055, SC-009, SC-010 (contradicts)
- [X] T121 Refuse to start stdio and loopback MCP transports while MCP is disabled and return documented enablement guidance, per US2/AC1, FR-074, SC-005 (contradicts)
- [X] T122 Write and fsync the durable audit record for a destructive MCP operation before executing it, and abort when that append fails, per FR-056, FR-062 (contradicts)
- [X] T123 Implement embedded MCP lifecycle in `apps/desktop/src-tauri/src/commands/automation.rs` and `apps/desktop/src-tauri/src/lib.rs`: start/stop the loopback server on enable/disable, publish endpoint and token, terminate sessions on shutdown, and add desktop command tests, per FR-045, FR-068, US2/AC10 (missing)
- [X] T124 Remove the `--token` command-line option from `crates/thriftp-mcp/src/main.rs` and load the bearer token only from a protected file or stdin, per FR-073, Constitution III, SC-004 (contradicts)
- [X] T125 Integrate desktop transfers with the single-owner transfer host so the desktop observes and controls CLI/MCP transfers, and closing or quitting cannot silently cancel them, per FR-031, FR-032, US6/AC3, US6/AC4, SC-024 (missing)
- [X] T126 Add explicit `--password-stdin` and `--key-passphrase-stdin` handling to CLI browse, stat, upload, download, and transfer operations when no stored credential exists, preserving prompt-free failure, per FR-022, FR-023, SC-022 (missing)
- [X] T127 Wire `trust-host-key` and `forget-host-key` fingerprints into the SFTP `known_hosts` verification path and require the exact fingerprint with `--yes` for new or changed host keys, per FR-024 (partial)
- [X] T128 Apply conflict policy, recursive directory handling, destination existence checks, and wait/timeout behavior in MCP upload and download tools on the direct path instead of blindly joining and overwriting destinations, per FR-009, FR-017, FR-025, FR-064 (contradicts)
- [X] T129 Resolve remote symlinks during MCP scope validation and deny any link whose canonical target escapes the granted scope, per FR-049, SC-008 (missing)
- [X] T130 Reject expired MCP grants on every operation independently of the session lifetime and audit the expiry, per FR-039, SC-015 (missing)
- [X] T131 Audit every MCP attempt, including missing-session, unenrolled, grant-missing, expired, revoked, disabled, and bootstrap status, challenge, and enrollment-status calls, per FR-057, SC-011 (missing)
- [X] T132 Populate audit records with target, client identity and display name, session ID, duration, and redacted error details, per FR-058, SC-011 (partial)
- [X] T133 Execute audit retention and storage-limit checks automatically at startup and daily for the CLI and MCP, deleting only sealed segments after the notice window and failing closed with `audit_unavailable`, per FR-059, FR-081, SC-013 (missing)
- [X] T134 Emit transfer progress and diagnostics on stderr in both human and JSON modes without contaminating stdout, per FR-019, FR-028 (missing)
- [X] T135 Report skipped symbolic links visibly in recursive CLI transfers instead of silently continuing, per FR-026 (partial)
- [X] T136 Print plain-FTP unencrypted-transport warnings to stderr before and during CLI operations in addition to the JSON warning field, per FR-083, SC-026 (partial)
- [X] T137 Publish output schemas for every MCP tool through `tools/list` in addition to input schemas, per FR-065 (partial)
- [X] T138 Mount the automation control surface in `apps/desktop/src/App.svelte` and display grants, scopes, and active sessions with revoke and disconnect actions, per FR-041, US3/AC1, FR-082 (partial)
- [X] T139 Revalidate canonical paths after create and rename operations before continuing or returning success, per FR-051 (missing)

---

## Phase 11: Convergence

- [X] T140 Complete rustdoc for every new public automation, host, approval, retention, and service API and enforce a `missing_docs` lint or equivalent gate in CI per Constitution V (partial)
- [X] T141 Propagate audit append failures from the MCP error path and return `audit_unavailable` instead of ignoring the failure per FR-061, SC-012 (contradicts)
- [X] T142 Implement a user-controlled single-operation confirmation surface that issues binding confirmation IDs for CLI and desktop approval flows while keeping policies opt-in, per FR-053, FR-054, SC-009 (missing)
- [X] T143 Apply each grant's disconnect policy to real MCP transport disconnects and process exit, auditing `continue` and `cancel` outcomes per FR-082 (missing)
- [X] T144 Delegate every desktop queue and transfer control command, including pause/resume, concurrency, bandwidth, reorder, priority, cancel-all, restart, and clear-history, to the host when the desktop is an observer, per FR-031, SC-024 (partial)
- [X] T145 Revalidate the grant revision, revocation epoch, and expiry at each file boundary for host-delegated MCP transfers and stop queued work after revocation, per FR-040, US3/AC6 (partial)
- [X] T146 Deny remote symlinks whose targets escape the granted scope during recursive MCP listings instead of silently skipping them, and audit the denial, per FR-049, SC-008 (contradicts)
- [X] T147 Add a CLI option to preserve supported symbolic links in recursive uploads and downloads while retaining skip-with-report as the fallback, per FR-026 (partial)
- [X] T148 Publish tool-specific output schemas through `tools/list` instead of only a generic result envelope, per FR-065 (partial)
- [X] T149 Expire and remove MCP session records from `run/mcp-sessions.json` on disconnect, revocation, expiry, and process exit, per FR-041 (partial)
- [X] T150 Emit chunk-level transfer progress on stderr during long CLI transfers, not only after each file completes, per FR-028 (partial)
- [X] T151 Warn about plain FTP before the `--no-wait` delegation path and before every connection attempt, per FR-083, SC-026 (partial)
- [X] T152 Honor `timeoutMs` and `wait` semantics for host-delegated MCP transfers, per FR-009, FR-064 (partial)
- [X] T153 Map `thriftp mcp serve` disabled-state errors to `mcp_disabled` with exit 9 and enablement guidance, per FR-074, US2/AC1 (partial)

---

## Phase 12: Convergence

- [X] T154 Fail closed with `audit_unavailable` when the audit storage limit is exceeded and no sealed segment can be safely deleted, blocking MCP operations until retention can proceed, per FR-081, SC-012 (missing)
- [X] T155 Implement cross-process client disconnect: `thriftp mcp clients disconnect` and desktop `automation_disconnect` must signal the running MCP process so active sessions terminate and owned transfers follow the grant's disconnect policy with an audited outcome, per FR-041, FR-082 (missing)
- [X] T156 Make every mutating CLI transfer and queue command (`transfer pause/resume/cancel/priority/reorder`, `queue pause/resume/cancel-all`, `config concurrency/bandwidth`) acquire the transfer host or delegate to the current host instead of writing `transfers.json` directly, per FR-031 and the transfer-host contract invariant that only the host writes transfer state (contradicts)
- [X] T157 Broadcast host `event.transferProgress`, `event.transferState`, and `event.queueChanged` notifications to subscribed control clients during transfers, per contracts/transfer-host.md, FR-031, SC-024 (missing)
- [X] T158 Implement long-metadata rendering for `thriftp browse --long` or remove and document the flag, per FR-013, T029 (partial)
- [X] T159 Implement symbolic-link preservation where the protocol interface supports it, or document the capability limitation and make `--preserve-symlinks` behavior explicit, per FR-026 (partial)

---

## Phase 13: Convergence

- [X] T160 Add an `events.subscribe` helper to `HostClient` and make the desktop observer subscribe to host `event.transferProgress`, `event.transferState`, `event.queueChanged`, and `event.hostShutdown`, updating transfer and queue stores without polling, per FR-031, SC-024 (missing)
- [X] T161 Emit host `event.transferProgress` notifications during long-running transfers (at a bounded interval), not only after each transfer record is persisted, per contracts/transfer-host.md, FR-031 (partial)
- [X] T162 Include active session IDs and counts in `thriftp mcp clients list` and `mcp status`, reading the session registry and omitting stale/expired entries, per FR-041, US3/AC1 (partial)
