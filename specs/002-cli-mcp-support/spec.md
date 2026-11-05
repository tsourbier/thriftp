# Feature Specification: CLI and MCP Automation Interfaces

**Feature Branch**: `002-cli-mcp-support`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Align with constitution.md and add mentions to CLI and MCP support: first-class command-line and Model Context Protocol interfaces that expose the same core file transfer capabilities as the desktop application."

## Clarifications

### Session 2026-11-05

- Q: How should the MCP server run relative to the desktop application? → A: Support both a standalone headless server and an embedded in-app mode with identical authorization, audit, and security behavior.
- Q: Should MCP authorization grants persist across restarts, or end with the current session? → A: Persist until revoked, with an optional expiry.
- Q: What should happen by default when an MCP client requests a destructive operation? → A: Require confirmation for each destructive operation, with an opt-in pre-approval policy per grant.
- Q: When a stored credential is unavailable, how should the CLI accept a secret? → A: Read it from standard input only; never from arguments or environment variables.
- Q: How long should MCP audit records be retained? → A: 90 days by default, configurable by the user, with clear notice before deletion.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automate file transfers from the command line (Priority: P1)

A developer or system administrator uses a scriptable command-line interface to select a saved connection, browse a remote directory, upload or download files and directories, and inspect the transfer queue without opening the desktop window.

**Why this priority**: Scriptable transfers are the primary value of a first-class CLI and enable repeatable operations, CI jobs, and server administration.

**Independent Test**: Using only documented commands, connect to a test server, upload a file, download a directory recursively, query the queue, and verify exit codes and machine-readable results for both success and failure.

**Acceptance Scenarios**:

1. **Given** a saved connection profile, **when** the user runs a CLI transfer command, **then** the transfer completes and the command returns a success exit code with a machine-readable result.
2. **Given** a directory selected for upload or download, **when** the CLI transfer runs, **then** the directory structure is preserved on the destination.
3. **Given** invalid credentials or an unreachable host, **when** the command runs, **then** it returns a distinct non-zero exit code, an actionable error, and no secret material.
4. **Given** machine-readable output is requested, **when** the command succeeds or fails, **then** the result is parseable and contains stable fields for status, target, and error information.
5. **Given** an active transfer, **when** the user queries or cancels it through the CLI, **then** the current status is reported and cancellation takes effect.
6. **Given** no usable credential in a non-interactive run, **when** the command needs authentication, **then** it fails promptly with a clear message instead of waiting for input.

---

### User Story 2 - Let an AI agent operate on explicitly authorized resources (Priority: P1)

A user enables the MCP server, authorizes a specific client to access selected connection profiles and directories, and the AI agent lists files, uploads, downloads, and queries transfer status within that scope.

**Why this priority**: MCP support is the modern differentiator that makes Thriftp usable by AI agents without weakening its security posture.

**Independent Test**: Enable MCP, connect a test client, grant access to one profile and one directory, perform browse and transfer operations, and verify that requests outside the grant are denied.

**Acceptance Scenarios**:

1. **Given** MCP is disabled, **when** a client attempts to connect, **then** no server is reachable and the user is shown how to enable it.
2. **Given** MCP is enabled and a client is authorized for one profile and directory, **when** the agent lists files and transfers within that scope, **then** the operations succeed.
3. **Given** an authorized scope, **when** the agent requests an unauthorized connection or path, **then** the request is denied with a clear error and no data is returned.
4. **Given** a destructive operation, **when** the agent requests it without confirmation or an applicable policy, **then** the operation is blocked.
5. **Given** a pre-approved destructive-action policy, **when** the agent performs a covered operation, **then** it proceeds and the policy use is audited.
5. **Given** any MCP operation, **when** it completes, fails, or is denied, **then** an audit record is created.
6. **Given** the standalone and embedded MCP modes, **when** equivalent operations are performed in each, **then** authorization, audit, and security behavior are identical.

---

### User Story 3 - Control, audit, and revoke MCP access (Priority: P2)

The user reviews connected MCP clients and their scopes, revokes access immediately, disables MCP entirely, and inspects an audit trail of every MCP-initiated operation.

**Why this priority**: Exposing file operations to AI agents requires visible, revocable control and accountability.

**Independent Test**: Grant access to a client, inspect the active grants, revoke them, verify subsequent operations are denied, and confirm the audit trail records both the allowed and denied attempts.

**Acceptance Scenarios**:

