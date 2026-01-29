//! WP-A: "Filesystem Is Projection" Rebuild Command
//!
//! Regenerates filesystem projections (docs/, memory/) from capsule (SoR) and
//! OverlayDb (vision constitution memories). This implements the "filesystem is
//! projection" contract - capsule is SoR, filesystem artifacts are rebuildable.
//!
//! ## Sources
//! - **Spec/Project projections**: Capsule IntakeCompleted events
//! - **Vision projections**: OverlayDb constitution memories
//!
//! ## Non-Goals
//! - Rebuild does NOT write to capsule (read-only operation)
//! - Vision target_users/problem_statement are NOT recoverable from OverlayDb
//!   (only constitution memories are stored there)

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::memvid_adapter::{
    CapsuleConfig, CapsuleHandle, DEFAULT_CAPSULE_RELATIVE_PATH, DEFAULT_WORKSPACE_ID, EventType,
    IntakeCompletedPayload, IntakeKind,
};

use super::error::SpecKitError;
use super::intake::{DesignBrief, ProjectBrief};
use super::intake_core::{
    CapsulePersistenceResult, ProjectCapsulePersistenceResult,
    create_project_filesystem_projection, create_spec_filesystem_projections,
};

// =============================================================================
// Request/Response Types
// =============================================================================

/// Request for rebuild operation.
#[derive(Debug, Clone, Default)]
pub struct RebuildRequest {
    /// Specific spec ID to rebuild (None = all specs from latest intakes)
    pub spec_id: Option<String>,
    /// Specific project ID to rebuild (None = all projects from latest intakes)
    pub project_id: Option<String>,
    /// Include vision rebuild from OverlayDb (default: true)
    pub include_vision: bool,
    /// Dry-run mode: list files without writing (default: false)
    pub dry_run: bool,
}

impl RebuildRequest {
    pub fn new() -> Self {
        Self {
            spec_id: None,
            project_id: None,
            include_vision: true,
            dry_run: false,
        }
    }

    pub fn with_spec(mut self, spec_id: String) -> Self {
        self.spec_id = Some(spec_id);
        self
    }

    pub fn with_project(mut self, project_id: String) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn no_vision(mut self) -> Self {
        self.include_vision = false;
        self
    }

    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// Information about a spec intake event used during rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecIntakeInfo {
    pub spec_id: String,
    pub intake_id: String,
    pub event_uri: String,
    pub answers_uri: String,
    pub brief_uri: String,
    pub deep: bool,
}

/// Information about a project intake event used during rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIntakeInfo {
    pub project_id: String,
    pub intake_id: String,
    pub event_uri: String,
    pub answers_uri: String,
    pub brief_uri: String,
    pub deep: bool,
}

/// Result from rebuild operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RebuildResult {
    /// Files that were written (or would be in dry-run)
    pub files_written: Vec<PathBuf>,
    /// Source URIs that were read from capsule
    pub source_uris: Vec<String>,
    /// Spec intake events that were processed
    pub spec_intakes: Vec<SpecIntakeInfo>,
    /// Project intake events that were processed
    pub project_intakes: Vec<ProjectIntakeInfo>,
    /// Whether vision was rebuilt
    pub vision_rebuilt: bool,
    /// Vision rebuild details (if rebuilt)
    pub vision_details: Option<VisionRebuildDetails>,
    /// Whether this was a dry-run
    pub dry_run: bool,
}

/// Details about vision rebuild.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionRebuildDetails {
    pub goals_count: usize,
    pub non_goals_count: usize,
    pub principles_count: usize,
    pub guardrails_count: usize,
    pub nl_vision_path: Option<PathBuf>,
    /// Note about limitations (users/problem not stored in SoR)
    pub limitation_note: Option<String>,
}

// =============================================================================
// Main Rebuild Function
// =============================================================================

