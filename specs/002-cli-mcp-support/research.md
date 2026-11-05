# Phase 0 Research: CLI and MCP Automation Interfaces

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

All technical unknowns and the specification's deferred architectural decisions are resolved below.

## Decision 1: CLI framework and command model

**Decision**: Build the `thriftp` binary with `clap` 4.5 in derive mode. Use a subcommand tree (`profiles`, `browse`, `stat`, `upload`, `download`, `mkdir`, `rename`, `delete`, `queue`, `transfer`, `config`, `mcp`, `audit`) with global options for output mode, profile selection, color, and verbosity. Parse arguments into typed command structs and dispatch to core services; no command handler contains protocol logic.

**Rationale**: `clap` is the de facto Rust CLI library, generates help and completions, provides deterministic usage errors, and supports strict validation of conflicting or missing options. Derive mode keeps the command surface reviewable and testable.

**Alternatives considered**:
- `argh`: smaller but less flexible for nested groups and global options.
- Hand-written parsing: error-prone and produces inconsistent help and exit codes.
- A shell wrapper: platform-specific and weakens testability.

## Decision 2: Secret input and non-interactive behavior

**Decision**: Secrets are accepted only from the native OS credential store or from standard input when an explicit `--password-stdin` flag is present. The CLI reads one line, trims only the line terminator, wraps the value in a zeroizing secret type, and never places it in arguments, environment variables, output, logs, errors, or process titles. Without a stored credential and without `--password-stdin`, a non-interactive command fails immediately with `credential_missing`. The CLI never prompts on a TTY implicitly.

**Rationale**: The specification forbids secrets in arguments and environment variables and requires prompt failure in non-interactive runs. An explicit stdin flag is script-friendly, avoids accidental hangs, and makes secret flow auditable.

**Alternatives considered**:
- `--password <value>`: leaks through process lists and shell history; forbidden.
- `THRIFTP_PASSWORD`: leaks through environment, crash dumps, and child processes; forbidden.
- Interactive prompt by default: can hang CI jobs and violates the non-interactive contract.
- `rpassword`: useful for echo suppression, but an implicit TTY prompt is not desired; a piped stdin reader plus zeroization is sufficient.

## Decision 3: MCP SDK and supported protocol revisions

**Decision**: Use the official `rmcp` Rust SDK, pinned to 3.8.0, which implements the stable MCP `2026-07-28` revision and remains compatible with `2025-11-25` and earlier. Support `2026-07-28` as the primary revision and `2025-11-25` as the legacy revision. Advertise supported versions through discovery and return a structured `UnsupportedProtocolVersionError` with the supported list for anything else. Raise the workspace MSRV to Rust 1.88.0, the minimum required by `rmcp` 3.8.

**Rationale**: `rmcp` is the official, actively maintained Rust implementation with Tokio support, protocol-version negotiation, stdio and Streamable HTTP transports, and JSON Schema tooling. Supporting the current stable revision plus the immediately previous revision gives clients a practical compatibility window without maintaining an unbounded matrix. The 2026-07-28 revision removes the initialize handshake and requires `server/discover`, so the server must implement both the modern stateless path and the legacy handshake path through the SDK.

**Alternatives considered**:
- Hand-written MCP implementation: duplicates a large, evolving protocol and increases security risk.
- Support only `2026-07-28`: rejects clients still on the previous revision during the transition.
- Support every historical revision: unbounded test and compatibility burden.
- Pin an older `rmcp` that supports Rust 1.85: loses current protocol support and fixes; the MSRV bump is acceptable for a project targeting current toolchains.

## Decision 4: MCP transports

**Decision**: Use stdio as the transport for the standalone `thriftp-mcp` process. Use a loopback-only Streamable HTTP endpoint, bound to `127.0.0.1` with an ephemeral port, for embedded mode inside the desktop application. Both transports share the same message handler, tool registry, authorization service, and audit service. The embedded endpoint requires a per-launch bearer token stored with restrictive permissions and is never bound to a non-loopback interface. Stdout is reserved for protocol messages; help, logs, and diagnostics go to stderr.

**Rationale**: Stdio is the natural transport when an MCP client launches a headless server and avoids opening a port. Embedded mode cannot own the desktop process's stdio, so a loopback endpoint is the least-privilege alternative. Sharing one handler guarantees behavioral parity and makes contract tests transport-agnostic.