1. **Given** an active MCP session, **when** the user opens the automation controls, **then** the connected clients and their granted profiles, directories, and operations are visible.
2. **Given** an active grant, **when** the user revokes it, **then** the client cannot perform another operation under that grant.
3. **Given** any MCP operation, **when** the user reviews the audit trail, **then** it shows the timestamp, client, operation, target, and result without exposing secrets.
4. **Given** MCP is enabled, **when** the user disables it, **then** all clients are disconnected and no new MCP sessions can start.
5. **Given** a grant with an expiry, **when** the expiry passes, **then** the client's next operation is denied and the expiry is recorded.

---

### User Story 4 - Discover and integrate with documented interfaces (Priority: P2)

A user or integrator reads the documentation, runs CLI help, lists the available MCP tools and their input and output shapes, and confirms the interface version before building automation.

**Why this priority**: Adoption depends on discoverable, stable, and versioned contracts rather than tribal knowledge.

**Independent Test**: From a clean environment, follow the documentation to run CLI help, list MCP tools, and complete a minimal example for each interface without reading source code.

**Acceptance Scenarios**:

1. **Given** the CLI is installed, **when** the user requests help, **then** commands, options, output modes, and exit codes are listed.
2. **Given** the MCP server is enabled, **when** a client requests capabilities, **then** the available tools, their purposes, and their input and output shapes are returned.
3. **Given** the published documentation, **when** a new user follows the examples, **then** a minimal transfer through each interface succeeds.
4. **Given** an incompatible client version, **when** it connects, **then** it receives a clear compatibility error rather than undefined behavior.

---

### User Story 5 - Use stable machine-readable results in automation (Priority: P3)

A script author parses command output and exit codes to make decisions, handle failures, and report results in a CI pipeline.

**Why this priority**: Stable machine-readable contracts turn the CLI from a convenience into a dependable automation surface.

**Independent Test**: Run successful and failing commands, parse the structured output, and verify that documented fields and exit codes remain consistent.

**Acceptance Scenarios**:

1. **Given** a successful command, **when** machine-readable output is requested, **then** the result contains the documented fields and a success status.
2. **Given** a failed command, **when** machine-readable output is requested, **then** the result contains a stable error code, an actionable message, and no secrets.
3. **Given** human-readable diagnostics, **when** machine-readable output is requested, **then** the structured result remains separable from progress and diagnostic text.

---

### Edge Cases

- The CLI is invoked with an unknown command, a missing required option, or conflicting options; it must print usage guidance and return a non-zero exit code.
- A user supplies a secret through a command-line argument or environment variable; the CLI must reject it with guidance and must never echo the value.
- A downstream consumer closes the output pipe early; the CLI must terminate cleanly without crashing or printing secrets.
- The desktop application, CLI, and MCP server act on the same profile or directory concurrently; conflicts must be reported and must not corrupt files.
- An MCP client disconnects during a transfer; the transfer must follow the user's policy and the outcome must be recorded.
- An MCP grant is revoked or expires while a session is active; the next operation must be denied.
- A remote client attempts to reach MCP while it is configured for local-only access; the attempt must be denied and recorded.
- An MCP client requests an unknown or unsupported tool; it must receive a clear error without affecting other sessions.
- The audit trail cannot be written; destructive operations must fail closed rather than proceed unrecorded.
- The audit retention period elapses or the configured storage limit is reached; deletion must follow the configured policy and the user must receive clear notice.
- A directory listing is very large; output must remain usable without exhausting memory or blocking other work.
- A CLI or MCP client uses an incompatible interface version; it must receive a clear compatibility error.
- Standalone and embedded MCP servers run at the same time; grants, state, and audit records must remain consistent, and conflicting sessions must be reconciled without granting unintended access.

## Requirements *(mandatory)*

### Functional Requirements

**Shared Automation Core**

- **FR-001**: Thriftp MUST provide a first-class command-line interface and a first-class MCP server in addition to the desktop UI.
- **FR-002**: Both interfaces MUST reuse the same core operations and MUST NOT implement protocol logic in parallel.
- **FR-003**: Equivalent operations through the desktop UI, CLI, and MCP server MUST produce consistent results.
- **FR-004**: Both interfaces MUST enforce the same credential, certificate, host-key, and least-privilege rules as the desktop UI.
- **FR-005**: Both interfaces MUST be independently testable and MUST be covered by automated contract tests.
- **FR-006**: Both interfaces MUST report their version and supported capabilities.
- **FR-007**: Unknown or unsupported requests MUST receive an actionable error without affecting other operations or sessions.
- **FR-008**: Both interfaces MUST share connection profiles, favorites, and transfer state with the desktop application.

**Command-Line Interface**

