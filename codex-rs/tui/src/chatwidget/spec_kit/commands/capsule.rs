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
    CapsuleConfig, CapsuleHandle, CapsuleStats, DiagnosticResult, ExportOptions, GcConfig,
    ImportOptions, IndexStatus,
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
        let subcommand = args.split_whitespace().next().unwrap_or("doctor");

        match subcommand {
            "doctor" => execute_doctor(widget),
            "stats" => execute_stats(widget),
            "checkpoints" => execute_checkpoints(widget),
            "commit" => execute_commit(widget, &args),
            "export" => execute_export(widget, &args),
            "import" => execute_import(widget, &args),
            "gc" => execute_gc(widget, &args),
            _ => {
                let lines = vec![
                    Line::from("📦 Capsule Commands (SPEC-KIT-971/974/SK974-1/SK974-3)"),
                    Line::from(""),
                    Line::from("/speckit.capsule doctor      # Verify capsule health"),
                    Line::from("/speckit.capsule stats       # Show size, frames, dedup ratio"),
                    Line::from("/speckit.capsule checkpoints # List all checkpoints"),
                    Line::from(
                        "/speckit.capsule commit --label <LABEL> # Create manual checkpoint",
                    ),
                    Line::from(""),
                    Line::from("Export (S974-010):"),
                    Line::from("/speckit.capsule export --spec <SPEC_ID> --run <RUN_ID> [options]"),
                    Line::from("  --out <PATH>    Custom output path"),
                    Line::from("  --no-encrypt    Produce .mv2 instead of .mv2e"),
                    Line::from("  --unsafe        Include raw LLM I/O (full_io events)"),
                    Line::from(""),
                    Line::from("Import (SK974-1, D103/D104):"),
                    Line::from("/speckit.capsule import <PATH> [options]"),
                    Line::from("  --mount-as <NAME>     Mount name (default: filename)"),
                    Line::from("  --require-verified    Fail on unverified capsules"),
                    Line::from(""),
                    Line::from("Garbage Collection (SK974-3, D20/D116):"),
                    Line::from("/speckit.capsule gc [options]"),
                    Line::from("  --retention-days <N>  Days to retain exports (default: 30)"),
                    Line::from("  --dry-run             Preview what would be deleted"),
                    Line::from("  --no-keep-pinned      Also delete pinned exports"),
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
    let no_encrypt = parts.contains(&"--no-encrypt");
    let encrypt = !no_encrypt;

    // Parse safe mode flags (default: safe)
    let unsafe_export = parts.contains(&"--unsafe");
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
                vec![Line::from(format!(
                    "❌ Failed to create export directory: {}",
                    e
                ))],
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

                    let encrypt_status = if encrypt {
                        "encrypted (.mv2e)"
                    } else {
                        "unencrypted (.mv2)"
                    };
                    let safe_status = if safe_mode {
                        "ON"
                    } else {
                        "OFF (includes raw LLM I/O)"
                    };

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
                        lines.push(Line::from(
                            "Hint: Set SPECKIT_MEMVID_PASSPHRASE env var or use --no-encrypt",
                        ));
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

/// Execute the import subcommand.
///
/// ## SK974-1: Import capsule with read-only mount + doctor verification
///
/// Usage:
/// - `/speckit.capsule import <PATH>` - Import with defaults
/// - `/speckit.capsule import <PATH> --mount-as <NAME>` - Custom mount name
/// - `/speckit.capsule import <PATH> --require-verified` - Fail on unverified
///
/// ## Decisions honored
/// - D103: Imported capsules read-only
/// - D104: Auto-register mounted capsules (CapsuleImported event)
fn execute_import(widget: &mut ChatWidget, args: &str) {
    // Parse arguments - first positional arg is the path
    let parts: Vec<&str> = args.split_whitespace().collect();

    // Find the path (first arg after "import")
    let source_path = parts.get(1).map(|s| PathBuf::from(*s));

    let source_path = match source_path {
        Some(p) => p,
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Usage: /speckit.capsule import <PATH> [options]"),
                    Line::from(""),
                    Line::from("Arguments:"),
                    Line::from("  <PATH>              Path to capsule file (.mv2 or .mv2e)"),
                    Line::from(""),
                    Line::from("Options:"),
                    Line::from(
                        "  --mount-as <NAME>   Mount name (default: filename without extension)",
                    ),
                    Line::from("  --require-verified  Fail if capsule doesn't pass verification"),
                    Line::from(""),
                    Line::from("Examples:"),
                    Line::from("  /speckit.capsule import ./exports/audit-2026-01.mv2"),
                    Line::from("  /speckit.capsule import ./customer.mv2e --mount-as customer-jan"),
                    Line::from(""),
                    Line::from("Decision IDs: D103 (read-only), D104 (auto-register)"),
                ],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Parse --mount-as argument
    let mount_as = if let Some(idx) = parts.iter().position(|&p| p == "--mount-as") {
        parts.get(idx + 1).map(|s| s.to_string())
    } else {
        None
    };

    // Parse --require-verified flag
    let require_verified = parts.contains(&"--require-verified");

    // Check source exists
    if !source_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(format!(
                "❌ Source capsule not found: {}",
                source_path.display()
            ))],
            HistoryCellType::Error,
        ));
        return;
    }

    // Open workspace capsule to record import event
    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Workspace capsule not found. Run `/speckit.capsule doctor` for details.",
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
            let options = ImportOptions {
                source_path: source_path.clone(),
                mount_as,
                interactive: true, // TUI is always interactive
                require_verified,
            };

            match handle.import(&options) {
                Ok(result) => {
                    let verification_status = if result.verification_passed {
                        "✅ PASSED"
                    } else {
                        "⚠️ WARNINGS"
                    };

                    let is_encrypted = source_path
                        .extension()
                        .map(|e| e == "mv2e")
                        .unwrap_or(false);
                    let format_str = if is_encrypted {
                        "encrypted (.mv2e)"
                    } else {
                        "unencrypted (.mv2)"
                    };

                    let mut lines = vec![
                        Line::from("✅ Capsule Imported (SK974-1, D103/D104)"),
                        Line::from(""),
                        Line::from(format!("   Source: {}", result.source_path.display())),
                        Line::from(format!("   Mount name: {}", result.mount_name)),
                        Line::from(format!("   Format: {}", format_str)),
                        Line::from("   Access: read-only (D103)"),
                        Line::from(""),
                        Line::from(format!("   Artifacts: {}", result.artifact_count)),
                        Line::from(format!("   Checkpoints: {}", result.checkpoint_count)),
                        Line::from(format!("   Events: {}", result.event_count)),
                        Line::from(""),
                        Line::from(format!("   Verification: {}", verification_status)),
                        Line::from(format!("   Hash: {}", &result.content_hash[..16])),
                        Line::from(""),
                        Line::from("   CapsuleImported event recorded in workspace (D104)."),
                    ];

                    // Show doctor results if there were warnings
                    if !result.verification_passed {
                        lines.push(Line::from(""));
                        lines.push(Line::from("   Doctor Results:"));
                        for dr in &result.doctor_results {
                            match dr {
                                crate::memvid_adapter::DiagnosticResult::Warning(msg, remedy) => {
                                    lines.push(Line::from(format!("   ⚠ {}", msg)));
                                    lines.push(Line::from(format!("     → {}", remedy)));
                                }
                                crate::memvid_adapter::DiagnosticResult::Error(msg, remedy) => {
                                    lines.push(Line::from(format!("   ✗ {}", msg)));
                                    lines.push(Line::from(format!("     → {}", remedy)));
                                }
                                _ => {}
                            }
                        }
                    }

                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
                }
                Err(e) => {
                    let error_msg = format!("{}", e);
                    let mut lines = vec![Line::from(format!("❌ Import failed: {}", error_msg))];

                    // Add hints for common errors
                    if error_msg.contains("assphrase") {
                        lines.push(Line::from(""));
                        lines.push(Line::from(
                            "Hint: Set SPECKIT_MEMVID_PASSPHRASE env var for encrypted capsules",
                        ));
                    } else if error_msg.contains("verification failed") {
                        lines.push(Line::from(""));
                        lines.push(Line::from(
                            "Hint: Remove --require-verified to import with warnings",
                        ));
                    }

                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Error));
                }
            }
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Failed to open workspace capsule: {}",
                    e
                ))],
                HistoryCellType::Error,
            ));
        }
    }
}

