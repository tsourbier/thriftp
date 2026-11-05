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

### Refinement Decisions (2026-11-05)

- Q: How is an MCP client identified and enrolled? → A: Each client has a stable, user-visible identity and must be enrolled through explicit user approval before receiving any grant.
- Q: Does revoking a grant end active sessions and already-authorized work? → A: Yes; revocation terminates active sessions and prevents queued or in-flight operations from continuing.
- Q: How are directory scopes enforced? → A: Every path is canonicalized and checked against the granted scope; traversal, alternate separators, encoded traversal, and symbolic-link escapes are denied for local and remote operations.
- Q: What happens when a headless MCP server cannot obtain destructive-action confirmation? → A: The operation is denied; only an explicit, scoped pre-approval policy can authorize it, and the absence of a graphical interface never implies authorization.
- Q: Do all MCP operations fail closed when their audit record cannot be written? → A: Yes; there is no read-only exception, and destructive operations never proceed without a durable audit record.
- Q: Does a CLI transfer keep running after the CLI process exits? → A: A CLI transfer command remains active until a terminal state by default; if the process exits early, the transfer becomes interrupted and resumable, not completed.
- Q: Are CLI directory operations in scope? → A: Yes; the CLI must support creating, renaming, and deleting remote and local files and directories, with explicit confirmation for destructive operations.
- Q: What are the CLI output stream conventions? → A: Machine-readable JSON goes to standard output, while progress and diagnostics go to standard error.
- Q: Are favorites available through CLI or MCP? → A: No; favorites remain a desktop UI capability in the initial release.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Automate file transfers from the command line (Priority: P1)

A developer or system administrator uses a scriptable command-line interface to select a saved connection, browse a remote directory, create or manage directories, upload or download files and directories, and inspect or control the transfer queue without opening the desktop window.

**Why this priority**: Scriptable transfers are the primary value of a first-class CLI and enable repeatable operations, CI jobs, and server administration.

**Independent Test**: Using only documented commands, connect to a test server, create a directory, upload a file, download a directory recursively, rename and delete an entry, query the queue, and verify exit codes and machine-readable results for both success and failure.

**Acceptance Scenarios**:

1. **Given** a saved connection profile, **when** the user runs a CLI transfer command, **then** the transfer completes and the command returns a success exit code with a machine-readable result.
2. **Given** a directory selected for upload or download, **when** the CLI transfer runs, **then** the directory structure is preserved on the destination.
3. **Given** invalid credentials or an unreachable host, **when** the command runs, **then** it returns a distinct non-zero exit code, an actionable error, and no secret material.
4. **Given** machine-readable output is requested, **when** the command succeeds or fails, **then** the result is parseable JSON on standard output and progress or diagnostics are written to standard error.
5. **Given** an active transfer, **when** the user queries, pauses, resumes, or cancels it through the CLI, **then** the current status is reported and the action takes effect.
6. **Given** no usable credential in a non-interactive run, **when** the command needs authentication, **then** it fails promptly with a clear message instead of waiting for input.
7. **Given** a remote or local directory, **when** the user creates, renames, or deletes it through the CLI, **then** the operation is performed and destructive operations require an explicit confirmation flag.
8. **Given** a conflict with an existing destination file, **when** no conflict policy is supplied, **then** the command fails instead of overwriting silently.

---

### User Story 2 - Let an AI agent operate on explicitly authorized resources (Priority: P1)

A user enrolls an MCP client, approves a specific scope covering selected connection profiles and directories, and the AI agent lists files, uploads, downloads, manages directories, and queries transfer status within that scope.

**Why this priority**: MCP support is the modern differentiator that makes Thriftp usable by AI agents without weakening its security posture.

**Independent Test**: Enable MCP, enroll a test client, grant access to one profile and one directory, perform browse and transfer operations, and verify that requests outside the grant, traversal attempts, and symbolic-link escapes are denied.

**Acceptance Scenarios**:

