# Contract: Transfer Host and Shared State

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

This contract defines how the desktop application, CLI, and MCP server coordinate transfer execution. Exactly one process at a time is the **transfer host**; all other processes are **control clients**. The host owns the transfer engine, the queue, and partial-file state. Control clients observe and control transfers through a local-only channel. No always-running daemon is introduced.

## Roles

| Role | Responsibilities |
|------|------------------|
| Transfer host | Holds the exclusive lock, runs the queue and transfer engine, persists state, serves the control endpoint, publishes events |
| Control client | Connects to the host, submits and controls transfers, observes events, never writes transfer state directly |
| Observer | A control client that only reads status, such as the desktop application when the CLI is host |

The desktop application, CLI, and MCP server can each act as host or control client depending on which process owns the lock.

## Ownership Rules

1. The host acquires an exclusive advisory lock on `run/host.lock` in the OS application-data directory.
2. A process that needs to execute transfers tries to acquire the lock. On success it becomes host, writes `run/host.json` and `run/host.token`, and starts the control endpoint.
3. On failure, it reads the published endpoint and token, connects as a control client, and delegates.
4. Exactly one host exists at a time. No process writes `transfers.json` unless it is the host.
5. The desktop application tries to become host on launch. If another host exists, it connects as a control client and displays the shared state.
6. The CLI becomes host only when no host exists and the command must execute or control a transfer. Otherwise it delegates.
7. The standalone MCP server becomes host only when no host exists; embedded MCP always uses the desktop host.
8. A host releases the lock on clean shutdown after notifying clients and persisting final state.
9. Read-only commands (`transfer list`, `transfer status`, `queue list`) use the host when present and otherwise read the persisted state. Mutating commands start a host when none exists, except `--no-wait`, which requires an existing host.

## Runtime Files

```text
<application-data>/run/
├── host.lock        # Exclusive advisory lock; no content requirements
├── host.json        # Instance metadata; restrictive permissions
├── host.token       # Per-instance access token; 0600 on Unix
└── control.sock     # Unix domain socket, or the named pipe endpoint
```

`host.json` contains:

```json
{
  "instanceId": "0f8fad5b-d9cb-469f-a165-70867728950e",
  "pid": 4242,
  "startedAt": "2026-11-05T00:00:00Z",
  "endpoint": "/…/run/control.sock",
  "transport": "unixSocket",
  "version": "1.0.0",
  "tokenHash": "…",
  "ownedTransferIds": []
}
```

**Rules**: The token is a random 256-bit value, written only to `host.token` with same-user permissions and never logged. `host.json` stores only its hash. All runtime files are removed on clean shutdown. Stale files are ignored when the lock is free and cleaned up by the next host.

## Control Channel

- Transport: Unix domain socket on macOS/Linux; named pipe on Windows.
- Encoding: newline-delimited JSON-RPC 2.0. Each message is one line of UTF-8 JSON, capped at 1 MiB.
- Authentication: every request carries `auth` with the value from `host.token`. The host compares it in constant time and rejects missing or invalid tokens.
- No TCP port is opened. No remote address can connect.
- The host enforces request timeouts, rejects malformed or oversized messages without crashing, and limits concurrent control connections.

### Request shape

```json
{
  "jsonrpc": "2.0",
  "id": 17,
  "method": "transfers.submit",
  "auth": "<host token>",
  "params": {
    "direction": "upload",
    "profileId": "…",
    "sources": ["/local/dist"],
    "destination": "/srv/app",
    "recursive": true,
    "conflictPolicy": "fail",
    "priority": "normal"
  }
}
```

### Methods

| Method | Parameters | Result |
|--------|------------|--------|
| `host.status` | none | Instance ID, PID, version, start time, mode, owned transfer count |
| `transfers.list` | `state?`, `cursor?`, `limit?` | Transfer summaries and next cursor |
| `transfers.status` | `transferId` | Full transfer state, progress, error, resumability |
| `transfers.submit` | `direction`, `profileId`, `sources[]`, `destination`, `recursive`, `conflictPolicy`, `priority`, `clientRole` | `transferId`, initial state |
| `transfers.pause` | `transferId` | Updated state |
| `transfers.resume` | `transferId` | Updated state and resume verification result |
| `transfers.cancel` | `transferId` | Updated state and preserved offset |
| `transfers.priority` | `transferId`, `priority` | Updated priority |
| `transfers.reorder` | `transferIds[]` | New order |
| `queue.get` | none | Concurrency, bandwidth, paused state, ordered IDs |
| `queue.setConcurrency` | `concurrency` | Updated queue |
| `queue.setBandwidth` | `bytesPerSecond` or null | Updated queue |
| `queue.pause` | none | Updated queue |
| `queue.resume` | none | Updated queue |
| `queue.cancelAll` | `confirmed: true` | Cancelled transfer IDs and preserved offsets |
| `events.subscribe` | `topics[]` | Subscription confirmation; subsequent notifications |

### Events

The host emits notifications:

- `event.transferProgress`: transfer ID, confirmed bytes, total bytes when known, speed, and ETA.
- `event.transferState`: transfer ID, previous state, new state, error code when applicable.
- `event.queueChanged`: queue limits, pause state, and order.
- `event.hostShutdown`: reason and expected interruption time.

Events are advisory. Persisted state is authoritative; a client that misses an event must re-query.

### Error mapping

