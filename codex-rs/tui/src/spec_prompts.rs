#![allow(dead_code)] // Spec-kit prompt templates and generation

use std::collections::HashMap;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use regex_lite::Regex;
use serde::Deserialize;
use std::fmt::Write as _;

use crate::templates::{TemplateSource, resolve_template_source};
use std::path::Path;

const PROMPTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/spec-kit/prompts.json"
));

// -----------------------------------------------------------------------------
// Unified Prompt-Source API (D113/D133 Parity Fix)
// -----------------------------------------------------------------------------

/// Load prompt template and version from project-local prompts.json if it exists,
/// otherwise fall back to embedded prompts.
///
/// Returns `(prompt_template, prompt_version)` from the SAME source to ensure parity.
///
/// # Arguments
/// * `stage_key` - The stage key (e.g., "spec-plan", "spec-tasks")
/// * `agent` - The agent to get the prompt for
/// * `project_cwd` - Optional project directory; if Some, tries to load from `cwd/docs/spec-kit/prompts.json`
pub fn get_prompt_with_version(
    stage_key: &str,
    agent: SpecAgent,
    project_cwd: Option<&Path>,
) -> Option<(String, String)> {
    // Try project-local first if path provided
    if let Some(cwd) = project_cwd {
        if let Some(result) = load_prompt_from_project(cwd, stage_key, agent) {
            return Some(result);
        }
    }

    // Fall back to embedded
    load_prompt_from_embedded(stage_key, agent)
}

/// Load prompt from project-local prompts.json
///
/// D113/D133: Distinguishes "file missing" (silent fallback) from "file broken" (warn + fallback).
fn load_prompt_from_project(
    cwd: &Path,
    stage_key: &str,
    agent: SpecAgent,
) -> Option<(String, String)> {
    let prompts_path = cwd.join("docs/spec-kit/prompts.json");

    // Check if file exists first - silent fallback if missing
    if !prompts_path.exists() {
        return None;
    }

    // File exists - attempt read (warn on failure)
    let content = match std::fs::read_to_string(&prompts_path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(
                "Project prompts.json exists but unreadable at {}: {}. Falling back to embedded.",
                prompts_path.display(),
                e
            );
            return None;
        }
    };

    // Attempt parse (warn on invalid JSON)
    let prompts: serde_json::Value = match serde_json::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                "Project prompts.json at {} contains invalid JSON: {}. Falling back to embedded.",
                prompts_path.display(),
                e
            );
            return None;
        }
    };

    // Stage/agent lookup failures are silent (partial overrides are valid)
    let stage_prompts = prompts.get(stage_key)?;

    // Extract version
    let version = stage_prompts
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unversioned")
        .to_string();

    // Extract agent's prompt template
    let agent_key = agent.canonical_name();
    let prompt_template = stage_prompts
        .get(agent_key)
        .and_then(|v| v.get("prompt"))
        .and_then(|v| v.as_str())?
        .to_string();

    Some((prompt_template, version))
}

/// Load prompt from embedded PROMPTS_JSON constant
fn load_prompt_from_embedded(stage_key: &str, agent: SpecAgent) -> Option<(String, String)> {
    let prompt_data = agent_prompt(stage_key, agent)?;
    let version = stage_version(stage_key).unwrap_or_else(|| "unversioned".to_string());
    Some((prompt_data.prompt, version))
}

/// Fully render a prompt template with all substitutions including ${TEMPLATE:*} expansion.
///
/// This is the single-point-of-truth for prompt rendering. All code paths
/// (TUI agent_orchestrator, headless, quality gates) should use this function.
///
/// Substitution order:
/// 1. User-provided variables (SPEC_ID, CONTEXT, etc.)
/// 2. Model metadata (MODEL_ID, MODEL_RELEASE, REASONING_MODE)
/// 3. Prompt version (PROMPT_VERSION)
/// 4. Template expansion (${TEMPLATE:*})
///
/// # Arguments
/// * `template` - The raw prompt template with ${VAR} placeholders
/// * `version` - The prompt version to inject
/// * `vars` - User-provided variables as (key, value) pairs
/// * `stage` - The spec stage (for model metadata lookup)
/// * `agent` - The agent (for model metadata lookup)
pub fn render_prompt_text(
    template: &str,
    version: &str,
    vars: &[(&str, &str)],
    stage: SpecStage,
    agent: SpecAgent,
) -> String {
    let mut text = template.to_string();

    // 1. User-provided variables
    for (key, value) in vars {
        let placeholder = format!("${{{}}}", key);
        text = text.replace(&placeholder, value);
    }

    // 2. Model metadata
    let metadata = model_metadata(stage, agent);
    for (key, value) in metadata {
        let placeholder = format!("${{{}}}", key);
        text = text.replace(&placeholder, &value);
    }

    // 3. Prompt version
    if text.contains("${PROMPT_VERSION}") {
        text = text.replace("${PROMPT_VERSION}", version);
    }

    // 4. Template expansion (CRITICAL: call expand_template_refs)
    text = expand_template_refs(&text);

    // Debug assertion: no template tokens should remain
    debug_assert!(
        !text.contains("${TEMPLATE:"),
        "Template token(s) leaked through prompt rendering: {}",
        text.chars().take(200).collect::<String>()
    );

    text
}

fn block_on_sync<F, Fut, T>(factory: F) -> T
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        let handle_clone = handle.clone();
        tokio::task::block_in_place(move || handle_clone.block_on(factory()))
    } else {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build runtime")
            .block_on(factory())
    }
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct AgentPrompt {
    #[serde(default)]
    pub role: Option<String>,
    pub prompt: String,
}

