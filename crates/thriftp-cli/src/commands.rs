use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde_json::{Value, json};
use thriftp_core::automation::{AuditEvent, AuditStore, AutomationStore, path_in_scope};
use thriftp_core::browser;
use thriftp_core::connections::ConnectionService;
use thriftp_core::credentials::{CredentialService, CredentialStore, Secret};
use thriftp_core::error::{CoreError, CoreResult};
use thriftp_core::host::{HostClient, HostRuntime, HostServices};
use thriftp_core::model::{
    AuthMethod, ConnectionProfile, CredentialPolicy, CredentialRef, EntryType, FileEntry, Priority,
    Protocol, Transfer, TransferDirection, TransferQueue, TransferState,
};
use thriftp_core::protocol::plain_ftp_warning;
use thriftp_core::transfers::TransferControl;
use thriftp_core::transfers::engine::{ProgressObserver, execute_download, execute_upload};
use thriftp_credentials_keyring::KeyringCredentialStore;
use thriftp_protocols::ProtocolFactory;
use thriftp_storage_json::{JsonMetadataStore, JsonTransferStore};
use uuid::Uuid;

use crate::output::envelope;
use crate::store;

use crate::{
    AuditCommand, ClientsCommand, Command, ConfigCommand, McpCommand, ProfileCommand, QueueCommand,
    TransferCommand,
};

pub async fn run(command: Command, selected_profile: Option<&str>) -> (Value, i32) {
    tracing::debug!(
        profile = selected_profile.unwrap_or("<none>"),
        "cli command started"
    );
    match command {
        Command::Profiles { command } => finish("profiles", profiles(command).await),
        Command::Browse {
            path,
            local,
            all,
            long,
            limit,
        } => finish(
            "browse",
            browse(path, local, all, long, limit, selected_profile).await,
        ),
        Command::Stat { path, local } => finish("stat", stat(path, local, selected_profile).await),
        Command::Upload {
            paths,
            recursive,
            conflict,
            no_wait,
            preserve_symlinks,
        } => finish(
            "transfer.upload",
            upload(
                paths,
                recursive,
                conflict,
                no_wait,
                preserve_symlinks,
                selected_profile,
            )
            .await,
        ),
        Command::Download {
            paths,
            recursive,
            conflict,
            no_wait,
            preserve_symlinks,
        } => finish(
            "transfer.download",
            download(
                paths,
                recursive,
                conflict,
                no_wait,
                preserve_symlinks,
                selected_profile,
            )
            .await,
        ),
        Command::Mkdir {
            path,
            local,
            parents,
        } => finish("mkdir", mkdir(path, local, parents, selected_profile).await),
        Command::Rename {
            from,
            to,
            local,
            yes,
        } => finish(
            "rename",
            rename(from, to, local, yes, selected_profile).await,
        ),
        Command::Delete {
            paths,
            local,
            recursive,
            yes,
        } => finish(
            "delete",
            delete(paths, local, recursive, yes, selected_profile).await,
        ),
        Command::Transfer { command } => finish("transfer", transfer(command)),
        Command::Queue { command } => finish("queue", queue(command)),
        Command::Config { command } => finish("config", config(command)),
        Command::Mcp { command } => finish("mcp", mcp(command, selected_profile).await),
    }
}

fn finish(command: &str, result: CoreResult<Value>) -> (Value, i32) {
    let outcome = match &result {
        Ok(_) => "ok",
        Err(error) => error.code(),
    };
    tracing::debug!(command, outcome, "cli command finished");
    envelope(command, result)
}

fn host_services() -> HostServices {
    HostServices {
        metadata: Arc::new(JsonMetadataStore::new(store::metadata_path())),
        transfers: Arc::new(JsonTransferStore::new(store::transfer_path())),
        credentials: Arc::new(KeyringCredentialStore::new()) as Arc<dyn CredentialStore>,
        factory: Arc::new(ProtocolFactory),
    }
}

fn plain_ftp_warnings(profile: &ConnectionProfile) -> Vec<Value> {
    plain_ftp_warning(profile.protocol)
        .and_then(|warning| serde_json::to_value(warning).ok())
        .into_iter()
        .collect()
}

fn compact_entries(entries: &[FileEntry]) -> Value {
    Value::Array(
        entries
            .iter()
            .map(|entry| {
                json!({
                    "name": entry.name,
                    "path": entry.path,
                    "entryType": entry.entry_type,
                    "size": entry.size,
                    "hidden": entry.hidden,
                })
            })
            .collect(),
    )
}

fn create_local_symlink(target: &str, path: &Path) -> CoreResult<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, path)
            .map_err(|error| CoreError::internal(format!("could not create symlink: {error}")))
    }
    #[cfg(not(unix))]
    {
        let _ = (target, path);
        Err(CoreError::UnsupportedCapability(
            "preserving symbolic links is not supported on this platform".to_owned(),
        ))
    }
}

fn warn_plain_ftp(profile: &ConnectionProfile) {
    if let Some(warning) = plain_ftp_warning(profile.protocol) {
        eprintln!("warning: {}: {}", warning.code, warning.message);
    }
}

fn report_transfer(transfer: &Transfer) {
    eprintln!(
        "progress: {:?} {} -> {} ({}/{} bytes, {:?})",
        transfer.direction,
        transfer.source_path,
        transfer.destination_path,
        transfer.transferred_bytes,
        transfer
            .total_bytes
            .map_or_else(|| "?".to_owned(), |value| value.to_string()),
        transfer.state
    );
}

fn progress_observer() -> Option<ProgressObserver> {
    let last = Arc::new(Mutex::new(Instant::now()));
    Some(Arc::new(move |bytes| {
        let Ok(mut last) = last.lock() else {
            return;
        };
        if last.elapsed() >= Duration::from_secs(1) {
            eprintln!("progress: {bytes} bytes transferred");
            *last = Instant::now();
        }
    }))
}

fn with_warnings(mut value: Value, warnings: Vec<Value>) -> Value {
    if let Some(object) = value.as_object_mut() {
        object.insert("warnings".to_owned(), json!(warnings));
    }
    value
}

fn check_interrupted() -> CoreResult<()> {
    if crate::interrupted() {
        Err(CoreError::Interrupted)
    } else {
        Ok(())
    }
}

