# Data Model: CLI and MCP Automation Interfaces

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

## Entity Overview

```text
ConnectionProfile (existing) 1 ─── 0..* DirectoryScope (kind=remote)
DirectoryScope (kind=local) 0..* ─── 0..* AuthorizationGrant
ConnectionProfile 1 ─── 0..* Transfer (existing)

MCPClient 1 ─── 0..1 AuthorizationGrant (active)
MCPClient 1 ─── 0..* MCPSession (runtime)
MCPClient 1 ─── 0..* AuditEvent
EnrollmentRequest 0..1 ─── 1 AuthorizationGrant
AuthorizationGrant 1 ─── 1..* DirectoryScope
AuthorizationGrant 1 ─── 0..* ApprovalPolicy
MCPSession 1 ─── 0..* ConfirmationRequest (runtime)
AuthorizationGrant 0..* ─── 0..* AuditEvent
TransferHost 1 ─── 0..* Transfer
TransferHost 1 ─── 0..* HostControlClient (runtime)
CLIInvocationResult 0..* ─── 0..* Transfer
```

All automation entities are Tauri-independent and belong to `thriftp-core`. Runtime entities are never persisted unless explicitly stated.

## Entities

### AutomationSettings

Singleton configuration for both automation surfaces.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `schemaVersion` | integer | Yes | Current schema version |
| `mcpEnabled` | boolean | Yes | Default `false` |
| `mcpMode` | enum | Yes | `standalone`, `embedded`, `both`; default `standalone` |
| `loopbackPort` | integer or null | No | Embedded mode; null or 0 selects an ephemeral port |
| `auditRetentionDays` | integer | Yes | Default 90; range 1–3,650 |
| `auditNoticeDays` | integer | Yes | Default 7; range 1–30 and less than retention |
| `auditMaxBytes` | integer or null | No | Default 1 GiB; null means retention-only |
| `enrollmentRequestTtlSeconds` | integer | Yes | Default 600 |
| `confirmationTtlSeconds` | integer | Yes | Default 120 |
| `maxPendingEnrollmentRequests` | integer | Yes | Default 10 |
| `maxPendingEnrollmentRequestsPerFingerprint` | integer | Yes | Default 3 |
| `maxEnrolledClients` | integer | Yes | Default 100 |
| `updatedAt` | timestamp | Yes | UTC |

**Rules**: MCP cannot be enabled without an explicit user action. Disabling MCP terminates sessions, removes the embedded endpoint, and leaves grants persisted but inactive. Retention settings cannot delete audit data immediately; they only affect future retention checks.

### MCPClient

A stable, user-visible automation client identity.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Internal record identifier |
| `fingerprint` | string | Yes | SHA-256 of the Ed25519 public key, lowercase hex; stable identity and unique |
| `publicKey` | string | Yes | Base64-encoded 32-byte Ed25519 public key |
| `displayName` | string | Yes | 1–100 characters after trimming; user-visible, not trusted for authorization |
| `state` | enum | Yes | `pending`, `active`, `revoked` |
| `enrolledAt` | timestamp or null | No | Set when the first grant is approved |
| `lastSeenAt` | timestamp or null | No | Most recent authenticated session |
| `revokedAt` | timestamp or null | No | Set when all access is revoked |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |

**Rules**: The fingerprint is the authorization identity and the client credential reference; the public key is the proof-of-possession material. The display name is presentation only. A public key maps to exactly one client. Revoking a client revokes all of its grants and terminates all sessions. A revoked client may re-enroll only through a new explicit approval, and the new grant is recorded separately.

### EnrollmentRequest

A pending request for user approval.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Request identifier shown to the user |
| `fingerprint` | string | Yes | Client fingerprint |
| `publicKey` | string | Yes | Proposed client key |
| `displayName` | string | Yes | Proposed display name |
| `requestedScopes` | object or null | No | Optional client-suggested profiles and paths; never grants access |
| `state` | enum | Yes | `pending`, `approved`, `denied`, `expired` |
| `requestedAt` | timestamp | Yes | UTC |
| `expiresAt` | timestamp | Yes | `requestedAt` plus the configured TTL |
| `decidedAt` | timestamp or null | No | UTC |
| `decidedBy` | enum or null | No | `desktop`, `cli` |
| `grantId` | UUID or null | No | Set when approved |

