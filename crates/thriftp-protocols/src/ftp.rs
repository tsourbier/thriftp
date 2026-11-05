use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{NaiveDateTime, TimeZone, Utc};
use native_tls::TlsConnector;
use suppaftp::{FtpStream, NativeTlsConnector, NativeTlsFtpStream};
use thriftp_core::credentials::Secret;
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::model::{
    ConnectionProfile, EntryType, FileEntry, Ownership, Protocol, ProtocolCapabilities,
};
use thriftp_core::protocol::{ProgressCallback, RemoteFs, RemoteFsFactory, TRANSFER_CHUNK_SIZE};

enum FtpSession {
    Plain(FtpStream),
    Secure(NativeTlsFtpStream),
}

impl FtpSession {
    fn login(&mut self, username: &str, password: &str) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.login(username, password).map_err(map_ftp_error),
            Self::Secure(stream) => stream.login(username, password).map_err(map_ftp_error),
        }
    }

    fn quit(&mut self) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.quit().map_err(map_ftp_error),
            Self::Secure(stream) => stream.quit().map_err(map_ftp_error),
        }
    }

    fn mlsd(&mut self, path: &str) -> CoreResult<Vec<String>> {
        match self {
            Self::Plain(stream) => stream.mlsd(Some(path)).map_err(map_ftp_error),
            Self::Secure(stream) => stream.mlsd(Some(path)).map_err(map_ftp_error),
        }
    }

    fn list(&mut self, path: &str) -> CoreResult<Vec<String>> {
        match self {
            Self::Plain(stream) => stream.list(Some(path)).map_err(map_ftp_error),
            Self::Secure(stream) => stream.list(Some(path)).map_err(map_ftp_error),
        }
    }

    fn list_stream(
        &mut self,
        path: &str,
        batch_size: usize,
        capabilities: &ProtocolCapabilities,
        emit: &mut dyn FnMut(Vec<FileEntry>) -> bool,
    ) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream_list(stream, path, batch_size, capabilities, emit),
            Self::Secure(stream) => stream_list(stream, path, batch_size, capabilities, emit),
        }
    }

    fn mlst(&mut self, path: &str) -> CoreResult<String> {
        match self {
            Self::Plain(stream) => stream.mlst(Some(path)).map_err(map_ftp_error),
            Self::Secure(stream) => stream.mlst(Some(path)).map_err(map_ftp_error),
        }
    }

    fn mkdir(&mut self, path: &str) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.mkdir(path).map_err(map_ftp_error),
            Self::Secure(stream) => stream.mkdir(path).map_err(map_ftp_error),
        }
    }

    fn rename(&mut self, from: &str, to: &str) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.rename(from, to).map_err(map_ftp_error),
            Self::Secure(stream) => stream.rename(from, to).map_err(map_ftp_error),
        }
    }

    fn remove_file(&mut self, path: &str) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.rm(path).map_err(map_ftp_error),
            Self::Secure(stream) => stream.rm(path).map_err(map_ftp_error),
        }
    }

    fn remove_dir(&mut self, path: &str) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream.rmdir(path).map_err(map_ftp_error),
            Self::Secure(stream) => stream.rmdir(path).map_err(map_ftp_error),
        }
    }

    fn secure_data(&mut self) -> CoreResult<()> {
        match self {
            Self::Plain(_) => Ok(()),
            Self::Secure(stream) => {
                stream
                    .custom_command("PBSZ 0", &[suppaftp::Status::CommandOk])
                    .map_err(map_ftp_error)?;
                stream
                    .custom_command("PROT P", &[suppaftp::Status::CommandOk])
                    .map_err(map_ftp_error)?;
                Ok(())
            }
        }
    }

    fn set_binary(&mut self) -> CoreResult<()> {
        match self {
            Self::Plain(stream) => stream
                .transfer_type(suppaftp::types::FileType::Binary)
                .map_err(map_ftp_error),
            Self::Secure(stream) => stream
                .transfer_type(suppaftp::types::FileType::Binary)
                .map_err(map_ftp_error),
        }
    }

    fn chmod(&mut self, path: &str, mode: u32) -> CoreResult<()> {
        let command = format!("CHMOD {mode:o} {path}");
        match self {
            Self::Plain(stream) => stream.site(command).map_err(map_ftp_error).map(|_| ()),
            Self::Secure(stream) => stream.site(command).map_err(map_ftp_error).map(|_| ()),
        }
    }

    fn download(
        &mut self,
        remote_path: &str,
        local_path: &Path,
        offset: u64,
        progress: &ProgressCallback,
    ) -> CoreResult<u64> {
        match self {
            Self::Plain(stream) => {
                download_stream(stream, remote_path, local_path, offset, progress)
            }
            Self::Secure(stream) => {
                download_stream(stream, remote_path, local_path, offset, progress)
            }
        }
    }

    fn upload(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        offset: u64,
        progress: &ProgressCallback,
    ) -> CoreResult<u64> {
        match self {
            Self::Plain(stream) => upload_stream(stream, local_path, remote_path, offset, progress),
            Self::Secure(stream) => {
                upload_stream(stream, local_path, remote_path, offset, progress)
            }
        }
    }
}

