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
                                            // Match against expected_agents
                                            if expected_agents.iter().any(|a| a.to_lowercase() == agent_name.to_lowercase()) {
                                                info_lines.push(format!("Found {} from memory", agent_name));

                                                results_map.insert(
                                                    agent_name.to_lowercase(),
                                                    QualityGateAgentPayload {
                                                        agent: agent_name.to_string(),
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
                                info_lines.push(format!("  First 200 chars: {}", &json_str.chars().take(200).collect::<String>()));
                            }
                        }
                    }
                    None => {
                        info_lines.push("  No JSON found in result".to_string());
                        info_lines.push(format!("  First 500 chars: {}", &result_text.chars().take(500).collect::<String>()));
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

/// Extract JSON content from agent result text.
///
/// Tries two strategies:
/// 1. Markdown code fence (```json ... ```)
/// 2. Raw JSON (starts with { or [)
fn extract_json_from_content(content: &str) -> Option<String> {
    // Try markdown fence first
    let lines: Vec<&str> = content.lines().collect();
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

    for line in content.lines() {
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
    for candidate in &json_candidates {
        if let Ok(json_val) = serde_json::from_str::<Value>(candidate) {
            // Must have "stage" field starting with "quality-gate-" (actual response, not prompt example)
            if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                if stage.starts_with("quality-gate-") {
                    return Some(candidate.clone());
                }
            }
        }
    }

    // If no candidate found with quality-gate stage, try searching for specific pattern
    // The code agent might output JSON after [codex] or similar markers
    if let Some(codex_section) = content.find("[codex]") {
        let after_codex = &content[codex_section..];
        // Recursively try extraction on the section after [codex]
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
    // Look for the LAST occurrence of a JSON block (most likely to be the final output)
    let mut last_valid_json: Option<String> = None;

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
                if serde_json::from_str::<Value>(candidate).is_ok() {
                    last_valid_json = Some(candidate.to_string());
                }
            }
        }
    }

    last_valid_json
}