1. **Given** MCP is disabled, **when** a client attempts to connect, **then** no server is reachable and the user is shown how to enable it.
2. **Given** a new MCP client, **when** it requests enrollment, **then** it receives no access until the user explicitly approves its identity and scope.
3. **Given** an enrolled client authorized for one profile and directory, **when** the agent lists files and transfers within that scope, **then** the operations succeed.
4. **Given** an authorized scope, **when** the agent requests an unauthorized connection or path, **then** the request is denied with a clear error and no data is returned.
5. **Given** an authorized directory, **when** the agent requests a path using traversal, an alternate separator, encoded traversal, or a symbolic link that escapes the scope, **then** the request is denied and audited.
6. **Given** a destructive operation, **when** the agent requests it without confirmation or an applicable policy, **then** the operation is blocked.
7. **Given** a pre-approved destructive-action policy, **when** the agent performs a covered operation, **then** it proceeds and the policy use is audited.
8. **Given** any MCP operation, **when** it completes, fails, or is denied, **then** an audit record is created.
9. **Given** the audit record cannot be durably written, **when** any MCP operation is requested, **then** the operation is blocked.
10. **Given** the standalone and embedded MCP modes, **when** equivalent operations are performed in each, **then** authorization, audit, and security behavior are identical.

---

### User Story 3 - Control, audit, and revoke MCP access (Priority: P2)

The user reviews enrolled clients and their scopes, revokes access immediately, disables MCP entirely, and inspects an audit trail of every MCP-initiated operation.

**Why this priority**: Exposing file operations to AI agents requires visible, revocable control and accountability.

**Independent Test**: Enroll and grant access to a client, inspect the active grants, revoke them, verify active sessions terminate and subsequent operations are denied, and confirm the audit trail records both allowed and denied attempts.

**Acceptance Scenarios**:

1. **Given** an active MCP session, **when** the user opens the automation controls, **then** the connected clients and their granted profiles, directories, and operations are visible.
2. **Given** an active grant, **when** the user revokes it, **then** active sessions terminate and the client cannot perform another operation under that grant.
3. **Given** any MCP operation, **when** the user reviews the audit trail, **then** it shows the timestamp, client identity, operation, target, and result without exposing secrets.
4. **Given** MCP is enabled, **when** the user disables it, **then** all clients are disconnected and no new MCP sessions can start.
5. **Given** a grant with an expiry, **when** the expiry passes, **then** the client's next operation is denied and the expiry is recorded.
6. **Given** a revocation occurs during an in-flight operation, **when** the operation reaches the next file boundary or the next operation would begin, **then** it stops and the revocation outcome is audited.

---

### User Story 4 - Discover and integrate with documented interfaces (Priority: P2)

A user or integrator reads the documentation, runs CLI help, lists the available MCP tools and their input and output shapes, and confirms the interface version before building automation.

**Why this priority**: Adoption depends on discoverable, stable, and versioned contracts rather than tribal knowledge.

**Independent Test**: From a clean environment, follow the documentation to run CLI help, list MCP tools, enroll a client, and complete a minimal example for each interface without reading source code.

**Acceptance Scenarios**:

1. **Given** the CLI is installed, **when** the user requests help, **then** commands, options, output modes, and exit codes are listed.
2. **Given** the MCP server is enabled, **when** a client requests capabilities, **then** the available tools, their purposes, and their input and output shapes are returned before the first operation.
3. **Given** the published documentation, **when** a new user follows the examples, **then** a minimal transfer through each interface succeeds.
4. **Given** an incompatible client version, **when** it connects, **then** it receives a clear compatibility error rather than undefined behavior.

---

### User Story 5 - Use stable machine-readable results in automation (Priority: P3)

A script author parses command output and exit codes to make decisions, handle failures, and report results in a CI pipeline.

**Why this priority**: Stable machine-readable contracts turn the CLI from a convenience into a dependable automation surface.

**Independent Test**: Run successful and failing commands, parse the structured output from standard output, and verify that documented fields, exit codes, and diagnostics remain consistent.

**Acceptance Scenarios**:

1. **Given** a successful command, **when** machine-readable output is requested, **then** standard output contains the documented JSON fields and a success status.
2. **Given** a failed command, **when** machine-readable output is requested, **then** standard output contains a stable error code and actionable message, and no secrets appear anywhere.
3. **Given** progress or diagnostic messages, **when** machine-readable output is requested, **then** those messages go to standard error and do not contaminate the JSON on standard output.

