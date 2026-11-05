# Security Review

**Feature**: `005-dependency-upgrade`
**Date**: 2026-11-05

## Scope

Security-sensitive upgrades reviewed: `keyring`, `russh`, `russh-sftp`, `suppaftp`, `ed25519-dalek`, `sha2`, `base64`, `native-tls`, `tauri-plugin-dialog`, and the Rust/Tauri runtime stack.

## Findings

- **Credential storage**: `keyring` 4.2.0 continues to use the native macOS Keychain, Windows Credential Manager, and Linux Secret Service backends through the existing adapter. No plaintext fallback was introduced.
- **SSH/SFTP**: `russh` 0.65.1 changed the server-key callback to `PublicKeyOrCertificate`; the adapter now extracts the public key for known-hosts verification. Host-key verification remains enabled by default and unexpected changes remain visible.
- **FTPS/TLS**: `suppaftp` 12.2.1 and `native-tls` 0.2.18 retain certificate validation by default. The protocol tests for explicit and implicit FTPS still pass.
- **Cryptography and encoding**: `ed25519-dalek` 3.0.0, `sha2` 0.11.1, and `base64` 0.23.1 passed MCP authentication, audit-chain, and secret-redaction tests. No secret material is logged or persisted.
- **Telemetry and privacy**: No upgraded dependency adds mandatory telemetry, account creation, cloud calls, or a new plaintext secret path.
- **Provenance**: No new dependency source, fork, alternate registry, or patched package was introduced. All packages resolve from the official crates.io and npm registries.
- **Residual advisory**: `cargo audit` reports RUSTSEC-2023-0071 for `rsa 0.10.0-rc.18` (transitive via `russh`). It is medium severity, has no fixed upgrade, and is covered by `EX-004`. Ed25519 remains the preferred key type.
- **Pre-release transitive**: `ssh-key 0.7.0-rc.11` is required by `russh 0.65.1`; it is covered by `EX-005`.
- **Linux credential backend**: the zbus 5.15.1 stack is marked unmaintained by RustSec; it is covered by `EX-006` and does not weaken credential storage.

## Sign-off

Security suites passed locally: `test_secret_redaction`, `test_security_parity`, `test_panic_safety`, credential-store tests, and MCP authorization/security contracts. Cross-platform execution remains CI-gated.
