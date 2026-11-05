# Security Policy

## Reporting a vulnerability

Please report security issues privately to the project maintainers rather than
opening a public issue. Include:

- A description of the issue and affected component.
- Reproduction steps or a proof of concept.
- The affected version or commit.
- Any suggested mitigation.

## Security expectations

- Credentials are stored only in the native operating-system credential store.
- Passwords, private keys, and passphrases never appear in configuration files,
  logs, errors, diagnostics, crash reports, or telemetry.
- TLS certificates and SSH host keys are verified by default.
- Verification overrides require explicit user action and remain visible.
- Dependency and container images are reviewed and pinned where required.

## Supported versions

Security fixes are applied to the current development line. See the repository
release notes for supported versions.
