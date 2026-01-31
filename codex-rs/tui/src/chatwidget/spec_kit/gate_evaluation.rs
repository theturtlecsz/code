//! Consensus checking infrastructure for multi-agent spec-kit automation
//!
//! This module handles consensus validation across multiple AI agents,
//! artifact collection from local-memory, and synthesis result persistence.
//!
//! ## GR-001 Policy Compliance (MODEL-POLICY.md v1.0.0)
//!
//! Multi-agent consensus is **disabled by default** per GR-001. The canonical pipeline is:
//!   Stage 0 → Single Architect → Single Implementer → Single Judge
//!
//! **Feature Flags:**
//! - `SPEC_KIT_CONSENSUS=true` - Enable legacy multi-agent consensus (DEPRECATED)
//! - `SPEC_KIT_SIDECAR_CRITIC=true` - Enable non-blocking critic-only sidecar
//!   (legacy alias: `SPEC_KIT_CRITIC` - deprecated)
//!
//! **Default Behavior (no env vars):**
//! - Single agent per stage (preferred_agent_for_stage())
//! - Consensus check skipped
//! - Quality enforced by compiler/tests and Judge audit

use super::error::{Result, SpecKitError};
// FORK-SPECIFIC (just-every/code): LocalMemoryClient removed, using native MCP
use crate::spec_prompts::SpecStage;
use codex_spec_kit::config::PolicyToggles;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::PathBuf;

// ============================================================================
// TYPES (moved from chatwidget/mod.rs)
//
// These are "wire types" for JSON serialization backward compatibility.
// Domain code should use types from `codex_spec_kit::gate_policy` where possible.
// ============================================================================

/// Wire type for artifact data from gate evaluation.
///
/// Used for collecting agent outputs during consensus/gate evaluation.
/// For domain logic, prefer `codex_spec_kit::gate_policy` types.
#[derive(Debug, Clone)]
pub(crate) struct GateArtifactData {
    pub memory_id: Option<String>,
    pub agent: String,
    pub version: Option<String>,
    pub content: Value,
}

/// Wire type for evidence file handles.
///
/// Tracks evidence artifacts with path and checksum.
#[derive(Clone)]
#[allow(dead_code)] // Used for evidence tracking, fields may be used in future
pub(crate) struct GateEvidenceHandle {
    pub path: PathBuf,
    pub sha256: String,
}

/// Wire type for telemetry path tracking.
///
/// Collects paths to agent artifacts, telemetry bundle, and synthesis output.
#[allow(dead_code)] // Used for telemetry path tracking, fields consumed externally
pub(crate) struct GateTelemetryPaths {
    pub agent_paths: Vec<PathBuf>,
    pub telemetry_path: PathBuf,
    pub synthesis_path: PathBuf,
}

/// Wire type for artifact verdict in JSON persistence.
///
/// Serialized as part of StageReviewVerdict for evidence persistence.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct GateArtifactVerdict {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    pub agent: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub content: Value,
}

/// Wire type for stage review verdict in JSON persistence.
///
/// Contains the full verdict from a stage gate evaluation, including
/// agreements, conflicts, and all agent artifacts.
///
/// **Serde Compatibility**: The `consensus_ok` field uses serde aliases to support both
/// legacy ("consensus_ok") and new ("gate_ok", "review_ok") field names when reading.
/// The legacy name is preserved for wire format stability with existing evidence files.
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StageReviewVerdict {
    pub spec_id: String,
    pub stage: String,
    pub recorded_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_version: Option<String>,
    /// Whether the stage review passed. Legacy field name kept for wire compatibility.
    /// New code should interpret this as "gate passed" or "review passed".
    #[serde(alias = "gate_ok", alias = "review_ok")]
    pub consensus_ok: bool,
    pub degraded: bool,
    pub required_fields_ok: bool,
    pub missing_agents: Vec<String>,
    pub agreements: Vec<String>,
    pub conflicts: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregator_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregator_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregator: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthesis_path: Option<String>,
    pub artifacts: Vec<GateArtifactVerdict>,
}

/// Wire type for synthesis summary (internal use).
///
/// Parsed from synthesis JSON files for display and validation.
#[derive(Debug)]
pub(crate) struct StageReviewSummary {
    pub status: String,
    pub missing_agents: Vec<String>,
    pub agreements: Vec<String>,
    pub conflicts: Vec<String>,
    pub prompt_version: Option<String>,
    pub path: PathBuf,
}

