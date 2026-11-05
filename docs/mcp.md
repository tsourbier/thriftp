# Thriftp MCP Server

Thriftp exposes a local-only Model Context Protocol server over standard input/output or a loopback-only HTTP endpoint.

## Transports

- `thriftp-mcp` (default) uses stdio. stdout carries protocol messages only; diagnostics go to stderr.
- `thriftp-mcp --transport loopback [--port 0] [--token-file PATH]` binds `127.0.0.1` only, publishes a per-launch bearer token in `run/mcp.token` with `0600` permissions, and rejects wildcard or non-loopback binds and non-loopback `Host`/`Origin` headers. The server refuses to start while MCP is disabled.

## Protocol versions

- `2026-07-28` — advertised through `server/discover`.
- `2025-11-25` — legacy initialize handshake.

The automation contract version is `1.0.0`.

## Enable and disable

MCP is disabled by default. Enable it explicitly:

```bash
thriftp mcp enable --mode standalone
thriftp mcp status
```

Disable it with `thriftp mcp disable`.

## Enrollment and grants

1. The client generates an Ed25519 keypair and calls `thriftp.automation.enroll` with its base64 public key and display name.
2. A user approves the pending request with `thriftp mcp clients approve`.
3. The client calls `thriftp.automation.challenge`, signs the nonce, and calls `thriftp.automation.authenticate`.
4. The returned session token is sent in `_meta` as `dev.thriftp/sessionToken` for operational tools.

Approval selects profiles, remote directory scopes, local directory scopes for uploads/downloads, operations, an optional expiry, and a disconnect policy (`continue` by default or `cancel`).

## Tools

- `thriftp.connections.list`, `thriftp.connections.get`
- `thriftp.browse.list`, `thriftp.files.stat`
- `thriftp.directories.create`, `thriftp.directories.rename`, `thriftp.directories.delete`, `thriftp.files.delete`
- `thriftp.transfers.upload`, `thriftp.transfers.download`, `thriftp.transfers.list`, `thriftp.transfers.status`, `thriftp.transfers.pause`, `thriftp.transfers.resume`, `thriftp.transfers.cancel`, `thriftp.transfers.priority`, `thriftp.transfers.reorder`
- `thriftp.queue.get`, `thriftp.queue.set_concurrency`, `thriftp.queue.set_bandwidth`
- `thriftp.automation.status`, `thriftp.automation.enroll`, `thriftp.automation.enrollment_status`, `thriftp.automation.challenge`, `thriftp.automation.authenticate`, `thriftp.automation.session`

Every operational call is authorized against the client grant and canonical scopes. Destructive calls require a user-issued single-use `confirmationId` or an applicable scoped pre-approval policy; agent-supplied `confirmed: true` is not accepted. Without either, the call fails closed with `confirmation_required`. Users issue a confirmation with:

```bash
thriftp mcp confirm <fingerprint> --operation files.delete --target /srv/app/file --ttl 120
```

Every tool advertises JSON Schema 2020-12 input and tool-specific output schemas through `tools/list` and declares a destructive hint.

## Audit

Every tool call is appended to `audit/YYYY-MM-DD.jsonl` with a hash chain and fsynced before the result is returned. When the audit store is unavailable, the operation is blocked with `audit_unavailable`. Inspect it with:

```bash
thriftp mcp audit list
thriftp mcp audit verify
thriftp mcp audit retention 90
thriftp mcp audit limit 1073741824
```

Retention honors the seven-day notice window, deletes only sealed segments, and writes a checkpoint before deletion so the remaining chain still verifies. If the configured storage limit cannot be satisfied by deleting sealed segments, operations fail closed with `audit_unavailable`.

MCP is local-only. No remote network transport is enabled.