---

### User Story 6 - Understand what happens when a CLI process exits (Priority: P2)

A user starts a CLI transfer, closes the terminal or interrupts the command, and later needs to know whether the transfer completed, continues, or can be resumed.

**Why this priority**: Automation is only trustworthy when process lifetime and transfer lifetime are clearly defined.

**Independent Test**: Start a CLI transfer, terminate the CLI process early, inspect the resulting state through the CLI and desktop application, and resume the transfer.

**Acceptance Scenarios**:

1. **Given** a CLI transfer command, **when** it is started in the foreground, **then** it remains active until the transfer reaches a terminal state and reports that state.
2. **Given** a CLI transfer in progress, **when** the CLI process exits before completion, **then** the transfer is marked interrupted and resumable and is never reported as complete.
3. **Given** the desktop application and CLI are both running, **when** a transfer is active, **then** both can observe and control the same transfer state.
4. **Given** a CLI-initiated transfer and an open desktop application, **when** the desktop application closes, **then** the transfer is not silently cancelled and either continues under the shared-state mechanism or becomes interrupted and resumable.
5. **Given** the initial release, **when** the CLI starts a transfer, **then** it does not start an undocumented background daemon or service.

---

### Edge Cases

- The CLI is invoked with an unknown command, a missing required option, or conflicting options; it must print usage guidance and return a non-zero exit code.
- A user supplies a secret through a command-line argument or environment variable; the CLI must reject it with guidance and must never echo the value.
- A downstream consumer closes the output pipe early; the CLI must terminate cleanly without crashing or printing secrets.
- A CLI transfer process exits before completion; the transfer must become interrupted and resumable, not completed.
- The desktop application closes while a CLI-initiated transfer is running; the transfer must follow the documented shared-state or foreground-ownership model and must never be reported as complete if it stopped.
- The desktop application, CLI, and MCP server act on the same profile or directory concurrently; conflicts must be reported and must not corrupt files.
- An MCP client disconnects during a transfer; the transfer must follow the user's policy and the outcome must be recorded.
- An MCP grant is revoked or expires while a session is active; active sessions terminate and the next operation is denied.
- A remote client attempts to reach MCP while it is configured for local-only access; the attempt must be denied and recorded.
- An MCP client requests a path containing traversal, alternate separators, encoded traversal, or a symbolic link that escapes its scope; the request must be denied and audited.
- A directory is renamed after a grant is issued; subsequent operations must revalidate the canonical path against the grant.
- An MCP client requests an unknown or unsupported tool; it must receive a clear error without affecting other sessions.
- The audit trail cannot be written; all MCP operations must fail closed, not only destructive ones.
- The audit retention period elapses or the configured storage limit is reached; deletion must follow the configured policy and the user must receive clear notice.
- A directory listing is very large; output must remain usable without exhausting memory or blocking other work.
- A CLI or MCP client uses an incompatible interface version; it must receive a clear compatibility error.
- Standalone and embedded MCP servers run at the same time; grants, state, and audit records must remain consistent, and conflicting sessions must be reconciled without granting unintended access.
- A headless MCP server cannot display a confirmation prompt; destructive operations without an applicable pre-approval policy must be denied.

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
- **FR-008**: Both interfaces MUST use the same connection profiles and transfer state as the desktop application; favorites remain a desktop-only capability in the initial release.
- **FR-009**: Both interfaces MUST follow the same transfer states, conflict policies, resume rules, partial-file handling, pause and resume behavior, concurrency limits, bandwidth limits, and prioritization behavior as the desktop specification.
- **FR-010**: Local-only transport MUST NOT be treated as proof of client identity or authorization; every MCP client MUST enroll and receive an explicit grant.

**Command-Line Interface**

