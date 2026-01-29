//! Intake schemas and helpers for "Architect-in-a-box" front doors.
//!
//! Intake artifacts are persisted to the capsule as the system-of-record (SoR).
//! Filesystem artifacts under `docs/` and `memory/` are projections derived from capsule.

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const SPEC_INTAKE_ANSWERS_SCHEMA_VERSION: &str = "intake_answers@1.0";
pub const PROJECT_INTAKE_ANSWERS_SCHEMA_VERSION: &str = "project_intake_answers@1.0";
pub const DESIGN_BRIEF_SCHEMA_VERSION: &str = "design_brief@1.0";
pub const PROJECT_BRIEF_SCHEMA_VERSION: &str = "project_brief@1.0";
pub const DEEP_ARTIFACT_SCHEMA_VERSION: &str = "deep_artifact@1.0";

/// ACE Intake Frame schema version - replayable decision explainability for intake
pub const ACE_INTAKE_FRAME_SCHEMA_VERSION: &str = "ace_intake_frame@1.0";

fn default_ace_intake_frame_schema_version() -> String {
    ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string()
}

/// ACE Intake Frame - replayable decision explainability for spec-kit intake
///
/// This frame captures the core decision elements from intake without duplicating
/// the full DesignBrief/ProjectBrief. It provides a stable, audit-friendly artifact
/// that links to the SoR artifacts (answers, brief) via URIs.
///
/// Generated deterministically from IntakeAnswers + DesignBrief/ProjectBrief (no LLM).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AceIntakeFrame {
    /// Schema version (defaults to ace_intake_frame@1.0)
    #[serde(default = "default_ace_intake_frame_schema_version")]
    pub schema_version: String,

    /// Unique intake identifier (UUID)
    pub intake_id: String,

    /// Intake kind: "spec" or "project"
    pub intake_kind: String,

    // =========================================================================
    // Core decision elements
    // =========================================================================
    /// Problem statement being addressed
    pub problem: String,

    /// Target users/audience
    pub users: Vec<String>,

    /// Desired outcome
    pub outcome: String,

    /// What's in scope
    pub scope: Vec<String>,

    /// Explicit non-goals
    pub non_goals: Vec<String>,

    /// Technical/business constraints
    pub constraints: Vec<String>,

    /// Integration points with other systems
    pub integration_points: Vec<String>,

    /// Identified risks
    pub risks: Vec<String>,

    /// Open questions to resolve
    pub open_questions: Vec<String>,

    // =========================================================================
    // Provenance links (URIs to SoR artifacts)
    // =========================================================================
    /// URI to intake answers artifact in capsule
    pub answers_uri: String,

    /// URI to design/project brief artifact in capsule
    pub brief_uri: String,

    /// URIs to grounding artifacts (deep mode only, empty otherwise)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grounding_uris: Vec<String>,
}

/// Build ACE intake frame from spec DesignBrief
///
/// Pure function: deterministic output from inputs (no LLM).
pub fn build_ace_intake_frame_from_spec(
    design_brief: &DesignBrief,
    answers_uri: &str,
    brief_uri: &str,
) -> AceIntakeFrame {
    AceIntakeFrame {
        schema_version: ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string(),
        intake_id: design_brief.intake_id.clone(),
        intake_kind: "spec".to_string(),
        problem: design_brief.problem.clone(),
        users: design_brief.target_users.clone(),
        outcome: design_brief.outcome.clone(),
        scope: design_brief.scope_in.clone(),
        non_goals: design_brief.non_goals.clone(),
        constraints: design_brief.constraints.clone(),
        integration_points: design_brief.integration_points.clone(),
        risks: design_brief.risks.clone(),
        open_questions: design_brief.open_questions.clone(),
        answers_uri: answers_uri.to_string(),
        brief_uri: brief_uri.to_string(),
        grounding_uris: design_brief.grounding_uris.clone(),
    }
}

