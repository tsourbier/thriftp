# Contributing to Thriftp

Thriftp is a free, open-source, cross-platform desktop file transfer client.

## Development setup

1. Install Rust stable, Node.js 20+, and the Tauri 2 prerequisites for your platform.
2. Build the Rust workspace:

   ```bash
   cargo build --workspace
   ```

3. Install and build the desktop frontend:

   ```bash
   cd apps/desktop
   npm install
   npm run build
   ```

4. Run tests:

   ```bash
   cargo test --workspace
   scripts/test-integration.sh   # requires a Docker-compatible runtime
   ```

## Contribution rules

- Follow the project constitution in `.specify/memory/constitution.md`.
- Keep protocol logic behind the `RemoteFs` contract.
- Never commit secrets or plaintext credentials.
- Add or update tests for behavior changes.
- Keep pull requests focused and explain any added complexity.

## Security reports

Please do not open public issues for security vulnerabilities. Follow
[`SECURITY.md`](SECURITY.md).
