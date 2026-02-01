//! Core intake validation and persistence logic - UI-independent
//!
//! Extracted from spec_intake_handler.rs and project_intake_handler.rs
//! for reuse by both TUI handlers and headless CLI.
//!
//! All functions are pure or take Path/capsule parameters only - no ChatWidget dependencies.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chrono::{Local, Utc};

use crate::memvid_adapter::{
    CapsuleConfig, CapsuleHandle, DEFAULT_CAPSULE_RELATIVE_PATH, DEFAULT_WORKSPACE_ID,
    IntakeCompletedPayload, IntakeKind, ObjectType,
};

use super::error::SpecKitError;
use super::grounding::extract_artifact_name_from_uri;
use super::intake::{
    ACE_INTAKE_FRAME_SCHEMA_VERSION, DEEP_ARTIFACT_SCHEMA_VERSION, DESIGN_BRIEF_SCHEMA_VERSION,
    DeepArtifactResult, DeepDesignSections, DesignBrief, IntakeAnswers,
    PROJECT_BRIEF_SCHEMA_VERSION, PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION, ProjectBrief,
    ProjectDeepArtifactResult, ProjectDeepSections, SPEC_INTAKE_ANSWERS_SCHEMA_VERSION,
    build_ace_intake_frame_from_project, build_ace_intake_frame_from_spec, format_design_doc,
    format_project_ops_baseline, format_project_threat_model, format_rollout_plan,
    format_test_plan, format_threat_model, generate_architecture_mermaid,
    generate_project_architecture_mermaid, parse_acceptance_criteria, sha256_hex,
    split_semicolon_list, validate_integration_points,
};
use super::spec_id_generator::create_slug;

// =============================================================================
// Types
// =============================================================================

/// Validation result for intake answers
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Result from capsule persistence operations
#[derive(Debug, Clone)]
pub struct CapsulePersistenceResult {
    pub answers_uri: String,
    pub answers_sha256: String,
    pub brief_uri: String,
    pub brief_sha256: String,
    pub checkpoint_label: String,
    /// Deep artifacts (None if not deep mode)
    pub deep_artifacts: Option<DeepArtifactResult>,
    /// ACE intake frame URI (ace_intake_frame@1.0)
    pub ace_intake_frame_uri: Option<String>,
    /// ACE intake frame SHA256
    pub ace_intake_frame_sha256: Option<String>,
}

/// Result from project capsule persistence operations
#[derive(Debug, Clone)]
pub struct ProjectCapsulePersistenceResult {
    pub answers_uri: String,
    pub answers_sha256: String,
    pub brief_uri: String,
    pub brief_sha256: String,
    pub checkpoint_label: String,
    /// Deep artifacts (None if not deep mode)
    pub deep_artifacts: Option<ProjectDeepArtifactResult>,
    /// ACE intake frame URI (ace_intake_frame@1.0)
    pub ace_intake_frame_uri: Option<String>,
    /// ACE intake frame SHA256
    pub ace_intake_frame_sha256: Option<String>,
}

// =============================================================================
// Spec Intake Functions
// =============================================================================

