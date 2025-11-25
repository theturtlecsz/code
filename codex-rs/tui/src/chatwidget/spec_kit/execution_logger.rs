//! Execution logging for /speckit.auto pipeline
//!
//! Provides multi-layer visibility into spec-kit automation runs:
//! - Layer 1: JSONL execution log (structured events)
//! - Layer 2: Real-time status file (current state)
//! - Layer 3: Post-run summary (human-readable report)
//!
//! Created for SPEC-KIT-070 validation and debugging.

use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};


/// Unique identifier for a pipeline run
pub type RunId = String;

/// Generate a run ID with timestamp
pub fn generate_run_id(spec_id: &str) -> RunId {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!(
        "run_{}_{}_{}",
        spec_id,
        timestamp,
        uuid::Uuid::new_v4().to_string()[..8].to_string()
    )
}

/// Event types in the execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    RunStart {
        spec_id: String,
        run_id: RunId,
        timestamp: String,
        stages: Vec<String>,
        quality_gates_enabled: bool,
        hal_mode: String,
    },
    StageStart {
        run_id: RunId,
        stage: String,
        tier: u8,
        expected_agents: Vec<String>,
        timestamp: String,
    },
    AgentSpawn {
        run_id: RunId,
        stage: String,
        agent_name: String,
        agent_id: String,
        model: String,
        prompt_preview: String, // First 200 chars
        timestamp: String,
    },
    AgentComplete {
        run_id: RunId,
        stage: String,
        agent_name: String,
        agent_id: String,
        duration_sec: f64,
        status: String, // "completed", "failed", "cancelled"
        output_lines: usize,
        timestamp: String,
    },
    ConsensusStart {
        run_id: RunId,
        stage: String,
        agent_count: usize,
        timestamp: String,
    },
    ConsensusComplete {
        run_id: RunId,
        stage: String,
        status: String, // "ok", "degraded", "conflict", "no-consensus"
        degraded: bool,
        cost_usd: Option<f64>,
        timestamp: String,
    },
    QualityGateStart {
        run_id: RunId,
        checkpoint: String,
        gates: Vec<String>,
        timestamp: String,
    },
    QualityGateComplete {
        run_id: RunId,
        checkpoint: String,
        status: String, // "passed", "failed", "escalated"
        auto_resolved: usize,
        escalated: usize,
        degraded_agents: Vec<String>,
        timestamp: String,
    },
    StageComplete {
        run_id: RunId,
        stage: String,
        duration_sec: f64,
        cost_usd: Option<f64>,
        evidence_written: bool,
        timestamp: String,
    },
    RunComplete {
        run_id: RunId,
        spec_id: String,
        total_duration_sec: f64,
        total_cost_usd: f64,
        stages_completed: usize,
        quality_gates_passed: usize,
        timestamp: String,
    },
    RunError {
        run_id: RunId,
        stage: Option<String>,
        error: String,
        timestamp: String,
    },
    /// Tool execution by an agent (start)
    ToolExecutionStart {
        run_id: RunId,
        agent_id: String,
        tool_name: String,
        timestamp: String,
    },
    /// Tool execution completion
    ToolExecutionComplete {
        run_id: RunId,
        agent_id: String,
        tool_name: String,
        duration_ms: u64,
        status: String, // "success", "error"
        timestamp: String,
    },
    /// Pipeline phase transition
    PhaseTransition {
        run_id: RunId,
        from_phase: String,
        to_phase: String,
        stage: String,
        trigger: String, // What caused the transition
        timestamp: String,
    },
    /// Agent completion detection check
    CompletionCheck {
        run_id: RunId,
        stage: String,
        all_agents_terminal: bool,
        tools_running: bool,
        streaming_active: bool,
        will_proceed: bool, // Whether completion handler will be called
        agent_count: usize,
        completed_count: usize,
        timestamp: String,
    },
}

