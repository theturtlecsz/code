//! SPEC-PM-004: PM service TUI commands
//!
//! Provides `/pm service status`, `/pm service doctor`, and all `/pm bot ...`
//! commands that talk directly to the PM service over its Unix domain socket
//! (JSON-RPC-lite).
//!
//! The socket path and protocol version are inlined here to avoid a circular
//! dependency (`codex-pm-service` already depends on `codex-tui`).

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// Inlined from codex-pm-service (avoids circular dep)
// ─────────────────────────────────────────────────────────────────────────────

const SOCKET_FILENAME: &str = "codex-pm.sock";
const PROTOCOL_VERSION: &str = "1.0";

fn default_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        PathBuf::from(runtime_dir).join(SOCKET_FILENAME)
    } else {
        let user = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
        PathBuf::from(format!("/tmp/codex-pm-{user}.sock"))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IPC client (adapted from cli/src/pm_cmd.rs:283-412)
// ─────────────────────────────────────────────────────────────────────────────

/// Connect to the PM service socket and perform the protocol handshake.
fn connect_and_handshake() -> Result<
    (
        std::io::BufReader<std::os::unix::net::UnixStream>,
        std::os::unix::net::UnixStream,
    ),
    String,
> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;

    let path = default_socket_path();

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
            "protocol_version": PROTOCOL_VERSION,
            "client_version": env!("CARGO_PKG_VERSION"),
        }
    });

    let mut writer = stream
        .try_clone()
        .map_err(|e| format!("Clone stream: {e}"))?;
    let mut hello_bytes =
        serde_json::to_vec(&hello).map_err(|e| format!("Serialize hello: {e}"))?;
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
        return Err(format!("Handshake failed: {hello_resp}"));
    }

    Ok((reader, writer))
}

/// Send a JSON-RPC request and return the result (or a user-visible error).
pub(crate) fn send_rpc(
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::io::{BufRead, Write};

    let (mut reader, mut writer) = connect_and_handshake()?;

    let request = serde_json::json!({
        "id": 1,
        "method": method,
        "params": params,
    });

    let mut req_bytes =
        serde_json::to_vec(&request).map_err(|e| format!("Serialize request: {e}"))?;
    req_bytes.push(b'\n');
    writer
        .write_all(&req_bytes)
        .map_err(|e| format!("Write request: {e}"))?;

    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Read response: {e}"))?;

    let response: serde_json::Value =
        serde_json::from_str(&line).map_err(|e| format!("Parse response: {e}"))?;

    if let Some(error) = response.get("error") {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown error");
        return Err(msg.to_string());
    }

    Ok(response
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

// ─────────────────────────────────────────────────────────────────────────────
// Flag parser for slash-command arguments
// ─────────────────────────────────────────────────────────────────────────────

/// Extract the value following `--flag` in a word slice. Returns `None` if
/// the flag is absent or has no subsequent value.
fn parse_flag<'a>(words: &[&'a str], flag: &str) -> Option<&'a str> {
    let target = format!("--{flag}");
    words.windows(2).find(|w| w[0] == target).map(|w| w[1])
}

// ─────────────────────────────────────────────────────────────────────────────
// Command registration
// ─────────────────────────────────────────────────────────────────────────────

/// Command: /pm open | /pm service ... | /pm bot ...
pub struct PmCommand;