**Alternatives considered**:
- Loopback HTTP for both modes: opens a port even when stdio is sufficient and complicates client configuration.
- Stdio for embedded mode: conflicts with the desktop process lifecycle and logging.
- Remote HTTP: explicitly out of scope and forbidden in the initial release.

## Decision 5: MCP client identity proofing

**Decision**: Each MCP client generates an Ed25519 keypair locally. Its identity is the SHA-256 fingerprint of the public key. Enrollment sends the public key, fingerprint, and a user-visible display name; the server creates a pending request but grants nothing. On approval, a grant is bound to that fingerprint. Session establishment uses a server-issued random nonce that the client signs with its private key; the server verifies the signature and issues a short-lived, transport-bound session credential. Grants are never transferable because possession of the private key is required for every new session.

**Rationale**: This provides stable, user-visible identity, cryptographic proof of possession, replay resistance, and non-transferable grants without storing a long-lived shared secret on the server. It also avoids coupling local authorization to the MCP transport or to an external identity provider.

**Alternatives considered**:
- Bearer token stored on disk: simple, but a copied token lets another client impersonate the enrolled client.
- Trusting the MCP `clientInfo` name: trivially spoofable.
- OAuth with an external provider: unnecessary for a local-only, offline-first tool and adds a cloud dependency.
- OS user identity alone: does not distinguish multiple agents running as the same user.

## Decision 6: Enrollment and approval surfaces

**Decision**: Enrollment requests are visible and actionable through both the CLI (`thriftp mcp clients pending`, `approve`, `deny`, `revoke`, `disconnect`) and the desktop automation control surface. Approval selects a grant scope: connection profiles, canonical directories, permitted operations, optional expiry, and optional destructive pre-approval policies. Pending requests expire after 10 minutes by default, are capped per client and globally, and never grant access implicitly. In headless mode, the CLI is the user-controlled approval interface.

**Rationale**: The specification requires explicit user approval and a control surface for headless operation. Providing both surfaces keeps standalone and embedded modes usable. Expiry and caps prevent unbounded pending-request accumulation.

**Alternatives considered**:
- Desktop-only approval: unusable for standalone headless servers.
- Auto-approval for loopback or stdio clients: violates the explicit-grant requirement and treats local transport as authorization.
- Approval by editing a JSON file: error-prone and bypasses validation and audit.

## Decision 7: Grant model, revisions, and revocation

**Decision**: A grant records the client fingerprint, granted profile IDs, directory scopes, operation set, optional expiry, and optional pre-approval policies. Every change increments a `revision`. Every session records the revision it authenticated under. Revocation increments a `revocationEpoch`, terminates active sessions, and causes in-flight operations to stop at the next file boundary or before the next operation. The authorization service re-checks the grant, revision, epoch, and expiry on every request and again at each file boundary for recursive or multi-file operations.

**Rationale**: Revision and epoch checks close the race between revocation and long-running work while keeping the check cheap enough to run frequently. Stopping at file boundaries avoids corrupting a partially written file.

**Alternatives considered**:
- Checking only at session start: allows work to continue after revocation.
- Aborting mid-chunk: risks partial writes and inconsistent state.
- Deleting grants immediately: loses the audit history needed to explain past access.

## Decision 8: Directory scope canonicalization

**Decision**: Grants store canonical remote scopes (associated with a connection profile) and canonical local scopes. Every request resolves and canonicalizes the target before authorization:
- Local scopes govern MCP upload sources and download destinations. Local paths use the OS canonicalizer, resolve `.` and `..`, reject invalid or non-absolute paths, and check containment using path-component boundaries (not string prefixes). Symbolic links are resolved; a link whose target escapes the scope is denied. If resolution fails, the request is denied.
- Remote POSIX paths are normalized lexically (collapse separators, resolve `.` and `..`, reject encoded separators and NUL), then resolved with the protocol's real-path or equivalent operation when available. For SFTP, use real-path and link-target inspection. For FTP/FTPS, where real-path support is not guaranteed, reject any path containing a symbolic-link component that cannot be verified, and deny ambiguous paths. Directory components are checked in order.
- The same rules apply to files, directories, uploads, downloads, renames, and deletes. Renames validate both source and destination, and revalidate after the operation.