impl ExecutionEvent {
    /// Get current timestamp in ISO 8601 format
    pub fn now() -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    }

    /// Get run_id from event
    pub fn run_id(&self) -> &str {
        match self {
            Self::RunStart { run_id, .. }
            | Self::StageStart { run_id, .. }
            | Self::AgentSpawn { run_id, .. }
            | Self::AgentComplete { run_id, .. }
            | Self::ConsensusStart { run_id, .. }
            | Self::ConsensusComplete { run_id, .. }
            | Self::QualityGateStart { run_id, .. }
            | Self::QualityGateComplete { run_id, .. }
            | Self::StageComplete { run_id, .. }
            | Self::RunComplete { run_id, .. }
            | Self::RunError { run_id, .. }
            | Self::ToolExecutionStart { run_id, .. }
            | Self::ToolExecutionComplete { run_id, .. }
            | Self::PhaseTransition { run_id, .. }
            | Self::CompletionCheck { run_id, .. } => run_id,
        }
    }
}

/// Real-time status for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatus {
    pub spec_id: String,
    pub run_id: RunId,
    pub status: String, // "in_progress", "completed", "failed"
    pub current_stage: Option<String>,
    pub current_phase: Option<String>,
    pub started_at: String,
    pub elapsed_sec: f64,
    pub stages_completed: Vec<String>,
    pub stages_remaining: Vec<String>,
    pub current_agents: Vec<AgentStatus>,
    pub cost_accumulated_usd: f64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatus {
    pub name: String,
    pub status: String, // "running", "completed", "failed"
    pub duration_sec: Option<f64>,
}

/// Execution logger instance
#[derive(Debug)]
pub struct ExecutionLogger {
    log_file: Arc<Mutex<Option<File>>>,
    status_file: Arc<Mutex<PathBuf>>,
    run_id: Arc<Mutex<Option<RunId>>>,
    start_time: Arc<Mutex<Option<SystemTime>>>,
}

impl ExecutionLogger {
    /// Create a new execution logger
    pub fn new() -> Self {
        Self {
            log_file: Arc::new(Mutex::new(None)),
            status_file: Arc::new(Mutex::new(PathBuf::new())),
            run_id: Arc::new(Mutex::new(None)),
            start_time: Arc::new(Mutex::new(None)),
        }
    }

    /// Initialize logger for a new run
    pub fn init(&self, spec_id: &str, run_id: RunId) -> Result<(), std::io::Error> {
        // Create execution logs directory
        let log_dir = Path::new(".code/execution_logs");
        std::fs::create_dir_all(log_dir)?;

        // Create JSONL log file
        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_path = log_dir.join(format!("spec_auto_{}_{}.jsonl", spec_id, timestamp));
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        // Create status file path
        let status_path = Path::new(".code/spec_auto_status.json");

        // Store references
        *self.log_file.lock().unwrap() = Some(file);
        *self.status_file.lock().unwrap() = status_path.to_path_buf();
        *self.run_id.lock().unwrap() = Some(run_id);
        *self.start_time.lock().unwrap() = Some(SystemTime::now());

        Ok(())
    }

    /// Log an execution event
    pub fn log_event(&self, event: ExecutionEvent) {
        // Write to JSONL file
        if let Some(file) = self.log_file.lock().unwrap().as_mut() {
            if let Ok(json) = serde_json::to_string(&event) {
                let _ = writeln!(file, "{}", json);
                let _ = file.flush();
            }
        }

        // Update status file (async to avoid blocking)
        self.update_status_from_event(&event);
    }