/// Validate spec intake answers per requirements.
///
/// ## Baseline requirements (always enforced):
/// - problem: non-empty
/// - target_users: >= 1
/// - outcome: non-empty
/// - constraints: >= 1
/// - scope_in: 3-7 items
/// - non_goals: 3-7 items
/// - integration_points: >= 1, not "unknown"
/// - risks: >= 1
/// - open_questions: >= 1
/// - acceptance_criteria: format validation (>= 1)
///
/// ## Deep requirements (when deep=true):
/// - acceptance_criteria: >= 5 items
/// - architecture_components: non-empty
/// - architecture_dataflows: non-empty
/// - integration_mapping: non-empty
/// - test_plan: non-empty
/// - threat_model: non-empty
/// - rollout_plan: non-empty
pub fn validate_spec_answers(answers: &HashMap<String, String>, deep: bool) -> ValidationResult {
    let warnings = Vec::new();
    let mut errors = Vec::new();

    // =========================================================================
    // Baseline validation (always enforced)
    // =========================================================================

    // Required: problem (non-empty)
    if answers.get("problem").is_none_or(|s| s.trim().is_empty()) {
        errors.push("Problem statement is required.".to_string());
    }

    // Required: target_users >= 1
    let target_users = split_semicolon_list(answers.get("target_users").unwrap_or(&String::new()));
    if target_users.is_empty() {
        errors.push("At least one target user is required.".to_string());
    }

    // Required: outcome (non-empty)
    if answers.get("outcome").is_none_or(|s| s.trim().is_empty()) {
        errors.push("Expected outcome is required.".to_string());
    }

    // Required: constraints >= 1
    let constraints = split_semicolon_list(answers.get("constraints").unwrap_or(&String::new()));
    if constraints.is_empty() {
        errors.push("At least one constraint is required.".to_string());
    }

    // Required: scope_in 3-7 items
    let scope_in = split_semicolon_list(answers.get("scope_in").unwrap_or(&String::new()));
    if scope_in.len() < 3 || scope_in.len() > 7 {
        errors.push(format!(
            "Scope must have 3-7 items (found {}).",
            scope_in.len()
        ));
    }

    // Required: non_goals 3-7 items
    let non_goals = split_semicolon_list(answers.get("non_goals").unwrap_or(&String::new()));
    if non_goals.len() < 3 || non_goals.len() > 7 {
        errors.push(format!(
            "Non-goals must have 3-7 items (found {}).",
            non_goals.len()
        ));
    }

    // Required: integration_points >= 1, not "unknown"
    let integration_points =
        split_semicolon_list(answers.get("integration_points").unwrap_or(&String::new()));
    if let Err(e) = validate_integration_points(&integration_points) {
        errors.push(format!("Integration points: {}", e));
    }

    // Required: risks >= 1
    let risks = split_semicolon_list(answers.get("risks").unwrap_or(&String::new()));
    if risks.is_empty() {
        errors.push("At least one risk is required.".to_string());
    }

    // Required: open_questions >= 1
    let open_questions =
        split_semicolon_list(answers.get("open_questions").unwrap_or(&String::new()));
    if open_questions.is_empty() {
        errors.push("At least one open question is required.".to_string());
    }

    // Required: acceptance_criteria format validation
    let empty = String::new();
    let ac_raw = answers.get("acceptance_criteria").unwrap_or(&empty);
    let ac_result = parse_acceptance_criteria(ac_raw);
    if let Err(e) = &ac_result {
        errors.push(format!("Acceptance criteria: {}", e));
    }

    // =========================================================================
    // Deep validation (when deep=true)
    // =========================================================================

    if deep {
        // Deep requires >= 5 acceptance criteria
        if let Ok(ref parsed_ac) = ac_result {
            if parsed_ac.len() < 5 {
                errors.push(format!(
                    "Deep mode requires at least 5 acceptance criteria (found {}).",
                    parsed_ac.len()
                ));
            }
        }

        // Deep requires architecture_components non-empty
        let arch_components = split_semicolon_list(
            answers
                .get("architecture_components")
                .unwrap_or(&String::new()),
        );
        if arch_components.is_empty() {
            errors.push("Deep mode requires architecture components.".to_string());
        }

        // Deep requires architecture_dataflows non-empty
        let arch_dataflows = split_semicolon_list(
            answers
                .get("architecture_dataflows")
                .unwrap_or(&String::new()),
        );
        if arch_dataflows.is_empty() {
            errors.push("Deep mode requires architecture dataflows.".to_string());
        }

        // Deep requires integration_mapping non-empty
        let integration_mapping =
            split_semicolon_list(answers.get("integration_mapping").unwrap_or(&String::new()));
        if integration_mapping.is_empty() {
            errors.push("Deep mode requires integration mapping.".to_string());
        }

        // Deep requires test_plan non-empty
        let test_plan = split_semicolon_list(answers.get("test_plan").unwrap_or(&String::new()));
        if test_plan.is_empty() {
            errors.push("Deep mode requires test plan.".to_string());
        }

        // Deep requires threat_model non-empty
        let threat_model =
            split_semicolon_list(answers.get("threat_model").unwrap_or(&String::new()));
        if threat_model.is_empty() {
            errors.push("Deep mode requires threat model.".to_string());
        }

        // Deep requires rollout_plan non-empty
        let rollout_plan =
            split_semicolon_list(answers.get("rollout_plan").unwrap_or(&String::new()));
        if rollout_plan.is_empty() {
            errors.push("Deep mode requires rollout plan.".to_string());
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        warnings,
        errors,
    }
}

/// Validate project intake answers per requirements.
///
/// ## Baseline requirements (always enforced):
/// - users: non-empty
/// - problem: non-empty
/// - artifact_kind: non-empty
/// - goals: 3-5 items
/// - non_goals: >= 1
/// - principles: >= 1
/// - guardrails: >= 1
///
/// ## Deep requirements (when deep=true):
/// - primary_components: >= 1
/// - deployment_target: non-empty
/// - data_classification: non-empty
/// - nfr_budgets: non-empty
/// - ops_baseline: non-empty
pub fn validate_project_answers(answers: &HashMap<String, String>, deep: bool) -> ValidationResult {
    let warnings = Vec::new();
    let mut errors = Vec::new();

    // =========================================================================
    // Baseline validation (always enforced)
    // =========================================================================

    // Required: users (non-empty)
    if answers.get("users").is_none_or(|s| s.trim().is_empty()) {
        errors.push("Target users is required.".to_string());
    }

    // Required: problem (non-empty)
    if answers.get("problem").is_none_or(|s| s.trim().is_empty()) {
        errors.push("Problem statement is required.".to_string());
    }

    // Required: artifact_kind (non-empty)
    if answers
        .get("artifact_kind")
        .is_none_or(|s| s.trim().is_empty())
    {
        errors.push("Artifact kind is required.".to_string());
    }

    // Required: goals 3-5 items
    let goals = split_semicolon_list(answers.get("goals").unwrap_or(&String::new()));
    if goals.len() < 3 || goals.len() > 5 {
        errors.push(format!(
            "Goals must have 3-5 items (found {}).",
            goals.len()
        ));
    }

    // Required: non_goals >= 1
    let non_goals = split_semicolon_list(answers.get("non_goals").unwrap_or(&String::new()));
    if non_goals.is_empty() {
        errors.push("At least one non-goal is required.".to_string());
    }

    // Required: principles >= 1
    let principles = split_semicolon_list(answers.get("principles").unwrap_or(&String::new()));
    if principles.is_empty() {
        errors.push("At least one principle is required.".to_string());
    }

    // Required: guardrails >= 1
    let guardrails = split_semicolon_list(answers.get("guardrails").unwrap_or(&String::new()));
    if guardrails.is_empty() {
        errors.push("At least one guardrail is required.".to_string());
    }

    // =========================================================================
    // Deep validation (when deep=true)
    // =========================================================================

    if deep {
        // Deep requires primary_components >= 1
        let primary_components =
            split_semicolon_list(answers.get("primary_components").unwrap_or(&String::new()));
        if primary_components.is_empty() {
            errors.push("Deep mode requires at least one primary component.".to_string());
        }

        // Deep requires deployment_target non-empty
        if answers
            .get("deployment_target")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires deployment target.".to_string());
        }

        // Deep requires data_classification non-empty
        if answers
            .get("data_classification")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires data classification.".to_string());
        }

        // Deep requires nfr_budgets non-empty
        if answers
            .get("nfr_budgets")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires NFR budgets.".to_string());
        }

        // Deep requires ops_baseline non-empty
        if answers
            .get("ops_baseline")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires ops baseline.".to_string());
        }

        // Deep requires security_posture non-empty (for threat model generation)
        if answers
            .get("security_posture")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires security posture.".to_string());
        }

        // Deep requires release_rollout non-empty
        if answers
            .get("release_rollout")
            .is_none_or(|s| s.trim().is_empty())
        {
            errors.push("Deep mode requires release rollout strategy.".to_string());
        }
    }

    ValidationResult {
        valid: errors.is_empty(),
        warnings,
        errors,
    }
}

/// Build IntakeAnswers struct from raw spec intake answers
pub fn build_spec_intake_answers(
    answers: &HashMap<String, String>,
    deep: bool,
    warnings: Vec<String>,
) -> IntakeAnswers {
    IntakeAnswers {
        schema_version: SPEC_INTAKE_ANSWERS_SCHEMA_VERSION.to_string(),
        question_set_id: if deep {
            "spec_intake_deep_v1".to_string()
        } else {
            "spec_intake_baseline_v1".to_string()
        },
        deep,
        answers: answers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        validation_warnings: warnings,
    }
}