**Rationale**: Lexical normalization alone does not prevent symbolic-link escapes, and string-prefix checks allow sibling-directory escapes. Denying ambiguous paths is safer than guessing and satisfies the specification's fail-closed requirement.

**Alternatives considered**:
- String prefix matching: vulnerable to `/allowed-evil` and separator tricks.
- Following links and checking the final target only: permits time-of-check/time-of-use races and intermediate escapes.
- Best-effort resolution on FTP: can silently authorize an escaped path; denial is safer.

## Decision 9: Destructive operations and confirmations

**Decision**: Treat delete, rename, and overwrite of an existing destination as destructive. By default, each MCP destructive request requires a single-use confirmation delivered through an available user channel: MCP elicitation when the client supports it, otherwise a pending approval in the desktop control surface or the CLI. Confirmations expire after 2 minutes and are bound to the session, operation, and canonical target. A grant may include a scoped, optionally expiring pre-approval policy; every policy use is audited. In standalone headless mode without an approval channel or applicable policy, the operation is denied.

**Rationale**: This matches the specification's default-confirmation and headless-deny rules, prevents a reusable confirmation from authorizing a different target, and gives users a deliberate opt-in for repetitive automation.

**Alternatives considered**:
- A global `--yes` equivalent for MCP: too broad and not scoped.
- Treating loopback or stdio as implicit trust: forbidden.
- Time-based auto-approval without a policy: violates explicit authorization.

## Decision 10: Audit durability and tamper evidence

**Decision**: Write audit records to daily append-only JSONL segments (`audit/YYYY-MM-DD.jsonl`). Each record includes a monotonic sequence, timestamp, interface, client identity, session, operation, canonical target, result, redacted error code, runtime mode, previous-record hash, and record hash. Append the record and `fsync` the file before returning any result or, for destructive operations, before execution. On startup, verify the hash chain and report tampering; a broken chain does not silently repair itself. A record write failure blocks the operation. Audit files use restrictive permissions, and no MCP tool can modify or delete them.

**Rationale**: Append-only segments bound disk usage and simplify retention. Hashing makes silent edits detectable. Fsync-before-result is required by the fail-closed rule. Refusing to repair a broken chain preserves evidence.

**Alternatives considered**:
- A database: adds a dependency and operational complexity for a local single-user tool.
- Buffered writes: can lose the record on crash, violating fail-closed.
- HMAC with a key in the credential store: stronger against forgery but makes every audit write depend on keychain availability and complicates recovery; hash chaining plus OS permissions is proportionate for the initial release.
- One unbounded file: difficult to rotate, verify, and retain.

## Decision 11: Audit retention and deletion notice

**Decision**: Retain audit segments for 90 days by default, configurable from 1 to 3,650 days. Apply a default 1 GiB storage limit so a busy server cannot exhaust the disk; the limit is configurable or nullable for retention-only operation. Run a daily retention check when the automation service starts and once per day while it runs. Before deleting any segment, write a `retention_notice` audit record at least 7 days before the planned deletion and surface the notice in the CLI and desktop control surface. Never delete a segment if the notice cannot be durably written or if the configured retention has not elapsed. When the storage limit is reached, only the oldest sealed segments are eligible, under the same notice rule; if no segment can be deleted after notice, new operations are blocked with `audit_unavailable`. Record each completed deletion, including the affected range.

**Rationale**: The specification requires a user-configurable retention period, clear notice before deletion, and protection from premature removal. A seven-day notice window gives users time to export or change the policy. Writing the notice into the audit log itself keeps the evidence with the data.

**Alternatives considered**:
- Immediate deletion at expiry: violates the notice requirement.
- No automatic deletion: unbounded growth and eventual disk pressure.
- Deleting without an audit record: destroys evidence of the deletion.

## Decision 12: Transfer process ownership

**Decision**: Use a single-owner **transfer host** model. Exactly one process at a time holds an exclusive advisory lock on `run/host.lock` and owns transfer execution. The desktop application tries to become host on launch. The CLI and standalone MCP server become host only when no host exists and they need to execute or control transfers; otherwise they connect to the existing host as clients. Embedded MCP always uses the desktop host. The host exposes a local-only control endpoint and publishes its instance ID, PID, endpoint, version, and a per-instance access token in `run/host.json`.

