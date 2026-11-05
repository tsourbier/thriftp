# Security Review: Server Favorites

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

## Scope

- Server-favorite creation, editing, deletion, grouping, pinning, and sorting.
- Credential reference creation, explicit reuse, shared updates, removal, and deletion.
- Native credential-store delegation.
- Persistence of non-secret metadata.
- Active-connection independence during favorite edits and deletion.
- Certificate and SSH host-key verification on the favorite connect path.

## Findings

| Area | Result |
|------|--------|
| Secret storage | Pass. Secrets are passed only to the native `CredentialStore`; the JSON metadata store never receives a secret. |
| Secret exposure | Pass for the implemented service. A sentinel-secret test confirms the persisted state does not contain secret material. |
| Credential binding | Pass. Reuse requires matching protocol, host, port, username, and authentication method; changing identity invalidates the previous reference. |
| Shared credentials | Pass for the implemented service. Impact calculation lists every affected favorite, and deletion of the last reference requires confirmation. |
| Missing/unavailable store | Existing `CredentialService` maps unavailable, locked, and denied stores to a fail-closed error. No plaintext fallback exists. |
| Active connections | Pass. Favorite edits and deletion do not touch the active connection registry. |
| Connection verification | Favorite connections reuse the existing `ConnectionService` certificate and host-key verification path. |
| Persistence | Pass for the implemented metadata fields. Writes use the existing atomic JSON adapter with schema versioning. |
| Redaction | Pass. Service errors use the shared `CoreError` mapping; no secret is included in error messages. |

## Residual risks

1. The native keyring adapter's `availability` currently reports `Available`; locked/unavailable detection depends on the backend error path.
2. Platform-gated native credential-store smoke tests are not yet implemented.
3. Frontend credential-impact, unavailable-store, and last-reference dialogs are not implemented.
4. Full IPC contract tests and keyboard accessibility tests remain outstanding.
5. Performance validation with 500 favorites remains outstanding.

## Sign-off

No plaintext secret path, silent credential fallback, or credential reuse across mismatched identities was found in the implemented scope. The residual risks above must be closed before claiming full specification compliance.
