//! Asynchronous broker for fetching quality gate artefacts.
//!
//! SPEC-KIT-068 restores the quality gate workflow by moving all local-memory
//! lookups off the Ratatui UI thread. The broker accepts lightweight commands
//! and performs the MCP calls inside Tokio tasks, emitting [`AppEvent`]s when
//! results are available (or when retries are exhausted).

use std::collections::HashMap;
use std::sync::Arc;

use codex_core::mcp_connection_manager::McpConnectionManager;
use serde_json::{json, Value};
use tokio::sync::{Mutex, mpsc};
use tokio::time::{Duration, sleep};

use crate::app_event::AppEvent;
use crate::app_event_sender::AppEventSender;

use super::state::QualityCheckpoint;

const RETRY_DELAYS_MS: [u64; 3] = [100, 200, 400];
const MIN_PARTICIPATING_AGENTS: usize = 2;

/// Payload returned for each agent artefact found in local-memory.
#[derive(Debug, Clone)]
pub(crate) struct QualityGateAgentPayload {
    pub agent: String,
    pub gate: Option<String>,
    pub content: serde_json::Value,
}

/// Result metadata for quality gate agent artefact retrieval.
#[derive(Debug, Clone)]
pub(crate) struct QualityGateBrokerResult {
    pub spec_id: String,
    pub checkpoint: QualityCheckpoint,
    pub attempts: u32,
    pub info_lines: Vec<String>,
    pub missing_agents: Vec<String>,
    pub found_agents: Vec<String>,
    pub payload: Result<Vec<QualityGateAgentPayload>, String>,
}

/// Result metadata for GPT-5 validation artefacts.
#[derive(Debug, Clone)]
pub(crate) struct QualityGateValidationResult {
    pub spec_id: String,
    pub checkpoint: QualityCheckpoint,
    pub attempts: u32,
    pub info_lines: Vec<String>,
    pub payload: Result<serde_json::Value, String>,
}

#[derive(Debug)]
enum QualityGateCommand {
    FetchAgentPayloads {
        spec_id: String,
        checkpoint: QualityCheckpoint,
        expected_agents: Vec<String>,
        gate_stages: Vec<String>,
    },
    FetchAgentPayloadsFromMemory {
        spec_id: String,
        checkpoint: QualityCheckpoint,
        expected_agents: Vec<String>,
        agent_ids: Vec<String>,
    },
    FetchValidationPayload {
        spec_id: String,
        checkpoint: QualityCheckpoint,
    },
}

/// Handle for submitting asynchronous quality gate work.
#[derive(Clone)]
pub(crate) struct QualityGateBroker {
    sender: mpsc::UnboundedSender<QualityGateCommand>,
}

