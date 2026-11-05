# Contract: Credential Store Adapter

**Feature**: `003-server-favorites` | **Date**: 2026-11-05

This contract defines the boundary between the Tauri-independent core and the native operating-system credential store. `thriftp-credentials-keyring` implements it using the platform-native backend.

## Trait

```rust
pub trait CredentialStore: Send + Sync {
    fn availability(&self) -> Result<CredentialAvailability, CredentialStoreError>;
    fn get(&self, reference: &CredentialRef) -> Result<Option<Secret>, CredentialStoreError>;
    fn set(&self, reference: &CredentialRef, secret: Secret) -> Result<(), CredentialStoreError>;
    fn delete(&self, reference: &CredentialRef) -> Result<(), CredentialStoreError>;
}

pub enum CredentialAvailability {
    Available,
    Locked,
    Unavailable,
    Denied,
}
```

## Secret Handling

- `Secret` is a non-serializable, non-debuggable wrapper around a zeroizing byte buffer.
- Secrets are never returned to the Svelte UI.
- Secrets are passed directly to the connection layer or credential-store adapter.
- Secrets are never included in logs, error messages, diagnostics, crash reports, or telemetry.
- A secret entered only for a connection test or a session-only connection is dropped at the end of the operation.

## Keying

- Fixed service name: `dev.thriftp.credentials`.
- Account name: the `CredentialRef.id` UUID.
- The keyring entry contains only the secret material; protocol, host, port, username, and auth method live in the non-secret `CredentialRef`.
- The adapter must not derive the account from a username, host, or other user-controlled string that can collide.

## Error Types

```rust
pub enum CredentialStoreError {
    Unavailable,
    Locked,
    Denied,
    NotFound,
    InvalidReference,
    Backend(String), // redacted before reaching the UI
}
```

- `Unavailable`, `Locked`, and `Denied` map to the user-facing `credential_store_unavailable` code with a platform-appropriate explanation.
- `NotFound` maps to `credential_missing`.
- `Backend` messages are redacted and never include secret material.
- No error path may fall back to plaintext storage.

## Behavior

### Availability probe

- `availability` checks whether the native backend can be reached.
- On Linux, a missing or unreachable Secret Service must return `Unavailable`, not a local file fallback.
- On macOS and Windows, locked or denied access returns `Locked` or `Denied`.

### Get

- Returns `None` when the reference has no stored secret.
- Returns `NotFound` only when the backend distinguishes a missing entry from an empty value.
- Returns `InvalidReference` when the reference is malformed.

### Set

- Creates or replaces the secret for the reference.
- Replacing a shared credential updates the single keyring entry used by all referencing favorites.
- Must not write a plaintext copy anywhere.

### Delete

- Deletes the secret for the reference.
- The caller must verify that no favorite references the credential and that the user confirmed.
- Deleting a missing entry returns `NotFound` and must not be treated as success unless the caller explicitly allows idempotent deletion.

## Test Double

The core test suite uses an in-memory `FakeCredentialStore` that:

- Supports `Available`, `Locked`, `Unavailable`, and `Denied` modes.
- Records `set`, `get`, and `delete` calls without exposing secrets in assertions.
- Can simulate missing and rejected credentials.
- Is never used in production.

## Platform Notes

- **macOS**: use the native Keychain backend; respect the user's keychain lock state.
- **Windows**: use Credential Manager; return a clear error if the user profile cannot access it.
- **Linux**: use Secret Service; if no service is available, fail closed and explain that a credential service is required.
- **CI**: use the fake store for deterministic tests; real-store smoke tests are optional and platform-gated.

## Future Reuse

The trait and `CredentialRef` model live in `thriftp-core` and have no Tauri dependency. Future CLI and MCP interfaces must use the same abstraction rather than implementing separate secret storage.
