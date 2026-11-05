# Thriftp

Thriftp is a free, open-source, cross-platform desktop file transfer client for
FTP, FTPS, and SFTP. It provides a dual-pane local/remote browser, a persistent
transfer queue, resumable streaming transfers, and native credential storage.

## Status

Feature-complete for the `001-desktop-file-transfer` scope: connections,
browsing, transfers, queue, tabs/favorites, inspector, previews, recovery, and
the security/privacy controls are implemented and tested. The `002` automation
scope adds the `thriftp` CLI and the local `thriftp-mcp` stdio/loopback server,
including explicit enrollment, canonical scopes, a hash-chained audit trail, and
a single-owner transfer host shared by the CLI, MCP, and desktop.

## Highlights

- FTP, explicit FTPS, implicit FTPS, and SFTP.
- Password and SSH private-key authentication with `known_hosts` verification.
- Native OS credential store; no plaintext secret fallback.
- Streaming transfers with pause/resume/cancel, conflict handling, and O(1)
  memory use.
- Persistent queue with concurrency, bandwidth, priority, and reorder.
- Dual-pane browser, tabs, directory favorites, file inspector, and previews.
- No account, subscription, cloud service, or mandatory telemetry.

## Requirements

- Rust 1.88.0+
- Node 22+ and npm for the desktop UI
- Tauri platform prerequisites
- Docker only for the optional protocol integration tests

## Build and test

```bash
cargo test --workspace --locked
scripts/test-integration.sh

cd apps/desktop
npm ci
npm test
npm run build
```

See [Testing](docs/testing.md) for the container harness, fixtures, and
contributor validation script.

## Documentation

- [Desktop client](docs/desktop.md)
- [Server favorites](docs/server-favorites.md)
- [Command-line interface](docs/cli.md)
- [MCP server](docs/mcp.md)
- [Automation security](docs/automation-security.md)
- [Transfer process model](docs/transfer-host.md)
- [Compatibility](docs/compatibility.md)
- [Architecture](docs/architecture.md)
- [Testing](docs/testing.md)
- [Security and privacy](docs/security.md)
- [Release process](docs/release.md)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).
