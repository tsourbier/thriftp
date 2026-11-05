# ADR 0001: FTP/FTPS TLS stack

**Status**: Accepted

**Context**: The implementation plan listed `suppaftp` with `tokio-rustls-ring`
for FTP/FTPS. The delivered adapter uses `suppaftp`'s `native-tls` integration
because it is the stable, synchronous data-connection API used by the transfer
worker and it validates certificates through the operating-system trust store.

**Decision**: Keep `suppaftp` with `native-tls` for the initial release. The
deviation is recorded here rather than silently diverging from the plan.

**Consequences**:

- Certificate validation remains enabled by default and follows the operating
  system trust store on macOS, Linux, and Windows.
- The TLS implementation is supplied by the platform rather than a Rust-only
  stack; dependency and license review covers `native-tls` and its platform
  backends.
- A future migration to `tokio-rustls-ring` can be made inside the protocol
  adapter without changing the core `RemoteFs` contract.

**References**: `specs/001-desktop-file-transfer/plan.md`,
`crates/thriftp-protocols/src/ftp.rs`, Constitution III.
