//! Maieutic elicitation module for mandatory pre-execution clarification (D130)
//!
//! This module implements the mandatory maieutic step that runs before automation proceeds.
//! The maieutic step collects structured clarifications that form the delegation contract.
//!
//! ## Key Decisions
//! - D130: Maieutic step is mandatory for every run/spec before automation begins (fast path allowed)
//! - D131: Persistence follows capture mode; `capture=none` runs in-memory only
//! - D132: Ship milestones hard-fail if required explainability artifacts are missing
//! - D133: Headless requires pre-supplied input (handled by later PR)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use super::error::{Result, SpecKitError};
use crate::memvid_adapter::LLMCaptureMode;

/// Schema version for Maieutic Spec artifacts
pub const MAIEUTIC_SPEC_VERSION: &str = "1.0";

/// Maieutic Spec - Pre-flight interview output capturing assumptions, clarifications,
/// and the delegation contract for automation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaieuticSpec {
    /// SPEC ID this maieutic is for
    pub spec_id: String,
    /// Run ID (unique per pipeline run)
    pub run_id: String,
    /// Timestamp when elicitation completed
    pub timestamp: DateTime<Utc>,
    /// Schema version for forward compatibility
    pub version: String,

    // === Fast-path questions (structured, deterministic) ===
    /// Primary goal/objective of the automation
    pub goal: String,
    /// Non-negotiable constraints
    pub constraints: Vec<String>,
    /// Acceptance criteria - how to verify success
    pub acceptance_criteria: Vec<String>,
    /// Known risks and concerns
    pub risks: Vec<String>,
    /// Delegation bounds - what can run automatically
    pub delegation_bounds: DelegationBounds,

    // === Metadata ===
    /// How the maieutic was collected
    pub elicitation_mode: ElicitationMode,
    /// Time taken to complete elicitation (milliseconds)
    pub duration_ms: u64,
}

impl MaieuticSpec {
    /// Create a new MaieuticSpec from collected answers
    pub fn new(
        spec_id: String,
        run_id: String,
        goal: String,
        constraints: Vec<String>,
        acceptance_criteria: Vec<String>,
        risks: Vec<String>,
        delegation_bounds: DelegationBounds,
        elicitation_mode: ElicitationMode,
        duration_ms: u64,
    ) -> Self {
        Self {
            spec_id,
            run_id,
            timestamp: Utc::now(),
            version: MAIEUTIC_SPEC_VERSION.to_string(),
            goal,
            constraints,
            acceptance_criteria,
            risks,
            delegation_bounds,
            elicitation_mode,
            duration_ms,
        }
    }

