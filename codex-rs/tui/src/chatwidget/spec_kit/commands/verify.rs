//! SPEC-KIT-900: Verification command for audit trail inspection
//!
//! Usage: /speckit.verify SPEC-ID [--run-id UUID]
//!
//! Displays comprehensive verification report including:
//! - Stage-by-stage execution timeline
//! - Agent spawns and completions with durations
//! - Output files and sizes
//! - SQLite data validation
//! - Quality gate results

#![allow(dead_code)] // Verification helpers pending

use crate::chatwidget::ChatWidget;
use crate::chatwidget::spec_kit::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use ratatui::text::Line;
use rusqlite::{Connection, params};
use std::path::PathBuf;

pub struct VerifyCommand;

impl SpecKitCommand for VerifyCommand {
    fn name(&self) -> &'static str {
        "speckit.verify"
    }

    fn description(&self) -> &'static str {
        "show SPEC verification report and audit trail"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.is_empty() {
            let error_msg = vec![Line::from("Usage: /speckit.verify SPEC-ID [--run-id UUID]")];
            widget.history_push(PlainHistoryCell::new(error_msg, HistoryCellType::Error));
            return;
        }

        let spec_id = parts[0];

        // Parse optional --run-id flag
        let run_id = if parts.len() >= 3 && parts[1] == "--run-id" {
            Some(parts[2].to_string())
        } else {
            // Get most recent run_id for this SPEC
            match get_latest_run_id(spec_id) {
                Ok(rid) => rid,
                Err(e) => {
                    let error_msg = vec![Line::from(format!("Error: {}", e))];
                    widget.history_push(PlainHistoryCell::new(error_msg, HistoryCellType::Error));
                    return;
                }
            }
        };

        // Generate verification report
        match generate_verification_report(spec_id, run_id.as_deref(), &widget.config.cwd) {
            Ok(report) => {
                widget.history_push(PlainHistoryCell::new(
                    report.into_iter().map(|s| Line::from(s)).collect(),
                    HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                let error_msg = vec![Line::from(format!("Verification failed: {}", e))];
                widget.history_push(PlainHistoryCell::new(error_msg, HistoryCellType::Error));
            }
        }
    }
}

fn get_latest_run_id(spec_id: &str) -> Result<Option<String>, String> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let result = conn.query_row(
        "SELECT DISTINCT run_id FROM agent_executions
         WHERE spec_id = ?1 AND run_id IS NOT NULL
         ORDER BY spawned_at DESC LIMIT 1",
        params![spec_id],
        |row| row.get::<_, Option<String>>(0),
    );

    match result {
        Ok(run_id) => Ok(run_id),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(format!("Database query failed: {}", e)),
    }
}

fn get_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    Ok(home.join(".code").join("consensus_artifacts.db"))
}

