# Security notes

## Credentials

- Credentials are stored only through the native operating-system credential
  store. There is no plaintext fallback.
- `Secret` values are zeroizing, non-serializable, non-debuggable, and never
  returned to the Svelte UI.
- A credential is bound to its connection profile (protocol, host, port,
  username, and authentication method). Identity changes invalidate reuse.
- Missing, rejected, unavailable, and locked credential states are distinct
  recovery states.

## Transport security

- FTPS uses TLS with certificate verification against the operating-system
  trust store. Certificate failures are reported as `certificate_invalid`.
- SFTP verifies server host keys through `known_hosts`. Unknown and changed keys
  are separate errors; changed keys may indicate interception.
- Plain FTP is unencrypted. The UI shows a warning and requires explicit
  confirmation before connecting.

## Local data

- Local deletes go through the operating-system trash.
- Remote deletes are permanent and require explicit confirmation.
- Partial transfer files use a `.thriftp-part` suffix and a JSON sidecar with
  source size/modification time for resume verification.
- Transfer history is retained for 1,000 records and can be cleared.

## Logs and errors

- Error payloads sent to the UI contain stable codes and redacted messages.
- Raw protocol diagnostics are logged locally with contextual identifiers.
- No secret is written to state files, logs, errors, diagnostics, crash reports,
  or telemetry.

## Privacy and supply chain

- No account, subscription, cloud service, or remote authentication service is
  required.
- No telemetry or user-data transmission is enabled by default. Privacy tests
  assert this.
- `Cargo.lock` and `package-lock.json` are committed; CI uses `--locked` and
  `npm ci`.
- Dependencies are audited with `cargo audit`; CI generates a CycloneDX SBOM and
  runs a license inventory.
- CI builds the workspace twice with remapped paths and compares artifact hashes.