fn parse_protocol(value: &str) -> CoreResult<Protocol> {
    match value {
        "ftp" => Ok(Protocol::Ftp),
        "ftps-explicit" | "ftpsExplicit" => Ok(Protocol::FtpsExplicit),
        "ftps-implicit" | "ftpsImplicit" => Ok(Protocol::FtpsImplicit),
        "sftp" => Ok(Protocol::Sftp),
        other => Err(CoreError::validation(format!("unknown protocol {other}"))),
    }
}

fn parse_auth(value: &str) -> CoreResult<AuthMethod> {
    match value {
        "password" => Ok(AuthMethod::Password),
        "ssh-key" | "sshPrivateKey" => Ok(AuthMethod::SshPrivateKey),
        other => Err(CoreError::validation(format!(
            "unknown auth method {other}"
        ))),
    }
}

fn resolve_profile(
    state: &thriftp_core::repository::AppState,
    value: &str,
) -> CoreResult<ConnectionProfile> {
    state
        .connection_profiles
        .iter()
        .find(|profile| profile.id.to_string() == value || profile.display_name == value)
        .cloned()
        .ok_or_else(|| CoreError::NotFound(format!("connection profile {value}")))
}

fn operation_secret(profile: &ConnectionProfile) -> CoreResult<Option<Secret>> {
    // An explicit stdin secret applies to this invocation and must not depend
    // on the native credential store being available.
    if let Some(secret) = crate::cli_secret() {
        return Ok(Some(Secret::new(secret.expose().to_vec())));
    }
    stored_secret(profile)
}

fn credential_service() -> CredentialService {
    CredentialService::new(Box::new(KeyringCredentialStore::new()))
}

fn stored_secret(profile: &ConnectionProfile) -> CoreResult<Option<Secret>> {
    let reference = CredentialRef::for_profile(profile);
    match credential_service().get(&reference) {
        Ok(secret) => Ok(secret),
        Err(CoreError::CredentialStoreUnavailable(message)) => {
            Err(CoreError::CredentialStoreUnavailable(message))
        }
        Err(CoreError::CredentialMissing) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn profiles(command: ProfileCommand) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.profiles").entered();
    let mut state = store::load_metadata()?;
    match command {
        ProfileCommand::List => Ok(json!(state.connection_profiles)),
        ProfileCommand::Show { id } => {
            let profile = resolve_profile(&state, &id)?;
            warn_plain_ftp(&profile);
            let warnings = plain_ftp_warnings(&profile);
            Ok(with_warnings(json!(profile), warnings))
        }
        ProfileCommand::Create {
            name,
            protocol,
            host,
            port,
            username,
            auth,
            key_file,
            group,
            timeout,
        } => {
            let protocol = parse_protocol(&protocol)?;
            let auth = parse_auth(&auth)?;
            let mut profile =
                ConnectionProfile::new(name, protocol, host, port, username, auth, timeout);
            profile.group_label = group;
            profile.ssh_key_ref = key_file;
            thriftp_core::validation::validate_profile(&profile)?;
            if state
                .connection_profiles
                .iter()
                .any(|existing| existing.id == profile.id)
            {
                return Err(CoreError::Conflict("profile already exists".to_owned()));
            }
            if let Some(secret) = crate::cli_secret() {
                let reference = CredentialRef::for_profile(&profile);
                credential_service().set(&reference, Secret::new(secret.expose().to_vec()))?;
                state.credential_refs.push(reference);
            }
            state.connection_profiles.push(profile.clone());
            store::save_metadata(&state)?;
            let warnings = plain_ftp_warnings(&profile);
            Ok(with_warnings(json!(profile), warnings))
        }
        ProfileCommand::Update {
            id,
            name,
            host,
            port,
            username,
            group,
            timeout,
        } => {
            let mut profile = resolve_profile(&state, &id)?;
            let old_profile = profile.clone();
            if let Some(value) = name {
                profile.display_name = value;
            }
            if let Some(value) = host {
                profile.host = value;
            }
            if let Some(value) = port {
                profile.port = value;
            }
            if let Some(value) = username {
                profile.username = value;
            }
            if let Some(value) = group {
                profile.group_label = Some(value);
            }
            if let Some(value) = timeout {
                profile.timeout_seconds = value;
            }
            profile.updated_at = Utc::now();
            thriftp_core::validation::validate_profile(&profile)?;
            if !thriftp_core::validation::identity_matches(
                &profile,
                &CredentialRef::for_profile(&old_profile),
            ) {
                let old_reference = CredentialRef::for_profile(&old_profile);
                if let Some(index) = state
                    .server_favorites
                    .iter()
                    .position(|favorite| favorite.connection_profile_id == profile.id)
                {
                    state.server_favorites[index].credential_ref_id = None;
                    state.server_favorites[index].credential_policy = CredentialPolicy::Ask;
                }
                if !state
                    .server_favorites
                    .iter()
                    .any(|favorite| favorite.credential_ref_id == Some(old_reference.id))
                {
                    if let Some(reference) = state
                        .credential_refs
                        .iter()
                        .find(|item| item.id == old_reference.id)
                    {
                        let _ = credential_service().delete(reference);
                    }
                    state
                        .credential_refs
                        .retain(|item| item.id != old_reference.id);
                }
            }
            for existing in &mut state.connection_profiles {
                if existing.id == profile.id {
                    *existing = profile.clone();
                }
            }
            store::save_metadata(&state)?;
            let warnings = plain_ftp_warnings(&profile);
            Ok(with_warnings(json!(profile), warnings))
        }
        ProfileCommand::SetCredential { id } => {
            let Some(secret) = crate::cli_secret() else {
                return Err(CoreError::CredentialMissing);
            };
            let profile = resolve_profile(&state, &id)?;
            let reference = CredentialRef::for_profile(&profile);
            credential_service().set(&reference, Secret::new(secret.expose().to_vec()))?;
            if !state
                .credential_refs
                .iter()
                .any(|item| item.id == reference.id)
            {
                state.credential_refs.push(reference);
            }
            store::save_metadata(&state)?;
            Ok(json!({"updated": true, "profileId": profile.id}))
        }
        ProfileCommand::Delete { id, yes } => {
            let profile = resolve_profile(&state, &id)?;
            let directory_references = state
                .directory_favorites
                .iter()
                .filter(|favorite| favorite.connection_profile_id == profile.id)
                .count();
            let server_references = state
                .server_favorites
                .iter()
                .filter(|favorite| favorite.connection_profile_id == profile.id)
                .count();
            let references = directory_references + server_references;
            if references > 0 && !yes {
                return Err(CoreError::Conflict(format!(
                    "profile is referenced by {references} favorites; deletion requires --yes"
                )));
            }
            state
                .connection_profiles
                .retain(|item| item.id != profile.id);
            state
                .directory_favorites
                .retain(|favorite| favorite.connection_profile_id != profile.id);
            state
                .server_favorites
                .retain(|favorite| favorite.connection_profile_id != profile.id);
            state
                .tabs
                .retain(|tab| tab.connection_profile_id != Some(profile.id));
            let credential_reference = CredentialRef::for_profile(&profile);
            if !state
                .server_favorites
                .iter()
                .any(|favorite| favorite.credential_ref_id == Some(credential_reference.id))
            {
                if let Some(reference) = state
                    .credential_refs
                    .iter()
                    .find(|item| item.id == credential_reference.id)
                {
                    let _ = credential_service().delete(reference);
                }
                state
                    .credential_refs
                    .retain(|item| item.id != credential_reference.id);
            }
            store::save_metadata(&state)?;
            Ok(json!({
                "deleted": true,
                "profileId": profile.id,
                "removedFavoriteReferences": references,
                "warnings": if references > 0 {
                    vec![json!({
                        "code": "favorites_removed",
                        "message": format!("{references} favorites referencing the profile were removed")
                    })]
                } else {
                    Vec::<Value>::new()
                }
            }))
        }
        ProfileCommand::Test { id } => {
            let profile = resolve_profile(&state, &id)?;
            let secret = operation_secret(&profile)?;
            let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
            connections.load_profiles(vec![profile.clone()]);
            let active = connections.connect(profile.id, secret).await?;
            connections.disconnect(active.id).await?;
            Ok(json!({"ok": true, "profileId": profile.id}))
        }
        ProfileCommand::TrustHostKey {
            id,
            fingerprint,
            yes,
        } => {
            if !yes {
                return Err(CoreError::Conflict(
                    "host-key trust requires --yes".to_owned(),
                ));
            }
            let profile = resolve_profile(&state, &id)?;
            thriftp_protocols::trust_fingerprint(
                &thriftp_protocols::known_hosts_store_path(),
                &profile.host,
                profile.port,
                &fingerprint,
            )?;
            Ok(json!({
                "profileId": profile.id,
                "trustedFingerprint": thriftp_protocols::normalize_fingerprint(&fingerprint),
                "warnings": plain_ftp_warnings(&profile),
            }))
        }
        ProfileCommand::ForgetHostKey { id, yes } => {
            if !yes {
                return Err(CoreError::Conflict(
                    "host-key removal requires --yes".to_owned(),
                ));
            }
            let profile = resolve_profile(&state, &id)?;
            thriftp_protocols::forget_fingerprint(
                &thriftp_protocols::known_hosts_store_path(),
                &profile.host,
                profile.port,
            )?;
            Ok(json!({"profileId": profile.id, "forgotten": true}))
        }
    }
}

async fn browse(
    path: String,
    local: bool,
    all: bool,
    long: bool,
    limit: usize,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.browse").entered();
    if local || selected_profile.is_none() {
        let entries = browser::list_local(Path::new(&path))?;
        let entries: Vec<_> = entries
            .into_iter()
            .filter(|entry| all || !entry.hidden)
            .take(limit)
            .collect();
        return Ok(json!({
            "path": path,
            "entries": if long { serde_json::to_value(&entries).map_err(|error| CoreError::internal(format!("serialize entries: {error}")))? } else { compact_entries(&entries) },
            "nextCursor": null
        }));
    }
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, selected_profile.unwrap())?;
    warn_plain_ftp(&profile);
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    let entries = browser::list_remote(&mut **guard, &path).await?;
    let entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| all || !entry.hidden)
        .take(limit)
        .collect();
    Ok(json!({
        "path": path,
        "entries": if long { serde_json::to_value(&entries).map_err(|error| CoreError::internal(format!("serialize entries: {error}")))? } else { compact_entries(&entries) },
        "nextCursor": null,
        "warnings": if profile.protocol == Protocol::Ftp { vec![json!({"code": "unencrypted_transport"})] } else { Vec::<Value>::new() },
    }))
}

