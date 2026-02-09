//! PM Bot CLI subcommands (SPEC-PM-002 / SPEC-PM-003).
//!
//! Thin IPC client that connects to `codex-pm-service` via Unix socket
//! and dispatches JSON-RPC-lite requests.
//!
//! ## Commands
//!
//! - `code speckit pm bot run --id <ID> --kind <KIND>`
//! - `code speckit pm bot status --id <ID>`
//! - `code speckit pm bot runs --id <ID>`
//! - `code speckit pm bot cancel --id <ID> --run <RUN_ID>`
//! - `code speckit pm service status`
//! - `code speckit pm service doctor`

#![allow(clippy::uninlined_format_args)]

use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

/// PM subcommand group.
#[derive(Debug, Parser)]
pub struct PmCli {
    #[command(subcommand)]
    pub command: PmSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PmSubcommand {
    /// Bot run operations (submit, query, cancel).
    Bot(BotCli),
    /// Service management (status, doctor).
    Service(ServiceCli),
}

// ─────────────────────────────────────────────────────────────────────────────
// Bot subcommands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct BotCli {
    #[command(subcommand)]
    pub command: BotSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum BotSubcommand {
    /// Submit a new bot run.
    Run(BotRunArgs),
    /// Query status of runs for a work item.
    Status(BotStatusArgs),
    /// List all runs for a work item.
    Runs(BotRunsArgs),
    /// Show details of a specific run.
    Show(BotShowArgs),
    /// Cancel an active run.
    Cancel(BotCancelArgs),
}

#[derive(Debug, Parser)]
pub struct BotRunArgs {
    /// Work item ID (e.g., SPEC-PM-001).
    #[arg(long = "id", short = 'i')]
    pub work_item_id: String,

    /// Bot kind: research or review.
    #[arg(long = "kind", short = 'k')]
    pub kind: String,

    /// Capture mode (prompts_only, full_io).
    #[arg(long = "capture", default_value = "prompts_only")]
    pub capture_mode: String,

    /// Write mode (none, worktree). Only valid for review.
    #[arg(long = "write-mode", default_value = "none")]
    pub write_mode: String,

