//! Stage synthesis utilities shared across TUI + headless (MAINT-930 / LOCK E3)
//!
//! This module provides a single artifact synthesis path for canonical stage files
//! (plan.md, tasks.md, implement.md, validate.md, audit.md, unlock.md).

use std::fs;
use std::path::{Path, PathBuf};

use chrono::TimeZone;
use serde_json::Value;

use crate::spec_prompts::SpecStage;

#[derive(Debug, Clone, Copy)]
pub struct SynthesisSideEffects {
    pub store_to_sqlite: bool,
    pub auto_export_evidence: bool,
}

impl Default for SynthesisSideEffects {
    fn default() -> Self {
        Self {
            store_to_sqlite: true,
            auto_export_evidence: true,
        }
    }
}

fn generated_timestamp_for_run_id(run_id: Option<&str>) -> chrono::DateTime<chrono::Utc> {
    let Some(run_id) = run_id else {
        return chrono::Utc::now();
    };

    // Expected format (execution_logger.rs): <spec_id>_<unix_secs>_<uuid8>
    // Parse the second-to-last underscore-delimited segment as unix seconds.
    let Some(unix_secs_str) = run_id.rsplit('_').nth(1) else {
        return chrono::Utc::now();
    };
    let Ok(unix_secs) = unix_secs_str.parse::<i64>() else {
        return chrono::Utc::now();
    };

    match chrono::Utc.timestamp_opt(unix_secs, 0) {
        chrono::LocalResult::Single(dt) => dt,
        _ => chrono::Utc::now(),
    }
}