**Rationale**: A single owner prevents concurrent writes to the queue and partial-file state and gives all surfaces one authoritative view. It avoids an always-running daemon: the host is whichever long-lived process is already running, or the foreground command process. It also preserves the specification's rule that a CLI transfer command remains active until a terminal state.

**Alternatives considered**:
- Multiple processes writing the queue with file locks per record: complex, prone to deadlocks, and hard to make consistent.
- A separate always-running supervisor daemon: violates the initial-release scope and adds installation and lifecycle burden.
- Letting each surface run its own transfers: causes conflicting states and duplicate work.

## Decision 13: Local control channel protocol and security

**Decision**: Use newline-delimited JSON-RPC 2.0 over a Unix domain socket on macOS/Linux and a named pipe on Windows, created under the application-data directory with same-user permissions. Authenticate clients with the per-instance token from `run/host.json`; reject any connection without it. Bind no TCP port. The channel exposes read methods (`host.status`, `transfers.list`, `transfers.status`), control methods (`transfers.submit`, `pause`, `resume`, `cancel`, `priority`, `reorder`, `queue.setConcurrency`, `queue.setBandwidth`), and an event subscription for progress and state changes. The host validates every request against the shared core rules.

**Rationale**: Local sockets and named pipes are not reachable from the network, are supported on all three target platforms, and avoid port conflicts. A per-instance token provides defense in depth if another local process can access the endpoint. JSON-RPC keeps the protocol inspectable and testable.

**Alternatives considered**:
- Loopback TCP: reachable by other local processes and potentially by remote clients through misconfiguration; more attack surface.
- Shared files with polling: no reliable request/response semantics and higher latency.
- OS-specific proprietary IPC: duplicates abstractions that `interprocess` already provides.

## Decision 14: Host loss, process exit, and recovery

**Decision**: If the host exits while a CLI or MCP client is waiting:
- The client receives a `host_lost` notification and stops waiting.
- Transfers owned by the host are persisted as `interrupted` (never `completed`) at the last confirmed offset.
- The CLI reports `interrupted` with exit code 8 and leaves the transfer resumable; it does not auto-resume unless the user runs an explicit resume command.
- A waiting CLI process may acquire the host lock and resume with `transfer resume`, but never silently.
- A stale lock or `run/host.json` without a live owner is detected by PID liveness plus endpoint connection failure, cleaned up, and the next process becomes host.
- When the CLI is the host and its process exits, the OS releases the lock; on the next startup, stale `active` transfers become `interrupted`.
- Closing the desktop while it is host follows the same rule; closing it while it is only a client has no effect on the transfer.

**Rationale**: This satisfies the specification's requirement that a process exit never produces a false completion, that interruption is resumable, and that the desktop closing does not silently cancel a CLI transfer when another owner can continue.

**Alternatives considered**:
- Auto-resume on host loss: surprising and can duplicate work if the old host is merely paused.
- Treating interruption as failure: loses resumability.
- Blocking desktop quit indefinitely: poor user experience; the existing desktop plan already requires wait or cancel for its own transfers.

## Decision 15: CLI output envelope and exit codes

**Decision**: When `--output json` is selected, write exactly one JSON document to stdout on success or failure. Progress and diagnostics go to stderr as human-readable text or JSON lines. The envelope contains `schemaVersion`, `command`, `ok`, `exitCode`, `data`, `warnings`, `error`, and `meta` (duration, CLI version, interface version). Error objects contain a stable `code`, actionable `message`, `retryable`, and redacted `details`. Use these exit codes: 0 success, 2 usage, 3 authentication, 4 network, 5 permission, 6 conflict, 7 cancelled, 8 interrupted, 9 authorization/policy, 10 validation, 11 credential store, 12 unsupported capability or version, 13 internal. Human-readable mode uses the same codes and writes results to stdout with diagnostics on stderr.

**Rationale**: Stable envelopes and exit codes make automation dependable and testable. Reserving stdout for the result prevents progress messages from corrupting parsers. Distinct codes let scripts branch without parsing prose.

**Alternatives considered**:
- Free-form output: untestable and fragile.
- A single non-zero code: loses actionable distinctions.
- NDJSON progress on stdout: contaminates the result document; progress belongs on stderr.

## Decision 16: Shared error taxonomy and redaction

