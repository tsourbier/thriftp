# Contract: Command-Line Interface

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

The `thriftp` binary is a non-interactive, scriptable adapter over `thriftp-core`. It exposes connection management, browsing, local and remote directory operations, transfers, queue control, MCP administration, and audit access. It contains no protocol logic and never bypasses core security rules.

## Global Options

| Option | Values | Default | Behavior |
|--------|--------|---------|----------|
| `--output <mode>` | `human`, `json` | `human` | Selects human-readable or machine-readable output |
| `--json` | flag | off | Shorthand for `--output json` |
| `--profile <id-or-name>` | string | none | Selects a saved connection profile for remote operations |
| `--no-color` | flag | off | Disables ANSI color |
| `--quiet` | flag | off | Suppresses progress and non-error diagnostics on stderr |
| `--verbose` | flag | off | Adds diagnostic detail on stderr without secrets |
| `--timeout <duration>` | duration | command-specific | Bounds a command's wait for a terminal state |
| `--help` | flag | — | Prints command help to stdout and exits 0 |
| `--version` | flag | — | Prints version and interface version to stdout and exits 0 |

`--profile` applies to commands that address a remote filesystem. A command that requires a profile and receives none, or receives an unknown profile, fails with exit code 2.

## Command Tree

### Profiles

```text
thriftp profiles list
thriftp profiles show <id-or-name>
thriftp profiles create --name <name> --protocol <ftp|ftps-explicit|ftps-implicit|sftp>
                        --host <host> --port <port> --username <user>
                        [--group <label>] [--auth <password|ssh-key>]
                        [--key-file <path>] [--password-stdin | --key-passphrase-stdin]
thriftp profiles update <id-or-name> [--name <name>] [--host <host>] [--port <port>]
                        [--username <user>] [--group <label>] [--timeout <seconds>]
thriftp profiles set-credential <id-or-name> (--password-stdin | --key-passphrase-stdin)
thriftp profiles delete <id-or-name> --yes
thriftp profiles test <id-or-name>
thriftp profiles trust-host-key <id-or-name> --fingerprint <sha256> --yes
thriftp profiles forget-host-key <id-or-name> --yes
```

**Rules**: `--key-file` names a private-key path; it is not secret material. Secret values come only from standard input through the explicit stdin flags. `profiles delete` removes the profile and its credential reference only when no other favorite references it, and requires `--yes`. If a desktop favorite references the profile, deletion warns that the favorite will become unavailable and requires explicit confirmation; it never deletes a favorite silently. `profiles test` verifies connectivity and certificate or host-key rules without transferring data. Certificate validation is never bypassed from the CLI. A new or changed SSH host key fails the connection; `profiles trust-host-key` is the only override, requires the exact fingerprint, displays the fingerprint and a warning, and requires `--yes`. `profiles forget-host-key` removes a trusted key. Neither command is implied by any other option.

### Browsing and Metadata

```text
thriftp browse <path> [--profile <id-or-name>] [--all] [--long]
                      [--limit <n>] [--cursor <cursor>] [--local]
thriftp stat <path> [--profile <id-or-name>] [--local]
```

**Rules**: `browse` streams entries and supports cursor pagination for large directories. `--all` includes hidden entries. `--long` adds size, modification time, permissions, ownership, and link target when available. `--local` targets the local filesystem and does not require a profile. The result includes the canonical path, entry count when known, and a next cursor when more entries exist.

### Transfers

```text
thriftp upload <local-path>... <remote-dir> --profile <id-or-name>
               [--recursive] [--conflict <fail|overwrite|skip|rename>]
               [--no-wait] [--timeout <duration>]
thriftp download <remote-path>... <local-dir> --profile <id-or-name>
                 [--recursive] [--conflict <fail|overwrite|skip|rename>]
                 [--no-wait] [--timeout <duration>]
thriftp transfer list [--state <state>] [--limit <n>]
thriftp transfer status <transfer-id>
thriftp transfer wait <transfer-id> [--timeout <duration>]
thriftp transfer pause <transfer-id>
thriftp transfer resume <transfer-id>
thriftp transfer cancel <transfer-id> [--yes]
thriftp transfer retry <transfer-id>
thriftp transfer priority <transfer-id> <low|normal|high>
thriftp transfer reorder <transfer-id>...
```

**Rules**: `upload` and `download` remain active until a terminal state by default and report progress on stderr. `--no-wait` enqueues the work and returns a transfer ID immediately and requires an existing transfer host. The default conflict policy is `fail`; an existing destination without an explicit policy returns exit code 6 and is never overwritten silently. An explicit `--conflict` value applies to every conflict in the current invocation, which is the CLI equivalent of apply-to-all. `--recursive` preserves directory structure. Symbolic links are never followed; they are preserved when supported or skipped with a visible warning. `transfer cancel` requires `--yes`. `transfer wait` returns the terminal state or exit code 8 on host loss or interruption.