/// Build ACE intake frame from project ProjectBrief
///
/// Pure function: deterministic output from inputs (no LLM).
/// Note: ProjectBrief has different field names, so we map them appropriately.
pub fn build_ace_intake_frame_from_project(
    project_brief: &ProjectBrief,
    answers_uri: &str,
    brief_uri: &str,
) -> AceIntakeFrame {
    AceIntakeFrame {
        schema_version: ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string(),
        intake_id: project_brief.intake_id.clone(),
        intake_kind: "project".to_string(),
        problem: project_brief.problem.clone(),
        // ProjectBrief.users is a String, convert to Vec<String>
        users: vec![project_brief.users.clone()],
        // Truthful generic outcome - projects have goals in scope, not a single outcome
        outcome: "Deliver project goals (see scope)".to_string(),
        // ProjectBrief.goals maps to scope
        scope: project_brief.goals.clone(),
        non_goals: project_brief.non_goals.clone(),
        // ProjectBrief has guardrails + principles as constraints
        constraints: project_brief.guardrails.clone(),
        // Empty - do not fabricate from unrelated fields (principles are not integrations)
        integration_points: Vec::new(),
        // Use empty risks for projects (not captured in ProjectBrief)
        risks: Vec::new(),
        // Use empty open_questions for projects (not captured in ProjectBrief)
        open_questions: Vec::new(),
        answers_uri: answers_uri.to_string(),
        brief_uri: brief_uri.to_string(),
        grounding_uris: project_brief.grounding_uris.clone(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntakeAnswers {
    pub schema_version: String,
    pub question_set_id: String,
    pub deep: bool,
    pub answers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub validation_warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub text: String,
    pub verification_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepDesignSections {
    pub architecture_components: Vec<String>,
    pub architecture_dataflows: Vec<String>,
    pub integration_mapping: Vec<String>,
    pub test_plan: Vec<String>,
    pub threat_model: Vec<String>,
    pub rollout_plan: Vec<String>,
    pub risk_register: Vec<String>,
    pub non_goals_rationale: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignBrief {
    pub schema_version: String,
    pub spec_id: String,
    pub intake_id: String,
    pub created_at: DateTime<Utc>,
    pub created_via: String,
    pub description_raw: String,
    pub problem: String,
    pub target_users: Vec<String>,
    pub outcome: String,
    pub scope_in: Vec<String>,
    pub non_goals: Vec<String>,
    pub acceptance_criteria: Vec<AcceptanceCriterion>,
    pub constraints: Vec<String>,
    pub integration_points: Vec<String>,
    pub risks: Vec<String>,
    pub open_questions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assumptions: Option<Vec<String>>,
    pub deep: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_sections: Option<DeepDesignSections>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grounding_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDeepSections {
    pub deployment_target: String,
    pub data_classification: String,
    pub nfr_budgets: String,
    pub ops_baseline: String,
    pub security_posture: String,
    pub release_rollout: String,
    pub primary_components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectBrief {
    pub schema_version: String,
    pub project_id: String,
    pub intake_id: String,
    pub created_at: DateTime<Utc>,
    pub created_via: String,
    pub users: String,
    pub problem: String,
    pub goals: Vec<String>,
    pub non_goals: Vec<String>,
    pub principles: Vec<String>,
    pub guardrails: Vec<String>,
    pub artifact_kind: String,
    pub deep: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep_sections: Option<ProjectDeepSections>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grounding_uris: Vec<String>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn split_semicolon_list(raw: &str) -> Vec<String> {
    raw.split(';')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

pub fn parse_acceptance_criteria(raw: &str) -> Result<Vec<AcceptanceCriterion>, String> {
    let items = split_semicolon_list(raw);
    if items.is_empty() {
        return Err("Acceptance criteria cannot be empty.".to_string());
    }

    let mut parsed: Vec<AcceptanceCriterion> = Vec::new();
    for item in items {
        let verify_marker = "(verify:";
        let marker_index = item.rfind(verify_marker).ok_or_else(|| {
            format!(
                "Acceptance criteria must include verification method. Use: \"<criterion> (verify: <how>)\". Got: {}",
                item
            )
        })?;

        let text = item[..marker_index].trim().trim_end_matches('-').trim();
        let after = item[marker_index + verify_marker.len()..].trim();
        let verification_method = after.trim_end_matches(')').trim();

        if text.is_empty() || verification_method.is_empty() {
            return Err(format!(
                "Acceptance criteria must include both text and verification method. Got: {}",
                item
            ));
        }

        parsed.push(AcceptanceCriterion {
            text: text.to_string(),
            verification_method: verification_method.to_string(),
        });
    }

    Ok(parsed)
}

fn normalize_token(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c: char| !c.is_alphanumeric())
        .to_ascii_lowercase()
}

pub fn validate_integration_points(points: &[String]) -> Result<(), String> {
    if points.is_empty() {
        return Err("At least one integration point is required.".to_string());
    }

    for point in points {
        if point.trim().is_empty() {
            return Err("Integration points cannot contain empty entries.".to_string());
        }
        if normalize_token(point) == "unknown" {
            return Err(
                "Integration points cannot be \"unknown\". Use a concrete or hypothesized integration point."
                    .to_string(),
            );
        }
    }

    Ok(())
}

// =============================================================================
// Deep Artifact Types
// =============================================================================

/// Result from persisting deep spec artifacts to capsule
#[derive(Debug, Clone)]
pub struct DeepArtifactResult {
    pub architecture_sketch_uri: String,
    pub architecture_sketch_sha256: String,
    pub test_plan_uri: String,
    pub test_plan_sha256: String,
    pub threat_model_uri: String,
    pub threat_model_sha256: String,
    pub rollout_plan_uri: String,
    pub rollout_plan_sha256: String,
    pub design_uri: String,
    pub design_sha256: String,
}

/// Result from persisting deep project artifacts to capsule
#[derive(Debug, Clone)]
pub struct ProjectDeepArtifactResult {
    pub architecture_sketch_uri: String,
    pub architecture_sketch_sha256: String,
    pub threat_model_uri: String,
    pub threat_model_sha256: String,
    pub ops_baseline_uri: Option<String>,
    pub ops_baseline_sha256: Option<String>,
}

// =============================================================================
// Deep Artifact Generation Helpers
// =============================================================================

/// Generate a Mermaid flowchart from architecture components and dataflows
///
/// Components become nodes (C0, C1, ...), dataflows become edges.
/// Dataflow format: "source -> target: description" or "source -> target"
pub fn generate_architecture_mermaid(
    components: &[String],
    dataflows: &[String],
    title: &str,
) -> String {
    use std::collections::HashMap;

    let mut lines = vec![
        "---".to_string(),
        format!("title: {}", title),
        "---".to_string(),
        "flowchart TD".to_string(),
    ];

    // Add component nodes
    for (i, component) in components.iter().enumerate() {
        let id = format!("C{}", i);
        let safe_label = sanitize_mermaid_label(component);
        lines.push(format!("    {}[\"{}\"]", id, safe_label));
    }

    // Build component name to ID mapping (lowercase for matching)
    let component_map: HashMap<String, String> = components
        .iter()
        .enumerate()
        .map(|(i, c)| (c.to_lowercase(), format!("C{}", i)))
        .collect();

    // Add dataflow edges
    for dataflow in dataflows {
        if let Some((source, rest)) = dataflow.split_once("->") {
            let source = source.trim().to_lowercase();
            let (target, label) = if let Some((t, l)) = rest.split_once(':') {
                (t.trim().to_lowercase(), Some(l.trim()))
            } else {
                (rest.trim().to_lowercase(), None)
            };

            // Find matching component IDs (fuzzy match)
            let source_id = find_component_id(&component_map, &source);
            let target_id = find_component_id(&component_map, &target);

            if let (Some(src), Some(tgt)) = (source_id, target_id) {
                if let Some(lbl) = label {
                    lines.push(format!(
                        "    {} -->|{}| {}",
                        src,
                        sanitize_mermaid_label(lbl),
                        tgt
                    ));
                } else {
                    lines.push(format!("    {} --> {}", src, tgt));
                }
            }
        }
    }

    lines.join("\n")
}

fn sanitize_mermaid_label(s: &str) -> String {
    s.replace('"', "'").replace('\n', " ").trim().to_string()
}

fn find_component_id(
    map: &std::collections::HashMap<String, String>,
    name: &str,
) -> Option<String> {
    // Exact match first
    if let Some(id) = map.get(name) {
        return Some(id.clone());
    }
    // Fuzzy match - component contains name or name contains component
    for (comp_name, id) in map {
        if comp_name.contains(name) || name.contains(comp_name.as_str()) {
            return Some(id.clone());
        }
    }
    None
}

/// Format test plan items into markdown
pub fn format_test_plan(items: &[String], spec_id: &str) -> String {
    let mut md = format!("# Test Plan: {}\n\n", spec_id);
    md.push_str("## Test Cases\n\n");
    for (i, item) in items.iter().enumerate() {
        md.push_str(&format!("### TC-{:03}: {}\n\n", i + 1, item));
    }
    md.push_str("\n---\n\n_Generated from deep intake._\n");
    md
}

/// Format threat model items into markdown
pub fn format_threat_model(items: &[String], spec_id: &str) -> String {
    let mut md = format!("# Threat Model: {}\n\n", spec_id);
    md.push_str("## Identified Threats\n\n");
    for (i, item) in items.iter().enumerate() {
        md.push_str(&format!("### T-{:03}: {}\n\n", i + 1, item));
    }
    md.push_str("\n---\n\n_Generated from deep intake._\n");
    md
}

/// Format rollout plan items into markdown
pub fn format_rollout_plan(items: &[String], spec_id: &str) -> String {
    let mut md = format!("# Rollout Plan: {}\n\n", spec_id);
    md.push_str("## Phases\n\n");
    for (i, item) in items.iter().enumerate() {
        md.push_str(&format!("{}. {}\n", i + 1, item));
    }
    md.push_str("\n---\n\n_Generated from deep intake._\n");
    md
}

/// Format design document from integration mapping and non-goals rationale
pub fn format_design_doc(
    integration_mapping: &[String],
    non_goals_rationale: &[String],
    spec_id: &str,
) -> String {
    let mut md = format!("# Design Document: {}\n\n", spec_id);

    md.push_str("## Integration Mapping\n\n");
    for item in integration_mapping {
        md.push_str(&format!("- {}\n", item));
    }

    md.push_str("\n## Non-Goals Rationale\n\n");
    for item in non_goals_rationale {
        md.push_str(&format!("- {}\n", item));
    }

    md.push_str("\n---\n\n_Generated from deep intake._\n");
    md
}

/// Format project threat model summary
pub fn format_project_threat_model(security_posture: &str, project_id: &str) -> String {
    format!(
        "# Threat Model Summary: {}\n\n## Security Posture\n\n{}\n\n---\n\n_Generated from deep intake._\n",
        project_id, security_posture
    )
}

/// Format project ops baseline
pub fn format_project_ops_baseline(ops_baseline: &str, project_id: &str) -> String {
    format!(
        "# Ops Baseline: {}\n\n{}\n\n---\n\n_Generated from deep intake._\n",
        project_id, ops_baseline
    )
}

/// Generate project architecture mermaid from primary components
pub fn generate_project_architecture_mermaid(components: &[String], project_id: &str) -> String {
    let mut lines = vec![
        "---".to_string(),
        format!("title: {} Architecture", project_id),
        "---".to_string(),
        "flowchart TD".to_string(),
    ];

    // Add component nodes
    for (i, component) in components.iter().enumerate() {
        let id = format!("C{}", i);
        let safe_label = sanitize_mermaid_label(component);
        lines.push(format!("    {}[\"{}\"]", id, safe_label));
    }

    lines.join("\n")
}

// =============================================================================
// ACE Intake Frame Schema Tests
// =============================================================================

#[cfg(test)]
mod ace_intake_frame_tests {
    use super::*;
    use schemars::schema_for;

    /// Test that schema generation produces stable output matching committed schema
    #[test]
    fn test_ace_intake_frame_schema_generation_stable() {
        let schema = schema_for!(AceIntakeFrame);
        let generated = serde_json::to_value(&schema).unwrap();

        let committed_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../spec-kit/src/config/schemas/ace_intake_frame.schema.v1.json"
        );

        let committed: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(committed_path).expect("schema file exists"),
        )
        .unwrap();

        // Compare properties (ignoring metadata like $schema, title)
        assert_eq!(
            generated.get("properties"),
            committed.get("properties"),
            "Schema properties changed. Regenerate with: cargo run --bin ace-schema-gen -p codex-tui -- -o spec-kit/src/config/schemas/"
        );

        assert_eq!(
            generated.get("required"),
            committed.get("required"),
            "Schema required fields changed. Regenerate with: cargo run --bin ace-schema-gen -p codex-tui -- -o spec-kit/src/config/schemas/"
        );
    }

    /// Test that schema_version field defaults correctly
    #[test]
    fn test_ace_intake_frame_schema_version_defaults() {
        // JSON without schema_version should deserialize with default
        let json = r#"{
            "intake_id": "test-intake-id",
            "intake_kind": "spec",
            "problem": "Test problem",
            "users": ["User A"],
            "outcome": "Test outcome",
            "scope": ["Feature 1"],
            "non_goals": ["Non-goal 1"],
            "constraints": ["Constraint 1"],
            "integration_points": ["API 1"],
            "risks": ["Risk 1"],
            "open_questions": ["Question 1"],
            "answers_uri": "mv2://default/SPEC-1/intake/answers.json",
            "brief_uri": "mv2://default/SPEC-1/intake/brief.json"
        }"#;

        let result: AceIntakeFrame = serde_json::from_str(json).unwrap();
        assert_eq!(result.schema_version, ACE_INTAKE_FRAME_SCHEMA_VERSION);
    }

    /// Test that ACE Intake Frame examples validate against schema
    #[test]
    fn test_ace_intake_frame_examples_validate() {
        let frame = AceIntakeFrame {
            schema_version: ACE_INTAKE_FRAME_SCHEMA_VERSION.to_string(),
            intake_id: "test-intake-id".to_string(),
            intake_kind: "spec".to_string(),
            problem: "Users cannot track their API usage".to_string(),
            users: vec!["API consumers".to_string(), "Billing admins".to_string()],
            outcome: "Clear visibility into API consumption".to_string(),
            scope: vec!["Usage tracking".to_string(), "Export to CSV".to_string()],
            non_goals: vec!["Real-time alerting".to_string()],
            constraints: vec!["Must integrate with existing auth".to_string()],
            integration_points: vec!["Auth service".to_string(), "Billing API".to_string()],
            risks: vec!["Performance impact on high-traffic endpoints".to_string()],
            open_questions: vec!["Retention policy?".to_string()],
            answers_uri: "mv2://default/SPEC-KIT-042/intake-123/artifact/intake/answers.json"
                .to_string(),
            brief_uri: "mv2://default/SPEC-KIT-042/intake-123/artifact/intake/design_brief.json"
                .to_string(),
            grounding_uris: vec![],
        };

        let schema_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../spec-kit/src/config/schemas/ace_intake_frame.schema.v1.json"
        );
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();

        let compiled = jsonschema::JSONSchema::compile(&schema).unwrap();
        let instance = serde_json::to_value(&frame).unwrap();

        let result = compiled.validate(&instance);
        assert!(
            result.is_ok(),
            "ACE Intake Frame should validate against schema"
        );
    }

    /// Test backward compatibility - old frames without schema_version should deserialize
    #[test]
    fn test_backward_compatibility_no_version() {
        let old_json = r#"{
            "intake_id": "legacy-intake",
            "intake_kind": "spec",
            "problem": "Legacy problem",
            "users": ["User"],
            "outcome": "Legacy outcome",
            "scope": ["Scope"],
            "non_goals": ["Non-goal"],
            "constraints": ["Constraint"],
            "integration_points": ["Integration"],
            "risks": ["Risk"],
            "open_questions": ["Question"],
            "answers_uri": "mv2://old/uri/answers.json",
            "brief_uri": "mv2://old/uri/brief.json"
        }"#;

        let frame: AceIntakeFrame = serde_json::from_str(old_json).unwrap();

        // Should deserialize with default schema version
        assert_eq!(frame.schema_version, ACE_INTAKE_FRAME_SCHEMA_VERSION);
        assert_eq!(frame.intake_id, "legacy-intake");
        assert_eq!(frame.intake_kind, "spec");
    }

    /// Test builder functions produce valid frames
    #[test]
    fn test_build_ace_intake_frame_from_spec() {
        let design_brief = DesignBrief {
            schema_version: DESIGN_BRIEF_SCHEMA_VERSION.to_string(),
            spec_id: "SPEC-KIT-042".to_string(),
            intake_id: "test-intake-id".to_string(),
            created_at: chrono::Utc::now(),
            created_via: "test".to_string(),
            description_raw: "Test feature".to_string(),
            problem: "Test problem".to_string(),
            target_users: vec!["User A".to_string()],
            outcome: "Test outcome".to_string(),
            scope_in: vec!["Scope item".to_string()],
            non_goals: vec!["Non-goal item".to_string()],
            acceptance_criteria: vec![AcceptanceCriterion {
                text: "Criterion".to_string(),
                verification_method: "test".to_string(),
            }],
            constraints: vec!["Constraint".to_string()],
            integration_points: vec!["Integration".to_string()],
            risks: vec!["Risk".to_string()],
            open_questions: vec!["Question".to_string()],
            assumptions: None,
            deep: false,
            deep_sections: None,
            grounding_uris: vec!["mv2://grounding/uri".to_string()],
        };

        let frame =
            build_ace_intake_frame_from_spec(&design_brief, "mv2://answers/uri", "mv2://brief/uri");

        assert_eq!(frame.schema_version, ACE_INTAKE_FRAME_SCHEMA_VERSION);
        assert_eq!(frame.intake_kind, "spec");
        assert_eq!(frame.intake_id, "test-intake-id");
        assert_eq!(frame.problem, "Test problem");
        assert_eq!(frame.users, vec!["User A"]);
        assert_eq!(frame.answers_uri, "mv2://answers/uri");
        assert_eq!(frame.brief_uri, "mv2://brief/uri");
        assert_eq!(frame.grounding_uris, vec!["mv2://grounding/uri"]);
    }

    /// Test builder for project frames
    #[test]
    fn test_build_ace_intake_frame_from_project() {
        let project_brief = ProjectBrief {
            schema_version: PROJECT_BRIEF_SCHEMA_VERSION.to_string(),
            project_id: "my-project".to_string(),
            intake_id: "project-intake-id".to_string(),
            created_at: chrono::Utc::now(),
            created_via: "test".to_string(),
            users: "Developers".to_string(),
            problem: "Project problem".to_string(),
            goals: vec!["Goal 1".to_string(), "Goal 2".to_string()],
            non_goals: vec!["Non-goal".to_string()],
            principles: vec!["Principle".to_string()],
            guardrails: vec!["Guardrail".to_string()],
            artifact_kind: "rust".to_string(),
            deep: false,
            deep_sections: None,
            grounding_uris: vec![],
        };

        let frame = build_ace_intake_frame_from_project(
            &project_brief,
            "mv2://project/answers",
            "mv2://project/brief",
        );

        assert_eq!(frame.schema_version, ACE_INTAKE_FRAME_SCHEMA_VERSION);
        assert_eq!(frame.intake_kind, "project");
        assert_eq!(frame.intake_id, "project-intake-id");
        assert_eq!(frame.problem, "Project problem");
        assert_eq!(frame.users, vec!["Developers"]);
        assert_eq!(frame.scope, vec!["Goal 1", "Goal 2"]);
        assert_eq!(frame.constraints, vec!["Guardrail"]);

        // Verify no fabrication of integration_points from principles
        assert!(
            frame.integration_points.is_empty(),
            "integration_points must be empty for projects (not fabricated from principles)"
        );
        // Verify truthful generic outcome
        assert_eq!(frame.outcome, "Deliver project goals (see scope)");
        // Verify empty arrays are preserved
        assert!(frame.risks.is_empty());
        assert!(frame.open_questions.is_empty());
    }
}
