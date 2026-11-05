//! Loopback HTTP transport: token auth, origin rejection, and tool access.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;

fn start(home: &TempDir) -> (std::process::Child, String, String) {
    let mut store =
        thriftp_core::automation::AutomationStore::open(home.path().join("automation.json"))
            .unwrap();
    store.enable("standalone").unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_thriftp-mcp"))
        .env("THRIFTP_HOME", home.path())
        .args(["--transport", "loopback", "--port", "0"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn loopback mcp");
    let mut line = String::new();
    BufReader::new(child.stderr.take().expect("stderr"))
        .read_line(&mut line)
        .expect("endpoint line");
    let endpoint = line
        .split_whitespace()
        .find(|part| part.starts_with("http://"))
        .expect("loopback endpoint")
        .trim_end_matches(';')
        .trim_end_matches("/mcp")
        .to_owned();
    let token = std::fs::read_to_string(home.path().join("run").join("mcp.token"))
        .expect("token file")
        .trim()
        .to_owned();
    (child, endpoint, token)
}

fn request(endpoint: &str, token: &str, origin: Option<&str>) -> String {
    let address = endpoint.trim_start_matches("http://");
    let mut stream = TcpStream::connect(address).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let body = json!({"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}).to_string();
    let mut headers = format!(
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(origin) = origin {
        headers.push_str(&format!("Origin: {origin}\r\n"));
    }
    headers.push_str("\r\n");
    stream.write_all(headers.as_bytes()).unwrap();
    stream.write_all(body.as_bytes()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

#[test]
fn loopback_requires_the_per_launch_token_and_rejects_remote_origins() {
    let home = TempDir::new().unwrap();
    let (mut child, endpoint, token) = start(&home);

    let unauthorized = request(&endpoint, "wrong-token", None);
    assert!(unauthorized.starts_with("HTTP/1.1 401"), "{unauthorized}");

    let remote_origin = request(&endpoint, &token, Some("https://agent.example"));
    assert!(remote_origin.starts_with("HTTP/1.1 403"), "{remote_origin}");

    let authorized = request(&endpoint, &token, Some("http://127.0.0.1:5173"));
    assert!(authorized.starts_with("HTTP/1.1 200"), "{authorized}");
    assert!(authorized.contains("thriftp.connections.list"));

    let _ = child.kill();
    let _ = child.wait();
}