fn download_stream<T: suppaftp::TlsStream>(
    stream: &mut suppaftp::ImplFtpStream<T>,
    remote_path: &str,
    local_path: &Path,
    offset: u64,
    progress: &ProgressCallback,
) -> CoreResult<u64> {
    if offset > 0 {
        stream
            .resume_transfer(offset as usize)
            .map_err(map_ftp_error)?;
    }
    let mut data = stream.retr_as_stream(remote_path).map_err(map_ftp_error)?;
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(offset == 0)
        .open(local_path)
        .map_err(map_io_error)?;
    file.seek(SeekFrom::Start(offset)).map_err(map_io_error)?;
    let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
    let mut moved = 0_u64;
    let mut keep_going = true;
    while keep_going {
        let read = data.read(&mut buffer).map_err(map_io_error)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read]).map_err(map_io_error)?;
        moved += read as u64;
        keep_going = progress(moved);
    }
    file.flush().map_err(map_io_error)?;
    if keep_going {
        data.finish().map_err(map_ftp_error)?;
    } else {
        stream.abort(data).map_err(map_ftp_error)?;
    }
    Ok(moved)
}

fn upload_stream<T: suppaftp::TlsStream>(
    stream: &mut suppaftp::ImplFtpStream<T>,
    local_path: &Path,
    remote_path: &str,
    offset: u64,
    progress: &ProgressCallback,
) -> CoreResult<u64> {
    let mut file = OpenOptions::new()
        .read(true)
        .open(local_path)
        .map_err(map_io_error)?;
    if offset > 0 {
        file.seek(SeekFrom::Start(offset)).map_err(map_io_error)?;
    }
    let mut data = if offset > 0 {
        stream
            .append_with_stream(remote_path)
            .map_err(map_ftp_error)?
    } else {
        stream.put_with_stream(remote_path).map_err(map_ftp_error)?
    };
    let mut buffer = vec![0_u8; TRANSFER_CHUNK_SIZE];
    let mut moved = 0_u64;
    let mut keep_going = true;
    while keep_going {
        let read = file.read(&mut buffer).map_err(map_io_error)?;
        if read == 0 {
            break;
        }
        data.write_all(&buffer[..read]).map_err(map_io_error)?;
        moved += read as u64;
        keep_going = progress(moved);
    }
    data.flush().map_err(map_io_error)?;
    if keep_going {
        data.finish().map_err(map_ftp_error)?;
    } else {
        stream.abort(data).map_err(map_ftp_error)?;
    }
    Ok(moved)
}

fn stream_list<T: suppaftp::TlsStream>(
    stream: &mut suppaftp::ImplFtpStream<T>,
    path: &str,
    batch_size: usize,
    capabilities: &ProtocolCapabilities,
    emit: &mut dyn FnMut(Vec<FileEntry>) -> bool,
) -> CoreResult<()> {
    let expected = [suppaftp::Status::AlreadyOpen, suppaftp::Status::AboutToSend];
    if let Ok((_, data)) = stream.custom_data_command(format!("MLSD {path}"), &expected) {
        return read_list_data(data, path, batch_size, capabilities, emit, true);
    }
    let (_, data) = stream
        .custom_data_command(format!("LIST {path}"), &expected)
        .map_err(map_ftp_error)?;
    read_list_data(data, path, batch_size, capabilities, emit, false)
}