**Rules**: Pending requests grant no access. Expired or denied requests cannot be approved later; the client must enroll again. Approval always selects an explicit grant scope, even if the client suggested scopes.

### AuthorizationGrant

A user-approved scope bound to one client identity.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Grant identifier |
| `clientFingerprint` | string | Yes | Bound client identity |
| `revision` | integer | Yes | Starts at 1; increments on any scope or policy change |
| `state` | enum | Yes | `active`, `expired`, `revoked` |
| `profileIds` | UUID[] | Yes | At least one existing connection profile |
| `scopeIds` | UUID[] | Yes | At least one directory scope for filesystem operations |
| `operations` | enum[] | Yes | Permitted operations; subset of the catalog |
| `createdAt` | timestamp | Yes | UTC |
| `updatedAt` | timestamp | Yes | UTC |
| `expiresAt` | timestamp or null | No | Optional absolute expiry |
| `revokedAt` | timestamp or null | No | Set on revocation |
| `revocationEpoch` | integer | Yes | Starts at 0; increments on revocation |
| `disconnectPolicy` | enum | Yes | `continue` (default) or `cancel` for transfers whose MCP client disconnects |
| `lastUsedAt` | timestamp or null | No | Most recent authorized operation |

**Operation enum**: `connections.list`, `connections.get`, `browse.list`, `files.stat`, `directories.create`, `directories.rename`, `directories.delete`, `files.delete`, `transfers.upload`, `transfers.download`, `transfers.list`, `transfers.status`, `transfers.pause`, `transfers.resume`, `transfers.cancel`, `transfers.priority`, `transfers.reorder`, `queue.get`, `queue.setConcurrency`, `queue.setBandwidth`.

**Rules**: A grant never grants access to a profile or path outside its lists. `operations` must be the minimum requested by the user. Changing profiles, scopes, operations, expiry, or approval policies increments `revision`. Revocation is immediate and sets `revocationEpoch`; sessions and in-flight work must observe the new epoch. Expiry is evaluated on every request. A grant for remote filesystem operations requires at least one remote directory scope; a grant that includes upload or download also requires at least one local directory scope, so the MCP client cannot read or write arbitrary local paths.

### DirectoryScope

A canonical directory grant, either remote (associated with a connection profile) or local.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Scope identifier |
| `grantId` | UUID | Yes | Owning grant |
| `kind` | enum | Yes | `remote` or `local` |
| `profileId` | UUID or null | Conditional | Required for `remote`; null for `local` |
| `canonicalPath` | string | Yes | Canonical path; local absolute path or remote POSIX absolute path |
| `recursive` | boolean | Yes | Always true in the initial release |
| `createdAt` | timestamp | Yes | UTC |

**Rules**: The path is canonical when stored and revalidated on every request. A `remote` scope requires an existing connection profile; a `local` scope has no profile and governs local upload sources and download destinations exposed through MCP. The root path `/` is allowed only with explicit user approval. Local paths use OS canonicalization; remote paths use protocol real-path resolution where available. A scope never implies access to a parent, sibling, or symlink target outside the canonical path. A grant that includes upload or download operations requires at least one `remote` scope and at least one `local` scope.

### ApprovalPolicy

An optional scoped pre-approval for destructive operations.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | UUID | Yes | Policy identifier |
| `grantId` | UUID | Yes | Owning grant |
| `scopeIds` | UUID[] | Yes | Scopes the policy applies to |
| `operations` | enum[] | Yes | Destructive operations: `directories.delete`, `files.delete`, `directories.rename`, `overwrite` |
| `createdAt` | timestamp | Yes | UTC |
| `expiresAt` | timestamp or null | No | Must not exceed the grant expiry |
| `revokedAt` | timestamp or null | No | Set when revoked |
| `singleUse` | boolean | Yes | Default `false` |
| `lastUsedAt` | timestamp or null | No | Most recent use |

