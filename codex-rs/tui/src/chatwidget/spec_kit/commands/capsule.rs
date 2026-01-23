//! SPEC-KIT-971: Capsule CLI commands
//! SPEC-KIT-974: Capsule export/import
//!
//! Commands for managing Memvid capsules:
//! - `/speckit.capsule doctor` - Verify capsule health
//! - `/speckit.capsule stats` - Show capsule statistics
//! - `/speckit.capsule checkpoints` - List checkpoints
//! - `/speckit.capsule commit` - Create manual checkpoint
//! - `/speckit.capsule export` - Export capsule to .mv2 file
//!
//! ## Decision IDs
//! - D7: Single-writer capsule model
//! - D18: Stage boundary checkpoints
//! - D8: Optional encryption for exports
//! - D23: Mandatory safe-export mode

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use crate::memvid_adapter::{
    CapsuleConfig, CapsuleHandle, CapsuleStats, DiagnosticResult, ExportOptions, IndexStatus,
};
use ratatui::text::Line;
use std::path::PathBuf;

// =============================================================================
// speckit.capsule
// =============================================================================

/// Command: /speckit.capsule [doctor|stats|checkpoints|commit]
/// Manage Memvid capsule operations.
pub struct CapsuleDoctorCommand;

impl SpecKitCommand for CapsuleDoctorCommand {
    fn name(&self) -> &'static str {
        "speckit.capsule"
    }

    fn aliases(&self) -> &[&'static str] {
        &["capsule.doctor"]
    }

    fn description(&self) -> &'static str {
        "manage capsule (doctor/stats/checkpoints/commit)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let subcommand = args.trim().split_whitespace().next().unwrap_or("doctor");

        match subcommand {
            "doctor" => execute_doctor(widget),
            "stats" => execute_stats(widget),
            "checkpoints" => execute_checkpoints(widget),
            "commit" => execute_commit(widget, &args),
            "export" => execute_export(widget, &args),
            _ => {
                let lines = vec![
                    Line::from("📦 Capsule Commands (SPEC-KIT-971/974/S974-010)"),
                    Line::from(""),
                    Line::from("/speckit.capsule doctor      # Verify capsule health"),
                    Line::from("/speckit.capsule stats       # Show size, frames, dedup ratio"),
                    Line::from("/speckit.capsule checkpoints # List all checkpoints"),
                    Line::from("/speckit.capsule commit --label <LABEL> # Create manual checkpoint"),
                    Line::from(""),
                    Line::from("Export (S974-010):"),
                    Line::from("/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> [options]"),
                    Line::from("  --out <PATH>    Custom output path"),
                    Line::from("  --no-encrypt    Produce .mv2 instead of .mv2e"),
                    Line::from("  --unsafe        Include raw LLM I/O (full_io events)"),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            }
        }
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

// =============================================================================
// Command implementations
// =============================================================================

fn get_capsule_path() -> PathBuf {
    // Use standard capsule path convention
    PathBuf::from(".speckit/memvid/workspace.mv2")
}

fn execute_doctor(widget: &mut ChatWidget) {
    let capsule_path = get_capsule_path();
    let results = CapsuleHandle::doctor(&capsule_path);

    let mut lines = vec![
        Line::from("🩺 Capsule Doctor"),
        Line::from(format!("   Path: {}", capsule_path.display())),
        Line::from(""),
    ];

    let mut has_errors = false;
    let mut has_warnings = false;

    for result in &results {
        match result {
            DiagnosticResult::Ok(msg) => {
                lines.push(Line::from(format!("✓ {}", msg)));
            }
            DiagnosticResult::Warning(msg, remedy) => {
                lines.push(Line::from(format!("⚠ {}", msg)));
                lines.push(Line::from(format!("  → {}", remedy)));
                has_warnings = true;
            }
            DiagnosticResult::Error(msg, remedy) => {
                lines.push(Line::from(format!("✗ {}", msg)));
                lines.push(Line::from(format!("  → {}", remedy)));
                has_errors = true;
            }
        }
    }

    lines.push(Line::from(""));
    if has_errors {
        lines.push(Line::from("Status: ❌ UNHEALTHY - see errors above"));
    } else if has_warnings {
        lines.push(Line::from("Status: ⚠️ WARNING - see warnings above"));
    } else {
        lines.push(Line::from("Status: ✅ HEALTHY"));
    }

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

fn execute_stats(widget: &mut ChatWidget) {
    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Capsule not found. Run `/speckit.capsule doctor` for details.",
            )],
            HistoryCellType::Error,
        ));
        return;
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    match CapsuleHandle::open(config) {
        Ok(handle) => {
            let stats = handle.stats();
            let lines = format_stats(&stats);
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to open capsule: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
}

fn format_stats(stats: &CapsuleStats) -> Vec<Line<'static>> {
    let size_display = if stats.size_bytes < 1024 {
        format!("{} B", stats.size_bytes)
    } else if stats.size_bytes < 1024 * 1024 {
        format!("{:.1} KB", stats.size_bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", stats.size_bytes as f64 / (1024.0 * 1024.0))
    };

    let index_status = match stats.index_status {
        IndexStatus::Healthy => "✅ Healthy",
        IndexStatus::Rebuilding => "🔄 Rebuilding",
        IndexStatus::Missing => "❌ Missing",
    };

    vec![
        Line::from("📊 Capsule Statistics"),
        Line::from(format!("   Path: {}", stats.path.display())),
        Line::from(format!("   Size: {}", size_display)),
        Line::from(format!("   Branch: {}", stats.current_branch.as_str())),
        Line::from(""),
        Line::from(format!("   Checkpoints: {}", stats.checkpoint_count)),
        Line::from(format!("   Events: {}", stats.event_count)),
        Line::from(format!("   URIs indexed: {}", stats.uri_count)),
        Line::from(format!("   Frames: {}", stats.frame_count)),
        Line::from(format!("   Index status: {}", index_status)),
        Line::from(format!("   Dedup ratio: {:.2}x", stats.dedup_ratio)),
    ]
}

fn execute_checkpoints(widget: &mut ChatWidget) {
    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Capsule not found. Run `/speckit.capsule doctor` for details.",
            )],
            HistoryCellType::Error,
        ));
        return;
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    match CapsuleHandle::open(config) {
        Ok(handle) => {
            let checkpoints = handle.list_checkpoints();
            if checkpoints.is_empty() {
                widget.history_push(PlainHistoryCell::new(
                    vec![
                        Line::from("📋 Checkpoints"),
                        Line::from(""),
                        Line::from("   No checkpoints yet. Run a stage to create one."),
                    ],
                    HistoryCellType::Notice,
                ));
                return;
            }

            let mut lines = vec![
                Line::from("📋 Checkpoints"),
                Line::from(""),
                Line::from("   ID                          | Stage | Spec/Run          | Type"),
                Line::from("   ----------------------------|-------|-------------------|------"),
            ];

            for cp in checkpoints.iter().rev().take(20) {
                let stage = cp.stage.as_deref().unwrap_or("-");
                let spec_run = match (&cp.spec_id, &cp.run_id) {
                    (Some(s), Some(r)) => format!("{}/{}", s, r),
                    (Some(s), None) => s.clone(),
                    _ => "-".to_string(),
                };
                let cp_type = if cp.is_manual { "manual" } else { "stage" };

                lines.push(Line::from(format!(
                    "   {:28} | {:5} | {:17} | {}",
                    cp.checkpoint_id.as_str(),
                    stage,
                    spec_run,
                    cp_type
                )));
            }

            if checkpoints.len() > 20 {
                lines.push(Line::from(""));
                lines.push(Line::from(format!(
                    "   Showing 20 of {} checkpoints",
                    checkpoints.len()
                )));
            }

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to open capsule: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
}

fn execute_commit(widget: &mut ChatWidget, args: &str) {
    // Parse --label argument
    let parts: Vec<&str> = args.split_whitespace().collect();
    let label = if let Some(idx) = parts.iter().position(|&p| p == "--label") {
        parts.get(idx + 1).copied()
    } else {
        None
    };

    let label = match label {
        Some(l) => l,
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from("Usage: /speckit.capsule commit --label <LABEL>")],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Capsule not found. Run `/speckit.capsule doctor` for details.",
            )],
            HistoryCellType::Error,
        ));
        return;
    }

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    match CapsuleHandle::open(config) {
        Ok(handle) => match handle.commit_manual(label) {
            Ok(checkpoint_id) => {
                let lines = vec![
                    Line::from("✅ Manual Checkpoint Created"),
                    Line::from(""),
                    Line::from(format!("   Label: {}", label)),
                    Line::from(format!("   Checkpoint ID: {}", checkpoint_id.as_str())),
                    Line::from(""),
                    Line::from("   Use this ID for time-travel/replay workflows."),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            }
            Err(e) => {
                widget.history_push(PlainHistoryCell::new(
                    vec![Line::from(format!("❌ Failed to create checkpoint: {}", e))],
                    HistoryCellType::Error,
                ));
            }
        },
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to open capsule: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
}

/// Execute the export subcommand.
///
/// ## S974-010: Export capsule to .mv2/.mv2e file
///
/// Usage:
/// - `/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID>` - Export with defaults
/// - `/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> --out <PATH>` - Custom output path
/// - `/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> --no-encrypt` - Unencrypted export
/// - `/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> --unsafe` - Include raw LLM I/O
///
/// ## Defaults (locked decisions)
/// - Encryption ON (D8) - produces .mv2e
/// - Safe mode ON (D23) - excludes raw LLM I/O
/// - Output path per D2: ./docs/specs/<SPEC_ID>/runs/<RUN_ID>/capsule.mv2e
fn execute_export(widget: &mut ChatWidget, args: &str) {
    // Parse arguments
    let parts: Vec<&str> = args.split_whitespace().collect();

    // Parse --spec argument (required)
    let spec_id = if let Some(idx) = parts.iter().position(|&p| p == "--spec") {
        parts.get(idx + 1).map(|s| s.to_string())
    } else {
        None
    };

    // Parse --run argument (required)
    let run_id = if let Some(idx) = parts.iter().position(|&p| p == "--run") {
        parts.get(idx + 1).map(|s| s.to_string())
    } else {
        None
    };

    // Validate required arguments
    let (spec_id, run_id) = match (spec_id, run_id) {
        (Some(s), Some(r)) => (s, r),
        _ => {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> [options]"),
                    Line::from(""),
                    Line::from("Required:"),
                    Line::from("  --spec <SPEC_ID>    Spec ID to export"),
                    Line::from("  --run <RUN_ID>      Run ID to export"),
                    Line::from(""),
                    Line::from("Options:"),
                    Line::from("  --out <PATH>        Output path (default: ./docs/specs/<SPEC>/runs/<RUN>/capsule.mv2e)"),
                    Line::from("  --encrypt           Enable encryption (default)"),
                    Line::from("  --no-encrypt        Disable encryption (produces .mv2)"),
                    Line::from("  --safe              Safe mode: exclude raw LLM I/O (default)"),
                    Line::from("  --unsafe            Include raw LLM I/O (full_io events)"),
                    Line::from(""),
                    Line::from("Examples:"),
                    Line::from("  /speckit.capsule export --spec SPEC-KIT-974 --run run-001"),
                    Line::from("  /speckit.capsule export --spec SPEC-KIT-974 --run run-001 --no-encrypt"),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Parse optional --out argument
    let output_path = if let Some(idx) = parts.iter().position(|&p| p == "--out") {
        parts.get(idx + 1).map(|s| PathBuf::from(*s))
    } else {
        None
    };

    // Parse encryption flags (default: encrypt)
    let no_encrypt = parts.iter().any(|&p| p == "--no-encrypt");
    let encrypt = !no_encrypt;

    // Parse safe mode flags (default: safe)
    let unsafe_export = parts.iter().any(|&p| p == "--unsafe");
    let safe_mode = !unsafe_export;

    // Determine output path per D2: ./docs/specs/<SPEC_ID>/runs/<RUN_ID>/capsule.mv2e
    let extension = if encrypt { "mv2e" } else { "mv2" };
    let output_path = output_path.unwrap_or_else(|| {
        PathBuf::from("docs")
            .join("specs")
            .join(&spec_id)
            .join("runs")
            .join(&run_id)
            .join(format!("capsule.{}", extension))
    });

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to create export directory: {}", e))],
                HistoryCellType::Error,
            ));
            return;
        }
    }

    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Capsule not found. Run `/speckit.capsule doctor` for details.",
            )],
            HistoryCellType::Error,
        ));
        return;
    }

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    match CapsuleHandle::open(config) {
        Ok(handle) => {
            use crate::memvid_adapter::BranchId;

            let options = ExportOptions {
                output_path: output_path.clone(),
                spec_id: Some(spec_id.clone()),
                run_id: Some(run_id.clone()),
                branch: Some(BranchId::for_run(&run_id)),
                safe_mode,
                encrypt,
                interactive: true, // TUI is always interactive
            };

            match handle.export(&options) {
                Ok(result) => {
                    let size_display = if result.bytes_written < 1024 {
                        format!("{} B", result.bytes_written)
                    } else if result.bytes_written < 1024 * 1024 {
                        format!("{:.1} KB", result.bytes_written as f64 / 1024.0)
                    } else {
                        format!("{:.2} MB", result.bytes_written as f64 / (1024.0 * 1024.0))
                    };

                    let encrypt_status = if encrypt { "encrypted (.mv2e)" } else { "unencrypted (.mv2)" };
                    let safe_status = if safe_mode { "ON" } else { "OFF (includes raw LLM I/O)" };

                    let lines = vec![
                        Line::from("✅ Capsule Exported (S974-010)"),
                        Line::from(""),
                        Line::from(format!("   Output: {}", result.output_path.display())),
                        Line::from(format!("   Format: {}", encrypt_status)),
                        Line::from(format!("   Safe mode: {}", safe_status)),
                        Line::from(format!("   Size: {}", size_display)),
                        Line::from(format!("   Artifacts: {}", result.artifact_count)),
                        Line::from(format!("   Checkpoints: {}", result.checkpoint_count)),
                        Line::from(format!("   Events: {}", result.event_count)),
                        Line::from(""),
                        Line::from(format!("   Hash: {}", &result.content_hash[..16])),
                        Line::from(""),
                        Line::from("   CapsuleExported event recorded in workspace."),
                    ];
                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    let mut lines = vec![Line::from(format!("❌ Export failed: {}", error_msg))];

                    // Add hint for passphrase errors
                    if error_msg.contains("assphrase") {
                        lines.push(Line::from(""));
                        lines.push(Line::from("Hint: Set SPECKIT_MEMVID_PASSPHRASE env var or use --no-encrypt"));
                    }

                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
                }
            }
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to open capsule: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_stats_displays_correctly() {
        use crate::memvid_adapter::BranchId;

        let stats = CapsuleStats {
            path: PathBuf::from(".speckit/memvid/workspace.mv2"),
            size_bytes: 1024 * 500, // 500 KB
            checkpoint_count: 5,
            event_count: 10,
            uri_count: 25,
            current_branch: BranchId::main(),
            frame_count: 100,
            index_status: IndexStatus::Healthy,
            dedup_ratio: 1.5,
        };

        let output = format_stats(&stats);
        let output_str: String = output.iter().map(|l| format!("{:?}", l)).collect();
        assert!(output_str.contains("500"));
        assert!(output_str.contains("Checkpoints: 5"));
        assert!(output_str.contains("1.50x"));
    }

    #[test]
    fn test_capsule_command_name() {
        let cmd = CapsuleDoctorCommand;
        assert_eq!(cmd.name(), "speckit.capsule");
        assert!(!cmd.requires_args());
        assert!(!cmd.is_prompt_expanding());
    }

    // =========================================================================
    // S974-010: Export argument parsing tests
    // =========================================================================

    /// Helper to parse export args from a string
    fn parse_export_args(args: &str) -> (Option<String>, Option<String>, Option<PathBuf>, bool, bool) {
        let parts: Vec<&str> = args.split_whitespace().collect();

        let spec_id = if let Some(idx) = parts.iter().position(|&p| p == "--spec") {
            parts.get(idx + 1).map(|s| s.to_string())
        } else {
            None
        };

        let run_id = if let Some(idx) = parts.iter().position(|&p| p == "--run") {
            parts.get(idx + 1).map(|s| s.to_string())
        } else {
            None
        };

        let output_path = if let Some(idx) = parts.iter().position(|&p| p == "--out") {
            parts.get(idx + 1).map(|s| PathBuf::from(*s))
        } else {
            None
        };

        let no_encrypt = parts.iter().any(|&p| p == "--no-encrypt");
        let encrypt = !no_encrypt;

        let unsafe_export = parts.iter().any(|&p| p == "--unsafe");
        let safe_mode = !unsafe_export;

        (spec_id, run_id, output_path, encrypt, safe_mode)
    }

    #[test]
    fn test_export_args_basic() {
        let (spec_id, run_id, _, encrypt, safe_mode) =
            parse_export_args("export --spec SPEC-974 --run run-001");
        assert_eq!(spec_id, Some("SPEC-974".to_string()));
        assert_eq!(run_id, Some("run-001".to_string()));
        assert!(encrypt, "default should be encrypted");
        assert!(safe_mode, "default should be safe mode");
    }

    #[test]
    fn test_export_args_with_out_path() {
        let (_, _, output_path, _, _) =
            parse_export_args("export --spec SPEC-974 --run run-001 --out /tmp/export.mv2e");
        assert_eq!(output_path, Some(PathBuf::from("/tmp/export.mv2e")));
    }

    #[test]
    fn test_export_args_no_encrypt() {
        let (_, _, _, encrypt, _) =
            parse_export_args("export --spec SPEC-974 --run run-001 --no-encrypt");
        assert!(!encrypt, "--no-encrypt should disable encryption");
    }

    #[test]
    fn test_export_args_unsafe() {
        let (_, _, _, _, safe_mode) =
            parse_export_args("export --spec SPEC-974 --run run-001 --unsafe");
        assert!(!safe_mode, "--unsafe should disable safe mode");
    }

    #[test]
    fn test_export_args_all_flags() {
        let (spec_id, run_id, output_path, encrypt, safe_mode) = parse_export_args(
            "export --spec SPEC-974 --run run-001 --out /tmp/test.mv2 --no-encrypt --unsafe",
        );
        assert_eq!(spec_id, Some("SPEC-974".to_string()));
        assert_eq!(run_id, Some("run-001".to_string()));
        assert_eq!(output_path, Some(PathBuf::from("/tmp/test.mv2")));
        assert!(!encrypt);
        assert!(!safe_mode);
    }

    #[test]
    fn test_export_default_output_path_encrypted() {
        // Test D2 default path: ./docs/specs/<SPEC_ID>/runs/<RUN_ID>/capsule.mv2e
        let spec_id = "SPEC-974";
        let run_id = "run-001";
        let encrypt = true;
        let extension = if encrypt { "mv2e" } else { "mv2" };
        let expected_path = PathBuf::from("docs")
            .join("specs")
            .join(spec_id)
            .join("runs")
            .join(run_id)
            .join(format!("capsule.{}", extension));

        assert_eq!(
            expected_path,
            PathBuf::from("docs/specs/SPEC-974/runs/run-001/capsule.mv2e")
        );
    }

    #[test]
    fn test_export_default_output_path_unencrypted() {
        // Test D2 default path with --no-encrypt
        let spec_id = "SPEC-974";
        let run_id = "run-001";
        let encrypt = false;
        let extension = if encrypt { "mv2e" } else { "mv2" };
        let expected_path = PathBuf::from("docs")
            .join("specs")
            .join(spec_id)
            .join("runs")
            .join(run_id)
            .join(format!("capsule.{}", extension));

        assert_eq!(
            expected_path,
            PathBuf::from("docs/specs/SPEC-974/runs/run-001/capsule.mv2")
        );
    }
}