impl SpecKitCommand for PmCommand {
    fn name(&self) -> &'static str {
        "speckit.pm"
    }

    fn aliases(&self) -> &[&'static str] {
        &["pm"]
    }

    fn description(&self) -> &'static str {
        "PM commands (open, service status/doctor, bot run/status/runs/show/cancel/resume)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let words: Vec<&str> = args.split_whitespace().collect();

        match words.as_slice() {
            ["open"] => widget.open_pm_overlay(),
            ["service", "status"] => execute_service_rpc(widget, "service.status"),
            ["service", "doctor"] => execute_service_rpc(widget, "service.doctor"),
            ["bot", "run", ..] => execute_bot_run(widget, &words[2..]),
            ["bot", "status", ..] => execute_bot_status(widget, &words[2..]),
            ["bot", "runs", ..] => execute_bot_runs(widget, &words[2..]),
            ["bot", "show", ..] => execute_bot_show(widget, &words[2..]),
            ["bot", "cancel", ..] => execute_bot_cancel(widget, &words[2..]),
            ["bot", "resume", ..] => execute_bot_resume(widget, &words[2..]),
            _ => {
                let lines = vec![
                    Line::from("PM Commands (SPEC-PM-004)"),
                    Line::from(""),
                    Line::from("  /pm open                                    # Open PM overlay"),
                    Line::from(""),
                    Line::from("  /pm service status                          # Service health"),
                    Line::from("  /pm service doctor                          # Deep diagnostics"),
                    Line::from(""),
                    Line::from("  /pm bot run --id <ID> --kind <research|review>"),
                    Line::from("              [--capture <prompts_only|full_io>]"),
                    Line::from("              [--write-mode <none|worktree>]"),
                    Line::from("  /pm bot status --id <ID> [--kind <KIND>]"),
                    Line::from("  /pm bot runs --id <ID> [--limit <N>] [--offset <N>]"),
                    Line::from("  /pm bot show --id <ID> --run <RUN_ID> [--format json]"),
                    Line::from("  /pm bot cancel --id <ID> --run <RUN_ID>"),
                    Line::from("  /pm bot resume --id <ID> --run <RUN_ID>"),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Subcommand dispatch — service
// ─────────────────────────────────────────────────────────────────────────────

fn execute_service_rpc(widget: &mut ChatWidget, method: &str) {
    match send_rpc(method, serde_json::json!({})) {
        Ok(result) => {
            let pretty =
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{result}"));
            let mut lines: Vec<Line<'static>> =
                vec![Line::from(format!("[{method}]")), Line::from("")];
            for l in pretty.lines() {
                lines.push(Line::from(l.to_string()));
            }
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("PM service error: {e}"))],
                HistoryCellType::Error,
            ));
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Subcommand dispatch — bot
// ─────────────────────────────────────────────────────────────────────────────

/// Send a bot RPC and pretty-print the result to chat history.
fn execute_bot_rpc(widget: &mut ChatWidget, method: &str, params: serde_json::Value) {
    match send_rpc(method, params) {
        Ok(result) => {
            let pretty =
                serde_json::to_string_pretty(&result).unwrap_or_else(|_| format!("{result}"));
            let mut lines: Vec<Line<'static>> =
                vec![Line::from(format!("[{method}]")), Line::from("")];
            for l in pretty.lines() {
                lines.push(Line::from(l.to_string()));
            }
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("PM service error: {e}"))],
                HistoryCellType::Error,
            ));
        }
    }
}

/// Push an error message for a missing required flag.
fn push_missing_flag(widget: &mut ChatWidget, flag: &str, usage: &str) {
    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from(format!("Missing required flag: --{flag}")),
            Line::from(format!("Usage: {usage}")),
        ],
        HistoryCellType::Error,
    ));
}

/// `/pm bot run --id <ID> --kind <KIND> [--capture ...] [--write-mode ...]`
fn execute_bot_run(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot run --id <ID> --kind <research|review> [--capture <prompts_only|full_io>] [--write-mode <none|worktree>]";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };
    let Some(kind) = parse_flag(flags, "kind") else {
        push_missing_flag(widget, "kind", usage);
        return;
    };

    let capture = parse_flag(flags, "capture").unwrap_or("prompts_only");
    let write_mode = parse_flag(flags, "write-mode").unwrap_or("none");
    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
        "kind": kind,
        "capture_mode": capture,
        "write_mode": write_mode,
        "subscribe": false,
    });

    execute_bot_rpc(widget, "bot.run", params);
}

/// `/pm bot status --id <ID> [--kind <KIND>]`
fn execute_bot_status(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot status --id <ID> [--kind <research|review>]";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };

    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let mut params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
    });

    if let Some(kind) = parse_flag(flags, "kind") {
        params["kind"] = serde_json::Value::String(kind.to_string());
    }

    execute_bot_rpc(widget, "bot.status", params);
}