**Rules**: A policy never broadens the grant. Every use produces a `policy_used` audit record. An expired or revoked policy cannot authorize an operation. A policy with `singleUse` is consumed after one successful use. Revoking a policy does not require incrementing the grant revision, but it does invalidate cached approvals.

### MCPSession (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Session identifier |
| `clientFingerprint` | string | Authenticated client |
| `grantId` | UUID | Active grant |
| `grantRevision` | integer | Revision authenticated under |
| `revocationEpoch` | integer | Epoch at authentication |
| `transport` | enum | `stdio`, `loopback` |
| `mode` | enum | `standalone`, `embedded` |
| `state` | enum | `authenticating`, `active`, `terminated` |
| `terminationReason` | enum or null | `closed`, `revoked`, `expired`, `disabled`, `error` |
| `startedAt` | timestamp | UTC |
| `lastActivityAt` | timestamp | UTC |
| `confirmationIds` | UUID[] | Pending confirmations |
| `auditSequence` | integer | Last audit sequence emitted for this session |

**Rules**: Sessions are never persisted. A session cannot outlive its grant or the MCP enabled state. Revocation terminates all sessions for the client. The next operation after any grant change revalidates revision, epoch, and expiry.

### ConfirmationRequest (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `id` | UUID | Confirmation identifier |
| `sessionId` | UUID | Requesting session |
| `clientFingerprint` | string | Requesting client |
| `operation` | enum | Destructive operation |
| `target` | object | Profile ID and canonical path |
| `state` | enum | `pending`, `approved`, `denied`, `expired` |
| `createdAt` | timestamp | UTC |
| `expiresAt` | timestamp | `createdAt` plus the configured TTL |
| `decidedAt` | timestamp or null | UTC |
| `decidedBy` | enum or null | `elicitation`, `desktop`, `cli`, `policy` |

**Rules**: A confirmation is single-use, bound to one session, operation, and canonical target. It cannot be reused, transferred, or extended. Without a confirmation or applicable policy, a destructive operation is denied and audited.

### AuditEvent

An immutable record of an automation operation.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `seq` | integer | Yes | Monotonic within the audit store |
| `timestamp` | timestamp | Yes | UTC with millisecond precision |
| `interface` | enum | Yes | `mcp` in the initial release |
| `mode` | enum | Yes | `standalone`, `embedded` |
| `clientFingerprint` | string or null | No | Null for pre-enrollment events |
| `clientDisplayName` | string or null | No | Presentation only |
| `sessionId` | UUID or null | No | Session identifier |
| `operation` | string | Yes | Tool name or lifecycle event |
| `target` | object or null | No | Profile ID and canonical path; redacted |
| `result` | enum | Yes | `allowed`, `denied`, `failed`, `blocked`, `completed` |
| `errorCode` | string or null | No | Stable automation or core error code |
| `durationMs` | integer or null | No | Operation duration |
| `details` | object or null | No | Non-secret context such as policy ID or revision |
| `prevHash` | string | Yes | Hash of the previous record; genesis uses 64 zeros |
| `hash` | string | Yes | SHA-256 of the canonical record excluding `hash` |

**Rules**: Every MCP operation, including denied, blocked, and failed attempts, produces a record. No secret, credential, token, private key, or raw server diagnostic appears. Destructive operations require a durable record before execution. A record is durable only after a successful append and `fsync`. The hash chain is verified on startup; a break is reported and not silently repaired.

### AuditSegment

| Field | Type | Notes |
|-------|------|-------|
| `date` | date | Segment date, `YYYY-MM-DD` |
| `path` | string | `audit/YYYY-MM-DD.jsonl` |
| `state` | enum | `open`, `sealed`, `noticePending`, `deleted` |
| `firstSeq` | integer | First record sequence |
| `lastSeq` | integer | Last record sequence |
| `recordCount` | integer | Records in the segment |
| `sealedAt` | timestamp or null | When the day ends |
| `noticeRecordedAt` | timestamp or null | When deletion notice was written |
| `deletedAt` | timestamp or null | When deletion completed |