- **FR-009**: The CLI MUST be non-interactive and scriptable for use in shells, scripts, and CI pipelines.
- **FR-010**: The CLI MUST support listing, inspecting, creating, updating, and deleting connection profiles using secure credential input.
- **FR-011**: The CLI MUST support browsing remote directories and displaying file metadata.
- **FR-012**: The CLI MUST support uploading and downloading individual files, multiple selected files, and directories recursively.
- **FR-013**: The CLI MUST support querying transfer status, cancelling transfers, and configuring concurrency.
- **FR-014**: The CLI MUST provide machine-readable output with stable fields for status, target, progress, and errors.
- **FR-015**: The CLI MUST provide human-readable output and built-in help for every command.
- **FR-016**: The CLI MUST use stable, documented exit codes that distinguish success, usage errors, authentication failures, network failures, permission failures, conflicts, and cancellations.
- **FR-017**: The CLI MUST NOT accept secrets as command-line arguments or environment variables; when no stored credential exists, it MUST read the secret from standard input and MUST NOT print it in output, logs, or errors.
- **FR-018**: When a required credential is unavailable from the credential store or standard input in a non-interactive run, the CLI MUST fail promptly with a clear message rather than wait for input.
- **FR-019**: The CLI MUST apply the same certificate and host-key verification rules as the desktop UI and MUST NOT offer a silent bypass.
- **FR-020**: The CLI MUST support the same overwrite, skip, rename, and apply-to-all conflict policies as the desktop UI; without an explicit policy, a conflict MUST fail instead of overwriting silently.
- **FR-021**: Recursive CLI transfers MUST preserve symbolic links when supported, skip them with a visible report otherwise, and MUST NOT follow them.

**MCP Server**

- **FR-022**: The MCP server MUST be opt-in and disabled by default, and MUST be available both as a standalone headless process and as an embedded mode within the desktop application.
- **FR-023**: The MCP server MUST use a local-only communication channel and MUST NOT accept remote network clients in the initial release.
- **FR-024**: The user MUST explicitly authorize each MCP client for specific connection profiles, directories, and operations before any access is granted.
- **FR-025**: MCP grants MUST follow least privilege, be scoped, persist across restarts until revoked or expired, support an optional expiry, and be revocable at any time.
- **FR-026**: The MCP server MUST expose tools for listing connections, browsing directories, inspecting files, uploading, downloading, creating, renaming, deleting, querying transfer status, and cancelling transfers.
- **FR-027**: Destructive MCP operations MUST require explicit confirmation for each operation by default; users MAY opt in to a scoped pre-approval policy within a grant, and every use of that policy MUST be audited.
- **FR-028**: Every MCP operation, including denied and failed attempts, MUST produce an audit record.
- **FR-029**: Audit records MUST include the timestamp, client identity, interface, operation, target, and result, and MUST NOT contain secrets; they MUST be retained for 90 days by default, with a user-configurable retention period and clear notice before deletion.
- **FR-030**: Audit records MUST be protected from modification or deletion by MCP clients and MUST NOT be removed before the configured retention period without explicit user action.
- **FR-031**: If the audit trail cannot be written, destructive MCP operations MUST fail closed.
- **FR-032**: MCP responses, logs, and audit records MUST NOT expose credentials, private keys, tokens, or other secrets.
- **FR-033**: Revoking a grant, reaching its expiry, or disabling MCP MUST take effect before the client's next operation.
- **FR-034**: The user MUST be able to view active MCP clients and their grants and MUST be able to disconnect any client.
- **FR-035**: MCP errors MUST be actionable and MUST distinguish authorization, validation, conflict, network, and internal failures.
- **FR-036**: The MCP server MUST publish its available tools, input and output shapes, and interface version to authorized clients.

**Security and Privacy**

- **FR-037**: Both interfaces MUST use the native operating system credential store and MUST NOT persist credentials in plaintext.
- **FR-038**: Both interfaces MUST NOT weaken or bypass certificate or host-key verification.
- **FR-039**: Both interfaces MUST apply the principle of least privilege to the operations they expose.
- **FR-040**: Enabling MCP MUST NOT grant access to any connection, directory, or operation by default.
- **FR-041**: The application MUST NOT run an MCP server without a visible, user-controlled enabled state.

**Documentation and Compatibility**

- **FR-042**: The project MUST document installation, configuration, authentication, commands, MCP tools, examples, exit codes, and audit behavior for both interfaces.
- **FR-043**: Both interfaces MUST be versioned and MUST document their compatibility policy.
- **FR-044**: Breaking changes to either interface MUST follow the project's versioning and amendment process.

### Key Entities *(include if feature involves data)*