### Local and Remote Directory Operations

```text
thriftp mkdir <path> [--profile <id-or-name>] [--local] [--parents]
thriftp rename <from> <to> [--profile <id-or-name>] [--local] [--yes]
thriftp delete <path>... [--profile <id-or-name>] [--local] [--recursive] --yes
```

**Rules**: Remote operations require a profile; `--local` selects the local filesystem. `delete` requires `--yes` and never deletes silently. Local deletion uses the operating-system trash when available; remote deletion is permanent and carries a warning. `rename` requires `--yes` when the destination exists. Non-empty directory deletion requires `--recursive` and states the scope and irreversibility in the warning.

### Queue and Limits

```text
thriftp queue list
thriftp queue pause
thriftp queue resume
thriftp queue cancel-all --yes
thriftp config concurrency <n>
thriftp config bandwidth <bytes-per-second|unlimited>
```

**Rules**: Queue control applies to the shared transfer host and is visible to the desktop application. `concurrency` must be at least 1. Bandwidth is a global payload limit; `unlimited` clears it. Pausing the queue pauses active transfers at chunk boundaries and does not start pending work.

### MCP Administration

```text
thriftp mcp enable [--mode <standalone|embedded|both>]
thriftp mcp disable
thriftp mcp status
thriftp mcp clients list                 # includes active session count and session IDs
thriftp mcp clients pending
thriftp mcp clients approve <request-id>
                          --profile <id-or-name>...
                          --dir <profile-id-or-name>:<path>...
                          [--local-dir <path>...]
                          --operations <operation>... [--expires <duration>]
                          [--pre-approve <operation>:<profile>:<path>] [--yes]
thriftp mcp clients deny <request-id>
thriftp mcp clients revoke <client-fingerprint> [--yes]
thriftp mcp clients disconnect <client-fingerprint>
thriftp mcp tools [--json]
thriftp mcp serve
thriftp mcp audit list [--since <timestamp>] [--until <timestamp>]
                        [--client <fingerprint>] [--result <result>] [--limit <n>]
thriftp mcp audit export --format <jsonl|json> --out <path>
thriftp mcp audit verify
thriftp mcp audit retention <days>
thriftp mcp audit limit <bytes|unlimited>
```

**Rules**: `mcp enable` is an explicit user action; MCP remains disabled by default. `mcp status` reports the enabled state, mode, active session count, and supported protocol revisions. `clients list` shows every enrolled client, its grant, and its active sessions so a headless operator can see the full authorization state. `clients approve` requires at least one profile, one remote directory scope, one local directory scope when upload or download is granted, and one operation, and always shows the resulting scope before applying `--yes`. `clients revoke` terminates active sessions immediately and requires `--yes`. `mcp serve` starts the standalone stdio server for convenience; the canonical standalone binary is `thriftp-mcp`. `mcp audit` is read-only except for `retention` and `limit`, which change future retention and storage-limit policy and never delete immediately. `mcp audit verify` reports hash-chain integrity and never repairs a break.

## Secret Input

- The CLI never accepts secrets as arguments or environment variables.
- Secrets are read from the native OS credential store or from standard input with an explicit `--password-stdin` or `--key-passphrase-stdin` flag.
- Standard input is read as one line; the line terminator is discarded; the value is zeroized after use.
- If a stored credential is unavailable and no stdin flag is present, the command fails immediately with exit code 3 and `credential_missing`. It never waits for a TTY prompt.
- If a documented secret environment variable such as `THRIFTP_PASSWORD` or `THRIFTP_KEY_PASSPHRASE` is present, the CLI rejects the run with exit code 10 and guidance to use standard input instead.
- Secret values never appear in output, logs, errors, warnings, process titles, or audit records.

## Output Streams

| Mode | Standard output | Standard error |
|------|-----------------|----------------|
| `human` | Result tables, summaries, and messages | Progress, warnings, diagnostics |
| `json` | Exactly one JSON result document | Progress and diagnostics; never JSON result documents |

**Rules**: In `json` mode, stdout contains exactly one document for both success and failure. Progress and diagnostics on stderr are human-readable by default and may be JSON lines when `--verbose` is combined with a future diagnostics flag; they never contaminate stdout. `--quiet` suppresses progress but not errors. If a downstream consumer closes stdout, the CLI stops writing without panicking, emits no further result, and exits with code 7 if the result could not be delivered. A transfer already in progress continues to a terminal state by default; closing stdout does not cancel it.

