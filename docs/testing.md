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

Requirements: Node.js 26.13.0+ and npm.

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
python3 scripts/validate-dependency-records.py specs/005-dependency-upgrade --release
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
scripts/check-dependency-policy.sh
cargo audit
scripts/check-licenses.sh
scripts/generate-sbom.sh target/sbom.json
scripts/verify-reproducible-build.sh
```

CI also runs the macOS/Windows test matrix, the frontend job, the container
harness, dependency audit, license inventory, SBOM generation, and the
reproducible-build comparison.

## Workspace feature tests

The workspace refresh adds coverage for layout normalization and persistence,
connection history deduplication and clearing, favorite path validation and
restoration, SSH key discovery and validation, breadcrumbs, command-palette
search, sidebar filtering, and the disconnected connection hub.

Run the focused frontend suites with:

```bash
cd apps/desktop
npm test -- src/lib/workspace src/lib/commands src/lib/browser/breadcrumbs.test.ts
```

Run the Rust workspace and desktop contract suites with:

```bash
cargo test --workspace --locked
cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --locked
```
