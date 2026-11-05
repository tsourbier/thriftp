# Thriftp Constitution

## Core Principles

### I. Product Vision

Thriftp is a free, open-source, cross-platform desktop file transfer client with a native
macOS experience. The application MUST prioritize simplicity, reliability, performance, and a
polished desktop experience. The initial supported platforms are macOS, Linux, and Windows.
The initial supported protocols are FTP, FTPS, and SFTP. The architecture MUST allow future
protocol integrations without requiring changes to the application core. Thriftp MUST provide
first-class command-line (CLI) and Model Context Protocol (MCP) interfaces that expose the
same core capabilities as the desktop UI. Every feature decision MUST be traceable to this
vision; features that add complexity without improving file transfer simplicity, reliability,
or performance MUST be rejected or deferred.

**Rationale:** A narrow, explicit vision keeps a small open-source project focused and
prevents scope creep that would compromise reliability or polish.

### II. Architecture

Rust is the primary language for all business logic and networking operations. Tauri 2
provides the desktop application runtime. Svelte and TypeScript provide the user interface.
The application core MUST remain independent of Tauri-specific APIs: it MUST build and pass
its test suite without Tauri and MUST NOT import Tauri crates or types. Protocol
implementations MUST be isolated behind stable, documented interfaces so that new protocols
can be added without modifying the core. Asynchronous operations MUST NOT block the UI
thread; UI handlers MUST NOT perform synchronous network, filesystem, or CPU-heavy work.
Every dependency MUST be justified by a clear need, actively maintained, and evaluated for
size, license, and security impact before adoption. Code MUST be organized as small,
cohesive modules rather than monolithic components. The core MUST expose stable, documented
operations that back the desktop UI, the CLI, and the MCP server; those interfaces MUST be
thin adapters over the core and MUST NOT contain protocol logic or depend on Tauri.

File data transfers MUST stream through fixed-size buffers or zero-copy primitives; memory
consumption during transfers MUST remain constant (`O(1)`) regardless of file size. Hot-path
networking and parsing code MUST include benchmark suites to detect performance regressions.
All asynchronous operations and networking layers MUST emit structured events via `tracing`
with contextual tags while strictly respecting secret redaction rules.

**Rationale:** Decoupling the core from Tauri and concrete protocols preserves portability,
testability, and scalability. Streamed transfers and structured tracing ensure the app handles
production workloads reliably without memory spikes or opaque failures.

### III. Security

Credentials MUST NOT be stored in plaintext. Persistent credentials MUST use a native
operating system credential store: macOS Keychain, Windows Credential Manager, or the
Secret Service API on Linux. Passwords, private keys, tokens, and other secrets MUST NOT
appear in logs, telemetry, crash reports, error messages, or source control. TLS
certificates MUST be validated by default. SSH host keys MUST be verified, and unexpected
host-key changes MUST be detected and surfaced to the user. Certificate and host-key
verification MUST NOT be bypassed silently; any override MUST require explicit user action
and MUST remain visible for the duration of the session. Plain FTP MUST clearly communicate
its lack of transport encryption before and during use. The codebase MUST apply the
principle of least privilege. Dependencies MUST be reviewed for known vulnerabilities before
adoption and on an ongoing basis.

Continuous integration MUST generate a Software Bill of Materials (SBOM) for official
releases. Dependencies and CI actions MUST be pinned to verified versions or hashes.
Lockfiles MUST be committed and strictly enforced during CI builds. The CLI and MCP
interfaces MUST inherit every security rule in this section and MUST NOT provide a weaker
path for credentials, verification, or secret handling.

**Rationale:** File transfer clients handle sensitive user credentials; strong transport
validation, credential isolation, and supply-chain hardening are required to protect users
against data theft and build tampering.

### IV. User Experience

The application MUST provide an intuitive, responsive dual-pane file manager. Directory
navigation MUST remain instantaneous even with large file trees. The application MUST
support native drag-and-drop and OS-level file previews such as Quick Look. It MUST provide
a robust transfer queue with pause, resume, rate-limiting, and prioritization controls.
File collisions and conflicts MUST be resolved gracefully and clearly, without intrusive
blocking popups. The application MUST support multiple connections, tabs, and saved site
bookmarks. Interactions MUST follow native desktop conventions and provide comprehensive
keyboard shortcuts. The project MUST NOT copy proprietary assets, icons, or distinctive
interface elements from other products.

The UI MUST automatically adapt to native system themes (light and dark mode) and
platform-native window controls. When platform-specific OS features such as Quick Look are
unavailable on a supported OS, the application MUST fall back gracefully to internal preview
handlers without disrupting UI flow.

**Rationale:** Transfers are high-frequency, latency-sensitive operations; a desktop client
earns trust through predictable navigation, clear status, native ergonomics, and rock-solid
platform compatibility.

### V. Quality

Every significant feature MUST include automated tests that cover its core behavior, and
those tests MUST pass before the change is merged. Protocol implementations MUST have
independent integration tests that exercise authentication, directory listing, upload,
download, rename, and deletion against a controlled server or recorded fixtures. Critical
networking and security behaviors MUST be thoroughly tested. The CLI and MCP interfaces MUST
have automated contract tests covering their documented commands, tools, outputs, and
authorization rules. Continuous integration MUST validate every supported operating system.
Clippy warnings MUST be resolved before merge.
Code MUST be formatted with `rustfmt`. Public APIs MUST be fully documented. Errors MUST be
explicit, actionable, and properly propagated; production code MUST NOT panic on recoverable
errors. A change that touches data-transfer paths MUST be evaluated for throughput and
UI-responsiveness regressions.