/// Build DesignBrief struct from raw spec intake answers
///
/// # Arguments
/// * `answers` - Raw answers from modal/CLI
/// * `spec_id` - Generated SPEC-ID
/// * `intake_id` - Generated UUID for this intake
/// * `description` - Feature description
/// * `deep` - Whether deep questions were answered
/// * `created_via` - Source of the intake (e.g., "spec_intake_modal", "headless_cli")
/// * `grounding_uris` - URIs of grounding artifacts (from deep grounding capture)
pub fn build_design_brief(
    answers: &HashMap<String, String>,
    spec_id: &str,
    intake_id: &str,
    description: &str,
    deep: bool,
    created_via: &str,
    grounding_uris: Vec<String>,
) -> Result<DesignBrief, String> {
    let acceptance_criteria =
        parse_acceptance_criteria(answers.get("acceptance_criteria").unwrap_or(&String::new()))?;

    let deep_sections = if deep {
        Some(DeepDesignSections {
            architecture_components: split_semicolon_list(
                answers
                    .get("architecture_components")
                    .unwrap_or(&String::new()),
            ),
            architecture_dataflows: split_semicolon_list(
                answers
                    .get("architecture_dataflows")
                    .unwrap_or(&String::new()),
            ),
            integration_mapping: split_semicolon_list(
                answers.get("integration_mapping").unwrap_or(&String::new()),
            ),
            test_plan: split_semicolon_list(answers.get("test_plan").unwrap_or(&String::new())),
            threat_model: split_semicolon_list(
                answers.get("threat_model").unwrap_or(&String::new()),
            ),
            rollout_plan: split_semicolon_list(
                answers.get("rollout_plan").unwrap_or(&String::new()),
            ),
            risk_register: split_semicolon_list(
                answers.get("risk_register").unwrap_or(&String::new()),
            ),
            non_goals_rationale: split_semicolon_list(
                answers.get("non_goals_rationale").unwrap_or(&String::new()),
            ),
        })
    } else {
        None
    };

    let assumptions = split_semicolon_list(answers.get("assumptions").unwrap_or(&String::new()));

    Ok(DesignBrief {
        schema_version: DESIGN_BRIEF_SCHEMA_VERSION.to_string(),
        spec_id: spec_id.to_string(),
        intake_id: intake_id.to_string(),
        created_at: Utc::now(),
        created_via: created_via.to_string(),
        description_raw: description.to_string(),
        problem: answers.get("problem").cloned().unwrap_or_default(),
        target_users: split_semicolon_list(answers.get("target_users").unwrap_or(&String::new())),
        outcome: answers.get("outcome").cloned().unwrap_or_default(),
        scope_in: split_semicolon_list(answers.get("scope_in").unwrap_or(&String::new())),
        non_goals: split_semicolon_list(answers.get("non_goals").unwrap_or(&String::new())),
        acceptance_criteria,
        constraints: split_semicolon_list(answers.get("constraints").unwrap_or(&String::new())),
        integration_points: split_semicolon_list(
            answers.get("integration_points").unwrap_or(&String::new()),
        ),
        risks: split_semicolon_list(answers.get("risks").unwrap_or(&String::new())),
        open_questions: split_semicolon_list(
            answers.get("open_questions").unwrap_or(&String::new()),
        ),
        assumptions: if assumptions.is_empty() {
            None
        } else {
            Some(assumptions)
        },
        deep,
        deep_sections,
        grounding_uris,
    })
}

/// Persist spec intake artifacts to capsule (SoR)
///
/// # Arguments
/// * `cwd` - Working directory containing capsule
/// * `spec_id` - SPEC-ID for this intake
/// * `intake_id` - UUID for this intake
/// * `intake_answers` - Validated IntakeAnswers struct
/// * `design_brief` - Built DesignBrief struct
/// * `deep` - Whether deep questions were answered
/// * `created_via` - Source identifier for the intake event
///
/// # Returns
/// * `Ok(CapsulePersistenceResult)` with URIs and hashes
/// * `Err(String)` on any failure (capsule open, put, emit, or checkpoint)
pub fn persist_spec_intake_to_capsule(
    cwd: &Path,
    spec_id: &str,
    intake_id: &str,
    intake_answers: &IntakeAnswers,
    design_brief: &DesignBrief,
    deep: bool,
    created_via: &str,
) -> Result<CapsulePersistenceResult, String> {
    // Open capsule with canonical config
    let capsule_path = cwd.join(DEFAULT_CAPSULE_RELATIVE_PATH);
    let config = CapsuleConfig {
        capsule_path,
        workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
        ..Default::default()
    };

    let capsule = CapsuleHandle::open(config).map_err(|e| format!("Capsule open failed: {}", e))?;

    // Serialize artifacts
    let answers_json = serde_json::to_vec_pretty(intake_answers)
        .map_err(|e| format!("Failed to serialize intake answers: {}", e))?;
    let answers_sha256 = sha256_hex(&answers_json);

    let brief_json = serde_json::to_vec_pretty(design_brief)
        .map_err(|e| format!("Failed to serialize design brief: {}", e))?;
    let brief_sha256 = sha256_hex(&brief_json);

    // Use intake_id as run_id for intake phase
    let run_id = intake_id;

    // Put answers artifact
    let answers_meta = serde_json::json!({
        "schema_version": SPEC_INTAKE_ANSWERS_SCHEMA_VERSION,
        "sha256": answers_sha256,
        "deep": deep,
    });
    let answers_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/answers.json",
            answers_json,
            answers_meta,
        )
        .map_err(|e| format!("Capsule put answers failed: {}", e))?;

    // Put design brief artifact
    let brief_meta = serde_json::json!({
        "schema_version": DESIGN_BRIEF_SCHEMA_VERSION,
        "sha256": brief_sha256,
        "spec_id": spec_id,
    });
    let brief_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/design_brief.json",
            brief_json,
            brief_meta,
        )
        .map_err(|e| format!("Capsule put brief failed: {}", e))?;

    // =========================================================================
    // ACE Intake Frame persistence (ace_intake_frame@1.0)
    // =========================================================================

    // Build ACE intake frame (deterministic, no LLM)
    let ace_frame =
        build_ace_intake_frame_from_spec(design_brief, answers_uri.as_str(), brief_uri.as_str());
    let ace_frame_json = serde_json::to_vec_pretty(&ace_frame)
        .map_err(|e| format!("Failed to serialize ACE intake frame: {}", e))?;
    let ace_frame_sha256 = sha256_hex(&ace_frame_json);

    let ace_frame_meta = serde_json::json!({
        "schema_version": ACE_INTAKE_FRAME_SCHEMA_VERSION,
        "sha256": ace_frame_sha256,
        "intake_kind": "spec",
    });
    let ace_frame_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/ace_intake_frame.json",
            ace_frame_json,
            ace_frame_meta,
        )
        .map_err(|e| format!("Capsule put ACE intake frame failed: {}", e))?;

    // =========================================================================
    // Deep artifact persistence (when deep=true)
    // =========================================================================

    let deep_artifacts = if deep {
        if let Some(ref deep_sections) = design_brief.deep_sections {
            // 1. Architecture sketch (Mermaid)
            let arch_mermaid = generate_architecture_mermaid(
                &deep_sections.architecture_components,
                &deep_sections.architecture_dataflows,
                &format!("{} Architecture", spec_id),
            );
            let arch_bytes = arch_mermaid.as_bytes().to_vec();
            let arch_sha256 = sha256_hex(&arch_bytes);
            let arch_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": arch_sha256,
                "artifact_type": "mermaid",
            });
            let arch_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/architecture_sketch.mmd",
                    arch_bytes,
                    arch_meta,
                )
                .map_err(|e| format!("Capsule put architecture_sketch failed: {}", e))?;

            // 2. Test plan (Markdown)
            let test_plan_md = format_test_plan(&deep_sections.test_plan, spec_id);
            let test_bytes = test_plan_md.as_bytes().to_vec();
            let test_sha256 = sha256_hex(&test_bytes);
            let test_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": test_sha256,
            });
            let test_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/test_plan.md",
                    test_bytes,
                    test_meta,
                )
                .map_err(|e| format!("Capsule put test_plan failed: {}", e))?;

            // 3. Threat model (Markdown)
            let threat_md = format_threat_model(&deep_sections.threat_model, spec_id);
            let threat_bytes = threat_md.as_bytes().to_vec();
            let threat_sha256 = sha256_hex(&threat_bytes);
            let threat_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": threat_sha256,
            });
            let threat_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/threat_model.md",
                    threat_bytes,
                    threat_meta,
                )
                .map_err(|e| format!("Capsule put threat_model failed: {}", e))?;

            // 4. Rollout plan (Markdown)
            let rollout_md = format_rollout_plan(&deep_sections.rollout_plan, spec_id);
            let rollout_bytes = rollout_md.as_bytes().to_vec();
            let rollout_sha256 = sha256_hex(&rollout_bytes);
            let rollout_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": rollout_sha256,
            });
            let rollout_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/rollout_plan.md",
                    rollout_bytes,
                    rollout_meta,
                )
                .map_err(|e| format!("Capsule put rollout_plan failed: {}", e))?;

            // 5. Design document (integration mapping + non-goals rationale)
            let design_md = format_design_doc(
                &deep_sections.integration_mapping,
                &deep_sections.non_goals_rationale,
                spec_id,
            );
            let design_bytes = design_md.as_bytes().to_vec();
            let design_sha256 = sha256_hex(&design_bytes);
            let design_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": design_sha256,
            });
            let design_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/design.md",
                    design_bytes,
                    design_meta,
                )
                .map_err(|e| format!("Capsule put design failed: {}", e))?;

            Some(DeepArtifactResult {
                architecture_sketch_uri: arch_uri.as_str().to_string(),
                architecture_sketch_sha256: arch_sha256,
                test_plan_uri: test_uri.as_str().to_string(),
                test_plan_sha256: test_sha256,
                threat_model_uri: threat_uri.as_str().to_string(),
                threat_model_sha256: threat_sha256,
                rollout_plan_uri: rollout_uri.as_str().to_string(),
                rollout_plan_sha256: rollout_sha256,
                design_uri: design_uri.as_str().to_string(),
                design_sha256,
            })
        } else {
            None
        }
    } else {
        None
    };

    // Emit IntakeCompleted event
    let payload = IntakeCompletedPayload {
        kind: IntakeKind::Spec,
        deep,
        intake_id: intake_id.to_string(),
        project_id: None,
        spec_id: Some(spec_id.to_string()),
        answers_uri: answers_uri.as_str().to_string(),
        answers_sha256: answers_sha256.clone(),
        brief_uri: brief_uri.as_str().to_string(),
        brief_sha256: brief_sha256.clone(),
        answers_schema_version: SPEC_INTAKE_ANSWERS_SCHEMA_VERSION.to_string(),
        brief_schema_version: DESIGN_BRIEF_SCHEMA_VERSION.to_string(),
        created_via: created_via.to_string(),
        // ACE intake frame fields (ace_intake_frame@1.0)
        ace_intake_frame_uri: Some(ace_frame_uri.as_str().to_string()),
        ace_intake_frame_sha256: Some(ace_frame_sha256.clone()),
        ace_intake_frame_schema_version: Some(ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string()),
    };
    capsule
        .emit_intake_completed(spec_id, run_id, &payload)
        .map_err(|e| format!("Capsule emit_intake_completed failed: {}", e))?;

    // Create manual checkpoint
    let checkpoint_label = format!("intake:spec:{}:{}", spec_id, intake_id);
    capsule
        .commit_manual(&checkpoint_label)
        .map_err(|e| format!("Capsule checkpoint failed: {}", e))?;

    Ok(CapsulePersistenceResult {
        answers_uri: answers_uri.as_str().to_string(),
        answers_sha256,
        brief_uri: brief_uri.as_str().to_string(),
        brief_sha256,
        checkpoint_label,
        deep_artifacts,
        ace_intake_frame_uri: Some(ace_frame_uri.as_str().to_string()),
        ace_intake_frame_sha256: Some(ace_frame_sha256),
    })
}

