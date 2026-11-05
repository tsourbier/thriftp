# Contract: MCP Server and Tools

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

The `thriftp-mcp` server exposes Thriftp's core file-transfer capabilities to explicitly enrolled MCP clients. It is local-only, disabled by default, and never bypasses core security, credential, certificate, host-key, or transfer rules. All tool calls are thin translations into `thriftp-core` services.

## Protocol Revisions

| Revision | Status | Behavior |
|----------|--------|----------|
| `2026-07-28` | Primary | Stateless requests with per-request version and capabilities in `_meta`; `server/discover` is implemented; no initialize handshake |
| `2025-11-25` | Legacy | Initialize-handshake semantics, served through the dual-era path |
| Anything else | Unsupported | Structured `UnsupportedProtocolVersionError` with the supported list |

The server reports its supported revisions through `server/discover` and legacy `initialize`. An incompatible client receives a clear compatibility error rather than undefined behavior. The server identifies itself with `serverInfo` containing its name, semantic version, and the Thriftp automation contract version.

## Transports

| Mode | Transport | Binding | Authentication |
|------|-----------|---------|----------------|
| Standalone | stdio | Process stdin/stdout | Thriftp enrollment plus signed session proof |
| Embedded | Streamable HTTP | `127.0.0.1` on an ephemeral port | Loopback bearer token plus Thriftp enrollment and signed session proof |

**Rules**: Stdout is reserved for protocol messages; help, logs, and diagnostics go to stderr. The embedded endpoint refuses wildcard or non-loopback binds, rejects requests with a non-loopback `Host` or `Origin` when present, and uses a per-launch bearer token stored with restrictive permissions. There is no remote transport, service discovery, or remote authorization flow in the initial release.

## Discovery and Bootstrap

### Discovery

- `server/discover` returns supported protocol versions, capabilities (`tools`), `serverInfo`, and `dev.thriftp/automationContractVersion`.
- Legacy `initialize` returns the negotiated protocol version, capabilities, and the same server information.
- `tools/list` is available before enrollment and returns the full catalog with names, descriptions, input and output schemas, and destructive flags. Listing tools grants no access.
- `tools/call` is available only for the bootstrap tools before session establishment. All other tools require an active session and grant.

### Bootstrap tools

| Tool | Purpose | Requires session |
|------|---------|------------------|
| `thriftp.automation.status` | Reports MCP enabled state, server version, contract version, supported protocol revisions, and enrollment availability | No |
| `thriftp.automation.enroll` | Submits a client public key, fingerprint, and display name as an enrollment request | No |
| `thriftp.automation.enrollment_status` | Reports whether a request is pending, approved, denied, or expired | No |
| `thriftp.automation.challenge` | Returns a single-use nonce for an enrolled client | No |
| `thriftp.automation.authenticate` | Verifies the signed nonce and returns a session token and grant summary | No |

**Enrollment flow**: The client generates an Ed25519 keypair, calls `enroll`, and waits for user approval through the desktop control surface or the CLI. The request expires after the configured TTL. Approval binds a grant to the key fingerprint. The client then requests a challenge, signs it, and authenticates.

**Session flow**: A successful `authenticate` returns an opaque session token, its expiry, the grant revision and epoch, and the granted profiles, scopes, and operations. Every subsequent `tools/call` includes `_meta` `{"dev.thriftp/sessionToken": "<token>"}`. Tokens are bound to the client fingerprint and transport, expire after 15 minutes of inactivity or 8 hours absolute, and are invalidated by revocation, grant revision changes, expiry, or MCP disablement. Re-authentication is required after invalidation.

## Tool Catalog

All operational tools require an active session. Tool names are stable and namespaced. Inputs and outputs use JSON Schema 2020-12.

### Connections

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.connections.list` | none | Granted profiles: ID, display name, protocol, host, port; never credentials | No |
| `thriftp.connections.get` | `profileId` | One granted profile without secrets | No |

### Browsing and Metadata

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.browse.list` | `profileId`, `path`, `cursor?`, `limit?`, `includeHidden?` | Canonical path, entries, `nextCursor?`; streams and paginates | No |
| `thriftp.files.stat` | `profileId`, `path` | Entry metadata and capabilities | No |

