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
                    Line::from("📦 Capsule Commands (SPEC-KIT-971/974)"),
                    Line::from(""),
                    Line::from("/speckit.capsule doctor      # Verify capsule health"),
                    Line::from("/speckit.capsule stats       # Show size, frames, dedup ratio"),
                    Line::from("/speckit.capsule checkpoints # List all checkpoints"),
                    Line::from(
                        "/speckit.capsule commit --label <LABEL> # Create manual checkpoint",
                    ),
                    Line::from(
                        "/speckit.capsule export --out <PATH> [--run <RUN_ID>] # Export to .mv2",
                    ),
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
/// ## SPEC-KIT-974: Single-file .mv2 export MVP
///
/// Usage:
/// - `/speckit.capsule export --out <PATH>` - Export entire capsule
/// - `/speckit.capsule export --out <PATH> --run <RUN_ID>` - Export specific run
/// - `/speckit.capsule export --out <PATH> --spec <SPEC_ID>` - Export specific spec
fn execute_export(widget: &mut ChatWidget, args: &str) {
    // Parse arguments
    let parts: Vec<&str> = args.split_whitespace().collect();

    // Parse --out argument (required)
    let output_path = if let Some(idx) = parts.iter().position(|&p| p == "--out") {
        parts.get(idx + 1).map(|s| PathBuf::from(*s))
    } else {
        None
    };

    let output_path = match output_path {
        Some(p) => p,
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /speckit.capsule export --out <PATH> [--run <RUN_ID>] [--spec <SPEC_ID>]"),
                    Line::from(""),
                    Line::from("Examples:"),
                    Line::from("  /speckit.capsule export --out ./export.mv2"),
                    Line::from("  /speckit.capsule export --out ./run-abc.mv2 --run abc123"),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Parse optional --run argument
    let run_id = if let Some(idx) = parts.iter().position(|&p| p == "--run") {
        parts.get(idx + 1).map(|s| s.to_string())
    } else {
        None
    };

    // Parse optional --spec argument
    let spec_id = if let Some(idx) = parts.iter().position(|&p| p == "--spec") {
        parts.get(idx + 1).map(|s| s.to_string())
    } else {
        None
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
        Ok(handle) => {
            let options = ExportOptions {
                output_path: output_path.clone(),
                spec_id,
                run_id,
                branch: None,    // Could parse --branch if needed
                safe_mode: true, // Default to safe mode per D23
                ..Default::default()
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

                    let lines = vec![
                        Line::from("✅ Capsule Exported (SPEC-KIT-974)"),
                        Line::from(""),
                        Line::from(format!("   Output: {}", result.output_path.display())),
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
                    widget.history_push(PlainHistoryCell::new(
                        vec![Line::from(format!("❌ Export failed: {}", e))],
                        HistoryCellType::Error,
                    ));
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
}