#[derive(Debug, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(default)]
pub struct StagePrompts {
    pub version: Option<String>,
    pub gemini: Option<AgentPrompt>,
    pub claude: Option<AgentPrompt>,
    pub code: Option<AgentPrompt>,
    #[serde(rename = "gpt_codex")]
    pub gpt_codex: Option<AgentPrompt>,
    #[serde(rename = "gpt_pro")]
    pub gpt_pro: Option<AgentPrompt>,
    pub orchestrator_notes: Option<Vec<String>>,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecAgent {
    Gemini,
    Claude,
    Code, // Claude Code (CLI assistant)
    GptCodex,
    GptPro,
}

// ARCH-006: Centralize agent name normalization
impl SpecAgent {
    /// Canonical name for storage/comparison (lowercase with underscores)
    pub fn canonical_name(&self) -> &'static str {
        match self {
            SpecAgent::Gemini => "gemini",
            SpecAgent::Claude => "claude",
            SpecAgent::Code => "code",
            SpecAgent::GptCodex => "gpt_codex",
            SpecAgent::GptPro => "gpt_pro",
        }
    }

    /// Parse from various string representations (case-insensitive)
    pub fn from_string(s: &str) -> Option<Self> {
        let normalized = s.to_ascii_lowercase().replace(['-', ' '], "_");
        let trimmed = normalized.trim_matches('_');

        if trimmed.is_empty() {
            return None;
        }

        if trimmed.starts_with("gemini") {
            return Some(Self::Gemini);
        }

        if trimmed.starts_with("claude") {
            return Some(Self::Claude);
        }

        if trimmed.starts_with("code") || trimmed.starts_with("claude_code") {
            return Some(Self::Code);
        }

        if trimmed.contains("codex") {
            return Some(Self::GptCodex);
        }

        if trimmed.starts_with("gpt") {
            return Some(Self::GptPro);
        }

        None
    }

    /// Display name for UI rendering
    pub fn display_name(&self) -> &'static str {
        match self {
            SpecAgent::Gemini => "Gemini",
            SpecAgent::Claude => "Claude",
            SpecAgent::Code => "Claude Code",
            SpecAgent::GptCodex => "GPT-5 Codex",
            SpecAgent::GptPro => "GPT-5 Pro",
        }
    }

    /// All expected agents for consensus checking
    pub fn all() -> [Self; 5] {
        [
            Self::Gemini,
            Self::Claude,
            Self::Code,
            Self::GptCodex,
            Self::GptPro,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct PromptRegistry {
    stages: HashMap<String, StagePrompts>,
}

static PROMPT_DATA: OnceCell<PromptRegistry> = OnceCell::new();

impl PromptRegistry {
    fn load() -> Self {
        let stages: HashMap<String, StagePrompts> =
            serde_json::from_str(PROMPTS_JSON).expect("invalid spec-kit prompt json");
        Self { stages }
    }

    pub fn stage(&self, name: &str) -> Option<&StagePrompts> {
        self.stages.get(name)
    }
}

pub fn registry() -> &'static PromptRegistry {
    PROMPT_DATA.get_or_init(PromptRegistry::load)
}

pub fn stage_version(stage: &str) -> Option<String> {
    registry().stage(stage)?.version.clone()
}

pub fn agent_prompt(stage: &str, agent: SpecAgent) -> Option<AgentPrompt> {
    let stage = registry().stage(stage)?;
    let prompt = match agent {
        SpecAgent::Gemini => stage.gemini.clone(),
        SpecAgent::Claude => stage.claude.clone(),
        SpecAgent::Code => stage.code.clone(),
        SpecAgent::GptCodex => stage.gpt_codex.clone(),
        SpecAgent::GptPro => stage.gpt_pro.clone(),
    }?;
    Some(prompt)
}

pub fn orchestrator_notes(stage: &str) -> Option<Vec<String>> {
    registry().stage(stage)?.orchestrator_notes.clone()
}

/// Expand `${TEMPLATE:name}` references in a prompt string.
///
/// Template references are resolved using the layered resolution system:
/// 1. Project-local: `./templates/{name}-template.md`
/// 2. User config: `~/.config/code/templates/{name}-template.md`
/// 3. Embedded: Compiled into binary
///
/// Returns the source path or `[embedded:name]` for embedded templates.
pub fn expand_template_refs(text: &str) -> String {
    // Lazy static regex for ${TEMPLATE:name} pattern
    static TEMPLATE_RE: once_cell::sync::OnceCell<Regex> = once_cell::sync::OnceCell::new();
    let re = TEMPLATE_RE
        .get_or_init(|| Regex::new(r"\$\{TEMPLATE:(\w+)\}").expect("valid template regex"));

    re.replace_all(text, |caps: &regex_lite::Captures| {
        let name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        // SPEC-KIT-964: Only project-local and embedded sources (hermetic isolation)
        match resolve_template_source(name) {
            TemplateSource::ProjectLocal(p) => p.display().to_string(),
            TemplateSource::Embedded => format!("[embedded:{}]", name),
        }
    })
    .to_string()
}

pub fn render_prompt(stage: &str, agent: SpecAgent, vars: &[(&str, &str)]) -> Option<String> {
    let prompt = agent_prompt(stage, agent)?;
    let mut text = prompt.prompt;
    for (key, value) in vars {
        let placeholder = format!("${{{}}}", key);
        text = text.replace(&placeholder, value);
    }
    if text.contains("${PROMPT_VERSION}") {
        let version = stage_version(stage).unwrap_or_else(|| "unversioned".to_string());
        text = text.replace("${PROMPT_VERSION}", &version);
    }
    // SPEC-KIT-962: Expand template references after other substitutions
    text = expand_template_refs(&text);
    Some(text)
}

fn stage_env_suffix(stage: SpecStage) -> String {
    stage.key().replace('-', "_").to_ascii_uppercase()
}

fn agent_env_prefix(agent: SpecAgent) -> &'static str {
    match agent {
        SpecAgent::Gemini => "GEMINI",
        SpecAgent::Claude => "CLAUDE",
        SpecAgent::Code => "CODE",
        SpecAgent::GptCodex => "GPT_CODEX",
        SpecAgent::GptPro => "GPT_PRO",
    }
}