**Decision**: Reuse the core typed error taxonomy from `001` and map it into CLI error codes and MCP error codes. Add automation-specific errors: `mcp_disabled`, `client_not_enrolled`, `grant_missing`, `grant_expired`, `grant_revoked`, `scope_denied`, `path_ambiguous`, `confirmation_required`, `confirmation_expired`, `audit_unavailable`, `host_unavailable`, `host_lost`, and `unsupported_protocol_version`. Messages are actionable, never include secrets or raw server diagnostics, and are safe to show to users.

**Rationale**: A shared taxonomy keeps behavior consistent across surfaces and makes contract tests objective. Redaction at the boundary prevents accidental disclosure.

**Alternatives considered**:
- Separate CLI and MCP taxonomies: drift and duplicated mapping logic.
- Passing raw protocol errors through: may leak credentials or server internals.

## Decision 17: MCP tool catalog and long-running transfers

**Decision**: Namespace tools as `thriftp.<domain>.<action>` (for example `thriftp.files.list`, `thriftp.transfers.upload`, `thriftp.queue.set_bandwidth`). Expose tools for connections, browsing, stat, directory create/rename/delete, file upload/download, transfer status/pause/resume/cancel, concurrency, bandwidth, priority/reorder, and automation status/enrollment. Do not expose favorites, shell execution, or server administration. Upload and download enqueue work and return a `transferId` immediately; an optional `wait` argument with a bounded timeout blocks until a terminal state. Status and progress use polling through `thriftp.transfers.status`; MCP progress notifications are emitted when the client supports them. Every tool declares input and output schemas and a destructive flag.

**Rationale**: Long-running transfers should not hold a tool call open indefinitely. Returning a transfer ID gives clients control and keeps the protocol responsive. Polling plus optional notifications fits the stateless `2026-07-28` revision and legacy clients.

**Alternatives considered**:
- Blocking until completion: risks client timeouts and provides no control during large transfers.
- Server-initiated streaming of file bytes: MCP tools are not a file transport; use the transfer engine.
- Unnamespaced tools: collision-prone as the catalog grows.

## Decision 18: Shared persistence and recovery

**Decision**: Reuse `favorites.json` and `transfers.json`. Store automation state in `automation.json` with a schema version and ordered migrations. Store audit segments separately. Use atomic replace with backup for mutable files and append plus fsync for audit. On load, recover `active` transfers to `interrupted`, preserve `pending` and `paused`, verify grant consistency, expire overdue grants and enrollment requests, and clean stale runtime host files. Corrupt automation state is quarantined and reported rather than silently replaced.

**Rationale**: Reusing the existing stores keeps profiles, queue, and history consistent across surfaces. Separating audit prevents frequent security writes from contending with settings updates. Atomic writes and explicit recovery protect user data.

**Alternatives considered**:
- A new database for automation: heavier and unnecessary for the data volume.
- Writing automation state into `favorites.json`: mixes security state with user preferences and increases corruption impact.
- Silent reset on corruption: loses grants and audit context without explanation.

## Decision 19: Local-only enforcement

**Decision**: The standalone MCP server uses stdio only. Embedded mode binds to `127.0.0.1` on an ephemeral port and refuses any configuration that resolves to a non-loopback address or a wildcard bind. The server rejects remote-origin requests when it can detect them (for example a non-loopback `Host` or `Origin` header), uses a per-launch bearer token for the loopback endpoint, and exposes no network discovery or remote authorization flow. Enabling MCP is an explicit user action; the default is disabled.

**Rationale**: The specification limits the initial release to local-only clients. Defense in depth against misconfiguration prevents accidental exposure.

**Alternatives considered**:
- Allowing `0.0.0.0` for convenience: violates the local-only requirement.
- Relying only on firewall settings: not a product guarantee.
- Remote OAuth support: explicitly out of scope.

## Decision 20: Versioning and compatibility policy

**Decision**: The CLI reports its semantic version through `thriftp --version` and in every JSON envelope. The MCP server reports `serverInfo`, the Thriftp automation contract version, and supported MCP protocol revisions through `server/discover` and `initialize`. Contract versions follow semantic versioning: additive tools and fields are minor, breaking changes are major, and deprecations are announced for at least one minor release. The automation contract version is independent of the application version. Unsupported protocol versions receive a structured error listing supported revisions.

**Rationale**: Versioned interfaces let integrators build reliably and let Thriftp evolve without breaking clients. Separating the automation contract version from the application version avoids unnecessary churn.