/// Execute the gc subcommand.
///
/// ## SK974-3: Garbage collection with retention policy
///
/// Usage:
/// - `/speckit.capsule gc` - Run GC with defaults (30 days, keep pinned)
/// - `/speckit.capsule gc --dry-run` - Preview what would be deleted
/// - `/speckit.capsule gc --retention-days 7` - Delete exports older than 7 days
/// - `/speckit.capsule gc --no-keep-pinned` - Also delete pinned exports
///
/// ## Decisions honored
/// - D20: Capsule growth management (retention/compaction)
/// - D116: Hybrid retention (TTL + milestone protection)
fn execute_gc(widget: &mut ChatWidget, args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();

    // Parse --retention-days argument
    let retention_days = if let Some(idx) = parts.iter().position(|&p| p == "--retention-days") {
        parts
            .get(idx + 1)
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(30)
    } else {
        30 // Default per D20
    };

    // Parse --dry-run flag
    let dry_run = parts.contains(&"--dry-run");

    // Parse --no-keep-pinned flag
    let keep_pinned = !parts.contains(&"--no-keep-pinned");

    // Open workspace capsule
    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from(
                "❌ Workspace capsule not found. Run `/speckit.capsule doctor` for details.",
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
            let gc_config = GcConfig {
                retention_days,
                keep_pinned,
                clean_temp_files: true,
                dry_run,
            };

            match handle.gc(&gc_config) {
                Ok(result) => {
                    let mode_str = if result.dry_run {
                        "DRY RUN"
                    } else {
                        "COMPLETED"
                    };

                    let bytes_str = if result.bytes_freed >= 1_000_000 {
                        format!("{:.2} MB", result.bytes_freed as f64 / 1_000_000.0)
                    } else if result.bytes_freed >= 1000 {
                        format!("{:.2} KB", result.bytes_freed as f64 / 1000.0)
                    } else {
                        format!("{} bytes", result.bytes_freed)
                    };

                    let mut lines = vec![
                        Line::from(format!("🗑️ Capsule GC {} (SK974-3, D20/D116)", mode_str)),
                        Line::from(""),
                        Line::from(format!("   Retention: {} days", retention_days)),
                        Line::from(format!(
                            "   Keep pinned: {}",
                            if keep_pinned { "yes" } else { "no" }
                        )),
                        Line::from(""),
                        Line::from(format!("   Exports deleted: {}", result.exports_deleted)),
                        Line::from(format!(
                            "   Exports preserved: {}",
                            result.exports_preserved
                        )),
                        Line::from(format!("   Exports skipped: {}", result.exports_skipped)),
                        Line::from(format!(
                            "   Temp files cleaned: {}",
                            result.temp_files_deleted
                        )),
                        Line::from(format!("   Space freed: {}", bytes_str)),
                    ];

                    if result.dry_run {
                        lines.push(Line::from(""));
                        lines.push(Line::from(
                            "   ⚠ This was a dry run. No files were actually deleted.",
                        ));
                        lines.push(Line::from(
                            "   Run without --dry-run to actually delete files.",
                        ));
                    } else if result.exports_deleted > 0 {
                        lines.push(Line::from(""));
                        lines.push(Line::from(
                            "   ✅ Audit trail event recorded for deletions.",
                        ));
                    }

                    // Show deleted paths if dry-run and there are some
                    if result.dry_run && !result.deleted_paths.is_empty() {
                        lines.push(Line::from(""));
                        lines.push(Line::from("   Would delete:"));
                        for path in result.deleted_paths.iter().take(10) {
                            lines.push(Line::from(format!("   • {}", path.display())));
                        }
                        if result.deleted_paths.len() > 10 {
                            lines.push(Line::from(format!(
                                "   ... and {} more",
                                result.deleted_paths.len() - 10
                            )));
                        }
                    }

                    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
                }
                Err(e) => {
                    widget.history_push(PlainHistoryCell::new(
                        vec![Line::from(format!("❌ GC failed: {}", e))],
                        HistoryCellType::Error,
                    ));
                }
            }
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Failed to open workspace capsule: {}",
                    e
                ))],
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
    fn parse_export_args(
        args: &str,
    ) -> (Option<String>, Option<String>, Option<PathBuf>, bool, bool) {
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

        let no_encrypt = parts.contains(&"--no-encrypt");
        let encrypt = !no_encrypt;

        let unsafe_export = parts.contains(&"--unsafe");
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

    // =========================================================================
    // SK974-1: Import argument parsing tests
    // =========================================================================

    /// Helper to parse import args from a string
    fn parse_import_args(args: &str) -> (Option<PathBuf>, Option<String>, bool) {
        let parts: Vec<&str> = args.split_whitespace().collect();

        // First positional arg after "import" is the path
        let source_path = parts.get(1).map(|s| PathBuf::from(*s));

        let mount_as = if let Some(idx) = parts.iter().position(|&p| p == "--mount-as") {
            parts.get(idx + 1).map(|s| s.to_string())
        } else {
            None
        };

        let require_verified = parts.contains(&"--require-verified");

        (source_path, mount_as, require_verified)
    }

    #[test]
    fn test_import_args_basic() {
        let (source_path, mount_as, require_verified) =
            parse_import_args("import ./exports/capsule.mv2");
        assert_eq!(source_path, Some(PathBuf::from("./exports/capsule.mv2")));
        assert_eq!(mount_as, None);
        assert!(!require_verified, "default should not require verification");
    }

    #[test]
    fn test_import_args_with_mount_name() {
        let (source_path, mount_as, _) =
            parse_import_args("import ./audit.mv2e --mount-as customer-jan");
        assert_eq!(source_path, Some(PathBuf::from("./audit.mv2e")));
        assert_eq!(mount_as, Some("customer-jan".to_string()));
    }

    #[test]
    fn test_import_args_require_verified() {
        let (source_path, _, require_verified) =
            parse_import_args("import ./audit.mv2 --require-verified");
        assert_eq!(source_path, Some(PathBuf::from("./audit.mv2")));
        assert!(require_verified, "--require-verified should enable flag");
    }

    #[test]
    fn test_import_args_all_flags() {
        let (source_path, mount_as, require_verified) =
            parse_import_args("import ./data.mv2e --mount-as data-2026 --require-verified");
        assert_eq!(source_path, Some(PathBuf::from("./data.mv2e")));
        assert_eq!(mount_as, Some("data-2026".to_string()));
        assert!(require_verified);
    }

    #[test]
    fn test_import_options_defaults() {
        let options = ImportOptions::default();
        assert!(options.source_path.as_os_str().is_empty());
        assert!(options.mount_as.is_none());
        assert!(options.interactive, "interactive should default to true");
        assert!(
            !options.require_verified,
            "require_verified should default to false"
        );
    }

    #[test]
    fn test_import_options_for_file() {
        let options = ImportOptions::for_file("/tmp/test.mv2");
        assert_eq!(options.source_path, PathBuf::from("/tmp/test.mv2"));
        assert!(options.mount_as.is_none());
    }

    #[test]
    fn test_import_options_builder() {
        let options = ImportOptions::for_file("/tmp/test.mv2e")
            .with_mount_name("my-mount")
            .require_verified();
        assert_eq!(options.source_path, PathBuf::from("/tmp/test.mv2e"));
        assert_eq!(options.mount_as, Some("my-mount".to_string()));
        assert!(options.require_verified);
    }

    // =========================================================================
    // SK974-3: GC argument parsing tests
    // =========================================================================

    /// Helper to parse gc args from a string
    fn parse_gc_args(args: &str) -> (u32, bool, bool) {
        let parts: Vec<&str> = args.split_whitespace().collect();

        let retention_days = if let Some(idx) = parts.iter().position(|&p| p == "--retention-days")
        {
            parts
                .get(idx + 1)
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(30)
        } else {
            30
        };

        let dry_run = parts.contains(&"--dry-run");
        let keep_pinned = !parts.contains(&"--no-keep-pinned");

        (retention_days, dry_run, keep_pinned)
    }

    #[test]
    fn test_gc_args_defaults() {
        let (retention_days, dry_run, keep_pinned) = parse_gc_args("gc");
        assert_eq!(retention_days, 30, "default retention should be 30 days");
        assert!(!dry_run, "default should not be dry run");
        assert!(keep_pinned, "default should keep pinned exports");
    }

    #[test]
    fn test_gc_args_retention_days() {
        let (retention_days, _, _) = parse_gc_args("gc --retention-days 7");
        assert_eq!(retention_days, 7);
    }

    #[test]
    fn test_gc_args_dry_run() {
        let (_, dry_run, _) = parse_gc_args("gc --dry-run");
        assert!(dry_run);
    }

    #[test]
    fn test_gc_args_no_keep_pinned() {
        let (_, _, keep_pinned) = parse_gc_args("gc --no-keep-pinned");
        assert!(
            !keep_pinned,
            "--no-keep-pinned should disable milestone protection"
        );
    }

    #[test]
    fn test_gc_args_all_flags() {
        let (retention_days, dry_run, keep_pinned) =
            parse_gc_args("gc --retention-days 14 --dry-run --no-keep-pinned");
        assert_eq!(retention_days, 14);
        assert!(dry_run);
        assert!(!keep_pinned);
    }

    #[test]
    fn test_gc_config_defaults() {
        let config = GcConfig::default();
        assert_eq!(config.retention_days, 30, "D20: 30 days default");
        assert!(
            config.keep_pinned,
            "D116: milestone protection enabled by default"
        );
        assert!(config.clean_temp_files);
        assert!(!config.dry_run);
    }

    #[test]
    fn test_gc_config_dry_run() {
        let config = GcConfig::dry_run();
        assert!(config.dry_run);
        assert_eq!(config.retention_days, 30);
    }

    #[test]
    fn test_gc_config_builder() {
        let config = GcConfig::default().with_retention_days(7);
        assert_eq!(config.retention_days, 7);
    }
}
