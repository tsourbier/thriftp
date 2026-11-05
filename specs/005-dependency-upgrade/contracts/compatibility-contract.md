# Compatibility Contract: Dependency and Toolchain Upgrade

**Feature**: `005-dependency-upgrade` | **Date**: 2026-11-05

## Purpose

This upgrade must not change the product's external contracts. This document freezes the interfaces that must remain compatible and defines how compatibility is verified. Any intentional change requires a revision to the feature specification and explicit maintainer approval; it cannot be introduced as an incidental upgrade side effect.

## Contract References

The following existing contracts remain authoritative:

| Contract | Location | Compatibility requirement |
|---|---|---|
| Desktop IPC | [`specs/001-desktop-file-transfer/contracts/desktop-ipc.md`](../../001-desktop-file-transfer/contracts/desktop-ipc.md) | Command names, request/response shapes, event names, and error codes remain compatible |
| Protocol adapter | [`specs/001-desktop-file-transfer/contracts/protocol-adapter.md`](../../001-desktop-file-transfer/contracts/protocol-adapter.md) | FTP, explicit FTPS, implicit FTPS, and SFTP behavior and capability reporting remain compatible |
| Transfer engine | [`specs/001-desktop-file-transfer/contracts/transfer-engine.md`](../../001-desktop-file-transfer/contracts/transfer-engine.md) | Streaming, pause/resume/cancel, conflict handling, and progress reporting remain compatible |
| Credential store | [`specs/001-desktop-file-transfer/contracts/credential-store.md`](../../001-desktop-file-transfer/contracts/credential-store.md) | Storage, lookup, deletion, and error behavior remain compatible and no plaintext fallback is introduced |
| CLI | [`specs/002-cli-mcp-support/contracts/cli.md`](../../002-cli-mcp-support/contracts/cli.md) | Commands, flags, machine-readable output, exit codes, and credential behavior remain compatible |
| MCP tools | [`specs/002-cli-mcp-support/contracts/mcp-tools.md`](../../002-cli-mcp-support/contracts/mcp-tools.md) | Tool names, schemas, scopes, approvals, and results remain compatible |
| Automation authorization | [`specs/002-cli-mcp-support/contracts/automation-authorization.md`](../../002-cli-mcp-support/contracts/automation-authorization.md) | Enrollment, scopes, authorization, and audit behavior remain compatible |
| Transfer host | [`specs/002-cli-mcp-support/contracts/transfer-host.md`](../../002-cli-mcp-support/contracts/transfer-host.md) | Process model, ownership, checkpointing, and recovery behavior remain compatible |
| Favorites IPC | [`specs/003-server-favorites/contracts/favorites-ipc.md`](../../003-server-favorites/contracts/favorites-ipc.md) | Favorite commands, payloads, and error behavior remain compatible |
| Favorites credential store | [`specs/003-server-favorites/contracts/credential-store.md`](../../003-server-favorites/contracts/credential-store.md) | Shared credential references and deletion rules remain compatible |

## Frozen Behaviors

### Desktop interface

- The dual-pane workspace, tabs, favorites, queue, inspector, previews, and recovery surfaces remain available.
- Keyboard shortcuts, focus behavior, modal behavior, theme handling, and accessibility semantics remain equivalent.
- Existing user preferences, saved profiles, server favorites, directory favorites, tab state, queue state, transfer history, and audit records load without loss.
- No new mandatory telemetry, account, or cloud dependency is introduced.

### Command-line interface

- Documented commands and flags remain available.
- Machine-readable output fields and exit codes remain stable.
- Credential lookup and verification use the same secure store and rules.
- Secrets are not printed.

### Automation server

- Documented tool names, input schemas, and result shapes remain stable.
- Explicit enrollment, canonical scopes, per-operation confirmation, least privilege, and audit behavior remain unchanged.
- No weaker credential, verification, or authorization path is introduced.

### Protocols and transfers

- FTP, explicit FTPS, implicit FTPS, and SFTP connection, authentication, listing, upload, download, rename, and delete behavior remains equivalent.
- TLS certificate validation and SSH host-key verification remain enabled by default.
- Unexpected host-key changes remain detectable and visible.
- Plain FTP remains clearly identified as unencrypted before and during use.
- Streaming remains constant-memory; pause, resume, cancel, conflict handling, and progress events remain compatible.

### Security and privacy

- Credentials remain in the operating-system credential store.
- Passwords, private keys, tokens, and other secrets do not appear in logs, error messages, telemetry, or generated artifacts.
- Certificate and host-key overrides remain explicit, user-driven, and visible.
- Dependency changes do not add a new plaintext secret path or weaken least privilege.

### Data formats

- Existing JSON state files remain readable without manual migration.
- Any internal serialization change required by a dependency upgrade must preserve the persisted shape or provide a tested, automatic migration.
- Audit records remain append-only and hash-chain compatible.

## Allowed Changes

The upgrade may change:

- Dependency versions, resolution records, toolchain pins, CI action versions, and container base images.
- Internal APIs required to migrate to a breaking dependency release, provided external contracts and behavior are preserved.
- Test implementation details required by updated test tooling, provided coverage and assertions are not weakened.
- Documentation that records new minimum versions, upgrade evidence, and approved exceptions.

## Prohibited Changes

The upgrade must not:

- Remove or rename a documented CLI command, flag, output field, or exit code.
- Remove or rename a documented MCP tool, scope, approval rule, or audit field.
- Change a desktop IPC command or event in a way that breaks the existing frontend.
- Disable, weaken, or bypass certificate validation, host-key verification, credential storage, plain-FTP warnings, or secret redaction.
- Skip, delete, or weaken tests to make the build pass.
- Add mandatory telemetry, accounts, cloud services, or a new plaintext credential path.
- Introduce a dependency from an untrusted source without security review.

## Verification Matrix

| Contract area | Verification | Pass condition |
|---|---|---|
| Desktop IPC | Existing desktop tests and IPC contract tests | All pass; no breaking payload or event change |
| CLI | CLI contract tests and documented command smoke tests | Outputs, exit codes, and secrets behavior unchanged |
| MCP | MCP contract and authorization tests | Tools, scopes, approvals, and audit unchanged |
| Protocols | Protocol integration tests and manual smoke tests | FTP/FTPS/SFTP operations behave as before |
| Credentials | Credential-store tests and security review | Native store behavior preserved; no plaintext fallback |
| Transfers | Transfer tests, recovery tests, benchmarks | Behavior preserved; performance within 5% |
| Data formats | Fixture load tests and migration tests | Existing state loads without loss |
| Desktop UI | Frontend tests and manual keyboard/theme smoke test | Behavior, focus, and accessibility equivalent |
| Build and release | Cross-platform CI, packaging, reproducible build | All required checks green |
| Security | Audit, SBOM, license, secret-redaction tests | Zero critical/high; no disallowed license; no secret leak |

## Change Control

Any proposed deviation from this contract must:

1. Be documented in the feature specification or an approved architecture decision.
2. Include the affected users, compatibility impact, and migration path.
3. Receive maintainer and security review where credentials, cryptography, transport, or automation are affected.
4. Add or update automated coverage before merge.
5. Be called out in release notes.

Incidental contract changes discovered during implementation are treated as regressions, not as accepted upgrade outcomes.