**Alternatives considered**:
- No explicit version: clients cannot detect incompatibility.
- Tying the contract to the app version: a UI release would imply an interface break.
- Supporting only one MCP revision forever: blocks protocol evolution.

## Decision 21: Testing strategy and fault injection

**Decision**: Use layered tests separated by runtime requirements:
- Docker-free core unit tests for grants, scope canonicalization, approvals, audit hashing, and host state.
- Docker-free CLI contract tests with `assert_cmd`, `insta`, and `jsonschema`, covering every command, exit code, stream separation, stdin secrets, and help/version.
- Docker-free MCP contract tests with an `rmcp` client harness covering discovery, both protocol revisions, every tool, pagination, errors, and transport parity.
- Docker-free multi-process host tests that spawn the CLI and MCP binaries, verify single ownership, delegation, control, host loss, stale-lock recovery, and interruption.
- Docker-free fault injection for audit write failure, disk-full, permission denial, keychain unavailability, timeout, scope escape, revocation mid-transfer, and confirmation expiry.
- Protocol integration tests reuse the shared `001` Docker Compose harness under `docker/protocol/`, run through `scripts/test-integration.sh`. The same suite runs locally on macOS through Docker Desktop and in Linux CI; Windows developers with Docker Desktop may use it too.
- CI exposes `unit/contract`, `protocol-integration`, `multi-process`, `platform`, and `build` stages separately. A missing container runtime skips only `protocol-integration`.
- Docker is a development/CI tool only. It is not a CLI or MCP runtime dependency and is not required to build, package, or run either binary.

**Rationale**: The specification's guarantees are mostly negative and concurrency-related, so they need explicit fault injection and multi-process tests rather than happy-path mocks alone. The shared Compose harness keeps protocol tests identical for macOS developers and Linux CI, while keeping the fast contract and host suites independent of Docker.

**Alternatives considered**:
- Linux-only container tests: rejected by the constitution and the macOS development environment.
- Fakes only: misses real protocol interoperability.
- Real servers on every platform: flaky and environment-dependent.
- Docker as a CLI/MCP runtime dependency: rejected; both binaries must work without a container runtime.

## Decision 22: Documentation surfaces

**Decision**: Publish `docs/cli.md` (installation, commands, options, output, exit codes, examples, secret input), `docs/mcp.md` (transports, enrollment, grants, tools, schemas, examples, compatibility), `docs/automation-security.md` (scope rules, confirmations, audit, fail-closed behavior), `docs/transfer-host.md` (ownership, shared state, process exit), and `docs/compatibility.md` (versions and deprecation policy). Generate CLI help from the command definitions and keep MCP tool schemas as the source of truth for the documentation.

**Rationale**: The constitution and specification require equivalent documentation clarity for both interfaces. Separating operational and security topics keeps each guide usable.

**Alternatives considered**:
- A single monolithic guide: hard to navigate and maintain.
- Source-code-only documentation: fails the open-source and discoverability requirements.

## Decision 23: Performance targets and benchmarks

**Decision**: Add `criterion` benchmarks for scope canonicalization, authorization checks, audit record serialization and hashing, MCP tool dispatch, CLI JSON serialization, and host control round trips. Track CLI startup, first-entry latency, and transfer throughput in CI smoke benchmarks. Use the existing transfer-engine benchmarks for streaming and memory behavior.

**Rationale**: The constitution requires benchmarks for hot-path networking and parsing. Authorization and audit are on every MCP request, so their overhead must remain bounded. CLI startup matters for scripting.

**Alternatives considered**:
- No benchmarks: regressions in authorization and audit overhead would go unnoticed.
- Only end-to-end timing: too noisy to isolate regressions.

## Decision 24: Dependency and MSRV impact

**Decision**: Raise the workspace MSRV to Rust 1.88.0 for `rmcp` 3.8 and its dependencies. Pin all new dependencies, review licenses, and add them to the SBOM and dependency-audit workflow. Keep optional features minimal: stdio and Streamable HTTP server support only, no OAuth client, no external telemetry, and no remote transport.

**Rationale**: The current MCP SDK requires Rust 1.88. A current MSRV is reasonable for a project targeting current toolchains. Minimal features reduce attack surface and build size.