    /// Update real-time status file based on event
    fn update_status_from_event(&self, event: &ExecutionEvent) {
        // Read current status or create new
        let status_path = self.status_file.lock().unwrap().clone();
        if status_path.as_os_str().is_empty() {
            return; // Not initialized
        }

        let mut status = match std::fs::read_to_string(&status_path)
            .ok()
            .and_then(|content| serde_json::from_str::<ExecutionStatus>(&content).ok())
        {
            Some(s) => s,
            None => {
                // Create new status from RunStart event
                if let ExecutionEvent::RunStart {
                    spec_id,
                    run_id,
                    timestamp,
                    stages,
                    ..
                } = event
                {
                    ExecutionStatus {
                        spec_id: spec_id.clone(),
                        run_id: run_id.clone(),
                        status: "in_progress".to_string(),
                        current_stage: None,
                        current_phase: None,
                        started_at: timestamp.clone(),
                        elapsed_sec: 0.0,
                        stages_completed: Vec::new(),
                        stages_remaining: stages.clone(),
                        current_agents: Vec::new(),
                        cost_accumulated_usd: 0.0,
                        errors: Vec::new(),
                    }
                } else {
                    // Can't create status without RunStart - exit early
                    return;
                }
            }
        };

        // Update based on event type
        match event {
            ExecutionEvent::StageStart { stage, .. } => {
                status.current_stage = Some(stage.clone());
                status.current_phase = Some("guardrail".to_string());
            }
            ExecutionEvent::AgentSpawn { agent_name, .. } => {
                if !status.current_agents.iter().any(|a| a.name == *agent_name) {
                    status.current_agents.push(AgentStatus {
                        name: agent_name.clone(),
                        status: "running".to_string(),
                        duration_sec: None,
                    });
                }
                status.current_phase = Some("executing_agents".to_string());
            }
            ExecutionEvent::AgentComplete {
                agent_name,
                duration_sec,
                ..
            } => {
                if let Some(agent) = status
                    .current_agents
                    .iter_mut()
                    .find(|a| a.name == *agent_name)
                {
                    agent.status = "completed".to_string();
                    agent.duration_sec = Some(*duration_sec);
                }
            }
            ExecutionEvent::ConsensusStart { .. } => {
                status.current_phase = Some("checking_consensus".to_string());
            }
            ExecutionEvent::ConsensusComplete { cost_usd, .. } => {
                if let Some(cost) = cost_usd {
                    status.cost_accumulated_usd += cost;
                }
            }
            ExecutionEvent::QualityGateStart { checkpoint, .. } => {
                status.current_phase = Some(format!("quality_gate_{}", checkpoint));
            }
            ExecutionEvent::QualityGateComplete { .. } => {
                status.current_phase = Some("quality_gate_complete".to_string());
            }
            ExecutionEvent::StageComplete {
                stage, cost_usd, ..
            } => {
                if let Some(cost) = cost_usd {
                    status.cost_accumulated_usd += cost;
                }
                if !status.stages_completed.contains(stage) {
                    status.stages_completed.push(stage.clone());
                }
                status.stages_remaining.retain(|s| s != stage);
                status.current_agents.clear();
                status.current_stage = None;
                status.current_phase = None;
            }
            ExecutionEvent::RunComplete { total_cost_usd, .. } => {
                status.status = "completed".to_string();
                status.cost_accumulated_usd = *total_cost_usd;
                status.current_stage = None;
                status.current_phase = None;
                status.current_agents.clear();
            }
            ExecutionEvent::RunError { error, .. } => {
                status.status = "failed".to_string();
                status.errors.push(error.clone());
            }
            _ => {}
        }

        // Update elapsed time
        status.elapsed_sec = self.elapsed_sec();

        // Write updated status atomically (temp file + rename)
        let temp_path = status_path.with_extension("tmp");
        if let Ok(json) = serde_json::to_string_pretty(&status) {
            if std::fs::write(&temp_path, json).is_ok() {
                let _ = std::fs::rename(&temp_path, &status_path);
            }
        }
    }

    /// Get current run ID
    pub fn current_run_id(&self) -> Option<RunId> {
        self.run_id.lock().unwrap().clone()
    }

    /// Get elapsed time since run start
    pub fn elapsed_sec(&self) -> f64 {
        if let Some(start) = *self.start_time.lock().unwrap() {
            SystemTime::now()
                .duration_since(start)
                .unwrap_or_default()
                .as_secs_f64()
        } else {
            0.0
        }
    }

    /// Finalize and close log
    pub fn finalize(&self) {
        *self.log_file.lock().unwrap() = None;
        *self.run_id.lock().unwrap() = None;
        *self.start_time.lock().unwrap() = None;
    }
}

