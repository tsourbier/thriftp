use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use russh::client::{self, Handle};
use russh::keys::{PublicKey, known_hosts};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, FileType, OpenFlags, StatusCode};
use thriftp_core::credentials::Secret;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, EntryType, FileEntry, Ownership, ProtocolCapabilities,
};
use thriftp_core::protocol::{ProgressCallback, RemoteFs, TRANSFER_CHUNK_SIZE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostKeyStatus {
    Unknown,
    Changed,
}

struct SshHandler {
    host: String,
    port: u16,
    known_hosts_path: Option<PathBuf>,
    expected_fingerprint: Option<String>,
    status: Arc<Mutex<Option<HostKeyStatus>>>,
}

impl client::Handler for SshHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let expected = self.expected_fingerprint.as_deref();
        let result = match &self.known_hosts_path {
            Some(path) => {
                known_hosts::check_known_hosts_path(&self.host, self.port, server_public_key, path)
            }
            None => known_hosts::check_known_hosts(&self.host, self.port, server_public_key),
        };
        match result {
            Ok(true) => Ok(true),
            Ok(false) => {
                if let Some(expected) = expected {
                    if key_fingerprint(server_public_key) == expected {
                        return Ok(true);
                    }
                    *self.status.lock().expect("host-key status lock") =
                        Some(HostKeyStatus::Changed);
                    return Ok(false);
                }
                *self.status.lock().expect("host-key status lock") = Some(HostKeyStatus::Unknown);
                Ok(false)
            }
            Err(russh::keys::Error::KeyChanged { line }) => {
                *self.status.lock().expect("host-key status lock") = Some(HostKeyStatus::Changed);
                Err(anyhow::anyhow!(
                    "known_hosts entry on line {line} does not match the server key"
                ))
            }
            Err(error) => Err(anyhow::anyhow!("host-key verification failed: {error}")),
        }
    }
}

/// Compute the OpenSSH SHA-256 fingerprint (base64, no padding) of a key.
#[must_use]
pub fn key_fingerprint(key: &PublicKey) -> String {
    normalize_fingerprint(
        &key.fingerprint(russh::keys::ssh_key::HashAlg::Sha256)
            .to_string(),
    )
}

/// Load a user-trusted fingerprint for `host:port` from the CLI/desktop store.
#[must_use]
pub fn trusted_fingerprint(path: &Path, host: &str, port: u16) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let values: std::collections::BTreeMap<String, String> = serde_json::from_slice(&bytes).ok()?;
    values
        .get(&format!("{host}:{port}"))
        .map(|value| normalize_fingerprint(value))
}

/// Normalize a SHA-256 fingerprint for comparison.
#[must_use]
pub fn normalize_fingerprint(value: &str) -> String {
    value
        .trim()
        .strip_prefix("SHA256:")
        .unwrap_or(value.trim())
        .trim_end_matches('=')
        .to_owned()
}

pub struct SftpAdapter {
    session: Option<Handle<SshHandler>>,
    sftp: Option<SftpSession>,
    capabilities: ProtocolCapabilities,
    known_hosts_path: Option<PathBuf>,
    trust_store_path: Option<PathBuf>,
}

impl SftpAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: None,
            sftp: None,
            capabilities: ProtocolCapabilities {
                resume: true,
                permissions: true,
                ownership: true,
                hidden_files: true,
                symbolic_links: true,
                server_side_rename: true,
                server_side_delete: true,
                max_path_length: None,
            },
            known_hosts_path: None,
            trust_store_path: Some(thriftp_core::automation::data_dir().join("known_hosts.json")),
        }
    }

    /// Override the user-trusted fingerprint store used for explicit trust.
    #[must_use]
    pub fn with_trust_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.trust_store_path = Some(path.into());
        self
    }

    /// Use an explicit `known_hosts` file instead of the user's default file.
    #[must_use]
    pub fn with_known_hosts_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.known_hosts_path = Some(path.into());
        self
    }

    fn require_sftp(&self) -> CoreResult<&SftpSession> {
        self.sftp
            .as_ref()
            .ok_or_else(|| CoreError::internal("SFTP session is not connected"))
    }
}

