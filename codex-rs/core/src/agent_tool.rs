use chrono::DateTime;
use chrono::Duration;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::config_types::AgentConfig;
use crate::openai_tools::JsonSchema;
use crate::openai_tools::OpenAiTool;
use crate::openai_tools::ResponsesApiTool;
use crate::protocol::AgentInfo;

// Agent status enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: String,
    pub batch_id: Option<String>,
    pub model: String,
    pub prompt: String,
    pub context: Option<String>,
    pub output_goal: Option<String>,
    pub files: Vec<String>,
    pub read_only: bool,
    pub status: AgentStatus,
    pub result: Option<String>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub progress: Vec<String>,
    pub worktree_path: Option<String>,
    pub branch_name: Option<String>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub config: Option<AgentConfig>,
    /// Enable tmux pane execution for observable agent runs (SPEC-KIT-923)
    #[serde(default)]
    pub tmux_enabled: bool,
}

// Global agent manager
lazy_static::lazy_static! {
    pub static ref AGENT_MANAGER: Arc<RwLock<AgentManager>> = Arc::new(RwLock::new(AgentManager::new()));
}

pub struct AgentManager {
    agents: HashMap<String, Agent>,
    handles: HashMap<String, JoinHandle<()>>,
    event_sender: Option<mpsc::UnboundedSender<AgentStatusUpdatePayload>>,
}

#[derive(Debug, Clone)]
pub struct AgentStatusUpdatePayload {
    pub agents: Vec<AgentInfo>,
    pub context: Option<String>,
    pub task: Option<String>,
}