fn read_list_data<T: suppaftp::TlsStream>(
    data: suppaftp::TransferStream<T>,
    path: &str,
    batch_size: usize,
    capabilities: &ProtocolCapabilities,
    emit: &mut dyn FnMut(Vec<FileEntry>) -> bool,
    mlsd: bool,
) -> CoreResult<()> {
    let mut reader = BufReader::new(data);
    let mut batch = Vec::with_capacity(batch_size.max(1));
    let mut line = String::new();
    loop {
        line.clear();
        let read = reader.read_line(&mut line).map_err(map_io_error)?;
        if read == 0 {
            break;
        }
        let text = line.trim();
        if text.is_empty() {
            continue;
        }
        let entries = if mlsd {
            vec![parse_mlsd_line(text, path, capabilities.clone())?]
        } else {
            parse_list_lines(&[text.to_owned()], path, capabilities.clone())?
        };
        for entry in entries {
            batch.push(entry);
            if batch.len() >= batch_size.max(1) && !emit(std::mem::take(&mut batch)) {
                return Err(CoreError::Cancelled);
            }
        }
    }
    if !batch.is_empty() && !emit(batch) {
        return Err(CoreError::Cancelled);
    }
    reader.into_inner().finish().map_err(map_ftp_error)?;
    Ok(())
}

pub struct FtpAdapter {
    protocol: Protocol,
    session: Option<Arc<Mutex<FtpSession>>>,
    capabilities: ProtocolCapabilities,
    tls_ca: Option<PathBuf>,
}

impl FtpAdapter {
    #[must_use]
    pub fn new(protocol: Protocol) -> Self {
        Self {
            protocol,
            session: None,
            capabilities: ProtocolCapabilities {
                resume: true,
                permissions: true,
                ownership: false,
                hidden_files: true,
                symbolic_links: true,
                server_side_rename: true,
                server_side_delete: true,
                max_path_length: None,
            },
            tls_ca: None,
        }
    }

    /// Trust an additional PEM certificate, used by the containerized test
    /// harness. Production connections use the operating-system trust store.
    #[must_use]
    pub fn with_tls_ca(mut self, path: impl Into<PathBuf>) -> Self {
        self.tls_ca = Some(path.into());
        self
    }

    fn require_session(&self) -> CoreResult<Arc<Mutex<FtpSession>>> {
        self.session
            .clone()
            .ok_or_else(|| CoreError::internal("FTP session is not connected"))
    }