async fn stat(path: String, local: bool, selected_profile: Option<&str>) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.stat").entered();
    if local || selected_profile.is_none() {
        return Ok(json!(browser::stat_local(Path::new(&path))?));
    }
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, selected_profile.unwrap())?;
    warn_plain_ftp(&profile);
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    let entry = browser::stat_remote(&mut **guard, &path).await?;
    Ok(with_warnings(json!(entry), plain_ftp_warnings(&profile)))
}

async fn upload(
    mut paths: Vec<String>,
    recursive: bool,
    conflict: String,
    no_wait: bool,
    preserve_symlinks: bool,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.upload").entered();
    if paths.len() < 2 {
        return Err(CoreError::validation(
            "upload requires at least one source and a remote directory",
        ));
    }
    let remote_dir = paths.pop().unwrap_or_default();
    let sources = paths;
    if no_wait {
        let profile_name =
            selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
        let state = store::load_metadata()?;
        let profile = resolve_profile(&state, profile_name)?;
        warn_plain_ftp(&profile);
        let mut client = HostClient::try_connect().map_err(|_| {
            CoreError::HostUnavailable(
                "--no-wait requires a running transfer host; run without --no-wait".to_owned(),
            )
        })?;
        let result = client.submit(json!({
            "direction": "upload",
            "profileId": profile.id,
            "sources": sources,
            "destination": remote_dir,
            "recursive": recursive,
            "conflictPolicy": conflict,
            "priority": "normal",
            "execute": false
        }))?;
        return Ok(with_warnings(result, plain_ftp_warnings(&profile)));
    }
    let mut host = HostRuntime::acquire("standalone", host_services())?;
    let profile_name =
        selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, profile_name)?;
    warn_plain_ftp(&profile);
    if !host.is_host() {
        let params = json!({
            "direction": "upload",
            "profileId": profile.id,
            "sources": sources,
            "destination": remote_dir,
            "recursive": recursive,
            "conflictPolicy": conflict,
            "priority": "normal",
            "execute": true
        });
        let result = {
            let client = host.client().ok_or_else(|| {
                CoreError::HostUnavailable("control channel unavailable".to_owned())
            })?;
            client.submit(params)?
        };
        host.shutdown();
        return Ok(with_warnings(result, plain_ftp_warnings(&profile)));
    }
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;

    let mut transfer_records = Vec::new();
    for source in sources {
        check_interrupted()?;
        let local = PathBuf::from(&source);
        if local.is_dir() {
            if !recursive {
                return Err(CoreError::Conflict(
                    "directory upload requires --recursive".to_owned(),
                ));
            }
            upload_directory(
                &mut **guard,
                &local,
                &remote_dir,
                &profile,
                &conflict,
                preserve_symlinks,
                &mut transfer_records,
            )
            .await?;
        } else {
            let name = local
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let Some(destination) =
                resolve_remote_destination(&mut **guard, &remote_dir, &name, &conflict).await?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Upload,
                profile.id,
                source,
                destination,
                None,
            );
            let control = TransferControl::new();
            execute_upload(
                &mut **guard,
                &mut transfer,
                &local,
                &control,
                progress_observer(),
            )
            .await?;
            report_transfer(&transfer);
            transfer_records.push(transfer);
        }
    }
    record_transfers(&transfer_records)?;
    host.shutdown();
    Ok(with_warnings(
        json!({"ok": true, "transfers": transfer_records}),
        plain_ftp_warnings(&profile),
    ))
}