### Directory and File Operations

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.directories.create` | `profileId`, `path`, `recursive?` | Created canonical path | No |
| `thriftp.directories.rename` | `profileId`, `fromPath`, `toPath`, `overwrite?` | New canonical path | Yes when the destination exists |
| `thriftp.directories.delete` | `profileId`, `path`, `recursive?` | Deleted path and affected count | Yes |
| `thriftp.files.delete` | `profileId`, `path` | Deleted path | Yes |

### Transfers

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.transfers.upload` | `profileId`, `localPaths[]`, `remoteDir`, `recursive?`, `conflictPolicy?`, `wait?`, `timeoutMs?` | `transferId`, state, bytes when available | Yes when overwriting |
| `thriftp.transfers.download` | `profileId`, `remotePaths[]`, `localDir`, `recursive?`, `conflictPolicy?`, `wait?`, `timeoutMs?` | `transferId`, state, bytes when available | Yes when overwriting |
| `thriftp.transfers.list` | `state?`, `cursor?`, `limit?` | Transfers visible to the grant | No |
| `thriftp.transfers.status` | `transferId` | State, bytes, speed, ETA, error, resumability | No |
| `thriftp.transfers.pause` | `transferId` | Updated state | No |
| `thriftp.transfers.resume` | `transferId` | Updated state and resume verification result | No |
| `thriftp.transfers.cancel` | `transferId` | Updated state and preserved offset | No |
| `thriftp.transfers.priority` | `transferId`, `priority` (`low`, `normal`, `high`) | Updated priority | No |
| `thriftp.transfers.reorder` | `transferIds[]` | New queue order | No |

**Long-running behavior**: Upload and download enqueue work and return a `transferId` immediately when `wait` is false or omitted. When `wait` is true, the call blocks up to `timeoutMs` (default 60,000; maximum 600,000) and returns the current state; a timeout returns `state: "running"` with the transfer ID rather than an error. Progress notifications are emitted when the client supports them, and `thriftp.transfers.status` is always available for polling. A client disconnect does not cancel a transfer by default; the grant's `disconnectPolicy` (`continue` or `cancel`) determines the outcome, and the outcome is audited.

**Transfer rules**: Every local upload source and local download destination must be inside a granted local directory scope; a path outside it returns `scope_denied`. The default conflict policy is `fail`; an existing destination without an explicit policy returns `conflict` and is never overwritten silently. `overwrite` is destructive and requires confirmation or an applicable policy. Recursive transfers preserve structure, never follow symbolic links, and skip links with a visible warning when the protocol cannot preserve them. Interruption by host loss preserves the confirmed offset and returns `host_lost`; the transfer is resumable through `thriftp.transfers.resume`.