- **FR-011**: The CLI MUST be non-interactive and scriptable for use in shells, scripts, and CI pipelines.
- **FR-012**: The CLI MUST support listing, inspecting, creating, updating, and deleting connection profiles using secure credential input.
- **FR-013**: The CLI MUST support browsing remote directories and displaying file metadata.
- **FR-014**: The CLI MUST support creating, renaming, and deleting remote files and directories.
- **FR-015**: The CLI MUST support creating, renaming, and deleting local files and directories.
- **FR-016**: Destructive CLI operations MUST require an explicit confirmation flag and MUST NOT delete silently; local deletes MUST use the operating system trash when available, and remote deletes MUST warn that they are permanent.
- **FR-017**: The CLI MUST support uploading and downloading individual files, multiple selected files, and directories recursively.
- **FR-018**: The CLI MUST support querying transfer status, pausing, resuming, cancelling, configuring concurrency, setting a global bandwidth limit, and prioritizing or reordering queued transfers.
- **FR-019**: When machine-readable output is requested, the CLI MUST write JSON to standard output and progress and diagnostics to standard error.
- **FR-020**: The CLI MUST provide human-readable output and built-in help for every command.
- **FR-021**: The CLI MUST use stable, documented exit codes that distinguish success, usage errors, authentication failures, network failures, permission failures, conflicts, and cancellations.
- **FR-022**: The CLI MUST NOT accept secrets as command-line arguments or environment variables; when no stored credential exists, it MUST read the secret from standard input and MUST NOT print it in output, logs, or errors.
- **FR-023**: When a required credential is unavailable from the credential store or standard input in a non-interactive run, the CLI MUST fail promptly with a clear message rather than wait for input.
- **FR-024**: The CLI MUST apply the same certificate and host-key verification rules as the desktop UI and MUST NOT offer a silent bypass.
- **FR-025**: The CLI MUST support the same overwrite, skip, rename, and apply-to-all conflict policies as the desktop UI; without an explicit policy, a conflict MUST fail instead of overwriting silently.
- **FR-026**: Recursive CLI transfers MUST preserve symbolic links when supported, skip them with a visible report otherwise, and MUST NOT follow them.
- **FR-027**: The CLI MUST preserve and report the same transfer lifecycle states as the desktop application, including pending, active, paused, completed, failed, cancelled, and interrupted.

**CLI Transfer Lifecycle**

- **FR-028**: A CLI transfer command MUST remain active until the transfer reaches a terminal state by default and MUST report progress on standard error and the final result according to the selected output mode.
- **FR-029**: If the CLI process exits before a transfer reaches a terminal state, the transfer MUST become interrupted and resumable and MUST NOT be reported as completed.
- **FR-030**: The initial release MUST NOT require or start an undocumented background daemon or service for CLI transfers.
- **FR-031**: When more than one of the desktop application, CLI, and MCP server are running, they MUST observe and control the same active transfers and present consistent states; the mechanism for sharing state is an architectural decision for planning.
- **FR-032**: Closing the desktop application MUST NOT silently cancel a CLI-initiated transfer; the transfer MUST continue when the shared-state mechanism supports it, otherwise it MUST become interrupted and resumable.
- **FR-033**: The chosen transfer process model MUST be documented and consistent across the desktop application, CLI, and MCP server.
- **FR-034**: A CLI transfer that is interrupted by process exit MUST retain the same partial-file and resume-verification behavior as the desktop specification.

**MCP Client Identity and Authorization**

- **FR-035**: Every MCP client MUST have a stable, user-visible identity before it can request any capability.
- **FR-036**: MCP client enrollment MUST require explicit user approval through a user-controlled interface, including a CLI or desktop control surface for headless operation.
- **FR-037**: Authorization grants MUST be bound to a specific client identity and MUST NOT be usable by another client.
- **FR-038**: Grants MUST restrict access to specific connection profiles, directories, and permitted operations.
- **FR-039**: Grants MUST follow least privilege, be scoped, persist across restarts until revoked or expired, support an optional expiry, and be revocable at any time.
- **FR-040**: Revoking a grant MUST terminate active sessions and MUST prevent queued or in-flight operations from continuing before the next file within the operation or before the next operation begins.
- **FR-041**: The user MUST be able to view enrolled clients, active sessions, and grants, and MUST be able to disconnect any client.
- **FR-042**: MCP client credentials MUST be protected from disclosure and MUST NOT be usable by an unenrolled client.
- **FR-043**: In standalone headless mode, the MCP enabled state, enrolled clients, and active sessions MUST be visible through a status command or user-controlled configuration.
- **FR-044**: Enabling MCP MUST NOT grant access to any connection, directory, or operation by default.
- **FR-045**: The MCP server MUST be opt-in and disabled by default, and MUST be available both as a standalone headless process and as an embedded mode within the desktop application.