impl AgentManager {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            handles: HashMap::new(),
            event_sender: None,
        }
    }

    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<AgentStatusUpdatePayload>) {
        self.event_sender = Some(sender);
    }

    async fn send_agent_status_update(&self) {
        if let Some(ref sender) = self.event_sender {
            let agents: Vec<AgentInfo> = self
                .agents
                .values()
                .map(|agent| {
                    // Just show the model name - status provides the useful info
                    let name = agent.model.clone();

                    AgentInfo {
                        id: agent.id.clone(),
                        name,
                        status: format!("{:?}", agent.status).to_lowercase(),
                        batch_id: agent.batch_id.clone(),
                        model: Some(agent.model.clone()),
                        last_progress: agent.progress.last().cloned(),
                        result: agent.result.clone(),
                        error: agent.error.clone(),
                    }
                })
                .collect();

            // Get context and task from the first agent (they're all the same)
            let (context, task) = self
                .agents
                .values()
                .next()
                .map(|agent| {
                    let context = agent.context.as_ref().and_then(|value| {
                        if value.trim().is_empty() {
                            None
                        } else {
                            Some(value.clone())
                        }
                    });
                    let task = if agent.prompt.trim().is_empty() {
                        None
                    } else {
                        Some(agent.prompt.clone())
                    };
                    (context, task)
                })
                .unwrap_or((None, None));
            let payload = AgentStatusUpdatePayload {
                agents,
                context,
                task,
            };
            let _ = sender.send(payload);
        }
    }

    pub async fn create_agent(
        &mut self,
        model: String,
        prompt: String,
        context: Option<String>,
        output_goal: Option<String>,
        files: Vec<String>,
        read_only: bool,
        batch_id: Option<String>,
    ) -> String {
        self.create_agent_internal(
            model,
            prompt,
            context,
            output_goal,
            files,
            read_only,
            batch_id,
            None,
            false, // tmux_enabled defaults to false
        )
        .await
    }

    /// Create agent using config name lookup (SPEC-KIT-900: Proper agent abstraction)
    ///
    /// This is the CORRECT way to spawn agents with specific model configurations.
    ///
    /// # Arguments
    /// * `config_name` - Name from [[agents]] config (e.g., "gemini_flash", "claude_haiku")
    /// * `agent_configs` - Vec of AgentConfig from config.toml
    /// * `prompt` - Task prompt
    /// * `read_only` - Whether agent runs read-only
    /// * `batch_id` - Optional batch identifier
    /// * `tmux_enabled` - Enable tmux pane execution for observability (SPEC-KIT-923)
    ///
    /// # Returns
    /// Agent ID if spawned successfully, error if config not found
    pub async fn create_agent_from_config_name(
        &mut self,
        config_name: &str,
        agent_configs: &[AgentConfig],
        prompt: String,
        read_only: bool,
        batch_id: Option<String>,
        tmux_enabled: bool,
    ) -> Result<String, String> {
        // Look up agent config by name
        let agent_config = agent_configs
            .iter()
            .find(|c| c.name == config_name)
            .ok_or_else(|| format!("Agent config '{}' not found in config.toml", config_name))?;

        if !agent_config.enabled {
            return Err(format!("Agent '{}' is disabled in config", config_name));
        }

        // Use the base command name as the "model" for execute_agent matching
        // The actual model/args come from config
        let base_command = agent_config.command.clone();

        let agent_id = self
            .create_agent_internal(
                base_command, // "gemini", "claude", "codex", etc. (for execute_agent matching)
                prompt,
                None,   // context
                None,   // output_goal
                vec![], // files
                read_only,
                batch_id,
                Some(agent_config.clone()),
                tmux_enabled,
            )
            .await;

        Ok(agent_id)
    }

    pub async fn create_agent_with_config(
        &mut self,
        model: String,
        prompt: String,
        context: Option<String>,
        output_goal: Option<String>,
        files: Vec<String>,
        read_only: bool,
        batch_id: Option<String>,
        config: AgentConfig,
    ) -> String {
        self.create_agent_internal(
            model,
            prompt,
            context,
            output_goal,
            files,
            read_only,
            batch_id,
            Some(config),
            false, // tmux_enabled defaults to false
        )
        .await
    }

    async fn create_agent_internal(
        &mut self,
        model: String,
        prompt: String,
        context: Option<String>,
        output_goal: Option<String>,
        files: Vec<String>,
        read_only: bool,
        batch_id: Option<String>,
        config: Option<AgentConfig>,
        tmux_enabled: bool,
    ) -> String {
        let agent_id = Uuid::new_v4().to_string();

        let agent = Agent {
            id: agent_id.clone(),
            batch_id,
            model,
            prompt,
            context,
            output_goal,
            files,
            read_only,
            status: AgentStatus::Pending,
            result: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            progress: Vec::new(),
            worktree_path: None,
            branch_name: None,
            config: config.clone(),
            tmux_enabled,
        };

        self.agents.insert(agent_id.clone(), agent.clone());

        // Send initial status update
        self.send_agent_status_update().await;

        // Spawn async agent
        let agent_id_clone = agent_id.clone();
        let handle = tokio::spawn(async move {
            execute_agent(agent_id_clone, config).await;
        });

        self.handles.insert(agent_id.clone(), handle);

        agent_id
    }

    pub fn get_agent(&self, agent_id: &str) -> Option<Agent> {
        self.agents.get(agent_id).cloned()
    }

    pub fn get_all_agents(&self) -> impl Iterator<Item = &Agent> {
        self.agents.values()
    }

    pub fn list_agents(
        &self,
        status_filter: Option<AgentStatus>,
        batch_id: Option<String>,
        recent_only: bool,
    ) -> Vec<Agent> {
        let cutoff = if recent_only {
            Some(Utc::now() - Duration::hours(2))
        } else {
            None
        };

        self.agents
            .values()
            .filter(|agent| {
                if let Some(ref filter) = status_filter {
                    if agent.status != *filter {
                        return false;
                    }
                }
                if let Some(ref batch) = batch_id {
                    if agent.batch_id.as_ref() != Some(batch) {
                        return false;
                    }
                }
                if let Some(cutoff) = cutoff {
                    if agent.created_at < cutoff {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    }

    pub fn has_active_agents(&self) -> bool {
        self.agents
            .values()
            .any(|agent| matches!(agent.status, AgentStatus::Pending | AgentStatus::Running))
    }

    pub async fn cancel_agent(&mut self, agent_id: &str) -> bool {
        if let Some(handle) = self.handles.remove(agent_id) {
            handle.abort();
            if let Some(agent) = self.agents.get_mut(agent_id) {
                agent.status = AgentStatus::Cancelled;
                agent.completed_at = Some(Utc::now());
            }
            true
        } else {
            false
        }
    }

    pub async fn cancel_batch(&mut self, batch_id: &str) -> usize {
        let agent_ids: Vec<String> = self
            .agents
            .values()
            .filter(|agent| agent.batch_id.as_ref() == Some(&batch_id.to_string()))
            .map(|agent| agent.id.clone())
            .collect();

        let mut count = 0;
        for agent_id in agent_ids {
            if self.cancel_agent(&agent_id).await {
                count += 1;
            }
        }
        count
    }

    pub async fn update_agent_status(&mut self, agent_id: &str, status: AgentStatus) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.status = status;
            if agent.status == AgentStatus::Running && agent.started_at.is_none() {
                agent.started_at = Some(Utc::now());
            }
            if matches!(
                agent.status,
                AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Cancelled
            ) {
                agent.completed_at = Some(Utc::now());
            }
            // Send status update event
            self.send_agent_status_update().await;
        }
    }

    pub async fn update_agent_result(&mut self, agent_id: &str, result: Result<String, String>) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            match result {
                Ok(output) => {
                    agent.result = Some(output);
                    agent.status = AgentStatus::Completed;
                }
                Err(error) => {
                    // SPEC-KIT-928: Extract raw output from error if available
                    // Format: "VALIDATION_FAILED: error\n\n--- RAW OUTPUT ---\ndata"
                    if let Some(raw_start) = error.find("--- RAW OUTPUT ---\n") {
                        let raw_output = &error[raw_start + 19..]; // Skip marker
                        agent.result = Some(raw_output.to_string());
                        tracing::info!(
                            "📦 Stored raw output ({} bytes) despite validation failure for agent {}",
                            raw_output.len(),
                            agent_id
                        );
                    }
                    agent.error = Some(error);
                    agent.status = AgentStatus::Failed;
                }
            }
            agent.completed_at = Some(Utc::now());
            // Send status update event
            self.send_agent_status_update().await;
        }
    }

    pub async fn add_progress(&mut self, agent_id: &str, message: String) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent
                .progress
                .push(format!("{}: {}", Utc::now().format("%H:%M:%S"), message));
            // Send updated agent status with the latest progress
            self.send_agent_status_update().await;
        }
    }

    /// SPEC-KIT-928: Check for concurrent agents of the same model type
    /// Returns list of (model_name, count) for any models with >1 running instance
    pub fn check_concurrent_agents(&self) -> Vec<(String, usize)> {
        let mut agent_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for agent in self.agents.values() {
            if matches!(
                agent.status,
                AgentStatus::Running | AgentStatus::Pending
            ) {
                // Extract base model name (e.g., "gemini" from "gemini-2.5-flash")
                let model_base = agent
                    .model
                    .split('-')
                    .next()
                    .unwrap_or(&agent.model)
                    .to_lowercase();
                *agent_counts.entry(model_base).or_insert(0) += 1;
            }
        }

        agent_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .collect()
    }

    /// SPEC-KIT-928: Get all currently running agents with details
    pub fn get_running_agents(&self) -> Vec<(String, String, String)> {
        // Returns: (agent_id, model, status)
        self.agents
            .values()
            .filter(|a| {
                matches!(
                    a.status,
                    AgentStatus::Running | AgentStatus::Pending
                )
            })
            .map(|a| {
                (
                    a.id.clone(),
                    a.model.clone(),
                    format!("{:?}", a.status),
                )
            })
            .collect()
    }

    pub async fn update_worktree_info(
        &mut self,
        agent_id: &str,
        worktree_path: String,
        branch_name: String,
    ) {
        if let Some(agent) = self.agents.get_mut(agent_id) {
            agent.worktree_path = Some(worktree_path);
            agent.branch_name = Some(branch_name);
        }
    }
}

async fn get_git_root() -> Result<PathBuf, String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output()
        .await
        .map_err(|e| format!("Git not installed or not in a git repository: {}", e))?;

    if output.status.success() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(PathBuf::from(path))
    } else {
        Err("Not in a git repository".to_string())
    }
}

use crate::git_worktree::sanitize_ref_component;

fn generate_branch_id(model: &str, agent: &str) -> String {
    // Extract first few meaningful words from agent for the branch name
    let stop = ["the", "and", "for", "with", "from", "into", "goal"]; // skip boilerplate
    let words: Vec<&str> = agent
        .split_whitespace()
        .filter(|w| w.len() > 2 && !stop.contains(&w.to_ascii_lowercase().as_str()))
        .take(3)
        .collect();

    let raw_suffix = if words.is_empty() {
        Uuid::new_v4()
            .to_string()
            .split('-')
            .next()
            .unwrap_or("agent")
            .to_string()
    } else {
        words.join("-")
    };

    // Sanitize both model and suffix for safety
    let model_s = sanitize_ref_component(model);
    let mut suffix_s = sanitize_ref_component(&raw_suffix);

    // Constrain length to keep branch names readable
    if suffix_s.len() > 40 {
        suffix_s.truncate(40);
        suffix_s = suffix_s.trim_matches('-').to_string();
        if suffix_s.is_empty() {
            suffix_s = "agent".to_string();
        }
    }

    format!("code-{}-{}", model_s, suffix_s)
}

