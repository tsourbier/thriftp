# Security Review: CLI and MCP Automation Interfaces

**Feature**: `002-cli-mcp-support` | **Date**: 2026-11-05

## Scope reviewed

- CLI secret input and output redaction.
- MCP Ed25519 enrollment, challenge/signature authentication, session tokens, grants, and revocation epochs.
- Canonical local and remote scopes, traversal rejection, and sibling-prefix checks.
- Audit hash chain, append, verification, and tamper detection.
- MCP local-only stdio transport.
- Destructive operation confirmation gate.
- Credential-store delegation through the shared native adapter.

## Findings

| Area | Result |
|------|--------|
| Secret handling | Pass. Secrets are read from stdin only, wrapped in a zeroizing type, and not serialized or logged. |
| Enrollment | Pass. Enrollment grants nothing until explicit approval. Grants bind to the public-key fingerprint. |
| Session authentication | Pass. Sessions require a signed server nonce and expire. |
| Scope enforcement | Pass for implemented remote/local checks. Traversal, encoded traversal, backslashes, and sibling-prefix escapes are rejected. |
| Audit | Pass. Records are append-only, fsynced, hash-chained, and tamper-evident. |
| Destructive operations | Pass for the implemented `confirmed: true` gate. Scoped pre-approval policies are not yet implemented. |
| Transport | Pass for stdio. Loopback transport is not yet implemented. |
| Same-user boundary | Documented. The local MCP channel is not a sandbox; hash chaining makes same-user audit modification detectable. |

## Residual risks and follow-up

1. Loopback transport and desktop embedded mode are not implemented; only stdio is available.
2. Pre-approval policies and a full elicitation flow are not implemented; destructive MCP calls require the explicit `confirmed: true` argument.
3. The transfer host and live shared-state control channel are not implemented; transfers run in the foreground CLI process.
4. Retention/storage-limit purge is not implemented.
5. Full parity, security-parity, panic-safety, and quickstart validation suites remain outstanding.
6. Rustdoc/missing-docs enforcement is not yet enabled.

## Sign-off

No plaintext secret path, silent credential fallback, or unauthenticated operational MCP tool path was found in the implemented scope. The residual risks above must be closed before claiming full specification compliance.