impl Default for SftpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RemoteFs for SftpAdapter {
    async fn connect(
        &mut self,
        profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        let config = Arc::new(client::Config::default());
        let status = Arc::new(Mutex::new(None));
        let expected_fingerprint = self
            .trust_store_path
            .as_deref()
            .and_then(|path| trusted_fingerprint(path, &profile.host, profile.port));
        let handler = SshHandler {
            host: profile.host.clone(),
            port: profile.port,
            known_hosts_path: self.known_hosts_path.clone(),
            expected_fingerprint,
            status: Arc::clone(&status),
        };
        let timeout = Duration::from_secs(u64::from(profile.timeout_seconds));
        let mut session = tokio::time::timeout(timeout, async {
            let stream = tokio::net::TcpStream::connect((profile.host.as_str(), profile.port))
                .await
                .map_err(map_io_error)?;
            client::connect_stream(config, stream, handler)
                .await
                .map_err(|error| map_connect_error(error, &status))
        })
        .await
        .map_err(|_| CoreError::Timeout("SSH handshake timed out".to_owned()))??;

        match profile.auth_method {
            AuthMethod::Password => {
                let credential = credential.ok_or(CoreError::CredentialMissing)?;
                let password = String::from_utf8_lossy(credential.expose()).into_owned();
                let result = session
                    .authenticate_password(&profile.username, password)
                    .await
                    .map_err(map_ssh_error)?;
                if !result.success() {
                    return Err(CoreError::CredentialRejected);
                }
            }
            AuthMethod::SshPrivateKey => {
                let key_path = profile.ssh_key_ref.as_deref().ok_or_else(|| {
                    CoreError::validation("sshKeyRef is required for private-key authentication")
                })?;
                let passphrase = credential
                    .as_ref()
                    .map(|secret| String::from_utf8_lossy(secret.expose()).into_owned());
                let key = russh::keys::load_secret_key(key_path, passphrase.as_deref()).map_err(
                    |error| CoreError::AuthFailed(format!("could not load SSH key: {error}")),
                )?;
                let key = russh::keys::PrivateKeyWithHashAlg::new(Arc::new(key), None);
                let result = session
                    .authenticate_publickey(&profile.username, key)
                    .await
                    .map_err(map_ssh_error)?;
                if !result.success() {
                    return Err(CoreError::CredentialRejected);
                }
            }
        }

        let channel = session
            .channel_open_session()
            .await
            .map_err(map_ssh_error)?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(map_ssh_error)?;
        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(map_sftp_error)?;

        self.session = Some(session);
        self.sftp = Some(sftp);
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        if let Some(sftp) = self.sftp.take() {
            let _ = sftp.close().await;
        }
        self.session.take();
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        self.capabilities.clone()
    }

    async fn list_dir(&mut self, path: &str) -> CoreResult<Vec<FileEntry>> {
        let sftp = self.require_sftp()?;
        let read_dir = sftp.read_dir(path).await.map_err(map_sftp_error)?;
        let mut entries = Vec::new();
        for entry in read_dir {
            let metadata = entry.metadata();
            entries.push(file_entry(
                entry.file_name(),
                entry.path(),
                &metadata,
                entry.file_type(),
                self.capabilities.clone(),
            ));
        }
        entries.sort_by_key(|entry| entry.name.to_lowercase());
        Ok(entries)
    }

