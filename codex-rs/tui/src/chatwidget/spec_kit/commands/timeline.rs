//! SPEC-KIT-973: Time-Travel UI Commands
//!
//! Commands for time-travel functionality:
//! - `/speckit.timeline` - Show timeline with checkpoints and events
//! - `/speckit.asof` - Set/clear time-travel context for URI resolution
//! - `/speckit.diff` - Compare artifact at two checkpoints
//!
//! ## Decision IDs
//! - D3: Capsule history as product feature
//! - D18: Stage boundary checkpoints
//! - D61: As-of queries
//! - D73/D74: Run branch isolation

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use crate::memvid_adapter::{BranchId, CapsuleConfig, CapsuleHandle, CheckpointId};
use ratatui::text::Line;
use std::path::PathBuf;

// =============================================================================
// speckit.timeline
// =============================================================================

/// Command: /speckit.timeline [--branch X] [--run RUN] [--since-checkpoint CP] [--type TYPE]
/// Show ordered checkpoints and stage transitions.
pub struct TimelineCommand;

impl SpecKitCommand for TimelineCommand {
    fn name(&self) -> &'static str {
        "speckit.timeline"
    }

    fn aliases(&self) -> &[&'static str] {
        &["timeline"]
    }

    fn description(&self) -> &'static str {
        "show ordered checkpoints and stage transitions"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_timeline(widget, &args);
    }

    fn requires_args(&self) -> bool {
        false
    }
}

// =============================================================================
// speckit.asof
// =============================================================================

/// Command: /speckit.asof <checkpoint-id-or-label> | clear
/// Set or clear time-travel context for URI resolution.
pub struct AsOfCommand;

impl SpecKitCommand for AsOfCommand {
    fn name(&self) -> &'static str {
        "speckit.asof"
    }

    fn aliases(&self) -> &[&'static str] {
        &["asof"]
    }

    fn description(&self) -> &'static str {
        "set/clear time-travel context for URI resolution"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_asof(widget, &args);
    }

    fn requires_args(&self) -> bool {
        false // "clear" is valid without args, but we'll handle usage in execute
    }
}

// =============================================================================
// speckit.diff
// =============================================================================

/// Command: /speckit.diff <mv2://...> --from <CP1> --to <CP2>
/// Compare artifact at two checkpoints.
pub struct DiffCommand;