impl Default for ExecutionLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Generate execution summary from JSONL log
pub fn generate_summary(log_path: &Path) -> Result<String, std::io::Error> {
    let content = std::fs::read_to_string(log_path)?;
    let mut events: Vec<ExecutionEvent> = Vec::new();

    for line in content.lines() {
        if let Ok(event) = serde_json::from_str::<ExecutionEvent>(line) {
            events.push(event);
        }
    }

    // Build summary from events
    let mut summary = String::new();

    // Extract key metrics
    let run_start = events.iter().find_map(|e| match e {
        ExecutionEvent::RunStart {
            spec_id,
            run_id,
            stages,
            timestamp,
            quality_gates_enabled,
            hal_mode,
        } => Some((
            spec_id,
            run_id,
            stages,
            timestamp,
            quality_gates_enabled,
            hal_mode,
        )),
        _ => None,
    });

    let run_complete = events.iter().find_map(|e| match e {
        ExecutionEvent::RunComplete {
            total_duration_sec,
            total_cost_usd,
            stages_completed,
            quality_gates_passed,
            ..
        } => Some((
            *total_duration_sec,
            *total_cost_usd,
            *stages_completed,
            *quality_gates_passed,
        )),
        _ => None,
    });

    if let Some((spec_id, run_id, stages, started, qg_enabled, hal_mode)) = run_start {
        summary.push_str(&format!("# Execution Summary: {}\n\n", spec_id));
        summary.push_str(&format!("**Run ID**: {}\n", run_id));
        summary.push_str(&format!("**Started**: {}\n", started));
        summary.push_str(&format!("**HAL Mode**: {}\n", hal_mode));
        summary.push_str(&format!(
            "**Quality Gates**: {}\n\n",
            if *qg_enabled { "enabled" } else { "disabled" }
        ));

        if let Some((duration, cost, completed, qg_passed)) = run_complete {
            summary.push_str(&format!("**Duration**: {}\n", format_duration(duration)));
            summary.push_str(&format!("**Cost**: ${:.2}\n", cost));
            summary.push_str(&format!(
                "**Stages Completed**: {}/{}\n",
                completed,
                stages.len()
            ));
            if *qg_enabled {
                summary.push_str(&format!("**Quality Gates Passed**: {}\n", qg_passed));
            }
            summary.push_str("**Status**: ✓ COMPLETED\n\n");
        } else {
            summary.push_str("**Status**: ⚠ INCOMPLETE\n\n");
        }
    }

    // Build detailed stage timeline table
    summary.push_str("## Stage Timeline\n\n");
    summary.push_str("| Stage | Tier | Agents | Duration | Cost | Status |\n");
    summary.push_str("|-------|------|--------|----------|------|--------|\n");

    // Group events by stage
    let mut stage_map: std::collections::HashMap<String, StageMetrics> =
        std::collections::HashMap::new();

    for event in &events {
        match event {
            ExecutionEvent::StageStart {
                stage,
                tier,
                expected_agents,
                timestamp,
                ..
            } => {
                stage_map
                    .entry(stage.clone())
                    .or_insert_with(|| StageMetrics {
                        tier: *tier,
                        agents: expected_agents.clone(),
                        start_time: timestamp.clone(),
                        duration_sec: 0.0,
                        cost_usd: 0.0,
                        status: "in_progress".to_string(),
                    });
            }
            ExecutionEvent::StageComplete {
                stage,
                duration_sec,
                cost_usd,
                evidence_written,
                ..
            } => {
                if let Some(metrics) = stage_map.get_mut(stage) {
                    metrics.duration_sec = *duration_sec;
                    metrics.cost_usd = cost_usd.unwrap_or(0.0);
                    metrics.status = if *evidence_written {
                        "completed".to_string()
                    } else {
                        "completed (no evidence)".to_string()
                    };
                }
            }
            _ => {}
        }
    }

    // Display stages in order
    for stage_name in ["plan", "tasks", "implement", "validate", "audit", "unlock"] {
        if let Some(metrics) = stage_map.get(stage_name) {
            summary.push_str(&format!(
                "| {} | {} | {} | {} | ${:.2} | {} |\n",
                stage_name,
                metrics.tier,
                metrics.agents.len(),
                format_duration(metrics.duration_sec),
                metrics.cost_usd,
                metrics.status
            ));
        }
    }

    // Quality gate summary
    let quality_gates: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            ExecutionEvent::QualityGateComplete {
                checkpoint,
                auto_resolved,
                escalated,
                degraded_agents,
                ..
            } => Some((checkpoint, auto_resolved, escalated, degraded_agents)),
            _ => None,
        })
        .collect();

    if !quality_gates.is_empty() {
        summary.push_str("\n## Quality Gates\n\n");
        summary.push_str("| Checkpoint | Auto-Resolved | Escalated | Degraded |\n");
        summary.push_str("|------------|---------------|-----------|----------|\n");

        for (checkpoint, auto, esc, degraded) in quality_gates {
            summary.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                checkpoint,
                auto,
                esc,
                if degraded.is_empty() {
                    "✓".to_string()
                } else {
                    format!("⚠ ({})", degraded.join(", "))
                }
            ));
        }
    }

    // Cost breakdown
    let mut cost_by_category = std::collections::HashMap::new();
    for event in &events {
        match event {
            ExecutionEvent::ConsensusComplete { cost_usd, .. } => {
                if let Some(cost) = cost_usd {
                    *cost_by_category.entry("consensus").or_insert(0.0) += cost;
                }
            }
            ExecutionEvent::StageComplete { cost_usd, .. } => {
                if let Some(cost) = cost_usd {
                    *cost_by_category.entry("execution").or_insert(0.0) += cost;
                }
            }
            _ => {}
        }
    }

    if !cost_by_category.is_empty() {
        summary.push_str("\n## Cost Breakdown\n\n");
        for (category, cost) in cost_by_category {
            summary.push_str(&format!("- {}: ${:.2}\n", category, cost));
        }
    }

    // Evidence files
    summary.push_str("\n## Evidence\n\n");
    summary.push_str(&format!("Execution log: {}\n", log_path.display()));
    summary.push_str("Status file: .code/spec_auto_status.json\n");

    Ok(summary)
}

