# Contract: Automation Authorization and Audit

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

This contract defines MCP client identity, enrollment, grants, canonical path scopes, destructive-action approvals, revocation, and the audit trail. It is owned by `thriftp-core` and is independent of the CLI, MCP transport, and Tauri.

## Security Principles

1. Local transport is never proof of identity or authorization.
2. No access is granted until a user explicitly approves a specific client and scope.
3. Grants follow least privilege and are bound to one client identity.
4. Every path is canonicalized and checked before an operation; ambiguous paths are denied.
5. Destructive operations require single-use confirmation or a scoped, audited pre-approval policy.
6. Every MCP operation produces a durable audit record; if the record cannot be written, the operation is blocked.
7. No secret, token, private key, or credential is stored in automation state or audit records.
8. Revocation and expiry take effect immediately and stop in-flight work at the next safe boundary.

## Client Identity

- The client generates an Ed25519 keypair locally. The private key never leaves the client.
- The public key is 32 bytes, encoded as base64. The fingerprint is the lowercase hexadecimal SHA-256 of the public key and is the stable authorization identity.
- The display name is user-visible and untrusted; it never affects authorization.
- One public key maps to exactly one client record. A client may re-enroll after revocation only through a new explicit approval.
- The server stores the public key and fingerprint; it never stores a client private key or a long-lived client secret.

## Enrollment

```text
Client                         Server                         User
  |                              |                              |
  |-- enroll(publicKey, name) -->|                              |
  |<-- requestId, expiresAt -----|                              |
  |                              |-- pending request ---------->|
  |                              |<-- approve(scope) -----------|
  |                              |-- create grant ------------->|
  |-- enrollment_status -------->|                              |
  |<-- approved, grant summary --|                              |
```

**Rules**:
- Enrollment creates a pending request and grants nothing.
- Pending requests expire after 10 minutes by default and are capped at 10 globally and 3 per fingerprint.
- Approval requires at least one existing connection profile, at least one remote directory scope for remote filesystem operations, at least one local directory scope when upload or download is granted, and an explicit operation set. The approval surface displays the exact resulting scope before it is applied.
- Denied, expired, and revoked requests cannot be approved retroactively.
- Approval, denial, expiry, and revocation are audited.
- Approval works whether or not an MCP server is currently running; a running server observes the shared `stateRevision` before its next request, and a future server loads the approved grant at startup.
- A client that never completes enrollment receives no operational capability.

## Session Authentication

1. The client calls `thriftp.automation.challenge` with its fingerprint.
2. The server returns a 32-byte random nonce, its expiry, and a challenge ID. Nonces are single-use and expire after 60 seconds.
3. The client signs the nonce with its Ed25519 private key and calls `thriftp.automation.authenticate`.
4. The server verifies the fingerprint, public key, signature, grant state, grant revision, revocation epoch, and expiry.
5. On success, the server creates an `MCPSession` and returns an opaque 256-bit session token, its idle and absolute expiry, and a grant summary.

**Rules**:
- Session tokens are held in memory, bound to the client fingerprint and transport, and never written to disk or logs.
- Tokens expire after 15 minutes of inactivity or 8 hours absolute, whichever comes first.
- Revocation, any grant revision change, grant expiry, or MCP disablement invalidates all matching sessions immediately.
- Re-authentication requires a new challenge and signature.
- A token presented for a different fingerprint or transport is rejected. Token comparison is constant-time.

## Grant Model

A grant contains:

- the bound client fingerprint;
- a revision that increments on any scope, operation, expiry, or policy change;
- granted connection profile IDs;
- granted directory scopes;
- granted operations;
- an optional absolute expiry;
- an optional `disconnectPolicy` (`continue` or `cancel`, default `continue`);
- optional pre-approval policies;
- a revocation epoch that increments on revocation.

**Rules**:
- Operations are a subset of the catalog in [mcp-tools.md](./mcp-tools.md); no wildcard is permitted.
- Remote filesystem operations require at least one remote directory scope. Upload and download additionally require at least one local directory scope, so local sources and destinations are never unrestricted.
- A grant never grants a parent, sibling, or symlink target outside its scopes.
- Effective capability is the intersection of grant operations and protocol capabilities. An operation the protocol cannot perform returns `unsupported_capability`.
- Changing a grant increments its revision and invalidates sessions that authenticated under an older revision.
- Expiry is evaluated on every request and at every file boundary.
- Grants persist across restarts until revoked or expired.

## Canonical Path Scopes

### Local paths

Local scopes govern MCP upload sources and download destinations. The CLI is user-authorized and is not restricted to MCP local scopes, but it still follows OS filesystem permissions and never follows escaping symlinks during recursive transfer.