    /// Working directory (defaults to current directory).
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Wait for run to complete (receive push notifications).
    #[arg(long = "wait")]
    pub wait: bool,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct BotStatusArgs {
    /// Work item ID.
    #[arg(long = "id", short = 'i')]
    pub work_item_id: String,

    /// Filter by kind.
    #[arg(long = "kind", short = 'k')]
    pub kind: Option<String>,

    /// Working directory.
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct BotRunsArgs {
    /// Work item ID.
    #[arg(long = "id", short = 'i')]
    pub work_item_id: String,

    /// Max results.
    #[arg(long = "limit", default_value = "10")]
    pub limit: u32,

    /// Working directory.
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct BotShowArgs {
    /// Work item ID.
    #[arg(long = "id", short = 'i')]
    pub work_item_id: String,

    /// Run ID.
    #[arg(long = "run", short = 'r')]
    pub run_id: String,

    /// Working directory.
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct BotCancelArgs {
    /// Work item ID.
    #[arg(long = "id", short = 'i')]
    pub work_item_id: String,

    /// Run ID to cancel.
    #[arg(long = "run", short = 'r')]
    pub run_id: String,

    /// Working directory.
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Service subcommands
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Parser)]
pub struct ServiceCli {
    #[command(subcommand)]
    pub command: ServiceSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ServiceSubcommand {
    /// Show service status (uptime, active runs).
    Status(ServiceStatusArgs),
    /// Run health checks.
    Doctor(ServiceDoctorArgs),
}

#[derive(Debug, Parser)]
pub struct ServiceStatusArgs {
    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

#[derive(Debug, Parser)]
pub struct ServiceDoctorArgs {
    /// Working directory for workspace-specific checks.
    #[arg(long = "cwd", short = 'C')]
    pub cwd: Option<PathBuf>,

    /// Output as JSON.
    #[arg(long = "json", short = 'j')]
    pub json: bool,

    /// Override socket path.
    #[arg(long = "socket")]
    pub socket: Option<PathBuf>,
}

// ─────────────────────────────────────────────────────────────────────────────
// IPC Client
// ─────────────────────────────────────────────────────────────────────────────

/// Connect to the PM service socket and send a JSON-RPC request.
fn send_rpc(
    socket_path: Option<&PathBuf>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::net::UnixStream;

    let path = socket_path
        .cloned()
        .unwrap_or_else(codex_pm_service::default_socket_path);

    let stream = UnixStream::connect(&path).map_err(|e| {
        format!(
            "Cannot connect to PM service at {}: {}\nHint: Start the service with: systemctl --user start codex-pm-service",
            path.display(),
            e
        )
    })?;

    // Send hello handshake
    let hello = serde_json::json!({
        "id": 0,
        "method": "hello",
        "params": {
            "protocol_version": codex_pm_service::PROTOCOL_VERSION,
            "client_version": env!("CARGO_PKG_VERSION"),
        }
    });

    let mut writer = stream
        .try_clone()
        .map_err(|e| format!("Clone stream: {e}"))?;
    let mut hello_bytes = serde_json::to_vec(&hello).map_err(|e| format!("Serialize: {e}"))?;
    hello_bytes.push(b'\n');
    writer
        .write_all(&hello_bytes)
        .map_err(|e| format!("Write hello: {e}"))?;

    // Read hello response
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Read hello response: {e}"))?;

    let hello_resp: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| format!("Parse hello response: {e}"))?;

    if hello_resp.get("error").is_some() {
        return Err(format!("Handshake failed: {}", hello_resp));
    }

    // Send the actual request
    let request = serde_json::json!({
        "id": 1,
        "method": method,
        "params": params,
    });

    let mut req_bytes = serde_json::to_vec(&request).map_err(|e| format!("Serialize: {e}"))?;
    req_bytes.push(b'\n');
    writer
        .write_all(&req_bytes)
        .map_err(|e| format!("Write request: {e}"))?;

    // Read response
    line.clear();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Read response: {e}"))?;

    let response: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| format!("Parse response: {e}"))?;

    if let Some(error) = response.get("error") {
        return Err(format!(
            "RPC error: {}",
            error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("unknown error")
        ));
    }

    Ok(response
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

fn resolve_cwd(cwd: &Option<PathBuf>) -> String {
    cwd.clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .to_string_lossy()
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// Dispatch
// ─────────────────────────────────────────────────────────────────────────────

impl PmCli {
    pub fn run(&self) -> Result<(), String> {
        match &self.command {
            PmSubcommand::Bot(bot) => bot.run(),
            PmSubcommand::Service(svc) => svc.run(),
        }
    }
}

impl BotCli {
    fn run(&self) -> Result<(), String> {
        match &self.command {
            BotSubcommand::Run(args) => cmd_bot_run(args),
            BotSubcommand::Status(args) => cmd_bot_status(args),
            BotSubcommand::Runs(args) => cmd_bot_runs(args),
            BotSubcommand::Show(args) => cmd_bot_show(args),
            BotSubcommand::Cancel(args) => cmd_bot_cancel(args),
        }
    }
}

impl ServiceCli {
    fn run(&self) -> Result<(), String> {
        match &self.command {
            ServiceSubcommand::Status(args) => cmd_service_status(args),
            ServiceSubcommand::Doctor(args) => cmd_service_doctor(args),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Command implementations
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_bot_run(args: &BotRunArgs) -> Result<(), String> {
    let params = serde_json::json!({
        "workspace_path": resolve_cwd(&args.cwd),
        "work_item_id": args.work_item_id,
        "kind": args.kind,
        "capture_mode": args.capture_mode,
        "write_mode": args.write_mode,
        "subscribe": args.wait,
    });

    let result = send_rpc(args.socket.as_ref(), "bot.run", params)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "Run submitted: {} (status: {}, kind: {})",
            result.get("run_id").and_then(|v| v.as_str()).unwrap_or("?"),
            result.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
            result.get("kind").and_then(|v| v.as_str()).unwrap_or("?"),
        );
    }

    Ok(())
}

fn cmd_bot_status(args: &BotStatusArgs) -> Result<(), String> {
    let mut params = serde_json::json!({
        "workspace_path": resolve_cwd(&args.cwd),
        "work_item_id": args.work_item_id,
    });

    if let Some(kind) = &args.kind {
        params["kind"] = serde_json::Value::String(kind.clone());
    }

    let result = send_rpc(args.socket.as_ref(), "bot.status", params)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        let runs = result
            .get("runs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if runs.is_empty() {
            println!("No runs found for {}", args.work_item_id);
        } else {
            for run in &runs {
                println!(
                    "  {} ({}) — {}",
                    run.get("run_id").and_then(|v| v.as_str()).unwrap_or("?"),
                    run.get("kind").and_then(|v| v.as_str()).unwrap_or("?"),
                    run.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
                );
            }
        }
    }

    Ok(())
}

fn cmd_bot_runs(args: &BotRunsArgs) -> Result<(), String> {
    let params = serde_json::json!({
        "workspace_path": resolve_cwd(&args.cwd),
        "work_item_id": args.work_item_id,
        "limit": args.limit,
    });

    let result = send_rpc(args.socket.as_ref(), "bot.runs", params)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Runs for {}:", args.work_item_id);
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    }

    Ok(())
}

fn cmd_bot_show(args: &BotShowArgs) -> Result<(), String> {
    let params = serde_json::json!({
        "workspace_path": resolve_cwd(&args.cwd),
        "work_item_id": args.work_item_id,
        "run_id": args.run_id,
    });

    let result = send_rpc(args.socket.as_ref(), "bot.show", params)?;

    println!(
        "{}",
        serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
    );

    Ok(())
}

fn cmd_bot_cancel(args: &BotCancelArgs) -> Result<(), String> {
    let params = serde_json::json!({
        "workspace_path": resolve_cwd(&args.cwd),
        "work_item_id": args.work_item_id,
        "run_id": args.run_id,
    });

    let result = send_rpc(args.socket.as_ref(), "bot.cancel", params)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "Run {} cancelled (status: {})",
            result.get("run_id").and_then(|v| v.as_str()).unwrap_or("?"),
            result.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
        );
    }

    Ok(())
}

fn cmd_service_status(args: &ServiceStatusArgs) -> Result<(), String> {
    let result = send_rpc(
        args.socket.as_ref(),
        "service.status",
        serde_json::json!({}),
    )?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!(
            "PM Service: uptime {}s, {} active runs",
            result
                .get("uptime_s")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
            result
                .get("active_runs")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(0),
        );
    }

    Ok(())
}

fn cmd_service_doctor(args: &ServiceDoctorArgs) -> Result<(), String> {
    let mut params = serde_json::json!({});
    if let Some(cwd) = &args.cwd {
        params["workspace_path"] = serde_json::Value::String(cwd.to_string_lossy().to_string());
    }

    let result = send_rpc(args.socket.as_ref(), "service.doctor", params)?;

    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        let checks = result
            .get("checks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        for check in &checks {
            println!(
                "  [{}] {} — {}",
                check.get("status").and_then(|v| v.as_str()).unwrap_or("?"),
                check.get("name").and_then(|v| v.as_str()).unwrap_or("?"),
                check.get("detail").and_then(|v| v.as_str()).unwrap_or(""),
            );
        }
    }

    Ok(())
}