/// Create spec filesystem projections (spec.md, PRD.md, INTAKE.md)
///
/// # Returns
/// * `Ok(dir_name)` - The created directory name (e.g., "SPEC-KIT-042-add-user-auth")
/// * `Err(SpecKitError)` on filesystem failure
pub fn create_spec_filesystem_projections(
    cwd: &Path,
    spec_id: &str,
    description: &str,
    brief: &DesignBrief,
    capsule_result: &CapsulePersistenceResult,
) -> Result<String, SpecKitError> {
    let slug = create_slug(description);
    let feature_name = capitalize_words(description);
    let dir_name = format!("{}-{}", spec_id, slug);
    let spec_dir = cwd.join("docs").join(&dir_name);

    fs::create_dir_all(&spec_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: spec_dir.clone(),
        source: e,
    })?;

    // Create spec.md
    let spec_content = format!(
        r#"**SPEC-ID**: {}
**Feature**: {}
**Status**: Backlog
**Created**: {}
**Owner**: Code

---

## Problem

{}

## Target Users

{}

## Outcome

{}

---

## Scope

### In Scope
{}

### Non-Goals
{}

---

## Acceptance Criteria

{}

---

## Constraints

{}

## Integration Points

{}

## Risks

{}

## Open Questions

{}

---

Created via Architect-in-a-box intake modal.
"#,
        spec_id,
        feature_name,
        Local::now().format("%Y-%m-%d"),
        brief.problem,
        brief.target_users.join("; "),
        brief.outcome,
        brief
            .scope_in
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n"),
        brief
            .non_goals
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n"),
        brief
            .acceptance_criteria
            .iter()
            .map(|ac| format!("- {} (verify: {})", ac.text, ac.verification_method))
            .collect::<Vec<_>>()
            .join("\n"),
        brief.constraints.join("; "),
        brief.integration_points.join("; "),
        brief
            .risks
            .iter()
            .map(|r| format!("- {}", r))
            .collect::<Vec<_>>()
            .join("\n"),
        brief
            .open_questions
            .iter()
            .map(|q| format!("- {}", q))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    fs::write(spec_dir.join("spec.md"), spec_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_dir.join("spec.md"),
        source: e,
    })?;

    // Create PRD.md (brief summary)
    let prd_content = format!(
        r#"# {} PRD

**SPEC-ID**: {}
**Status**: Draft
**Created**: {}

## Overview

{}

## Problem Statement

{}

## Target Users

{}

## Success Criteria

{}

---

_Generated from intake. Run `/speckit.auto {}` for full pipeline._
"#,
        feature_name,
        spec_id,
        Local::now().format("%Y-%m-%d"),
        brief.outcome,
        brief.problem,
        brief.target_users.join(", "),
        brief
            .acceptance_criteria
            .iter()
            .map(|ac| format!("- {}", ac.text))
            .collect::<Vec<_>>()
            .join("\n"),
        spec_id,
    );

    fs::write(spec_dir.join("PRD.md"), prd_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_dir.join("PRD.md"),
        source: e,
    })?;

    // Create INTAKE.md with capsule provenance
    let generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let mut intake_content = format!(
        r#"# Intake Record

**SPEC-ID**: {}
**Intake ID**: {}
**Deep Mode**: {}
**Created**: {}
**Created Via**: {}
**Generated At**: {}

---

## Capsule Provenance

| Artifact | URI | SHA256 |
|----------|-----|--------|
| Answers | `{}` | `{}` |
| Design Brief | `{}` | `{}` |

## Schema Versions

- Answers: `{}`
- Design Brief: `{}`
"#,
        spec_id,
        brief.intake_id,
        if brief.deep { "Yes" } else { "No" },
        brief.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        brief.created_via,
        generated_at,
        capsule_result.answers_uri,
        capsule_result.answers_sha256,
        capsule_result.brief_uri,
        capsule_result.brief_sha256,
        SPEC_INTAKE_ANSWERS_SCHEMA_VERSION,
        DESIGN_BRIEF_SCHEMA_VERSION,
    );

    // Add deep artifacts section if present
    if let Some(ref deep_artifacts) = capsule_result.deep_artifacts {
        intake_content.push_str(&format!(
            r#"
---

## Deep Artifacts

| Artifact | URI | SHA256 |
|----------|-----|--------|
| Architecture Sketch | `{}` | `{}` |
| Test Plan | `{}` | `{}` |
| Threat Model | `{}` | `{}` |
| Rollout Plan | `{}` | `{}` |
| Design Document | `{}` | `{}` |

### Deep Schema Versions

- Deep Artifacts: `{}`
"#,
            deep_artifacts.architecture_sketch_uri,
            deep_artifacts.architecture_sketch_sha256,
            deep_artifacts.test_plan_uri,
            deep_artifacts.test_plan_sha256,
            deep_artifacts.threat_model_uri,
            deep_artifacts.threat_model_sha256,
            deep_artifacts.rollout_plan_uri,
            deep_artifacts.rollout_plan_sha256,
            deep_artifacts.design_uri,
            deep_artifacts.design_sha256,
            DEEP_ARTIFACT_SCHEMA_VERSION,
        ));
    }

    // Add grounding artifacts section if present
    if !brief.grounding_uris.is_empty() {
        intake_content.push_str("\n---\n\n## Grounding Artifacts\n\n");
        intake_content.push_str("| Artifact | URI |\n|----------|-----|\n");
        for uri in &brief.grounding_uris {
            let name = extract_artifact_name_from_uri(uri);
            intake_content.push_str(&format!("| {} | `{}` |\n", name, uri));
        }
    }

    intake_content.push_str(&format!(
        r#"
---

## Raw Description

{}

---

_This file is a filesystem projection of the capsule SoR._
"#,
        brief.description_raw,
    ));

    fs::write(spec_dir.join("INTAKE.md"), intake_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_dir.join("INTAKE.md"),
        source: e,
    })?;

    // =========================================================================
    // Deep filesystem projections (when deep_artifacts present)
    // =========================================================================

    if let Some(ref deep_artifacts) = capsule_result.deep_artifacts {
        if let Some(ref deep_sections) = brief.deep_sections {
            let provenance_generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

            // DESIGN.md
            let design_content = format!(
                "{}
---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{}` |
| SHA256 | `{}` |
| Schema Version | `{}` |
| Generated At | `{}` |
",
                format_design_doc(
                    &deep_sections.integration_mapping,
                    &deep_sections.non_goals_rationale,
                    spec_id,
                ),
                deep_artifacts.design_uri,
                deep_artifacts.design_sha256,
                DEEP_ARTIFACT_SCHEMA_VERSION,
                provenance_generated_at,
            );
            fs::write(spec_dir.join("DESIGN.md"), design_content).map_err(|e| {
                SpecKitError::FileWrite {
                    path: spec_dir.join("DESIGN.md"),
                    source: e,
                }
            })?;

            // TEST_PLAN.md
            let test_plan_content = format!(
                "{}
---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{}` |
| SHA256 | `{}` |
| Schema Version | `{}` |
| Generated At | `{}` |
",
                format_test_plan(&deep_sections.test_plan, spec_id),
                deep_artifacts.test_plan_uri,
                deep_artifacts.test_plan_sha256,
                DEEP_ARTIFACT_SCHEMA_VERSION,
                provenance_generated_at,
            );
            fs::write(spec_dir.join("TEST_PLAN.md"), test_plan_content).map_err(|e| {
                SpecKitError::FileWrite {
                    path: spec_dir.join("TEST_PLAN.md"),
                    source: e,
                }
            })?;

            // THREAT_MODEL.md
            let threat_model_content = format!(
                "{}
---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{}` |
| SHA256 | `{}` |
| Schema Version | `{}` |
| Generated At | `{}` |
",
                format_threat_model(&deep_sections.threat_model, spec_id),
                deep_artifacts.threat_model_uri,
                deep_artifacts.threat_model_sha256,
                DEEP_ARTIFACT_SCHEMA_VERSION,
                provenance_generated_at,
            );
            fs::write(spec_dir.join("THREAT_MODEL.md"), threat_model_content).map_err(|e| {
                SpecKitError::FileWrite {
                    path: spec_dir.join("THREAT_MODEL.md"),
                    source: e,
                }
            })?;

            // ROLLOUT.md
            let rollout_content = format!(
                "{}
---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{}` |
| SHA256 | `{}` |
| Schema Version | `{}` |
| Generated At | `{}` |
",
                format_rollout_plan(&deep_sections.rollout_plan, spec_id),
                deep_artifacts.rollout_plan_uri,
                deep_artifacts.rollout_plan_sha256,
                DEEP_ARTIFACT_SCHEMA_VERSION,
                provenance_generated_at,
            );
            fs::write(spec_dir.join("ROLLOUT.md"), rollout_content).map_err(|e| {
                SpecKitError::FileWrite {
                    path: spec_dir.join("ROLLOUT.md"),
                    source: e,
                }
            })?;
        }
    }

    // Update SPEC.md tracker
    update_spec_tracker(cwd, spec_id, &feature_name, &dir_name)?;

    Ok(dir_name)
}