use crate::git_worktree::setup_worktree;

/// SPEC-KIT-927+: Extract JSON from agent output with mixed content
///
/// Handles three patterns:
/// 1. Markdown fence (anywhere): "text...\n```json\n{...}\n```"
/// 2. Codex headers: "[timestamp] headers... [timestamp] User instructions: ... {json}"
/// 3. Plain JSON: "{...}" (no extraction needed)
///
/// Returns extracted JSON string, or original if no extraction needed.
fn extract_json_from_mixed_output(output: &str, model: &str) -> String {
    let output_preview = if output.len() > 100 {
        format!("{}...", &output[..100])
    } else {
        output.to_string()
    };
    tracing::trace!("🔍 Extraction input for {}: {} bytes, starts with: {}", model, output.len(), output_preview);

    // Pattern 1: Check for markdown code fence (ANYWHERE in output, not just start)
    if let Some(fence_start) = output.find("```json") {
        tracing::trace!("   Found ```json at position {}", fence_start);

        // Skip "```json" and any immediate newline
        let mut start_offset = fence_start + 7;
        let after_fence = &output[start_offset..];

        // Skip leading newline if present
        if after_fence.starts_with('\n') {
            start_offset += 1;
        } else if after_fence.starts_with("\r\n") {
            start_offset += 2;
        }

        let content_after_fence = &output[start_offset..];

        if let Some(fence_end) = content_after_fence.find("```") {
            let json_content = content_after_fence[..fence_end].trim();
            if !json_content.is_empty() {
                tracing::debug!(
                    "📦 Extracted JSON from markdown fence: {} -> {} bytes",
                    output.len(),
                    json_content.len()
                );
                return json_content.to_string();
            } else {
                tracing::warn!("⚠️ Markdown fence found but content is empty after trim");
            }
        } else {
            tracing::warn!("⚠️ Markdown fence ```json found but no closing ```");
        }
    }

    // Pattern 2: Check for codex headers (timestamp + headers + "User instructions:")
    // SPEC-KIT-928: Output structure has prompt schema BEFORE actual response
    // Lines 1-40: [timestamp] Codex header + User instructions + PROMPT SCHEMA
    // Lines 50-198: [timestamp] thinking ... [timestamp] codex
    // Lines 199+: ACTUAL AGENT RESPONSE { ... }
    // CRITICAL: Use "] codex" marker to find where actual response starts
    if output.contains("OpenAI Codex v") && output.contains("User instructions:") {
        // SPEC-KIT-928: Look for "] codex" marker - appears right before actual response
        if let Some(codex_marker_pos) = output.rfind("] codex\n") {
            let after_marker = &output[codex_marker_pos + 8..]; // Skip "] codex\n"
            tracing::debug!("📍 Found '] codex\\n' marker at position {}, extracting response", codex_marker_pos);

            // Strip trailing footer ([timestamp] tokens used: N)
            let cleaned = if let Some(footer_pos) = after_marker.rfind("] tokens used:") {
                if let Some(bracket_pos) = after_marker[..footer_pos].rfind('[') {
                    let result = after_marker[..bracket_pos].trim_end();
                    tracing::debug!("📍 Stripped tokens footer, response is now {} bytes", result.len());
                    result
                } else {
                    after_marker
                }
            } else {
                after_marker
            };

            if cleaned.len() > 100 {
                tracing::debug!(
                    "📦 Extracted JSON after '] codex' marker: {} -> {} bytes",
                    output.len(),
                    cleaned.len()
                );
                return cleaned.to_string();
            }
        } else if let Some(codex_marker_pos) = output.rfind("] codex") {
            // Handle case without newline
            let after_marker = output[codex_marker_pos + 7..].trim_start();
            tracing::debug!("📍 Found '] codex' marker (no newline) at position {}", codex_marker_pos);

            // Strip trailing footer
            let cleaned = if let Some(footer_pos) = after_marker.rfind("] tokens used:") {
                if let Some(bracket_pos) = after_marker[..footer_pos].rfind('[') {
                    after_marker[..bracket_pos].trim_end()
                } else {
                    after_marker
                }
            } else {
                after_marker
            };

            if cleaned.len() > 100 {
                tracing::debug!("📦 Extracted JSON after '] codex': {} bytes", cleaned.len());
                return cleaned.to_string();
            }
        }

        // Fallback: If codex marker not found, log warning
        tracing::warn!(
            "⚠️ Codex headers detected but no '] codex' marker found - output may include prompt schema"
        );
    }

    // Pattern 3: No extraction needed
    output.to_string()
}