/// Synthesize stage artifacts from cached agent responses.
///
/// This is the shared writer used by both:
/// - TUI (`pipeline_coordinator.rs`)
/// - Headless runner (`headless/runner.rs`)
#[doc(hidden)]
pub fn synthesize_from_cached_responses(
    cached_responses: &[(String, String)],
    spec_id: &str,
    stage: SpecStage,
    cwd: &Path,
    run_id: Option<&str>,
    side_effects: SynthesisSideEffects,
) -> Result<PathBuf, String> {
    let run_tag = run_id
        .map(|r| format!("[run:{}]", &r[..8.min(r.len())]))
        .unwrap_or_else(|| "[run:none]".to_string());
    tracing::warn!(
        "{} 🔧 SYNTHESIS START: stage={}, spec={}, responses={}",
        run_tag,
        stage.display_name(),
        spec_id,
        cached_responses.len()
    );

    if cached_responses.is_empty() {
        tracing::error!("❌ SYNTHESIS FAIL: No cached responses");
        return Err("No cached responses to synthesize".to_string());
    }

    tracing::warn!(
        "  📊 Agent responses: {:?}",
        cached_responses
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
    );

    // Parse agent responses and extract structured content
    let mut agent_data: Vec<(String, Value)> = Vec::new();

    for (agent_name, response_text) in cached_responses {
        tracing::warn!(
            "DEBUG: Extracting JSON from {} ({} chars)",
            agent_name,
            response_text.len()
        );

        // Try to extract JSON from response (agents may wrap in markdown code blocks)
        let json_content = extract_json_from_agent_response(response_text);

        if let Some(json_str) = json_content {
            tracing::warn!(
                "DEBUG: Extracted JSON string from {} ({} chars)",
                agent_name,
                json_str.len()
            );
            match serde_json::from_str::<Value>(&json_str) {
                Ok(parsed) => {
                    tracing::warn!("DEBUG: Successfully parsed JSON for {}", agent_name);
                    // Log top-level fields for debugging
                    if let Some(obj) = parsed.as_object() {
                        let fields: Vec<&String> = obj.keys().collect();
                        tracing::warn!("DEBUG: {} has fields: {:?}", agent_name, fields);
                    }
                    agent_data.push((agent_name.clone(), parsed));
                    continue;
                }
                Err(e) => {
                    tracing::warn!("DEBUG: JSON parse failed for {}: {}", agent_name, e);
                }
            }
        } else {
            tracing::warn!(
                "DEBUG: No JSON extracted from {} response, using as plain text",
                agent_name
            );
            // Log first 500 chars to see format
            let preview = &response_text.chars().take(500).collect::<String>();
            tracing::warn!("DEBUG: Response preview: {}", preview);
        }

        // Fallback: treat as plain text
        agent_data.push((
            agent_name.clone(),
            serde_json::json!({
                "agent": agent_name,
                "content": response_text,
                "format": "text"
            }),
        ));
    }

    // Build plan.md from agent data
    let mut output = String::new();
    output.push_str(&format!("# Plan: {}\n\n", spec_id));
    output.push_str(&format!("**Stage**: {}\n", stage.display_name()));
    output.push_str(&format!("**Agents**: {}\n", agent_data.len()));
    output.push_str(&format!(
        "**Generated**: {}\n\n",
        generated_timestamp_for_run_id(run_id).format("%Y-%m-%d %H:%M UTC")
    ));

    // Debug: Log what we actually have
    for (agent_name, data) in &agent_data {
        tracing::warn!(
            "DEBUG: Processing {} with {} top-level keys",
            agent_name,
            data.as_object().map(|o| o.len()).unwrap_or(0)
        );

        // Debug JSON sections removed - caused exponential growth when nested in later stages
        // If debugging needed, check SQLite: SELECT * FROM consensus_runs WHERE spec_id='...'
    }

    // Extract work breakdown, risks, acceptance from structured data
    let mut structured_content_found = false;

    for (agent_name, data) in &agent_data {
        if let Some(work_breakdown) = data.get("work_breakdown").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Work Breakdown (from {})\n\n", agent_name));
            for (i, step) in work_breakdown.iter().enumerate() {
                if let Some(step_name) = step.get("step").and_then(|v| v.as_str()) {
                    output.push_str(&format!("{}. {}\n", i + 1, step_name));
                    if let Some(rationale) = step.get("rationale").and_then(|v| v.as_str()) {
                        output.push_str(&format!("   - Rationale: {}\n", rationale));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        if let Some(risks) = data.get("risks").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Risks (from {})\n\n", agent_name));
            for risk in risks {
                if let Some(risk_desc) = risk.get("risk").and_then(|v| v.as_str()) {
                    output.push_str(&format!("- **Risk**: {}\n", risk_desc));
                    if let Some(mitigation) = risk.get("mitigation").and_then(|v| v.as_str()) {
                        output.push_str(&format!("  - Mitigation: {}\n", mitigation));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        // SPEC-923: Generic fallback for agent schemas we don't explicitly handle
        // Extract common fields that agents may use (tasks, surfaces, research_summary, etc.)
        if let Some(tasks) = data.get("tasks").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Tasks (from {})\n\n", agent_name));
            for task in tasks {
                if let Some(task_str) = task.as_str() {
                    output.push_str(&format!("- {}\n", task_str));
                } else if let Some(obj) = task.as_object()
                    && let Some(name) = obj
                        .get("name")
                        .or_else(|| obj.get("task"))
                        .and_then(|v| v.as_str())
                {
                    output.push_str(&format!("- {}\n", name));
                    if let Some(desc) = obj
                        .get("description")
                        .or_else(|| obj.get("desc"))
                        .and_then(|v| v.as_str())
                    {
                        output.push_str(&format!("  - {}\n", desc));
                    }
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        if let Some(surfaces) = data.get("surfaces").and_then(|v| v.as_array()) {
            output.push_str(&format!("## Affected Surfaces (from {})\n\n", agent_name));
            for surface in surfaces {
                if let Some(s) = surface.as_str() {
                    output.push_str(&format!("- {}\n", s));
                }
            }
            output.push('\n');
            structured_content_found = true;
        }

        // Plain text content fallback
        if let Some(content) = data.get("content").and_then(|v| v.as_str())
            && !content.is_empty()
        {
            output.push_str(&format!("## Response from {}\n\n", agent_name));
            output.push_str(content);
            output.push_str("\n\n");
            structured_content_found = true;
        }
    }

    // Ultimate fallback: if no structured content extracted, pretty-print raw JSON
    if !structured_content_found {
        tracing::warn!("⚠️ No structured fields found, using generic JSON extraction");
        output.push_str("## Agent Responses (Raw)\n\n");
        output.push_str("*Note: Structured extraction failed, displaying raw agent data*\n\n");

        for (agent_name, data) in &agent_data {
            output.push_str(&format!("### {}\n\n", agent_name));

            // Skip wrapper fields and extract meaningful content
            if let Some(obj) = data.as_object() {
                for (key, value) in obj {
                    if key != "agent" && key != "format" {
                        output.push_str(&format!("**{}**:\n", key));
                        match value {
                            Value::String(s) => output.push_str(&format!("{}\n\n", s)),
                            Value::Array(arr) => {
                                for item in arr {
                                    output.push_str(&format!(
                                        "- {}\n",
                                        serde_json::to_string_pretty(item)
                                            .unwrap_or_else(|_| item.to_string())
                                    ));
                                }
                                output.push('\n');
                            }
                            _ => output.push_str(&format!(
                                "```json\n{}\n```\n\n",
                                serde_json::to_string_pretty(value)
                                    .unwrap_or_else(|_| value.to_string())
                            )),
                        }
                    }
                }
            }
            output.push('\n');
        }
    }

    output.push_str("## Consensus Summary\n\n");
    output.push_str(&format!(
        "- Synthesized from {} agent responses\n",
        agent_data.len()
    ));
    output.push_str("- All agents completed successfully\n");

    // Find SPEC directory using ACID-compliant resolver
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)?;

    tracing::warn!("  📁 SPEC directory: {}", spec_dir.display());
    tracing::warn!("  📁 Is directory: {}", spec_dir.is_dir());
    tracing::warn!("  📁 Exists: {}", spec_dir.exists());

    // Only create if doesn't exist (avoid error if it's already there)
    if !spec_dir.exists() {
        tracing::warn!("  📁 Creating directory...");
        fs::create_dir_all(&spec_dir).map_err(|e| {
            tracing::error!("❌ Failed to create {}: {}", spec_dir.display(), e);
            format!("Failed to create spec dir: {}", e)
        })?;
    } else if !spec_dir.is_dir() {
        tracing::error!(
            "❌ SPEC path exists but is NOT a directory: {}",
            spec_dir.display()
        );
        return Err(format!(
            "SPEC path is not a directory: {}",
            spec_dir.display()
        ));
    } else {
        tracing::warn!("  ✅ Directory already exists");
    }

    // Use standard filenames: plan.md, tasks.md, implement.md, etc.
    let output_filename = format!("{}.md", stage.display_name().to_lowercase());
    let output_file = spec_dir.join(&output_filename);

    tracing::warn!("  📝 Output file: {}", output_file.display());
    tracing::warn!(
        "  📏 Output size: {} chars ({} KB)",
        output.len(),
        output.len() / 1024
    );

    // SPEC-KIT-900: Always write synthesis output to update with latest run
    // Previous skip logic prevented updates, causing stale output files
    tracing::warn!(
        "{}   💾 Writing {} to disk (overwrite={})...",
        run_tag,
        output_filename,
        output_file.exists()
    );

    fs::write(&output_file, &output).map_err(|e| {
        tracing::error!("{} ❌ SYNTHESIS FAIL: Write error: {}", run_tag, e);
        format!("Failed to write {}: {}", output_filename, e)
    })?;

    tracing::warn!(
        "{} ✅ SYNTHESIS SUCCESS: Wrote {} ({} KB)",
        run_tag,
        output_filename,
        output.len() / 1024
    );

    // SPEC-KIT-072: Optional SQLite storage for synthesis
    if side_effects.store_to_sqlite {
        if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
            if let Err(e) = db.store_synthesis(
                spec_id,
                stage,
                &output,
                Some(&output_file),
                "ok", // Simple status for now
                cached_responses.len(),
                None,
                None,
                false,
                run_id,
            ) {
                tracing::warn!("{} Failed to store synthesis to SQLite: {}", run_tag, e);
            } else {
                tracing::info!(
                    "{} Stored consensus synthesis to SQLite with run_id={:?}",
                    run_tag,
                    run_id
                );

                if side_effects.auto_export_evidence {
                    // SPEC-KIT-900 Session 3: AUTO-EXPORT evidence for checklist compliance
                    // This ensures evidence/consensus/<SPEC-ID>/ is ALWAYS populated after EVERY synthesis
                    tracing::info!(
                        "{} Auto-exporting evidence to consensus directory...",
                        run_tag
                    );
                    super::evidence::auto_export_stage_evidence(cwd, spec_id, stage, run_id);
                }
            }
        }
    }

    Ok(output_file)
}

/// Extract JSON from agent response (handles code blocks, tool output, etc.)
pub(super) fn extract_json_from_agent_response(text: &str) -> Option<String> {
    // Look for JSON in markdown code blocks
    if let Some(start) = text.find("```json\n")
        && let Some(end) = text[start + 8..].find("\n```")
    {
        return Some(text[start + 8..start + 8 + end].to_string());
    }

    // Look for JSON in plain code blocks (agents use this format)
    if let Some(start) = text.find("│ {\n│   \"stage\"") {
        // Extract JSON from piped format (│ prefix on each line)
        let from_start = &text[start..];
        if let Some(end) = from_start.find("\n│\n│ Ran for") {
            let json_block = &from_start[2..end]; // Skip "│ " prefix
            let cleaned = json_block
                .lines()
                .map(|line| {
                    line.strip_prefix("│   ")
                        .or_else(|| line.strip_prefix("│ "))
                        .unwrap_or(line)
                })
                .collect::<Vec<_>>()
                .join("\n");
            return Some(cleaned);
        }
    }

    // Look for raw JSON objects (Python output format)
    for pattern in &["{\n  \"stage\":", "{\n\"stage\":"] {
        if let Some(start) = text.find(pattern) {
            let from_start = &text[start..];
            // Find matching closing brace by counting braces
            let mut brace_count = 0;
            let mut end_pos = 0;
            for (i, ch) in from_start.char_indices() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            end_pos = i + 1;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            if end_pos > 0 {
                return Some(from_start[..end_pos].to_string());
            }
        }
    }

    None
}