/// `/pm bot runs --id <ID> [--limit <N>] [--offset <N>]`
fn execute_bot_runs(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot runs --id <ID> [--limit <N>] [--offset <N>]";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };

    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let mut params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
    });

    if let Some(limit) = parse_flag(flags, "limit") {
        if let Ok(n) = limit.parse::<u32>() {
            params["limit"] = serde_json::json!(n);
        }
    }

    if let Some(offset) = parse_flag(flags, "offset") {
        if let Ok(n) = offset.parse::<u32>() {
            params["offset"] = serde_json::json!(n);
        }
    }

    execute_bot_rpc(widget, "bot.runs", params);
}

/// `/pm bot show --id <ID> --run <RUN_ID> [--format json]`
fn execute_bot_show(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot show --id <ID> --run <RUN_ID> [--format json]";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };
    let Some(run_id) = parse_flag(flags, "run") else {
        push_missing_flag(widget, "run", usage);
        return;
    };

    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let mut params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
        "run_id": run_id,
    });

    if let Some(fmt) = parse_flag(flags, "format") {
        params["format"] = serde_json::Value::String(fmt.to_string());
    }

    execute_bot_rpc(widget, "bot.show", params);
}

/// `/pm bot cancel --id <ID> --run <RUN_ID>`
fn execute_bot_cancel(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot cancel --id <ID> --run <RUN_ID>";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };
    let Some(run_id) = parse_flag(flags, "run") else {
        push_missing_flag(widget, "run", usage);
        return;
    };

    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
        "run_id": run_id,
    });

    execute_bot_rpc(widget, "bot.cancel", params);
}

/// `/pm bot resume --id <ID> --run <RUN_ID>`
fn execute_bot_resume(widget: &mut ChatWidget, flags: &[&str]) {
    let usage = "/pm bot resume --id <ID> --run <RUN_ID>";

    let Some(id) = parse_flag(flags, "id") else {
        push_missing_flag(widget, "id", usage);
        return;
    };
    let Some(run_id) = parse_flag(flags, "run") else {
        push_missing_flag(widget, "run", usage);
        return;
    };

    let workspace = widget.config.cwd.to_string_lossy().to_string();

    let params = serde_json::json!({
        "workspace_path": workspace,
        "work_item_id": id,
        "run_id": run_id,
    });

    execute_bot_rpc(widget, "bot.resume", params);
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pm_command_metadata() {
        let cmd = PmCommand;
        assert_eq!(cmd.name(), "speckit.pm");
        assert_eq!(cmd.aliases(), &["pm"]);
        assert!(!cmd.requires_args());
        assert!(!cmd.is_prompt_expanding());
    }

    #[test]
    fn test_default_socket_path_format() {
        let path = default_socket_path();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("codex-pm"),
            "Socket path should contain 'codex-pm': {path_str}"
        );
        assert!(
            path_str.ends_with(".sock"),
            "Socket path should end with .sock: {path_str}"
        );
    }

    #[test]
    fn test_parse_flag_found() {
        let words = vec!["--id", "SPEC-001", "--kind", "research"];
        assert_eq!(parse_flag(&words, "id"), Some("SPEC-001"));
        assert_eq!(parse_flag(&words, "kind"), Some("research"));
    }

    #[test]
    fn test_parse_flag_missing() {
        let words = vec!["--id", "SPEC-001"];
        assert_eq!(parse_flag(&words, "kind"), None);
    }

    #[test]
    fn test_parse_flag_at_end() {
        // Flag at the last position with no value
        let words = vec!["--id"];
        assert_eq!(parse_flag(&words, "id"), None);
    }

    #[test]
    fn test_parse_flag_hyphenated() {
        let words = vec!["--write-mode", "worktree", "--capture", "full_io"];
        assert_eq!(parse_flag(&words, "write-mode"), Some("worktree"));
        assert_eq!(parse_flag(&words, "capture"), Some("full_io"));
    }
}