**MCP Directory Scope Enforcement**

- **FR-046**: Directory grants MUST be expressed as canonical paths associated with a connection profile.
- **FR-047**: Every MCP operation MUST resolve and canonicalize the requested path and verify that it is contained within the granted scope before performing the operation.
- **FR-048**: MCP MUST deny traversal sequences, alternate path separators, encoded traversal, and equivalent path-manipulation techniques that escape the granted scope.
- **FR-049**: MCP MUST NOT follow symbolic links that resolve outside the granted scope and MUST report the denial.
- **FR-050**: Directory scope enforcement MUST apply to both local and remote filesystem operations wherever those operations are exposed.
- **FR-051**: MCP MUST revalidate the canonical path against the grant after a rename, create, or other operation that can change path resolution.
- **FR-052**: If a path cannot be resolved unambiguously, MCP MUST deny the operation rather than guess.

**MCP Destructive Operations**

- **FR-053**: Destructive MCP operations MUST require explicit confirmation for each operation by default; users MAY opt in to a scoped pre-approval policy within a grant, and every use of that policy MUST be audited.
- **FR-054**: When the MCP server cannot obtain the required confirmation, including in standalone headless operation without an interactive approval channel, the destructive operation MUST be denied.
- **FR-055**: The absence of a graphical interface MUST NEVER implicitly authorize an operation.
- **FR-056**: A destructive operation MUST NOT proceed without a durable audit record.

**MCP Audit**

- **FR-057**: Every MCP operation, including denied, blocked, and failed attempts, MUST produce an audit record.
- **FR-058**: Audit records MUST include the timestamp, client identity, interface, operation, target, result, and redacted error context, and MUST NOT contain secrets.
- **FR-059**: Audit records MUST be retained for 90 days by default, with a user-configurable retention period and clear notice before deletion.
- **FR-060**: Audit records MUST be protected from modification or deletion by MCP clients and MUST NOT be removed before the configured retention period without explicit user action.
- **FR-061**: If the audit record cannot be durably written, all MCP operations MUST fail closed; there is no read-only exception.
- **FR-062**: Destructive MCP operations MUST never proceed without a durable audit record, even if other operations are treated differently by a future amendment.
- **FR-063**: Audit failure, retention deletion, and revocation events MUST themselves be recorded where the audit system remains writable.

**MCP Tools and Capabilities**

- **FR-064**: The MCP server MUST expose tools for listing connections, browsing directories, inspecting files, uploading, downloading, creating, renaming, deleting, querying transfer status, pausing, resuming, cancelling, setting a global bandwidth limit, and prioritizing or reordering transfers.
- **FR-065**: The MCP server MUST publish its available tools, input and output shapes, and interface version to enrolled clients before their first operation.
- **FR-066**: MCP errors MUST be actionable and MUST distinguish authorization, validation, conflict, network, and internal failures.
- **FR-067**: MCP MUST NOT expose favorites management, arbitrary shell commands, or server-administration tools in the initial release.
- **FR-068**: Standalone and embedded MCP modes MUST expose the same tools and enforce the same authorization, audit, and security rules.
- **FR-069**: The MCP server MUST use a local-only communication channel and MUST NOT accept remote network clients in the initial release.

**Security and Privacy**