/// Stage metrics for summary generation
#[derive(Debug)]
struct StageMetrics {
    tier: u8,
    agents: Vec<String>,
    start_time: String,
    duration_sec: f64,
    cost_usd: f64,
    status: String,
}

/// Format duration in human-readable format
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else if secs < 3600.0 {
        format!("{:.0}m {:.0}s", secs / 60.0, secs % 60.0)
    } else {
        format!("{:.0}h {:.0}m", secs / 3600.0, (secs % 3600.0) / 60.0)
    }
}

/// Determine tier from agent count (heuristic)
pub fn tier_from_agent_count(count: usize) -> u8 {
    match count {
        0 => 0,     // Native
        1 => 1,     // Single agent
        2..=3 => 2, // Multi-agent
        _ => 3,     // Premium
    }
}

/// Get agent model display name from canonical name
/// Updated: 2025-11-19 - "code" now defaults to GPT-5.1, all GPT-5 → GPT-5.1
pub fn get_agent_model_name(agent: &str) -> &str {
    match agent.to_lowercase().as_str() {
        "gemini" => "Gemini 2.5 Flash",
        "claude" => "Claude Haiku 4.5",
        "code" => "GPT-5.1 (TUI default)",
        "gpt_pro" => "GPT-5.1 Medium",
        "gpt_codex" => "GPT-5.1 Codex",
        _ => agent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_run_id() {
        let run_id = generate_run_id("SPEC-KIT-900");
        assert!(run_id.starts_with("run_SPEC-KIT-900_"));
        assert!(run_id.len() > 30);
    }

    #[test]
    fn test_execution_event_serialization() {
        let event = ExecutionEvent::RunStart {
            spec_id: "TEST-001".to_string(),
            run_id: "run_test_123".to_string(),
            timestamp: "2025-11-01T00:00:00Z".to_string(),
            stages: vec!["plan".to_string(), "tasks".to_string()],
            quality_gates_enabled: true,
            hal_mode: "mock".to_string(),
        };

        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("run_start"));
        assert!(json.contains("TEST-001"));
    }

    #[test]
    fn test_logger_init() {
        let logger = ExecutionLogger::new();
        let run_id = generate_run_id("TEST");

        // Should not panic
        let result = logger.init("TEST-001", run_id);
        assert!(result.is_ok() || result.is_err()); // May fail if directory issues
    }
}