/// Rebuild filesystem projections from capsule SoR and OverlayDb.
///
/// This function:
/// 1. Opens capsule read-only
/// 2. Queries IntakeCompleted events
/// 3. Retrieves briefs from capsule
/// 4. Calls existing projection writers to regenerate files
/// 5. Optionally rebuilds vision from OverlayDb constitution memories
///
/// # Arguments
/// * `cwd` - Working directory (project root)
/// * `request` - Rebuild request with filters and options
///
/// # Returns
/// * `Ok(RebuildResult)` with details of what was rebuilt
/// * `Err(SpecKitError)` on failure (capsule not found, URI unresolvable, etc.)
pub fn rebuild_projections(
    cwd: &Path,
    request: RebuildRequest,
) -> Result<RebuildResult, SpecKitError> {
    let mut result = RebuildResult {
        dry_run: request.dry_run,
        ..Default::default()
    };

    // Step 1: Open capsule read-only
    let capsule_path = cwd.join(DEFAULT_CAPSULE_RELATIVE_PATH);
    if !capsule_path.exists() {
        return Err(SpecKitError::RebuildError(format!(
            "Capsule not found at {}",
            capsule_path.display()
        )));
    }

    let config = CapsuleConfig {
        capsule_path: capsule_path.clone(),
        workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
        ..Default::default()
    };

    let capsule = CapsuleHandle::open(config)
        .map_err(|e| SpecKitError::RebuildError(format!("Failed to open capsule: {}", e)))?;

    // Step 2: Query IntakeCompleted events
    let all_events = capsule.list_events();
    let intake_events: Vec<_> = all_events
        .iter()
        .filter(|e| e.event_type == EventType::IntakeCompleted)
        .collect();

    if intake_events.is_empty() && !request.include_vision {
        return Err(SpecKitError::RebuildError(
            "No IntakeCompleted events found in capsule".to_string(),
        ));
    }

    // Step 3: Process spec intakes
    let spec_events: Vec<_> = intake_events
        .iter()
        .filter_map(|e| {
            let payload: IntakeCompletedPayload = serde_json::from_value(e.payload.clone()).ok()?;
            if payload.kind == IntakeKind::Spec {
                // Filter by spec_id if specified
                if let Some(ref filter_id) = request.spec_id {
                    if payload.spec_id.as_ref() != Some(filter_id) {
                        return None;
                    }
                }
                Some((e, payload))
            } else {
                None
            }
        })
        .collect();

    // Use latest spec intake per spec_id
    let mut latest_spec_intakes: HashMap<
        String,
        (
            &crate::memvid_adapter::RunEventEnvelope,
            IntakeCompletedPayload,
        ),
    > = HashMap::new();
    for (event, payload) in spec_events {
        if let Some(spec_id) = &payload.spec_id {
            // Keep latest by timestamp
            let should_replace = latest_spec_intakes
                .get(spec_id)
                .map(
                    |(existing_event, _): &(
                        &crate::memvid_adapter::RunEventEnvelope,
                        IntakeCompletedPayload,
                    )| { event.timestamp > existing_event.timestamp },
                )
                .unwrap_or(true);
            if should_replace {
                latest_spec_intakes.insert(spec_id.clone(), (event, payload));
            }
        }
    }

    // Rebuild spec projections
    for (spec_id, (event, payload)) in &latest_spec_intakes {
        let files = rebuild_spec_projection(
            cwd,
            &capsule,
            spec_id,
            payload,
            event.uri.as_str(),
            request.dry_run,
        )?;

        result.files_written.extend(files);
        result.source_uris.push(payload.brief_uri.clone());
        result.source_uris.push(payload.answers_uri.clone());

        result.spec_intakes.push(SpecIntakeInfo {
            spec_id: spec_id.to_string(),
            intake_id: payload.intake_id.clone(),
            event_uri: event.uri.as_str().to_string(),
            answers_uri: payload.answers_uri.clone(),
            brief_uri: payload.brief_uri.clone(),
            deep: payload.deep,
        });
    }

    // Step 4: Process project intakes
    let project_events: Vec<_> = intake_events
        .iter()
        .filter_map(|e| {
            let payload: IntakeCompletedPayload = serde_json::from_value(e.payload.clone()).ok()?;
            if payload.kind == IntakeKind::Project {
                // Filter by project_id if specified
                if let Some(ref filter_id) = request.project_id {
                    if payload.project_id.as_ref() != Some(filter_id) {
                        return None;
                    }
                }
                Some((e, payload))
            } else {
                None
            }
        })
        .collect();

    // Use latest project intake per project_id
    let mut latest_project_intakes: HashMap<
        String,
        (
            &crate::memvid_adapter::RunEventEnvelope,
            IntakeCompletedPayload,
        ),
    > = HashMap::new();
    for (event, payload) in project_events {
        if let Some(project_id) = &payload.project_id {
            let should_replace = latest_project_intakes
                .get(project_id)
                .map(
                    |(existing_event, _): &(
                        &crate::memvid_adapter::RunEventEnvelope,
                        IntakeCompletedPayload,
                    )| { event.timestamp > existing_event.timestamp },
                )
                .unwrap_or(true);
            if should_replace {
                latest_project_intakes.insert(project_id.clone(), (event, payload));
            }
        }
    }

    // Rebuild project projections
    for (project_id, (event, payload)) in &latest_project_intakes {
        let files = rebuild_project_projection(
            cwd,
            &capsule,
            project_id,
            payload,
            event.uri.as_str(),
            request.dry_run,
        )?;

        result.files_written.extend(files);
        result.source_uris.push(payload.brief_uri.clone());
        result.source_uris.push(payload.answers_uri.clone());

        result.project_intakes.push(ProjectIntakeInfo {
            project_id: project_id.to_string(),
            intake_id: payload.intake_id.clone(),
            event_uri: event.uri.as_str().to_string(),
            answers_uri: payload.answers_uri.clone(),
            brief_uri: payload.brief_uri.clone(),
            deep: payload.deep,
        });
    }

    // Step 5: Rebuild vision from OverlayDb (if requested)
    if request.include_vision {
        match rebuild_vision_projection(cwd, request.dry_run) {
            Ok(details) => {
                if let Some(ref path) = details.nl_vision_path {
                    result.files_written.push(path.clone());
                }
                result.vision_rebuilt = true;
                result.vision_details = Some(details);
            }
            Err(e) => {
                // Vision rebuild failure is hard-fail unless user opted out
                return Err(e);
            }
        }
    }

    Ok(result)
}