    /// Create from a map of answers (category -> answer)
    pub fn from_answers(
        spec_id: String,
        run_id: String,
        answers: &HashMap<String, String>,
        duration_ms: u64,
    ) -> Self {
        // Parse goal
        let goal = answers
            .get("goal")
            .cloned()
            .unwrap_or_else(|| "Not specified".to_string());

        // Parse constraints (comma-separated or multi-select)
        let constraints = answers
            .get("constraints")
            .map(|s| {
                s.split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Parse acceptance criteria
        let acceptance_criteria = answers
            .get("acceptance")
            .map(|s| {
                s.split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec!["All tests pass".to_string()]);

        // Parse risks
        let risks = answers
            .get("risks")
            .map(|s| {
                s.split(',')
                    .map(|c| c.trim().to_string())
                    .filter(|c| !c.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        // Parse delegation bounds
        let delegation_bounds = DelegationBounds::from_answer(
            answers.get("delegation").map(|s| s.as_str()).unwrap_or(""),
        );

        Self::new(
            spec_id,
            run_id,
            goal,
            constraints,
            acceptance_criteria,
            risks,
            delegation_bounds,
            ElicitationMode::Interactive,
            duration_ms,
        )
    }

    /// Validate that the maieutic spec is complete enough to proceed
    pub fn validate(&self) -> Result<()> {
        if self.goal.trim().is_empty() {
            return Err(SpecKitError::from_string(
                "Maieutic spec missing goal/objective",
            ));
        }
        if self.acceptance_criteria.is_empty() {
            return Err(SpecKitError::from_string(
                "Maieutic spec missing acceptance criteria",
            ));
        }
        Ok(())
    }
}

/// Delegation bounds - what can run automatically without asking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DelegationBounds {
    /// Auto-approve file writes to evidence directories
    pub auto_approve_file_writes: bool,
    /// Auto-approve specific command patterns
    pub auto_approve_commands: Vec<String>,
    /// Actions that require explicit approval
    pub require_approval_for: Vec<String>,
    /// Maximum iterations without human check (0 = always check)
    pub max_iterations_without_check: u32,
}

impl DelegationBounds {
    /// Parse delegation bounds from answer string
    pub fn from_answer(answer: &str) -> Self {
        match answer.to_uppercase().as_str() {
            s if s.contains("FILE WRITES IN DOCS/") || s.starts_with('A') => Self {
                auto_approve_file_writes: true,
                auto_approve_commands: vec!["cargo fmt".to_string(), "cargo clippy".to_string()],
                require_approval_for: vec!["git push".to_string(), "rm -rf".to_string()],
                max_iterations_without_check: 5,
            },
            s if s.contains("FMT/CLIPPY") || s.starts_with('B') => Self {
                auto_approve_file_writes: false,
                auto_approve_commands: vec!["cargo fmt".to_string(), "cargo clippy".to_string()],
                require_approval_for: vec![],
                max_iterations_without_check: 3,
            },
            s if s.contains("ALL SAFE") || s.starts_with('C') => Self {
                auto_approve_file_writes: true,
                auto_approve_commands: vec![
                    "cargo fmt".to_string(),
                    "cargo clippy".to_string(),
                    "cargo build".to_string(),
                    "cargo test".to_string(),
                ],
                require_approval_for: vec!["git push".to_string()],
                max_iterations_without_check: 10,
            },
            s if s.contains("NOTHING") || s.starts_with('D') => Self {
                auto_approve_file_writes: false,
                auto_approve_commands: vec![],
                require_approval_for: vec!["*".to_string()],
                max_iterations_without_check: 0,
            },
            _ => Self::default(),
        }
    }

    /// Create restrictive bounds (approve nothing automatically)
    pub fn restrictive() -> Self {
        Self {
            auto_approve_file_writes: false,
            auto_approve_commands: vec![],
            require_approval_for: vec!["*".to_string()],
            max_iterations_without_check: 0,
        }
    }

    /// Create permissive bounds (approve safe operations)
    pub fn permissive() -> Self {
        Self {
            auto_approve_file_writes: true,
            auto_approve_commands: vec![
                "cargo fmt".to_string(),
                "cargo clippy".to_string(),
                "cargo build".to_string(),
                "cargo test".to_string(),
            ],
            require_approval_for: vec!["git push".to_string()],
            max_iterations_without_check: 10,
        }
    }
}

/// How the maieutic elicitation was performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElicitationMode {
    /// Interactive modal in TUI
    Interactive,
    /// Pre-supplied via CLI flag (--maieutic <path> or --maieutic-answers <json>)
    PreSupplied,
}

/// Question definition for maieutic elicitation
#[derive(Debug, Clone)]
pub struct MaieuticQuestion {
    /// Unique identifier for the question
    pub id: &'static str,
    /// Category label (displayed as badge)
    pub category: &'static str,
    /// The question text
    pub text: &'static str,
    /// Available options
    pub options: Vec<MaieuticOption>,
    /// Whether this question is required
    pub required: bool,
    /// Whether multiple options can be selected
    pub multi_select: bool,
}

/// Option for a maieutic question
#[derive(Debug, Clone)]
pub struct MaieuticOption {
    /// Option label (A, B, C, D)
    pub label: char,
    /// Option text
    pub text: &'static str,
    /// Whether this is the custom input option
    pub is_custom: bool,
}

/// Returns the default fast-path question set (5 questions, 30-90 seconds)
pub fn default_fast_path_questions() -> Vec<MaieuticQuestion> {
    vec![
        MaieuticQuestion {
            id: "goal",
            category: "Goal",
            text: "What is the primary objective of this automation?",
            options: vec![
                MaieuticOption {
                    label: 'A',
                    text: "Implement the full feature as specified",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'B',
                    text: "Create a prototype/proof-of-concept",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'C',
                    text: "Refactor existing code",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            required: true,
            multi_select: false,
        },
        MaieuticQuestion {
            id: "constraints",
            category: "Constraints",
            text: "What constraints are non-negotiable? (select all that apply)",
            options: vec![
                MaieuticOption {
                    label: 'A',
                    text: "Must not modify existing public APIs",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'B',
                    text: "Must maintain backward compatibility",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'C',
                    text: "Must pass all existing tests",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            required: true,
            multi_select: true,
        },
        MaieuticQuestion {
            id: "acceptance",
            category: "Acceptance",
            text: "How will you verify success?",
            options: vec![
                MaieuticOption {
                    label: 'A',
                    text: "All tests pass (cargo test)",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'B',
                    text: "Manual verification",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'C',
                    text: "Code review approval",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'D',
                    text: "Custom...",
                    is_custom: true,
                },
            ],
            required: true,
            multi_select: false,
        },
        MaieuticQuestion {
            id: "risks",
            category: "Risks",
            text: "What risks concern you most?",
            options: vec![
                MaieuticOption {
                    label: 'A',
                    text: "Breaking existing functionality",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'B',
                    text: "Security vulnerabilities",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'C',
                    text: "Performance regression",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'D',
                    text: "None / Custom...",
                    is_custom: true,
                },
            ],
            required: false,
            multi_select: false,
        },
        MaieuticQuestion {
            id: "delegation",
            category: "Delegation",
            text: "What should run automatically without asking?",
            options: vec![
                MaieuticOption {
                    label: 'A',
                    text: "File writes within docs/SPEC-*/",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'B',
                    text: "cargo fmt and cargo clippy",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'C',
                    text: "All safe operations",
                    is_custom: false,
                },
                MaieuticOption {
                    label: 'D',
                    text: "Nothing - approve everything",
                    is_custom: false,
                },
            ],
            required: true,
            multi_select: false,
        },
    ]
}

/// Persist maieutic spec based on capture mode (D131)
///
/// - `capture=none`: Returns Ok(None), no file written (in-memory only)
/// - `capture=prompts_only` or `capture=full_io`: Writes to evidence directory
pub fn persist_maieutic_spec(
    spec_id: &str,
    maieutic: &MaieuticSpec,
    capture_mode: LLMCaptureMode,
    cwd: &std::path::Path,
) -> Result<Option<PathBuf>> {
    match capture_mode {
        LLMCaptureMode::None => {
            // D131: capture=none runs in-memory only
            tracing::info!(
                spec_id = %spec_id,
                "Maieutic spec not persisted (capture_mode=none)"
            );
            Ok(None)
        }
        LLMCaptureMode::PromptsOnly | LLMCaptureMode::FullIo => {
            // Persist to evidence directory
            let evidence_dir = super::evidence::evidence_base_for_spec(cwd, spec_id);
            std::fs::create_dir_all(&evidence_dir).map_err(|e| SpecKitError::DirectoryCreate {
                path: evidence_dir.clone(),
                source: e,
            })?;

            let filename = format!(
                "maieutic_spec_{}.json",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );
            let path = evidence_dir.join(filename);

            let json = serde_json::to_string_pretty(maieutic)
                .map_err(|e| SpecKitError::JsonSerialize { source: e })?;

            std::fs::write(&path, json).map_err(|e| SpecKitError::FileWrite {
                path: path.clone(),
                source: e,
            })?;

            tracing::info!(
                spec_id = %spec_id,
                path = %path.display(),
                "Maieutic spec persisted"
            );
            Ok(Some(path))
        }
    }
}

/// Check if maieutic elicitation has been completed for a spec/run
pub fn has_maieutic_completed(spec_id: &str, run_id: &str, cwd: &std::path::Path) -> bool {
    let evidence_dir = super::evidence::evidence_base_for_spec(cwd, spec_id);
    let pattern = format!("maieutic_spec_");

    // Check if any maieutic spec file exists for this run
    std::fs::read_dir(&evidence_dir)
        .ok()
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(|n| n.starts_with(&pattern) && n.ends_with(".json"))
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maieutic_spec_creation() {
        let spec = MaieuticSpec::new(
            "SPEC-TEST-001".to_string(),
            "run-123".to_string(),
            "Implement feature X".to_string(),
            vec!["No API changes".to_string()],
            vec!["All tests pass".to_string()],
            vec!["Breaking changes".to_string()],
            DelegationBounds::default(),
            ElicitationMode::Interactive,
            45000,
        );

        assert_eq!(spec.spec_id, "SPEC-TEST-001");
        assert_eq!(spec.goal, "Implement feature X");
        assert_eq!(spec.version, MAIEUTIC_SPEC_VERSION);
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn test_maieutic_spec_from_answers() {
        let mut answers = HashMap::new();
        answers.insert("goal".to_string(), "Implement full feature".to_string());
        answers.insert(
            "constraints".to_string(),
            "No API changes, Tests pass".to_string(),
        );
        answers.insert("acceptance".to_string(), "All tests pass".to_string());
        answers.insert("risks".to_string(), "Breaking changes".to_string());
        answers.insert("delegation".to_string(), "A".to_string());

        let spec = MaieuticSpec::from_answers(
            "SPEC-TEST-001".to_string(),
            "run-123".to_string(),
            &answers,
            30000,
        );

        assert_eq!(spec.goal, "Implement full feature");
        assert_eq!(spec.constraints.len(), 2);
        assert!(spec.delegation_bounds.auto_approve_file_writes);
    }

    #[test]
    fn test_delegation_bounds_parsing() {
        let restrictive = DelegationBounds::from_answer("D");
        assert!(!restrictive.auto_approve_file_writes);
        assert_eq!(restrictive.max_iterations_without_check, 0);

        let permissive = DelegationBounds::from_answer("C");
        assert!(permissive.auto_approve_file_writes);
        assert_eq!(permissive.max_iterations_without_check, 10);
    }

    #[test]
    fn test_default_fast_path_questions() {
        let questions = default_fast_path_questions();
        assert_eq!(questions.len(), 5);

        // Check question IDs
        assert_eq!(questions[0].id, "goal");
        assert_eq!(questions[1].id, "constraints");
        assert_eq!(questions[2].id, "acceptance");
        assert_eq!(questions[3].id, "risks");
        assert_eq!(questions[4].id, "delegation");

        // Check that each question has 4 options
        for q in &questions {
            assert_eq!(q.options.len(), 4);
        }
    }

    #[test]
    fn test_maieutic_validation() {
        let mut spec = MaieuticSpec::new(
            "SPEC-TEST-001".to_string(),
            "run-123".to_string(),
            "Goal".to_string(),
            vec![],
            vec!["Tests pass".to_string()],
            vec![],
            DelegationBounds::default(),
            ElicitationMode::Interactive,
            1000,
        );
        assert!(spec.validate().is_ok());

        // Empty goal should fail
        spec.goal = "".to_string();
        assert!(spec.validate().is_err());

        // Empty acceptance criteria should fail
        spec.goal = "Goal".to_string();
        spec.acceptance_criteria = vec![];
        assert!(spec.validate().is_err());
    }

    /// D130: Verify that maieutic gate blocks when no previous elicitation exists
    #[test]
    fn test_maieutic_required_before_execute() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-GATE";
        let run_id = "run-001";

        // No maieutic file exists - gate should return false (pause pipeline)
        assert!(
            !has_maieutic_completed(spec_id, run_id, temp_dir.path()),
            "has_maieutic_completed should return false when no maieutic exists"
        );

        // Now create a maieutic spec and persist it
        let spec = MaieuticSpec::new(
            spec_id.to_string(),
            run_id.to_string(),
            "Test goal".to_string(),
            vec!["Constraint".to_string()],
            vec!["Tests pass".to_string()],
            vec![],
            DelegationBounds::default(),
            ElicitationMode::Interactive,
            1000,
        );

        // Persist with PromptsOnly (should create file)
        let result =
            persist_maieutic_spec(spec_id, &spec, LLMCaptureMode::PromptsOnly, temp_dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().is_some(), "File should be created");

        // Now gate should return true (fast path - use existing)
        assert!(
            has_maieutic_completed(spec_id, run_id, temp_dir.path()),
            "has_maieutic_completed should return true after persistence"
        );
    }

    /// D131: Verify that capture_mode=None does not persist maieutic
    #[test]
    fn test_capture_none_does_not_persist_maieutic() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-CAPTURE";

        let spec = MaieuticSpec::new(
            spec_id.to_string(),
            "run-001".to_string(),
            "Test goal".to_string(),
            vec!["Constraint".to_string()],
            vec!["Tests pass".to_string()],
            vec![],
            DelegationBounds::default(),
            ElicitationMode::Interactive,
            1000,
        );

        // With capture_mode=None, should return Ok(None) - no file written
        let result = persist_maieutic_spec(spec_id, &spec, LLMCaptureMode::None, temp_dir.path());

        assert!(result.is_ok(), "persist_maieutic_spec should succeed");
        assert!(
            result.unwrap().is_none(),
            "capture_mode=None should return None (no file path)"
        );

        // Verify no file was created in evidence directory
        let evidence_dir =
            crate::chatwidget::spec_kit::evidence::evidence_base_for_spec(temp_dir.path(), spec_id);
        let has_files = evidence_dir.exists()
            && std::fs::read_dir(&evidence_dir)
                .map(|entries| entries.count() > 0)
                .unwrap_or(false);
        assert!(
            !has_files,
            "No files should be created when capture_mode=None"
        );
    }
}