/// Update SPEC.md tracker with new spec entry
pub fn update_spec_tracker(
    cwd: &Path,
    spec_id: &str,
    feature_name: &str,
    dir_name: &str,
) -> Result<(), SpecKitError> {
    let spec_md_path = cwd.join("SPEC.md");

    let content = fs::read_to_string(&spec_md_path).map_err(|e| SpecKitError::FileRead {
        path: spec_md_path.clone(),
        source: e,
    })?;

    // Find the table in the Backlog section and add entry
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let mut in_backlog = false;
    let mut insert_index = None;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("## Backlog") {
            in_backlog = true;
            continue;
        }
        if in_backlog && line.starts_with("## ") {
            // End of backlog section
            break;
        }
        if in_backlog
            && line.starts_with("| ")
            && !line.contains("SPEC-ID")
            && !line.contains("---")
        {
            // Found a table row, insert before it
            insert_index = Some(i);
            break;
        }
        if in_backlog && line.trim().is_empty() && insert_index.is_none() {
            // Empty line after table header, insert here if we haven't found a row
            if let Some(prev) = lines.get(i.saturating_sub(1)) {
                if prev.contains("---") {
                    insert_index = Some(i);
                }
            }
        }
    }

    if let Some(idx) = insert_index {
        let new_row = format!(
            "| {} | {} | Backlog | [spec](docs/{}/spec.md) |",
            spec_id, feature_name, dir_name
        );
        lines.insert(idx, new_row);
    } else {
        // Fallback: append to end of file
        lines.push(format!(
            "\n| {} | {} | Backlog | [spec](docs/{}/spec.md) |",
            spec_id, feature_name, dir_name
        ));
    }

    fs::write(&spec_md_path, lines.join("\n")).map_err(|e| SpecKitError::FileWrite {
        path: spec_md_path,
        source: e,
    })?;

    Ok(())
}