- **FR-070**: Both interfaces MUST use the native operating system credential store and MUST NOT persist credentials in plaintext.
- **FR-071**: Both interfaces MUST NOT weaken or bypass certificate or host-key verification.
- **FR-072**: Both interfaces MUST apply the principle of least privilege to the operations they expose.
- **FR-073**: Both interfaces MUST NOT expose credentials, private keys, tokens, secrets, or unredacted client credentials in responses, logs, or audit records.
- **FR-074**: The application MUST NOT run an MCP server without a visible, user-controlled enabled state.
- **FR-075**: CLI and MCP operations MUST respect operating-system filesystem permissions and the same local access rules as the desktop application.

**Documentation and Compatibility**

- **FR-076**: The project MUST document installation, configuration, authentication, commands, directory operations, MCP enrollment, grants, MCP tools, examples, exit codes, output streams, and audit behavior for both interfaces.
- **FR-077**: Both interfaces MUST be versioned and MUST document their compatibility policy.
- **FR-078**: Breaking changes to either interface MUST follow the project's versioning and amendment process.
- **FR-079**: The documentation MUST state the transfer process model, including whether transfers continue after the CLI exits and how desktop, CLI, and MCP observe shared transfer state.
- **FR-080**: The documentation MUST state how MCP clients enroll, how grants are scoped and revoked, and what happens when confirmation or audit logging is unavailable.

### Key Entities *(include if feature involves data)*

- **Automation Interface**: A first-class non-UI surface, either the CLI or the MCP server, with a version, capabilities, and documentation.
- **CLI Command**: A documented invocation with arguments, options, output modes, exit codes, and a structured result schema.
- **CLI Invocation Result**: The outcome of a command, including exit code, structured JSON result, diagnostics, duration, and affected targets.
- **CLI Transfer Session**: The lifecycle of a CLI-initiated transfer, including whether the CLI process owns it, whether it is shared, and its resulting interrupted, completed, or cancelled state.
- **MCP Client**: An enrolled AI agent or tool with a stable identity, display name, credential reference, enrollment state, sessions, and grants.
- **Enrollment Request**: A pending request from an MCP client for a user-approved identity and initial scope.
- **MCP Tool**: A documented operation exposed to authorized clients, with a purpose, input shape, output shape, and destructive-action flag.
- **Authorization Grant**: A user-approved scope that binds a specific MCP client to connection profiles, canonical directories, and permitted operations, with an optional expiration and a revoked, expired, or active state.
- **Approval Policy**: An opt-in rule within a grant that pre-approves specific destructive operations, with a defined scope, an audit requirement, and a revocation state.
- **Directory Scope**: A canonical directory path associated with a connection profile, used to validate every MCP path request.
- **MCP Session**: The lifetime of a client connection, including its runtime mode (standalone or embedded), grants, activity, and audit sequence.
- **Audit Event**: An immutable record of an automation operation, including timestamp, client identity, interface, operation, target, result, redacted error context, and retention status.
- **Connection Profile**: An existing saved server configuration reused by both interfaces.
- **Transfer**: An existing upload or download operation whose state is shared across the desktop UI, CLI, and MCP server.

### Out of Scope for the Initial Release

- Remote MCP access over public networks; the initial release is local-only.
- An always-running background transfer daemon or service; the process model is an architectural decision for planning.
- A terminal user interface or interactive CLI shell.
- Favorites management through CLI or MCP; favorites remain a desktop UI capability.
- Plugins or third-party extensions for either interface.
- Arbitrary shell or server-administration tools exposed through MCP.
- Cloud synchronization of profiles, grants, or audit records.
- Non-file-transfer MCP tools unrelated to Thriftp's core capabilities.

### Architectural Decisions Deferred to Planning