| Host error | Meaning |
|------------|---------|
| `host_unavailable` | No endpoint or connection failed |
| `host_lost` | The host exited during the request |
| `unauthorized` | Missing or invalid token |
| `not_found` | Unknown transfer |
| `conflict` | Destination conflict or resume mismatch |
| `invalid_argument` | Invalid parameters |
| `unsupported_capability` | Protocol cannot perform the operation |
| `internal_error` | Unexpected failure with a redacted message |

## Process Lifetime and Handoff

### CLI as host

- A CLI transfer command with the default wait behavior becomes host when none exists, runs the transfer to a terminal state, persists the result, releases the lock, and exits.
- If the CLI process exits early (signal, crash, or user interrupt), the OS releases the lock. Transfers it owned are recovered as `interrupted` at the last confirmed offset; they are never reported as completed.
- `SIGINT` stops the transfer at the next chunk boundary, marks it `interrupted`, preserves the offset, and exits with code 8. A second signal exits immediately without claiming completion.
- `--no-wait` requires an existing host. If none exists, the command fails with `host_unavailable` and guidance to run without `--no-wait`; the CLI does not start an undocumented background host.

### Desktop as host

- The desktop application tries to acquire ownership on launch and normally acts as host while running.
- CLI and MCP requests are delegated to the desktop host and share its queue and transfer state.
- On quit with active transfers, the desktop prompts the user to wait or cancel its own transfers. Transfers submitted by other surfaces that cannot finish within the quit grace period are marked `interrupted` and reported as resumable; they are never marked complete.
- The desktop notifies control clients with `event.hostShutdown` before releasing the lock.

### Desktop as observer

- When the CLI or standalone MCP server is host, the desktop connects as a control client.
- Closing the desktop in observer mode has no effect on transfers.
- The desktop displays the same queue, progress, and terminal states as the host.

### MCP as host or client

- Embedded MCP always delegates to the desktop host.
- Standalone MCP becomes host when none exists and keeps ownership for its process lifetime. If the MCP client disconnects and the process exits, active transfers follow the grant's `disconnectPolicy` (`continue` by default) and are persisted as `interrupted` if they cannot continue.
- A standalone MCP server that is only a control client never writes transfer state directly.

### Host loss

When a control client detects host loss:

1. It stops waiting and receives or synthesizes `host_lost`.
2. It reads the persisted transfer state; active records have been recovered to `interrupted`.
3. The CLI reports `interrupted` with exit code 8. It does not auto-resume.
4. `thriftp transfer resume <id>` starts a new host if needed and resumes after verification.
5. Queued but not started transfers remain pending and do not auto-start.

When a process finds a stale lock or unreachable endpoint:

- If the lock can be acquired, it cleans stale runtime files and becomes host.
- If the lock is held by a live process but the endpoint is unresponsive, it reports `host_unavailable` and does not steal ownership.
- A live PID with a missing endpoint is treated as a starting or stopping host and retried with a bounded timeout.

## Shared State Guarantees

- `transfers.json` is written only by the host and is the authoritative queue and history store.
- Every state transition is persisted before it is reported.
- Control clients see a consistent view through the host; they never merge local copies.
- A control client that disconnects does not stop transfers unless it explicitly cancels them.
- A transfer has exactly one owning host. Reassignment happens only after the previous host is confirmed lost.
- The desktop, CLI, and MCP server present the same state for the same transfer.

## Security Boundary

- The control channel is local-only and same-user. It is not a sandbox boundary.
- The host token protects against accidental cross-process access and is never exposed to MCP clients.
- MCP authorization is enforced in the MCP adapter before any host request. The host also validates all core rules, so a malformed request cannot bypass scope, conflict, or resume safety.
- A same-user process with filesystem access can read the token file or invoke the CLI directly. Thriftp cannot prevent this without OS sandboxing; MCP grants constrain the MCP surface, not the operating-system user. This limitation is documented in `docs/automation-security.md`.
- No file-transfer credential, private key, or MCP client secret is stored in runtime host files. The host token is an access credential for the local channel only.

## Recovery

1. On startup, a process tries the lock.
2. If acquired, it reads `transfers.json`, converts stale `active` records to `interrupted`, preserves `pending` and `paused`, and removes stale runtime files.
3. It verifies partial-file metadata and leaves resume verification to the normal transfer engine.
4. It starts the endpoint and publishes `host.json` and `host.token`.
5. If the state store is corrupt, it recovers from the backup or reports a safe state without claiming completion.

## Test Contract

- Exactly one of several concurrently started processes acquires ownership.
- A control client delegates submission and observes progress and terminal state.
- The desktop and CLI report identical state for the same transfer.
- Killing the host marks owned active transfers interrupted and never completed.
- Killing the CLI while it is host releases the lock and allows recovery.
- Killing the desktop while it is host notifies clients and preserves resumability.
- A stale lock with no live process is cleaned up.
- A live owner with an unresponsive endpoint is not stolen.
- `--no-wait` without a host fails with guidance and starts no background process.
- Invalid tokens, malformed JSON, oversized messages, and unknown methods are rejected without crashing.
- Cross-platform tests cover Unix domain sockets and Windows named pipes.

## Contract Invariants

1. Exactly one transfer host owns execution at a time.
2. Only the host writes transfer state.
3. The control channel is local-only and requires the per-instance token.
4. A control client never merges or overwrites host state.
5. Process exit never reports an incomplete transfer as completed.
6. Host loss produces interrupted, resumable state and a distinct report.
7. No always-running daemon or undocumented background service is started.
8. Desktop, CLI, and MCP present consistent state for the same transfer.
9. No secret or MCP client credential appears in runtime host files.
10. MCP authorization is applied before host submission and cannot be bypassed by the host protocol.
