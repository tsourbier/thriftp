# Phase 0 Research: Thriftp Desktop File Transfer Client

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

All technical unknowns and the specification's open product decisions are resolved below.

## Decision 1: Protocol implementations

**Decision**: Use `suppaftp` 12.x with its Tokio and rustls support for FTP and explicit/implicit FTPS, and `russh` + `russh-sftp` for SFTP. Use `rustls` with native root certificates for TLS. Enable `suppaftp`'s `tokio-rustls-ring` feature and the `deprecated` feature required for implicit FTPS, isolating implicit FTPS behind the adapter so it can be replaced if upstream removes it. This requires Rust edition 2024 and the MSRV required by the pinned dependency set (at least Rust 1.85.1).

**Rationale**: The chosen libraries are Rust-native, actively maintained, and fit the async core. `suppaftp` covers FTP and FTPS with a TLS connector; `russh`/`russh-sftp` provides async SFTP, key authentication, and known-host verification without linking a C library. Pinning the implicit-FTPS dependency to the adapter limits the impact of its documented deprecation.

**Alternatives considered**:
- `ssh2`/libssh2: mature, but a C dependency with blocking APIs and more platform packaging risk.
- `async-ftp`: less feature-complete for FTPS modes.
- `native-tls`: platform-variable behavior and certificate-store differences; rustls gives consistent validation.
- Dropping implicit FTPS: rejected because the initial release explicitly supports it; the deprecated upstream path is isolated and monitored.

## Decision 2: Async transfer engine

**Decision**: Run the transfer engine on Tokio. Each transfer is a cancellable task; a shared semaphore enforces concurrency; a priority queue selects the next eligible transfer; a shared token bucket enforces the global bandwidth limit; pause and cancel use cooperative cancellation at chunk boundaries; progress events are emitted at a bounded cadence.

**Rationale**: The constitution requires asynchronous work that never blocks the UI, bounded memory, and configurable concurrency. Cooperative cancellation lets pause/resume preserve progress without corrupting files. A shared token bucket enforces the aggregate limit rather than per-transfer limits.

**Alternatives considered**:
- One OS thread per transfer: does not scale and complicates cancellation.
- Unbounded tasks: can exhaust memory and connections.
- Per-transfer bandwidth limits: explicitly out of scope.

## Decision 3: Metadata and transfer-state persistence

**Decision**: Extend the shared `favorites.json` store from `003` to hold connection profiles, server favorites, directory favorites, preferences, and tab state. Store the transfer queue and capped history in `transfers.json`. Use schema versions, ordered migrations, atomic replace with backup, restrictive permissions, and periodic checkpoints for active progress.

**Rationale**: The dataset is modest, the stores are non-secret, and JSON keeps the implementation inspectable and dependency-light. Separate files reduce write contention between slow-changing settings and frequently updated transfer state. Credentials remain in the OS store. A capped history bounds file size.

**Alternatives considered**:
- SQLite for all state: robust transactions, but a heavier dependency for this scope; can replace the adapter later behind the repository trait.
- Writing every progress chunk: excessive I/O; checkpoints are sufficient for recovery.
- One monolithic state file: couples frequent transfer writes to settings writes and increases corruption exposure.

## Decision 4: Credential storage

**Decision**: Reuse the `CredentialStore` abstraction and native `keyring` adapter defined by `003`. Secrets are zeroized, non-serializable, never returned to the UI, and never included in logs or errors. Store failures fail closed with no plaintext fallback.

**Rationale**: This is the constitution's required security model and avoids a second credential implementation. The shared abstraction also supports future CLI/MCP reuse.

**Alternatives considered**:
- A separate desktop credential implementation: duplicates security-critical code.
- A custom encrypted vault: explicitly out of scope and forbidden without approval.
- Browser storage: not a native credential store.

## Decision 5: Protocol abstraction and capabilities

**Decision**: Define a `RemoteFs` trait in `thriftp-core` with connection, streaming directory listing, stat, create/rename/delete, open-read/open-write, resume, chmod, capability discovery, and disconnect operations. FTP/FTPS/SFTP implement it and return a `ProtocolCapabilities` value describing resume, permissions, ownership, hidden files, symlinks, and server-side behavior.

**Rationale**: The constitution requires protocols to be isolated behind stable interfaces and the core to remain protocol-agnostic. Capability discovery lets the UI degrade gracefully when a server lacks an operation.

