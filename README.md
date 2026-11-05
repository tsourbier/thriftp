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
a single-owner transfer host shared by the CLI, MCP, and desktop. The `003`
scope adds server favorites, `004` refreshes the workspace UI, and `005`
upgrades the dependency and toolchain baseline to Rust 1.99.1, Node.js 26.13.0,
and Tauri 2.12.5.

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

- Rust 1.90.0+ (the pinned `rust-toolchain.toml` selects 1.99.1)
- Node 26.13.0+ and npm for the desktop UI
- [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/)
- Docker only for the optional protocol integration tests

## Get started

Clone the repository:

```bash
git clone https://github.com/tsourbier/thriftp.git
cd thriftp
```

Install the desktop dependencies and launch the app in development mode:

```bash
cd apps/desktop
npm ci
npx tauri dev
```

`npx tauri dev` starts the Vite dev server and opens the native desktop window.
To produce an installable bundle instead, run `npx tauri build` from the same
directory.

The command-line client runs directly from the workspace:

```bash
cargo run -p thriftp-cli -- --help
cargo run -p thriftp-cli -- profiles list
```

Install the `thriftp` binary onto your `PATH` with:

```bash
cargo install --path crates/thriftp-cli
```

See the [CLI documentation](docs/cli.md) for profiles, transfers, and the MCP
server.

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