**Alternatives considered**:
- Pin an older SDK: loses current protocol support and fixes.
- Vendor the SDK: maintenance and security-review burden.
- Enable all features: unnecessary OAuth and remote code paths.

## Decision 25: Concurrent standalone and embedded MCP servers

**Decision**: Allow both servers to run at the same time. They share `automation.json` through an exclusive cross-process write lock and a monotonically increasing `stateRevision`, and they append to one audit store through an exclusive `audit.lock`. Every request checks whether the revision changed and reloads settings and grants before authorizing. Revocation, expiry, disablement, and scope changes therefore take effect in every server no later than the next request or file boundary. Sessions remain per-process and are terminated by the server that observes the invalidation.

**Rationale**: The specification explicitly covers standalone and embedded servers running together. A shared revision and a single audit chain provide consistency without a coordinator process or an always-running daemon.

**Alternatives considered**:
- Forbid concurrent servers: fails the specification's edge case.
- Elect a single MCP server: adds another ownership protocol and prevents an embedded desktop and a headless server from coexisting.
- Rely on file modification times: unreliable for correctness and vulnerable to lost updates.

## Decision 26: Scope discipline and exclusions

**Decision**: Do not add favorites tools, arbitrary shell commands, server-administration tools, remote MCP access, cloud synchronization, an interactive CLI shell, a TUI, plugins, or an always-running daemon. Do not duplicate protocol adapters. Do not add a second credential store. Do not expose audit deletion to MCP clients.

**Rationale**: These exclusions are explicit in the specification and constitution. Applying YAGNI keeps the initial release reviewable and secure.

**Alternatives considered**:
- Adding favorites "for parity": explicitly out of scope; the desktop remains the favorites surface.
- A TUI: not requested and expands scope significantly.

## Resolved Unknowns Summary

| Unknown or open decision | Resolution |
|--------------------------|------------|
| CLI framework | `clap` 4.5 derive with a typed subcommand tree |
| Secret input | Native credential store or explicit `--password-stdin`; zeroized; never args/env |
| MCP SDK | `rmcp` 3.8.0; Rust 1.88 MSRV |
| MCP protocol revisions | `2026-07-28` primary, `2025-11-25` legacy; structured unsupported-version error |
| MCP transports | Stdio standalone; loopback Streamable HTTP embedded; stdout protocol-only |
| Client identity | Ed25519 keypair; SHA-256 fingerprint; signed nonce session proof |
| Enrollment | Explicit CLI/desktop approval; 10-minute expiry; no implicit local trust |
| Grants | Fingerprint-bound, scoped, expiring, revisioned, immediately revocable |
| Scope enforcement | Canonical remote and local scopes, component-boundary containment, symlink resolution/denial, revalidation |
| Destructive actions | Per-operation confirmation; scoped pre-approval policy; headless deny |
| Audit | Daily hash-chained JSONL; fsync before result; fail closed; 90-day default retention, 1 GiB default limit, 7-day notice |
| Concurrent MCP servers | Shared `stateRevision` and automation write lock; cross-process audit lock; revocation observed on next request or file boundary |
| Transfer process model | Single-owner transfer host; local control channel; desktop/CLI/MCP share state |
| Host loss | Persist interrupted at last confirmed offset; report, never complete; explicit resume |
| Local IPC | Unix domain socket / Windows named pipe; JSON-RPC 2.0; per-instance token |
| CLI output | One JSON document on stdout; progress on stderr; stable schema |
| Exit codes | 0, 2–13 mapped to documented categories |
| MCP long transfers | Enqueue and return transfer ID; polling plus optional wait and progress notifications |
| Persistence | Reuse `favorites.json`/`transfers.json`; add `automation.json` and audit segments |
| Local-only enforcement | Stdio or 127.0.0.1; reject wildcard/remote binds; loopback bearer token |
| Versioning | Semantic contract versions; `server/discover`; deprecation policy |
| Testing | Docker-free unit/contract/multi-process tests; shared Docker Compose protocol integration on macOS Docker Desktop and Linux CI; separate CI stages |
| Documentation | Dedicated CLI, MCP, security, host, and compatibility guides |
| Benchmarks | Scope checks, authorization, audit, dispatch, serialization, host round trips |
| Exclusions | No favorites, shell, remote MCP, cloud sync, TUI, plugins, or always-running daemon |
