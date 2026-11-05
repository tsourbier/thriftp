# Automation Security

## Identity

MCP clients identify themselves with an Ed25519 public key. The SHA-256 fingerprint of that key is the stable authorization identity. Private keys never leave the client.

## Enrollment and grants

Enrollment grants nothing. A user must approve a pending request with an explicit profile, scope, and operation set. Grants are revisioned, can expire, and can be revoked. Revocation increments a revocation epoch and invalidates sessions that authenticated under an older epoch.

## Canonical scopes

- Local paths are canonicalized with the operating-system canonicalizer.
- Remote paths are absolute POSIX paths; traversal, alternate separators, encoded traversal, and symbolic-link escapes are denied.
- Containment is checked on path-component boundaries, so `/srv/app2` is not inside `/srv/app`.

## Destructive operations

Destructive MCP operations require a single-use confirmation bound to the
session, grant, operation, and target. Confirmations expire (default 120 s) and
can be used once. A scoped pre-approval policy may substitute for a confirmation;
each use records a `policy_used` audit event. The absence of a graphical
interface never authorizes an operation: headless callers fail closed with
`confirmation_required`.

## Audit

Every MCP operation is written to an append-only JSONL audit segment and fsynced
before a result is returned. Each record includes a monotonic sequence and
previous-record hash. A broken chain is reported and never silently repaired.
An unavailable audit store blocks the operation with `audit_unavailable`.
Retention deletes only sealed segments, after a seven-day notice, and writes a
checkpoint before deletion so the remaining chain still verifies.

## Same-user boundary

The local MCP channel is not a sandbox. A same-user process with filesystem access can read the local automation store or invoke the CLI directly. MCP grants constrain the MCP surface, not the operating-system user. Hash chaining makes same-user audit modification detectable.

## Secrets

Passwords, private keys, passphrases, tokens, and session secrets must never appear in arguments, environment variables, logs, errors, diagnostics, or audit records.
