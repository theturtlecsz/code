//! Spec intake event handlers (Architect-in-a-box, Phase 1)
//!
//! Handles completion and cancellation events from the SpecIntakeModal.
//! Persists intake artifacts to capsule FIRST (SoR), then creates filesystem projections.
//!
//! ## Capsule SoR Pattern
//! 1. Open short-lived CapsuleHandle from canonical config
//! 2. put() SpecIntakeAnswers and DesignBrief artifacts
//! 3. emit_intake_completed() event
//! 4. commit_manual() checkpoint
//! 5. Hard-fail if any step fails (no filesystem-first fallback)
//! 6. Only then create filesystem projections
//!
//! Core logic is extracted to intake_core.rs for headless CLI reuse.

use std::collections::HashMap;

use ratatui::text::Line;
use uuid::Uuid;

use crate::chatwidget::ChatWidget;
use crate::history_cell::{new_error_event, HistoryCellType, PlainHistoryCell};

use super::grounding::capture_grounding_for_spec_intake;
use super::intake_core::{
    build_design_brief, build_spec_intake_answers, capitalize_words,
    create_spec_filesystem_projections, persist_spec_intake_to_capsule,
    validate_spec_answers, write_intake_md_only,
};
use super::spec_id_generator::generate_next_spec_id;