- **Transfer process ownership and shared state**: whether CLI, desktop, and MCP transfers are coordinated by a foreground process, a persistent local service, or another mechanism. The specification requires consistent observable state and does not assume an always-on daemon.
- **MCP client identity proofing**: the credential format and enrollment handshake used to bind a client identity to its grants. The specification requires stable identity, explicit approval, and non-transferable grants.
- **Supported MCP protocol versions**: which Model Context Protocol versions are supported and how compatibility is negotiated.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A user can connect to a server and complete an upload and a download through the CLI in under 2 minutes using only the published documentation.
- **SC-002**: 100% of documented CLI commands return a stable exit code and machine-readable result for both success and failure.
- **SC-003**: 100% of machine-readable CLI runs place valid JSON on standard output and progress or diagnostics on standard error, with no contamination between streams.
- **SC-004**: A security review finds zero credentials, private keys, or client credentials in CLI output, MCP responses, logs, or audit records.
- **SC-005**: MCP is disabled by default in 100% of fresh installations and cannot be reached until the user explicitly enables it.
- **SC-006**: 100% of MCP clients are enrolled through explicit user approval before receiving any grant.
- **SC-007**: 100% of requests outside an MCP client's authorized scope are denied and audit-recorded.
- **SC-008**: 100% of traversal, alternate-separator, encoded-traversal, and symbolic-link escape attempts are denied for local and remote operations.
- **SC-009**: 100% of destructive MCP operations require confirmation or a pre-approved policy before execution, with confirmation as the default.
- **SC-010**: In headless mode without an approval channel or applicable policy, 100% of destructive MCP operations are denied.
- **SC-011**: 100% of MCP operations, including denied attempts, produce an audit record with the documented fields.
- **SC-012**: When the audit record cannot be durably written, 100% of MCP operations are blocked.
- **SC-013**: 100% of audit records are retained for the configured period, and users receive notice before any deletion.
- **SC-014**: Revoking an MCP grant terminates active sessions and prevents the next operation in 100% of tests.
- **SC-015**: 100% of revoked or expired grants are denied before the client's next operation.
- **SC-016**: 100% of MCP clients receive the available tool list and interface version before their first operation.
- **SC-017**: Standalone and embedded MCP modes produce identical authorization, audit, and security results in 100% of contract tests.
- **SC-018**: With default settings, the MCP server accepts zero remote network clients in the initial release.
- **SC-019**: CLI and MCP contract tests pass on every supported operating system in continuous integration.
- **SC-020**: A new integrator can complete a minimal CLI example and a minimal MCP example in under 10 minutes each.
- **SC-021**: Equivalent operations through the desktop UI, CLI, and MCP server produce identical results in 100% of contract tests.
- **SC-022**: 100% of CLI authentication tests using standard input complete without the secret appearing in process arguments, environment, output, logs, or audit records.
- **SC-023**: A CLI transfer command remains active until a terminal state by default, and an early process exit produces an interrupted, resumable transfer in 100% of tests.
- **SC-024**: When the desktop application, CLI, and MCP server are running, they report the same state for the same active transfer in 100% of tests.
- **SC-025**: 100% of CLI directory create, rename, and delete operations require explicit confirmation for destructive actions and never delete silently.

## Assumptions

- The CLI and MCP server can operate headlessly using the same core and operating system credential store as the desktop application.
- The MCP server supports both standalone headless and embedded modes; both modes share the same authorization, audit, and security rules.
- This feature implements the constitution's Automation & Integration principle, including shared core operations, security parity, explicit authorization, auditability, and contract tests.
- Initial-release MCP transport is local-only; remote access is a future capability requiring explicit configuration.
- The default machine-readable output format is structured JSON on standard output; alternative formats may be added later.
- Users explicitly enroll and authorize every MCP client and scope; grants persist across restarts until revoked or expired, and no client is trusted implicitly.
- Destructive MCP operations require per-operation confirmation by default; pre-approval policies are opt-in, scoped, and audited.
- The MCP implementation follows the Model Context Protocol and documents the supported protocol version.
- CLI and MCP operations share the same connection profiles and transfer state as the desktop application; favorites remain desktop-only.
- The initial CLI covers connection management, browsing, directory operations, transfers, and queue control; it does not provide an interactive shell.
- Audit records are stored locally, retained for 90 days by default, and configurable by the user; they are not transmitted to a remote service.
- All MCP operations fail closed when audit logging is unavailable; there is no read-only exception in the initial release.
- Authentication secrets are read from standard input or retrieved from the operating system credential store, never from command-line arguments or environment variables.
- The initial release does not require an always-running background daemon; the shared-state mechanism and its fallback behavior are architectural decisions for planning.
- Technology and process architecture choices are deferred to the planning phase; this specification defines observable behavior only.