### JSON Result Envelope

```json
{
  "schemaVersion": 1,
  "command": "transfer.upload",
  "ok": true,
  "exitCode": 0,
  "data": {
    "transferId": "0f8fad5b-d9cb-469f-a165-70867728950e",
    "state": "completed",
    "transferredBytes": 1048576,
    "totalBytes": 1048576
  },
  "warnings": [],
  "error": null,
  "meta": {
    "durationMs": 842,
    "cliVersion": "0.1.0",
    "automationContractVersion": "1.0.0",
    "hostMode": "delegated"
  }
}
```

```json
{
  "schemaVersion": 1,
  "command": "transfer.upload",
  "ok": false,
  "exitCode": 3,
  "data": null,
  "warnings": [],
  "error": {
    "code": "authentication_failed",
    "message": "The server rejected the saved credential. Update it with 'thriftp profiles set-credential'.",
    "retryable": false,
    "details": {}
  },
  "meta": {
    "durationMs": 214,
    "cliVersion": "0.1.0",
    "automationContractVersion": "1.0.0",
    "hostMode": "local"
  }
}
```

**Rules**: `data`, `warnings`, and `error` are always present (nullable where shown). `error.code` is a stable identifier. `error.details` contains only redacted, non-secret context. Unknown fields must be ignored by consumers. The envelope schema is versioned independently of the application version.

## Exit Codes

| Code | Category | Examples |
|------|----------|----------|
| 0 | Success | Transfer completed, listing returned, grant approved |
| 2 | Usage | Unknown command, missing required option, conflicting options, unknown profile |
| 3 | Authentication | Invalid credentials, credential rejected, credential missing |
| 4 | Network | Unreachable host, timeout, DNS failure, host endpoint unavailable |
| 5 | Permission | Remote permission denied, local filesystem permission denied |
| 6 | Conflict | Existing destination without a policy, resume metadata mismatch |
| 7 | Cancelled | User cancellation, output pipe closed before result delivery |
| 8 | Interrupted | Host lost, process interruption, resumable transfer |
| 9 | Authorization or policy | MCP disabled, grant missing, scope denied, confirmation required |
| 10 | Validation | Unsafe path, ambiguous path, forbidden secret environment variable |
| 11 | Credential store | Keychain or Secret Service unavailable or locked |
| 12 | Unsupported | Unsupported capability, operation, or interface version |
| 13 | Internal | Unexpected internal failure with a redacted message |

**Rules**: A command returns exactly one exit code. Scripts must be able to branch on the code without parsing output. Human and JSON modes use the same codes.

## Help and Version

- `thriftp --help` lists the command tree, global options, and output modes.
- `thriftp <command> --help` lists the command's arguments, options, defaults, examples, and exit-code notes.
- `thriftp --version` prints the CLI version, automation contract version, and supported MCP versions.
- `thriftp mcp tools --json` lists tool names, descriptions, destructive flags, and contract versions without connecting to a server.

## Examples

```bash
# Store a credential from stdin, then browse
printf '%s\n' "$PASSWORD" | thriftp profiles set-credential dev-server --password-stdin
thriftp browse /var/log --profile dev-server --long

# Upload a directory with an explicit conflict policy and machine-readable result
thriftp --json upload ./dist /srv/app --profile dev-server --recursive --conflict fail

# Enqueue without waiting, then inspect status
thriftp --json upload ./build.tar.gz /srv --profile dev-server --no-wait
thriftp --json transfer status <transfer-id>

# Enable MCP, review a pending client, and grant one profile and directory
thriftp mcp enable --mode standalone
thriftp mcp clients pending
thriftp mcp clients approve <request-id> \
  --profile dev-server --dir dev-server:/srv/app --local-dir ./uploads \
  --operations browse.list,transfers.upload,transfers.download,files.stat

# Inspect and verify the audit trail
thriftp mcp audit list --since 2026-11-01T00:00:00Z --result denied
thriftp mcp audit verify
```

## Contract Invariants

1. The CLI never implements protocol logic; every operation delegates to `thriftp-core`.
2. The CLI never accepts a secret through arguments or environment variables.
3. JSON mode writes exactly one result document to stdout.
4. Progress and diagnostics never appear on stdout in JSON mode.
5. Every command has a stable exit code from the documented table.
6. A conflict without an explicit policy fails and never overwrites silently.
7. Destructive operations require an explicit confirmation flag.
8. Local deletion uses the OS trash when available; remote deletion warns that it is permanent.
9. The CLI never reports an incomplete transfer as completed.
10. The CLI never exposes favorites.