/// Write only INTAKE.md to existing spec directory (backfill mode)
pub fn write_intake_md_only(
    cwd: &Path,
    spec_id: &str,
    brief: &DesignBrief,
    capsule_result: &CapsulePersistenceResult,
) -> Result<(), SpecKitError> {
    use super::spec_directory::find_spec_directory;

    let spec_dir = find_spec_directory(cwd, spec_id).map_err(|e| {
        SpecKitError::Other(format!("SPEC directory not found for {}: {}", spec_id, e))
    })?;

    let generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let intake_content = format!(
        r#"# Intake Record

**SPEC-ID**: {}
**Intake ID**: {}
**Deep Mode**: {}
**Created**: {}
**Created Via**: {}
**Generated At**: {}

---

## Capsule Provenance

| Artifact | URI | SHA256 |
|----------|-----|--------|
| Answers | `{}` | `{}` |
| Design Brief | `{}` | `{}` |

## Schema Versions

- Answers: `{}`
- Design Brief: `{}`

---

## Raw Description

{}

---

_This file is a filesystem projection of the capsule SoR._
"#,
        spec_id,
        brief.intake_id,
        if brief.deep { "Yes" } else { "No" },
        brief.created_at.format("%Y-%m-%d %H:%M:%S UTC"),
        brief.created_via,
        generated_at,
        capsule_result.answers_uri,
        capsule_result.answers_sha256,
        capsule_result.brief_uri,
        capsule_result.brief_sha256,
        SPEC_INTAKE_ANSWERS_SCHEMA_VERSION,
        DESIGN_BRIEF_SCHEMA_VERSION,
        brief.description_raw,
    );

    fs::write(spec_dir.join("INTAKE.md"), intake_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_dir.join("INTAKE.md"),
        source: e,
    })?;

    Ok(())
}

// =============================================================================
// Project Intake Functions
// =============================================================================

/// Build IntakeAnswers struct from raw project intake answers
pub fn build_project_intake_answers(
    answers: &HashMap<String, String>,
    deep: bool,
) -> IntakeAnswers {
    IntakeAnswers {
        schema_version: PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION.to_string(),
        question_set_id: if deep {
            "project_intake_deep_v1".to_string()
        } else {
            "project_intake_baseline_v1".to_string()
        },
        deep,
        answers: answers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        validation_warnings: Vec::new(),
    }
}

/// Build ProjectBrief struct from raw project intake answers
///
/// # Arguments
/// * `answers` - Raw answers from modal/CLI
/// * `project_id` - Project directory name / identifier
/// * `intake_id` - Generated UUID for this intake
/// * `deep` - Whether deep questions were answered
/// * `created_via` - Source of the intake (e.g., "project_intake_modal", "headless_cli")
/// * `grounding_uris` - URIs of grounding artifacts (from deep grounding capture)
pub fn build_project_brief(
    answers: &HashMap<String, String>,
    project_id: &str,
    intake_id: &str,
    deep: bool,
    created_via: &str,
    grounding_uris: Vec<String>,
) -> ProjectBrief {
    let deep_sections = if deep {
        Some(ProjectDeepSections {
            deployment_target: answers
                .get("deployment_target")
                .cloned()
                .unwrap_or_default(),
            data_classification: answers
                .get("data_classification")
                .cloned()
                .unwrap_or_default(),
            nfr_budgets: answers.get("nfr_budgets").cloned().unwrap_or_default(),
            ops_baseline: answers.get("ops_baseline").cloned().unwrap_or_default(),
            security_posture: answers.get("security_posture").cloned().unwrap_or_default(),
            release_rollout: answers.get("release_rollout").cloned().unwrap_or_default(),
            primary_components: split_semicolon_list(
                answers.get("primary_components").unwrap_or(&String::new()),
            ),
        })
    } else {
        None
    };

    ProjectBrief {
        schema_version: PROJECT_BRIEF_SCHEMA_VERSION.to_string(),
        project_id: project_id.to_string(),
        intake_id: intake_id.to_string(),
        created_at: Utc::now(),
        created_via: created_via.to_string(),
        users: answers.get("users").cloned().unwrap_or_default(),
        problem: answers.get("problem").cloned().unwrap_or_default(),
        goals: split_semicolon_list(answers.get("goals").unwrap_or(&String::new())),
        non_goals: split_semicolon_list(answers.get("non_goals").unwrap_or(&String::new())),
        principles: split_semicolon_list(answers.get("principles").unwrap_or(&String::new())),
        guardrails: split_semicolon_list(answers.get("guardrails").unwrap_or(&String::new())),
        artifact_kind: answers.get("artifact_kind").cloned().unwrap_or_default(),
        deep,
        deep_sections,
        grounding_uris,
    }
}