### Queue and Limits

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.queue.get` | none | Concurrency, bandwidth limit, paused state, ordered IDs | No |
| `thriftp.queue.set_concurrency` | `concurrency` (1–64) | Updated queue settings | No |
| `thriftp.queue.set_bandwidth` | `bytesPerSecond` or `null` for unlimited | Updated queue settings | No |

### Session

| Tool | Input | Output | Destructive |
|------|-------|--------|-------------|
| `thriftp.automation.session` | none | Current client identity, grant revision, epoch, expiry, scopes, and operations | No |

**Excluded tools**: favorites management, arbitrary shell commands, server administration, audit modification or deletion, remote client management, and any operation outside the granted profiles, scopes, and operation set.

## Tool Result Envelope

Tool results use MCP structured content plus a text summary. The structured payload is:

```json
{
  "schemaVersion": "1.0.0",
  "ok": true,
  "data": {},
  "warnings": [],
  "error": null
}
```

On a domain failure, the tool returns `isError: true` and a structured error:

```json
{
  "schemaVersion": "1.0.0",
  "ok": false,
  "data": null,
  "warnings": [],
  "error": {
    "code": "scope_denied",
    "message": "The requested path is outside the granted directory scope.",
    "retryable": false,
    "details": {
      "profileId": "…",
      "path": "/srv/other"
    }
  }
}
```

**Rules**: JSON-RPC errors are reserved for protocol problems such as malformed requests or unsupported versions. Domain failures use `isError: true` with the envelope. Messages are actionable and contain no secrets or raw server diagnostics. Unknown fields must be ignored by clients.

## Error Codes

| Code | Meaning | Retryable |
|------|---------|-----------|
| `mcp_disabled` | MCP is not enabled | No |
| `client_not_enrolled` | No approved identity exists | No |
| `grant_missing` | No active grant covers the request | No |
| `grant_expired` | The grant expired | No |
| `grant_revoked` | The grant was revoked | No |
| `scope_denied` | Profile or canonical path is outside the grant | No |
| `path_ambiguous` | The path cannot be resolved unambiguously | No |
| `confirmation_required` | A destructive operation needs confirmation or policy | Yes, after approval |
| `confirmation_expired` | The confirmation expired | Yes, by requesting a new one |
| `audit_unavailable` | A durable audit record cannot be written | Yes, after the audit store recovers |
| `host_unavailable` | No transfer host is reachable | Yes |
| `host_lost` | The transfer host exited during the request | Yes, by resuming |
| `unsupported_protocol_version` | The requested MCP revision is not supported | No |
| `invalid_argument` | Input validation failed | No |
| `not_found` | The profile, path, or transfer does not exist | No |
| `conflict` | Destination conflict or resume mismatch | Depends on the conflict policy |
| `permission_denied` | Remote or local permission failure | No |
| `network_error` | Network, timeout, or DNS failure | Yes |
| `unsupported_capability` | The protocol or server does not support the operation | No |
| `internal_error` | Unexpected failure with a redacted message | Maybe |

## Authorization and Audit Order

Every operational `tools/call` follows this order:

1. Confirm MCP is enabled and the transport is allowed; reload if the shared `stateRevision` changed.
2. Validate the session token, client fingerprint, grant state, revision, epoch, and expiry.
3. Confirm the operation is granted.
4. Confirm the profile is granted.
5. Canonicalize the requested path and verify containment in a granted directory scope.
6. For destructive operations, require an unexpired confirmation bound to the session, operation, and target, or an applicable unexpired pre-approval policy. If neither exists, return `confirmation_required` and create a pending confirmation when a user channel is available; otherwise deny.
7. Append and `fsync` the audit record. For destructive operations, the authorization record is durable before execution. If the record cannot be written, return `audit_unavailable` and do not execute.
8. Execute through the core.
9. Append the outcome record for non-destructive operations, or the completion record for destructive operations. If an outcome record cannot be written after a non-destructive failure, return `audit_unavailable` rather than a success.
10. Return the structured result.

Denials and blocks are audited with `result: denied` or `result: blocked`. Revocation and expiry are checked again at each file boundary for recursive or multi-file operations.

## Pagination

- `limit` defaults to 200 and is capped at 1,000 for directory and transfer listings.
- `cursor` is opaque and bound to the query; an invalid cursor returns `invalid_argument`.
- Listing results never buffer the complete directory or transfer history.
- `nextCursor` is omitted when no further results exist.

## Versioning and Compatibility

- The automation contract version follows semantic versioning and is independent of the application version.
- Additive tools and fields are minor versions; removing or changing behavior is a major version.
- Deprecations are documented and retained for at least one minor release.
- `server/discover` and every tool result include the contract version.
- An unsupported MCP revision receives `UnsupportedProtocolVersionError` listing the supported revisions.

## Contract Invariants

1. MCP is disabled by default and cannot serve operational tool calls until enabled.
2. No tool call executes without an enrolled client, an active grant, a valid session, and canonical in-scope remote and local paths.
3. Every MCP operation, including denials, produces a durable audit record before a result is returned.
4. No destructive operation executes without confirmation or an applicable policy.
5. No secret, token, private key, credential, or raw server diagnostic appears in any tool result, log, or audit record.
6. The standalone and embedded modes expose the same tools and enforce the same rules.
7. Long transfers return a transfer ID and never block unboundedly.
8. Favorites, shell commands, server administration, and audit modification are never exposed.
9. Unsupported protocol revisions receive a clear compatibility error.
10. A client disconnect follows the grant's disconnect policy and is audited.
