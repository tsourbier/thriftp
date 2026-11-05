# Contract: Protocol Adapter (`RemoteFs`)

**Feature**: `001-desktop-file-transfer` | **Date**: 2026-11-05

This contract isolates FTP, FTPS, and SFTP implementations from the Rust core. The core depends only on the trait and capability model; no Tauri or UI types appear here.

## Trait

```rust
#[async_trait]
pub trait RemoteFs: Send + Sync {
    async fn connect(
        &mut self,
        profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> Result<(), ProtocolError>;

    async fn disconnect(&mut self) -> Result<(), ProtocolError>;

    fn capabilities(&self) -> ProtocolCapabilities;

    async fn list_dir(&mut self, path: &str) -> Result<DirectoryStream, ProtocolError>;

    async fn stat(&mut self, path: &str) -> Result<FileEntry, ProtocolError>;

    async fn mkdir(&mut self, path: &str) -> Result<(), ProtocolError>;

    async fn rename(&mut self, from: &str, to: &str) -> Result<(), ProtocolError>;

    async fn delete_file(&mut self, path: &str) -> Result<(), ProtocolError>;

    async fn delete_dir(&mut self, path: &str) -> Result<(), ProtocolError>;

    async fn chmod(&mut self, path: &str, mode: u32) -> Result<(), ProtocolError>;

    async fn open_read(
        &mut self,
        path: &str,
        resume_offset: u64,
    ) -> Result<RemoteReader, ProtocolError>;

    async fn open_write(
        &mut self,
        path: &str,
        resume_offset: u64,
    ) -> Result<RemoteWriter, ProtocolError>;
}
```

`DirectoryStream` yields batches of `FileEntry`; `RemoteReader` and `RemoteWriter` expose async chunk reads/writes and a confirmed offset. The transfer engine owns chunk sizing, cancellation, bandwidth, and persistence.

## Capabilities

```rust
pub struct ProtocolCapabilities {
    pub resume: bool,
    pub permissions: bool,
    pub ownership: bool,
    pub hidden_files: bool,
    pub symbolic_links: bool,
    pub server_side_rename: bool,
    pub server_side_delete: bool,
    pub max_path_length: Option<usize>,
}
```

The UI and core use capabilities to enable or explain operations. Unsupported operations return `unsupported_capability`, not silent no-ops.

## Error Taxonomy

```rust
pub enum ProtocolError {
    AuthFailed,
    CredentialRejected,
    CertificateInvalid,
    HostKeyUnknown,
    HostKeyChanged,
    Network,
    Timeout,
    PermissionDenied,
    NotFound,
    Conflict,
    UnsupportedCapability,
    Protocol(String), // redacted before user display
    Cancelled,
}
```

Raw protocol diagnostics are logged only through redacted `tracing` fields and never returned to the UI.

## FTP and FTPS

- Implemented with `suppaftp` 12.x using Tokio and rustls (`tokio-rustls-ring` feature).
- Support FTP, explicit FTPS, and implicit FTPS. Implicit FTPS uses the upstream `deprecated` feature and is isolated in this adapter so it can be replaced if the API is removed.
- TLS certificates are validated against native roots by default.
- Plain FTP is marked as unencrypted in the UI before and during the session.
- Listing responses are parsed incrementally into `FileEntry` batches.
- Resumption is advertised only when the server supports REST/APPE-style continuation.

## SFTP

- Implemented with `russh` and `russh-sftp`.
- Supports password and SSH private-key authentication.
- Host keys are verified against known-hosts data; unknown or changed keys require explicit user action and are visible for the session.
- Directory entries come from SFTP readdir batches.
- Permissions and ownership are exposed when the server returns them.
- Resumption is advertised only when the server supports the required offset operations.

## Symbolic Links

- Recursive transfers never follow symbolic links.
- Links are preserved when the protocol and destination support it; otherwise they are skipped with a visible report.
- A link target is available for display but is not used to escape the selected tree.

## Directory Stream Contract

- The stream must deliver entries incrementally; it must not buffer the complete listing first.
- The stream must be cancellable.
- The first batch should be emitted as soon as entries are parsed, independent of total listing completion.
- Errors after partial emission are reported as a terminal error while preserving entries already delivered.

## Security and Redaction

- Credentials are accepted as a zeroizing `Secret`, used only for the connection, and dropped promptly.
- Certificate and host-key failures are distinct errors with explicit recovery actions.
- No protocol adapter may implement a silent verification bypass.
- Every connection, listing, and transfer operation emits a `tracing` span with profile ID and operation outcome, never secret values.

## Implementations

| Protocol | Crate | Auth | TLS/Host key |
|----------|-------|------|--------------|
| FTP | `suppaftp` | Password | None; explicit unencrypted warning |
| FTPS explicit | `suppaftp` + rustls | Password | Certificate validation |
| FTPS implicit | `suppaftp` + rustls | Password | Certificate validation |
| SFTP | `russh` + `russh-sftp` | Password or private key | Known-hosts verification |

## Test Contract

- Adapter integration tests run against the shared Docker Compose harness on macOS through Docker Desktop and in Linux CI; Windows developers with Docker Desktop may use it too.
- Docker-free unit tests cover deterministic error paths and parsing with fakes.
- Tests cover authentication failure, TLS failure, host-key change, missing capability, partial listing, cancellation, resume offsets, and permission errors.