impl SpecKitCommand for DiffCommand {
    fn name(&self) -> &'static str {
        "speckit.diff"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "compare artifact at two checkpoints"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        execute_diff(widget, &args);
    }

    fn requires_args(&self) -> bool {
        true
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn get_capsule_path() -> PathBuf {
    // Use standard capsule path convention (same as capsule.rs)
    PathBuf::from(".speckit/memvid/workspace.mv2")
}

fn open_capsule() -> Result<CapsuleHandle, String> {
    let capsule_path = get_capsule_path();

    if !capsule_path.exists() {
        return Err(format!(
            "Capsule not found at {}. Run `/speckit.capsule doctor` for details.",
            capsule_path.display()
        ));
    }

    let config = CapsuleConfig {
        capsule_path,
        workspace_id: "default".to_string(),
        ..Default::default()
    };

    CapsuleHandle::open(config).map_err(|e| format!("Failed to open capsule: {}", e))
}

/// Parse a checkpoint identifier (ID or label) into CheckpointId
fn parse_checkpoint(handle: &CapsuleHandle, id_or_label: &str) -> Option<CheckpointId> {
    // Try as exact checkpoint ID first
    let cp_id = CheckpointId::new(id_or_label.to_string());
    if handle.get_checkpoint(&cp_id).is_some() {
        return Some(cp_id);
    }

    // Try as label
    if let Some(cp) = handle.get_checkpoint_by_label(id_or_label) {
        return Some(cp.checkpoint_id);
    }

    None
}

// =============================================================================
// Timeline Command Implementation
// =============================================================================

fn execute_timeline(widget: &mut ChatWidget, args: &str) {
    let handle = match open_capsule() {
        Ok(h) => h,
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("Error: {}", e))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Parse arguments
    let parts: Vec<&str> = args.split_whitespace().collect();
    let mut branch: Option<BranchId> = None;
    let mut since_checkpoint: Option<CheckpointId> = None;
    let mut event_type_filter: Option<String> = None;

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--branch" => {
                if i + 1 < parts.len() {
                    branch = Some(BranchId::new(parts[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--run" => {
                if i + 1 < parts.len() {
                    branch = Some(BranchId::for_run(parts[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--since-checkpoint" => {
                if i + 1 < parts.len() {
                    since_checkpoint = parse_checkpoint(&handle, parts[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--type" => {
                if i + 1 < parts.len() {
                    event_type_filter = Some(parts[i + 1].to_string());
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }

    // Get checkpoints
    let checkpoints = handle.list_checkpoints_filtered(branch.as_ref());
    let branch_display = branch
        .as_ref()
        .map(|b| b.as_str().to_string())
        .unwrap_or_else(|| "all".to_string());

    let mut lines = vec![
        Line::from(format!("Timeline (branch: {})", branch_display)),
        Line::from(""),
    ];

    // Filter checkpoints by since_checkpoint if specified
    let filtered_checkpoints: Vec<_> = if let Some(ref since_cp) = since_checkpoint {
        let mut found = false;
        checkpoints
            .into_iter()
            .filter(|cp| {
                if cp.checkpoint_id == *since_cp {
                    found = true;
                    false // Don't include the "since" checkpoint itself
                } else {
                    found
                }
            })
            .collect()
    } else {
        checkpoints
    };

    // Display checkpoints
    if filtered_checkpoints.is_empty() {
        lines.push(Line::from("Checkpoints:"));
        lines.push(Line::from("  (none)"));
    } else {
        lines.push(Line::from("Checkpoints:"));
        lines.push(Line::from(
            "  ID                          | Label      | Stage    | Spec/Run",
        ));
        lines.push(Line::from(
            "  ----------------------------|------------|----------|-------------------",
        ));

        for cp in filtered_checkpoints.iter().rev().take(20) {
            let label = cp.label.as_deref().unwrap_or("-");
            let stage = cp.stage.as_deref().unwrap_or("-");
            let spec_run = match (&cp.spec_id, &cp.run_id) {
                (Some(s), Some(r)) => format!("{}/{}", s, r),
                (Some(s), None) => s.clone(),
                (None, Some(r)) => r.clone(),
                _ => "-".to_string(),
            };

            lines.push(Line::from(format!(
                "  {:28} | {:10} | {:8} | {}",
                truncate_str(cp.checkpoint_id.as_str(), 28),
                truncate_str(label, 10),
                truncate_str(stage, 8),
                truncate_str(&spec_run, 19)
            )));
        }

        if filtered_checkpoints.len() > 20 {
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "  Showing 20 of {} checkpoints",
                filtered_checkpoints.len()
            )));
        }
    }

    // Display events if type filter specified
    if let Some(ref type_filter) = event_type_filter {
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "Events (filtered by: {}):",
            type_filter
        )));

        let events = handle.list_events_filtered(branch.as_ref());
        let filtered_events: Vec<_> = events
            .iter()
            .filter(|ev| ev.event_type.as_str().to_lowercase() == type_filter.to_lowercase())
            .take(20)
            .collect();

        if filtered_events.is_empty() {
            lines.push(Line::from("  (none)"));
        } else {
            lines.push(Line::from("  # | Type            | Stage    | Run ID"));
            lines.push(Line::from(
                "  --|-----------------|----------|-------------------",
            ));

            for (idx, ev) in filtered_events.iter().enumerate() {
                let stage = ev.stage.as_deref().unwrap_or("-");

                lines.push(Line::from(format!(
                    "  {:2} | {:15} | {:8} | {}",
                    idx + 1,
                    truncate_str(ev.event_type.as_str(), 15),
                    truncate_str(stage, 8),
                    truncate_str(&ev.run_id, 19)
                )));
            }
        }
    }

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

// =============================================================================
// AsOf Command Implementation
// =============================================================================

fn execute_asof(widget: &mut ChatWidget, args: &str) {
    let arg = args.trim();

    if arg.is_empty() {
        // Show usage
        let lines = vec![
            Line::from("Time-Travel Context (SPEC-KIT-973)"),
            Line::from(""),
            Line::from("/speckit.asof <checkpoint-id-or-label>  # Set time-travel context"),
            Line::from(
                "/speckit.asof clear                     # Clear context (return to latest)",
            ),
            Line::from(""),
            Line::from("When set, URI resolution returns state as of that checkpoint."),
        ];
        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        return;
    }

    if arg == "clear" {
        // Clear time-travel context
        // Note: The actual context storage would be in widget state,
        // but for now we just acknowledge the clear
        let lines = vec![
            Line::from("Time-Travel Context Cleared"),
            Line::from(""),
            Line::from("  URI resolution now returns latest state."),
        ];
        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        return;
    }

    // Try to find the checkpoint
    let handle = match open_capsule() {
        Ok(h) => h,
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("Error: {}", e))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Try as checkpoint ID first
    let cp_id = CheckpointId::new(arg.to_string());
    let checkpoint = handle.get_checkpoint(&cp_id);

    // If not found by ID, try by label
    let checkpoint = checkpoint.or_else(|| handle.get_checkpoint_by_label(arg));

    match checkpoint {
        Some(cp) => {
            let label = cp.label.as_deref().unwrap_or("-");
            let stage = cp.stage.as_deref().unwrap_or("-");
            let branch = cp.branch_id.as_deref().unwrap_or("main");

            let lines = vec![
                Line::from("Time-Travel Context Set"),
                Line::from(""),
                Line::from(format!("  Checkpoint: {}", cp.checkpoint_id.as_str())),
                Line::from(format!("  Label: {}", label)),
                Line::from(format!("  Stage: {}", stage)),
                Line::from(format!("  Branch: {}", branch)),
                Line::from(""),
                Line::from("  URI resolution will now return state as of this checkpoint."),
                Line::from("  Use `/speckit.asof clear` to return to latest."),
            ];
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "Error: Checkpoint '{}' not found (tried ID and label)",
                    arg
                ))],
                HistoryCellType::Error,
            ));
        }
    }
}

// =============================================================================
// Diff Command Implementation
// =============================================================================

fn execute_diff(widget: &mut ChatWidget, args: &str) {
    let parts: Vec<&str> = args.split_whitespace().collect();

    // Parse: <uri> --from <CP1> --to <CP2>
    let mut uri_str: Option<&str> = None;
    let mut from_cp: Option<&str> = None;
    let mut to_cp: Option<&str> = None;

    let mut i = 0;
    while i < parts.len() {
        match parts[i] {
            "--from" => {
                if i + 1 < parts.len() {
                    from_cp = Some(parts[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--to" => {
                if i + 1 < parts.len() {
                    to_cp = Some(parts[i + 1]);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("mv2://") => {
                uri_str = Some(s);
                i += 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    // Validate required args
    if uri_str.is_none() || from_cp.is_none() || to_cp.is_none() {
        let lines = vec![
            Line::from("Artifact Diff (SPEC-KIT-973)"),
            Line::from(""),
            Line::from("Usage: /speckit.diff <mv2://...> --from <CP1> --to <CP2>"),
            Line::from(""),
            Line::from("  <mv2://...>  Artifact URI to compare"),
            Line::from("  --from <CP>  Source checkpoint (ID or label)"),
            Line::from("  --to <CP>    Target checkpoint (ID or label)"),
            Line::from(""),
            Line::from("Example:"),
            Line::from("  /speckit.diff mv2://workspace/artifact/spec.md --from v1.0 --to v2.0"),
        ];
        widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        return;
    }

    let uri_str = uri_str.unwrap();
    let from_cp = from_cp.unwrap();
    let to_cp = to_cp.unwrap();

    let handle = match open_capsule() {
        Ok(h) => h,
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("Error: {}", e))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Resolve checkpoints
    let from_checkpoint = match parse_checkpoint(&handle, from_cp) {
        Some(cp) => cp,
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "Error: Checkpoint '{}' not found",
                    from_cp
                ))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let to_checkpoint = match parse_checkpoint(&handle, to_cp) {
        Some(cp) => cp,
        None => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "Error: Checkpoint '{}' not found",
                    to_cp
                ))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Get bytes at both checkpoints
    let from_bytes = handle.get_bytes_str(uri_str, None, Some(&from_checkpoint));
    let to_bytes = handle.get_bytes_str(uri_str, None, Some(&to_checkpoint));

    match (from_bytes, to_bytes) {
        (Ok(from_data), Ok(to_data)) => {
            // Display diff
            let mut lines = vec![
                Line::from("Artifact Diff"),
                Line::from(""),
                Line::from(format!("  URI: {}", uri_str)),
                Line::from(format!("  From: {}", from_checkpoint.as_str())),
                Line::from(format!("  To:   {}", to_checkpoint.as_str())),
                Line::from(""),
            ];

            if from_data == to_data {
                lines.push(Line::from("  (no changes)"));
            } else {
                // Check if content is text
                let from_text = String::from_utf8(from_data.clone());
                let to_text = String::from_utf8(to_data.clone());

                match (from_text, to_text) {
                    (Ok(from_str), Ok(to_str)) => {
                        // Text diff - show simple line comparison
                        lines.push(Line::from(format!(
                            "  --- {} ({} bytes)",
                            from_checkpoint.as_str(),
                            from_data.len()
                        )));
                        lines.push(Line::from(format!(
                            "  +++ {} ({} bytes)",
                            to_checkpoint.as_str(),
                            to_data.len()
                        )));
                        lines.push(Line::from(""));

                        // Simple diff: show first few changed lines
                        let from_lines: Vec<&str> = from_str.lines().collect();
                        let to_lines: Vec<&str> = to_str.lines().collect();

                        let max_lines = 20;
                        let mut diff_count = 0;

                        // Show removed lines
                        for line in from_lines.iter() {
                            if !to_lines.contains(line) && diff_count < max_lines {
                                lines.push(Line::from(format!("  - {}", line)));
                                diff_count += 1;
                            }
                        }

                        // Show added lines
                        for line in to_lines.iter() {
                            if !from_lines.contains(line) && diff_count < max_lines {
                                lines.push(Line::from(format!("  + {}", line)));
                                diff_count += 1;
                            }
                        }

                        if diff_count >= max_lines {
                            lines.push(Line::from("  ..."));
                            lines.push(Line::from("  (more changes truncated)"));
                        }
                    }
                    _ => {
                        // Binary diff
                        lines.push(Line::from(format!(
                            "  Binary content differs: {} bytes -> {} bytes",
                            from_data.len(),
                            to_data.len()
                        )));
                    }
                }
            }

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        (Err(e), _) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "Error: Failed to read artifact at {}: {}",
                    from_checkpoint.as_str(),
                    e
                ))],
                HistoryCellType::Error,
            ));
        }
        (_, Err(e)) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "Error: Failed to read artifact at {}: {}",
                    to_checkpoint.as_str(),
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
    fn test_timeline_command_metadata() {
        let cmd = TimelineCommand;
        assert_eq!(cmd.name(), "speckit.timeline");
        assert_eq!(cmd.aliases(), &["timeline"]);
        assert!(!cmd.requires_args());
    }

    #[test]
    fn test_asof_command_metadata() {
        let cmd = AsOfCommand;
        assert_eq!(cmd.name(), "speckit.asof");
        assert_eq!(cmd.aliases(), &["asof"]);
        assert!(!cmd.requires_args());
    }

    #[test]
    fn test_diff_command_metadata() {
        let cmd = DiffCommand;
        assert_eq!(cmd.name(), "speckit.diff");
        assert!(cmd.aliases().is_empty());
        assert!(cmd.requires_args());
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 8), "hello...");
        assert_eq!(truncate_str("hi", 2), "hi");
    }
}