    async fn run<T, F>(&self, operation: F) -> CoreResult<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut FtpSession) -> CoreResult<T> + Send + 'static,
    {
        let session = self.require_session()?;
        tokio::task::spawn_blocking(move || {
            let mut session = session
                .lock()
                .map_err(|_| CoreError::internal("FTP session lock poisoned"))?;
            operation(&mut session)
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP task failed: {error}")))?
    }
}

#[async_trait]
impl RemoteFs for FtpAdapter {
    async fn connect(
        &mut self,
        profile: &ConnectionProfile,
        credential: Option<Secret>,
    ) -> CoreResult<()> {
        let credential = credential.ok_or(CoreError::CredentialMissing)?;
        let password = String::from_utf8_lossy(credential.expose()).into_owned();
        let address = format!("{}:{}", profile.host, profile.port);
        let host = profile.host.clone();
        let username = profile.username.clone();
        let protocol = self.protocol;
        let tls_ca = self.tls_ca.clone();

        let session = tokio::task::spawn_blocking(move || -> CoreResult<FtpSession> {
            let mut session = match protocol {
                Protocol::Ftp => {
                    FtpSession::Plain(FtpStream::connect(&address).map_err(map_ftp_error)?)
                }
                Protocol::FtpsExplicit => {
                    let plain = NativeTlsFtpStream::connect(&address).map_err(map_ftp_error)?;
                    let connector =
                        NativeTlsConnector::from(build_tls_connector(tls_ca.as_deref())?);
                    FtpSession::Secure(plain.into_secure(connector, &host).map_err(map_ftp_error)?)
                }
                Protocol::FtpsImplicit => FtpSession::Secure(
                    NativeTlsFtpStream::connect_secure_implicit(
                        &address,
                        NativeTlsConnector::from(build_tls_connector(tls_ca.as_deref())?),
                        &host,
                    )
                    .map_err(map_ftp_error)?,
                ),
                Protocol::Sftp => {
                    return Err(CoreError::UnsupportedCapability(
                        "SFTP is not handled by the FTP adapter".to_owned(),
                    ));
                }
            };
            session.login(&username, &password)?;
            if protocol == Protocol::FtpsImplicit {
                session.secure_data()?;
            }
            session.set_binary()?;
            Ok(session)
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP connect task failed: {error}")))??;

        self.session = Some(Arc::new(Mutex::new(session)));
        Ok(())
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        let Some(session) = self.session.take() else {
            return Ok(());
        };
        tokio::task::spawn_blocking(move || {
            session
                .lock()
                .map_err(|_| CoreError::internal("FTP session lock poisoned"))?
                .quit()
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP disconnect task failed: {error}")))?
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        self.capabilities.clone()
    }

    async fn list_dir(&mut self, path: &str) -> CoreResult<Vec<FileEntry>> {
        let path = path.to_owned();
        let parse_path = path.clone();
        let capabilities = self.capabilities.clone();
        let mlsd = {
            let path = path.clone();
            self.run(move |session| session.mlsd(&path)).await
        };
        match mlsd {
            Ok(entries) => entries
                .into_iter()
                .map(|line| parse_mlsd_line(&line, &parse_path, capabilities.clone()))
                .collect(),
            Err(original) => {
                // Many servers (including vsftpd) do not implement MLSD.
                let listings = {
                    let path = path.clone();
                    self.run(move |session| session.list(&path)).await
                };
                match listings {
                    Ok(lines) => parse_list_lines(&lines, &parse_path, capabilities),
                    Err(_) => Err(original),
                }
            }
        }
    }

    async fn list_dir_stream(
        &mut self,
        path: &str,
        batch_size: usize,
        emit: &mut (dyn FnMut(Vec<FileEntry>) -> bool + Send),
    ) -> CoreResult<()> {
        let session = self.require_session()?;
        let path = path.to_owned();
        let capabilities = self.capabilities.clone();
        let (sender, mut receiver) = tokio::sync::mpsc::channel::<CoreResult<Vec<FileEntry>>>(8);
        tokio::task::spawn_blocking(move || {
            let mut session = match session.lock() {
                Ok(session) => session,
                Err(_) => {
                    let _ =
                        sender.blocking_send(Err(CoreError::internal("FTP session lock poisoned")));
                    return;
                }
            };
            let result = session.list_stream(&path, batch_size, &capabilities, &mut |batch| {
                sender.blocking_send(Ok(batch)).is_ok()
            });
            if let Err(error) = result {
                let _ = sender.blocking_send(Err(error));
            }
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP listing task failed: {error}")))?;

        while let Some(message) = receiver.recv().await {
            match message {
                Ok(batch) => {
                    if !emit(batch) {
                        return Err(CoreError::Cancelled);
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    async fn stat(&mut self, path: &str) -> CoreResult<FileEntry> {
        let path = path.to_owned();
        let parse_path = path.clone();
        let capabilities = self.capabilities.clone();
        let output = self.run(move |session| session.mlst(&path)).await?;
        let line = output
            .lines()
            .find(|line| line.contains("type="))
            .ok_or_else(|| CoreError::internal("FTP MLST response did not contain facts"))?;
        parse_mlsd_line(line, &parse_path, capabilities)
    }

    async fn mkdir(&mut self, path: &str) -> CoreResult<()> {
        let path = path.to_owned();
        self.run(move |session| session.mkdir(&path)).await
    }

    async fn rename(&mut self, from: &str, to: &str) -> CoreResult<()> {
        let from = from.to_owned();
        let to = to.to_owned();
        self.run(move |session| session.rename(&from, &to)).await
    }

    async fn delete_file(&mut self, path: &str) -> CoreResult<()> {
        let path = path.to_owned();
        self.run(move |session| session.remove_file(&path)).await
    }

    async fn delete_dir(&mut self, path: &str) -> CoreResult<()> {
        let path = path.to_owned();
        self.run(move |session| session.remove_dir(&path)).await
    }

    async fn chmod(&mut self, path: &str, mode: u32) -> CoreResult<()> {
        let path = path.to_owned();
        self.run(move |session| session.chmod(&path, mode)).await
    }

    async fn download_to(
        &mut self,
        remote_path: &str,
        local_path: &Path,
        offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        let session = self.require_session()?;
        let remote_path = remote_path.to_owned();
        let local_path = local_path.to_path_buf();
        tokio::task::spawn_blocking(move || {
            let mut session = session
                .lock()
                .map_err(|_| CoreError::internal("FTP session lock poisoned"))?;
            session.download(&remote_path, &local_path, offset, &progress)
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP download task failed: {error}")))?
    }

    async fn upload_from(
        &mut self,
        local_path: &Path,
        remote_path: &str,
        offset: u64,
        progress: ProgressCallback,
    ) -> CoreResult<u64> {
        let session = self.require_session()?;
        let local_path = local_path.to_path_buf();
        let remote_path = remote_path.to_owned();
        tokio::task::spawn_blocking(move || {
            let mut session = session
                .lock()
                .map_err(|_| CoreError::internal("FTP session lock poisoned"))?;
            session.upload(&local_path, &remote_path, offset, &progress)
        })
        .await
        .map_err(|error| CoreError::internal(format!("FTP upload task failed: {error}")))?
    }
}

pub struct FtpFactory;

impl RemoteFsFactory for FtpFactory {
    fn create(&self, protocol: Protocol) -> Box<dyn RemoteFs> {
        match protocol {
            Protocol::Ftp | Protocol::FtpsExplicit | Protocol::FtpsImplicit => {
                Box::new(FtpAdapter::new(protocol))
            }
            Protocol::Sftp => Box::new(UnsupportedAdapter),
        }
    }
}

struct UnsupportedAdapter;

#[async_trait]
impl RemoteFs for UnsupportedAdapter {
    async fn connect(
        &mut self,
        _profile: &ConnectionProfile,
        _credential: Option<Secret>,
    ) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP adapter is not implemented yet".to_owned(),
        ))
    }

    async fn disconnect(&mut self) -> CoreResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> ProtocolCapabilities {
        ProtocolCapabilities::default()
    }

    async fn list_dir(&mut self, _path: &str) -> CoreResult<Vec<FileEntry>> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn stat(&mut self, _path: &str) -> CoreResult<FileEntry> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn mkdir(&mut self, _path: &str) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn rename(&mut self, _from: &str, _to: &str) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn delete_file(&mut self, _path: &str) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn delete_dir(&mut self, _path: &str) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }

    async fn chmod(&mut self, _path: &str, _mode: u32) -> CoreResult<()> {
        Err(CoreError::UnsupportedCapability(
            "SFTP is not implemented".to_owned(),
        ))
    }
}

fn parse_list_lines(
    lines: &[String],
    directory: &str,
    capabilities: ProtocolCapabilities,
) -> CoreResult<Vec<FileEntry>> {
    use suppaftp::list::ListParser;

    let mut entries = Vec::new();
    for line in lines {
        let file = ListParser::parse_posix(line)
            .or_else(|_| ListParser::parse_dos(line))
            .map_err(|error| CoreError::internal(format!("invalid LIST line: {error}")))?;
        let name = file.name().to_owned();
        if name == "." || name == ".." {
            continue;
        }
        let entry_type = if file.is_directory() {
            EntryType::Directory
        } else if file.is_symlink() {
            EntryType::Symlink
        } else if file.is_file() {
            EntryType::File
        } else {
            EntryType::Other
        };
        let modified_at = {
            let system_time = file.modified();
            let duration = system_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            Utc.timestamp_opt(duration.as_secs() as i64, 0).single()
        };
        let ownership = file.uid().map(|uid| Ownership {
            owner: uid.to_string(),
            group: file.gid().map(|gid| gid.to_string()),
        });
        entries.push(FileEntry {
            hidden: name.starts_with('.'),
            path: join_remote_path(directory, &name),
            name,
            entry_type,
            size: (entry_type == EntryType::File).then_some(file.size() as u64),
            modified_at,
            permissions: None,
            ownership,
            symlink_target: file.symlink().map(|target| target.display().to_string()),
            capabilities: capabilities.clone(),
        });
    }
    entries.sort_by_key(|entry| entry.name.to_lowercase());
    Ok(entries)
}

fn join_remote_path(directory: &str, name: &str) -> String {
    if directory == "." || directory.is_empty() {
        name.to_owned()
    } else if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

pub fn parse_mlsd_line(
    line: &str,
    parent: &str,
    capabilities: ProtocolCapabilities,
) -> CoreResult<FileEntry> {
    let (facts, raw_name) = line
        .split_once("; ")
        .ok_or_else(|| CoreError::internal(format!("invalid MLSD line: {line}")))?;
    let facts: HashMap<&str, &str> = facts
        .split(';')
        .filter_map(|fact| fact.split_once('='))
        .collect();

    let name = raw_name.trim();
    let file_name = name.rsplit('/').next().unwrap_or(name).to_owned();
    let entry_type = match facts.get("type").copied() {
        Some("dir") => EntryType::Directory,
        Some("file") => EntryType::File,
        Some(kind) if kind.contains("slink") => EntryType::Symlink,
        _ => EntryType::Other,
    };
    let path = if name.starts_with('/') {
        name.to_owned()
    } else {
        format!("{}/{}", parent.trim_end_matches('/'), name)
    };
    let modified_at = facts
        .get("modify")
        .and_then(|value| NaiveDateTime::parse_from_str(value, "%Y%m%d%H%M%S").ok())
        .map(|value| Utc.from_utc_datetime(&value));
    let permissions = facts
        .get("unix.mode")
        .and_then(|value| u32::from_str_radix(value, 8).ok())
        .map(|value| format!("{value:o}"));
    let size = facts
        .get("size")
        .and_then(|value| value.parse::<u64>().ok());

    Ok(FileEntry {
        name: file_name.clone(),
        path,
        entry_type,
        size: (entry_type == EntryType::File).then_some(size).flatten(),
        modified_at,
        permissions,
        ownership: None,
        hidden: file_name.starts_with('.'),
        symlink_target: None,
        capabilities,
    })
}

fn map_ftp_error(error: suppaftp::FtpError) -> CoreError {
    match error {
        suppaftp::FtpError::ConnectionError(error) => map_io_error(error),
        suppaftp::FtpError::SecureError(message) => {
            CoreError::CertificateInvalid(format!("TLS handshake failed: {message}"))
        }
        suppaftp::FtpError::UnexpectedResponse(response) => match response.status.code() {
            530 => CoreError::CredentialRejected,
            421 | 425 | 426 => CoreError::Network(response.status.desc()),
            550 => CoreError::NotFound(response.status.desc()),
            553 => CoreError::Validation(response.status.desc()),
            code => CoreError::internal(format!(
                "FTP server replied {code}: {}",
                response.status.desc()
            )),
        },
        suppaftp::FtpError::InvalidAddress(error) => {
            CoreError::Validation(format!("invalid FTP address: {error}"))
        }
        suppaftp::FtpError::BadResponse => {
            CoreError::internal("FTP server returned a malformed response")
        }
        suppaftp::FtpError::DataConnectionAlreadyOpen => {
            CoreError::internal("FTP data connection is already open")
        }
    }
}

fn map_io_error(error: std::io::Error) -> CoreError {
    match error.kind() {
        std::io::ErrorKind::TimedOut => CoreError::Timeout(error.to_string()),
        _ => CoreError::Network(error.to_string()),
    }
}

fn tls_error(error: native_tls::Error) -> CoreError {
    CoreError::CertificateInvalid(format!("TLS setup failed: {error}"))
}

fn build_tls_connector(ca_path: Option<&Path>) -> CoreResult<TlsConnector> {
    let mut builder = TlsConnector::builder();
    if let Some(path) = ca_path {
        let pem = std::fs::read(path).map_err(|error| {
            CoreError::CertificateInvalid(format!(
                "could not read CA certificate {}: {error}",
                path.display()
            ))
        })?;
        let certificate = native_tls::Certificate::from_pem(&pem).map_err(tls_error)?;
        builder.add_root_certificate(certificate);
    }
    builder.build().map_err(tls_error)
}