**Alternatives considered**:
- Per-protocol UI branches: violates isolation and multiplies test paths.
- A lowest-common-denominator interface: hides supported features and weakens UX.
- Separate transfer engines per protocol: duplicates queue, retry, and resume logic.

## Decision 6: Directory enumeration and large-listing performance

**Decision**: Stream directory entries from each protocol adapter in batches. The local adapter reads directories incrementally; FTP parses listing responses incrementally; SFTP reads directory handles in batches. The UI virtualizes the list and renders first entries before completion. Local and remote targets are measured separately.

**Rationale**: The specification distinguishes first-entry latency from complete-listing latency and forbids blocking the UI. Streaming plus virtualization keeps memory bounded and the interface responsive.

**Alternatives considered**:
- Load the complete listing before rendering: fails the first-entry requirement.
- One unbounded UI list: memory and rendering cost grows with directory size.
- Identical local/remote targets: unrealistic because remote latency is server and network dependent.

## Decision 7: Partial files, pause, cancel, and resume safety

**Decision**: Retain local partial data in an identifiable incomplete file associated with the transfer, and keep resume metadata (source size, source modification time, destination offset, protocol identity). Pause preserves progress and stops at a chunk boundary; cancel and failure preserve the last confirmed progress. Before resume, verify source size/modification time and destination offset; if either changed, refuse a silent resume and offer restart or cancel. Clean up remote partial files when the protocol supports deletion; otherwise report them.

**Rationale**: This is the specification's explicit behavior and avoids silently appending to stale data. Size and modification time are available across supported protocols; checksums are not uniformly available and can be expensive.

**Alternatives considered**:
- Always restart from zero: wastes bandwidth and violates resumability.
- Resume without verification: risks corrupt files.
- Full-file checksums: expensive and unavailable consistently.
- Delete partial files on every cancel: prevents resume.

## Decision 8: Queue lifecycle and history retention

**Decision**: Persist the queue on state transitions and checkpoint active progress every 5 seconds or 1 MB, whichever comes first. On restart, active transfers become interrupted, pending transfers remain pending and do not auto-start, and history is retained for the 1,000 most recent transfers with a user-clear action. Normal quit requires the user to wait or cancel.

**Rationale**: The specification requires queue/history persistence, interrupted representation, explicit user control, and a bounded history. Checkpoints limit lost progress without excessive writes.

**Alternatives considered**:
- Auto-resume all active transfers on launch: violates explicit user control and can surprise users.
- Unbounded history: grows without limit and slows startup.
- Persist every chunk: excessive I/O.
- Discard history on quit: violates persistence.

## Decision 9: Conflict policy and symbolic links

**Decision**: Default to prompting per conflict with overwrite, skip, rename, and apply-to-all remaining. Apply the chosen policy only to the current operation unless the user selects apply-to-all. Never follow symbolic links; preserve links when the protocol and destination support it, otherwise skip with a visible report. The same policy is shared by desktop, CLI, and MCP through the core.

**Rationale**: This matches the specification and avoids silent data loss or escaping the selected tree. A shared core policy keeps interfaces consistent.

**Alternatives considered**:
- Silent overwrite: risks data loss.
- Follow symlinks: can escape the selected tree or loop.
- Skip all links silently: hides omitted data.

## Decision 10: File previews

**Decision**: Request native OS preview through a Tauri command where available (for example Quick Look on macOS). Provide an internal fallback for text and images with size limits when native preview is unavailable. Previewing a remote file must not download the entire file without explicit user consent; large or unsupported files show metadata or a bounded preview.

**Rationale**: The specification requires native integration with a cross-platform fallback and bounded memory. Previews must not block browsing or trigger unbounded downloads.

**Alternatives considered**:
- Webview-only preview: loses native integration.
- Always download remote files for preview: violates memory/bandwidth expectations.
- No fallback: breaks platforms without the native capability.

## Decision 11: Local deletion and trash

**Decision**: Use the `trash` crate to move local items to the operating-system trash where available. Remote deletion remains permanent after explicit confirmation and a clear warning. Deletion of non-empty directories states the scope and irreversibility.

**Rationale**: The specification requires OS trash for local deletes and a permanent-delete warning for remote deletes. A dedicated crate avoids custom recycle-bin logic.

**Alternatives considered**:
- Permanent local delete: violates the specification.
- Custom trash directory: duplicates OS behavior and platform rules.
- Silent remote delete: unsafe.

## Decision 12: TLS and SSH host-key verification

