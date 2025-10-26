//! Asynchronous broker for fetching quality gate artefacts.
//!
//! SPEC-KIT-068 restores the quality gate workflow by moving all local-memory
//! lookups off the Ratatui UI thread. The broker accepts lightweight commands
//! and performs the MCP calls inside Tokio tasks, emitting [`AppEvent`]s when
//! results are available (or when retries are exhausted).

use std::collections::HashMap;
use std::sync::Arc;

use codex_core::mcp_connection_manager::McpConnectionManager;
use serde_json::json;
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
    mcp_manager: Arc<Mutex<Option<Arc<McpConnectionManager>>>>,
    spec_id: &str,
    checkpoint: QualityCheckpoint,
    expected_agents: &[String],
    gate_stages: &[String],
) -> QualityGateBrokerResult {
    let mut attempts: u32 = 0;
    let mut info_lines = Vec::new();
    let mut results_map: HashMap<(String, Option<String>), QualityGateAgentPayload> =
        HashMap::new();
    let mut last_error: Option<String> = None;

    for (idx, delay_ms) in RETRY_DELAYS_MS.iter().enumerate() {
        attempts = idx as u32 + 1;

        let manager = {
            let guard = mcp_manager.lock().await;
            guard.as_ref().cloned()
        };

        if let Some(manager) = manager {
            for stage in gate_stages {
                let args = json!({
                    "query": format!("{} quality-gate {} {}", spec_id, checkpoint.name(), stage),
                    "limit": 24,
                    "tags": [
                        "quality-gate",
                        format!("spec:{}", spec_id),
                        format!("checkpoint:{}", checkpoint.name()),
                        format!("stage:{}", stage),
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
                            Ok(memory_results) => {
                                for item in memory_results {
                                    match serde_json::from_str::<serde_json::Value>(
                                        &item.memory.content,
                                    ) {
                                        Ok(json_value) => {
                                            let agent = json_value
                                                .get("agent")
                                                .and_then(|v| v.as_str())
                                                .unwrap_or("unknown")
                                                .to_string();

                                            let key = (agent.to_lowercase(), Some(stage.clone()));
                                            results_map.entry(key).or_insert(
                                                QualityGateAgentPayload {
                                                    agent,
                                                    gate: Some(stage.clone()),
                                                    content: json_value,
                                                },
                                            );
                                        }
                                        Err(err) => {
                                            last_error = Some(format!(
                                                "failed to decode agent JSON: {}",
                                                err
                                            ));
                                        }
                                    }
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
            }
        } else {
            last_error = Some("MCP manager not available".to_string());
        }

        let unique_agents: std::collections::HashSet<String> = results_map
            .values()
            .map(|payload| payload.agent.to_lowercase())
            .collect();
        info_lines.push(format!(
            "Attempt {}: collected {} of {} agent artefacts",
            attempts,
            unique_agents.len(),
            expected_agents.len()
        ));

        let min_required = match expected_agents.len() {
            0 => 0,
            1 => 1,
            _ => MIN_PARTICIPATING_AGENTS,
        };

        if unique_agents.len() >= expected_agents.len()
            || (unique_agents.len() >= min_required && idx == RETRY_DELAYS_MS.len() - 1)
        {
            break;
        }

        if idx < RETRY_DELAYS_MS.len() - 1 {
            sleep(Duration::from_millis(*delay_ms)).await;
        }
    }

    let payloads: Vec<QualityGateAgentPayload> = results_map.values().cloned().collect();
    let found_agents_set: std::collections::HashSet<String> = payloads
        .iter()
        .map(|payload| payload.agent.to_lowercase())
        .collect();
    let mut found_agents: Vec<String> = found_agents_set.iter().cloned().collect();
    found_agents.sort();
    let missing_agents: Vec<String> = expected_agents
        .iter()
        .filter(|agent| !found_agents_set.contains(&agent.to_lowercase()))
        .cloned()
        .collect();

    let min_required = match expected_agents.len() {
        0 => 0,
        1 => 1,
        _ => MIN_PARTICIPATING_AGENTS,
    };

    let payload = if payloads.is_empty() {
        Err(last_error.unwrap_or_else(|| {
            format!(
                "No quality gate artefacts found for checkpoint {}",
                checkpoint.name()
            )
        }))
    } else if found_agents_set.len() < min_required {
        Err(format!(
            "Only {} of {} expected agents produced artefacts",
            found_agents_set.len(),
            expected_agents.len()
        ))
    } else {
        Ok(payloads)
    };

    QualityGateBrokerResult {
        spec_id: spec_id.to_string(),
        checkpoint,
        attempts,
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
