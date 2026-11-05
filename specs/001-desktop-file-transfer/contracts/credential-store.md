# Contract: Credential Store

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

The desktop client reuses the credential-store contract defined by the Server Favorites feature. The canonical definition is:

- [`specs/003-server-favorites/contracts/credential-store.md`](../../003-server-favorites/contracts/credential-store.md)

## Summary of the Shared Contract

- `CredentialStore` is a Tauri-independent Rust trait with `availability`, `get`, `set`, and `delete`.
- `thriftp-credentials-keyring` implements it with macOS Keychain, Windows Credential Manager, and Linux Secret Service through the `keyring` crate.
- Secrets are non-serializable, non-debuggable, zeroizing values. They are never returned to the Svelte UI or written to JSON, logs, errors, diagnostics, crash reports, or telemetry.
- The store uses service `dev.thriftp.credentials` and account `CredentialRef.id`.
- Unavailable, locked, or denied backends fail closed with a typed error. There is no plaintext fallback.
- Missing entries map to `credential_missing`; rejected secrets map to `credential_rejected`.
- The `CredentialRef` identity binding (protocol, host, port, username, authentication method) is enforced by the core service, not by the store key.
- Shared credential updates apply to every referencing favorite; deletion requires no remaining references and explicit confirmation.
- Tests use an in-memory fake; real-store smoke tests are platform-gated.

## Desktop-Specific Use

- Connection profiles and server favorites reference `CredentialRef` values through the shared metadata store.
- A connection may use a saved credential or a session-only secret. Session-only secrets are accepted only after a clear warning and are dropped when the session ends.
- If the credential store is unavailable while saving, non-secret profile settings may still be saved; the user is told that credentials were not saved.
- If unavailable while connecting, the UI may collect a session-only secret with a warning; it must not persist it.

No desktop-specific secret storage exists outside this shared contract.