async fn download(
    mut paths: Vec<String>,
    recursive: bool,
    conflict: String,
    no_wait: bool,
    preserve_symlinks: bool,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.download").entered();
    if paths.len() < 2 {
        return Err(CoreError::validation(
            "download requires at least one source and a local directory",
        ));
    }
    let local_dir = paths.pop().unwrap_or_default();
    let sources = paths;
    if no_wait {
        let profile_name =
            selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
        let state = store::load_metadata()?;
        let profile = resolve_profile(&state, profile_name)?;
        warn_plain_ftp(&profile);
        let mut client = HostClient::try_connect().map_err(|_| {
            CoreError::HostUnavailable(
                "--no-wait requires a running transfer host; run without --no-wait".to_owned(),
            )
        })?;
        let result = client.submit(json!({
            "direction": "download",
            "profileId": profile.id,
            "sources": sources,
            "destination": local_dir,
            "recursive": recursive,
            "conflictPolicy": conflict,
            "priority": "normal",
            "execute": false
        }))?;
        return Ok(with_warnings(result, plain_ftp_warnings(&profile)));
    }
    let mut host = HostRuntime::acquire("standalone", host_services())?;
    let profile_name =
        selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, profile_name)?;
    warn_plain_ftp(&profile);
    if !host.is_host() {
        let params = json!({
            "direction": "download",
            "profileId": profile.id,
            "sources": sources,
            "destination": local_dir,
            "recursive": recursive,
            "conflictPolicy": conflict,
            "priority": "normal",
            "execute": true
        });
        let result = {
            let client = host.client().ok_or_else(|| {
                CoreError::HostUnavailable("control channel unavailable".to_owned())
            })?;
            client.submit(params)?
        };
        host.shutdown();
        return Ok(with_warnings(result, plain_ftp_warnings(&profile)));
    }
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    let destination_root = PathBuf::from(&local_dir);

    let mut transfer_records = Vec::new();
    for source in sources {
        check_interrupted()?;
        let entry = browser::stat_remote(&mut **guard, &source).await?;
        if entry.entry_type == EntryType::Symlink {
            if preserve_symlinks {
                let target = entry
                    .symlink_target
                    .as_deref()
                    .ok_or(CoreError::PathAmbiguous)?;
                let Some(local) =
                    resolve_local_destination(&destination_root, &entry.name, &conflict)?
                else {
                    continue;
                };
                create_local_symlink(target, &local)?;
                eprintln!("preserved symbolic link {} -> {target}", local.display());
            } else {
                eprintln!("warning: skipping symbolic link {}", entry.path);
            }
            continue;
        }
        if entry.entry_type == EntryType::Directory {
            if !recursive {
                return Err(CoreError::Conflict(
                    "directory download requires --recursive".to_owned(),
                ));
            }
            download_directory(
                &mut **guard,
                &source,
                &destination_root,
                &profile,
                &conflict,
                preserve_symlinks,
                &mut transfer_records,
            )
            .await?;
        } else {
            let Some(local) = resolve_local_destination(&destination_root, &entry.name, &conflict)?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Download,
                profile.id,
                source,
                local.display().to_string(),
                Some(entry.size.unwrap_or(0)),
            );
            let control = TransferControl::new();
            execute_download(
                &mut **guard,
                &mut transfer,
                &local,
                &control,
                progress_observer(),
            )
            .await?;
            report_transfer(&transfer);
            transfer_records.push(transfer);
        }
    }
    record_transfers(&transfer_records)?;
    host.shutdown();
    Ok(with_warnings(
        json!({"ok": true, "transfers": transfer_records}),
        plain_ftp_warnings(&profile),
    ))
}

async fn upload_directory(
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    local: &Path,
    remote_dir: &str,
    profile: &ConnectionProfile,
    conflict: &str,
    preserve_symlinks: bool,
    records: &mut Vec<Transfer>,
) -> CoreResult<()> {
    let target_dir = join_remote(
        remote_dir,
        &local.file_name().unwrap_or_default().to_string_lossy(),
    );
    browser::create_dir_remote(remote, &target_dir).await.ok();
    for entry in std::fs::read_dir(local).map_err(|error| CoreError::internal(error.to_string()))? {
        check_interrupted()?;
        let entry = entry.map_err(|error| CoreError::internal(error.to_string()))?;
        let path = entry.path();
        if path.is_dir() {
            Box::pin(upload_directory(
                remote,
                &path,
                &target_dir,
                profile,
                conflict,
                preserve_symlinks,
                records,
            ))
            .await?;
        } else {
            if path
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.file_type().is_symlink())
            {
                if preserve_symlinks {
                    return Err(CoreError::UnsupportedCapability(
                        "preserving symbolic links is not supported by this protocol".to_owned(),
                    ));
                }
                eprintln!("warning: skipping symbolic link {}", path.display());
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(destination) =
                resolve_remote_destination(remote, &target_dir, &name, conflict).await?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Upload,
                profile.id,
                path.display().to_string(),
                destination,
                None,
            );
            let control = TransferControl::new();
            execute_upload(remote, &mut transfer, &path, &control, progress_observer()).await?;
            report_transfer(&transfer);
            records.push(transfer);
        }
    }
    Ok(())
}