async fn execute_agent(agent_id: String, config: Option<AgentConfig>) {
    let mut manager = AGENT_MANAGER.write().await;

    // Get agent details
    let agent = match manager.get_agent(&agent_id) {
        Some(t) => t,
        None => return,
    };

    // Update status to running
    manager
        .update_agent_status(&agent_id, AgentStatus::Running)
        .await;
    manager
        .add_progress(
            &agent_id,
            format!("Starting agent with model: {}", agent.model),
        )
        .await;

    let model = agent.model.clone();
    let prompt = agent.prompt.clone();
    let read_only = agent.read_only;
    let context = agent.context.clone();
    let output_goal = agent.output_goal.clone();
    let files = agent.files.clone();
    let tmux_enabled = agent.tmux_enabled;

    drop(manager); // Release the lock before executing

    // SPEC-KIT-928: Log execution parameters for debugging
    tracing::warn!(
        "🔍 AGENT EXEC START: agent_id={}, model={}, read_only={}, tmux={}",
        agent_id, model, read_only, tmux_enabled
    );

    // SPEC-KIT-927: Track execution duration for suspicious completion detection
    let execution_start = std::time::Instant::now();

    // Build the full prompt with context
    let mut full_prompt = prompt.clone();
    // Prepend any per-agent instructions from config when available
    if let Some(cfg) = config.as_ref() {
        if let Some(instr) = cfg.instructions.as_ref() {
            if !instr.trim().is_empty() {
                full_prompt = format!("{}\n\n{}", instr.trim(), full_prompt);
            }
        }
    }
    if let Some(context) = &context {
        full_prompt = format!("Context: {}\n\nAgent: {}", context, full_prompt);
    }
    if let Some(output_goal) = &output_goal {
        full_prompt = format!("{}\n\nDesired output: {}", full_prompt, output_goal);
    }
    if !files.is_empty() {
        full_prompt = format!("{}\n\nFiles to consider: {}", full_prompt, files.join(", "));
    }

    // Setup working directory and execute
    let result = if !read_only {
        // Check git and setup worktree for non-read-only mode
        match get_git_root().await {
            Ok(git_root) => {
                let branch_id = generate_branch_id(&model, &prompt);

                let mut manager = AGENT_MANAGER.write().await;
                manager
                    .add_progress(&agent_id, format!("Creating git worktree: {}", branch_id))
                    .await;
                drop(manager);

                match setup_worktree(&git_root, &branch_id).await {
                    Ok((worktree_path, used_branch)) => {
                        let mut manager = AGENT_MANAGER.write().await;
                        manager
                            .add_progress(
                                &agent_id,
                                format!("Executing in worktree: {}", worktree_path.display()),
                            )
                            .await;
                        manager
                            .update_worktree_info(
                                &agent_id,
                                worktree_path.display().to_string(),
                                used_branch.clone(),
                            )
                            .await;
                        drop(manager);

                        // Execute with full permissions in the worktree
                        execute_model_with_permissions(
                            &model,
                            &full_prompt,
                            false,
                            Some(worktree_path),
                            config.clone(),
                            tmux_enabled,
                        )
                        .await
                    }
                    Err(e) => Err(format!("Failed to setup worktree: {}", e)),
                }
            }
            Err(e) => Err(format!("Git is required for non-read-only agents: {}", e)),
        }
    } else {
        // Execute in read-only mode
        full_prompt = format!(
            "{}\n\n[Running in read-only mode - no modifications allowed]",
            full_prompt
        );
        execute_model_with_permissions(&model, &full_prompt, true, None, config, tmux_enabled).await
    };

    // SPEC-KIT-927: Calculate execution duration for suspicious completion detection
    let execution_duration = execution_start.elapsed();

    // SPEC-KIT-928: Log execution result for debugging
    match &result {
        Ok(output) => tracing::warn!(
            "✅ AGENT EXEC OK: agent_id={}, output_bytes={}, duration={:.2}s",
            agent_id, output.len(), execution_duration.as_secs_f64()
        ),
        Err(e) => tracing::warn!(
            "❌ AGENT EXEC FAILED: agent_id={}, error={}, duration={:.2}s",
            agent_id, e, execution_duration.as_secs_f64()
        ),
    }

    // SPEC-KIT-927+: Comprehensive output diagnostics before validation
    // Helps diagnose corruption patterns (TUI text, headers, schemas)
    if let Ok(ref output) = result {
        // Analyze output characteristics
        let has_json_start = output.trim_start().starts_with('{') || output.trim_start().starts_with('[');
        let has_markdown_fence = output.contains("```json") || output.contains("```");
        let has_codex_header = output.contains("OpenAI Codex v") || output.contains("[2025-");
        let has_tui_text = output.contains("codex\n\n") || output.contains("thetu@arch-dev");
        let has_unquoted_types = output.contains(": string") || output.contains(": number");
        let has_template_vars = output.contains("${");
        let line_count = output.lines().count();

        tracing::debug!(
            "📊 Agent {} output analysis: size={} bytes, lines={}, duration={}s, json_start={}, markdown={}, header={}, tui_text={}, unquoted_types={}, template_vars={}",
            model,
            output.len(),
            line_count,
            execution_duration.as_secs(),
            has_json_start,
            has_markdown_fence,
            has_codex_header,
            has_tui_text,
            has_unquoted_types,
            has_template_vars
        );

        // Detailed preview logging
        let preview_len = 300.min(output.len());
        tracing::debug!("📄 Output preview (first {} chars): {}", preview_len, &output[..preview_len]);
    }

    // SPEC-KIT-927: Validate output before marking agent as complete
    // This prevents storing partial/invalid output (schema templates, headers only)
    // SPEC-KIT-928: Keep reference to raw output for storing on validation failure
    let raw_output_for_storage = result.as_ref().ok().map(|s| s.clone());
    let validated_result = match result {
        Ok(output) => {
            // SPEC-KIT-927: Warn about suspiciously fast completions
            // Fast + small output often indicates premature collection
            if execution_duration < std::time::Duration::from_secs(30) && output.len() < 1000 {
                tracing::warn!(
                    "⚠️ SUSPICIOUS: Agent {} completed in {}s with only {} bytes - possible premature collection!",
                    model,
                    execution_duration.as_secs(),
                    output.len()
                );
            }

            // SPEC-KIT-927+: Extract JSON from mixed content
            // Handles both markdown-wrapped JSON and codex headers with embedded JSON
            let cleaned_output = extract_json_from_mixed_output(&output, &model);

            // Log extraction result
            if cleaned_output.len() != output.len() {
                tracing::info!(
                    "📦 Extraction changed output: {} -> {} bytes",
                    output.len(),
                    cleaned_output.len()
                );
            } else {
                tracing::trace!("   No extraction performed (output unchanged)");
            }

            // Validation 0: Output corruption detection (TUI text, conversation, etc.)
            // Expanded patterns based on diagnostic analysis
            if cleaned_output.contains("thetu@arch-dev") ||
               cleaned_output.contains("codex\n\nShort answer:") ||
               cleaned_output.contains("How do you want to proceed") ||
               (cleaned_output.contains("codex") && cleaned_output.contains("Got it. I'm focused")) {
                tracing::error!(
                    "❌ Agent {} output contains TUI conversation text! This indicates stdout mixing/pollution.",
                    model
                );
                tracing::error!("🔍 Corruption pattern: Terminal prompt or conversation detected");
                tracing::debug!("Corrupted output sample: {}", &cleaned_output.chars().take(500).collect::<String>());
                Err(format!(
                    "Agent output polluted with TUI conversation text. Stdout redirection broken."
                ))
            }
            // Check for headers-only output (codex initialization without actual response)
            else if cleaned_output.contains("OpenAI Codex v") && cleaned_output.contains("User instructions:") && !cleaned_output.contains("{") {
                tracing::error!(
                    "❌ Agent {} returned headers only (no JSON)! Premature collection detected.",
                    model
                );
                tracing::debug!("Headers-only output: {}", &cleaned_output.chars().take(400).collect::<String>());
                Err(format!(
                    "Agent returned initialization headers without JSON output. Premature collection."
                ))
            }
            // Validation 1: Minimum size check (>500 bytes for valid agent output)
            else if cleaned_output.len() < 500 {
                tracing::warn!(
                    "⚠️ Agent {} output too small: {} bytes (minimum 500) after {}s",
                    model,
                    cleaned_output.len(),
                    execution_duration.as_secs()
                );
                Err(format!(
                    "Agent output too small ({} bytes, minimum 500). Possible premature collection after {}s.",
                    cleaned_output.len(),
                    execution_duration.as_secs()
                ))
            }
            // Validation 2: Schema template detection (common in corrupted outputs)
            else if cleaned_output.contains("{ \"path\": string") ||
                    cleaned_output.contains("\"diff_proposals\": [ {") ||
                    cleaned_output.contains("\"change\": string (diff or summary)") {
                tracing::error!(
                    "❌ Agent {} returned JSON schema instead of data after {}s!",
                    model,
                    execution_duration.as_secs()
                );
                tracing::debug!("Schema output preview: {}...",
                    &cleaned_output.chars().take(500).collect::<String>());
                Err("Agent returned JSON schema template instead of actual data. Premature output collection detected.".to_string())
            }
            // Validation 3: JSON parsing (must be valid JSON)
            else if let Err(e) = serde_json::from_str::<serde_json::Value>(&cleaned_output) {
                tracing::error!(
                    "❌ Agent {} output is not valid JSON after {}s: {}",
                    model,
                    execution_duration.as_secs(),
                    e
                );
                tracing::debug!("Invalid JSON preview: {}...",
                    &cleaned_output.chars().take(500).collect::<String>());

                // SPEC-KIT-928: Save full invalid output to temp file for debugging
                let temp_file = format!("/tmp/agent-invalid-json-{}.txt", agent_id);
                if let Err(write_err) = std::fs::write(&temp_file, &cleaned_output) {
                    tracing::warn!("Failed to write invalid JSON to {}: {}", temp_file, write_err);
                } else {
                    tracing::error!("📝 Full invalid JSON saved to: {}", temp_file);
                }

                Err(format!("Agent output is not valid JSON: {}", e))
            }
            // All validations passed
            else {
                tracing::info!(
                    "✅ Agent {} output validated: {} bytes, valid JSON, completed in {}s",
                    model,
                    cleaned_output.len(),
                    execution_duration.as_secs()
                );
                Ok(cleaned_output)
            }
        }
        Err(e) => {
            // Error already occurred during execution
            tracing::error!("❌ Agent {} execution failed after {}s: {}", model, execution_duration.as_secs(), e);
            Err(e)
        }
    };

    // SPEC-KIT-928: Store raw output even if validation fails (for debugging)
    // Quality gate orchestrator needs to access agent.result to extract/fix JSON
    let mut manager = AGENT_MANAGER.write().await;
    match &validated_result {
        Ok(_) => {
            // Validation passed - store normally
            manager.update_agent_result(&agent_id, validated_result).await;
        }
        Err(validation_error) => {
            // Validation failed - but store the RAW output anyway for debugging
            // The error will be in agent.error, but result will have the raw data
            if let Some(raw_output) = raw_output_for_storage {
                let cleaned = extract_json_from_mixed_output(&raw_output, &model);
                tracing::warn!(
                    "⚠️ Storing raw output despite validation failure for agent {}",
                    agent_id
                );
                // Store output with validation error message prepended
                let output_with_error = format!(
                    "VALIDATION_FAILED: {}\n\n--- RAW OUTPUT ---\n{}",
                    validation_error, cleaned
                );
                manager.update_agent_result(&agent_id, Err(output_with_error)).await;
            } else {
                // Execution itself failed (not just validation)
                manager.update_agent_result(&agent_id, validated_result).await;
            }
        }
    }
}