// =============================================================================
// Internal: Spec Projection Rebuild
// =============================================================================

fn rebuild_spec_projection(
    cwd: &Path,
    capsule: &CapsuleHandle,
    spec_id: &str,
    payload: &IntakeCompletedPayload,
    _event_uri: &str,
    dry_run: bool,
) -> Result<Vec<PathBuf>, SpecKitError> {
    // Retrieve DesignBrief from capsule
    let brief_bytes = capsule
        .get_bytes_str(&payload.brief_uri, None, None)
        .map_err(|e| {
            SpecKitError::RebuildError(format!(
                "Failed to retrieve brief from {}: {}",
                payload.brief_uri, e
            ))
        })?;

    let design_brief: DesignBrief = serde_json::from_slice(&brief_bytes).map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to deserialize DesignBrief: {}", e))
    })?;

    // Build a CapsulePersistenceResult for the projection writer
    // (It needs URIs and hashes for provenance table)
    let capsule_result = CapsulePersistenceResult {
        answers_uri: payload.answers_uri.clone(),
        answers_sha256: payload.answers_sha256.clone(),
        brief_uri: payload.brief_uri.clone(),
        brief_sha256: payload.brief_sha256.clone(),
        checkpoint_label: "rebuild".to_string(),
        // Deep artifacts would need to be retrieved from capsule if present
        // For now, we don't have them in the payload, so leave as None
        deep_artifacts: None,
        ace_intake_frame_uri: payload.ace_intake_frame_uri.clone(),
        ace_intake_frame_sha256: payload.ace_intake_frame_sha256.clone(),
    };

    if dry_run {
        // In dry-run, return the paths that would be written
        let docs_dir = cwd.join("docs").join(spec_id);
        return Ok(vec![
            docs_dir.join("spec.md"),
            docs_dir.join("PRD.md"),
            docs_dir.join("INTAKE.md"),
        ]);
    }

    // Create the projections using existing writer
    let _dir_name = create_spec_filesystem_projections(
        cwd,
        spec_id,
        &design_brief.description_raw,
        &design_brief,
        &capsule_result,
    )
    .map_err(|e| SpecKitError::RebuildError(format!("Failed to create spec projections: {}", e)))?;

    // Return paths that were written
    let docs_dir = cwd.join("docs").join(spec_id);
    Ok(vec![
        docs_dir.join("spec.md"),
        docs_dir.join("PRD.md"),
        docs_dir.join("INTAKE.md"),
    ])
}