/// Persist project intake artifacts to capsule (SoR)
///
/// Uses spec_id="project" and run_id=<project_id> to produce URIs like:
/// mv2://default/project/<project_id>/artifact/intake/answers.json
pub fn persist_project_intake_to_capsule(
    cwd: &Path,
    project_id: &str,
    intake_id: &str,
    intake_answers: &IntakeAnswers,
    project_brief: &ProjectBrief,
    deep: bool,
    created_via: &str,
) -> Result<ProjectCapsulePersistenceResult, String> {
    // Open capsule with canonical config
    let capsule_path = cwd.join(DEFAULT_CAPSULE_RELATIVE_PATH);
    let config = CapsuleConfig {
        capsule_path,
        workspace_id: DEFAULT_WORKSPACE_ID.to_string(),
        ..Default::default()
    };

    let capsule = CapsuleHandle::open(config).map_err(|e| format!("Capsule open failed: {}", e))?;

    // Serialize artifacts
    let answers_json = serde_json::to_vec_pretty(intake_answers)
        .map_err(|e| format!("Failed to serialize intake answers: {}", e))?;
    let answers_sha256 = sha256_hex(&answers_json);

    let brief_json = serde_json::to_vec_pretty(project_brief)
        .map_err(|e| format!("Failed to serialize project brief: {}", e))?;
    let brief_sha256 = sha256_hex(&brief_json);

    // Use "project" as spec_id and project_id as run_id
    let spec_id = "project";
    let run_id = project_id;

    // Put answers artifact
    let answers_meta = serde_json::json!({
        "schema_version": PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION,
        "sha256": answers_sha256,
        "deep": deep,
    });
    let answers_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/answers.json",
            answers_json,
            answers_meta,
        )
        .map_err(|e| format!("Capsule put answers failed: {}", e))?;

    // Put project brief artifact
    let brief_meta = serde_json::json!({
        "schema_version": PROJECT_BRIEF_SCHEMA_VERSION,
        "sha256": brief_sha256,
        "project_id": project_id,
    });
    let brief_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/project_brief.json",
            brief_json,
            brief_meta,
        )
        .map_err(|e| format!("Capsule put brief failed: {}", e))?;

    // =========================================================================
    // ACE Intake Frame persistence (ace_intake_frame@1.0)
    // =========================================================================

    // Build ACE intake frame (deterministic, no LLM)
    let ace_frame = build_ace_intake_frame_from_project(
        project_brief,
        answers_uri.as_str(),
        brief_uri.as_str(),
    );
    let ace_frame_json = serde_json::to_vec_pretty(&ace_frame)
        .map_err(|e| format!("Failed to serialize ACE intake frame: {}", e))?;
    let ace_frame_sha256 = sha256_hex(&ace_frame_json);

    let ace_frame_meta = serde_json::json!({
        "schema_version": ACE_INTAKE_FRAME_SCHEMA_VERSION,
        "sha256": ace_frame_sha256,
        "intake_kind": "project",
    });
    let ace_frame_uri = capsule
        .put(
            spec_id,
            run_id,
            ObjectType::Artifact,
            "intake/ace_intake_frame.json",
            ace_frame_json,
            ace_frame_meta,
        )
        .map_err(|e| format!("Capsule put ACE intake frame failed: {}", e))?;

    // Emit IntakeCompleted event with kind=Project
    let payload = IntakeCompletedPayload {
        kind: IntakeKind::Project,
        deep,
        intake_id: intake_id.to_string(),
        project_id: Some(project_id.to_string()),
        spec_id: None,
        answers_uri: answers_uri.as_str().to_string(),
        answers_sha256: answers_sha256.clone(),
        brief_uri: brief_uri.as_str().to_string(),
        brief_sha256: brief_sha256.clone(),
        answers_schema_version: PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION.to_string(),
        brief_schema_version: PROJECT_BRIEF_SCHEMA_VERSION.to_string(),
        created_via: created_via.to_string(),
        // ACE intake frame fields (ace_intake_frame@1.0)
        ace_intake_frame_uri: Some(ace_frame_uri.as_str().to_string()),
        ace_intake_frame_sha256: Some(ace_frame_sha256.clone()),
        ace_intake_frame_schema_version: Some(ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string()),
    };
    capsule
        .emit_intake_completed(spec_id, run_id, &payload)
        .map_err(|e| format!("Capsule emit_intake_completed failed: {}", e))?;

    // Create manual checkpoint
    let checkpoint_label = format!("intake:project:{}:{}", project_id, intake_id);
    capsule
        .commit_manual(&checkpoint_label)
        .map_err(|e| format!("Capsule checkpoint failed: {}", e))?;

    // =========================================================================
    // Deep artifact persistence (when deep=true)
    // =========================================================================

    let deep_artifacts = if deep {
        if let Some(ref deep_sections) = project_brief.deep_sections {
            // 1. Architecture sketch (Mermaid) - from primary components
            let arch_mermaid = generate_project_architecture_mermaid(
                &deep_sections.primary_components,
                project_id,
            );
            let arch_bytes = arch_mermaid.as_bytes().to_vec();
            let arch_sha256 = sha256_hex(&arch_bytes);
            let arch_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": arch_sha256,
                "artifact_type": "mermaid",
            });
            let arch_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/architecture_sketch.mmd",
                    arch_bytes,
                    arch_meta,
                )
                .map_err(|e| format!("Capsule put architecture_sketch failed: {}", e))?;

            // 2. Threat model (Markdown) - from security posture
            let threat_md =
                format_project_threat_model(&deep_sections.security_posture, project_id);
            let threat_bytes = threat_md.as_bytes().to_vec();
            let threat_sha256 = sha256_hex(&threat_bytes);
            let threat_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": threat_sha256,
            });
            let threat_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/threat_model.md",
                    threat_bytes,
                    threat_meta,
                )
                .map_err(|e| format!("Capsule put threat_model failed: {}", e))?;

            // 3. Ops baseline (Markdown) - from ops baseline
            let ops_md = format_project_ops_baseline(&deep_sections.ops_baseline, project_id);
            let ops_bytes = ops_md.as_bytes().to_vec();
            let ops_sha256 = sha256_hex(&ops_bytes);
            let ops_meta = serde_json::json!({
                "schema_version": DEEP_ARTIFACT_SCHEMA_VERSION,
                "sha256": ops_sha256,
            });
            let ops_uri = capsule
                .put(
                    spec_id,
                    run_id,
                    ObjectType::Artifact,
                    "intake/ops_baseline.md",
                    ops_bytes,
                    ops_meta,
                )
                .map_err(|e| format!("Capsule put ops_baseline failed: {}", e))?;

            Some(ProjectDeepArtifactResult {
                architecture_sketch_uri: arch_uri.as_str().to_string(),
                architecture_sketch_sha256: arch_sha256,
                threat_model_uri: threat_uri.as_str().to_string(),
                threat_model_sha256: threat_sha256,
                ops_baseline_uri: Some(ops_uri.as_str().to_string()),
                ops_baseline_sha256: Some(ops_sha256),
            })
        } else {
            None
        }
    } else {
        None
    };

    Ok(ProjectCapsulePersistenceResult {
        answers_uri: answers_uri.as_str().to_string(),
        answers_sha256,
        brief_uri: brief_uri.as_str().to_string(),
        brief_sha256,
        checkpoint_label,
        deep_artifacts,
        ace_intake_frame_uri: Some(ace_frame_uri.as_str().to_string()),
        ace_intake_frame_sha256: Some(ace_frame_sha256),
    })
}