**Rationale:** Automated, cross-platform verification is the only reliable way to keep a
Rust and TypeScript codebase correct across three operating systems and multiple protocols.

### VI. Open-Source

The project is public and community-oriented. It MUST use the MIT license unless a technical
or legal constraint requires otherwise. It MUST NOT require a cloud service, user account, or
subscription to function. Telemetry MUST NOT be mandatory; any telemetry MUST be opt-in and
documented. Installation, configuration, and contribution workflows MUST be documented
clearly enough for a new contributor to build and run the project. The CLI and MCP interfaces
MUST be documented with equivalent clarity. Third-party dependencies and their licenses MUST
remain transparent and reviewable.

**Rationale:** Friction-free, transparent open source is a core product commitment, not a
distribution detail; users and contributors must be able to inspect, build, and trust the
application.

### VII. AI-Assisted Development

Specifications are the single source of truth for expected behavior. Architecture decisions
MUST be documented, and agents MUST NOT introduce major architectural changes without
explicit justification and maintainer approval. Every implementation task MUST have clear
acceptance criteria. Agents MUST run the relevant unit and integration tests before
considering a task complete. The project MUST apply YAGNI: unnecessary abstractions,
speculative functionality, and code bloat MUST be avoided. Documentation and specifications
MUST be kept synchronized with the implementation.

**Rationale:** AI-assisted contribution accelerates delivery but amplifies drift; explicit
specifications, acceptance criteria, and automated checks keep agent output aligned with the
constitution.

### VIII. Automation & Integration

Thriftp MUST ship two first-class automation interfaces in addition to the desktop UI:

- **Command-line interface (CLI):** MUST be non-interactive and scriptable; MUST support
  connection management, browsing, upload, download, directory operations, and transfer-queue
  control; MUST provide machine-readable output and stable exit codes; MUST use the same
  credential store and verification rules as the desktop UI; and MUST NOT print secrets.
- **Model Context Protocol (MCP) server:** MUST be opt-in and disabled by default; MUST bind
  to local-only transport, such as standard input/output or loopback, by default; MUST require
  explicit user authorization before exposing a connection, directory, or operation to an MCP
  client; MUST require per-operation confirmation or policy for destructive actions; MUST
  record an audit trail of MCP-initiated operations; and MUST NOT expose credentials or
  secrets.
- **Shared core:** Both interfaces MUST reuse the core operations and MUST NOT implement
  protocol logic in parallel. They MUST be independently testable, documented, and versioned.
- **Security parity:** Both interfaces MUST enforce the same certificate, host-key,
  credential, and least-privilege rules as the desktop UI and MUST NOT introduce a bypass.

**Rationale:** A modern file transfer client must be automatable by scripts and AI agents
without creating a second, weaker implementation; reusing the core keeps behavior consistent
and security reviewable.

## Supported Platforms & Protocols

Thriftp MUST build, run, and pass its automated tests on macOS, Linux, and Windows.

The initial protocol set is FTP, FTPS, and SFTP. Adding a protocol MUST require only: an
implementation of the documented protocol interface, contract and integration tests, a
security review of its authentication and transport behavior, and user documentation. It
MUST NOT require changes to application core modules.

Plain FTP is supported for compatibility but MUST be presented as unencrypted transport.
FTPS and SFTP MUST be recommended wherever the server supports them.

## Development Workflow & Quality Gates

Significant work MUST begin with a written specification and an implementation plan before
code is written. Every change MUST be reviewed for compliance with this constitution.
Continuous integration MUST run formatting and lint checks, unit and integration tests,
CLI and MCP contract tests, dependency vulnerability checks, SBOM generation, and builds for
all supported platforms. Changes that touch authentication, cryptography, credential storage,
protocol parsing, or automation interfaces MUST receive an explicit security review. Added complexity MUST be justified in the change
description. A change that cannot satisfy a principle MUST either be revised or accompanied
by a constitution amendment; it MUST NOT be merged with an undocumented exception.

AI agents MUST follow the same gates as human contributors, including acceptance criteria,
tests, documented architecture decisions, and documentation updates.

## Governance

This constitution supersedes all other development practices. When guidance in other
documents conflicts with it, this document takes precedence.

Amendments MUST be proposed as a pull request that modifies this file, states the rationale,
and includes a Sync Impact Report. An amendment becomes effective once approved by a project
maintainer. Amendments MUST include a migration note when existing code or workflows must
change to comply.

Versioning follows semantic versioning:

- MAJOR: a backward-incompatible governance change, including removal or redefinition of a
  principle.
- MINOR: a new principle or section, or materially expanded guidance.
- PATCH: clarifications, wording improvements, or non-semantic refinements.

Every pull request and review MUST verify compliance with this constitution, including
changes produced by AI agents. Reviewers MUST block changes that violate a principle without
an approved amendment. The constitution MUST be reviewed at least annually and whenever the
supported platforms, protocols, or technology stack change materially. Runtime development
guidance lives in the repository's agent and contributor guidance files.

**Version**: 1.2.0 | **Ratified**: 2026-11-05 | **Last Amended**: 2026-11-05