fn resolve_metadata_field(
    field: &str,
    stage: SpecStage,
    agent: SpecAgent,
    default: &str,
) -> String {
    let stage_key = stage_env_suffix(stage);
    let agent_key = agent_env_prefix(agent);
    let mut env_name = String::new();
    write!(env_name, "SPECKIT_{}_{}_{}", field, agent_key, stage_key).unwrap();
    if let Ok(value) = std::env::var(&env_name) {
        return value;
    }
    env_name.clear();
    write!(env_name, "SPECKIT_{}_{}", field, agent_key).unwrap();
    if let Ok(value) = std::env::var(&env_name) {
        return value;
    }
    default.to_string()
}

pub fn model_metadata(stage: SpecStage, agent: SpecAgent) -> Vec<(String, String)> {
    let (model_id_default, release_default, mode_default) = match (stage, agent) {
        (SpecStage::Tasks | SpecStage::Unlock, SpecAgent::Gemini) => {
            ("gemini-2.5-flash", "2025-05-14", "fast")
        }
        (_, SpecAgent::Gemini) => ("gemini-2.5-pro", "2025-05-14", "thinking"),
        (SpecStage::Unlock, SpecAgent::Claude) => ("claude-4.5-sonnet", "2025-09-29", "balanced"),
        (_, SpecAgent::Claude) => ("claude-4.5-sonnet", "2025-09-29", "balanced"),
        (_, SpecAgent::Code) => ("claude-sonnet-4-5", "2025-10-22", "extended"),
        (SpecStage::Implement, SpecAgent::GptCodex) => ("gpt-5-codex", "2025-09-29", "auto"),
        (_, SpecAgent::GptCodex) => ("gpt-5-codex", "2025-09-29", "auto"),
        (SpecStage::Implement, SpecAgent::GptPro) => ("gpt-5", "2025-08-06", "high"),
        (_, SpecAgent::GptPro) => ("gpt-5", "2025-08-06", "high"),
    };

    vec![
        (
            "MODEL_ID".into(),
            resolve_metadata_field("MODEL_ID", stage, agent, model_id_default),
        ),
        (
            "MODEL_RELEASE".into(),
            resolve_metadata_field("MODEL_RELEASE", stage, agent, release_default),
        ),
        (
            "REASONING_MODE".into(),
            resolve_metadata_field("REASONING_MODE", stage, agent, mode_default),
        ),
    ]
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SpecStage {
    // Pre-pipeline stage (PRD generation)
    Specify,
    // Main 6-stage pipeline
    Plan,
    Tasks,
    Implement,
    Validate,
    Audit,
    Unlock,
    // Quality commands (not part of main pipeline)
    Clarify,
    Analyze,
    Checklist,
}

impl SpecStage {
    pub fn all() -> [SpecStage; 6] {
        [
            SpecStage::Plan,
            SpecStage::Tasks,
            SpecStage::Implement,
            SpecStage::Validate,
            SpecStage::Audit,
            SpecStage::Unlock,
        ]
    }

    /// All stages including quality commands and pre-pipeline stages
    pub fn all_including_quality() -> [SpecStage; 10] {
        [
            SpecStage::Specify,
            SpecStage::Plan,
            SpecStage::Tasks,
            SpecStage::Implement,
            SpecStage::Validate,
            SpecStage::Audit,
            SpecStage::Unlock,
            SpecStage::Clarify,
            SpecStage::Analyze,
            SpecStage::Checklist,
        ]
    }

    pub fn key(self) -> &'static str {
        match self {
            SpecStage::Specify => "spec-specify",
            SpecStage::Plan => "spec-plan",
            SpecStage::Tasks => "spec-tasks",
            SpecStage::Implement => "spec-implement",
            SpecStage::Validate => "spec-validate",
            SpecStage::Audit => "spec-audit",
            SpecStage::Unlock => "spec-unlock",
            SpecStage::Clarify => "spec-clarify",
            SpecStage::Analyze => "spec-analyze",
            SpecStage::Checklist => "spec-checklist",
        }
    }

    pub fn command_name(self) -> &'static str {
        match self {
            SpecStage::Specify => "speckit.specify",
            SpecStage::Plan => "spec-plan",
            SpecStage::Tasks => "spec-tasks",
            SpecStage::Implement => "spec-implement",
            SpecStage::Validate => "spec-validate",
            SpecStage::Audit => "spec-audit",
            SpecStage::Unlock => "spec-unlock",
            SpecStage::Clarify => "spec-clarify",
            SpecStage::Analyze => "spec-analyze",
            SpecStage::Checklist => "spec-checklist",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            SpecStage::Specify => "Specify",
            SpecStage::Plan => "Plan",
            SpecStage::Tasks => "Tasks",
            SpecStage::Implement => "Implement",
            SpecStage::Validate => "Validate",
            SpecStage::Audit => "Audit",
            SpecStage::Unlock => "Unlock",
            SpecStage::Clarify => "Clarify",
            SpecStage::Analyze => "Analyze",
            SpecStage::Checklist => "Checklist",
        }
    }

    /// Check if this is a quality command (not part of main pipeline)
    pub fn is_quality_command(self) -> bool {
        matches!(
            self,
            SpecStage::Clarify | SpecStage::Analyze | SpecStage::Checklist
        )
    }

    /// Check if this is a pre-pipeline stage (before main 6-stage pipeline)
    pub fn is_pre_pipeline(self) -> bool {
        matches!(self, SpecStage::Specify)
    }

    /// Parse from stage name string (used by headless for TUI parity)
    ///
    /// Accepts both short names ("plan") and full keys ("spec-plan")
    pub fn from_stage_name(name: &str) -> Option<Self> {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "plan" | "spec-plan" => Some(SpecStage::Plan),
            "tasks" | "spec-tasks" => Some(SpecStage::Tasks),
            "implement" | "spec-implement" => Some(SpecStage::Implement),
            "validate" | "spec-validate" => Some(SpecStage::Validate),
            "audit" | "spec-audit" => Some(SpecStage::Audit),
            "unlock" | "spec-unlock" => Some(SpecStage::Unlock),
            "specify" | "spec-specify" => Some(SpecStage::Specify),
            "clarify" | "spec-clarify" => Some(SpecStage::Clarify),
            "analyze" | "spec-analyze" => Some(SpecStage::Analyze),
            "checklist" | "spec-checklist" => Some(SpecStage::Checklist),
            _ => None,
        }
    }
}