// =============================================================================
// Internal: Project Projection Rebuild
// =============================================================================

fn rebuild_project_projection(
    cwd: &Path,
    capsule: &CapsuleHandle,
    project_id: &str,
    payload: &IntakeCompletedPayload,
    _event_uri: &str,
    dry_run: bool,
) -> Result<Vec<PathBuf>, SpecKitError> {
    // Retrieve ProjectBrief from capsule
    let brief_bytes = capsule
        .get_bytes_str(&payload.brief_uri, None, None)
        .map_err(|e| {
            SpecKitError::RebuildError(format!(
                "Failed to retrieve project brief from {}: {}",
                payload.brief_uri, e
            ))
        })?;

    let project_brief: ProjectBrief = serde_json::from_slice(&brief_bytes).map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to deserialize ProjectBrief: {}", e))
    })?;

    // Build a ProjectCapsulePersistenceResult for the projection writer
    let capsule_result = ProjectCapsulePersistenceResult {
        answers_uri: payload.answers_uri.clone(),
        answers_sha256: payload.answers_sha256.clone(),
        brief_uri: payload.brief_uri.clone(),
        brief_sha256: payload.brief_sha256.clone(),
        checkpoint_label: "rebuild".to_string(),
        // Deep artifacts (not in payload, would need capsule query)
        deep_artifacts: None,
        ace_intake_frame_uri: payload.ace_intake_frame_uri.clone(),
        ace_intake_frame_sha256: payload.ace_intake_frame_sha256.clone(),
    };

    if dry_run {
        let docs_dir = cwd.join("docs");
        let mut paths = vec![docs_dir.join("PROJECT_BRIEF.md")];
        if payload.deep {
            paths.push(docs_dir.join("PROJECT_ARCHITECTURE.md"));
            paths.push(docs_dir.join("PROJECT_THREATS.md"));
        }
        return Ok(paths);
    }

    // Create the projections using existing writer
    create_project_filesystem_projection(
        cwd,
        project_id,
        &project_brief,
        &capsule_result,
        payload.deep,
    )
    .map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to create project projections: {}", e))
    })?;

    // Return paths that were written
    let docs_dir = cwd.join("docs");
    let mut paths = vec![docs_dir.join("PROJECT_BRIEF.md")];
    if payload.deep {
        paths.push(docs_dir.join("PROJECT_ARCHITECTURE.md"));
        paths.push(docs_dir.join("PROJECT_THREATS.md"));
    }
    Ok(paths)
}

// =============================================================================
// Internal: Vision Projection Rebuild
// =============================================================================