    async fn list_dir_stream(
        &mut self,
        path: &str,
        batch_size: usize,
        emit: &mut (dyn FnMut(Vec<FileEntry>) -> bool + Send),
    ) -> CoreResult<()> {
        let sftp = self.require_sftp()?;
        let read_dir = sftp.read_dir(path).await.map_err(map_sftp_error)?;
        let capabilities = self.capabilities.clone();
        let batch_size = batch_size.max(1);
        let mut batch = Vec::with_capacity(batch_size);
        for entry in read_dir {
            let metadata = entry.metadata();
            batch.push(file_entry(
                entry.file_name(),
                entry.path(),
                &metadata,
                entry.file_type(),
                capabilities.clone(),
            ));
            if batch.len() >= batch_size && !emit(std::mem::take(&mut batch)) {
                return Err(CoreError::Cancelled);
            }
        }
        if !batch.is_empty() && !emit(batch) {
            return Err(CoreError::Cancelled);
        }
        Ok(())
    }

    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
        let sftp = self.require_sftp()?;
        let metadata = sftp.metadata(path).await.map_err(map_sftp_error)?;
        let name = path.rsplit('/').next().unwrap_or(path).to_owned();
        let file_type = file_type_from_mode(metadata.permissions);
        Ok(file_entry(
            name,
            path.to_owned(),
            &metadata,
            file_type,
            self.capabilities.clone(),
        ))
    }

    async fn mkdir(&mut self, path: &str) -> CoreResult<()> {
        self.require_sftp()?
            .create_dir(path)
            .await
            .map_err(map_sftp_error)
    }

    async fn rename(&mut self, from: &str, to: &str) -> CoreResult<()> {
        self.require_sftp()?
            .rename(from, to)
            .await
            .map_err(map_sftp_error)
    }

    async fn delete_file(&mut self, path: &str) -> CoreResult<()> {
        self.require_sftp()?
            .remove_file(path)
            .await
            .map_err(map_sftp_error)
    }

    async fn delete_dir(&mut self, path: &str) -> CoreResult<()> {
        self.require_sftp()?
            .remove_dir(path)
            .await
            .map_err(map_sftp_error)
    }

    async fn chmod(&mut self, path: &str, mode: u32) -> CoreResult<()> {
        let attributes = FileAttributes {
            permissions: Some(mode),
            ..FileAttributes::default()
        };
        self.require_sftp()?
            .set_metadata(path, attributes)
            .await
            .map_err(map_sftp_error)
    }

    async fn create_symlink(&mut self, target: &str, link_path: &str) -> CoreResult<()> {
        self.require_sftp()?
            .symlink(link_path, target)
            .await
            .map_err(map_sftp_error)
    }

    async fn download_to(
        &mut self,
        remote_path: &str,
        local_path: &Path,
        offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

        let sftp = self.require_sftp()?;
        let mut remote = sftp
            .open_with_flags(remote_path, OpenFlags::READ)
            .await
            .map_err(map_sftp_error)?;
        let mut local = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(offset == 0)
            .open(local_path)
            .await
            .map_err(map_io_error)?;
        remote
            .seek(SeekFrom::Start(offset))
            .await
            .map_err(map_io_error)?;
        local
            .seek(SeekFrom::Start(offset))
            .await
            .map_err(map_io_error)?;

        let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
        let mut moved = 0_u64;
        let mut keep_going = true;
        while keep_going {
            let read = remote.read(&mut buffer).await.map_err(map_io_error)?;
            if read == 0 {
                break;
            }
            local
                .write_all(&buffer[..read])
                .await
                .map_err(map_io_error)?;
            moved += read as u64;
            keep_going = progress(moved);
        }
        local.flush().await.map_err(map_io_error)?;
        let _ = remote.shutdown().await;
        Ok(moved)
    }

    async fn upload_from(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};

        let sftp = self.require_sftp()?;
        let flags = if offset == 0 {
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE
        } else {
            OpenFlags::CREATE | OpenFlags::WRITE
        };
        let mut remote = sftp
            .open_with_flags(remote_path, flags)
            .await
            .map_err(map_sftp_error)?;
        let mut local = tokio::fs::File::open(local_path)
            .await
            .map_err(map_io_error)?;
        if offset > 0 {
            remote
                .seek(SeekFrom::Start(offset))
                .await
                .map_err(map_io_error)?;
            local
                .seek(SeekFrom::Start(offset))
                .await
                .map_err(map_io_error)?;
        }

        let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
        let mut moved = 0_u64;
        let mut keep_going = true;
        while keep_going {
            let read = local.read(&mut buffer).await.map_err(map_io_error)?;
            if read == 0 {
                break;
            }
            remote
                .write_all(&buffer[..read])
                .await
                .map_err(map_io_error)?;
            moved += read as u64;
            keep_going = progress(moved);
        }
        remote.flush().await.map_err(map_io_error)?;
        let _ = remote.shutdown().await;
        Ok(moved)
    }
}

fn file_entry(
    name: String,
    path: String,
    metadata: &FileAttributes,
    file_type: FileType,
    capabilities: ProtocolCapabilities,
) -> FileEntry {
    let entry_type = match file_type {
        FileType::Dir => EntryType::Directory,
        FileType::File => EntryType::File,
        FileType::Symlink => EntryType::Symlink,
        _ => EntryType::Other,
    };
    let permissions = metadata
        .permissions
        .map(|mode| format!("{:o}", mode & 0o777));
    let ownership = metadata
        .user
        .clone()
        .or_else(|| metadata.uid.map(|uid| uid.to_string()))
        .map(|owner| Ownership {
            owner,
            group: metadata
                .group
                .clone()
                .or_else(|| metadata.gid.map(|gid| gid.to_string())),
        });
    let modified_at = metadata
        .mtime
        .and_then(|mtime| Utc.timestamp_opt(i64::from(mtime), 0).single());
    FileEntry {
        hidden: name.starts_with('.'),
        name,
        path,
        entry_type,
        size: (entry_type == EntryType::File)
            .then_some(metadata.size)
            .flatten(),
        modified_at,
        permissions,
        ownership,
        symlink_target: None,
        capabilities,
    }
}

