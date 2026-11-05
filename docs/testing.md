# Testing

## Unit and integration tests (Docker-free)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

This covers models, validation, persistence, browsing, the transfer engine,
queue scheduling, recovery, previews, permissions, IPC payloads, privacy, and
cross-crate offline workflows. Unit tests never require Docker.

Shared fakes live in `crates/thriftp-core/tests/support/`:
`FakeRemoteFs`, `MemoryCredentialStore`, and temporary JSON stores.

## Frontend tests

```bash
cd apps/desktop
npm ci
npm run lint
npm run format
npm test
npm run build
```

Vitest runs Svelte component tests, keyboard tests, theme tests,
acknowledgement-latency tests, and an end-to-end IPC contract workflow.

## Protocol integration tests (containerized)

```bash
scripts/test-integration.sh
```

The script detects Docker, starts `docker/protocol/docker-compose.yml`, exports
endpoints, runs the protocol tests, and tears the stack down. If no container
runtime is available it exits cleanly and unit tests remain available.

| Protocol | Endpoint | Credentials |
|----------|----------|-------------|
| FTP | 127.0.0.1:2121 | tester / testpass |
| FTPS explicit | 127.0.0.1:2122 | tester / testpass |
| FTPS implicit | 127.0.0.1:2990 | tester / testpass |
| SFTP | 127.0.0.1:2222 | tester / testpass, or key `docker/sftp/keys/client_ed25519` (passphrase `testpass`) |

Fixtures: `docker/protocol/certs/server.crt` for FTPS and
`docker/sftp/keys/known_hosts` for SFTP. Docker is a test-only dependency; the
application never requires it.

## Contributor validation

```bash
scripts/validate-contributor-build.sh
```

Builds the Rust workspace, installs frontend dependencies, and builds the UI.

## Supply chain and reproducibility

```bash
cargo audit
scripts/check-licenses.sh
scripts/generate-sbom.sh target/sbom.json
scripts/verify-reproducible-build.sh
```

CI also runs the macOS/Windows test matrix, the frontend job, the container
harness, dependency audit, license inventory, SBOM generation, and the
reproducible-build comparison.