impl QualityGateBroker {
    pub(crate) fn new(
        app_event_tx: AppEventSender,
        mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    ) -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<QualityGateCommand>();
        let tx_results = app_event_tx.clone();
        let manager = mcp_manager.clone();

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    QualityGateCommand::FetchAgentPayloads {
                        spec_id,
                        checkpoint,
                        expected_agents,
                        gate_stages,
                    } => {
                        let result = fetch_agent_payloads(
                            manager.clone(),
                            &spec_id,
                            checkpoint,
                            &expected_agents,
                            &gate_stages,
                        )
                        .await;

                        tx_results.send(AppEvent::SpecKitQualityGateResults {
                            broker_result: result,
                        });
                    }
                    QualityGateCommand::FetchAgentPayloadsFromMemory {
                        spec_id,
                        checkpoint,
                        expected_agents,
                        agent_ids,
                    } => {
                        let result = fetch_agent_payloads_from_memory(
                            &spec_id,
                            checkpoint,
                            &expected_agents,
                            &agent_ids,
                        )
                        .await;

                        tx_results.send(AppEvent::SpecKitQualityGateResults {
                            broker_result: result,
                        });
                    }
                    QualityGateCommand::FetchValidationPayload {
                        spec_id,
                        checkpoint,
                    } => {
                        let result =
                            fetch_validation_payload(manager.clone(), &spec_id, checkpoint).await;

                        tx_results.send(AppEvent::SpecKitQualityGateValidationResults {
                            broker_result: result,
                        });
                    }
                }
            }
        });

        Self { sender: tx }
    }

    /// Request asynchronous retrieval of quality gate agent outputs.
    pub(crate) fn fetch_agent_payloads(
        &self,
        spec_id: impl Into<String>,
        checkpoint: QualityCheckpoint,
        expected_agents: Vec<String>,
        gate_stages: Vec<String>,
    ) {
        if let Err(err) = self.sender.send(QualityGateCommand::FetchAgentPayloads {
            spec_id: spec_id.into(),
            checkpoint,
            expected_agents,
            gate_stages,
        }) {
            tracing::error!("quality gate broker channel closed: {err}");
        }
    }

    /// Request asynchronous retrieval from AGENT_MANAGER memory (native orchestrator)
    pub(crate) fn fetch_agent_payloads_from_memory(
        &self,
        spec_id: impl Into<String>,
        checkpoint: QualityCheckpoint,
        expected_agents: Vec<String>,
        agent_ids: Vec<String>,
    ) {
        if let Err(err) = self.sender.send(QualityGateCommand::FetchAgentPayloadsFromMemory {
            spec_id: spec_id.into(),
            checkpoint,
            expected_agents,
            agent_ids,
        }) {
            tracing::error!("quality gate broker channel closed: {err}");
        }
    }

    /// Request asynchronous retrieval of GPT-5 validation artefacts.
    pub(crate) fn fetch_validation_payload(
        &self,
        spec_id: impl Into<String>,
        checkpoint: QualityCheckpoint,
    ) {
        if let Err(err) = self
            .sender
            .send(QualityGateCommand::FetchValidationPayload {
                spec_id: spec_id.into(),
                checkpoint,
            })
        {
            tracing::error!("quality gate broker channel closed: {err}");
        }
    }
}

async fn fetch_agent_payloads(
    _mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    expected_agents: &[String],
    _gate_stages: &[String],
) -> QualityGateBrokerResult {
    fetch_agent_payloads_from_filesystem(spec_id, checkpoint, expected_agents).await
}

