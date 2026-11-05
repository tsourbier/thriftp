# Contract: Transfer Engine

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

This contract defines the Tauri-independent transfer engine used by the desktop UI and reusable by future CLI/MCP interfaces.

## Responsibilities

- Own the transfer queue, concurrency, priority, bandwidth, pause/resume, cancellation, conflict policy, persistence checkpoints, and history.
- Stream file data in fixed-size chunks with constant memory.
- Emit progress and state events without flooding the UI.
- Verify resume safety before continuing an interrupted or paused transfer.
- Never depend on Tauri, Svelte, or platform credential-store types.

## States

```text
Pending → Active → Completed
Pending → Active → Paused → Active
Pending → Active → Cancelled
Pending → Active → Failed
Pending → Active → Interrupted (unexpected termination)
Pending → Cancelled
Paused → Cancelled
Interrupted → Active (resume) | Cancelled
```

State rules:
- `Pending`: queued, not consuming bandwidth.
- `Active`: consuming a concurrency slot.
- `Paused`: preserving progress, consuming no transfer bandwidth, not counting against concurrency.
- `Completed`: 100% of known total, or the stream ended successfully when total is unknown.
- `Failed`: terminal error with last confirmed progress preserved.
- `Cancelled`: user-requested stop with last confirmed progress preserved.
- `Interrupted`: unexpected termination; resumable only after verification.

## Queue

- Default concurrency: 4, configurable.
- Ordering: priority (`high` > `normal` > `low`), then explicit order, then creation time.
- Users may reorder queued transfers and change priority. Active transfers are not preempted.
- Pausing the queue pauses active transfers at chunk boundaries and prevents pending starts.
- Cancelling all requires explicit confirmation and preserves each transfer's last confirmed progress.

## Bandwidth

- A single global token bucket enforces `bandwidthLimitBytesPerSecond`; `null` means unlimited.
- Active transfers share the bucket. Lowering the limit affects active transfers; raising it takes effect at the next refill.
- Bandwidth accounting covers payload bytes, not protocol overhead.
- A paused transfer consumes no tokens.

## Chunking and Memory

- File data is read and written in fixed-size chunks (64 KiB in the reference implementation).
- No transfer buffers an entire file.
- Local and remote readers/writers expose a confirmed offset after each successful chunk.
- Progress is persisted at state transitions and checkpointed every 5 seconds or 1 MB, whichever comes first.

## Pause, Cancel, and Failure

- Pause and cancel use cooperative cancellation checked at chunk boundaries.
- Pause persists the confirmed offset and moves to `paused`.
- Cancel stops the transfer, preserves the confirmed offset, and marks `cancelled`.
- Failure stops the transfer, preserves the confirmed offset, records a redacted error, and marks `failed`.
- Partial local data is retained in an identifiable incomplete state; remote cleanup occurs only when the protocol supports it and the user allows it.

## Resume Safety

Before resuming, the engine verifies:

1. The source still exists.
2. Source size and modification time match the values recorded when the transfer was paused or interrupted, where available.
3. The destination partial data matches the recorded confirmed offset.
4. The protocol and server advertise resumability.

If verification fails, the engine returns a conflict requiring the user to restart or cancel. It never appends silently to stale data.

## Conflict Resolution

- When a destination exists, the engine requests a decision through a callback supplied by the caller.
- Options: overwrite, skip, rename, apply-to-all remaining.
- The default without an explicit decision is to block, not overwrite.
- Apply-to-all is scoped to the current operation.
- The same core policy is used by desktop, CLI, and MCP.

## Recursive Transfers and Symbolic Links

- Directory traversal enumerates entries in batches.
- Symbolic links are never followed.
- Links are preserved when the protocol and destination support it; otherwise they are skipped with a visible report.
- Name collisions during recursion use the same conflict policy.

## Persistence

- Queue and history are stored in `transfers.json`.
- Persist on every state transition and on checkpoint.
- On load: `active` → `interrupted`; `pending` stays `pending` and does not auto-start; `paused` stays `paused`.
- History retains the 1,000 most recent terminal records; users can clear it.
- Atomic writes and a backup protect against corruption.
- No secret is written to this store.

## Progress Events

- Emit progress at most every 100 ms or every 1 MB, whichever comes first.
- Include transfer ID, confirmed bytes, total bytes when known, speed, and estimated remaining time.
- Emit state-change events immediately.
- Events are advisory; persisted state is authoritative.

## Testing Hooks

- A fake `RemoteFs` can inject latency, partial reads/writes, failures, unsupported capabilities, and resume mismatches.
- Tests use a virtual clock for bandwidth, checkpoint, and progress cadence.
- Tests cover pause/resume, queue pause, priority order, bandwidth caps, cancellation, conflict policies, symlink handling, interrupted recovery, history eviction, and O(1) memory behavior for large files.

## Invariants

1. No transfer buffers an entire file.
2. A completed transfer has complete progress; interrupted/failed transfers preserve last confirmed progress.
3. No transfer resumes without source and destination verification.
4. No destination is overwritten without an explicit conflict decision.
5. Paused transfers consume no bandwidth and do not occupy concurrency slots.
6. The queue and history survive restart with the documented state mapping.
7. No secret appears in progress events, persisted state, logs, or errors.