**Decision**: Use `rustls` with native root certificates for FTPS; verify SFTP host keys against known-hosts data. Verification is on by default. An override requires explicit user action, is visible for the session, and is recorded. Plain FTP is clearly marked as unencrypted.

**Rationale**: Required by the constitution and specification. No silent bypass is permitted.

**Alternatives considered**:
- Accept-all verifier: forbidden.
- Native TLS with platform differences: less consistent.
- Ignoring host keys: unsafe.

## Decision 13: Containerized protocol integration testing

**Decision**: Use one Docker Compose harness under `docker/protocol/` for FTP, explicit FTPS, implicit FTPS, and SFTP, and run the same Rust integration test suite against it locally on macOS through Docker Desktop and in Linux CI. A single `scripts/test-integration.sh` starts the containers, waits for health, exports endpoint environment variables, runs the ignored/feature-gated protocol integration tests, and tears the environment down. Windows developers with Docker Desktop may use the same harness.

**Details**:
- Services: plain FTP, explicit FTPS, implicit FTPS, and SFTP, with TLS certificates and SSH host keys generated as test fixtures or by a one-shot init service.
- Images are version-pinned (preferably by digest) to keep local and CI environments reproducible.
- Health checks and a bounded wait loop prevent race conditions; the harness uses a bridge network and explicitly mapped ports.
- Test endpoints are supplied through environment variables so the same test code runs locally and in CI without branching on the host OS.
- Unit tests and platform-specific tests do not require a container runtime. `cargo test` runs the Docker-free suite.
- CI exposes separate `unit`, `protocol-integration`, `platform`, `frontend`, and `build` stages. A missing container runtime skips only `protocol-integration` with a clear message.
- Docker is a development/CI tool only. It is not a Rust dependency, is not called by the application at runtime, and is not required to build, package, or run Thriftp.

**Rationale**: A shared Compose harness gives developers on macOS Docker Desktop and Linux CI the same multi-service topology, certificates, credentials, and endpoint contract. That minimizes local/CI drift while keeping the fast unit-test loop independent of Docker. Compose is a better fit than per-test container orchestration here because the FTP/FTPS/SFTP servers need fixed multi-service configuration and shared certificate/host-key fixtures.

**Alternatives considered**:
- Testcontainers for Rust: dynamic ports and per-test lifecycle are attractive, but multi-service certificate/key setup and container reuse add complexity; it can be revisited later behind the same test harness interface.
- Raw `docker run` commands: more shell code and easier drift between local and CI.
- Linux-only container tests: rejected by the constitution and by the primary macOS development environment.
- Local native protocol servers: platform configuration drift and harder reproducibility.
- Mocks only: misses real protocol interoperability.
- Docker as an application runtime dependency: rejected; it would violate the product's offline, self-contained desktop model.

## Decision 14: Observability and redaction

**Decision**: Use `tracing` spans for connection, browsing, transfer, queue, and credential operations with contextual IDs, state transitions, byte counts, and durations. Redact secrets and raw protocol errors. Logs are local and never transmitted.

**Rationale**: The constitution requires structured tracing with secret redaction. Contextual fields make failures diagnosable without exposing credentials.

**Alternatives considered**:
- Unstructured logging: harder to correlate and easier to leak.
- Logging raw server responses: may contain sensitive data.
- No tracing: fails the constitution.

## Decision 15: Accessibility target

**Decision**: Target WCAG 2.2 AA where applicable to the desktop UI, supplemented by native platform accessibility APIs. Provide keyboard navigation, visible focus, screen-reader labels, accessible names for protocol and transfer states, and reduced-motion support.

**Rationale**: The specification leaves the formal target open; WCAG 2.2 AA is a widely recognized baseline and is testable. Native APIs cover desktop-specific controls.

**Alternatives considered**:
- No formal target: leaves acceptance criteria ambiguous.
- WCAG AAA: unnecessarily strict for the initial release.
- Platform-only checks: inconsistent across platforms.

## Decision 16: Transfer history retention

**Decision**: Retain the 1,000 most recent transfer-history records. Provide a user action to clear history. Keep the active queue separate and fully persisted.

**Rationale**: Resolves the specification's open retention decision with a bounded, predictable default that supports troubleshooting without unbounded growth.

**Alternatives considered**:
- Unlimited retention: unbounded file growth.
- Session-only: loses recovery and audit value.
- Time-based retention: less predictable for occasional users.

## Decision 17: UI state and virtualization

**Decision**: Use Svelte stores and a typed IPC client. Virtualize large directory lists. Use native dialogs and drag-and-drop. Do not add an external state-management library.

