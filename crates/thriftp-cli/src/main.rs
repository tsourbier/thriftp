//! `thriftp` command-line interface.

mod commands;
mod output;
mod secret_input;
mod store;

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{Parser, Subcommand};

pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static CLI_SECRET: OnceLock<thriftp_core::credentials::Secret> = OnceLock::new();

/// Whether a first interrupt has been observed.
#[must_use]
pub fn interrupted() -> bool {
    INTERRUPTED.load(Ordering::SeqCst)
}

/// Secret supplied through an explicit stdin flag for this invocation.
#[must_use]
pub fn cli_secret() -> Option<&'static thriftp_core::credentials::Secret> {
    CLI_SECRET.get()
}

#[derive(Debug, Parser)]
#[command(
    name = "thriftp",
    version,
    long_version = concat!(
        env!("CARGO_PKG_VERSION"),
        "\nautomation contract: 1.0.0\nMCP protocol versions: 2026-07-28, 2025-11-25"
    ),
    about = "Thriftp command-line file transfer client"
)]
struct Cli {
    /// Emit one JSON result document on stdout.
    #[arg(long, global = true)]
    json: bool,
    /// Suppress human-readable success output; errors still go to stderr.
    #[arg(long, global = true)]
    quiet: bool,
    /// Emit diagnostic details on stderr only.
    #[arg(long, global = true)]
    verbose: bool,
    /// Read a password from one line of stdin for this invocation.
    #[arg(long, global = true)]
    password_stdin: bool,
    /// Read an SSH key passphrase from one line of stdin for this invocation.
    #[arg(long, global = true)]
    key_passphrase_stdin: bool,
    /// Select a saved connection profile.
    #[arg(long, global = true)]
    profile: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Profiles {
        #[command(subcommand)]
        command: ProfileCommand,
    },
    Browse {
        path: String,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        long: bool,
        #[arg(long, default_value_t = 200)]
        limit: usize,
    },
    Stat {
        path: String,
        #[arg(long)]
        local: bool,
    },
    Upload {
        paths: Vec<String>,
        #[arg(long)]
        recursive: bool,
        #[arg(long, default_value = "fail")]
        conflict: String,
        #[arg(long)]
        no_wait: bool,
        #[arg(long)]
        preserve_symlinks: bool,
    },
    Download {
        paths: Vec<String>,
        #[arg(long)]
        recursive: bool,
        #[arg(long, default_value = "fail")]
        conflict: String,
        #[arg(long)]
        no_wait: bool,
        #[arg(long)]
        preserve_symlinks: bool,
    },
    Mkdir {
        path: String,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        parents: bool,
    },
    Rename {
        from: String,
        to: String,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        yes: bool,
    },
    Delete {
        paths: Vec<String>,
        #[arg(long)]
        local: bool,
        #[arg(long)]
        recursive: bool,
        #[arg(long)]
        yes: bool,
    },
    Transfer {
        #[command(subcommand)]
        command: TransferCommand,
    },
    Queue {
        #[command(subcommand)]
        command: QueueCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProfileCommand {
    List,
    Show {
        id: String,
    },
    Create {
        #[arg(long)]
        name: String,
        #[arg(long, default_value = "sftp")]
        protocol: String,
        #[arg(long)]
        host: String,
        #[arg(long)]
        port: u16,
        #[arg(long)]
        username: String,
        #[arg(long, default_value = "password")]
        auth: String,
        #[arg(long)]
        key_file: Option<String>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long, default_value_t = 30)]
        timeout: u32,
    },
    Update {
        id: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        username: Option<String>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        timeout: Option<u32>,
    },
    SetCredential {
        id: String,
    },
    Delete {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Test {
        id: String,
    },
    TrustHostKey {
        id: String,
        #[arg(long)]
        fingerprint: String,
        #[arg(long)]
        yes: bool,
    },
    ForgetHostKey {
        id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
enum TransferCommand {
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    Status {
        id: String,
    },
    Wait {
        id: String,
    },
    Pause {
        id: String,
    },
    Resume {
        id: String,
    },
    Cancel {
        id: String,
        #[arg(long)]
        yes: bool,
    },
    Retry {
        id: String,
    },
    Priority {
        id: String,
        priority: String,
    },
    Reorder {
        ids: Vec<String>,
    },
}

#[derive(Debug, Subcommand)]
enum QueueCommand {
    List,
    Pause,
    Resume,
    CancelAll {
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Subcommand)]
enum ConfigCommand {
    Concurrency { value: usize },
    Bandwidth { value: String },
}

#[derive(Debug, Subcommand)]
enum McpCommand {
    Enable {
        #[arg(long, default_value = "standalone")]
        mode: String,
    },
    Disable,
    Status,
    Clients {
        #[command(subcommand)]
        command: ClientsCommand,
    },
    Tools,
    Serve,
    Confirm {
        fingerprint: String,
        #[arg(long)]
        operation: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long, default_value_t = 120)]
        ttl: i64,
    },
    Audit {
        #[command(subcommand)]
        command: AuditCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ClientsCommand {
    List,
    Pending,
    Approve {
        request_id: String,
        #[arg(long = "dir")]
        dirs: Vec<String>,
        #[arg(long = "local-dir")]
        local_dirs: Vec<String>,
        #[arg(long = "operations", value_delimiter = ',')]
        operations: Vec<String>,
        #[arg(long)]
        expires: Option<String>,
        #[arg(long = "disconnect-policy", default_value = "continue")]
        disconnect_policy: String,
        #[arg(long = "pre-approve")]
        pre_approve: Vec<String>,
        #[arg(long)]
        yes: bool,
    },
    Deny {
        request_id: String,
    },
    Revoke {
        fingerprint: String,
        #[arg(long)]
        yes: bool,
    },
    Disconnect {
        fingerprint: String,
    },
}

#[derive(Debug, Subcommand)]
enum AuditCommand {
    List {
        #[arg(long)]
        since: Option<String>,
        #[arg(long)]
        until: Option<String>,
        #[arg(long)]
        client: Option<String>,
        #[arg(long)]
        result: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: usize,
    },
    Export {
        #[arg(long, default_value = "jsonl")]
        format: String,
        #[arg(long)]
        out: String,
    },
    Verify,
    Retention {
        days: u32,
    },
    Limit {
        bytes: String,
    },
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if std::env::var_os("THRIFTP_PASSWORD").is_some()
        || std::env::var_os("THRIFTP_KEY_PASSPHRASE").is_some()
    {
        eprintln!("secret environment variables are forbidden; use --password-stdin");
        std::process::exit(10);
    }
    thriftp_core::observability::init_tracing();
    if let Err(error) = thriftp_core::automation::maintain_audit() {
        tracing::warn!(%error, "audit maintenance did not complete");
    }
    install_signal_handler();
    let cli = Cli::parse();
    if cli.password_stdin || cli.key_passphrase_stdin {
        match secret_input::read_secret() {
            Ok(secret) => {
                let _ = CLI_SECRET.set(secret);
            }
            Err(error) => {
                eprintln!("error: {error}");
                std::process::exit(error.cli_exit_code());
            }
        }
    }
    let (value, _code) = commands::run(cli.command, cli.profile.as_deref()).await;
    let code = output::emit(&value, cli.json, cli.quiet, cli.verbose);
    std::process::exit(code);
}

fn install_signal_handler() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        if let Ok(mut interrupt) = signal(SignalKind::interrupt()) {
            tokio::spawn(async move {
                interrupt.recv().await;
                INTERRUPTED.store(true, Ordering::SeqCst);
                // A second signal exits immediately without claiming completion.
                interrupt.recv().await;
                std::process::exit(130);
            });
        }
    }
    #[cfg(not(unix))]
    {
        tokio::spawn(async {
            if tokio::signal::ctrl_c().await.is_ok() {
                INTERRUPTED.store(true, Ordering::SeqCst);
            }
        });
    }
}