async fn download_directory(
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    remote_dir: &str,
    local_dir: &Path,
    profile: &ConnectionProfile,
    conflict: &str,
    preserve_symlinks: bool,
    records: &mut Vec<Transfer>,
) -> CoreResult<()> {
    let target = local_dir.join(remote_dir.rsplit('/').next().unwrap_or("download"));
    std::fs::create_dir_all(&target).map_err(|error| CoreError::internal(error.to_string()))?;
    let entries = browser::list_remote(remote, remote_dir).await?;
    for entry in entries {
        check_interrupted()?;
        if entry.entry_type == EntryType::Directory {
            Box::pin(download_directory(
                remote,
                &entry.path,
                &target,
                profile,
                conflict,
                preserve_symlinks,
                records,
            ))
            .await?;
        } else {
            if entry.entry_type == EntryType::Symlink {
                if preserve_symlinks {
                    let link_target = entry
                        .symlink_target
                        .as_deref()
                        .ok_or(CoreError::PathAmbiguous)?;
                    let Some(local) = resolve_local_destination(&target, &entry.name, conflict)?
                    else {
                        continue;
                    };
                    create_local_symlink(link_target, &local)?;
                    eprintln!(
                        "preserved symbolic link {} -> {link_target}",
                        local.display()
                    );
                } else {
                    eprintln!("warning: skipping symbolic link {}", entry.path);
                }
                continue;
            }
            let Some(local_path) = resolve_local_destination(&target, &entry.name, conflict)?
            else {
                continue;
            };
            let mut transfer = Transfer::new(
                TransferDirection::Download,
                profile.id,
                entry.path.clone(),
                local_path.display().to_string(),
                entry.size,
            );
            let control = TransferControl::new();
            execute_download(
                remote,
                &mut transfer,
                &local_path,
                &control,
                progress_observer(),
            )
            .await?;
            report_transfer(&transfer);
            records.push(transfer);
        }
    }
    Ok(())
}

fn join_remote(directory: &str, name: &str) -> String {
    if directory.ends_with('/') {
        format!("{directory}{name}")
    } else {
        format!("{directory}/{name}")
    }
}

async fn resolve_remote_destination(
    remote: &mut dyn thriftp_core::protocol::RemoteFs,
    directory: &str,
    name: &str,
    policy: &str,
) -> CoreResult<Option<String>> {
    let candidate = join_remote(directory, name);
    if remote.stat(&candidate).await.is_err() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            for suffix in 1..1000 {
                let candidate = join_remote(directory, &format!("{name}.{suffix}"));
                if remote.stat(&candidate).await.is_err() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict(
                "could not find a free destination name".to_owned(),
            ))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {candidate}"
        ))),
        other => Err(CoreError::validation(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn resolve_local_destination(
    directory: &Path,
    name: &str,
    policy: &str,
) -> CoreResult<Option<PathBuf>> {
    let candidate = directory.join(name);
    if !candidate.exists() {
        return Ok(Some(candidate));
    }
    match policy {
        "overwrite" => Ok(Some(candidate)),
        "skip" => Ok(None),
        "rename" => {
            for suffix in 1..1000 {
                let candidate = directory.join(format!("{name}.{suffix}"));
                if !candidate.exists() {
                    return Ok(Some(candidate));
                }
            }
            Err(CoreError::Conflict(
                "could not find a free destination name".to_owned(),
            ))
        }
        "fail" => Err(CoreError::Conflict(format!(
            "destination exists: {}",
            candidate.display()
        ))),
        other => Err(CoreError::validation(format!(
            "unknown conflict policy {other}"
        ))),
    }
}

fn record_transfers(transfers: &[Transfer]) -> CoreResult<()> {
    if transfers.is_empty() {
        return Ok(());
    }
    let mut state = store::load_transfers()?;
    if state.queue.concurrency_limit == 0 {
        state.queue = TransferQueue::default();
    }
    for transfer in transfers {
        state.transfers.push(transfer.clone());
        state.history.push(transfer.clone());
    }
    store::save_transfers(&state)
}

async fn mkdir(
    path: String,
    local: bool,
    parents: bool,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.mkdir").entered();
    if local {
        browser::create_dir_local(Path::new(&path))?;
        return Ok(json!({"created": path}));
    }
    let profile_name =
        selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, profile_name)?;
    warn_plain_ftp(&profile);
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    browser::create_dir_remote(&mut **guard, &path).await?;
    Ok(json!({"created": path, "parents": parents}))
}

async fn rename(
    from: String,
    to: String,
    local: bool,
    yes: bool,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.rename").entered();
    if local {
        browser::rename_local(Path::new(&from), Path::new(&to))?;
        return Ok(json!({"renamed": true, "from": from, "to": to}));
    }
    if !yes {
        return Err(CoreError::Conflict(
            "remote rename requires --yes".to_owned(),
        ));
    }
    let profile_name =
        selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, profile_name)?;
    warn_plain_ftp(&profile);
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    browser::rename_remote(&mut **guard, &from, &to).await?;
    Ok(json!({"renamed": true, "from": from, "to": to}))
}

async fn delete(
    paths: Vec<String>,
    local: bool,
    recursive: bool,
    yes: bool,
    selected_profile: Option<&str>,
) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.delete").entered();
    if !yes {
        return Err(CoreError::Conflict("deletion requires --yes".to_owned()));
    }
    if local {
        for path in &paths {
            browser::delete_local(Path::new(path))?;
        }
        return Ok(json!({"deleted": paths, "recursive": recursive}));
    }
    let profile_name =
        selected_profile.ok_or_else(|| CoreError::validation("--profile is required"))?;
    let state = store::load_metadata()?;
    let profile = resolve_profile(&state, profile_name)?;
    warn_plain_ftp(&profile);
    let secret = operation_secret(&profile)?;
    let mut connections = ConnectionService::new(Box::new(ProtocolFactory));
    connections.load_profiles(vec![profile.clone()]);
    let active = connections.connect(profile.id, secret).await?;
    let session = connections.session(active.id)?;
    let mut guard = session.lock().await;
    for path in &paths {
        let entry = browser::stat_remote(&mut **guard, path).await?;
        browser::delete_remote(&mut **guard, path, entry.entry_type, true).await?;
    }
    Ok(json!({"deleted": paths, "recursive": recursive, "permanent": true}))
}

fn transfer_warnings(transfers: &[Transfer]) -> Vec<Value> {
    let Ok(metadata) = store::load_metadata() else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    let mut warnings = Vec::new();
    for transfer in transfers {
        if seen.contains(&transfer.connection_profile_id) {
            continue;
        }
        if let Some(profile) = metadata
            .connection_profiles
            .iter()
            .find(|profile| profile.id == transfer.connection_profile_id)
        {
            seen.push(profile.id);
            warn_plain_ftp(profile);
            warnings.extend(plain_ftp_warnings(profile));
        }
    }
    warnings
}