async fn execute_model_with_permissions(
    model: &str,
    prompt: &str,
    read_only: bool,
    working_dir: Option<PathBuf>,
    config: Option<AgentConfig>,
    use_tmux: bool,
) -> Result<String, String> {
    // Helper: cross‑platform check whether an executable is available in PATH
    // and is directly spawnable by std::process::Command (no shell wrappers).
    fn command_exists(cmd: &str) -> bool {
        // Absolute/relative path with separators: check directly (files only).
        if cmd.contains(std::path::MAIN_SEPARATOR) || cmd.contains('/') || cmd.contains('\\') {
            return std::fs::metadata(cmd).map(|m| m.is_file()).unwrap_or(false);
        }

        #[cfg(target_os = "windows")]
        {
            // On Windows, ensure we only accept spawnable extensions. PowerShell
            // scripts like .ps1 are not directly spawnable via Command::new.
            if let Ok(p) = which::which(cmd) {
                if !p.is_file() {
                    return false;
                }
                match p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_ascii_lowercase())
                {
                    Some(ext) if matches!(ext.as_str(), "exe" | "com" | "cmd" | "bat") => true,
                    _ => false,
                }
            } else {
                false
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            use std::os::unix::fs::PermissionsExt;
            let Some(path_os) = std::env::var_os("PATH") else {
                return false;
            };
            for dir in std::env::split_paths(&path_os) {
                if dir.as_os_str().is_empty() {
                    continue;
                }
                let candidate = dir.join(cmd);
                if let Ok(meta) = std::fs::metadata(&candidate) {
                    if meta.is_file() {
                        let mode = meta.permissions().mode();
                        if mode & 0o111 != 0 {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }

    // Use config command if provided, otherwise use model name
    let command = if let Some(ref cfg) = config {
        cfg.command.clone()
    } else {
        model.to_lowercase()
    };

    // Special case: for the built‑in Codex agent, prefer invoking the currently
    // running executable with the `exec` subcommand rather than relying on a
    // `codex` binary to be present on PATH. This improves portability,
    // especially on Windows where global shims may be missing.
    let model_lower = model.to_lowercase();
    let mut cmd = if (model_lower == "code" || model_lower == "codex") && config.is_none() {
        match std::env::current_exe() {
            Ok(path) => Command::new(path),
            Err(e) => return Err(format!("Failed to resolve current executable: {}", e)),
        }
    } else {
        Command::new(command.clone())
    };

    // Set working directory if provided
    if let Some(dir) = working_dir.clone() {
        cmd.current_dir(dir);
    }

    // Add environment variables from config if provided
    if let Some(ref cfg) = config {
        if let Some(ref env) = cfg.env {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Add any configured args first, preferring mode‑specific values
        if read_only {
            if let Some(ro) = cfg.args_read_only.as_ref() {
                for arg in ro {
                    cmd.arg(arg);
                }
            } else {
                for arg in &cfg.args {
                    cmd.arg(arg);
                }
            }
        } else if let Some(w) = cfg.args_write.as_ref() {
            for arg in w {
                cmd.arg(arg);
            }
        } else {
            for arg in &cfg.args {
                cmd.arg(arg);
            }
        }
    }

    // Build command based on model and permissions
    // Use command instead of model for matching if config provided
    let model_name = if config.is_some() {
        command.as_str()
    } else {
        model_lower.as_str()
    };

    match model_name {
        "claude" | "gemini" | "qwen" => {
            let mut defaults = crate::agent_defaults::default_params_for(model_name, read_only);
            defaults.push("-p".into());
            defaults.push(prompt.to_string());
            cmd.args(defaults);
        }
        "codex" | "code" => {
            // If config provided explicit args for this mode, do not append defaults.
            let have_mode_args = config
                .as_ref()
                .map(|c| {
                    if read_only {
                        c.args_read_only.is_some()
                    } else {
                        c.args_write.is_some()
                    }
                })
                .unwrap_or(false);
            if have_mode_args {
                cmd.arg(prompt);
            } else {
                let mut defaults = crate::agent_defaults::default_params_for(model_name, read_only);
                defaults.push(prompt.to_string());
                cmd.args(defaults);
            }
        }
        _ => {
            return Err(format!("Unknown model: {}", model));
        }
    }

    // Proactively check for presence of external command before spawn when not
    // using the current executable fallback. This avoids confusing OS errors
    // like "program not found" and lets us surface a cleaner message.
    if model_name != "codex" && model_name != "code" && !command_exists(&command) {
        return Err(format!(
            "Required agent '{}' is not installed or not in PATH",
            command
        ));
    }

    // SPEC-KIT-923: Observable agent execution via tmux panes
    // If use_tmux is enabled and tmux is available, execute in a tmux pane
    if use_tmux && crate::tmux::is_tmux_available().await {
        tracing::info!("Using tmux pane execution for observable agent run");

        // Generate session name based on context
        let session_name = format!("agents-{}", model);

        // Ensure session exists
        if let Err(e) = crate::tmux::ensure_session(&session_name).await {
            tracing::warn!(
                "Failed to create tmux session, falling back to normal execution: {}",
                e
            );
            // Fall through to normal execution
        } else {
            // Create pane for this agent
            let pane_title = format!("{}", model);
            // SPEC-923: Always split new panes (is_first=false) to avoid reusing stale panes from previous runs
            // Each agent gets a fresh pane, ensuring clean shell state and proper completion marker detection
            match crate::tmux::create_pane(&session_name, &pane_title, false).await {
                Ok(pane_id) => {
                    // Build environment map, filtering out debug-related vars that pollute output
                    let mut env: std::collections::HashMap<String, String> =
                        std::env::vars()
                            .filter(|(k, _)| {
                                // Filter out debug/logging vars that would pollute agent JSON output
                                k != "RUST_LOG"
                                    && k != "RUST_BACKTRACE"
                                    && k != "RUST_LOG_STYLE"
                                    && !k.starts_with("RUST_LOG_")
                            })
                            .collect();
                    if let Some(ref cfg) = config {
                        if let Some(ref e) = cfg.env {
                            for (k, v) in e {
                                env.insert(k.clone(), v.clone());
                            }
                        }
                    }

                    // Build command string with args
                    let program =
                        if (model_lower == "code" || model_lower == "codex") && config.is_none() {
                            std::env::current_exe()
                                .map(|p| p.to_string_lossy().to_string())
                                .unwrap_or_else(|_| command.clone())
                        } else {
                            command.clone()
                        };

                    // Build args exactly as normal execution would
                    let mut args: Vec<String> = Vec::new();
                    if let Some(ref cfg) = config {
                        if read_only {
                            if let Some(ro) = cfg.args_read_only.as_ref() {
                                args.extend(ro.iter().cloned());
                            } else {
                                args.extend(cfg.args.iter().cloned());
                            }
                        } else if let Some(w) = cfg.args_write.as_ref() {
                            args.extend(w.iter().cloned());
                        } else {
                            args.extend(cfg.args.iter().cloned());
                        }
                    }

                    match model_name {
                        "claude" | "gemini" | "qwen" => {
                            let mut defaults =
                                crate::agent_defaults::default_params_for(model_name, read_only);
                            defaults.push("-p".into());
                            defaults.push(prompt.to_string());
                            args.extend(defaults);
                        }
                        "codex" | "code" => {
                            let have_mode_args = config
                                .as_ref()
                                .map(|c| {
                                    if read_only {
                                        c.args_read_only.is_some()
                                    } else {
                                        c.args_write.is_some()
                                    }
                                })
                                .unwrap_or(false);
                            if have_mode_args {
                                args.push(prompt.to_string());
                            } else {
                                let mut defaults = crate::agent_defaults::default_params_for(
                                    model_name, read_only,
                                );
                                defaults.push(prompt.to_string());
                                args.extend(defaults);
                            }
                        }
                        _ => {}
                    }

                    // Execute in tmux pane with 10 minute timeout
                    let timeout_secs = 600;
                    match crate::tmux::execute_in_pane(
                        &session_name,
                        &pane_id,
                        &program,
                        &args,
                        &env,
                        working_dir.as_deref(),
                        timeout_secs,
                    )
                    .await
                    {
                        Ok(output) => {
                            tracing::info!(
                                "Agent completed via tmux pane, {} bytes output",
                                output.len()
                            );

                            // Print attach instructions for user
                            let instructions = crate::tmux::get_attach_instructions(&session_name);
                            tracing::info!("{}", instructions);

                            return Ok(output);
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Tmux execution failed, falling back to normal execution: {}",
                                e
                            );
                            // Fall through to normal execution
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to create tmux pane, falling back to normal execution: {}",
                        e
                    );
                    // Fall through to normal execution
                }
            }
        }
    }

    // Agents: run without OS sandboxing; rely on per-branch worktrees for isolation.
    use crate::protocol::SandboxPolicy;
    use crate::spawn::StdioPolicy;
    let output = if !read_only {
        // Build env from current process then overlay any config-provided vars.
        let mut env: std::collections::HashMap<String, String> = std::env::vars().collect();
        let orig_home: Option<String> = env.get("HOME").cloned();
        if let Some(ref cfg) = config {
            if let Some(ref e) = cfg.env {
                for (k, v) in e {
                    env.insert(k.clone(), v.clone());
                }
            }
        }

        // Convenience: map common key names so external CLIs "just work".
        if let Some(google_key) = env.get("GOOGLE_API_KEY").cloned() {
            env.entry("GEMINI_API_KEY".to_string())
                .or_insert(google_key);
        }
        if let Some(claude_key) = env.get("CLAUDE_API_KEY").cloned() {
            env.entry("ANTHROPIC_API_KEY".to_string())
                .or_insert(claude_key);
        }
        if let Some(anthropic_key) = env.get("ANTHROPIC_API_KEY").cloned() {
            env.entry("CLAUDE_API_KEY".to_string())
                .or_insert(anthropic_key);
        }
        if let Some(anthropic_base) = env.get("ANTHROPIC_BASE_URL").cloned() {
            env.entry("CLAUDE_BASE_URL".to_string())
                .or_insert(anthropic_base);
        }
        // Qwen/DashScope convenience: mirror API keys and base URLs both ways so
        // either variable name works across tools.
        if let Some(qwen_key) = env.get("QWEN_API_KEY").cloned() {
            env.entry("DASHSCOPE_API_KEY".to_string())
                .or_insert(qwen_key);
        }
        if let Some(dashscope_key) = env.get("DASHSCOPE_API_KEY").cloned() {
            env.entry("QWEN_API_KEY".to_string())
                .or_insert(dashscope_key);
        }
        if let Some(qwen_base) = env.get("QWEN_BASE_URL").cloned() {
            env.entry("DASHSCOPE_BASE_URL".to_string())
                .or_insert(qwen_base);
        }
        if let Some(ds_base) = env.get("DASHSCOPE_BASE_URL").cloned() {
            env.entry("QWEN_BASE_URL".to_string()).or_insert(ds_base);
        }
        // Reduce startup overhead for Claude CLI: disable auto-updater/telemetry.
        env.entry("DISABLE_AUTOUPDATER".to_string())
            .or_insert("1".to_string());
        env.entry("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string())
            .or_insert("1".to_string());
        env.entry("DISABLE_ERROR_REPORTING".to_string())
            .or_insert("1".to_string());
        // Prefer explicit Claude config dir to avoid touching $HOME/.claude.json.
        // Do not force CLAUDE_CONFIG_DIR here; leave CLI free to use its default
        // (including Keychain) unless we explicitly redirect HOME below.

        // If GEMINI_API_KEY not provided, try pointing to host config for read‑only
        // discovery (Gemini CLI supports GEMINI_CONFIG_DIR). We keep HOME as-is so
        // CLIs that require ~/.gemini and ~/.claude continue to work with your
        // existing config.
        if env.get("GEMINI_API_KEY").is_none() {
            if let Some(h) = orig_home.clone() {
                let host_gem_cfg = std::path::PathBuf::from(&h).join(".gemini");
                if host_gem_cfg.is_dir() {
                    env.insert(
                        "GEMINI_CONFIG_DIR".to_string(),
                        host_gem_cfg.to_string_lossy().to_string(),
                    );
                }
            }
        }

        // No OS sandbox.

        // Resolve the command and args we prepared above into Vec<String> for spawn helpers.
        // Intentionally build args fresh for sandbox helpers; `Command` does not expose argv.
        // Rebuild the invocation as `command` + args set above.
        // We reconstruct to run under our sandbox helpers.
        let program = if (model_lower == "code" || model_lower == "codex") && config.is_none() {
            // Use current exe path
            std::env::current_exe()
                .map_err(|e| format!("Failed to resolve current executable: {}", e))?
        } else {
            // Use program name; PATH resolution will be handled by spawn helper with provided env.
            std::path::PathBuf::from(&command)
        };

        // Rebuild args exactly as above
        let mut args: Vec<String> = Vec::new();
        // Include configured args (mode‑specific preferred) first, to mirror the
        // immediate-Command path above.
        if let Some(ref cfg) = config {
            if read_only {
                if let Some(ro) = cfg.args_read_only.as_ref() {
                    args.extend(ro.iter().cloned());
                } else {
                    args.extend(cfg.args.iter().cloned());
                }
            } else if let Some(w) = cfg.args_write.as_ref() {
                args.extend(w.iter().cloned());
            } else {
                args.extend(cfg.args.iter().cloned());
            }
        }

        match model_name {
            "claude" | "gemini" | "qwen" => {
                let mut defaults = crate::agent_defaults::default_params_for(model_name, read_only);
                defaults.push("-p".into());
                defaults.push(prompt.to_string());
                args.extend(defaults);
            }
            "codex" | "code" => {
                let have_mode_args = config
                    .as_ref()
                    .map(|c| {
                        if read_only {
                            c.args_read_only.is_some()
                        } else {
                            c.args_write.is_some()
                        }
                    })
                    .unwrap_or(false);
                if have_mode_args {
                    args.push(prompt.to_string());
                } else {
                    let mut defaults =
                        crate::agent_defaults::default_params_for(model_name, read_only);
                    defaults.push(prompt.to_string());
                    args.extend(defaults);
                }
            }
            _ => {}
        }

        // Always run agents without OS sandboxing.
        let sandbox_type = crate::exec::SandboxType::None;

        // Spawn via helpers and capture output
        let child_result: std::io::Result<tokio::process::Child> = match sandbox_type {
            crate::exec::SandboxType::None
            | crate::exec::SandboxType::MacosSeatbelt
            | crate::exec::SandboxType::LinuxSeccomp => {
                crate::spawn::spawn_child_async(
                    program.clone(),
                    args.clone(),
                    Some(&program.to_string_lossy()),
                    working_dir.clone().unwrap_or_else(|| {
                        std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
                    }),
                    &SandboxPolicy::DangerFullAccess,
                    StdioPolicy::RedirectForShellTool,
                    env.clone(),
                )
                .await
            }
        };

        match child_result {
            Ok(child) => child
                .wait_with_output()
                .await
                .map_err(|e| format!("Failed to read output: {}", e))?,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    return Err(format!(
                        "Required agent '{}' is not installed or not in PATH",
                        command
                    ));
                }
                return Err(format!("Failed to spawn sandboxed agent: {}", e));
            }
        }
    } else {
        // Read-only path: use prior behavior
        match cmd.output().await {
            Ok(o) => o,
            Err(e) => {
                // Only fall back for external CLIs (not the built-in code/codex path)
                if model_name == "codex" || model_name == "code" {
                    return Err(format!("Failed to execute {}: {}", model, e));
                }
                let mut fb = match std::env::current_exe() {
                    Ok(p) => Command::new(p),
                    Err(e2) => {
                        return Err(format!(
                            "Failed to execute {} and could not resolve built-in fallback: {} / {}",
                            model, e, e2
                        ));
                    }
                };
                if read_only {
                    fb.args([
                        "-s",
                        "read-only",
                        "-a",
                        "never",
                        "exec",
                        "--skip-git-repo-check",
                        prompt,
                    ]);
                } else {
                    fb.args([
                        "-s",
                        "workspace-write",
                        "-a",
                        "never",
                        "exec",
                        "--skip-git-repo-check",
                        prompt,
                    ]);
                }
                fb.output().await.map_err(|e2| {
                    format!(
                        "Failed to execute {} ({}). Built-in fallback also failed: {}",
                        model, e, e2
                    )
                })?
            }
        }
    };

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let combined = if stderr.trim().is_empty() {
            stdout.trim().to_string()
        } else if stdout.trim().is_empty() {
            stderr.trim().to_string()
        } else {
            format!("{}\n{}", stderr.trim(), stdout.trim())
        };
        Err(format!("Command failed: {}", combined))
    }
}

// Tool creation functions
pub fn create_run_agent_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "task".to_string(),
        JsonSchema::String {
            description: Some("The task prompt - what to perform (required)".to_string()),
        },
    );

    properties.insert(
        "models".to_string(),
        JsonSchema::Array {
            items: Box::new(JsonSchema::String { description: None }),
            description: Some(
                "Optional: Array of model names (e.g., ['claude','gemini','qwen','code'])"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "context".to_string(),
        JsonSchema::String {
            description: Some("Optional: Background context for the agent".to_string()),
        },
    );

    properties.insert(
        "output".to_string(),
        JsonSchema::String {
            description: Some("Optional: The desired output/success state".to_string()),
        },
    );

    properties.insert(
        "files".to_string(),
        JsonSchema::Array {
            items: Box::new(JsonSchema::String { description: None }),
            description: Some(
                "Optional: Array of file paths to include in the agent context".to_string(),
            ),
        },
    );

    properties.insert(
        "read_only".to_string(),
        JsonSchema::Boolean {
            description: Some(
                "Optional: When true, agent runs in read-only mode (default: false)".to_string(),
            ),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_run".to_string(),
        description: "Start a complex AI task asynchronously. Returns an agent ID immediately to check status and retrieve results. Once an agent is running, enables: agent_check, agent_result, agent_cancel, agent_wait, agent_list.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["task".to_string()]),
            additional_properties: Some(false),
        },
    })
}

pub fn create_check_agent_status_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "agent_id".to_string(),
        JsonSchema::String {
            description: Some("The agent ID returned from run_agent".to_string()),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_check".to_string(),
        description: "Check the status of a running agent. Returns current status, progress, and partial results if available.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["agent_id".to_string()]),
            additional_properties: Some(false),
        },
    })
}

