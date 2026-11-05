# Thriftp CLI

The `thriftp` binary is a non-interactive adapter over the shared Rust core.

## Global options

- `--json` emits one JSON result document on stdout. Diagnostics go to stderr.
- `--quiet` suppresses human-readable success output; errors still go to stderr.
- `--verbose` emits diagnostic details on stderr only.
- `--password-stdin` / `--key-passphrase-stdin` read one secret line from stdin for this invocation; they apply to profile creation and to browse, stat, upload, download, and transfer commands when no stored credential exists.
- `--profile <name-or-id>` selects a saved connection.

## Commands

```text
thriftp profiles list|show|create|update|set-credential|delete|test|trust-host-key|forget-host-key
thriftp browse <path> [--local]
thriftp stat <path> [--local]
thriftp upload <sources...> <remote-dir> [--recursive] [--conflict fail|overwrite|skip|rename] [--preserve-symlinks]
thriftp download <sources...> <local-dir> [--recursive] [--conflict fail|overwrite|skip|rename] [--preserve-symlinks]
thriftp mkdir <path> [--local] [--parents]
thriftp rename <from> <to> [--local] [--yes]
thriftp delete <paths...> [--local] [--recursive] [--yes]
thriftp transfer list|status|wait|pause|resume|cancel|retry|priority|reorder
thriftp queue list|pause|resume|cancel-all
thriftp config concurrency|bandwidth
thriftp mcp enable|disable|status|clients|tools|serve|confirm|audit
```

## Secrets

Secrets are accepted only through the native credential store or an explicit stdin flag such as `--password-stdin`. They are never accepted as arguments or environment variables and never printed.

## Plain FTP

Plain FTP connections emit warning code `unencrypted_transport` before and during use. In JSON mode the warning appears in the envelope-level `warnings` array for profile, browse, stat, upload, download, transfer status, and transfer list results.

## Transfer host

`upload` and `download` become the single transfer host when no other host exists and run to a terminal state. When another host owns the queue they delegate through the local control channel. `SIGINT` stops at the next chunk boundary, marks the transfer `interrupted`, and exits 8; a second signal exits immediately. `--no-wait` requires an existing host, returns a transfer ID, and exits 4 with guidance when no host is running.

`transfer wait` reports `host_lost` with exit 8 if the host disappears. `transfer resume` verifies partial metadata before resuming and never auto-resumes.

Mutating transfer and queue commands acquire the transfer host or delegate to the current host; they never write `transfers.json` while another process owns it. `browse --long` returns full file metadata; without it, entries are compact.

`--preserve-symlinks` preserves remote symlinks when downloading on Unix-like hosts; upload preservation requires a protocol symlink capability and otherwise fails explicitly.

## Disconnect policy

`thriftp mcp clients approve` accepts `--disconnect-policy <continue|cancel>`. The default is `continue`; the choice is stored on the grant and audited.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 2 | Usage |
| 3 | Authentication or credential rejection |
| 4 | Network, timeout, or not found |
| 5 | Permission denied |
| 6 | Conflict |
| 7 | Cancelled |
| 8 | Interrupted |
| 9 | Authorization or policy |
| 10 | Validation |
| 11 | Credential store unavailable |
| 12 | Unsupported capability |
| 13 | Internal |

## JSON envelope

```json
{
  "schemaVersion": 1,
  "command": "mcp.status",
  "ok": true,
  "exitCode": 0,
  "data": {},
  "warnings": [],
  "error": null,
  "meta": {
    "cliVersion": "0.1.0",
    "automationContractVersion": "1.0.0",
    "mcpProtocolVersions": "2026-07-28, 2025-11-25",
    "hostMode": "local"
  }
}
```

A broken stdout pipe never panics; the CLI exits 7 when the result cannot be delivered.
