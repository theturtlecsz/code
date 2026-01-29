//! Project intake event handlers for /speckit.projectnew flow
//!
//! Handles completion and cancellation events from the ProjectIntakeModal.
//! Persists intake artifacts to capsule FIRST (SoR), then creates filesystem projections.
//!
//! ## Capsule SoR Pattern
//! 1. Open short-lived CapsuleHandle from canonical config
//! 2. put() ProjectIntakeAnswers and ProjectBrief artifacts
//! 3. emit_intake_completed() event with kind=Project
//! 4. commit_manual() checkpoint
//! 5. Hard-fail if any step fails (no filesystem-first fallback)
//! 6. Only then create filesystem projections (docs/PROJECT_BRIEF.md)
//!
//! Core logic is extracted to intake_core.rs for headless CLI reuse.

use std::collections::HashMap;

use ratatui::text::Line;
use uuid::Uuid;

use crate::chatwidget::ChatWidget;
use crate::history_cell::{HistoryCellType, PlainHistoryCell, new_error_event};

use super::commands::projectnew::ProjectNewPhase;
use super::grounding::capture_grounding_for_project_intake;
use super::intake_core::{
    build_project_brief, build_project_intake_answers, create_project_filesystem_projection,
    persist_project_intake_to_capsule, validate_project_answers,
};

/// Handle project intake submission
///
/// Called when user completes the ProjectIntakeModal.
/// Persists to capsule SoR, creates filesystem projection, then advances
/// the projectnew flow to bootstrap spec if requested.
pub fn on_project_intake_submitted(
    widget: &mut ChatWidget,
    project_id: String,
    deep: bool,
    answers: HashMap<String, String>,
) {
    let intake_id = Uuid::new_v4().to_string();
    let created_via = "project_intake_modal";

    // Step 1: Validate answers (using intake_core)
    let validation = validate_project_answers(&answers, deep);
    if !validation.valid {
        let error_msg = validation.errors.join("\n  - ");
        widget.history_push(new_error_event(format!(
            "Project intake validation failed:\n  - {}",
            error_msg
        )));
        widget.request_redraw();
        widget.pending_projectnew = None;
        return;
    }

    // Step 1.5: Deep grounding capture (Phase 3B)
    // In deep mode, capture grounding artifacts before building the brief
    let grounding_uris = if deep {
        // Show progress message
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from("Deep grounding in progress..."),
                Line::from("   Running Architect Harvest + Project Intel"),
            ],
            HistoryCellType::Notice,
        ));
        widget.request_redraw();

        match capture_grounding_for_project_intake(&widget.config.cwd, &project_id) {
            Ok(result) => {
                widget.history_push(PlainHistoryCell::new(
                    vec![Line::from(format!(
                        "Deep grounding complete: {} artifacts captured",
                        result.grounding_uris.len()
                    ))],
                    HistoryCellType::Notice,
                ));
                result.grounding_uris
            }
            Err(e) => {
                // Deep grounding failure blocks completion (SoR integrity)
                widget.history_push(new_error_event(format!(
                    "Deep grounding failed: {}\n\nDeep mode requires grounding data for capsule SoR integrity.",
                    e
                )));
                widget.request_redraw();
                widget.pending_projectnew = None;
                return;
            }
        }
    } else {
        Vec::new()
    };

    // Step 2: Build structs (using intake_core)
    let intake_answers = build_project_intake_answers(&answers, deep);
    let project_brief = build_project_brief(
        &answers,
        &project_id,
        &intake_id,
        deep,
        created_via,
        grounding_uris,
    );

    // Step 3: Persist to capsule (SoR-first, using intake_core)
    let capsule_result = match persist_project_intake_to_capsule(
        &widget.config.cwd,
        &project_id,
        &intake_id,
        &intake_answers,
        &project_brief,
        deep,
        created_via,
    ) {
        Ok(r) => r,
        Err(e) => {
            widget.history_push(new_error_event(format!(
                "Project intake capsule persistence failed: {}",
                e
            )));
            widget.request_redraw();
            // Clear pending projectnew state on failure
            widget.pending_projectnew = None;
            return;
        }
    };

    // Step 4: Create filesystem projection (using intake_core)
    if let Err(e) = create_project_filesystem_projection(
        &widget.config.cwd,
        &project_id,
        &project_brief,
        &capsule_result,
        deep,
    ) {
        // Log warning but don't fail - capsule is SoR
        tracing::warn!("Failed to create PROJECT_BRIEF.md: {}", e);
    }

    // Step 5: Check if we need to bootstrap a spec
    // Contract: bootstrap happens by default unless --no-bootstrap-spec is specified
    let should_bootstrap = widget
        .pending_projectnew
        .as_ref()
        .map_or(false, |p| !p.no_bootstrap_spec);

    if should_bootstrap {
        // Get bootstrap description and deep flag
        let (bootstrap_desc, inherit_deep) = {
            let pending = widget.pending_projectnew.as_ref().unwrap();
            (
                pending
                    .bootstrap_desc
                    .clone()
                    .unwrap_or_else(|| "Initial setup".to_string()),
                pending.deep,
            )
        };

        // Advance phase
        if let Some(ref mut pending) = widget.pending_projectnew {
            pending.phase = ProjectNewPhase::BootstrapSpecPending;
        }

        // Show success message for project intake
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from(format!("Project intake completed for: {}", project_id)),
                Line::from(""),
                Line::from(format!("   Intake ID: {}", intake_id)),
                Line::from(format!("   Answers: {}", capsule_result.answers_uri)),
                Line::from(format!("   Brief: {}", capsule_result.brief_uri)),
                Line::from(""),
                Line::from("Starting bootstrap spec intake..."),
            ],
            HistoryCellType::Notice,
        ));

        // Show spec intake modal with bootstrap description
        // Inherit deep flag from projectnew
        widget.show_spec_intake_modal(bootstrap_desc, inherit_deep);
    } else {
        // No bootstrap - complete the flow
        widget.pending_projectnew = None;

        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from(format!("Project setup completed: {}", project_id)),
                Line::from(""),
                Line::from(format!("   Intake ID: {}", intake_id)),
                Line::from(format!("   Answers: {}", capsule_result.answers_uri)),
                Line::from(format!("   Brief: {}", capsule_result.brief_uri)),
                Line::from("   Projection: docs/PROJECT_BRIEF.md"),
                Line::from(""),
                Line::from("Next steps:"),
                Line::from("   /speckit.new <feature description>"),
            ],
            HistoryCellType::Notice,
        ));
    }

    widget.request_redraw();
}

/// Handle project intake cancellation
///
/// Called when user cancels the ProjectIntakeModal.
/// Aborts the projectnew flow but leaves scaffold in place.
pub fn on_project_intake_cancelled(widget: &mut ChatWidget, project_id: String) {
    // Clear pending state
    widget.pending_projectnew = None;

    widget.history_push(PlainHistoryCell::new(
        vec![
            Line::from(format!("Project intake cancelled for: {}", project_id)),
            Line::from(""),
            Line::from("Project scaffold remains in place."),
            Line::from("Vision constitution has been captured."),
            Line::from(""),
            Line::from("To resume setup, run:"),
            Line::from("   /speckit.vision   (if not already captured)"),
            Line::from("   /speckit.new <feature description>"),
        ],
        HistoryCellType::Notice,
    ));
    widget.request_redraw();
}
