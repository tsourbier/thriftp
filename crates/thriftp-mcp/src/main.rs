//! Standalone `thriftp-mcp` server.

fn main() {
    let mut transport = "stdio".to_owned();
    let mut port = None;
    let mut token_file = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--transport" => {
                transport = arguments.next().unwrap_or_else(|| "stdio".to_owned());
            }
            "--port" => {
                port = arguments.next().and_then(|value| value.parse::<u16>().ok());
            }
            "--token-file" => {
                token_file = arguments.next().map(std::path::PathBuf::from);
            }
            "--token" => {
                eprintln!(
                    "thriftp-mcp: --token is not accepted; use --token-file with restrictive permissions"
                );
                std::process::exit(2);
            }
            "--help" | "-h" => {
                println!(
                    "thriftp-mcp [--transport stdio|loopback] [--port PORT] [--token-file PATH]"
                );
                return;
            }
            other => {
                eprintln!("thriftp-mcp: unknown argument {other}");
                std::process::exit(2);
            }
        }
    }
    let result = match transport.as_str() {
        "stdio" => thriftp_mcp::server::run_stdio(),
        "loopback" | "http" => thriftp_mcp::transport::run_loopback(port, token_file),
        other => {
            eprintln!("thriftp-mcp: unsupported transport {other}");
            std::process::exit(2);
        }
    };
    if let Err(error) = result {
        eprintln!("thriftp-mcp: {error}");
        if matches!(error, thriftp_core::CoreError::McpDisabled) {
            eprintln!("Enable MCP with `thriftp mcp enable` before starting the server.");
        }
        std::process::exit(1);
    }
}
