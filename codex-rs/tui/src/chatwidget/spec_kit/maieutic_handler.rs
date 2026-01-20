//! Maieutic elicitation event handlers (D130)
//!
//! Handles MaieuticSubmitted and MaieuticCancelled events from the modal.

use std::collections::HashMap;

use super::super::ChatWidget;
use super::maieutic::MaieuticSpec;
use super::pipeline_coordinator::{cancel_pipeline_after_maieutic, resume_pipeline_after_maieutic};

/// Handle MaieuticSubmitted event from modal
///
/// Called when user completes all maieutic questions.
/// Builds a MaieuticSpec from the answers and resumes the pipeline.
pub fn on_maieutic_submitted(
    widget: &mut ChatWidget,
    spec_id: String,
    answers: HashMap<String, String>,
    duration_ms: u64,
) {
    tracing::info!(
        spec_id = %spec_id,
        duration_ms = duration_ms,
        answers_count = answers.len(),
        "Maieutic elicitation submitted"
    );

    // Get run_id from pending maieutic state
    let run_id = widget
        .pending_maieutic
        .as_ref()
        .map(|pm| pm.spec_id.clone()) // Use spec_id as fallback for run_id
        .unwrap_or_else(|| format!("{}-{}", spec_id, chrono::Utc::now().timestamp()));

    // Build MaieuticSpec from answers using the helper
    let maieutic_spec = MaieuticSpec::from_answers(spec_id.clone(), run_id, &answers, duration_ms);

    // Log the spec for debugging
    tracing::debug!(
        spec_id = %spec_id,
        goal = %maieutic_spec.goal,
        constraints_count = maieutic_spec.constraints.len(),
        "Maieutic spec built from answers"
    );

    // Resume the pipeline with the completed maieutic spec
    resume_pipeline_after_maieutic(widget, maieutic_spec);
}

/// Handle MaieuticCancelled event from modal
///
/// Called when user cancels the maieutic modal (Esc or Ctrl+C).
/// Aborts the pipeline.
pub fn on_maieutic_cancelled(widget: &mut ChatWidget, spec_id: &str) {
    tracing::info!(
        spec_id = %spec_id,
        "Maieutic elicitation cancelled by user"
    );

    // Cancel the pipeline
    cancel_pipeline_after_maieutic(widget, spec_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maieutic_spec_from_answers() {
        let mut answers = HashMap::new();
        answers.insert("goal".to_string(), "Implement feature X".to_string());
        answers.insert("constraints".to_string(), "No breaking changes".to_string());
        answers.insert("acceptance".to_string(), "All tests pass".to_string());

        let spec = MaieuticSpec::from_answers(
            "SPEC-TEST-001".to_string(),
            "run-001".to_string(),
            &answers,
            1000,
        );

        assert_eq!(spec.goal, "Implement feature X");
        assert!(!spec.constraints.is_empty());
        assert!(!spec.acceptance_criteria.is_empty());
    }

    #[test]
    fn test_maieutic_spec_defaults() {
        let answers = HashMap::new();

        let spec = MaieuticSpec::from_answers(
            "SPEC-TEST-002".to_string(),
            "run-002".to_string(),
            &answers,
            500,
        );

        // Should use defaults
        assert_eq!(spec.goal, "Not specified");
        assert!(!spec.acceptance_criteria.is_empty()); // Has default "All tests pass"
    }
}