pub fn stage_version_enum(stage: SpecStage) -> Option<String> {
    stage_version(stage.key())
}

#[derive(Debug, thiserror::Error)]
pub enum PromptBuildError {
    #[error("`/{command}` requires a SPEC ID (e.g. `/{command} SPEC-OPS-005`)")]
    MissingSpecId { command: String },
    #[error("No prompts defined for stage '{0}'")]
    MissingStage(&'static str),
}

pub fn build_stage_prompt(stage: SpecStage, raw_args: &str) -> Result<String, PromptBuildError> {
    build_stage_prompt_with_mcp(stage, raw_args, None)
}

pub fn build_stage_prompt_with_mcp(
    stage: SpecStage,
    raw_args: &str,
    mcp_manager: Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>,
) -> Result<String, PromptBuildError> {
    let trimmed = raw_args.trim();
    if trimmed.is_empty() {
        return Err(PromptBuildError::MissingSpecId {
            command: stage.command_name().to_string(),
        });
    }

    let mut parts = trimmed.split_whitespace();
    let spec_id = parts.next().unwrap();
    let remainder = trimmed[spec_id.len()..].trim().to_string();

    let context_hint = format!(
        "Use local-memory search (domains: spec-tracker, docs-ops, impl-notes, infra-ci) to gather current context for {spec_id}. Summaries must cite memory IDs or captured CLI output."
    );

    let goal_hint = if !remainder.is_empty() {
        remainder.clone()
    } else {
        "(no additional goal provided)".to_string()
    };

    let mut replacements: Vec<(String, String)> = vec![
        ("SPEC_ID".into(), spec_id.to_string()),
        ("CONTEXT".into(), context_hint.clone()),
        ("GOAL".into(), goal_hint.clone()),
    ];

    match stage {
        SpecStage::Specify => {
            // Pre-pipeline stage: PRD generation from initial SPEC template
            replacements.push((
                "SPEC_TEMPLATE".into(),
                "Initial SPEC template created by /speckit.new (docs/SPEC-*/PRD.md).".into(),
            ));
        }
        SpecStage::Plan => {
            replacements.push((
                "PREVIOUS_OUTPUTS.gemini".into(),
                "Gemini Ultra findings stored in local-memory (spec-tracker domain).".into(),
            ));
            replacements.push((
                "PREVIOUS_OUTPUTS".into(),
                "Refer to Gemini + Claude outputs captured in local-memory for consensus notes."
                    .into(),
            ));
        }
        SpecStage::Tasks => {
            replacements.push((
                "PREVIOUS_OUTPUTS.gemini".into(),
                "Gemini research from /spec-plan (local-memory spec-tracker).".into(),
            ));
            replacements.push((
                "PREVIOUS_OUTPUTS.plan".into(),
                "Final plan consensus written during /spec-plan.".into(),
            ));
        }
        SpecStage::Implement => {
            replacements.push((
                "PREVIOUS_OUTPUTS.tasks".into(),
                "Latest /spec-tasks consensus stored in docs/SPEC-*/tasks.md and local-memory."
                    .into(),
            ));
        }
        SpecStage::Validate | SpecStage::Audit | SpecStage::Unlock => {
            // No extra replacements required
        }
        SpecStage::Clarify | SpecStage::Analyze | SpecStage::Checklist => {
            // Quality commands: no special replacements
        }
    }

    // Provide fallbacks for placeholders that might appear in prompts
    replacements.push((
        "PREVIOUS_OUTPUTS".into(),
        "See local-memory entries from earlier /spec-* stages.".into(),
    ));
    replacements.push((
        "PREVIOUS_OUTPUTS.plan".into(),
        "Final plan consensus (if available).".into(),
    ));
    replacements.push((
        "PREVIOUS_OUTPUTS.tasks".into(),
        "Task breakdown consensus (if available).".into(),
    ));

    let registry = registry();
    let stage_prompts = registry
        .stage(stage.key())
        .ok_or(PromptBuildError::MissingStage(stage.key()))?;
    let prompt_version = stage_prompts
        .version
        .clone()
        .unwrap_or_else(|| "unversioned".to_string());
    replacements.push(("PROMPT_VERSION".into(), prompt_version.clone()));

    let mut bundle = String::new();
    bundle.push_str(&format!("# /{} — {}\n\n", stage.command_name(), spec_id));
    bundle.push_str("Leverage local-memory before starting, then run the agents below in parallel using these prompts. Record outputs back into local-memory (spec-tracker, impl-notes, docs-ops).\n\n");
    if let SpecStage::Plan = stage {
        bundle.push_str(&format!("Goal: {}\n\n", goal_hint));
    }
    bundle.push_str(&format!("Prompt version: {}\n\n", prompt_version));

    match gather_local_memory_context(spec_id, stage, mcp_manager) {
        Ok(entries) if !entries.is_empty() => {
            bundle.push_str("## Local-memory context\n");
            for entry in entries {
                bundle.push_str("- ");
                bundle.push_str(&entry);
                bundle.push('\n');
            }
            bundle.push('\n');
        }
        Ok(_) => {
            bundle.push_str(
                "## Local-memory context\n- No stage-specific local-memory entries found yet.\n\n",
            );
        }
        Err(err) => {
            bundle.push_str(&format!(
                "## Local-memory context\n- Unable to fetch local-memory context: {}\n\n",
                err
            ));
        }
    }

    bundle.push_str("## HTTP MCP (HAL)\n");
    bundle.push_str(
        "- If a HAL HTTP MCP profile is configured (see docs/SPEC-KIT-018-hal-http-mcp), drive the health/REST/GraphQL templates via `cargo run -p codex-mcp-client --bin call_tool -- --tool … -- npx -y hal-mcp` and archive the outputs in the project’s evidence folder.\n\n",
    );

    if let Some(prompt) = stage_prompts.gemini.clone() {
        let mut gemini_vars = replacements.clone();
        gemini_vars.extend(model_metadata(stage, SpecAgent::Gemini));
        let gemini_refs: Vec<(&str, &str)> = gemini_vars
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let rendered =
            render_prompt(stage.key(), SpecAgent::Gemini, &gemini_refs).unwrap_or(prompt.prompt);
        bundle.push_str("## Gemini Ultra — Research\n");
        bundle.push_str(&rendered);
        bundle.push_str("\n\n");
    }
    if let Some(prompt) = stage_prompts.claude.clone() {
        let mut claude_vars = replacements.clone();
        claude_vars.extend(model_metadata(stage, SpecAgent::Claude));
        let claude_refs: Vec<(&str, &str)> = claude_vars
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let rendered =
            render_prompt(stage.key(), SpecAgent::Claude, &claude_refs).unwrap_or(prompt.prompt);
        bundle.push_str("## Claude Sonnet 4.5 — Synthesis\n");
        bundle.push_str(&rendered);
        bundle.push_str("\n\n");
    }
    if let Some(prompt) = stage_prompts.gpt_codex.clone() {
        let mut codex_vars = replacements.clone();
        codex_vars.extend(model_metadata(stage, SpecAgent::GptCodex));
        let codex_refs: Vec<(&str, &str)> = codex_vars
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let rendered =
            render_prompt(stage.key(), SpecAgent::GptCodex, &codex_refs).unwrap_or(prompt.prompt);
        bundle.push_str("## GPT-5 Codex — Code Diff Proposal\n");
        bundle.push_str(&rendered);
        bundle.push_str("\n\n");
    }
    if let Some(prompt) = stage_prompts.gpt_pro.clone() {
        let mut gpt_vars = replacements.clone();
        gpt_vars.extend(model_metadata(stage, SpecAgent::GptPro));
        let gpt_refs: Vec<(&str, &str)> = gpt_vars
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let rendered =
            render_prompt(stage.key(), SpecAgent::GptPro, &gpt_refs).unwrap_or(prompt.prompt);
        bundle.push_str("## GPT-5 — Arbiter & QA\n");
        bundle.push_str(&rendered);
        bundle.push('\n');
    }

    Ok(bundle)
}

// FORK-SPECIFIC (just-every/code): Migrated to native MCP (ARCH-004 completion)
fn gather_local_memory_context(
    spec_id: &str,
    stage: SpecStage,
    _mcp_manager: Option<Arc<codex_core::mcp_connection_manager::McpConnectionManager>>,
) -> Result<Vec<String>, String> {
    if !crate::local_memory_cli::local_memory_daemon_healthy_blocking(
        std::time::Duration::from_millis(750),
    ) {
        return Err("local-memory daemon not available at http://localhost:3002".to_string());
    }

    let stage_name = stage.command_name().to_string();
    let query = format!("{} {}", spec_id, stage_name);
    let tags = vec![format!("spec:{spec_id}"), format!("stage:{stage_name}")];
    let results = crate::local_memory_cli::search_blocking(&query, 8, &tags, None, 500)?;

    let mut entries: Vec<String> = Vec::new();

    for result in results.into_iter().take(5) {
        let mut snippet = result.memory.content.trim().replace('\n', " ");
        if snippet.is_empty() {
            continue;
        }
        if snippet.len() > 160 {
            snippet.truncate(160);
            snippet.push('…');
        }

        if let Some(id) = result.memory.id.as_ref() {
            entries.push(format!("{} — {}", id, snippet));
        } else {
            entries.push(snippet);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::collections::HashMap;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn build_stub_script(responses: &[(&str, &str)]) -> String {
        let mut script =
            String::from("#!/usr/bin/env python3\nimport json\nimport sys\nresponses = {\n");
        for (key, json) in responses {
            script.push_str(&format!("    {key:?}: json.loads({json:?}),\n"));
        }
        if !responses.iter().any(|(key, _)| *key == "default") {
            script.push_str("    'default': {'success': True, 'data': {'results': []}},\n");
        }
        script.push_str(
            "}\nstage = 'default'\nfor arg in sys.argv:\n  if 'stage:' in arg:\n    for token in arg.split(','):\n      if token.startswith('stage:'):\n        stage = token\n        break\n",
        );
        script.push_str(
            "payload = responses.get(stage, responses.get('default', {'success': True, 'data': {'results': []}}))\n",
        );
        script.push_str("json.dump(payload, sys.stdout)\n");
        script.push_str("sys.stdout.flush()\n");
        script
    }

    fn with_local_memory_stub<F, R>(responses: &[(&str, &str)], f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp_dir = TempDir::new().expect("temp dir");
        let script_path = temp_dir.path().join("local-memory-stub.py");
        let script_content = build_stub_script(responses);
        fs::write(&script_path, script_content).expect("write stub");
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms).unwrap();
        }
        unsafe {
            std::env::set_var("LOCAL_MEMORY_BIN", &script_path);
            std::env::set_var("LOCAL_MEMORY_SKIP_HEALTHCHECK", "1");
        }
        let result = f();
        unsafe {
            std::env::remove_var("LOCAL_MEMORY_BIN");
            std::env::remove_var("LOCAL_MEMORY_SKIP_HEALTHCHECK");
        }
        drop(temp_dir);
        result
    }

    fn metadata_map(stage: SpecStage, agent: SpecAgent) -> HashMap<String, String> {
        model_metadata(stage, agent)
            .into_iter()
            .collect::<HashMap<_, _>>()
    }

    #[test]
    fn agent_prompt_is_loaded() {
        let gemini = agent_prompt("spec-plan", SpecAgent::Gemini).expect("gemini prompt");
        assert!(gemini.prompt.contains("Summarize:"));
    }

    #[test]
    fn placeholder_substitution() {
        let mut owned: Vec<(String, String)> = vec![
            ("SPEC_ID".into(), "SPEC-OPS-123".into()),
            ("CONTEXT".into(), "<ctx>".into()),
        ];
        owned.extend(model_metadata(SpecStage::Plan, SpecAgent::Gemini));
        let refs: Vec<(&str, &str)> = owned
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        let rendered = render_prompt("spec-plan", SpecAgent::Gemini, &refs).expect("rendered");
        assert!(rendered.contains("SPEC-OPS-123"));
        assert!(rendered.contains("<ctx>"));
    }

    #[test]
    fn orchestrator_notes_present_for_auto() {
        let notes = orchestrator_notes("spec-auto").expect("notes");
        assert!(!notes.is_empty());
    }

    #[test]
    fn build_stage_prompt_requires_spec_id() {
        let err = build_stage_prompt(SpecStage::Plan, " ").unwrap_err();
        assert!(matches!(err, PromptBuildError::MissingSpecId { .. }));
    }

    #[test]
    fn build_stage_prompt_includes_agent_sections() {
        let prompt = build_stage_prompt(SpecStage::Plan, "SPEC-OPS-999 Align rollout").unwrap();
        assert!(prompt.contains("/spec-plan"));
        assert!(prompt.contains("Prompt version: 20251002-plan-a"));
        assert!(prompt.contains("Gemini Ultra"));
        assert!(prompt.contains("Claude Sonnet"));
        assert!(prompt.contains("GPT-5"));
    }

    #[test]
    fn gather_local_memory_context_returns_empty_without_mcp() {
        let entries = with_local_memory_stub(
            &[("default", r#"{"success": true, "data": {"results": []}}"#)],
            || gather_local_memory_context("SPEC-OPS-123", SpecStage::Plan, None).unwrap(),
        );
        assert!(entries.is_empty());
    }

    #[test]
    fn gather_local_memory_context_handles_no_runtime() {
        let entries = with_local_memory_stub(
            &[("default", r#"{"success": true, "data": {"results": []}}"#)],
            || gather_local_memory_context("SPEC-OPS-123", SpecStage::Plan, None).unwrap(),
        );
        assert!(entries.is_empty());
    }

    #[test]
    fn build_stage_prompt_works_without_mcp() {
        let prompt = with_local_memory_stub(
            &[("default", r#"{"success": true, "data": {"results": []}}"#)],
            || build_stage_prompt(SpecStage::Plan, "SPEC-OPS-123 Align migration").unwrap(),
        );

        assert!(prompt.contains("## Local-memory context"));
        assert!(prompt.contains("No stage-specific local-memory entries"));
        assert!(prompt.contains("Prompt version"));
    }

    #[test]
    fn build_stage_prompt_emits_empty_notice_for_tasks() {
        let prompt = with_local_memory_stub(
            &[("default", r#"{"success": true, "data": {"results": []}}"#)],
            || build_stage_prompt(SpecStage::Tasks, "SPEC-OPS-123").unwrap(),
        );

        assert!(prompt.contains("No stage-specific local-memory entries"));
        assert!(prompt.contains("Prompt version: 20251002-tasks-a"));
    }

    #[test]
    fn build_stage_prompt_includes_version_for_tasks() {
        let prompt = with_local_memory_stub(
            &[("default", r#"{"success": true, "data": {"results": []}}"#)],
            || build_stage_prompt(SpecStage::Tasks, "SPEC-OPS-123").unwrap(),
        );

        assert!(prompt.contains("## Local-memory context"));
        assert!(prompt.contains("Prompt version: 20251002-tasks-a"));
    }

    #[test]
    fn all_versioned_prompts_include_placeholder() {
        for stage in SpecStage::all() {
            let stage_key = stage.key();
            let version = stage_version(stage_key);
            if version.is_none() {
                continue;
            }
            let prompts = registry().stage(stage_key).expect("stage present");
            for (agent, prompt_opt) in [
                ("gemini", prompts.gemini.as_ref()),
                ("claude", prompts.claude.as_ref()),
                ("gpt_codex", prompts.gpt_codex.as_ref()),
                ("gpt_pro", prompts.gpt_pro.as_ref()),
            ] {
                if let Some(prompt) = prompt_opt {
                    assert!(
                        prompt.prompt.contains("${PROMPT_VERSION}"),
                        "prompt for stage {} agent {} missing ${{PROMPT_VERSION}}",
                        stage_key,
                        agent
                    );
                }
            }
        }
    }

    #[test]
    #[serial]
    fn model_metadata_defaults_align_with_strategy() {
        let gemini = metadata_map(SpecStage::Plan, SpecAgent::Gemini);
        assert_eq!(gemini.get("MODEL_ID"), Some(&"gemini-2.5-pro".to_string()));
        assert_eq!(gemini.get("REASONING_MODE"), Some(&"thinking".to_string()));

        let claude = metadata_map(SpecStage::Implement, SpecAgent::Claude);
        assert_eq!(
            claude.get("MODEL_ID"),
            Some(&"claude-4.5-sonnet".to_string())
        );

        let codex = metadata_map(SpecStage::Implement, SpecAgent::GptCodex);
        assert_eq!(codex.get("MODEL_ID"), Some(&"gpt-5-codex".to_string()));
        assert_eq!(codex.get("REASONING_MODE"), Some(&"auto".to_string()));

        let gpt = metadata_map(SpecStage::Implement, SpecAgent::GptPro);
        assert_eq!(gpt.get("MODEL_ID"), Some(&"gpt-5".to_string()));
        assert_eq!(gpt.get("REASONING_MODE"), Some(&"high".to_string()));
    }

    #[test]
    #[serial]
    fn model_metadata_env_overrides_apply() {
        unsafe {
            std::env::set_var("SPECKIT_MODEL_ID_GPT_PRO_SPEC_IMPLEMENT", "custom-gpt");
            std::env::set_var("SPECKIT_MODEL_RELEASE_GPT_PRO", "2025-09-27");
            std::env::set_var("SPECKIT_REASONING_MODE_GPT_PRO", "deep");
        }
        let map = metadata_map(SpecStage::Implement, SpecAgent::GptPro);
        unsafe {
            std::env::remove_var("SPECKIT_MODEL_ID_GPT_PRO_SPEC_IMPLEMENT");
            std::env::remove_var("SPECKIT_MODEL_RELEASE_GPT_PRO");
            std::env::remove_var("SPECKIT_REASONING_MODE_GPT_PRO");
        }

        assert_eq!(map.get("MODEL_ID"), Some(&"custom-gpt".to_string()));
        assert_eq!(map.get("MODEL_RELEASE"), Some(&"2025-09-27".to_string()));
        assert_eq!(map.get("REASONING_MODE"), Some(&"deep".to_string()));
    }

    // SPEC-KIT-962: Template expansion tests
    #[test]
    fn expand_template_refs_embedded_fallback() {
        // Without local templates, should resolve to embedded
        let input = "Template: ${TEMPLATE:plan}\n\nTask: Build something";
        let output = expand_template_refs(input);
        assert!(output.contains("[embedded:plan]"));
        assert!(output.contains("Task: Build something"));
    }

    #[test]
    fn expand_template_refs_multiple_templates() {
        let input = "Use ${TEMPLATE:plan} and ${TEMPLATE:tasks} for guidance.";
        let output = expand_template_refs(input);
        assert!(output.contains("[embedded:plan]"));
        assert!(output.contains("[embedded:tasks]"));
        assert!(!output.contains("${TEMPLATE:"));
    }

    #[test]
    fn expand_template_refs_no_templates() {
        let input = "No template references here.";
        let output = expand_template_refs(input);
        assert_eq!(input, output);
    }

    #[test]
    fn expand_template_refs_unknown_template() {
        // Unknown templates still expand (to embedded marker, but content would be empty)
        let input = "${TEMPLATE:nonexistent}";
        let output = expand_template_refs(input);
        assert!(output.contains("[embedded:nonexistent]"));
    }

    // D113/D133 Parity: Unified prompt-source API tests

    #[test]
    fn get_prompt_with_version_embedded_fallback() {
        // Without project path, should fall back to embedded
        let result = get_prompt_with_version("spec-plan", SpecAgent::Gemini, None);
        assert!(result.is_some());
        let (template, version) = result.unwrap();
        assert!(!template.is_empty());
        assert!(!version.is_empty());
        assert_ne!(version, "unversioned");
    }

    #[test]
    fn get_prompt_with_version_nonexistent_project() {
        // With non-existent project path, should fall back to embedded
        let temp = TempDir::new().unwrap();
        let result = get_prompt_with_version("spec-plan", SpecAgent::Gemini, Some(temp.path()));
        assert!(result.is_some());
        let (template, version) = result.unwrap();
        assert!(!template.is_empty());
        assert_ne!(version, "unversioned");
    }

    #[test]
    fn get_prompt_with_version_invalid_json_falls_back_to_embedded() {
        // D113/D133: Invalid JSON in project prompts.json should warn and fall back to embedded
        let temp = TempDir::new().unwrap();
        let prompts_dir = temp.path().join("docs/spec-kit");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Write invalid JSON
        std::fs::write(prompts_dir.join("prompts.json"), "{ invalid json }").unwrap();

        // Should return Some(...) via embedded fallback, not None
        let result = get_prompt_with_version("spec-plan", SpecAgent::Gemini, Some(temp.path()));
        assert!(
            result.is_some(),
            "Expected embedded fallback when project prompts.json is invalid"
        );
        let (template, version) = result.unwrap();
        assert!(!template.is_empty());
        assert_ne!(version, "unversioned");
    }

    #[test]
    fn render_prompt_text_no_template_leakage() {
        // Critical: no ${TEMPLATE:*} tokens should remain after rendering
        let template = "Template: ${TEMPLATE:plan}\nSPEC: ${SPEC_ID}\nContext: ${CONTEXT}";
        let prompt = render_prompt_text(
            template,
            "v1.0",
            &[("SPEC_ID", "TEST-001"), ("CONTEXT", "test context")],
            SpecStage::Plan,
            SpecAgent::Gemini,
        );

        assert!(
            !prompt.contains("${TEMPLATE:"),
            "Template token leaked: {}",
            prompt
        );
        assert!(prompt.contains("TEST-001"));
        assert!(prompt.contains("test context"));
        assert!(prompt.contains("[embedded:plan]") || prompt.contains("templates/plan"));
    }

    #[test]
    fn render_prompt_text_all_variables_replaced() {
        let template =
            "Model: ${MODEL_ID} v${MODEL_RELEASE} (${REASONING_MODE})\nVersion: ${PROMPT_VERSION}";
        let prompt = render_prompt_text(
            template,
            "20251002-test",
            &[],
            SpecStage::Plan,
            SpecAgent::Gemini,
        );

        assert!(!prompt.contains("${MODEL_ID}"));
        assert!(!prompt.contains("${MODEL_RELEASE}"));
        assert!(!prompt.contains("${REASONING_MODE}"));
        assert!(!prompt.contains("${PROMPT_VERSION}"));
        assert!(prompt.contains("20251002-test"));
    }

    #[test]
    fn spec_stage_from_stage_name_short() {
        assert_eq!(SpecStage::from_stage_name("plan"), Some(SpecStage::Plan));
        assert_eq!(SpecStage::from_stage_name("tasks"), Some(SpecStage::Tasks));
        assert_eq!(
            SpecStage::from_stage_name("implement"),
            Some(SpecStage::Implement)
        );
        assert_eq!(
            SpecStage::from_stage_name("validate"),
            Some(SpecStage::Validate)
        );
        assert_eq!(SpecStage::from_stage_name("audit"), Some(SpecStage::Audit));
        assert_eq!(
            SpecStage::from_stage_name("unlock"),
            Some(SpecStage::Unlock)
        );
    }

    #[test]
    fn spec_stage_from_stage_name_full_key() {
        assert_eq!(
            SpecStage::from_stage_name("spec-plan"),
            Some(SpecStage::Plan)
        );
        assert_eq!(
            SpecStage::from_stage_name("spec-tasks"),
            Some(SpecStage::Tasks)
        );
        assert_eq!(
            SpecStage::from_stage_name("spec-implement"),
            Some(SpecStage::Implement)
        );
    }

    #[test]
    fn spec_stage_from_stage_name_case_insensitive() {
        assert_eq!(SpecStage::from_stage_name("PLAN"), Some(SpecStage::Plan));
        assert_eq!(SpecStage::from_stage_name("Plan"), Some(SpecStage::Plan));
        assert_eq!(
            SpecStage::from_stage_name("SPEC-PLAN"),
            Some(SpecStage::Plan)
        );
    }

    #[test]
    fn spec_stage_from_stage_name_invalid() {
        assert_eq!(SpecStage::from_stage_name("unknown"), None);
        assert_eq!(SpecStage::from_stage_name(""), None);
    }

    #[test]
    fn tasks_stage_preferred_agent_no_previous_outputs_leakage() {
        // MAINT-14: Verify Tasks stage Claude prompt does not contain ${PREVIOUS_OUTPUTS
        // This is critical because Tasks stage runs in single-agent mode (only Claude)
        // and Gemini never executes, so ${PREVIOUS_OUTPUTS.gemini} would leak to the model.

        // Get the Claude prompt for Tasks stage (Claude is the preferred/only agent)
        let result = get_prompt_with_version("spec-tasks", SpecAgent::Claude, None);
        assert!(result.is_some(), "Expected spec-tasks Claude prompt to exist");

        let (template, version) = result.unwrap();

        // Render the prompt with standard variables
        let prompt = render_prompt_text(
            &template,
            &version,
            &[
                ("SPEC_ID", "TEST-001"),
                ("CONTEXT", "Test context for tasks stage"),
            ],
            SpecStage::Tasks,
            SpecAgent::Claude,
        );

        // Assert no ${PREVIOUS_OUTPUTS leak
        assert!(
            !prompt.contains("${PREVIOUS_OUTPUTS"),
            "PREVIOUS_OUTPUTS placeholder leaked to model in Tasks stage prompt: {}",
            &prompt[..prompt.len().min(500)]
        );

        // Also check the raw template doesn't have the problematic references
        assert!(
            !template.contains("${PREVIOUS_OUTPUTS.gemini}"),
            "Tasks Claude template still contains ${{PREVIOUS_OUTPUTS.gemini}}"
        );
        assert!(
            !template.contains("${PREVIOUS_OUTPUTS.plan}"),
            "Tasks Claude template still contains ${{PREVIOUS_OUTPUTS.plan}}"
        );
    }

    // MAINT-15: Guard test - no PREVIOUS_OUTPUTS in embedded prompts
    #[test]
    fn embedded_prompts_json_no_previous_outputs() {
        // Assert PROMPTS_JSON contains no ${PREVIOUS_OUTPUTS substring anywhere
        // This prevents consensus-era relics from creeping back in
        assert!(
            !PROMPTS_JSON.contains("${PREVIOUS_OUTPUTS"),
            "Embedded prompts.json still contains ${{PREVIOUS_OUTPUTS}} - consensus era relic"
        );
    }

    // MAINT-15: Guard test - preferred-agent prompts have ${CONTEXT}
    #[test]
    fn preferred_agent_prompts_have_context() {
        // Table-driven test for stages with Claude as preferred agent
        let stages = [
            ("spec-implement", SpecStage::Implement),
            ("spec-validate", SpecStage::Validate),
            ("spec-audit", SpecStage::Audit),
            ("spec-unlock", SpecStage::Unlock),
        ];

        for (stage_key, stage) in stages {
            let result = get_prompt_with_version(stage_key, SpecAgent::Claude, None);
            assert!(
                result.is_some(),
                "Expected {} Claude prompt to exist",
                stage_key
            );

            let (template, version) = result.unwrap();

            // Template must contain ${CONTEXT}
            assert!(
                template.contains("${CONTEXT}"),
                "{} Claude template missing ${{CONTEXT}}",
                stage_key
            );

            // Rendered prompt must not leak ${PREVIOUS_OUTPUTS} or ${TEMPLATE:
            let prompt = render_prompt_text(
                &template,
                &version,
                &[("SPEC_ID", "TEST-001"), ("CONTEXT", "test context")],
                stage,
                SpecAgent::Claude,
            );

            assert!(
                !prompt.contains("${PREVIOUS_OUTPUTS"),
                "{} leaks ${{PREVIOUS_OUTPUTS}}: {}",
                stage_key,
                &prompt[..prompt.len().min(200)]
            );
            assert!(
                !prompt.contains("${TEMPLATE:"),
                "{} leaks ${{TEMPLATE:}}: {}",
                stage_key,
                &prompt[..prompt.len().min(200)]
            );
        }
    }
}