fn rebuild_vision_projection(
    cwd: &Path,
    dry_run: bool,
) -> Result<VisionRebuildDetails, SpecKitError> {
    // Connect to OverlayDb
    let config = codex_stage0::Stage0Config::load()
        .map_err(|e| SpecKitError::RebuildError(format!("Failed to load Stage0 config: {}", e)))?;

    let db = codex_stage0::OverlayDb::connect_and_init(&config).map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to connect to OverlayDb: {}", e))
    })?;

    // Get constitution memories
    let memories = db.get_constitution_memories(100).map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to get constitution memories: {}", e))
    })?;

    if memories.is_empty() {
        return Ok(VisionRebuildDetails {
            goals_count: 0,
            non_goals_count: 0,
            principles_count: 0,
            guardrails_count: 0,
            nl_vision_path: None,
            limitation_note: Some(
                "No constitution memories found in OverlayDb. Vision may not have been defined."
                    .to_string(),
            ),
        });
    }

    // Categorize memories by type (based on memory_id prefix and priority)
    let mut goals: Vec<String> = Vec::new();
    let mut non_goals: Vec<String> = Vec::new();
    let mut principles: Vec<String> = Vec::new();
    let mut guardrails: Vec<String> = Vec::new();

    for mem in &memories {
        let content = mem.content_raw.clone().unwrap_or_default();
        let id = &mem.memory_id;
        let priority = mem.initial_priority;

        // Categorize by memory_id prefix (vision-goal-, vision-nongoal-, etc.)
        // or fall back to priority
        if id.starts_with("vision-goal-") {
            goals.push(content);
        } else if id.starts_with("vision-nongoal-") {
            non_goals.push(content);
        } else if id.starts_with("vision-principle-") {
            principles.push(content);
        } else if id.starts_with("vision-guardrail-") {
            guardrails.push(content);
        } else {
            // Fallback: use priority
            match priority {
                10 => guardrails.push(content),
                9 => principles.push(content),
                8 => goals.push(content), // Could be goal or non-goal; default to goal
                _ => {}                   // Ignore non-constitution memories
            }
        }
    }

    let details = VisionRebuildDetails {
        goals_count: goals.len(),
        non_goals_count: non_goals.len(),
        principles_count: principles.len(),
        guardrails_count: guardrails.len(),
        nl_vision_path: None,
        limitation_note: Some(
            "Target users and problem statement are not stored in OverlayDb SoR; \
             only constitution memories (goals, non-goals, principles, guardrails) are recovered."
                .to_string(),
        ),
    };

    if dry_run {
        let vision_path = cwd.join("memory").join("NL_VISION.md");
        return Ok(VisionRebuildDetails {
            nl_vision_path: Some(vision_path),
            ..details
        });
    }

    // Generate NL_VISION.md
    let memory_dir = cwd.join("memory");
    std::fs::create_dir_all(&memory_dir).map_err(|e| {
        SpecKitError::RebuildError(format!("Failed to create memory directory: {}", e))
    })?;

    let mut md = String::new();
    md.push_str("# Project Vision\n\n");
    md.push_str("_Rebuilt from OverlayDb constitution memories_\n\n");
    md.push_str("> **Note**: Target users and problem statement are not stored in OverlayDb.\n");
    md.push_str("> Only constitution memories (goals, non-goals, principles, guardrails) are recovered.\n\n");

    md.push_str("## Goals\n\n");
    for goal in &goals {
        md.push_str(&format!("- {}\n", goal));
    }
    if goals.is_empty() {
        md.push_str("_No goals found_\n");
    }
    md.push('\n');

    md.push_str("## Non-Goals\n\n");
    for nongoal in &non_goals {
        md.push_str(&format!("- {}\n", nongoal));
    }
    if non_goals.is_empty() {
        md.push_str("_No non-goals found_\n");
    }
    md.push('\n');

    md.push_str("## Principles\n\n");
    for principle in &principles {
        md.push_str(&format!("- {}\n", principle));
    }
    if principles.is_empty() {
        md.push_str("_No principles found_\n");
    }
    md.push('\n');

    md.push_str("## Guardrails\n\n");
    for guardrail in &guardrails {
        md.push_str(&format!("- {}\n", guardrail));
    }
    if guardrails.is_empty() {
        md.push_str("_No guardrails found_\n");
    }

    let vision_path = memory_dir.join("NL_VISION.md");
    std::fs::write(&vision_path, &md)
        .map_err(|e| SpecKitError::RebuildError(format!("Failed to write NL_VISION.md: {}", e)))?;

    Ok(VisionRebuildDetails {
        nl_vision_path: Some(vision_path),
        ..details
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_rebuild_request_builder() {
        let req = RebuildRequest::new()
            .with_spec("SPEC-KIT-042".to_string())
            .dry_run();

        assert_eq!(req.spec_id, Some("SPEC-KIT-042".to_string()));
        assert!(req.dry_run);
        assert!(req.include_vision);
    }

    #[test]
    fn test_rebuild_request_no_vision() {
        let req = RebuildRequest::new().no_vision();
        assert!(!req.include_vision);
    }
}
