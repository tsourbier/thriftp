//! Control client for the local transfer host.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use super::lock::{read_status, read_token};
use super::protocol::{RpcRequest, RpcResponse};
use super::server::ControlStream;
use crate::error::{CoreError, CoreResult};

pub struct HostClient {
    run_dir: PathBuf,
    stream: ControlStream,
    reader: BufReader<ControlStream>,
    token: String,
    next_id: u64,
}

/// Reader for host event notifications.
pub struct HostEventStream {
    reader: BufReader<ControlStream>,
}

impl HostEventStream {
    /// Read the next event notification, ignoring non-event responses.
    pub fn next_event(&mut self) -> CoreResult<Option<(String, Value)>> {
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => return Ok(None),
                Ok(_) => {}
                Err(_) => return Ok(None),
            }
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(line.trim()).map_err(|error| {
                CoreError::HostUnavailable(format!("invalid host event: {error}"))
            })?;
            if let Some(method) = value.get("method").and_then(Value::as_str)
                && method.starts_with("event.")
            {
                return Ok(Some((
                    method.to_owned(),
                    value.get("params").cloned().unwrap_or(Value::Null),
                )));
            }
        }
    }
}

impl HostClient {
    /// Open a separate connection and subscribe to host events.
    pub fn subscribe_at(
        run_dir: impl Into<PathBuf>,
        topics: &[&str],
    ) -> CoreResult<HostEventStream> {
        let client = Self::connect_at(run_dir)?;
        client.subscribe(topics)
    }

    /// Subscribe this client's connection to event topics.
    pub fn subscribe(mut self, topics: &[&str]) -> CoreResult<HostEventStream> {
        let _ = self.call("events.subscribe", json!({"topics": topics}))?;
        Ok(HostEventStream {
            reader: self.reader,
        })
    }

    /// Connect to the host published in `run_dir`.
    pub fn connect_at(run_dir: impl Into<PathBuf>) -> CoreResult<Self> {
        let run_dir = run_dir.into();
        let status = read_status(&run_dir)?;
        let endpoint = status.endpoint.as_deref().ok_or_else(|| {
            CoreError::HostUnavailable("the host has not published an endpoint".to_owned())
        })?;
        let token = read_token(&run_dir)?;
        let stream = connect_stream(endpoint)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(io_error)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(io_error)?;
        let reader = BufReader::new(stream.try_clone().map_err(io_error)?);
        Ok(Self {
            run_dir,
            stream,
            reader,
            token,
            next_id: 1,
        })
    }

    /// Connect if a host is published, otherwise report `host_unavailable`.
    pub fn try_connect() -> CoreResult<Self> {
        Self::connect_at(crate::automation::data_dir().join("run"))
    }

    /// Verify that a host is reachable and responds to `host.status`.
    pub fn is_available(run_dir: &Path) -> bool {
        Self::connect_at(run_dir)
            .and_then(|mut client| client.call("host.status", json!({})))
            .is_ok()
    }

    /// Perform one authenticated request, reconnecting once if the host
    /// closed an idle connection between calls.
    pub fn call(&mut self, method: &str, params: Value) -> CoreResult<Value> {
        match self.call_once(method, params.clone()) {
            Err(CoreError::HostLost) => {
                self.reconnect()?;
                self.call_once(method, params)
            }
            result => result,
        }
    }

    fn call_once(&mut self, method: &str, params: Value) -> CoreResult<Value> {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let request = RpcRequest {
            jsonrpc: super::protocol::JSONRPC_VERSION.to_owned(),
            id: Some(json!(id)),
            method: method.to_owned(),
            auth: Some(self.token.clone()),
            params,
        };
        let encoded = serde_json::to_string(&request)
            .map_err(|error| CoreError::internal(format!("control serialization: {error}")))?;
        writeln!(self.stream, "{encoded}").map_err(io_error)?;
        self.stream.flush().map_err(io_error)?;
        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .map_err(|_| CoreError::HostLost)?;
        if line.trim().is_empty() {
            return Err(CoreError::HostLost);
        }
        let response: RpcResponse = serde_json::from_str(line.trim()).map_err(|error| {
            CoreError::HostUnavailable(format!("invalid host response: {error}"))
        })?;
        if let Some(error) = response.error {
            let code = error
                .data
                .as_ref()
                .and_then(|data| data.get("code"))
                .and_then(Value::as_str)
                .unwrap_or("internal");
            return Err(CoreError::from_code(code, error.message));
        }
        response
            .result
            .ok_or_else(|| CoreError::HostUnavailable("empty host response".to_owned()))
    }

    fn reconnect(&mut self) -> CoreResult<()> {
        let status = read_status(&self.run_dir)?;
        let endpoint = status.endpoint.as_deref().ok_or_else(|| {
            CoreError::HostUnavailable("the host has not published an endpoint".to_owned())
        })?;
        let token = read_token(&self.run_dir)?;
        let stream = connect_stream(endpoint)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(io_error)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(30)))
            .map_err(io_error)?;
        self.reader = BufReader::new(stream.try_clone().map_err(io_error)?);
        self.stream = stream;
        self.token = token;
        Ok(())
    }

    pub fn submit(&mut self, params: Value) -> CoreResult<Value> {
        self.call("transfers.submit", params)
    }

    pub fn list_transfers(&mut self, state: Option<&str>) -> CoreResult<Value> {
        self.call("transfers.list", json!({"state": state, "limit": 1_000}))
    }

    pub fn status(&mut self) -> CoreResult<Value> {
        self.call("host.status", json!({}))
    }
}

// A dedicated variant keeps the client mapping precise without widening the
// public error taxonomy for a transport-only condition.

#[cfg(unix)]
fn connect_stream(endpoint: &str) -> CoreResult<ControlStream> {
    std::os::unix::net::UnixStream::connect(endpoint).map_err(|error| {
        CoreError::HostUnavailable(format!("cannot connect to transfer host: {error}"))
    })
}

#[cfg(windows)]
fn connect_stream(endpoint: &str) -> CoreResult<ControlStream> {
    std::net::TcpStream::connect(endpoint).map_err(|error| {
        CoreError::HostUnavailable(format!("cannot connect to transfer host: {error}"))
    })
}

fn io_error(error: std::io::Error) -> CoreError {
    CoreError::HostUnavailable(format!("control channel I/O error: {error}"))
}