1. Require an absolute path and reject NUL bytes.
2. Canonicalize the path with the operating system canonicalizer, resolving `.`, `..`, and symbolic links.
3. For a path that does not exist yet (create or rename destination), canonicalize the nearest existing parent and append the validated final component.
4. Verify containment using path-component boundaries, not string prefixes. `/srv/app2` is not inside `/srv/app`.
5. Deny if canonicalization fails, if the path is not absolute, or if any resolved component escapes the scope.
6. Revalidate the canonical path after create, rename, or any operation that can change resolution.

### Remote paths

1. Require an absolute POSIX path. Reject NUL, backslash separators, repeated leading separators, and empty components.
2. Reject encoded traversal or separators, including percent-encoded `.`, `/`, and `\` sequences, before normalization.
3. Normalize `.` and `..` lexically. A `..` that would escape the remote root is denied.
4. Resolve the real path with the protocol's real-path or equivalent operation when available. SFTP uses real-path and link-target inspection. FTP/FTPS may not support real-path; in that case inspect each component through stat/listing and deny any symbolic-link component that cannot be verified as inside the scope.
5. Verify containment against every granted scope using component boundaries.
6. Deny ambiguous paths rather than guessing.
7. Revalidate both source and destination after rename, and the canonical path after create.
8. Never follow a symbolic link that resolves outside the scope; report the denial.

**Common rules**:
- Remote scope checks apply to browsing, stat, create, rename, delete, and recursive traversal.
- Local scope checks apply to every MCP upload source and every MCP download destination, including each file in a recursive operation. An MCP client cannot use upload to read an arbitrary local file or download to write an arbitrary local path outside its local scopes.
- Recursive traversal rechecks each entry before operating on it, so a link or rename cannot escape mid-operation.
- Paths in audit records are the canonical authorized form, never a raw unvalidated input.

## Destructive Operations and Approvals

Destructive operations are delete, rename, and overwrite of an existing destination. Directory create and non-overwriting transfer are not destructive.

### Default confirmation

- Each destructive request requires a single-use confirmation bound to the session, operation, and canonical target.
- Confirmations expire after 2 minutes and cannot be reused, extended, or transferred.
- Confirmation is requested through MCP elicitation when the client supports it; otherwise it is queued for the desktop control surface or the CLI.
- A tool call without a valid confirmation returns `confirmation_required` with a confirmation ID and expiry. The client retries the same call after approval.
- In standalone headless mode without an approval channel, the operation is denied.
- The absence of a graphical interface never implies authorization.

### Pre-approval policies

- A grant may include policies scoped to specific directories and destructive operations, with an optional expiry and optional single use.
- A policy cannot exceed the grant's scopes or expiry.
- Every use produces a `policy_used` audit record with the policy ID.
- Revoking a policy takes effect immediately.
- A policy never authorizes a target outside its scopes.

## Revocation, Expiry, and Disablement

- Revoking a client or grant increments the revocation epoch and terminates all matching sessions.
- Active operations stop at the next file boundary; the next operation is denied before it starts.
- Queued work under the revoked grant is removed or denied.
- Grant expiry is evaluated on every request and at every file boundary.
- Disabling MCP terminates sessions, closes the embedded endpoint, and stops new sessions. Grants and audit records remain persisted but inactive. Re-enabling does not reactivate revoked or expired grants.
- Revocation, expiry, and disablement are audited, including the reason and affected session or grant.

## Concurrent MCP Servers

- Standalone and embedded MCP servers may run at the same time. They share `automation.json` and the audit store; sessions remain per-process.
- Every write to automation state takes an exclusive cross-process lock and increments `stateRevision`.
- Every MCP request compares the revision it last loaded. If the revision changed, it reloads grants and MCP settings before authorizing. Revocation, expiry, disablement, and scope changes therefore take effect in every server no later than the next request or file boundary.
- A server that detects a revoked or expired grant terminates its matching sessions and reports the termination in the audit trail.
- No server can grant access that another server has revoked, and no server can broaden a scope by writing stale state.

## Audit Trail

### Record requirements

Every MCP operation, including denied, blocked, failed, and pre-enrollment attempts, produces an audit record with:

- monotonic sequence, UTC timestamp, interface, and runtime mode;
- client fingerprint and display name when known;
- session ID when known;
- operation or lifecycle event;
- canonical target profile and path when applicable;
- result (`allowed`, `denied`, `blocked`, `failed`, `completed`);
- stable error code and redacted context;
- duration;
- previous-record hash and current-record hash.

No secret, token, private key, credential, raw server diagnostic, or full user home path beyond the authorized target appears in a record.

### Durability

- Records are appended to daily JSONL segments and `fsync`ed before any result is returned.
- Every append takes an exclusive cross-process lock so concurrent MCP servers produce one linear hash chain.
- For destructive operations, the authorization record is durable before execution.
- If a record cannot be written, the operation returns `audit_unavailable` and does not execute.
- Non-destructive failures also require a durable outcome record; if that write fails, the caller receives `audit_unavailable`, not success.
- A broken hash chain is reported at startup and by `thriftp mcp audit verify`; it is never silently repaired.
- Audit files use restrictive permissions and are not writable through any MCP tool.

### Retention and storage limit

- Default retention is 90 days, configurable from 1 to 3,650 days.
- The audit store has a default 1 GiB storage limit, configurable or nullable for retention-only operation.
- A `retention_notice` record is written at least 7 days before any segment deletion and surfaced in the CLI and desktop control surface.
- No segment is deleted before its retention period elapses, before the notice is durable, or if the notice cannot be written.
- When the storage limit is reached, only the oldest sealed segments are eligible; the same notice rule applies. If no segment can be deleted after notice, new operations are blocked with `audit_unavailable`.
- Each deletion is audited before the segment is removed.

## MCP Enabled State

- MCP is disabled by default in every fresh installation and upgrade.
- Enabling requires an explicit user action in the CLI or desktop control surface.
- The enabled state, mode, enrolled clients, active sessions, and recent audit events are visible in both surfaces.
- Disabling is immediate and does not require restarting the application.
- The standalone server refuses to start operational tools when disabled and reports `mcp_disabled`.
- The embedded server is not started when disabled.

## Persistence and Recovery

- Automation state is stored in `automation.json` with a schema version and ordered migrations. Writes use atomic replace with backup and restrictive permissions.
- Clients, enrollment requests, grants, directory scopes, and approval policies are persisted. Sessions, confirmations, nonces, and tokens are runtime-only.
- On startup: expire overdue requests and grants, remove runtime sessions and nonces, verify grant/scope consistency, clean stale host files, and verify the audit chain.
- Corrupt automation state is quarantined and reported; it is never silently replaced or reset.
- A missing scope or profile referenced by a grant makes the affected operation unavailable and is reported; it is never treated as a broader scope.

## User Surfaces

| Action | CLI | Desktop |
|--------|-----|---------|
| Enable or disable MCP | `thriftp mcp enable/disable` | Automation settings |
| Review pending clients | `thriftp mcp clients pending` | Pending requests list |
| Approve or deny | `thriftp mcp clients approve/deny` | Approval dialog |
| View grants and scopes | `thriftp mcp clients list` | Client and grant list |
| Revoke or disconnect | `thriftp mcp clients revoke/disconnect` | Revoke and disconnect actions |
| Review audit | `thriftp mcp audit list/export` | Audit view |
| Verify audit chain | `thriftp mcp audit verify` | Integrity status |
| Change retention | `thriftp mcp audit retention` | Retention setting |

Headless deployments use the CLI as the user-controlled interface. No action requires a graphical session.

## Error Codes

| Code | Condition |
|------|-----------|
| `mcp_disabled` | MCP is not enabled |
| `client_not_enrolled` | No approved identity for the fingerprint |
| `grant_missing` | No active grant |
| `grant_expired` | Grant expiry reached |
| `grant_revoked` | Grant or client revoked |
| `scope_denied` | Profile or path outside the grant |
| `path_ambiguous` | Canonical resolution is not unambiguous |
| `confirmation_required` | Destructive operation lacks confirmation or policy |
| `confirmation_expired` | Confirmation expired |
| `audit_unavailable` | Durable audit write failed |
| `unsupported_protocol_version` | Requested MCP revision is unsupported |

## Test Contract

- Enrollment cannot grant access before approval; expired and denied requests never work.
- A grant is unusable by a different key, fingerprint, or transport.
- Scope tests cover sibling prefixes, `.` and `..`, repeated separators, backslashes, percent-encoded traversal, symlink escapes, and rename-based escapes for both local and remote paths.
- Revocation tests cover active, in-flight, queued, and re-authentication scenarios.
- Confirmation tests cover single use, expiry, target binding, policy use, and headless denial.
- Audit tests inject write failure, disk full, permission denial, and process crash; every case blocks or reports per the contract.
- Retention tests verify the notice window, no premature deletion, storage-limit behavior, and audited deletion.
- Concurrent standalone and embedded servers share grants and audit consistently; revocation in one is observed by the other.
- Secret tests assert that no credential, token, private key, or raw diagnostic appears in state, output, logs, or audit.
- Standalone and embedded modes run the same authorization test suite.

## Contract Invariants

1. Enrollment grants nothing until explicit user approval.
2. A grant is bound to exactly one client fingerprint and cannot be transferred.
3. Every path is canonicalized and checked with component boundaries before every operation.
4. Ambiguous paths and unverifiable symlink components are denied.
5. Destructive operations require single-use confirmation or an applicable policy.
6. Headless operation without a confirmation channel denies destructive operations.
7. Every MCP operation produces a durable audit record; audit failure blocks the operation.
8. Revocation and expiry take effect immediately and stop work at a safe boundary.
9. No secret appears in automation state, audit records, output, logs, or errors.
10. MCP is disabled by default and can only be enabled by explicit user action.
11. Concurrent standalone and embedded servers share one revisioned authorization state and one linear audit chain.