pub fn create_get_agent_result_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "agent_id".to_string(),
        JsonSchema::String {
            description: Some("The agent ID returned from run_agent".to_string()),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_result".to_string(),
        description: "Get the final result of a completed agent.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec!["agent_id".to_string()]),
            additional_properties: Some(false),
        },
    })
}

pub fn create_cancel_agent_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "agent_id".to_string(),
        JsonSchema::String {
            description: Some(
                "The agent ID to cancel (required if batch_id not provided)".to_string(),
            ),
        },
    );

    properties.insert(
        "batch_id".to_string(),
        JsonSchema::String {
            description: Some(
                "Cancel all agents with this batch ID (required if agent_id not provided)"
                    .to_string(),
            ),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_cancel".to_string(),
        description: "Cancel a pending or running agent, or all agents in a batch.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![]),
            additional_properties: Some(false),
        },
    })
}

pub fn create_wait_for_agent_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "agent_id".to_string(),
        JsonSchema::String {
            description: Some(
                "Wait for this specific agent to complete (required if batch_id not provided)"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "batch_id".to_string(),
        JsonSchema::String {
            description: Some(
                "Wait for any agent in this batch to complete (required if agent_id not provided)"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "timeout_seconds".to_string(),
        JsonSchema::Number {
            description: Some(
                "Maximum seconds to wait before timing out (default: 300, max: 600)".to_string(),
            ),
        },
    );

    properties.insert(
        "return_all".to_string(),
        JsonSchema::Boolean {
            description: Some("For batch_id: return all completed agents instead of just the first one (default: false)".to_string()),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_wait".to_string(),
        description: "Wait for a agent or any agent in a batch to complete, fail, or be cancelled."
            .to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![]),
            additional_properties: Some(false),
        },
    })
}