pub fn generate_verification_report(
    spec_id: &str,
    run_id: Option<&str>,
    cwd: &PathBuf,
) -> Result<Vec<String>, String> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let mut lines = Vec::new();

    // Header
    lines.push("╔═══════════════════════════════════════════════════════════════╗".to_string());
    lines.push(format!(
        "║ SPEC-KIT VERIFICATION REPORT: {}                          ║",
        spec_id
    ));
    lines.push("╚═══════════════════════════════════════════════════════════════╝".to_string());
    lines.push(String::new());

    if let Some(rid) = run_id {
        lines.push(format!("Run ID: {} (full: {})", &rid[..8], rid));
    } else {
        lines.push("Run ID: Not specified (showing all runs)".to_string());
    }
    lines.push(String::new());

    // Query agent executions
    let query = if let Some(rid) = run_id {
        format!(
            "SELECT stage, agent_name, phase_type, spawned_at, completed_at, run_id
             FROM agent_executions
             WHERE spec_id = '{}' AND run_id = '{}'
             ORDER BY spawned_at",
            spec_id, rid
        )
    } else {
        format!(
            "SELECT stage, agent_name, phase_type, spawned_at, completed_at, run_id
             FROM agent_executions
             WHERE spec_id = '{}'
             ORDER BY spawned_at DESC
             LIMIT 50",
            spec_id
        )
    };

    let mut stmt = conn
        .prepare(&query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Failed to execute query: {}", e))?;

    // Group by stage
    let mut stage_data: std::collections::HashMap<String, Vec<(String, String, String, String)>> =
        std::collections::HashMap::new();
    let mut total_agents = 0;
    let mut completed_agents = 0;

    while let Some(row) = rows.next().map_err(|e| format!("Row error: {}", e))? {
        let stage: String = row.get(0).unwrap_or_default();
        let agent_name: String = row.get(1).unwrap_or_default();
        let phase_type: String = row.get(2).unwrap_or_default();
        let spawned: String = row.get(3).unwrap_or_default();
        let completed: Option<String> = row.get(4).unwrap_or(None);

        total_agents += 1;
        if completed.is_some() {
            completed_agents += 1;
        }

        let duration = if let Some(ref comp) = completed {
            calculate_duration(&spawned, comp)
        } else {
            "in progress".to_string()
        };

        let status = if completed.is_some() { "✓" } else { "⏳" };

        stage_data.entry(stage).or_default().push((
            agent_name,
            phase_type,
            duration,
            status.to_string(),
        ));
    }

    // Display stage execution
    lines.push("═══ Stage Execution ═══".to_string());
    lines.push(String::new());

    let stage_order = vec![
        "spec-plan",
        "spec-tasks",
        "spec-implement",
        "spec-validate",
        "spec-audit",
        "spec-unlock",
    ];

    for stage in &stage_order {
        if let Some(agents) = stage_data.get(*stage) {
            let stage_name = stage.strip_prefix("spec-").unwrap_or(stage);
            lines.push(format!(
                "├─ {} ({} agents)",
                stage_name.to_uppercase(),
                agents.len()
            ));

            for (agent_name, phase_type, duration, status) in agents {
                lines.push(format!(
                    "│  {} {} ({}) - {}",
                    status, agent_name, phase_type, duration
                ));
            }

            // Check for output file
            let output_file = format!("{}.md", stage_name);
            let _output_path = cwd
                .join("docs")
                .join(format!("{}-*", spec_id))
                .join(&output_file);

            if let Some(size) = get_file_size_fuzzy(cwd, spec_id, &output_file) {
                lines.push(format!(
                    "│  Output: {} ({})",
                    output_file,
                    format_size(size)
                ));
            } else {
                lines.push(format!("│  Output: {} (not found)", output_file));
            }

            lines.push("│".to_string());
        }
    }

    lines.push(String::new());

    // Query synthesis records
    let synthesis_query = if let Some(rid) = run_id {
        format!(
            "SELECT stage, artifacts_count, LENGTH(output_markdown) as size, status, created_at
             FROM consensus_synthesis
             WHERE spec_id = '{}' AND run_id = '{}'
             ORDER BY created_at",
            spec_id, rid
        )
    } else {
        format!(
            "SELECT stage, artifacts_count, LENGTH(output_markdown) as size, status, created_at
             FROM consensus_synthesis
             WHERE spec_id = '{}'
             ORDER BY created_at DESC
             LIMIT 10",
            spec_id
        )
    };

    let mut synthesis_stmt = conn
        .prepare(&synthesis_query)
        .map_err(|e| format!("Failed to prepare synthesis query: {}", e))?;

    let mut synthesis_rows = synthesis_stmt
        .query([])
        .map_err(|e| format!("Failed to execute synthesis query: {}", e))?;

    lines.push("═══ Synthesis Records ═══".to_string());
    lines.push(String::new());

    let mut synthesis_count = 0;
    while let Some(row) = synthesis_rows
        .next()
        .map_err(|e| format!("Row error: {}", e))?
    {
        let stage: String = row.get(0).unwrap_or_default();
        let artifacts: i64 = row.get(1).unwrap_or(0);
        let size: i64 = row.get(2).unwrap_or(0);
        let status: String = row.get(3).unwrap_or_default();

        let stage_name = stage.strip_prefix("spec-").unwrap_or(&stage);
        lines.push(format!(
            "  {} - {} agents, {} bytes, status: {}",
            stage_name, artifacts, size, status
        ));
        synthesis_count += 1;
    }

    if synthesis_count == 0 {
        lines.push("  (No synthesis records found)".to_string());
    }

    lines.push(String::new());

    // Summary
    lines.push("═══ Summary ═══".to_string());
    lines.push(String::new());
    lines.push(format!("Total Agents: {}", total_agents));
    lines.push(format!(
        "Completed: {} ({:.1}%)",
        completed_agents,
        if total_agents > 0 {
            (completed_agents as f64 / total_agents as f64) * 100.0
        } else {
            0.0
        }
    ));
    lines.push(format!("Stages: {} with data", stage_data.len()));
    lines.push(format!("Synthesis Records: {}", synthesis_count));
    lines.push(String::new());

    // Validation
    let all_complete = completed_agents == total_agents && total_agents > 0;
    let has_synthesis = synthesis_count > 0;

    if all_complete && has_synthesis {
        lines.push("✅ PASS: Pipeline completed successfully".to_string());
    } else if total_agents == 0 {
        lines.push("⚠️  WARNING: No agent executions found for this SPEC/run".to_string());
    } else {
        lines.push(format!(
            "⚠️  IN PROGRESS: {}/{} agents complete",
            completed_agents, total_agents
        ));
    }

    lines.push(String::new());
    lines.push("═══════════════════════════════════════════════════════════════".to_string());

    Ok(lines)
}

fn calculate_duration(start: &str, end: &str) -> String {
    // Parse timestamps and calculate duration
    // Format: "2025-11-04 02:05:00"
    use chrono::NaiveDateTime;

    let start_dt = NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S").ok();
    let end_dt = NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S").ok();

    if let (Some(s), Some(e)) = (start_dt, end_dt) {
        let duration = e.signed_duration_since(s);
        let seconds = duration.num_seconds();

        if seconds < 60 {
            format!("{}s", seconds)
        } else if seconds < 3600 {
            format!("{}m {}s", seconds / 60, seconds % 60)
        } else {
            format!("{}h {}m", seconds / 3600, (seconds % 3600) / 60)
        }
    } else {
        "unknown".to_string()
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn get_file_size_fuzzy(cwd: &PathBuf, spec_id: &str, filename: &str) -> Option<u64> {
    // Try to find the file in docs/SPEC-ID-*/filename
    let docs_dir = cwd.join("docs");

    if let Ok(entries) = std::fs::read_dir(&docs_dir) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string()
                && file_name.starts_with(spec_id) {
                    let file_path = entry.path().join(filename);
                    if let Ok(metadata) = std::fs::metadata(&file_path) {
                        return Some(metadata.len());
                    }
                }
        }
    }

    None
}
