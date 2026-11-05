# Compatibility

## CLI

The CLI contract version is `1.0.0`. The JSON envelope has `schemaVersion: 1`. Additive fields are backward compatible; removing fields or changing their meaning requires a major contract version.

## MCP

Supported protocol revisions:

- `2026-07-28` — current, advertised through `server/discover`.
- `2025-11-25` — legacy initialize handshake.

Tool inputs use JSON Schema 2020-12. The automation contract version is `1.0.0`. Transports are stdio and loopback-only HTTP; both are local. An unsupported revision receives a structured compatibility error listing the supported revisions.

## Deprecation

Deprecated behavior is documented for at least one minor release before removal. Breaking changes require an explicit contract-version change and a migration note.

## Local-only policy

MCP is local-only in this release. Remote transport is a future capability and requires a separate authorization design.