/// Wire type for reading raw synthesis JSON from disk.
///
/// Intermediate type for deserializing synthesis files before conversion.
#[derive(Debug, Deserialize)]
pub(crate) struct StageReviewRaw {
    pub stage: Option<String>,
    #[serde(rename = "specId")]
    pub spec_id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub missing_agents: Vec<String>,
    #[serde(default)]
    pub consensus: StageReviewConsensusRaw,
    pub prompt_version: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct StageReviewConsensusRaw {
    #[serde(default)]
    pub agreements: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub(in super::super) fn telemetry_value_truthy(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

pub(in super::super) fn telemetry_agent_slug(agent: &str) -> String {
    let mut slug = String::new();
    let mut last_was_sep = false;
    for ch in agent.chars() {
        let lower = ch.to_ascii_lowercase();
        let is_alnum = lower.is_ascii_alphanumeric();
        if is_alnum {
            slug.push(lower);
            last_was_sep = false;
        } else if !slug.is_empty() && !last_was_sep {
            slug.push('_');
            last_was_sep = true;
        }
    }
    let trimmed = slug.trim_matches('_');
    if trimmed.is_empty() {
        "agent".to_string()
    } else {
        trimmed.to_string()
    }
}

// ============================================================================
// GR-001 POLICY FEATURE FLAGS
// ============================================================================
//
// NOTE: Policy toggle resolution has been centralized in
// `codex_spec_kit::config::PolicyToggles`. The functions below are thin
// wrappers that delegate to the canonical boundary. This ensures:
// - Single source of truth for env var parsing and precedence
// - Warn-once logic is handled centrally
// - Pure decision functions are unit-testable without env mutation

/// Check if legacy multi-agent consensus is enabled.
///
/// **REMOVED in PR6**: This function always returns `false`.
/// Legacy voting has been removed. The single-owner pipeline is now the only
/// supported mode.
///
/// If `SPEC_KIT_CONSENSUS` is set, `PolicyToggles::from_env_and_config()` will
/// emit a deprecation warning, but voting remains disabled.
///
/// See: docs/MODEL-POLICY.md Section 2 (GR-001)
#[deprecated(
    since = "0.1.0",
    note = "Legacy voting removed in PR6. Always returns false."
)]
pub fn is_consensus_enabled() -> bool {
    // PR6: Trigger the warning if env var is set, but always return false
    let _ = PolicyToggles::from_env_and_config();
    false
}

/// Check if critic-only sidecar is enabled (non-authoritative review).
///
/// Default: `false` (no critic).
/// Set `SPEC_KIT_SIDECAR_CRITIC=true` for non-blocking secondary review.
///
/// Critic outputs: risks, contradictions, missing requirements, guardrail conflicts.
/// Critic does NOT block progression or rewrite outputs.
///
/// **Note**: Delegates to `PolicyToggles::from_env_and_config()`.
///
/// See: docs/MODEL-POLICY.md Section 2 (Allowed Patterns)
pub fn is_critic_enabled() -> bool {
    PolicyToggles::from_env_and_config().sidecar_critic_enabled
}

/// Parse stage name from string (used by /spec-consensus command)
pub(in super::super) fn parse_consensus_stage(stage: &str) -> Option<SpecStage> {
    match stage.to_ascii_lowercase().as_str() {
        "plan" | "spec-plan" => Some(SpecStage::Plan),
        "tasks" | "spec-tasks" => Some(SpecStage::Tasks),
        "implement" | "spec-implement" => Some(SpecStage::Implement),
        "validate" | "spec-validate" => Some(SpecStage::Validate),
        "audit" | "review" | "spec-audit" | "spec-review" => Some(SpecStage::Audit),
        "unlock" | "spec-unlock" => Some(SpecStage::Unlock),
        "clarify" | "spec-clarify" => Some(SpecStage::Clarify),
        "analyze" | "spec-analyze" => Some(SpecStage::Analyze),
        "checklist" | "spec-checklist" => Some(SpecStage::Checklist),
        _ => None,
    }
}

/// Get the preferred single agent for a spec stage (GR-001 compliant).
///
/// This is the default behavior when `SPEC_KIT_CONSENSUS=false` (default).
/// Returns a single agent per stage based on the canonical pipeline:
/// - All stages except Implement: GPT Pro (architect/judge roles)
/// - Implement: GPT Codex (code generation)
///
/// SPEC-KIT-981: Changed defaults from Gemini/Claude to GPT.
pub fn preferred_agent_for_stage(stage: SpecStage) -> crate::spec_prompts::SpecAgent {
    use crate::spec_prompts::SpecAgent;
    match stage {
        // Implementer role → gpt_codex (code generation)
        SpecStage::Implement => SpecAgent::GptCodex,
        // All other roles → gpt_pro (architect/judge/planner)
        SpecStage::Specify
        | SpecStage::Plan
        | SpecStage::Tasks
        | SpecStage::Validate
        | SpecStage::Audit
        | SpecStage::Unlock
        | SpecStage::Clarify
        | SpecStage::Analyze
        | SpecStage::Checklist => SpecAgent::GptPro,
    }
}

/// Get the agent for a stage, checking config override first.
///
/// Resolution order:
/// 1. Config override (if valid agent name)
/// 2. Default from preferred_agent_for_stage()
///
/// SPEC-KIT-981: Config-aware agent selection.
pub fn agent_for_stage(
    stage: SpecStage,
    config: Option<&codex_core::config_types::SpecKitStageAgents>,
) -> crate::spec_prompts::SpecAgent {
    use crate::spec_prompts::SpecAgent;

    if let Some(cfg) = config {
        if let Some(agent_str) = cfg.get_agent_for_stage(stage.key()) {
            if let Some(agent) = SpecAgent::from_string(agent_str) {
                return agent;
            }
            tracing::warn!(
                "Invalid agent '{}' in config for stage {:?}, using default",
                agent_str,
                stage
            );
        }
    }
    preferred_agent_for_stage(stage)
}

/// Get expected agent roster for a spec stage.
///
/// **PR6**: Always returns a single preferred agent. Legacy multi-agent
/// voting has been removed.
///
/// **SPEC-KIT-981**: Now accepts optional config for stage→agent overrides.
/// If config specifies a valid agent for the stage, that agent is used;
/// otherwise falls back to `preferred_agent_for_stage()` defaults.
///
/// See: docs/MODEL-POLICY.md Section 2 (GR-001)
// ARCH-006: Use SpecAgent enum instead of strings
pub(in super::super) fn expected_agents_for_stage(
    stage: SpecStage,
    config: Option<&codex_core::config_types::SpecKitStageAgents>,
) -> Vec<crate::spec_prompts::SpecAgent> {
    // SPEC-KIT-981: Use config-aware agent selection
    vec![agent_for_stage(stage, config)]
}

/// Extract string array from JSON value
pub(in super::super) fn extract_string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

/// Validate that summary has required fields for the stage
pub(in super::super) fn validate_required_fields(stage: SpecStage, summary: &Value) -> bool {
    let obj = match summary.as_object() {
        Some(o) => o,
        None => return false,
    };

    // Common fields
    if !obj.contains_key("stage") || !obj.contains_key("agent") {
        return false;
    }

    // Stage-specific required fields
    match stage {
        SpecStage::Specify => obj.contains_key("prd_sections"),
        SpecStage::Plan => {
            obj.contains_key("work_breakdown") && obj.contains_key("acceptance_mapping")
        }
        SpecStage::Tasks => obj.contains_key("tasks"),
        SpecStage::Implement => obj.contains_key("implementation"),
        SpecStage::Validate => obj.contains_key("test_strategy"),
        SpecStage::Audit => obj.contains_key("audit_verdict"),
        SpecStage::Unlock => obj.contains_key("unlock_decision"),
        SpecStage::Clarify | SpecStage::Analyze => obj.contains_key("issues"),
        SpecStage::Checklist => obj.contains_key("requirements"),
    }
}

// ============================================================================
// CORE CONSENSUS LOGIC
// ============================================================================

use crate::local_memory_util::LocalMemorySearchResult;
use std::fs;
use std::path::Path;

/// Build consensus artifacts from cached agent responses (bypasses memory/file lookup)
#[allow(dead_code)] // Reserved for caching optimization path
pub(crate) fn artifacts_from_cached_responses(
    cached_responses: &[(String, String)],
    stage: SpecStage,
) -> Result<Vec<GateArtifactData>> {
    let mut artifacts = Vec::new();

    for (agent_name, response_text) in cached_responses {
        // Try to parse response as JSON (agents may output structured data)
        let content = match serde_json::from_str::<Value>(response_text) {
            Ok(json) => json,
            Err(_) => {
                // Not JSON, wrap as text content
                json!({
                    "agent": agent_name,
                    "stage": stage.command_name(),
                    "content": response_text
                })
            }
        };

        artifacts.push(GateArtifactData {
            memory_id: Some(format!("cached_{}", agent_name)),
            agent: agent_name.clone(),
            version: None,
            content,
        });
    }

    Ok(artifacts)
}

/// Collect consensus artifacts from SQLite (primary), local-memory, or evidence files (fallback)
// SPEC-KIT-072: SQLite is now primary source for consensus artifacts
pub(crate) async fn collect_consensus_artifacts(
    evidence_root: &Path,
    spec_id: &str,
    stage: SpecStage,
    mcp_manager: &codex_core::mcp_connection_manager::McpConnectionManager,
) -> Result<(Vec<GateArtifactData>, Vec<String>)> {
    let mut warnings: Vec<String> = Vec::new();

    // SPEC-KIT-072 Phase 3: SQLite is PRIMARY source (local-memory deprecated for artifacts)
    if let Ok(db) = super::consensus_db::ConsensusDb::init_default() {
        match db.query_artifacts(spec_id, stage) {
            Ok(sqlite_artifacts) if !sqlite_artifacts.is_empty() => {
                tracing::info!(
                    "✓ Loaded {} consensus artifacts from SQLite (primary source)",
                    sqlite_artifacts.len()
                );

                let mut artifacts = Vec::new();
                for artifact in sqlite_artifacts {
                    if let Ok(content) = serde_json::from_str::<Value>(&artifact.content_json) {
                        artifacts.push(GateArtifactData {
                            memory_id: Some(format!("sqlite_{}", artifact.id)),
                            agent: artifact.agent_name,
                            version: None,
                            content,
                        });
                    }
                }

                if !artifacts.is_empty() {
                    warnings.push(format!(
                        "Loaded {} artifacts from SQLite database",
                        artifacts.len()
                    ));
                    return Ok((artifacts, warnings));
                }
            }
            Ok(_) => {
                tracing::info!(
                    "No SQLite artifacts found for {} {}",
                    spec_id,
                    stage.command_name()
                );
                warnings.push(
                    "No artifacts in SQLite database (expected if agents just completed)"
                        .to_string(),
                );
            }
            Err(e) => {
                tracing::warn!("SQLite query failed: {}", e);
                warnings.push(format!("SQLite error: {}", e));
            }
        }
    } else {
        warnings
            .push("SQLite database initialization failed - check ~/.code/ permissions".to_string());
    }

    tracing::warn!("Falling back to local-memory (CLI/REST) for artifacts");

    match fetch_memory_entries(spec_id, stage, mcp_manager).await {
        Ok((entries, mut memory_warnings)) => {
            warnings.append(&mut memory_warnings);

            // Parse local-memory results into artifacts
            let mut artifacts: Vec<GateArtifactData> = Vec::new();

            for result in entries {
                let memory_id = result.memory.id.clone();
                let content_str = result.memory.content.trim();
                if content_str.is_empty() {
                    warnings.push("local-memory entry had empty content".to_string());
                    continue;
                }

                let value = match serde_json::from_str::<Value>(content_str) {
                    Ok(v) => v,
                    Err(err) => {
                        warnings.push(format!("unable to parse consensus artifact JSON: {err}"));
                        continue;
                    }
                };

                let agent = match value
                    .get("agent")
                    .or_else(|| value.get("model"))
                    .and_then(|v| v.as_str())
                {
                    Some(agent) if !agent.trim().is_empty() => agent.trim().to_string(),
                    _ => {
                        warnings.push("consensus artifact missing agent field".to_string());
                        continue;
                    }
                };

                let stage_matches = value
                    .get("stage")
                    .or_else(|| value.get("stage_name"))
                    .and_then(|v| v.as_str())
                    .and_then(parse_consensus_stage)
                    .map(|parsed| parsed == stage)
                    .unwrap_or(false);

                if !stage_matches {
                    warnings.push(format!(
                        "skipping local-memory entry for agent {} because stage did not match {}",
                        agent,
                        stage.command_name()
                    ));
                    continue;
                }

                let spec_matches = value
                    .get("spec_id")
                    .or_else(|| value.get("specId"))
                    .and_then(|v| v.as_str())
                    .map(|reported| reported.eq_ignore_ascii_case(spec_id))
                    .unwrap_or(true);

                if !spec_matches {
                    warnings.push(format!(
                        "skipping local-memory entry for agent {} because spec id did not match {}",
                        agent, spec_id
                    ));
                    continue;
                }

                let version = value
                    .get("prompt_version")
                    .or_else(|| value.get("promptVersion"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                artifacts.push(GateArtifactData {
                    memory_id,
                    agent,
                    version,
                    content: value,
                });
            }

            return Ok((artifacts, warnings));
        }
        Err(mcp_err) => {
            tracing::warn!(
                "local-memory fetch failed, falling back to file-based evidence: {}",
                mcp_err
            );
            warnings.push(format!(
                "⚠ Using file-based evidence (local-memory unavailable: {})",
                mcp_err
            ));

            match load_artifacts_from_evidence(evidence_root, spec_id, stage) {
                Ok(Some((artifacts, mut evidence_warnings))) => {
                    warnings.append(&mut evidence_warnings);
                    return Ok((artifacts, warnings));
                }
                Ok(None) => {
                    warnings.push("No file-based evidence found either".to_string());
                }
                Err(err) => {
                    warnings.push(format!("File-based evidence also failed: {}", err));
                }
            }
        }
    }

    // If both local-memory and file-based evidence failed
    Err(SpecKitError::NoConsensusFound {
        spec_id: spec_id.to_string(),
        stage: stage.command_name().to_string(),
        directory: evidence_root.to_path_buf(),
    })
}

fn load_artifacts_from_evidence(
    evidence_root: &Path,
    spec_id: &str,
    stage: SpecStage,
) -> Result<Option<(Vec<GateArtifactData>, Vec<String>)>> {
    let consensus_dir = evidence_root.join(spec_id);
    if !consensus_dir.exists() {
        return Ok(None);
    }

    let stage_prefix = format!("{}_", stage.command_name());
    let suffix = "_artifact.json";

    let entries = fs::read_dir(&consensus_dir).map_err(|e| {
        format!(
            "Failed to read consensus evidence directory {}: {}",
            consensus_dir.display(),
            e
        )
    })?;

    let mut artifacts: Vec<GateArtifactData> = Vec::new();
    let warnings: Vec<String> = Vec::new();

    for entry_res in entries {
        let entry = entry_res.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with(&stage_prefix) || !name.ends_with(suffix) {
            continue;
        }

        let contents = fs::read_to_string(&path).map_err(|e| {
            format!(
                "Failed to read consensus artifact {}: {}",
                path.display(),
                e
            )
        })?;

        let value: Value = serde_json::from_str(&contents).map_err(|e| {
            format!(
                "Failed to parse consensus artifact JSON {}: {}",
                path.display(),
                e
            )
        })?;

        let agent = value
            .get("agent")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let version = value
            .get("prompt_version")
            .or_else(|| value.get("promptVersion"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        artifacts.push(GateArtifactData {
            memory_id: None,
            agent,
            version,
            content: value,
        });
    }

    if artifacts.is_empty() {
        Ok(None)
    } else {
        Ok(Some((artifacts, warnings)))
    }
}

// FORK-SPECIFIC (just-every/code): Native MCP for local-memory
// SPEC-KIT-964 Phase 7: Project scoping for hermetic isolation
async fn fetch_memory_entries(
    spec_id: &str,
    stage: SpecStage,
    _mcp_manager: &codex_core::mcp_connection_manager::McpConnectionManager,
) -> Result<(Vec<LocalMemorySearchResult>, Vec<String>)> {
    let query = format!("{} {}", spec_id, stage.command_name());
    // Note: Agents may tag with either "stage:plan" or "stage:spec-plan"
    // Currently using query-based search; tag filtering available if needed:
    // - format!("stage:{}", stage.display_name().to_lowercase()) e.g., "stage:plan"
    // - format!("stage:{}", stage.command_name()) e.g., "stage:spec-plan"

    // SPEC-KIT-964 Phase 7: Derive project tag from current working directory
    // This scopes memory queries to the current project, preventing cross-project leakage
    let project_tag = std::env::current_dir()
        .ok()
        .and_then(|p| {
            // Extract project identifier: last component of path or git remote
            p.file_name()
                .map(|name| format!("project:{}", name.to_string_lossy()))
        })
        .unwrap_or_else(|| "project:unknown".to_string());

    if !crate::local_memory_cli::local_memory_daemon_healthy(std::time::Duration::from_millis(750))
        .await
    {
        return Err(SpecKitError::from_string(
            "local-memory daemon not available at http://localhost:3002".to_string(),
        ));
    }

    let tags = vec![format!("spec:{spec_id}"), project_tag];
    let results = crate::local_memory_cli::search(&query, 20, &tags, None, 50_000)
        .await
        .map_err(|e| SpecKitError::from_string(format!("local-memory search failed: {}", e)))?;

    if results.is_empty() {
        Err(SpecKitError::NoConsensusFound {
            spec_id: spec_id.to_string(),
            stage: stage.command_name().to_string(),
            directory: std::path::PathBuf::from("local-memory"),
        })
    } else {
        Ok((results, Vec::new()))
    }
}

/// Load latest consensus synthesis file for spec/stage
pub(crate) fn load_latest_consensus_synthesis(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
) -> Result<Option<StageReviewSummary>> {
    // MAINT-7: Use centralized path helper (dynamic per spec_id)
    let base = super::evidence::consensus_dir_for_spec(cwd, spec_id);
    if !base.exists() {
        return Ok(None);
    }

    let stage_prefix = format!("{}_", stage.command_name());
    let suffix = "_synthesis.json";

    let mut candidates: Vec<PathBuf> = fs::read_dir(&base)
        .map_err(|e| {
            format!(
                "Failed to read consensus synthesis directory {}: {}",
                base.display(),
                e
            )
        })?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(&stage_prefix) && name.ends_with(suffix) {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    if candidates.is_empty() {
        return Ok(None);
    }

    candidates.sort();
    let latest_path = candidates.pop().unwrap();

    let contents = fs::read_to_string(&latest_path).map_err(|e| {
        format!(
            "Failed to read consensus synthesis {}: {}",
            latest_path.display(),
            e
        )
    })?;

    let raw: StageReviewRaw = serde_json::from_str(&contents).map_err(|e| {
        format!(
            "Failed to parse consensus synthesis {}: {}",
            latest_path.display(),
            e
        )
    })?;

    if let Some(raw_stage) = raw.stage.as_deref()
        && raw_stage != stage.command_name()
    {
        return Err(format!(
            "Consensus synthesis stage mismatch: expected {}, found {}",
            stage.command_name(),
            raw_stage
        )
        .into());
    }

    if let Some(raw_spec) = raw.spec_id.as_deref()
        && !raw_spec.eq_ignore_ascii_case(spec_id)
    {
        return Err(format!(
            "Consensus synthesis spec mismatch: expected {}, found {}",
            spec_id, raw_spec
        )
        .into());
    }

    Ok(Some(StageReviewSummary {
        status: raw.status,
        missing_agents: raw.missing_agents,
        agreements: raw.consensus.agreements,
        conflicts: raw.consensus.conflicts,
        prompt_version: raw.prompt_version,
        path: latest_path,
    }))
}

use std::collections::HashSet;

// FORK-SPECIFIC (just-every/code): Made async for native MCP
/// Run consensus check for spec stage.
///
/// **GR-001 Compliance**: When `SPEC_KIT_CONSENSUS=false` (default), this function
/// returns early with `consensus_ok=true` and skips multi-agent validation.
///
/// **PR6**: Multi-agent consensus has been removed. This function now always
/// returns early with `consensus_ok=true`. The legacy code path below is retained
/// for reference but is unreachable.
#[allow(deprecated)] // is_consensus_enabled() is deprecated
pub async fn run_spec_consensus(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    telemetry_enabled: bool,
    mcp_manager: &codex_core::mcp_connection_manager::McpConnectionManager,
) -> Result<(Vec<ratatui::text::Line<'static>>, bool)> {
    // GR-001: Skip consensus when disabled (default behavior)
    if !is_consensus_enabled() {
        let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
        lines.push(ratatui::text::Line::from(format!(
            "[Stage Review] {} {} — SKIPPED (GR-001: single-owner pipeline)",
            stage.display_name(),
            spec_id
        )));
        lines.push(ratatui::text::Line::from(
            "  Quality enforced by: compiler/tests, constitution gates, Judge audit",
        ));

        // If critic mode enabled, note it
        if is_critic_enabled() {
            lines.push(ratatui::text::Line::from(
                "  Critic sidecar: ENABLED (non-blocking review)",
            ));
        }

        // Always return consensus_ok=true when consensus is disabled
        return Ok((lines, true));
    }

    // Legacy multi-agent consensus (DEPRECATED)
    // Warning is emitted by PolicyToggles::from_env_and_config()

    // MAINT-7: Use centralized path helper
    let evidence_root = super::evidence::consensus_dir(cwd);

    let (artifacts, mut warnings) =
        collect_consensus_artifacts(&evidence_root, spec_id, stage, mcp_manager).await?;
    if artifacts.is_empty() {
        return Err(format!(
            "No structured local-memory entries found for {} stage '{}'. Ensure agents stored their JSON via local-memory remember.",
            spec_id,
            stage.command_name()
        ).into());
    }

    let synthesis_summary = match load_latest_consensus_synthesis(cwd, spec_id, stage) {
        Ok(summary) => summary,
        Err(err) => {
            warnings.push(format!("Failed to load consensus synthesis: {}", err));
            None
        }
    };

    let mut present_agents: HashSet<String> = HashSet::new();
    let mut aggregator_summary: Option<Value> = None;
    let mut aggregator_version: Option<String> = None;
    let mut aggregator_agent: Option<String> = None;
    let mut agreements: Vec<String> = Vec::new();
    let mut conflicts: Vec<String> = Vec::new();
    let mut required_fields_ok = false;

    for artifact in &artifacts {
        // ARCH-006: Use SpecAgent enum for type safety
        let agent_enum = crate::spec_prompts::SpecAgent::from_string(&artifact.agent);
        if let Some(agent) = agent_enum {
            present_agents.insert(agent.canonical_name().to_string());

            if matches!(agent, crate::spec_prompts::SpecAgent::GptPro) {
                let consensus_node = artifact
                    .content
                    .get("consensus")
                    .cloned()
                    .unwrap_or(Value::Null);
                agreements = extract_string_list(consensus_node.get("agreements"));
                conflicts = extract_string_list(consensus_node.get("conflicts"));
                required_fields_ok = validate_required_fields(stage, &artifact.content);
                aggregator_summary = Some(artifact.content.clone());
                aggregator_version = artifact.version.clone();
                aggregator_agent = Some(artifact.agent.clone());
            }
        } else {
            // Unknown agent name - keep old behavior (insert as-is)
            present_agents.insert(artifact.agent.to_ascii_lowercase());
        }
    }

    // ARCH-006: Use SpecAgent enum for expected agents
    // Note: Legacy consensus path - no config available, use defaults
    let expected_agents = expected_agents_for_stage(stage, None);
    let mut missing_agents: Vec<String> = expected_agents
        .iter()
        .map(|agent| agent.canonical_name().to_string())
        .filter(|agent| !present_agents.contains(agent))
        .collect();

    if aggregator_summary.is_none() {
        required_fields_ok = false;
    }

    let mut synthesis_evidence_path: Option<PathBuf> = None;
    let mut prompt_version =
        crate::spec_prompts::stage_version_enum(stage).unwrap_or_else(|| "unversioned".to_string());
    let has_conflict;
    let degraded;
    let consensus_ok;

    if let Some(summary) = &synthesis_summary {
        synthesis_evidence_path = Some(summary.path.clone());
        if let Some(version) = &summary.prompt_version
            && !version.trim().is_empty()
        {
            prompt_version = version.clone();
        }
        agreements = summary.agreements.clone();
        conflicts = summary.conflicts.clone();
        missing_agents = summary.missing_agents.clone();
        has_conflict = summary.status.eq_ignore_ascii_case("conflict") || !conflicts.is_empty();
        degraded = summary.status.eq_ignore_ascii_case("degraded")
            || (!missing_agents.is_empty() && !has_conflict);
        consensus_ok = summary.status.eq_ignore_ascii_case("ok");
    } else {
        has_conflict = !conflicts.is_empty();
        degraded = aggregator_summary.is_none() || !missing_agents.is_empty();
        consensus_ok = aggregator_summary.is_some()
            && conflicts.is_empty()
            && missing_agents.is_empty()
            && required_fields_ok;
    }

    let consensus_ok = consensus_ok;
    let has_conflict = if consensus_ok { false } else { has_conflict };
    let degraded = if consensus_ok { false } else { degraded };

    missing_agents.sort_unstable();
    missing_agents.dedup();
    conflicts.sort_unstable();
    conflicts.dedup();

    let consensus_status = if consensus_ok {
        "ok"
    } else if has_conflict {
        "conflict"
    } else if degraded {
        "degraded"
    } else {
        "unknown"
    };
    let consensus_status = consensus_status.to_string();

    let mut lines: Vec<ratatui::text::Line<'static>> = Vec::new();
    let status_label = if consensus_ok {
        "REVIEW OK"
    } else if has_conflict {
        "REVIEW CONFLICT"
    } else if degraded {
        "REVIEW DEGRADED"
    } else {
        "REVIEW UNKNOWN"
    };
    lines.push(ratatui::text::Line::from(format!(
        "[Stage Review] {} {} — {}",
        stage.display_name(),
        spec_id,
        status_label
    )));
    lines.push(ratatui::text::Line::from(format!(
        "  Prompt version: {}",
        prompt_version
    )));

    for warning in warnings.drain(..) {
        lines.push(ratatui::text::Line::from(format!("  Warning: {warning}")));
    }

    if let Some(path) = synthesis_evidence_path.as_ref() {
        lines.push(ratatui::text::Line::from(format!(
            "  Synthesis: {}",
            path.display()
        )));
    }

    if !missing_agents.is_empty() {
        lines.push(ratatui::text::Line::from(format!(
            "  Missing agents: {}",
            missing_agents.join(", ")
        )));
    }

    if aggregator_summary.is_none() {
        lines.push(ratatui::text::Line::from(
            "  Aggregator (gpt_pro) summary not found in local-memory.",
        ));
    }

    if !agreements.is_empty() {
        lines.push(ratatui::text::Line::from(format!(
            "  Agreements: {}",
            agreements.join("; ")
        )));
    }

    if !conflicts.is_empty() {
        lines.push(ratatui::text::Line::from(format!(
            "  Conflicts: {}",
            conflicts.join("; ")
        )));
    }

    lines.push(ratatui::text::Line::from(format!(
        "  Artifacts: {} agent(s)",
        present_agents.len()
    )));

    // Persistence: Write verdict, telemetry, and remember in local-memory
    if telemetry_enabled {
        let evidence_slug = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let verdict_obj = StageReviewVerdict {
            spec_id: spec_id.to_string(),
            stage: stage.command_name().to_string(),
            recorded_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            prompt_version: Some(prompt_version.clone()),
            consensus_ok,
            degraded,
            required_fields_ok,
            missing_agents: missing_agents.clone(),
            agreements: agreements.clone(),
            conflicts: conflicts.clone(),
            aggregator_agent: aggregator_agent.clone(),
            aggregator_version: aggregator_version.clone(),
            aggregator: aggregator_summary.clone(),
            synthesis_path: synthesis_evidence_path
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned()),
            artifacts: artifacts
                .iter()
                .map(|artifact| GateArtifactVerdict {
                    memory_id: artifact.memory_id.clone(),
                    agent: artifact.agent.clone(),
                    version: artifact.version.clone(),
                    content: artifact.content.clone(),
                })
                .collect(),
        };

        match persist_consensus_verdict(cwd, spec_id, stage, &verdict_obj) {
            Ok(verdict_path) => {
                lines.push(ratatui::text::Line::from(format!(
                    "  Evidence: {}",
                    verdict_path.display()
                )));

                let verdict_handle = GateEvidenceHandle {
                    path: verdict_path.clone(),
                    sha256: String::new(), // Not computing hash for now
                };

                // Persist telemetry bundle
                match persist_consensus_telemetry_bundle(
                    cwd,
                    spec_id,
                    stage,
                    &verdict_obj,
                    &verdict_handle,
                    &evidence_slug,
                    &consensus_status,
                ) {
                    Ok(_paths) => {
                        // Success - telemetry written
                    }
                    Err(err) => {
                        lines.push(ratatui::text::Line::from(format!(
                            "  Warning: failed to persist telemetry bundle: {}",
                            err
                        )));
                    }
                }

                // Remember in local-memory
                if let Err(err) =
                    remember_consensus_verdict(spec_id, stage, &verdict_obj, mcp_manager).await
                {
                    lines.push(ratatui::text::Line::from(format!(
                        "  Warning: failed to store in local-memory: {}",
                        err
                    )));
                }
            }
            Err(err) => {
                lines.push(ratatui::text::Line::from(format!(
                    "  Warning: failed to persist consensus evidence: {}",
                    err
                )));
            }
        }
    }

    Ok((lines, consensus_ok))
}

use std::io::Write;

/// Persist consensus verdict to evidence directory
pub(crate) fn persist_consensus_verdict(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    verdict: &StageReviewVerdict,
) -> Result<PathBuf> {
    // MAINT-7: Use centralized path helper (dynamic per spec_id)
    let consensus_dir = super::evidence::consensus_dir_for_spec(cwd, spec_id);
    fs::create_dir_all(&consensus_dir)
        .map_err(|e| format!("Failed to create consensus directory: {e}"))?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H_%M_%S%.3fZ");
    let filename = format!("{}_{}_verdict.json", stage.command_name(), timestamp);
    let path = consensus_dir.join(&filename);

    let json = serde_json::to_string_pretty(verdict)
        .map_err(|e| format!("Failed to serialize verdict: {e}"))?;

    let mut file =
        fs::File::create(&path).map_err(|e| format!("Failed to create verdict file: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write verdict: {e}"))?;

    Ok(path)
}

/// Persist consensus telemetry bundle with artifacts
pub(crate) fn persist_consensus_telemetry_bundle(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    verdict: &StageReviewVerdict,
    verdict_handle: &GateEvidenceHandle,
    slug: &str,
    consensus_status: &str,
) -> Result<GateTelemetryPaths> {
    // MAINT-7: Use centralized path helper (dynamic per spec_id)
    let base = super::evidence::consensus_dir_for_spec(cwd, spec_id);
    fs::create_dir_all(&base).map_err(|e| {
        format!(
            "failed to create consensus evidence directory {}: {}",
            base.display(),
            e
        )
    })?;

    let stage_name = stage.command_name();

    let to_relative = |path: &Path| -> String {
        path.strip_prefix(cwd)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned()
    };

    // Write individual agent artifacts
    let mut agent_paths: Vec<PathBuf> = Vec::new();
    for artifact in &verdict.artifacts {
        let agent_slug = telemetry_agent_slug(&artifact.agent);
        let filename = format!("{}_{agent_slug}_{slug}_artifact.json", stage_name);
        let agent_path = base.join(&filename);

        let json = serde_json::to_string_pretty(&artifact.content)
            .map_err(|e| format!("Failed to serialize agent artifact: {e}"))?;

        let mut file = fs::File::create(&agent_path)
            .map_err(|e| format!("Failed to create agent artifact file: {e}"))?;
        file.write_all(json.as_bytes())
            .map_err(|e| format!("Failed to write agent artifact: {e}"))?;

        agent_paths.push(agent_path);
    }

    // Write consensus telemetry bundle
    let telemetry_filename = format!("{}_{slug}_telemetry.json", stage_name);
    let telemetry_path = base.join(&telemetry_filename);

    let telemetry_bundle = serde_json::json!({
        "spec_id": spec_id,
        "stage": stage_name,
        "slug": slug,
        "consensus_status": consensus_status,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "verdict": to_relative(&verdict_handle.path),
        "artifacts": agent_paths.iter().map(|p| to_relative(p)).collect::<Vec<_>>(),
    });

    let json = serde_json::to_string_pretty(&telemetry_bundle)
        .map_err(|e| format!("Failed to serialize telemetry bundle: {e}"))?;

    let mut file = fs::File::create(&telemetry_path)
        .map_err(|e| format!("Failed to create telemetry file: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write telemetry: {e}"))?;

    // Write synthesis metadata
    let synthesis_filename = format!("{}_{slug}_synthesis.json", stage_name);
    let synthesis_path = base.join(&synthesis_filename);

    let synthesis_data = serde_json::json!({
        "spec_id": spec_id,
        "stage": stage_name,
        "status": consensus_status,
        "missing_agents": verdict.missing_agents,
        "consensus": {
            "agreements": verdict.agreements,
            "conflicts": verdict.conflicts,
        },
        "prompt_version": verdict.prompt_version,
    });

    let json = serde_json::to_string_pretty(&synthesis_data)
        .map_err(|e| format!("Failed to serialize synthesis: {e}"))?;

    let mut file = fs::File::create(&synthesis_path)
        .map_err(|e| format!("Failed to create synthesis file: {e}"))?;
    file.write_all(json.as_bytes())
        .map_err(|e| format!("Failed to write synthesis: {e}"))?;

    Ok(GateTelemetryPaths {
        agent_paths,
        telemetry_path,
        synthesis_path,
    })
}

/// Remember consensus verdict in SQLite (SPEC-934)
///
/// Replaces MCP local-memory storage with SQLite consensus_db.
// FORK-SPECIFIC (just-every/code): Made async for native MCP, now async for SQLite
pub(crate) async fn remember_consensus_verdict(
    spec_id: &str,
    stage: SpecStage,
    verdict: &StageReviewVerdict,
    _mcp_manager: &codex_core::mcp_connection_manager::McpConnectionManager,
) -> Result<()> {
    let mut summary_value = serde_json::json!({
        "spec_id": spec_id,
        "stage": stage.command_name(),
        "status": if verdict.consensus_ok {
            "ok"
        } else if !verdict.conflicts.is_empty() {
            "conflict"
        } else {
            "degraded"
        },
        "missing_agents": verdict.missing_agents,
        "agreements": verdict.agreements,
        "conflicts": verdict.conflicts,
    });

    if let Some(version) = &verdict.prompt_version
        && let serde_json::Value::Object(obj) = &mut summary_value
    {
        obj.insert(
            "promptVersion".to_string(),
            serde_json::Value::String(version.clone()),
        );
    }

    if let Some(path) = &verdict.synthesis_path
        && let serde_json::Value::Object(obj) = &mut summary_value
    {
        obj.insert(
            "synthesisPath".to_string(),
            serde_json::Value::String(path.clone()),
        );
    }

    let summary = serde_json::to_string(&summary_value)
        .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

    // SPEC-934: Store to SQLite instead of MCP local-memory
    let db = super::consensus_db::ConsensusDb::init_default().map_err(|e| {
        SpecKitError::from_string(format!("Failed to initialize consensus DB: {}", e))
    })?;

    // Store as artifact with special agent name to distinguish from agent outputs
    db.store_artifact(
        spec_id,
        stage,
        "consensus-verdict",
        &summary,
        None, // response_text
        None, // run_id
    )
    .map_err(|e| SpecKitError::from_string(format!("SQLite storage failed: {}", e)))?;

    tracing::debug!(
        "Stored consensus verdict to SQLite: spec={}, stage={}",
        spec_id,
        stage.command_name()
    );

    Ok(())
}

// ============================================================================
// INTEGRATION TESTS FOR GR-001 FEATURE FLAGS
// ============================================================================
//
// NOTE: The pure decision logic is unit-tested in
// `codex_spec_kit::config::policy_toggles::tests`. These tests verify the
// env -> wrapper function delegation works correctly.

#[cfg(test)]
mod gr001_tests {
    use super::*;
    use serial_test::serial;

    // Note: Rust 2024 edition requires unsafe for env::set_var/remove_var
    // SAFETY: These tests run sequentially and only modify test-specific env vars
    //
    // Tests that modify env vars are marked with #[serial] to prevent race conditions.

    #[test]
    #[allow(deprecated)]
    fn test_consensus_disabled_by_default() {
        // Clear any env var that might be set
        // SAFETY: Test isolation
        unsafe { std::env::remove_var("SPEC_KIT_CONSENSUS") };

        // Default behavior: consensus disabled (always, after PR6)
        assert!(!is_consensus_enabled());
    }

    #[test]
    #[allow(deprecated)]
    fn test_consensus_always_disabled_pr6() {
        // PR6: Even when env var is set to "true", consensus is always disabled
        // SAFETY: Test isolation
        unsafe {
            std::env::set_var("SPEC_KIT_CONSENSUS", "true");
        }
        assert!(!is_consensus_enabled(), "PR6: voting always disabled");

        unsafe {
            std::env::set_var("SPEC_KIT_CONSENSUS", "1");
        }
        assert!(!is_consensus_enabled(), "PR6: voting always disabled");

        unsafe {
            std::env::set_var("SPEC_KIT_CONSENSUS", "false");
        }
        assert!(!is_consensus_enabled(), "PR6: voting always disabled");

        // Cleanup
        unsafe { std::env::remove_var("SPEC_KIT_CONSENSUS") };
    }

    #[test]
    #[serial]
    fn test_critic_disabled_by_default() {
        // SAFETY: Test isolation - clear both env vars
        unsafe {
            std::env::remove_var("SPEC_KIT_CRITIC");
            std::env::remove_var("SPEC_KIT_SIDECAR_CRITIC");
        }
        assert!(!is_critic_enabled());
    }

    #[test]
    #[serial]
    fn test_critic_enabled_canonical_var() {
        // SAFETY: Test isolation
        unsafe {
            std::env::remove_var("SPEC_KIT_CRITIC");
            std::env::set_var("SPEC_KIT_SIDECAR_CRITIC", "true");
        }
        assert!(is_critic_enabled());

        // Cleanup
        unsafe { std::env::remove_var("SPEC_KIT_SIDECAR_CRITIC") };
    }

    #[test]
    #[serial]
    fn test_critic_enabled_deprecated_var() {
        // SAFETY: Test isolation
        unsafe {
            std::env::remove_var("SPEC_KIT_SIDECAR_CRITIC");
            std::env::set_var("SPEC_KIT_CRITIC", "true");
        }
        // Deprecated var still works (with warning)
        assert!(is_critic_enabled());

        // Cleanup
        unsafe { std::env::remove_var("SPEC_KIT_CRITIC") };
    }

    #[test]
    #[serial]
    fn test_critic_canonical_wins_over_deprecated() {
        // SAFETY: Test isolation
        unsafe {
            std::env::set_var("SPEC_KIT_SIDECAR_CRITIC", "false");
            std::env::set_var("SPEC_KIT_CRITIC", "true");
        }
        // Canonical (false) wins over deprecated (true)
        assert!(!is_critic_enabled());

        // Cleanup
        unsafe {
            std::env::remove_var("SPEC_KIT_SIDECAR_CRITIC");
            std::env::remove_var("SPEC_KIT_CRITIC");
        }
    }

    #[test]
    fn test_always_single_agent_pr6() {
        // PR6: Always returns single agent, even if env var is set
        // SAFETY: Test isolation
        unsafe { std::env::remove_var("SPEC_KIT_CONSENSUS") };

        let agents = expected_agents_for_stage(SpecStage::Implement, None);
        assert_eq!(agents.len(), 1, "PR6: always single agent");

        // Even with env var set, still single agent
        unsafe {
            std::env::set_var("SPEC_KIT_CONSENSUS", "true");
        }

        let agents = expected_agents_for_stage(SpecStage::Implement, None);
        assert_eq!(
            agents.len(),
            1,
            "PR6: always single agent even with env var"
        );

        // Cleanup
        unsafe { std::env::remove_var("SPEC_KIT_CONSENSUS") };
    }

    #[test]
    fn test_preferred_agent_for_stages() {
        use crate::spec_prompts::SpecAgent;

        // SPEC-KIT-981: All stages use GptPro except Implement which uses GptCodex
        // Architect roles use GptPro
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Specify),
            SpecAgent::GptPro
        );
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Plan),
            SpecAgent::GptPro
        );

        // Implementer uses GptCodex
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Implement),
            SpecAgent::GptCodex
        );