**Rules**: Only sealed segments older than the retention period are eligible for deletion, or the oldest sealed segments when the configured storage limit is reached. A notice is written at least the configured notice period before deletion. Deletion records its own audit event before removing the segment. If the storage limit is reached and no segment can be deleted after notice, new operations are blocked with `audit_unavailable`. No MCP tool exposes segment paths or deletion.

### TransferHost (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `instanceId` | UUID | Host instance |
| `pid` | integer | Owning process |
| `startedAt` | timestamp | UTC |
| `endpoint` | string | Unix socket path or Windows named pipe name |
| `transport` | enum | `unixSocket`, `namedPipe` |
| `version` | string | Host contract version |
| `tokenHash` | string | SHA-256 of the per-instance access token |
| `ownedTransferIds` | UUID[] | Transfers currently executed by this host |
| `state` | enum | `acquiring`, `owner`, `releasing`, `released`, `lost` |

**Rules**: Exactly one host owns the lock. A CLI Transfer Session is represented by the owning `TransferHost` plus the existing `Transfer` record; it is not a separate persisted entity. The token is never logged or written in plaintext outside the restricted runtime file. A host that cannot renew or serve its endpoint is treated as lost. Runtime files are removed on clean shutdown and cleaned up when stale.

### HostControlClient (runtime only)

| Field | Type | Notes |
|-------|------|-------|
| `processId` | UUID | Client process |
| `role` | enum | `desktop`, `cli`, `mcp` |
| `connectedAt` | timestamp | UTC |
| `lastRequestAt` | timestamp | UTC |
| `subscriptions` | enum[] | `progress`, `state` |

**Rules**: Clients are advisory; the host remains authoritative. A disconnected client does not stop a transfer unless it explicitly cancels it.

### CLIInvocationResult

The machine-readable result of a CLI invocation.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `schemaVersion` | integer | Yes | Envelope schema version, currently 1 |
| `command` | string | Yes | Stable command path, for example `transfer.upload` |
| `ok` | boolean | Yes | Whether the command succeeded |
| `exitCode` | integer | Yes | Documented exit code |
| `data` | object or null | No | Command-specific result |
| `warnings` | object[] | Yes | Non-fatal warnings with stable codes |
| `error` | object or null | No | Stable code, actionable message, retryable flag, redacted details |
| `meta` | object | Yes | Duration, CLI version, automation contract version, host mode |

**Rules**: Exactly one result document appears on stdout in JSON mode. Progress and diagnostics never appear on stdout. Secrets never appear in any field. Error details are redacted.

### CLIExitCode

| Value | Meaning |
|-------|---------|
| 0 | Success |
| 2 | Usage or argument error |
| 3 | Authentication or credential rejection |
| 4 | Network, timeout, or host unreachable |
| 5 | Permission denied or local filesystem permission failure |
| 6 | Conflict, including an existing destination without a policy |
| 7 | Cancelled by the user |
| 8 | Interrupted, including host loss; work is resumable |
| 9 | Authorization or policy denial |
| 10 | Validation error, including an unsafe path |
| 11 | Credential store unavailable or locked |
| 12 | Unsupported capability, operation, or interface version |
| 13 | Internal error |

**Rules**: Codes are stable. A command returns exactly one code. Automation must be able to branch on the code without parsing prose.

## Persistence

### `automation.json`

```json
{
  "schemaVersion": 1,
  "stateRevision": 1,
  "settings": {
    "mcpEnabled": false,
    "mcpMode": "standalone",
    "loopbackPort": null,
    "auditRetentionDays": 90,
    "auditNoticeDays": 7,
    "auditMaxBytes": 1073741824,
    "enrollmentRequestTtlSeconds": 600,
    "confirmationTtlSeconds": 120,
    "maxPendingEnrollmentRequests": 10,
    "maxPendingEnrollmentRequestsPerFingerprint": 3,
    "maxEnrolledClients": 100,
    "updatedAt": "2026-11-05T00:00:00Z"
  },
  "clients": [],
  "enrollmentRequests": [],
  "grants": [],
  "directoryScopes": [],
  "approvalPolicies": []
}
```