fn file_type_from_mode(permissions: Option<u32>) -> FileType {
    match permissions.map(|mode| mode & 0o170000) {
        Some(0o040000) => FileType::Dir,
        Some(0o120000) => FileType::Symlink,
        Some(0o100000) => FileType::File,
        _ => FileType::Other,
    }
}

fn map_io_error(error: std::io::Error) -> CoreError {
    if error.kind() == std::io::ErrorKind::TimedOut {
        CoreError::Timeout(error.to_string())
    } else {
        CoreError::Network(error.to_string())
    }
}

fn map_connect_error(error: anyhow::Error, status: &Mutex<Option<HostKeyStatus>>) -> CoreError {
    if let Some(russh::Error::UnknownKey) = error.downcast_ref::<russh::Error>() {
        return match *status.lock().expect("host-key status lock") {
            Some(HostKeyStatus::Changed) => {
                CoreError::HostKeyChanged("the server key no longer matches known_hosts".to_owned())
            }
            _ => {
                CoreError::HostKeyUnknown("the server key is not present in known_hosts".to_owned())
            }
        };
    }
    match error.downcast_ref::<russh::Error>() {
        Some(russh::Error::ConnectionTimeout)
        | Some(russh::Error::KeepaliveTimeout)
        | Some(russh::Error::InactivityTimeout) => CoreError::Timeout(error.to_string()),
        Some(russh::Error::KeyChanged { line }) => {
            CoreError::HostKeyChanged(format!("known_hosts line {line}"))
        }
        Some(russh::Error::HUP)
        | Some(russh::Error::Disconnect)
        | Some(russh::Error::SendError)
        | Some(russh::Error::WrongChannel) => CoreError::Network(error.to_string()),
        _ => CoreError::internal(format!("SSH connect failed: {error}")),
    }
}

fn map_ssh_error(error: russh::Error) -> CoreError {
    match error {
        russh::Error::ConnectionTimeout
        | russh::Error::KeepaliveTimeout
        | russh::Error::InactivityTimeout => CoreError::Timeout(error.to_string()),
        russh::Error::UnknownKey => {
            CoreError::HostKeyUnknown("the server key is not trusted".to_owned())
        }
        russh::Error::KeyChanged { line } => {
            CoreError::HostKeyChanged(format!("known_hosts line {line}"))
        }
        russh::Error::HUP | russh::Error::Disconnect | russh::Error::SendError => {
            CoreError::Network(error.to_string())
        }
        russh::Error::NotAuthenticated | russh::Error::NoAuthMethod => {
            CoreError::AuthFailed(error.to_string())
        }
        _ => CoreError::internal(format!("SSH error: {error}")),
    }
}

fn map_sftp_error(error: russh_sftp::client::error::Error) -> CoreError {
    use russh_sftp::client::error::Error as SftpError;
    match error {
        SftpError::Status(status) => match status.status_code {
            StatusCode::NoSuchFile => CoreError::NotFound(status.error_message),
            StatusCode::PermissionDenied => CoreError::PermissionDenied(status.error_message),
            StatusCode::NoConnection | StatusCode::ConnectionLost => {
                CoreError::Network(status.error_message)
            }
            _ => CoreError::internal(format!(
                "SFTP status {}: {}",
                status.status_code as u32, status.error_message
            )),
        },
        SftpError::Timeout => CoreError::Timeout("SFTP operation timed out".to_owned()),
        SftpError::IO(message) => CoreError::Network(message),
        SftpError::Limited(message) => CoreError::UnsupportedCapability(message),
        SftpError::UnexpectedPacket | SftpError::UnexpectedBehavior(_) => {
            CoreError::internal(format!("SFTP protocol error: {error}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_openssh_fingerprints() {
        assert_eq!(normalize_fingerprint("SHA256:abc+/=="), "abc+/");
        assert_eq!(normalize_fingerprint("abc+/"), "abc+/");
    }

    #[test]
    fn reads_trusted_fingerprint_by_host_and_port() {
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("known_hosts.json");
        std::fs::write(&path, r#"{"example.com:22":"SHA256:abc+/=="}"#).unwrap();
        assert_eq!(
            trusted_fingerprint(&path, "example.com", 22).as_deref(),
            Some("abc+/")
        );
        assert!(trusted_fingerprint(&path, "example.com", 2222).is_none());
    }
}
