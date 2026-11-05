# Transfer Process Model

Exactly one process at a time is the **transfer host**. The host owns the queue,
the transfer engine, partial-file state, and the local control endpoint. CLI,
desktop, and MCP are otherwise control clients or observers.

## Runtime files

```text
<application-data>/run/
├── host.lock        # Exclusive single-owner lock (PID)
├── host.json        # Instance metadata, endpoint, mode, token hash
├── host.token       # Per-instance access token (0600 on Unix)
└── control.sock     # Unix domain socket (loopback TCP on Windows)
```

Stale lock files from dead processes are detected by PID liveness and cleaned
up by the next owner. A live owner is never stolen. All runtime files are
removed on clean shutdown.

## Control channel

The endpoint accepts newline-delimited JSON-RPC 2.0 with a 1 MiB message cap.
Every request carries the per-instance token and is compared in constant time.
Methods include `host.status`, `transfers.list`, `transfers.status`,
`transfers.submit`, `transfers.pause`, `transfers.resume`, `transfers.cancel`,
`transfers.priority`, `transfers.reorder`, `queue.get`, `queue.setConcurrency`,
`queue.setBandwidth`, `queue.pause`, `queue.resume`, `queue.cancelAll`, and
`events.subscribe`. The host emits advisory progress and state events, including
`event.hostShutdown`.

## Shared state

Connection profiles live in `metadata.json`; queue and transfer records live in
`transfers.json`. Only the host writes transfer state. Every transition is
persisted before it is reported, so all surfaces observe the same state.

## CLI

`upload` and `download` become host when none exists and run to a terminal
state. When another host owns the queue, the CLI delegates submission. `SIGINT`
stops at the next chunk boundary, marks the transfer `interrupted`, preserves
the offset, and exits 8. A second signal exits immediately. `--no-wait` requires
an existing host and never starts a background daemon.

## Desktop and MCP

The desktop normally becomes host on launch and operates as an observer when a
CLI or standalone MCP host exists; closing it as an observer does not affect
transfers. Embedded MCP is started by the desktop when the user enables
`embedded` or `both` mode. Active transfer lists, status, controls, and queue
settings are delegated to the host when the desktop is an observer.

## Host loss and recovery

On startup the new owner converts stale `active` transfers to `interrupted`,
preserves `pending` and `paused` transfers and partial-file metadata, and
publishes a fresh endpoint and token. `transfer wait` reports `host_lost` with
exit 8 and never auto-resumes. `transfer resume` verifies source size,
modification metadata, and the destination offset before continuing. A queued
transfer that never started stays pending and does not auto-start.

Subscribed control clients receive `event.transferProgress`,
`event.transferState`, `event.queueChanged`, and `event.hostShutdown`
notifications. Persisted state remains authoritative.

`thriftp mcp clients disconnect` and the desktop disconnect action persist a
cross-process request that the MCP server drains before its next authorization
decision, terminating the session and applying the grant's disconnect policy.
