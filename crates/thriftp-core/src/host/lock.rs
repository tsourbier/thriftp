//! Single-owner lock and runtime-file publication.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::Utc;
use uuid::Uuid;

use super::model::HostStatus;
use crate::automation::data_dir;
use crate::error::{CoreError, CoreResult};

pub struct HostLock {
    run_dir: PathBuf,
    released: bool,
}

impl HostLock {
    /// Acquire the single transfer-host lock in the default application data directory.
    pub fn acquire() -> CoreResult<Self> {
        Self::acquire_at(data_dir().join("run"))
    }

    /// Acquire the lock in an explicit runtime directory.
    pub fn acquire_at(run_dir: impl Into<PathBuf>) -> CoreResult<Self> {
        let run_dir = run_dir.into();
        fs::create_dir_all(&run_dir).map_err(io_error)?;
        let lock_path = run_dir.join("host.lock");
        if lock_path.exists() {
            if owner_is_alive(&lock_path) {
                return Err(CoreError::Conflict(
                    "another transfer host owns the queue".to_owned(),
                ));
            }
            cleanup_runtime_files(&run_dir);
            let _ = fs::remove_file(&lock_path);
        }
        let mut lock = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
            .map_err(io_error)?;
        writeln!(lock, "{}", std::process::id()).map_err(io_error)?;
        lock.sync_all().map_err(io_error)?;
        Ok(Self {
            run_dir,
            released: false,
        })
    }

    #[must_use]
    pub fn run_dir(&self) -> &Path {
        &self.run_dir
    }

    #[must_use]
    pub fn status_path(&self) -> PathBuf {
        self.run_dir.join("host.json")
    }

    #[must_use]
    pub fn token_path(&self) -> PathBuf {
        self.run_dir.join("host.token")
    }

    #[must_use]
    pub fn endpoint_path(&self) -> PathBuf {
        self.run_dir.join("control.sock")
    }

    /// Publish host metadata for control clients.
    pub fn publish(&self, status: &HostStatus) -> CoreResult<()> {
        let bytes = serde_json::to_vec_pretty(status)
            .map_err(|error| CoreError::internal(format!("host status: {error}")))?;
        write_private(&self.status_path(), &bytes)?;
        Ok(())
    }

    /// Publish the per-instance token with same-user permissions.
    pub fn publish_token(&self, token: &str) -> CoreResult<()> {
        write_private(&self.token_path(), token.as_bytes())
    }

    pub fn status(&self) -> CoreResult<HostStatus> {
        read_status(&self.run_dir)
    }

    pub fn release(mut self) -> CoreResult<()> {
        self.released = true;
        cleanup_runtime_files(&self.run_dir);
        match fs::remove_file(self.run_dir.join("host.lock")) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(io_error(error)),
        }
    }
}

impl Drop for HostLock {
    fn drop(&mut self) {
        if !self.released {
            cleanup_runtime_files(&self.run_dir);
            let _ = fs::remove_file(self.run_dir.join("host.lock"));
        }
    }
}

/// Read published host metadata from a runtime directory.
pub fn read_status(run_dir: &Path) -> CoreResult<HostStatus> {
    let bytes = fs::read(run_dir.join("host.json")).map_err(|error| {
        CoreError::HostUnavailable(format!("host metadata is unavailable: {error}"))
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| CoreError::HostUnavailable(format!("host metadata is invalid: {error}")))
}

/// Read the published control token from a runtime directory.
pub fn read_token(run_dir: &Path) -> CoreResult<String> {
    fs::read_to_string(run_dir.join("host.token"))
        .map(|token| token.trim().to_owned())
        .map_err(|error| CoreError::HostUnavailable(format!("host token is unavailable: {error}")))
}

/// Clean up stale runtime files without touching the lock itself.
pub fn cleanup_runtime_files(run_dir: &Path) {
    for name in ["host.json", "host.token", "control.sock"] {
        let _ = fs::remove_file(run_dir.join(name));
    }
}

fn owner_is_alive(lock_path: &Path) -> bool {
    let Ok(contents) = fs::read_to_string(lock_path) else {
        return false;
    };
    let Some(pid) = contents
        .lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
    else {
        return false;
    };
    pid == std::process::id() || process_is_alive(pid)
}

/// Best-effort PID liveness check without adding a platform dependency.
#[must_use]
pub fn process_is_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains(&pid.to_string()))
    }
}

/// Create a fresh host metadata value before the endpoint is bound.
#[must_use]
pub fn new_status(mode: &str) -> HostStatus {
    HostStatus {
        instance_id: Uuid::new_v4(),
        pid: std::process::id(),
        started_at: Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        state: "owner".to_owned(),
        mode: mode.to_owned(),
        endpoint: None,
        transport: String::new(),
        token_hash: String::new(),
        owned_transfer_ids: Vec::new(),
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
    }
    fs::rename(&temporary, path).map_err(io_error)
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::internal(format!("host I/O error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_owner_lock_and_stale_cleanup() {
        let home = tempfile::TempDir::new().unwrap();
        let run_dir = home.path().join("run");
        let lock = HostLock::acquire_at(&run_dir).unwrap();
        assert!(HostLock::acquire_at(&run_dir).is_err());
        lock.release().unwrap();
        let next = HostLock::acquire_at(&run_dir).unwrap();
        next.release().unwrap();

        fs::write(run_dir.join("host.lock"), b"not-a-pid").unwrap();
        let recovered = HostLock::acquire_at(&run_dir).unwrap();
        recovered.release().unwrap();
    }

    #[test]
    fn publishes_private_runtime_files() {
        let home = tempfile::TempDir::new().unwrap();
        let lock = HostLock::acquire_at(home.path().join("run")).unwrap();
        let status = new_status("standalone");
        lock.publish(&status).unwrap();
        lock.publish_token("token-value").unwrap();
        assert_eq!(
            read_status(lock.run_dir()).unwrap().instance_id,
            status.instance_id
        );
        assert_eq!(read_token(lock.run_dir()).unwrap(), "token-value");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(lock.token_path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        lock.release().unwrap();
    }
}