fn host_delegate(method: &str, params: Value) -> CoreResult<Option<Value>> {
    match HostClient::try_connect() {
        Ok(mut client) => Ok(Some(client.call(method, params)?)),
        Err(_) => Ok(None),
    }
}

fn transfer(command: TransferCommand) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.transfer").entered();
    let mut state = store::load_transfers()?;
    match command {
        TransferCommand::List {
            state: filter,
            limit,
        } => {
            let mut transfers: Vec<_> = state
                .transfers
                .iter()
                .filter(|transfer| {
                    filter.as_deref().is_none_or(|value| {
                        format!("{:?}", transfer.state).eq_ignore_ascii_case(value)
                    })
                })
                .cloned()
                .collect();
            transfers.sort_by_key(|transfer| transfer.created_at);
            transfers.truncate(limit);
            let warnings = transfer_warnings(&transfers);
            Ok(json!({"transfers": transfers, "warnings": warnings}))
        }
        TransferCommand::Status { id } => {
            let transfer = find_transfer(&state.transfers, &id)?;
            let warnings = transfer_warnings(std::slice::from_ref(&transfer));
            Ok(with_warnings(json!(transfer), warnings))
        }
        TransferCommand::Wait { id } => {
            let run_dir = thriftp_core::automation::data_dir().join("run");
            let mut transfer = find_transfer(&state.transfers, &id)?;
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
            while transfer.state == TransferState::Active {
                if !HostClient::is_available(&run_dir) {
                    return Err(CoreError::HostLost);
                }
                if std::time::Instant::now() > deadline {
                    return Err(CoreError::Timeout("waiting for transfer".to_owned()));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                state = store::load_transfers()?;
                transfer = find_transfer(&state.transfers, &id)?;
            }
            if transfer.state == TransferState::Interrupted {
                return Err(CoreError::Interrupted);
            }
            let warnings = transfer_warnings(std::slice::from_ref(&transfer));
            Ok(with_warnings(json!(transfer), warnings))
        }
        TransferCommand::Pause { id } => {
            if let Some(result) = host_delegate("transfers.pause", json!({"transferId": id}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            let transfer = set_transfer_state(&mut state.transfers, &id, TransferState::Paused)?;
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(transfer))
        }
        TransferCommand::Resume { id } => {
            if let Some(result) = host_delegate("transfers.resume", json!({"transferId": id}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            let transfer_id = parse_uuid(&id)?;
            if let Some(partial) = state
                .partial_transfers
                .iter()
                .find(|partial| partial.transfer_id == transfer_id)
            {
                if partial
                    .source_size
                    .is_some_and(|size| partial.offset > size)
                {
                    return Err(CoreError::Conflict(
                        "resume offset is larger than the source".to_owned(),
                    ));
                }
                if let Some(local) = &partial.local_path {
                    let local_size = std::fs::metadata(local).map(|metadata| metadata.len());
                    if local_size.is_ok_and(|size| size < partial.offset) {
                        return Err(CoreError::Conflict(
                            "local partial file is shorter than the resume offset".to_owned(),
                        ));
                    }
                }
            }
            let transfer = set_transfer_state(&mut state.transfers, &id, TransferState::Pending)?;
            store::save_transfers(&state)?;
            host.shutdown();
            let warnings = transfer_warnings(std::slice::from_ref(&transfer));
            Ok(with_warnings(json!(transfer), warnings))
        }
        TransferCommand::Cancel { id, yes } => {
            if !yes {
                return Err(CoreError::Conflict(
                    "cancellation requires --yes".to_owned(),
                ));
            }
            if let Some(result) = host_delegate("transfers.cancel", json!({"transferId": id}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            let transfer = set_transfer_state(&mut state.transfers, &id, TransferState::Cancelled)?;
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(transfer))
        }
        TransferCommand::Retry { id } => {
            let transfer = set_transfer_state(&mut state.transfers, &id, TransferState::Pending)?;
            store::save_transfers(&state)?;
            Ok(json!(transfer))
        }
        TransferCommand::Priority { id, priority } => {
            if let Some(result) = host_delegate(
                "transfers.priority",
                json!({"transferId": id, "priority": priority}),
            )? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            let priority = match priority.as_str() {
                "low" => Priority::Low,
                "high" => Priority::High,
                _ => Priority::Normal,
            };
            let mut transfer = find_transfer(&state.transfers, &id)?;
            transfer.priority = priority;
            transfer.updated_at = Utc::now();
            for existing in &mut state.transfers {
                if existing.id == transfer.id {
                    *existing = transfer.clone();
                }
            }
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(transfer))
        }
        TransferCommand::Reorder { ids } => {
            if let Some(result) = host_delegate("transfers.reorder", json!({"transferIds": ids}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            let mut order = Vec::new();
            for id in ids {
                order.push(parse_uuid(&id)?);
            }
            state.queue.ordered_transfer_ids = order.clone();
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(order))
        }
    }
}

fn queue(command: QueueCommand) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.queue").entered();
    let mut state = store::load_transfers()?;
    match command {
        QueueCommand::List => Ok(json!(state.queue)),
        QueueCommand::Pause => {
            if let Some(result) = host_delegate("queue.pause", json!({}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            state.queue.paused = true;
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(state.queue))
        }
        QueueCommand::Resume => {
            if let Some(result) = host_delegate("queue.resume", json!({}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            state.queue.paused = false;
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!(state.queue))
        }
        QueueCommand::CancelAll { yes } => {
            if !yes {
                return Err(CoreError::Conflict(
                    "queue cancellation requires --yes".to_owned(),
                ));
            }
            if let Some(result) = host_delegate("queue.cancelAll", json!({"confirmed": true}))? {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            for transfer in &mut state.transfers {
                if transfer.state == TransferState::Pending {
                    transfer.state = TransferState::Cancelled;
                }
            }
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!({"cancelled": true}))
        }
    }
}

fn config(command: ConfigCommand) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.config").entered();
    let mut state = store::load_transfers()?;
    match command {
        ConfigCommand::Concurrency { value } => {
            if let Some(result) =
                host_delegate("queue.setConcurrency", json!({"concurrency": value}))?
            {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            state.queue.concurrency_limit = value.max(1);
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(json!({"concurrency": state.queue.concurrency_limit}))
        }
        ConfigCommand::Bandwidth { value } => {
            let delegated = if value == "unlimited" {
                None
            } else {
                Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| CoreError::validation("invalid bandwidth"))?,
                )
            };
            if let Some(result) =
                host_delegate("queue.setBandwidth", json!({"bytesPerSecond": delegated}))?
            {
                return Ok(result);
            }
            let host = HostRuntime::acquire("cli", host_services())?;
            state.queue.bandwidth_limit_bytes_per_second = if value == "unlimited" {
                None
            } else {
                Some(
                    value
                        .parse::<u64>()
                        .map_err(|_| CoreError::validation("invalid bandwidth"))?,
                )
            };
            store::save_transfers(&state)?;
            host.shutdown();
            Ok(
                json!({"bandwidthLimitBytesPerSecond": state.queue.bandwidth_limit_bytes_per_second}),
            )
        }
    }
}

async fn mcp(command: McpCommand, selected_profile: Option<&str>) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.mcp").entered();
    let mut store = AutomationStore::open_default()?;
    match command {
        McpCommand::Enable { mode } => {
            store.enable(&mode)?;
            audit("mcp.enable", "allowed", None)?;
            Ok(store.status())
        }
        McpCommand::Disable => {
            store.disable()?;
            audit("mcp.disable", "allowed", None)?;
            Ok(store.status())
        }
        McpCommand::Status => {
            let mut status = store.status();
            if let Some(object) = status.as_object_mut() {
                object.insert("sessions".to_owned(), json!(active_sessions()));
            }
            Ok(status)
        }
        McpCommand::Clients { command } => match command {
            ClientsCommand::List => {
                let sessions = active_sessions();
                let clients: Vec<Value> = store
                    .clients()
                    .into_iter()
                    .map(|client| {
                        let mut value = serde_json::to_value(&client).unwrap_or_else(|_| json!({}));
                        let ids: Vec<&str> = sessions
                            .iter()
                            .filter(|session| {
                                session.get("fingerprint").and_then(Value::as_str)
                                    == Some(client.fingerprint.as_str())
                            })
                            .filter_map(|session| session.get("token").and_then(Value::as_str))
                            .collect();
                        if let Some(object) = value.as_object_mut() {
                            object.insert("activeSessions".to_owned(), json!(ids.len()));
                            object.insert("sessionIds".to_owned(), json!(ids));
                        }
                        value
                    })
                    .collect();
                Ok(json!(clients))
            }
            ClientsCommand::Pending => Ok(json!(store.pending())),
            ClientsCommand::Approve {
                request_id,
                dirs,
                local_dirs,
                operations,
                expires,
                disconnect_policy,
                pre_approve,
                yes,
            } => {
                if !yes {
                    return Err(CoreError::Conflict(
                        "grant approval requires --yes".to_owned(),
                    ));
                }
                let state = store::load_metadata()?;
                let profile_values: Vec<&str> = selected_profile
                    .map(|value| value.split(',').collect())
                    .unwrap_or_default();
                if profile_values.is_empty() {
                    return Err(CoreError::validation("--profile is required for approval"));
                }
                let profile_ids: Vec<Uuid> = profile_values
                    .iter()
                    .map(|value| resolve_profile(&state, value).map(|profile| profile.id))
                    .collect::<CoreResult<_>>()?;
                let mut scopes = Vec::new();
                for dir in dirs {
                    let (profile, path) = dir
                        .split_once(':')
                        .ok_or_else(|| CoreError::validation("--dir must be profile:path"))?;
                    let profile_id = resolve_profile(&state, profile)?.id;
                    scopes.push(("remote".to_owned(), Some(profile_id), path.to_owned()));
                }
                for path in local_dirs {
                    scopes.push(("local".to_owned(), None, path));
                }
                let expires_at = expires
                    .map(|value| {
                        DateTime::parse_from_rfc3339(&value).map(|value| value.with_timezone(&Utc))
                    })
                    .transpose()
                    .map_err(|_| CoreError::validation("invalid expiry"))?;
                let grant = store.approve(
                    parse_uuid(&request_id)?,
                    profile_ids,
                    scopes,
                    operations,
                    expires_at,
                    &disconnect_policy,
                    "cli",
                )?;
                if !pre_approve.is_empty() {
                    store.add_policy(
                        grant.id,
                        pre_approve,
                        grant.scope_ids.clone(),
                        expires_at,
                        false,
                    )?;
                }
                audit("mcp.clients.approve", "allowed", Some(grant.id))?;
                Ok(json!(grant))
            }
            ClientsCommand::Deny { request_id } => {
                store.deny(parse_uuid(&request_id)?)?;
                audit("mcp.clients.deny", "allowed", None)?;
                Ok(json!({"denied": true}))
            }
            ClientsCommand::Revoke { fingerprint, yes } => {
                if !yes {
                    return Err(CoreError::Conflict("revocation requires --yes".to_owned()));
                }
                store.revoke(&fingerprint)?;
                audit("mcp.clients.revoke", "allowed", None)?;
                Ok(json!({"revoked": true, "fingerprint": fingerprint}))
            }
            ClientsCommand::Disconnect { fingerprint } => {
                let request = thriftp_core::automation::disconnect::request(&fingerprint)?;
                audit("mcp.clients.disconnect", "allowed", None)?;
                Ok(json!({
                    "disconnected": true,
                    "fingerprint": fingerprint,
                    "requestId": request.request_id
                }))
            }
        },
        McpCommand::Tools => Ok(tool_catalog()),
        McpCommand::Serve => {
            thriftp_mcp::server::run_stdio()?;
            Ok(json!({"stopped": true}))
        }
        McpCommand::Confirm {
            fingerprint,
            operation,
            target,
            ttl,
        } => {
            let grant = store
                .grant_for(&fingerprint)
                .cloned()
                .ok_or(CoreError::GrantMissing)?;
            let confirmation = thriftp_core::automation::confirmation::issue(
                grant.id,
                &operation,
                target.as_deref(),
                ttl,
            )?;
            audit("mcp.confirmations.issue", "allowed", Some(grant.id))?;
            Ok(json!({
                "confirmationId": confirmation.id,
                "expiresAt": confirmation.expires_at,
                "operation": confirmation.operation,
                "target": confirmation.target
            }))
        }
        McpCommand::Audit { command } => audit_command(command),
    }
}

fn active_sessions() -> Vec<Value> {
    let path = thriftp_core::automation::data_dir()
        .join("run")
        .join("mcp-sessions.json");
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let Ok(sessions) = serde_json::from_slice::<Vec<Value>>(&bytes) else {
        return Vec::new();
    };
    let now = Utc::now();
    sessions
        .into_iter()
        .filter(|session| {
            session
                .get("expiresAt")
                .and_then(Value::as_str)
                .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
                .is_some_and(|expires| expires.with_timezone(&Utc) > now)
        })
        .collect()
}

fn audit_command(command: AuditCommand) -> CoreResult<Value> {
    let _span = tracing::info_span!("cli.audit").entered();
    let store = AuditStore::open_default()?;
    match command {
        AuditCommand::List {
            since,
            until,
            client,
            result,
            limit,
        } => {
            let events = store.list(limit)?;
            let events: Vec<_> = events
                .into_iter()
                .filter(|event| {
                    let stamp = event.timestamp.to_rfc3339();
                    since.as_deref().is_none_or(|value| stamp.as_str() >= value)
                        && until.as_deref().is_none_or(|value| stamp.as_str() <= value)
                        && client
                            .as_deref()
                            .is_none_or(|value| event.client_fingerprint.as_deref() == Some(value))
                        && result.as_deref().is_none_or(|value| event.result == value)
                })
                .collect();
            Ok(json!(events))
        }
        AuditCommand::Export { format, out } => {
            let events = store.list(usize::MAX)?;
            let content = if format == "json" {
                serde_json::to_string_pretty(&events).unwrap_or_else(|_| "[]".to_owned())
            } else {
                events
                    .iter()
                    .map(|event| serde_json::to_string(event).unwrap_or_default())
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            std::fs::write(&out, content)
                .map_err(|error| CoreError::internal(error.to_string()))?;
            Ok(json!({"exported": events.len(), "path": out}))
        }
        AuditCommand::Verify => Ok(json!({"valid": store.verify()?})),
        AuditCommand::Retention { days } => {
            let mut store = AutomationStore::open_default()?;
            store.state.settings.audit_retention_days = days.clamp(1, 3650);
            store.save()?;
            Ok(json!({"retentionDays": store.state.settings.audit_retention_days}))
        }
        AuditCommand::Limit { bytes } => {
            let mut store = AutomationStore::open_default()?;
            store.state.settings.audit_max_bytes = if bytes == "unlimited" {
                None
            } else {
                Some(
                    bytes
                        .parse::<u64>()
                        .map_err(|_| CoreError::validation("invalid limit"))?,
                )
            };
            store.save()?;
            Ok(json!({"auditMaxBytes": store.state.settings.audit_max_bytes}))
        }
    }
}

fn audit(operation: &str, result: &str, _grant: Option<Uuid>) -> CoreResult<()> {
    let store = AuditStore::open_default()?;
    let mut event = AuditEvent {
        seq: 0,
        timestamp: Utc::now(),
        interface: "mcp".to_owned(),
        mode: "standalone".to_owned(),
        client_fingerprint: None,
        client_display_name: None,
        session_id: None,
        operation: operation.to_owned(),
        target: None,
        result: result.to_owned(),
        error_code: None,
        duration_ms: None,
        details: None,
        prev_hash: String::new(),
        hash: String::new(),
    };
    store.append(&mut event)
}

fn tool_catalog() -> Value {
    json!([
        {"name":"thriftp.connections.list","description":"List granted connection profiles","destructive":false},
        {"name":"thriftp.connections.get","description":"Get one granted profile","destructive":false},
        {"name":"thriftp.browse.list","description":"List a remote directory","destructive":false},
        {"name":"thriftp.files.stat","description":"Inspect a remote file","destructive":false},
        {"name":"thriftp.directories.create","description":"Create a remote directory","destructive":false},
        {"name":"thriftp.directories.rename","description":"Rename a remote path","destructive":true},
        {"name":"thriftp.directories.delete","description":"Delete a remote directory","destructive":true},
        {"name":"thriftp.files.delete","description":"Delete a remote file","destructive":true},
        {"name":"thriftp.transfers.upload","description":"Upload files or directories","destructive":false},
        {"name":"thriftp.transfers.download","description":"Download files or directories","destructive":false},
        {"name":"thriftp.transfers.list","description":"List transfers","destructive":false},
        {"name":"thriftp.transfers.status","description":"Get transfer status","destructive":false},
        {"name":"thriftp.transfers.pause","description":"Pause a transfer","destructive":false},
        {"name":"thriftp.transfers.resume","description":"Resume a transfer","destructive":false},
        {"name":"thriftp.transfers.cancel","description":"Cancel a transfer","destructive":false},
        {"name":"thriftp.transfers.priority","description":"Set transfer priority","destructive":false},
        {"name":"thriftp.transfers.reorder","description":"Reorder queued transfers","destructive":false},
        {"name":"thriftp.queue.get","description":"Get queue settings","destructive":false},
        {"name":"thriftp.queue.set_concurrency","description":"Set queue concurrency","destructive":false},
        {"name":"thriftp.queue.set_bandwidth","description":"Set global bandwidth","destructive":false},
        {"name":"thriftp.automation.status","description":"Automation status","destructive":false},
        {"name":"thriftp.automation.enroll","description":"Submit enrollment","destructive":false},
        {"name":"thriftp.automation.enrollment_status","description":"Enrollment status","destructive":false},
        {"name":"thriftp.automation.challenge","description":"Request session challenge","destructive":false},
        {"name":"thriftp.automation.authenticate","description":"Authenticate a session","destructive":false},
        {"name":"thriftp.automation.session","description":"Describe the current session","destructive":false}
    ])
}

fn find_transfer(transfers: &[Transfer], id: &str) -> CoreResult<Transfer> {
    let id = parse_uuid(id)?;
    transfers
        .iter()
        .find(|transfer| transfer.id == id)
        .cloned()
        .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))
}

fn set_transfer_state(
    transfers: &mut [Transfer],
    id: &str,
    state: TransferState,
) -> CoreResult<Transfer> {
    let id = parse_uuid(id)?;
    let transfer = transfers
        .iter_mut()
        .find(|transfer| transfer.id == id)
        .ok_or_else(|| CoreError::NotFound("transfer".to_owned()))?;
    transfer.state = state;
    transfer.updated_at = Utc::now();
    Ok(transfer.clone())
}

fn parse_uuid(value: &str) -> CoreResult<Uuid> {
    Uuid::parse_str(value).map_err(|_| CoreError::validation("invalid UUID"))
}

#[allow(dead_code)]
fn scope_contains(path: &str, scope: &str) -> bool {
    path_in_scope(path, scope)
}