        // Other stages use GptPro
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Tasks),
            SpecAgent::GptPro
        );
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Validate),
            SpecAgent::GptPro
        );
        assert_eq!(
            preferred_agent_for_stage(SpecStage::Audit),
            SpecAgent::GptPro
        );
    }

    // =========================================================================
    // SPEC-KIT-981: Config-aware agent selection tests
    // =========================================================================

    #[test]
    fn test_agent_for_stage_uses_default_when_no_config() {
        use crate::spec_prompts::SpecAgent;

        // Without config, should use GPT defaults
        let agent = agent_for_stage(SpecStage::Plan, None);
        assert_eq!(agent, SpecAgent::GptPro);

        let agent = agent_for_stage(SpecStage::Implement, None);
        assert_eq!(agent, SpecAgent::GptCodex);
    }

    #[test]
    fn test_agent_for_stage_uses_config_override() {
        use crate::spec_prompts::SpecAgent;
        use codex_core::config_types::SpecKitStageAgents;

        let mut config = SpecKitStageAgents::default();
        config.plan = Some("claude".to_string());

        let agent = agent_for_stage(SpecStage::Plan, Some(&config));
        assert_eq!(agent, SpecAgent::Claude);

        // Implement should still use default if not overridden
        let agent = agent_for_stage(SpecStage::Implement, Some(&config));
        assert_eq!(agent, SpecAgent::GptCodex);
    }

    #[test]
    fn test_agent_for_stage_invalid_config_falls_back() {
        use crate::spec_prompts::SpecAgent;
        use codex_core::config_types::SpecKitStageAgents;

        let mut config = SpecKitStageAgents::default();
        config.plan = Some("invalid_agent_name".to_string());

        // Invalid agent should fall back to default
        let agent = agent_for_stage(SpecStage::Plan, Some(&config));
        assert_eq!(agent, SpecAgent::GptPro);
    }

    #[test]
    fn test_expected_agents_for_stage_with_config() {
        use crate::spec_prompts::SpecAgent;
        use codex_core::config_types::SpecKitStageAgents;

        let mut config = SpecKitStageAgents::default();
        config.validate = Some("gemini".to_string());

        let agents = expected_agents_for_stage(SpecStage::Validate, Some(&config));
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0], SpecAgent::Gemini);
    }

    // =========================================================================
    // PR7: Golden Evidence Tests - Wire Format Stability
    // =========================================================================

    /// Golden test: Ensure StageReviewVerdict serializes with LEGACY key names.
    /// This prevents accidental wire format changes that would break evidence files.
    #[test]
    fn test_consensus_verdict_serializes_legacy_keys() {
        let verdict = StageReviewVerdict {
            spec_id: "SPEC-GOLDEN-001".to_string(),
            stage: "plan".to_string(),
            recorded_at: "2025-01-01T00:00:00Z".to_string(),
            prompt_version: Some("v1.0".to_string()),
            consensus_ok: true, // Legacy field name
            degraded: false,
            required_fields_ok: true,
            missing_agents: vec![],
            agreements: vec!["agreement1".to_string()],
            conflicts: vec![],
            aggregator_agent: Some("gemini".to_string()),
            aggregator_version: Some("1.5".to_string()),
            aggregator: None,
            synthesis_path: Some("/evidence/synthesis.json".to_string()),
            artifacts: vec![],
        };

        let json = serde_json::to_string(&verdict).expect("serialize");

        // CRITICAL: Assert JSON uses LEGACY key name "consensus_ok"
        // This is the wire format contract - do NOT change without migration
        assert!(
            json.contains("\"consensus_ok\":true"),
            "Wire format MUST use legacy key 'consensus_ok', got: {}",
            json
        );

        // Should NOT contain new key names in serialized output
        assert!(
            !json.contains("\"gate_ok\""),
            "Wire format should NOT use 'gate_ok' (alias is for reading only)"
        );
        assert!(
            !json.contains("\"review_ok\""),
            "Wire format should NOT use 'review_ok' (alias is for reading only)"
        );
    }

    /// Golden test: Ensure StageReviewVerdict deserializes NEW key names via aliases.
    /// This enables forward compatibility - new evidence writers can use new names.
    #[test]
    fn test_consensus_verdict_deserializes_new_keys() {
        // JSON using new key name "gate_ok" (via serde alias)
        let json_gate_ok = r#"{
            "spec_id": "SPEC-GOLDEN-002",
            "stage": "plan",
            "recorded_at": "2025-01-01T00:00:00Z",
            "gate_ok": true,
            "degraded": false,
            "required_fields_ok": true,
            "missing_agents": [],
            "agreements": [],
            "conflicts": [],
            "artifacts": []
        }"#;

        let verdict: StageReviewVerdict =
            serde_json::from_str(json_gate_ok).expect("deserialize with gate_ok alias");
        assert!(
            verdict.consensus_ok,
            "gate_ok alias should map to consensus_ok"
        );

        // JSON using alternate new key name "review_ok" (via serde alias)
        let json_review_ok = r#"{
            "spec_id": "SPEC-GOLDEN-003",
            "stage": "plan",
            "recorded_at": "2025-01-01T00:00:00Z",
            "review_ok": false,
            "degraded": true,
            "required_fields_ok": true,
            "missing_agents": ["claude"],
            "agreements": [],
            "conflicts": ["conflict1"],
            "artifacts": []
        }"#;

        let verdict: StageReviewVerdict =
            serde_json::from_str(json_review_ok).expect("deserialize with review_ok alias");
        assert!(
            !verdict.consensus_ok,
            "review_ok alias should map to consensus_ok"
        );
        assert!(verdict.degraded);
    }

    /// Golden test: Legacy JSON (with consensus_ok) still deserializes correctly.
    #[test]
    fn test_consensus_verdict_deserializes_legacy_keys() {
        let json_legacy = r#"{
            "spec_id": "SPEC-GOLDEN-004",
            "stage": "implement",
            "recorded_at": "2024-12-01T00:00:00Z",
            "consensus_ok": true,
            "degraded": false,
            "required_fields_ok": true,
            "missing_agents": [],
            "agreements": ["all_good"],
            "conflicts": [],
            "artifacts": []
        }"#;

        let verdict: StageReviewVerdict =
            serde_json::from_str(json_legacy).expect("deserialize legacy format");
        assert!(verdict.consensus_ok);
        assert_eq!(verdict.spec_id, "SPEC-GOLDEN-004");
    }
}