- **Automation Interface**: A first-class non-UI surface, either the CLI or the MCP server, with a version, capabilities, and documentation.
- **CLI Command**: A documented invocation with arguments, options, output modes, exit codes, and a structured result schema.
- **CLI Invocation Result**: The outcome of a command, including exit code, structured result, diagnostics, duration, and affected targets.
- **MCP Client**: A connected AI agent or tool with an identity, session, and set of grants.
- **MCP Tool**: A documented operation exposed to authorized clients, with a purpose, input shape, output shape, and destructive-action flag.
- **Authorization Grant**: A user-approved scope that binds an MCP client to specific connection profiles, directories, and operations, with an optional expiration and a revoked, expired, or active state.
- **Approval Policy**: An opt-in rule within a grant that pre-approves specific destructive operations, with a defined scope, an audit requirement, and a revocation state.
- **MCP Session**: The lifetime of a client connection, including its runtime mode (standalone or embedded), grants, activity, and audit sequence.
- **Audit Event**: An immutable record of an automation operation, including timestamp, actor, interface, operation, target, result, redacted error context, and retention status.
- **Connection Profile**: An existing saved server configuration reused by both interfaces.
- **Transfer**: An existing upload or download operation whose state is shared across the desktop UI, CLI, and MCP server.

### Out of Scope for the Initial Release

- Remote MCP access over public networks; the initial release is local-only.
- A terminal user interface or interactive CLI shell.
- Plugins or third-party extensions for either interface.
- Arbitrary shell or server-administration tools exposed through MCP.
- Cloud synchronization of profiles, grants, or audit records.
- Non-file-transfer MCP tools unrelated to Thriftp's core capabilities.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can connect to a server and complete an upload and a download through the CLI in under 2 minutes using only the published documentation.
- **SC-002**: 100% of documented CLI commands return a stable exit code and machine-readable result for both success and failure.
- **SC-003**: A security review finds zero credentials or private keys in CLI output, MCP responses, logs, or audit records.
- **SC-004**: MCP is disabled by default in 100% of fresh installations and cannot be reached until the user explicitly enables it.
- **SC-005**: 100% of requests outside an MCP client's authorized scope are denied and audit-recorded.
- **SC-006**: 100% of destructive MCP operations require confirmation or a pre-approved policy before execution, with confirmation as the default.
- **SC-007**: 100% of MCP operations, including denied attempts, produce an audit record with the documented fields.
- **SC-008**: Revoking an MCP grant prevents the client's next operation in 100% of tests.
- **SC-009**: CLI and MCP contract tests pass on every supported operating system in continuous integration.
- **SC-010**: A new integrator can complete a minimal CLI example and a minimal MCP example in under 10 minutes each.
- **SC-011**: Equivalent operations through the desktop UI, CLI, and MCP server produce identical results in 100% of contract tests.
- **SC-012**: With default settings, the MCP server accepts zero remote network clients in the initial release.
- **SC-013**: When the audit trail is unavailable, 100% of destructive MCP operations are blocked.
- **SC-014**: 100% of MCP clients receive the available tool list and interface version before their first operation.
- **SC-015**: Standalone and embedded MCP modes produce identical authorization, audit, and security results in 100% of contract tests.
- **SC-016**: 100% of revoked or expired grants are denied before the client's next operation.
- **SC-017**: 100% of CLI authentication tests using standard input complete without the secret appearing in process arguments, environment, output, logs, or audit records.
- **SC-018**: 100% of audit records are retained for the configured period, and users receive notice before any deletion.

## Assumptions

- The CLI and MCP server can operate headlessly using the same core and operating system credential store as the desktop application.
- The MCP server supports both standalone headless and embedded modes; both modes share the same authorization, audit, and security rules.
- This feature implements the constitution's Automation & Integration principle, including shared core operations, security parity, explicit authorization, auditability, and contract tests.
- Initial-release MCP transport is local-only; remote access is a future capability requiring explicit configuration.
- The default machine-readable output format is structured JSON; alternative formats may be added later.
- Users explicitly authorize every MCP client and scope; grants persist across restarts until revoked or expired, and no client is trusted implicitly.
- Destructive MCP operations require per-operation confirmation by default; pre-approval policies are opt-in, scoped, and audited.
- The MCP implementation follows the Model Context Protocol and documents the supported protocol version.
- CLI and MCP operations share the same connection profiles, favorites, and transfer queue as the desktop application.
- The initial CLI covers connection management, browsing, transfers, and queue control; it does not provide an interactive shell.
- Audit records are stored locally, retained for 90 days by default, and configurable by the user; they are not transmitted to a remote service.
- Authentication secrets are read from standard input or retrieved from the operating system credential store, never from command-line arguments or environment variables.
- Technology and process architecture choices are deferred to the planning phase; this specification defines observable behavior only.