/// Called when user completes the spec intake modal
pub fn on_spec_intake_submitted(
    widget: &mut ChatWidget,
    description: String,
    deep: bool,
    answers: HashMap<String, String>,
    existing_spec_id: Option<String>,
) {
    let is_backfill = existing_spec_id.is_some();

    // Step 1: Validate answers (using intake_core)
    let validation = validate_spec_answers(&answers, deep);
    if !validation.valid {
        let error_msg = validation.errors.join("\n  - ");
        widget.history_push(new_error_event(format!(
            "Intake validation failed:\n  - {}",
            error_msg
        )));
        widget.request_redraw();
        return;
    }

    // Step 2: Generate or use existing spec_id
    let intake_id = Uuid::new_v4().to_string();
    let spec_id = match existing_spec_id {
        Some(id) => id, // Backfill mode: use existing spec_id
        None => {
            // New spec mode: generate new ID
            match generate_next_spec_id(&widget.config.cwd) {
                Ok(id) => id,
                Err(e) => {
                    widget.history_push(new_error_event(format!(
                        "Failed to generate SPEC-ID: {}",
                        e
                    )));
                    widget.request_redraw();
                    return;
                }
            }
        }
    };

    // Step 3: Determine created_via based on mode
    let created_via = if is_backfill {
        "spec_intake_modal_backfill"
    } else {
        "spec_intake_modal"
    };

    // Step 3.5: Deep grounding capture (Phase 3B)
    // In deep mode (and not backfill), capture grounding artifacts before building the brief
    let grounding_uris = if deep && !is_backfill {
        // Show progress message
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from("Deep grounding in progress..."),
                Line::from("   Running Architect Harvest + Project Intel"),
            ],
            HistoryCellType::Notice,
        ));
        widget.request_redraw();

        match capture_grounding_for_spec_intake(&widget.config.cwd, &spec_id, &intake_id) {
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
                return;
            }
        }
    } else {
        Vec::new()
    };

    // Step 4: Build structs (using intake_core)
    let intake_answers = build_spec_intake_answers(&answers, deep, validation.warnings);
    let design_brief = match build_design_brief(
        &answers,
        &spec_id,
        &intake_id,
        &description,
        deep,
        created_via,
        grounding_uris,
    ) {
        Ok(b) => b,
        Err(e) => {
            widget.history_push(new_error_event(format!(
                "Failed to build design brief: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Step 5: Persist to capsule (SoR) - hard-fail if this fails (using intake_core)
    let capsule_result = match persist_spec_intake_to_capsule(
        &widget.config.cwd,
        &spec_id,
        &intake_id,
        &intake_answers,
        &design_brief,
        deep,
        created_via,
    ) {
        Ok(result) => result,
        Err(e) => {
            widget.history_push(new_error_event(format!(
                "Capsule persistence failed: {}",
                e
            )));
            widget.request_redraw();
            return;
        }
    };

    // Step 6: Handle filesystem projections (different for backfill vs new spec)
    if is_backfill {
        // Backfill mode: Only write INTAKE.md to existing spec directory (using intake_core)
        match write_intake_md_only(
            &widget.config.cwd,
            &spec_id,
            &design_brief,
            &capsule_result,
        ) {
            Ok(()) => {
                widget.history_push(PlainHistoryCell::new(
                    vec![
                        Line::from(format!("Backfill complete for {}", spec_id)),
                        Line::from(""),
                        Line::from(format!("   Intake ID: {}", intake_id)),
                        Line::from("   Written: INTAKE.md"),
                        Line::from(format!("   Answers URI: {}", capsule_result.answers_uri)),
                        Line::from(format!("   Brief URI: {}", capsule_result.brief_uri)),
                    ],
                    HistoryCellType::Notice,
                ));
            }
            Err(e) => {
                widget.history_push(new_error_event(format!(
                    "Failed to write INTAKE.md: {}",
                    e
                )));
                widget.request_redraw();
                return;
            }
        }

        // Resume pipeline if this was triggered by IntakePresenceGate
        super::pipeline_coordinator::resume_pipeline_after_intake_backfill(widget);
    } else {
        // New spec mode: Full filesystem projection (using intake_core)
        let dir_name = match create_spec_filesystem_projections(
            &widget.config.cwd,
            &spec_id,
            &description,
            &design_brief,
            &capsule_result,
        ) {
            Ok(d) => d,
            Err(e) => {
                widget.history_push(new_error_event(format!(
                    "Filesystem projection failed (capsule SoR exists): {}",
                    e
                )));
                widget.request_redraw();
                return;
            }
        };

        // Step 7: Success message
        let feature_name = capitalize_words(&description);
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from(format!("Created {}: {}", spec_id, feature_name)),
                Line::from(""),
                Line::from(format!("   Directory: docs/{}/", dir_name)),
                Line::from("   Files: spec.md, PRD.md, INTAKE.md"),
                Line::from("   Updated: SPEC.md tracker"),
                Line::from(""),
                Line::from(format!("   Intake ID: {}", intake_id)),
                Line::from(format!("   Answers URI: {}", capsule_result.answers_uri)),
                Line::from(format!("   Brief URI: {}", capsule_result.brief_uri)),
                Line::from(""),
                Line::from("Next steps:"),
                Line::from(format!(
                    "   /speckit.clarify {} - resolve ambiguities",
                    spec_id
                )),
                Line::from(format!("   /speckit.auto {} - full pipeline", spec_id)),
            ],
            HistoryCellType::Notice,
        ));
    }

    // Check for pending projectnew bootstrap completion
    if let Some(ref pending) = widget.pending_projectnew {
        if pending.phase == super::commands::projectnew::ProjectNewPhase::BootstrapSpecPending {
            // Projectnew flow is now complete
            widget.pending_projectnew = None;
            // Success message already shown by the spec intake handler
        }
    }

    widget.request_redraw();
}

/// Called when user cancels the spec intake modal
pub fn on_spec_intake_cancelled(
    widget: &mut ChatWidget,
    description: String,
    existing_spec_id: Option<String>,
) {
    if let Some(spec_id) = existing_spec_id {
        // Backfill mode: cancel pipeline
        widget.history_push(PlainHistoryCell::new(
            vec![
                Line::from(format!("Intake backfill cancelled for {}", spec_id)),
                Line::from(""),
                Line::from("Pipeline cancelled - intake required for /speckit.auto"),
            ],
            HistoryCellType::Notice,
        ));
        // Cancel the pending pipeline
        super::pipeline_coordinator::cancel_pipeline_after_intake_backfill(widget, &spec_id);
    } else {
        // Check if this is a bootstrap spec cancellation during projectnew
        let was_bootstrap = widget.pending_projectnew.as_ref().map_or(false, |p| {
            p.phase == super::commands::projectnew::ProjectNewPhase::BootstrapSpecPending
        });

        if was_bootstrap {
            // Clear pending state - project setup is complete (without bootstrap spec)
            widget.pending_projectnew = None;
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Bootstrap spec cancelled"),
                    Line::from(""),
                    Line::from("Project setup completed without bootstrap spec."),
                    Line::from(""),
                    Line::from("Next steps:"),
                    Line::from("   /speckit.new <description> - Create a spec when ready"),
                ],
                HistoryCellType::Notice,
            ));
        } else {
            // Normal new spec mode: just show cancellation message
            widget.history_push(PlainHistoryCell::new(
                vec![
                    Line::from("Spec intake cancelled"),
                    Line::from(format!("   Description: {}", description)),
                    Line::from(""),
                    Line::from("To try again: /speckit.new <description> [--deep]"),
                ],
                HistoryCellType::Notice,
            ));
        }
    }
    widget.request_redraw();
}