**Storage rules**: Atomic replace with backup, restrictive permissions (`0600` on Unix), schema version, ordered migrations, and explicit quarantine on corruption. `stateRevision` is a monotonically increasing generation shared by every running automation process; every MCP request compares the revision it last loaded and reloads on change. Writes use an exclusive cross-process lock so two MCP servers cannot overwrite each other's grants. Public keys and fingerprints are not secrets. No session, confirmation, token, or private key is persisted.

### Audit store

```text
<application-data>/audit/
├── 2026-11-05.jsonl
├── 2026-11-06.jsonl
└── ...
```

**Storage rules**: Append-only, one JSON object per line, `fsync` before success, restrictive permissions, daily rotation, hash chain across segments, and a startup integrity check. Every append takes an exclusive cross-process `audit.lock` so multiple MCP servers keep a single linear hash chain. Retention or storage-limit deletion follows the notice rule.

### Runtime host store

```text
<application-data>/run/
├── host.lock        # Exclusive advisory lock
├── host.json        # Instance metadata and token hash; 0600
├── host.token       # Plaintext per-instance access token; 0600
└── control.sock     # Unix domain socket, or the named pipe endpoint
```

**Storage rules**: Removed on clean shutdown. Stale files are detected by PID liveness and endpoint connection failure. The plaintext access token is written only to `host.token`, readable by the same user, and is never logged or exposed to MCP clients.

### Existing stores

`favorites.json` (profiles, favorites, credential references, preferences, tabs) and `transfers.json` (queue, active transfers, partial data, history) remain owned by `001`/`003`. Automation code only reads and writes them through the shared core services.

## State Transitions

### Enrollment request

```text
Pending → Approved (creates a grant)
Pending → Denied
Pending → Expired
```

### Client

```text
Pending → Active (first grant approved)
Active → Revoked (all grants revoked)
Revoked → Active (new explicit enrollment and approval)
```

### Grant

```text
Active → Expired (expiry reached)
Active → Revoked (user action or client revocation)
Expired → Active (only by creating a new grant, not by editing)
```

Any scope, operation, expiry, or policy change on an active grant increments `revision` and invalidates sessions that authenticated under an older revision.

### Session

```text
Authenticating → Active
Active → Terminated (closed | revoked | expired | disabled | error)
```

### Confirmation

```text
Pending → Approved
Pending → Denied
Pending → Expired
```

### Transfer host

```text
Acquiring → Owner
Owner → Releasing → Released
Owner → Lost (process exit or endpoint failure)
Lost → Acquiring (next process after stale cleanup)
```

### Audit record

```text
Appended → Durable (fsync succeeded)
Appended → Failed (operation blocked)
```

### Audit segment

```text
Open → Sealed → NoticePending → Deleted
```

## Invariants

1. MCP is disabled by default and cannot serve tool calls until the user enables it.
2. Every MCP client has a stable fingerprint derived from its Ed25519 public key; no grant is valid for another fingerprint.
3. No MCP operation executes without an active grant covering its profile, canonical remote path, canonical local path when applicable, and operation.
4. Every path is canonicalized and containment-checked; ambiguous or unresolvable paths are denied.
5. No destructive operation executes without a single-use confirmation or an applicable, unexpired, unrevoked pre-approval policy.
6. Every MCP operation, including denials, produces a durable audit record before a result is returned; if the record cannot be written, the operation is blocked.
7. No secret, token, private key, credential, or raw server diagnostic appears in automation state, audit records, CLI output, logs, or errors.
8. Exactly one transfer host owns execution at a time; all surfaces observe the same persisted transfer state.
9. A process exit never reports an incomplete transfer as completed; owned transfers become interrupted and resumable.
10. Revocation terminates sessions and stops in-flight work at the next file boundary or before the next operation.
11. Favorites are not exposed through CLI or MCP.
12. CLI secrets come only from the native credential store or standard input, never from arguments or environment variables.
13. CLI JSON mode writes exactly one result document to stdout; progress and diagnostics go to stderr.
14. A grant's scope never implies a parent, sibling, or symlink target outside its canonical paths.
15. Audit hash-chain breaks are reported, not silently repaired.
16. Audit records are not writable or deletable through MCP tools and are tamper-evident against same-user modification.
17. Remote directory mutation tools never accept local paths; local scopes govern upload sources and download destinations.