/// Create project filesystem projection (docs/PROJECT_BRIEF.md)
/// Also creates deep artifact projections (PROJECT_ARCHITECTURE.md, etc.) when deep=true
pub fn create_project_filesystem_projection(
    cwd: &Path,
    project_id: &str,
    brief: &ProjectBrief,
    capsule_result: &ProjectCapsulePersistenceResult,
    deep: bool,
) -> Result<(), String> {
    // Ensure docs/ exists
    let docs_dir = cwd.join("docs");
    fs::create_dir_all(&docs_dir).map_err(|e| format!("Failed to create docs directory: {}", e))?;

    // Build PROJECT_BRIEF.md content
    let deep_indicator = if deep { "Yes" } else { "No" };
    let generated_at = Utc::now().to_rfc3339();

    let mut content = format!(
        r#"# Project Brief: {project_id}

**Project ID**: {project_id}
**Created**: {created}
**Deep Mode**: {deep_indicator}

---

## Capsule Provenance

| Artifact | URI | SHA256 |
|----------|-----|--------|
| Answers | `{answers_uri}` | `{answers_sha256}` |
| Brief | `{brief_uri}` | `{brief_sha256}` |

## Schema Versions

- Answers: `{answers_schema}`
- Brief: `{brief_schema}`

**Generated At**: {generated_at}

---

## Users

{users}

## Problem

{problem}

## Goals

{goals}

## Non-Goals

{non_goals}

## Principles

{principles}

## Guardrails

{guardrails}

## Artifact Kind

{artifact_kind}
"#,
        project_id = project_id,
        created = Local::now().format("%Y-%m-%d"),
        deep_indicator = deep_indicator,
        answers_uri = capsule_result.answers_uri,
        answers_sha256 = capsule_result.answers_sha256,
        brief_uri = capsule_result.brief_uri,
        brief_sha256 = capsule_result.brief_sha256,
        answers_schema = PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION,
        brief_schema = PROJECT_BRIEF_SCHEMA_VERSION,
        generated_at = generated_at,
        users = brief.users,
        problem = brief.problem,
        goals = format_list(&brief.goals),
        non_goals = format_list(&brief.non_goals),
        principles = format_list(&brief.principles),
        guardrails = format_list(&brief.guardrails),
        artifact_kind = brief.artifact_kind,
    );

    // Add deep sections if present
    if let Some(ref deep_sections) = brief.deep_sections {
        content.push_str(&format!(
            r#"
---

## Deep Intake Sections

### Deployment Target

{deployment_target}

### Data Classification

{data_classification}

### NFR Budgets

{nfr_budgets}

### Ops Baseline

{ops_baseline}

### Security Posture

{security_posture}

### Release Rollout

{release_rollout}

### Primary Components

{primary_components}
"#,
            deployment_target = deep_sections.deployment_target,
            data_classification = deep_sections.data_classification,
            nfr_budgets = deep_sections.nfr_budgets,
            ops_baseline = deep_sections.ops_baseline,
            security_posture = deep_sections.security_posture,
            release_rollout = deep_sections.release_rollout,
            primary_components = format_list(&deep_sections.primary_components),
        ));
    }

    // Add grounding artifacts section if present
    if !brief.grounding_uris.is_empty() {
        content.push_str("\n---\n\n## Grounding Artifacts\n\n");
        content.push_str("| Artifact | URI |\n|----------|-----|\n");
        for uri in &brief.grounding_uris {
            let name = extract_artifact_name_from_uri(uri);
            content.push_str(&format!("| {} | `{}` |\n", name, uri));
        }
    }

    content.push_str(
        r#"
---

_Generated from project intake. This is a filesystem projection of the capsule SoR._
"#,
    );

    // Write to docs/PROJECT_BRIEF.md
    let brief_path = docs_dir.join("PROJECT_BRIEF.md");
    fs::write(&brief_path, &content)
        .map_err(|e| format!("Failed to write PROJECT_BRIEF.md: {}", e))?;

    // =========================================================================
    // Deep filesystem projections (when deep_artifacts present)
    // =========================================================================

    if let Some(ref deep_artifacts) = capsule_result.deep_artifacts {
        if let Some(ref deep_sections) = brief.deep_sections {
            let provenance_generated_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");

            // PROJECT_ARCHITECTURE.md (mermaid + provenance table)
            let arch_content = format!(
                r#"# Project Architecture: {project_id}

{arch_mermaid}

---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{arch_uri}` |
| SHA256 | `{arch_sha256}` |
| Schema Version | `{schema_version}` |
| Generated At | `{generated_at}` |

---

_This is a filesystem projection of the capsule SoR deep artifact._
"#,
                project_id = project_id,
                arch_mermaid = generate_project_architecture_mermaid(
                    &deep_sections.primary_components,
                    project_id,
                ),
                arch_uri = deep_artifacts.architecture_sketch_uri,
                arch_sha256 = deep_artifacts.architecture_sketch_sha256,
                schema_version = DEEP_ARTIFACT_SCHEMA_VERSION,
                generated_at = provenance_generated_at,
            );
            fs::write(docs_dir.join("PROJECT_ARCHITECTURE.md"), arch_content)
                .map_err(|e| format!("Failed to write PROJECT_ARCHITECTURE.md: {}", e))?;

            // PROJECT_THREATS.md (threat model + provenance table)
            let threat_content = format!(
                r#"{threat_model}

---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{threat_uri}` |
| SHA256 | `{threat_sha256}` |
| Schema Version | `{schema_version}` |
| Generated At | `{generated_at}` |

---

_This is a filesystem projection of the capsule SoR deep artifact._
"#,
                threat_model =
                    format_project_threat_model(&deep_sections.security_posture, project_id),
                threat_uri = deep_artifacts.threat_model_uri,
                threat_sha256 = deep_artifacts.threat_model_sha256,
                schema_version = DEEP_ARTIFACT_SCHEMA_VERSION,
                generated_at = provenance_generated_at,
            );
            fs::write(docs_dir.join("PROJECT_THREATS.md"), threat_content)
                .map_err(|e| format!("Failed to write PROJECT_THREATS.md: {}", e))?;

            // PROJECT_OPS_BASELINE.md (ops baseline + provenance table)
            if let (Some(ops_uri), Some(ops_sha256)) = (
                &deep_artifacts.ops_baseline_uri,
                &deep_artifacts.ops_baseline_sha256,
            ) {
                let ops_content = format!(
                    r#"{ops_baseline}

---

## Provenance

| Field | Value |
|-------|-------|
| Capsule URI | `{ops_uri}` |
| SHA256 | `{ops_sha256}` |
| Schema Version | `{schema_version}` |
| Generated At | `{generated_at}` |

---

_This is a filesystem projection of the capsule SoR deep artifact._
"#,
                    ops_baseline =
                        format_project_ops_baseline(&deep_sections.ops_baseline, project_id),
                    ops_uri = ops_uri,
                    ops_sha256 = ops_sha256,
                    schema_version = DEEP_ARTIFACT_SCHEMA_VERSION,
                    generated_at = provenance_generated_at,
                );
                fs::write(docs_dir.join("PROJECT_OPS_BASELINE.md"), ops_content)
                    .map_err(|e| format!("Failed to write PROJECT_OPS_BASELINE.md: {}", e))?;
            }
        }
    }

    Ok(())
}

// =============================================================================
// Helpers
// =============================================================================

/// Capitalize first letter of each word
pub fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a list of strings as markdown bullet points
pub fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".to_string()
    } else {
        items
            .iter()
            .map(|s| format!("- {}", s))
            .collect::<Vec<_>>()
            .join("\n")
    }
}