/// Collect agent payloads from AGENT_MANAGER (native orchestrator)
async fn fetch_agent_payloads_from_memory(
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    expected_agents: &[String],
    agent_ids: &[String],
) -> QualityGateBrokerResult {
    let mut info_lines = Vec::new();
    let mut results_map: HashMap<String, QualityGateAgentPayload> = HashMap::new();

    info_lines.push(format!("Collecting from AGENT_MANAGER: {} agents", agent_ids.len()));

    // Read from AGENT_MANAGER memory (native orchestrator path)
    let manager = codex_core::agent_tool::AGENT_MANAGER.read().await;

    for agent_id in agent_ids {
        if let Some(agent) = manager.get_agent(agent_id) {
            info_lines.push(format!("Agent {} ({}): {:?}", agent_id, agent.model, agent.status));

            if let Some(result_text) = &agent.result {
                info_lines.push(format!("  Result length: {} chars", result_text.len()));

                // Extract JSON from result
                match extract_json_from_content(result_text) {
                    Some(json_str) => {
                        info_lines.push(format!("  Extracted JSON ({} chars)", json_str.len()));

                        match serde_json::from_str::<Value>(&json_str) {
                            Ok(json_val) => {
                                let stage = json_val.get("stage").and_then(|v| v.as_str());
                                let agent_name = json_val.get("agent").and_then(|v| v.as_str());
                                info_lines.push(format!("  JSON stage: {:?}, agent: {:?}", stage, agent_name));

                                // Check if this is a quality gate artifact
                                if let Some(stage) = stage {
                                    if stage.starts_with("quality-gate-") {
                                        if let Some(agent_name) = agent_name {
                                            // Match against expected_agents (flexible: exact match or starts-with)
                                            let matched_expected = expected_agents.iter().find(|expected| {
                                                let expected_lower = expected.to_lowercase();
                                                let agent_lower = agent_name.to_lowercase();
                                                // Exact match OR agent starts with expected (e.g., "claude-haiku-4-5" matches "claude")
                                                agent_lower == expected_lower || agent_lower.starts_with(&format!("{}-", expected_lower))
                                            });

                                            if let Some(expected) = matched_expected {
                                                info_lines.push(format!("Found {} (as '{}') from memory", expected, agent_name));

                                                // Use expected name as key for deduplication
                                                results_map.insert(
                                                    expected.to_lowercase(),
                                                    QualityGateAgentPayload {
                                                        agent: expected.to_string(),
                                                        gate: Some(stage.to_string()),
                                                        content: json_val,
                                                    },
                                                );
                                            } else {
                                                info_lines.push(format!("  Agent '{}' not in expected list", agent_name));
                                            }
                                        } else {
                                            info_lines.push("  Missing 'agent' field in JSON".to_string());
                                        }
                                    } else {
                                        info_lines.push(format!("  Stage '{}' doesn't start with 'quality-gate-'", stage));
                                    }
                                }
                            }
                            Err(e) => {
                                info_lines.push(format!("  JSON parse error: {}", e));
                                info_lines.push(format!("  JSON length: {} chars", json_str.len()));
                                info_lines.push(format!("  First 200 chars: {}", &json_str.chars().take(200).collect::<String>()));
                                info_lines.push(format!("  Last 200 chars: {}", &json_str.chars().rev().take(200).collect::<Vec<_>>().into_iter().rev().collect::<String>()));
                            }
                        }
                    }
                    None => {
                        info_lines.push("  No JSON found via standard extraction".to_string());

                        // LAST RESORT: Search for "quality-gate-clarify" string and extract surrounding JSON
                        // The code agent buries JSON deep in verbose output
                        // Try ALL occurrences from last to first (actual response usually near end)
                        let marker = r#""stage": "quality-gate-clarify""#;
                        let mut search_pos = result_text.len();
                        let mut found_valid_json = false;

                        while let Some(relative_pos) = result_text[..search_pos].rfind(marker) {
                            info_lines.push(format!("  Found stage marker at position {} (searching backwards)", relative_pos));

                            // Search backwards for opening brace (within 5000 chars to handle large JSON)
                            let search_start = relative_pos.saturating_sub(5000);
                            let before = &result_text[search_start..relative_pos];

                            if let Some(rel_open) = before.rfind('{') {
                                let abs_open = search_start + rel_open;
                                let from_open = &result_text[abs_open..];

                                // Find matching closing brace using BYTE indices (char_indices, not chars().enumerate())
                                // This is critical because string slicing uses bytes, not character positions
                                let mut depth = 0;
                                let mut json_end_bytes = 0;
                                for (byte_pos, ch) in from_open.char_indices() {
                                    if ch == '{' {
                                        depth += 1;
                                    }
                                    if ch == '}' {
                                        depth -= 1;
                                        if depth == 0 {
                                            // Add char length to include the closing brace itself
                                            json_end_bytes = byte_pos + ch.len_utf8();
                                            break;
                                        }
                                    }
                                }

                                if json_end_bytes > 0 {
                                    let candidate = &from_open[..json_end_bytes];
                                    info_lines.push(format!("  Extracted via stage-marker search ({} bytes)", candidate.len()));

                                    match serde_json::from_str::<Value>(candidate) {
                                        Ok(json_val) => {
                                            // Validate this is actually a quality-gate JSON
                                            let stage_field = json_val.get("stage").and_then(|v| v.as_str());
                                            if stage_field != Some("quality-gate-clarify") {
                                                info_lines.push(format!("  Extracted JSON has wrong stage field: {:?}, trying earlier occurrence", stage_field));
                                                // Continue searching for earlier occurrences
                                                search_pos = relative_pos;
                                                continue;
                                            }

                                            // Valid quality-gate JSON found!
                                            if let Some(agent_name) = json_val.get("agent").and_then(|v| v.as_str()) {
                                                let matched_expected = expected_agents.iter().find(|expected| {
                                                    let expected_lower = expected.to_lowercase();
                                                    let agent_lower = agent_name.to_lowercase();
                                                    agent_lower == expected_lower || agent_lower.starts_with(&format!("{}-", expected_lower))
                                                });

                                                if let Some(expected) = matched_expected {
                                                    info_lines.push(format!("Found {} (via fallback extraction) from memory", expected));
                                                    results_map.insert(
                                                        expected.to_lowercase(),
                                                        QualityGateAgentPayload {
                                                            agent: expected.to_string(),
                                                            gate: Some("quality-gate-clarify".to_string()),
                                                            content: json_val,
                                                        },
                                                    );
                                                    found_valid_json = true;
                                                    break; // Found valid JSON, stop searching
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            info_lines.push(format!("  Stage-marker JSON parse failed: {}, trying earlier occurrence", e));
                                            search_pos = relative_pos;
                                            continue;
                                        }
                                    }
                                } else {
                                    info_lines.push("  No matching closing brace found, trying earlier occurrence".to_string());
                                    search_pos = relative_pos;
                                    continue;
                                }
                            } else {
                                info_lines.push("  No opening brace found before marker, trying earlier occurrence".to_string());
                                search_pos = relative_pos;
                                continue;
                            }
                        }

                        if !found_valid_json {
                            info_lines.push("  No valid quality-gate JSON found in any occurrence".to_string());
                            info_lines.push(format!("  First 500 chars: {}", &result_text.chars().take(500).collect::<String>()));
                        }
                    }
                }
            } else {
                info_lines.push("  No result available".to_string());
            }
        } else {
            info_lines.push(format!("Agent {} not found in AGENT_MANAGER", agent_id));
        }
    }

    let found_agents: Vec<String> = results_map.keys().cloned().collect();
    let missing_agents: Vec<String> = expected_agents
        .iter()
        .filter(|a| !found_agents.contains(&a.to_lowercase()))
        .cloned()
        .collect();

    info_lines.push(format!("Found {}/{} agents via memory", found_agents.len(), expected_agents.len()));

    QualityGateBrokerResult {
        spec_id: spec_id.to_string(),
        checkpoint,
        attempts: 1,
        info_lines,
        missing_agents,
        found_agents,
        payload: if results_map.len() == expected_agents.len() {
            Ok(results_map.values().cloned().collect())
        } else {
            Err(format!("Only found {}/{} agents", results_map.len(), expected_agents.len()))
        },
    }
}

/// Collect agent payloads from filesystem (legacy LLM orchestrator path)
async fn fetch_agent_payloads_from_filesystem(
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    expected_agents: &[String],
) -> QualityGateBrokerResult {
    let mut info_lines = Vec::new();
    let mut results_map: HashMap<String, QualityGateAgentPayload> = HashMap::new();

    // Scan .code/agents/ directory for result files
    let agents_dir = std::path::Path::new(".code/agents");

    if !agents_dir.exists() {
        return QualityGateBrokerResult {
            spec_id: spec_id.to_string(),
            checkpoint,
            attempts: 1,
            info_lines: vec!["Agents directory .code/agents not found".to_string()],
            missing_agents: expected_agents.to_vec(),
            found_agents: vec![],
            payload: Err("Agents directory not found".to_string()),
        };
    }

    match std::fs::read_dir(agents_dir) {
        Ok(entries) => {
            // Limit scan to prevent stack overflow with too many agents
            let mut scanned = 0;
            const MAX_SCAN: usize = 100;

            for entry in entries.flatten() {
                if scanned >= MAX_SCAN {
                    info_lines.push(format!("Scanned {} agents (limit reached)", MAX_SCAN));
                    break;
                }

                if !entry.path().is_dir() {
                    continue;
                }

                let result_path = entry.path().join("result.txt");

                // Only check recent result files (last 1 hour)
                if let Ok(metadata) = std::fs::metadata(&result_path) {
                    if let Ok(modified) = metadata.modified() {
                        if let Ok(elapsed) = modified.elapsed() {
                            if elapsed.as_secs() > 3600 {
                                continue; // Skip old agents
                            }
                        }
                    }
                }

                scanned += 1;

                if let Ok(content) = std::fs::read_to_string(&result_path) {
                    // Extract JSON from content
                    if let Some(json_str) = extract_json_from_content(&content) {
                        if let Ok(json_val) = serde_json::from_str::<Value>(&json_str) {
                            // Check if this is a quality gate artifact for THIS checkpoint
                            if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                                // Match quality-gate stages
                                if stage.starts_with("quality-gate-") {
                                    if let Some(agent) = json_val.get("agent").and_then(|v| v.as_str()) {
                                        // Match against expected_agents
                                        if expected_agents.iter().any(|a| a.to_lowercase() == agent.to_lowercase()) {
                                            info_lines.push(format!(
                                                "Found {} from {}",
                                                agent,
                                                result_path.display()
                                            ));

                                            results_map.insert(
                                                agent.to_lowercase(),
                                                QualityGateAgentPayload {
                                                    agent: agent.to_string(),
                                                    gate: Some(stage.to_string()),
                                                    content: json_val,
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Err(err) => {
            return QualityGateBrokerResult {
                spec_id: spec_id.to_string(),
                checkpoint,
                attempts: 1,
                info_lines: vec![format!("Failed to read agents directory: {}", err)],
                missing_agents: expected_agents.to_vec(),
                found_agents: vec![],
                payload: Err(format!("Failed to read agents directory: {}", err)),
            };
        }
    }

    // Build result from collected artifacts
    let payloads: Vec<_> = results_map.values().cloned().collect();
    let found_agents: Vec<_> = payloads.iter().map(|p| p.agent.clone()).collect();
    let missing_agents: Vec<_> = expected_agents
        .iter()
        .filter(|a| !found_agents.iter().any(|f| f.to_lowercase() == a.to_lowercase()))
        .cloned()
        .collect();

    info_lines.push(format!(
        "Found {} of {} expected agents via filesystem scan",
        found_agents.len(),
        expected_agents.len()
    ));

    let min_required = match expected_agents.len() {
        0 => 0,
        1 => 1,
        _ => MIN_PARTICIPATING_AGENTS,
    };

    let payload = if payloads.len() >= min_required {
        Ok(payloads)
    } else {
        Err(format!(
            "Only found {}/{} agents",
            payloads.len(),
            expected_agents.len()
        ))
    };

    QualityGateBrokerResult {
        spec_id: spec_id.to_string(),
        checkpoint,
        attempts: 1,
        info_lines,
        missing_agents,
        found_agents,
        payload,
    }
}

async fn fetch_validation_payload(
    mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    spec_id: &str,
    checkpoint: QualityCheckpoint,
) -> QualityGateValidationResult {
    let mut attempts: u32 = 0;
    let mut info_lines = Vec::new();
    let mut validation_json: Option<serde_json::Value> = None;
    let mut last_error: Option<String> = None;

    for (idx, delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
        attempts = idx as u32 + 1;

        let manager = {
            let guard = mcp_manager.lock().await;
            guard.as_ref().cloned()
        };

        if let Some(manager) = manager {
            let args = json!({
                "query": format!("{} gpt5-validation", spec_id),
                "limit": 10,
                "tags": [
                    "quality-gate",
                    format!("spec:{}", spec_id),
                    format!("checkpoint:{}", checkpoint.name()),
                    "stage:gpt5-validation",
                ],
                "search_type": "hybrid"
            });

            match manager
                .call_tool(
                    "local-memory",
                    "search",
                    Some(args),
                    Some(Duration::from_secs(10)),
                )
                .await
            {
                Ok(call_result) => {
                    match crate::spec_prompts::parse_mcp_results_to_local_memory(&call_result) {
                        Ok(results) => {
                            if let Some(first) = results.first() {
                                match serde_json::from_str::<serde_json::Value>(
                                    &first.memory.content,
                                ) {
                                    Ok(value) => {
                                        validation_json = Some(value);
                                        info_lines.push("Validation artefact located".to_string());
                                        break;
                                    }
                                    Err(err) => {
                                        last_error = Some(format!(
                                            "failed to decode GPT-5 validation JSON: {}",
                                            err
                                        ));
                                    }
                                }
                            } else {
                                last_error =
                                    Some("No GPT-5 validation artefacts found".to_string());
                            }
                        }
                        Err(err) => {
                            last_error = Some(format!(
                                "failed to parse local-memory search results: {}",
                                err
                            ));
                        }
                    }
                }
                Err(err) => {
                    last_error = Some(format!("local-memory search failed: {err}"));
                }
            }
        } else {
            last_error = Some("MCP manager not available".to_string());
        }

        if validation_json.is_some() {
            break;
        }

        info_lines.push(format!("Validation retry {} scheduled", attempts + 1));
        if idx < RETRY_DELAYS_MS.len() - 1 {
            sleep(Duration::from_millis(*delay_ms)).await;
        }
    }

    let payload = validation_json.ok_or_else(|| {
        last_error.unwrap_or_else(|| {
            format!(
                "GPT-5 validation artefact not available for checkpoint {}",
                checkpoint.name()
            )
        })
    });

    QualityGateValidationResult {
        spec_id: spec_id.to_string(),
        checkpoint,
        attempts,
        info_lines,
        payload,
    }
}

/// Strip agent metadata prefixes (timestamps, version info, config lines)
/// SPEC-KIT-900 Session 2: Fix for "code" agent wrapping JSON in metadata
fn strip_agent_metadata(content: &str) -> String {
    content
        .lines()
        .skip_while(|line| {
            let trimmed = line.trim();
            // Skip timestamp lines: [2025-11-02T21:09:09]
            let is_timestamp = trimmed.starts_with('[') &&
                               trimmed.contains(']') &&
                               trimmed.len() < 30;

            // Skip version/product lines: OpenAI Codex v0.0.0, Anthropic Claude...
            let is_version = trimmed.starts_with("OpenAI") ||
                            trimmed.starts_with("Codex") ||
                            trimmed.starts_with("Anthropic") ||
                            trimmed.starts_with("Claude");

            // Skip separator lines: --------
            let is_separator = trimmed.starts_with("---") || trimmed.starts_with("===");

            // Skip config lines: workdir:, model:, provider:, sandbox:, etc.
            let is_config = trimmed.contains(':') &&
                           trimmed.len() < 100 &&
                           (trimmed.starts_with("workdir") ||
                            trimmed.starts_with("model") ||
                            trimmed.starts_with("provider") ||
                            trimmed.starts_with("sandbox") ||
                            trimmed.starts_with("approval") ||
                            trimmed.starts_with("reasoning"));

            is_timestamp || is_version || is_separator || is_config
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract JSON content from agent result text.
///
/// Tries multiple strategies:
/// 0. Strip metadata prefixes (timestamps, version info)
/// 1. Markdown code fence (```json ... ```)
/// 2. Raw JSON (starts with { or [)
fn extract_json_from_content(content: &str) -> Option<String> {
    // SPEC-KIT-900 Session 2 fix: Pre-process to remove agent metadata
    let cleaned_content = strip_agent_metadata(content);
    let content_to_parse = if cleaned_content.trim().is_empty() {
        content  // Fallback to original if stripping removed everything
    } else {
        &cleaned_content
    };

    tracing::warn!("🔍 Quality gate JSON extraction: original={} bytes, cleaned={} bytes",
        content.len(), content_to_parse.len());
    // Try markdown fence first
    let lines: Vec<&str> = content_to_parse.lines().collect();
    let mut in_fence = false;
    let mut json_lines = Vec::new();

    for line in &lines {
        let trimmed = line.trim();
        if trimmed == "```json" || trimmed == "``` json" {
            in_fence = true;
            continue;
        }
        if trimmed == "```" && in_fence {
            break;
        }
        if in_fence {
            json_lines.push(*line);
        }
    }

    if !json_lines.is_empty() {
        return Some(json_lines.join("\n"));
    }

    // Try raw JSON - look for actual JSON response, not prompt text
    // Strategy: Find all potential JSON blocks and validate them
    let mut json_candidates = Vec::new();
    let mut current_json = Vec::new();
    let mut in_json = false;
    let mut brace_depth: i32 = 0;

    for line in content_to_parse.lines() {
        let trimmed = line.trim();

        // Skip obvious prompt/instruction lines
        if trimmed.contains("Output JSON:") || trimmed.contains("User instructions:") {
            continue;
        }

        // Track JSON blocks by brace depth
        for ch in trimmed.chars() {
            if ch == '{' {
                if brace_depth == 0 {
                    in_json = true;
                    current_json.clear();
                }
                brace_depth += 1;
            }
            if in_json {
                // Collect full line when in JSON
                if !current_json.contains(&line.to_string()) {
                    current_json.push(line.to_string());
                }
            }
            if ch == '}' {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0 && in_json {
                    in_json = false;
                    // Complete JSON block found
                    let candidate = current_json.join("\n");
                    json_candidates.push(candidate);
                    current_json.clear();
                }
            }
        }
    }

    // Try each candidate, return first valid one with "stage" field that starts with "quality-gate-"
    // Skip template/example JSON that contains TypeScript type annotations
    for candidate in &json_candidates {
        // Quick check: skip if contains type annotation patterns (template JSON, not real response)
        let has_type_annotations = candidate.contains(r#""id": string"#) ||
                                   candidate.contains(r#""text": string"#) ||
                                   candidate.contains(r#": number"#) ||
                                   candidate.contains(r#": boolean"#) ||
                                   candidate.contains("${MODEL_ID}");

        if has_type_annotations {
            continue; // Skip template JSON from prompt
        }

        if let Ok(json_val) = serde_json::from_str::<Value>(candidate) {
            // Must have "stage" field starting with "quality-gate-" (actual response, not prompt example)
            if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                if stage.starts_with("quality-gate-") {
                    return Some(candidate.clone());
                }
            }
        }
    }

    // If no candidate found, try finding JSON by searching for quality-gate stage marker
    if let Some(stage_pos) = content.find(r#""stage": "quality-gate-"#) {
        // Found the stage field! Now find the enclosing JSON block
        // Search backwards for opening brace
        let before_stage = &content[..stage_pos];
        if let Some(open_brace) = before_stage.rfind('{') {
            // Search forwards from open brace for matching close
            let from_open = &content[open_brace..];
            let mut depth = 0;
            let mut json_end = 0;

            for (pos, ch) in from_open.chars().enumerate() {
                if ch == '{' { depth += 1; }
                if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        json_end = pos + 1;
                        break;
                    }
                }
            }

            if json_end > 0 {
                let candidate = &from_open[..json_end];
                if serde_json::from_str::<Value>(candidate).is_ok() {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    // Last resort: try extraction after [codex] marker
    if let Some(codex_section) = content.find("[codex]") {
        let after_codex = &content[codex_section..];
        if let Some(extracted) = extract_json_from_section(after_codex) {
            if let Ok(json_val) = serde_json::from_str::<Value>(&extracted) {
                if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                    if stage.starts_with("quality-gate-") {
                        return Some(extracted);
                    }
                }
            }
        }
    }

    // Fallback: try simple approach from beginning
    let mut found_json_start = false;
    let mut raw_json_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !found_json_start && trimmed.starts_with('{') {
            // Skip if this looks like prompt example
            if trimmed.contains("${") || line.contains("Output JSON:") {
                continue;
            }
            found_json_start = true;
            raw_json_lines.push(line);
            continue;
        }
        if found_json_start {
            if trimmed.starts_with('[') && trimmed.contains("tokens used") {
                break;
            }
            raw_json_lines.push(line);
        }
    }

    if !raw_json_lines.is_empty() {
        let json_str = raw_json_lines.join("\n");
        if serde_json::from_str::<Value>(&json_str).is_ok() {
            return Some(json_str);
        }
    }

    None
}

/// Extract JSON from a section of text (helper for nested extraction)
fn extract_json_from_section(content: &str) -> Option<String> {
    // Look for JSON blocks with "stage": "quality-gate-*" specifically
    let mut best_candidate: Option<String> = None;

    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('{') {
            // Try to parse from this point to end
            let remaining = content.lines().skip(idx).collect::<Vec<_>>().join("\n");

            // Find matching closing brace
            let mut depth = 0;
            let mut json_end = 0;
            for (pos, ch) in remaining.chars().enumerate() {
                if ch == '{' { depth += 1; }
                if ch == '}' {
                    depth -= 1;
                    if depth == 0 {
                        json_end = pos + 1;
                        break;
                    }
                }
            }

            if json_end > 0 {
                let candidate = &remaining[..json_end];
                if let Ok(json_val) = serde_json::from_str::<Value>(candidate) {
                    // Check if this is a quality-gate JSON
                    if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                        if stage.starts_with("quality-gate-") {
                            // Found quality gate JSON! Return immediately
                            return Some(candidate.to_string());
                        }
                    }
                    // Keep last valid JSON as fallback
                    best_candidate = Some(candidate.to_string());
                }
            }
        }
    }

    best_candidate
}