pub fn create_list_agents_tool() -> OpenAiTool {
    let mut properties = BTreeMap::new();

    properties.insert(
        "status_filter".to_string(),
        JsonSchema::String {
            description: Some("Optional: Filter agents by status (pending, running, completed, failed, cancelled)".to_string()),
        },
    );

    properties.insert(
        "batch_id".to_string(),
        JsonSchema::String {
            description: Some("Optional: Filter agents by batch ID".to_string()),
        },
    );

    properties.insert(
        "recent_only".to_string(),
        JsonSchema::Boolean {
            description: Some(
                "Optional: Only show agents from the last 2 hours (default: false)".to_string(),
            ),
        },
    );

    OpenAiTool::Function(ResponsesApiTool {
        name: "agent_list".to_string(),
        description: "List all agents with their current status.".to_string(),
        strict: false,
        parameters: JsonSchema::Object {
            properties,
            required: Some(vec![]),
            additional_properties: Some(false),
        },
    })
}

// Parameter structs for handlers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunAgentParams {
    pub task: String,
    #[serde(default, deserialize_with = "deserialize_models_field")]
    pub models: Vec<String>,
    pub context: Option<String>,
    pub output: Option<String>,
    pub files: Option<Vec<String>>,
    pub read_only: Option<bool>,
}

fn deserialize_models_field<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ModelsInput {
        Seq(Vec<String>),
        One(String),
    }

    let parsed = Option::<ModelsInput>::deserialize(deserializer)?;
    Ok(match parsed {
        Some(ModelsInput::Seq(seq)) => seq,
        Some(ModelsInput::One(single)) => vec![single],
        None => Vec::new(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckAgentStatusParams {
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetAgentResultParams {
    pub agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelAgentParams {
    pub agent_id: Option<String>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitForAgentParams {
    pub agent_id: Option<String>,
    pub batch_id: Option<String>,
    pub timeout_seconds: Option<u64>,
    pub return_all: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAgentsParams {
    pub status_filter: Option<String>,
    pub batch_id: Option<String>,
    pub recent_only: Option<bool>,
}