**Rationale**: The state is local to the desktop UI and the feature scope is bounded. Virtualization is required for large directories; an external store library adds no necessary value.

**Alternatives considered**:
- External store library: unnecessary dependency.
- Unvirtualized lists: poor performance with 10,000 entries.
- Component-local state only: hard to keep tabs, queue, and connection state consistent.

## Decision 18: Benchmarks

**Decision**: Use `criterion` benchmarks for protocol parsing, directory-enumeration batching, and the transfer chunk loop. Run them in CI with a regression threshold and keep results as artifacts.

**Rationale**: The constitution requires benchmark suites for hot-path networking and parsing code. Criterion is the standard Rust benchmarking tool and produces stable comparisons.

**Alternatives considered**:
- No benchmarks: fails the constitution.
- Ad hoc timing: noisy and non-comparable.
- Benchmarks only on one platform: misses platform-specific regressions.

## Decision 19: Error taxonomy

**Decision**: Use typed errors with stable codes for validation, authentication, credential-store, certificate, host-key, network, timeout, permission, conflict, cancellation, unsupported capability, and internal failures. Map them to actionable user messages at the Tauri boundary; never include secrets or raw backend diagnostics.

**Rationale**: Stable codes make tests objective and let the UI choose recovery actions. Redaction is required by the constitution.

**Alternatives considered**:
- Free-form strings: hard to test and easy to leak.
- Returning raw protocol errors: may expose sensitive details.

## Decision 20: File-name handling and path rules

**Decision**: Use `OsString`/`PathBuf` for local paths so non-UTF-8 names are preserved. Use UTF-8 strings for remote paths and reject or clearly report names that cannot be represented safely. Remote paths use POSIX separators and protocol-normalized paths; local paths follow OS rules. Never silently replace invalid characters.

**Rationale**: The specification requires clear handling of invalid or differently encoded names. Preserving local OS names avoids data loss; rejecting unrepresentable remote names avoids silent corruption.

**Alternatives considered**:
- Lossy UTF-8 conversion: can rename or collide files silently.
- Treating remote and local paths identically: breaks platform and protocol semantics.

## Decision 21: CLI/MCP reuse boundary

**Decision**: Keep connection, browsing, transfer, queue, favorites, credential, and preference services in `thriftp-core` with interfaces independent of Tauri. The CLI/MCP feature (`002`) must call those services and must not implement protocol logic in parallel. No CLI/MCP management surface is added by this feature.

**Rationale**: The constitution requires a shared core and CLI/MCP security parity. This plan preserves that boundary without expanding the desktop feature scope.

**Alternatives considered**:
- Business logic in Tauri commands: blocks CLI/MCP reuse and violates core independence.
- Separate protocol implementations for automation: duplicates security-critical logic.

## Resolved Unknowns Summary

| Unknown or open decision | Resolution |
|--------------------------|------------|
| Protocol libraries | `suppaftp` 12.x for FTP/FTPS, `russh`/`russh-sftp` for SFTP |
| Transfer concurrency and control | Tokio tasks, semaphore, priority queue, token bucket, cooperative cancellation |
| Metadata persistence | Shared JSON metadata store plus `transfers.json` |
| Transfer history retention | 1,000 most recent records, user-clear action |
| Credential storage | Native `keyring` adapter shared with `003`; fail closed |
| Protocol abstraction | `RemoteFs` trait with capability discovery |
| Large-directory behavior | Streaming batches plus virtualized UI; separate local/remote targets |
| Resume safety | Verify source size/mod time and destination offset; no silent resume |
| Conflict and symlink policy | Prompt with apply-to-all; never follow links |
| Previews | Native OS preview plus bounded internal fallback |
| Local delete | OS trash through `trash`; remote deletion permanent with warning |
| TLS and host keys | `rustls` native roots and known-hosts; explicit override only |
| Testing | Shared Docker Compose harness on macOS Docker Desktop and Linux CI; Docker-free unit and platform tests; separate CI stages |
| Observability | `tracing` with contextual fields and redaction |
| Accessibility | WCAG 2.2 AA where applicable plus native platform APIs |
| UI state | Svelte stores plus list virtualization |
| Benchmarks | `criterion` for parsing, enumeration, and transfer loops |
| Error handling | Typed stable codes with actionable, redacted messages |
| File names | `OsString` locally; UTF-8 remote paths with explicit rejection |
| CLI/MCP reuse | Shared core services and traits; no parallel protocol logic |
