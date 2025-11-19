//! Asynchronous broker for fetching quality gate artefacts.
//!
//! SPEC-KIT-068 restores the quality gate workflow by moving all local-memory
//! lookups off the Ratatui UI thread. The broker accepts lightweight commands
//! and performs the MCP calls inside Tokio tasks, emitting [`AppEvent`]s when
//! results are available (or when retries are exhausted).

use std::collections::HashMap;
use std::sync::Arc;

use codex_core::mcp_connection_manager::McpConnectionManager;
use serde_json::{Value, json};
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

/// Result metadata for GPT-5.1 validation artefacts.
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
        threshold: f64,
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
                        threshold,
                    } => {
                        let result = fetch_agent_payloads_from_memory(
                            &spec_id,
                            checkpoint,
                            &expected_agents,
                            &agent_ids,
                            threshold,
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
        threshold: f64,
    ) {
        if let Err(err) = self
            .sender
            .send(QualityGateCommand::FetchAgentPayloadsFromMemory {
                spec_id: spec_id.into(),
                checkpoint,
                expected_agents,
                agent_ids,
                threshold,
            })
        {
            tracing::error!("quality gate broker channel closed: {err}");
        }
    }

    /// Request asynchronous retrieval of GPT-5.1 validation artefacts.
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
    threshold: f64,
) -> QualityGateBrokerResult {
    let mut info_lines = Vec::new();
    let mut results_map: HashMap<String, QualityGateAgentPayload> = HashMap::new();

    info_lines.push(format!(
        "Collecting from AGENT_MANAGER: {} agents",
        agent_ids.len()
    ));

    // Read from AGENT_MANAGER memory (native orchestrator path)
    let manager = codex_core::agent_tool::AGENT_MANAGER.read().await;

    for agent_id in agent_ids {
        if let Some(agent) = manager.get_agent(agent_id) {
            info_lines.push(format!(
                "Agent {} ({}): {:?}",
                agent_id, agent.model, agent.status
            ));

            if let Some(result_text) = &agent.result {
                info_lines.push(format!("  Result length: {} chars", result_text.len()));

                // SPEC-KIT-927: Use robust JSON extraction with cascade strategies
                match super::json_extractor::extract_and_validate_quality_gate(
                    result_text,
                    &format!("agent-{}", agent_id),
                ) {
                    Ok(extraction_result) => {
                        let json_val = extraction_result.json;
                        info_lines.push(format!(
                            "  Extracted via {:?} (confidence: {:.2})",
                            extraction_result.method, extraction_result.confidence
                        ));

                        // Add any warnings to info_lines
                        for warning in extraction_result.warnings {
                            info_lines.push(format!("  ⚠️ {}", warning));
                        }

                        let stage = json_val.get("stage").and_then(|v| v.as_str());
                        let agent_name = json_val.get("agent").and_then(|v| v.as_str());

                        // Check if this is a quality gate artifact (already validated by extractor)
                        if let Some(stage) = stage {
                            if stage.starts_with("quality-gate-") {
                                if let Some(agent_name) = agent_name {
                                    // Match against expected_agents (flexible: exact match or starts-with)
                                    let matched_expected =
                                        expected_agents.iter().find(|expected| {
                                            let expected_lower = expected.to_lowercase();
                                            let agent_lower = agent_name.to_lowercase();
                                            // Exact match OR agent starts with expected (e.g., "claude-haiku-4-5" matches "claude")
                                            agent_lower == expected_lower
                                                || agent_lower
                                                    .starts_with(&format!("{}-", expected_lower))
                                        });

                                    if let Some(expected) = matched_expected {
                                        info_lines.push(format!(
                                            "Found {} (as '{}') from memory",
                                            expected, agent_name
                                        ));

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
                                        info_lines.push(format!(
                                            "  Agent '{}' not in expected list",
                                            agent_name
                                        ));
                                    }
                                } else {
                                    info_lines.push("  Missing 'agent' field in JSON".to_string());
                                }
                            } else {
                                info_lines.push(format!(
                                    "  Stage '{}' doesn't start with 'quality-gate-'",
                                    stage
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        info_lines.push(format!("  Extraction failed: {}", e));
                        info_lines.push(format!(
                            "  First 500 chars: {}",
                            &result_text.chars().take(500).collect::<String>()
                        ));

                        // SPEC-KIT-927: Store raw output even on extraction failure
                        if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                            let error_msg = format!("{}", e);
                            if let Err(db_err) =
                                db.record_extraction_failure(agent_id, result_text, &error_msg)
                            {
                                tracing::warn!(
                                    "Failed to record extraction failure for {}: {}",
                                    agent_id,
                                    db_err
                                );
                            } else {
                                info_lines.push(format!(
                                    "  Stored raw output to DB (extraction_error column)"
                                ));
                            }
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

    info_lines.push(format!(
        "Found {}/{} agents via memory",
        found_agents.len(),
        expected_agents.len()
    ));

    // SPEC-939: Calculate minimum required agents based on consensus threshold
    // Threshold is now passed from widget.config.quality_gates[checkpoint].threshold
    let min_required = if expected_agents.len() >= 2 {
        // Calculate minimum as ceil(count * threshold)
        let min = (expected_agents.len() as f64 * threshold).ceil() as usize;
        min.max(1).min(expected_agents.len())
    } else {
        // For 0-1 agents, require all
        expected_agents.len()
    };
    let is_valid = results_map.len() >= min_required;

    if !is_valid {
        info_lines.push(format!(
            "⚠️ Insufficient: {}/{} (need {})",
            results_map.len(),
            expected_agents.len(),
            min_required
        ));
    } else if results_map.len() < expected_agents.len() {
        info_lines.push(format!(
            "✓ Degraded: {}/{} agents (acceptable)",
            results_map.len(),
            expected_agents.len()
        ));
    }

    QualityGateBrokerResult {
        spec_id: spec_id.to_string(),
        checkpoint,
        attempts: 1,
        info_lines,
        missing_agents,
        found_agents,
        payload: if is_valid {
            Ok(results_map.values().cloned().collect())
        } else {
            Err(format!(
                "Only found {}/{} agents (need {})",
                results_map.len(),
                expected_agents.len(),
                min_required
            ))
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
                    // SPEC-KIT-927: Use robust JSON extraction
                    match super::json_extractor::extract_and_validate_quality_gate(
                        &content,
                        "filesystem-agent",
                    ) {
                        Ok(extraction_result) => {
                            let json_val = extraction_result.json;

                            if let Some(stage) = json_val.get("stage").and_then(|v| v.as_str()) {
                                if stage.starts_with("quality-gate-") {
                                    if let Some(agent) =
                                        json_val.get("agent").and_then(|v| v.as_str())
                                    {
                                        // Match against expected_agents
                                        if expected_agents
                                            .iter()
                                            .any(|a| a.to_lowercase() == agent.to_lowercase())
                                        {
                                            info_lines.push(format!(
                                                "Found {} from {} via {:?}",
                                                agent,
                                                result_path.display(),
                                                extraction_result.method
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
                        Err(e) => {
                            // SPEC-KIT-927: Store raw output on extraction failure
                            tracing::debug!(
                                "Extraction failed for {}: {}",
                                result_path.display(),
                                e
                            );

                            // Try to get agent_id from directory name
                            if let Some(agent_id) = result_path
                                .parent()
                                .and_then(|p| p.file_name())
                                .and_then(|n| n.to_str())
                            {
                                if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
                                    let error_msg = format!("{}", e);
                                    let _ = db
                                        .record_extraction_failure(agent_id, &content, &error_msg);
                                    tracing::debug!("Stored raw output to DB for {}", agent_id);
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
        .filter(|a| {
            !found_agents
                .iter()
                .any(|f| f.to_lowercase() == a.to_lowercase())
        })
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
                                            "failed to decode GPT-5.1 validation JSON: {}",
                                            err
                                        ));
                                    }
                                }
                            } else {
                                last_error =
                                    Some("No GPT-5.1 validation artefacts found".to_string());
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
                "GPT-5.1 validation artefact not available for checkpoint {}",
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

// SPEC-KIT-927: Old extraction functions removed - replaced by json_extractor.rs module
// - strip_agent_metadata() - handled by json_extractor cascade
// - extract_json_from_content() - replaced by extract_json_robust()
// - extract_json_from_section() - replaced by schema marker search strategy
