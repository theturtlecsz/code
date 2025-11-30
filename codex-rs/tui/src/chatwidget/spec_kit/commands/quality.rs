//! Quality command implementations (clarify, analyze, checklist)
//!
//! FORK-SPECIFIC (just-every/code): Native quality heuristics (zero agents, zero cost)
//! Eliminates ALL agent usage from pattern-matching quality checks

use super::super::super::ChatWidget;
use super::super::analyze_native;
use super::super::checklist_native;
use super::super::clarify_native;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell;

/// Command: /speckit.clarify
/// Interactive clarification resolution + native ambiguity detection
///
/// Two modes:
/// 1. If [NEEDS CLARIFICATION: ...] markers exist → interactive modal
/// 2. Otherwise → native ambiguity heuristics (zero agents, <1s, FREE)
pub struct SpecKitClarifyCommand;

impl SpecKitCommand for SpecKitClarifyCommand {
    fn name(&self) -> &'static str {
        "speckit.clarify"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "resolve clarification markers & detect ambiguities"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = args.split_whitespace().next().unwrap_or("");
        if spec_id.is_empty() {
            widget.history_push(history_cell::new_error_event(
                "Usage: /speckit.clarify SPEC-ID".to_string(),
            ));
            return;
        }

        // First, check for [NEEDS CLARIFICATION] markers
        match clarify_native::find_clarification_markers(spec_id, &widget.config.cwd) {
            Ok(markers) if !markers.is_empty() => {
                // Convert to modal questions and launch interactive resolution
                let questions: Vec<crate::bottom_pane::clarify_modal::ClarifyQuestion> = markers
                    .into_iter()
                    .map(|m| crate::bottom_pane::clarify_modal::ClarifyQuestion {
                        id: m.id,
                        question: m.question,
                        file_path: m.file_path,
                        line_number: m.line_number,
                        original_text: m.original_text,
                    })
                    .collect();

                widget.history_push(history_cell::PlainHistoryCell::new(
                    vec![ratatui::text::Line::from(format!(
                        "Found {} clarification marker{} in {} - launching interactive resolution...",
                        questions.len(),
                        if questions.len() == 1 { "" } else { "s" },
                        spec_id
                    ))],
                    history_cell::HistoryCellType::Notice,
                ));

                widget.show_clarify_modal(spec_id.to_string(), questions);
                return;
            }
            Ok(_) => {
                // No markers - fall through to ambiguity detection
            }
            Err(err) => {
                widget.history_push(history_cell::new_error_event(format!(
                    "Failed to scan for markers: {}",
                    err
                )));
                return;
            }
        }

        // No markers found - use NATIVE ambiguity heuristics
        match clarify_native::find_ambiguities(spec_id, &widget.config.cwd) {
            Ok(ambiguities) => {
                display_clarify_results(widget, spec_id, ambiguities);
            }
            Err(err) => {
                widget.history_push(history_cell::new_error_event(format!(
                    "Clarify failed: {}",
                    err
                )));
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.analyze
/// Native consistency checking (zero agents, <1s, FREE)
pub struct SpecKitAnalyzeCommand;

impl SpecKitCommand for SpecKitAnalyzeCommand {
    fn name(&self) -> &'static str {
        "speckit.analyze"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "check cross-artifact consistency (native)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = args.split_whitespace().next().unwrap_or("");
        if spec_id.is_empty() {
            widget.history_push(history_cell::new_error_event(
                "Usage: /speckit.analyze SPEC-ID".to_string(),
            ));
            return;
        }

        // Use NATIVE consistency checking (no agents!)
        match analyze_native::check_consistency(spec_id, &widget.config.cwd) {
            Ok(issues) => {
                display_analyze_results(widget, spec_id, issues);
            }
            Err(err) => {
                widget.history_push(history_cell::new_error_event(format!(
                    "Analyze failed: {}",
                    err
                )));
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Command: /speckit.checklist
/// Native quality scoring (zero agents, <1s, FREE)
pub struct SpecKitChecklistCommand;

impl SpecKitCommand for SpecKitChecklistCommand {
    fn name(&self) -> &'static str {
        "speckit.checklist"
    }

    fn aliases(&self) -> &[&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "evaluate requirement quality (native scoring)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let spec_id = args.split_whitespace().next().unwrap_or("");
        if spec_id.is_empty() {
            widget.history_push(history_cell::new_error_event(
                "Usage: /speckit.checklist SPEC-ID".to_string(),
            ));
            return;
        }

        // Use NATIVE quality scoring (no agents!)
        match checklist_native::score_quality(spec_id, &widget.config.cwd) {
            Ok(report) => {
                display_checklist_results(widget, spec_id, report);
            }
            Err(err) => {
                widget.history_push(history_cell::new_error_event(format!(
                    "Checklist failed: {}",
                    err
                )));
            }
        }
    }

    fn requires_args(&self) -> bool {
        true
    }
}

/// Display clarify results in TUI
fn display_clarify_results(
    widget: &mut ChatWidget,
    spec_id: &str,
    ambiguities: Vec<clarify_native::Ambiguity>,
) {
    if ambiguities.is_empty() {
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "✅ No ambiguities found in {}",
                spec_id
            ))],
            history_cell::HistoryCellType::Notice,
        ));
        return;
    }

    // Header
    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "🔍 Found {} ambiguities in {}:",
            ambiguities.len(),
            spec_id
        ))],
        history_cell::HistoryCellType::Notice,
    ));

    // Group by severity
    let critical: Vec<_> = ambiguities
        .iter()
        .filter(|a| matches!(a.severity, clarify_native::Severity::Critical))
        .collect();
    let important: Vec<_> = ambiguities
        .iter()
        .filter(|a| matches!(a.severity, clarify_native::Severity::Important))
        .collect();
    let minor: Vec<_> = ambiguities
        .iter()
        .filter(|a| matches!(a.severity, clarify_native::Severity::Minor))
        .collect();

    // Display by severity
    if !critical.is_empty() {
        widget.history_push(history_cell::new_error_event(format!(
            "CRITICAL ({})",
            critical.len()
        )));
        for amb in critical {
            let mut lines = vec![
                ratatui::text::Line::from(format!("  {} [{}]", amb.id, amb.pattern)),
                ratatui::text::Line::from(format!("  Question: {}", amb.question)),
                ratatui::text::Line::from(format!("  Location: {}", amb.location)),
            ];
            if !amb.context.is_empty() {
                lines.push(ratatui::text::Line::from(format!(
                    "  Context: {}",
                    amb.context
                )));
            }
            if let Some(suggestion) = &amb.suggestion {
                lines.push(ratatui::text::Line::from(format!(
                    "  Suggestion: {}",
                    suggestion
                )));
            }
            lines.push(ratatui::text::Line::from(""));

            widget.history_push(history_cell::PlainHistoryCell::new(
                lines,
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    if !important.is_empty() {
        widget.history_push(history_cell::new_warning_event(format!(
            "IMPORTANT ({})",
            important.len()
        )));
        for amb in important {
            let mut lines = vec![
                ratatui::text::Line::from(format!("  {} [{}]", amb.id, amb.pattern)),
                ratatui::text::Line::from(format!("  Question: {}", amb.question)),
                ratatui::text::Line::from(format!("  Location: {}", amb.location)),
            ];
            if let Some(suggestion) = &amb.suggestion {
                lines.push(ratatui::text::Line::from(format!(
                    "  Suggestion: {}",
                    suggestion
                )));
            }
            lines.push(ratatui::text::Line::from(""));

            widget.history_push(history_cell::PlainHistoryCell::new(
                lines,
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    if !minor.is_empty() {
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "MINOR ({})",
                minor.len()
            ))],
            history_cell::HistoryCellType::Notice,
        ));
        for amb in minor.iter().take(5) {
            // Limit minor display
            let lines = vec![
                ratatui::text::Line::from(format!("  {} [{}]", amb.id, amb.pattern)),
                ratatui::text::Line::from(format!("  Question: {}", amb.question)),
                ratatui::text::Line::from(format!("  Location: {}", amb.location)),
                ratatui::text::Line::from(""),
            ];

            widget.history_push(history_cell::PlainHistoryCell::new(
                lines,
                history_cell::HistoryCellType::Notice,
            ));
        }
        if minor.len() > 5 {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  ... and {} more minor issues",
                    minor.len() - 5
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Cost savings: $0.80 (zero agents used)"),
        ],
        history_cell::HistoryCellType::Notice,
    ));
}

/// Display analyze results in TUI
fn display_analyze_results(
    widget: &mut ChatWidget,
    spec_id: &str,
    issues: Vec<analyze_native::InconsistencyIssue>,
) {
    if issues.is_empty() {
        widget.history_push(history_cell::new_warning_event(format!(
            "No consistency issues found in {}",
            spec_id
        )));
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(
                "Cost savings: $0.80 (zero agents used)",
            )],
            history_cell::HistoryCellType::Notice,
        ));
        return;
    }

    // Header
    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "🔍 Found {} consistency issues in {}:",
            issues.len(),
            spec_id
        ))],
        history_cell::HistoryCellType::Notice,
    ));

    // Group by severity
    let critical: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.severity, clarify_native::Severity::Critical))
        .collect();
    let important: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.severity, clarify_native::Severity::Important))
        .collect();
    let minor: Vec<_> = issues
        .iter()
        .filter(|i| matches!(i.severity, clarify_native::Severity::Minor))
        .collect();

    // Display by severity
    if !critical.is_empty() {
        widget.history_push(history_cell::new_error_event(format!(
            "\n❌ CRITICAL ({}):",
            critical.len()
        )));
        for issue in critical {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  {} [{}]\n    {}\n    {} @ {} → {} @ {}\n    Fix: {}",
                    issue.id,
                    issue.issue_type,
                    issue.description,
                    issue.source_file,
                    issue.source_location,
                    issue.target_file,
                    issue.target_location,
                    issue.suggested_fix.as_ref().unwrap_or(&"N/A".to_string())
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    if !important.is_empty() {
        widget.history_push(history_cell::new_warning_event(format!(
            "\n⚠️  IMPORTANT ({}):",
            important.len()
        )));
        for issue in important {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  {} [{}] {}\n    {} → {}",
                    issue.id,
                    issue.issue_type,
                    issue.description,
                    issue.source_file,
                    issue.target_file
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    if !minor.is_empty() {
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![ratatui::text::Line::from(format!(
                "\nℹ️  MINOR ({}):",
                minor.len()
            ))],
            history_cell::HistoryCellType::Notice,
        ));
        for issue in minor.iter().take(3) {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  {} [{}] {}",
                    issue.id, issue.issue_type, issue.description
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
        if minor.len() > 3 {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  ... and {} more minor issues",
                    minor.len() - 3
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(
            "\n💡 Cost savings: $0.80 (zero agents used)".to_string(),
        )],
        history_cell::HistoryCellType::Notice,
    ));
}

/// Display checklist results in TUI
fn display_checklist_results(
    widget: &mut ChatWidget,
    spec_id: &str,
    report: checklist_native::QualityReport,
) {
    // Header with overall score
    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![ratatui::text::Line::from(format!(
            "Quality Report for {}: {} ({:.1}%)",
            spec_id,
            report.grade(),
            report.overall_score
        ))],
        history_cell::HistoryCellType::Notice,
    ));

    // Score breakdown
    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Scores:"),
            ratatui::text::Line::from(format!("  Completeness:  {:.1}%", report.completeness)),
            ratatui::text::Line::from(format!("  Clarity:       {:.1}%", report.clarity)),
            ratatui::text::Line::from(format!("  Testability:   {:.1}%", report.testability)),
            ratatui::text::Line::from(format!("  Consistency:   {:.1}%", report.consistency)),
        ],
        history_cell::HistoryCellType::Notice,
    ));

    // Issues
    if !report.issues.is_empty() {
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from(""),
                ratatui::text::Line::from(format!("Issues ({})", report.issues.len())),
            ],
            history_cell::HistoryCellType::Notice,
        ));

        let critical: Vec<_> = report
            .issues
            .iter()
            .filter(|i| matches!(i.severity, clarify_native::Severity::Critical))
            .collect();
        let important: Vec<_> = report
            .issues
            .iter()
            .filter(|i| matches!(i.severity, clarify_native::Severity::Important))
            .collect();

        for issue in critical.iter().chain(important.iter()).take(5) {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![
                    ratatui::text::Line::from(format!("  {} [{}]", issue.id, issue.category)),
                    ratatui::text::Line::from(format!("  {}", issue.description)),
                    ratatui::text::Line::from(format!("  Impact: {}", issue.impact)),
                    ratatui::text::Line::from(format!("  Suggestion: {}", issue.suggestion)),
                    ratatui::text::Line::from(""),
                ],
                history_cell::HistoryCellType::Notice,
            ));
        }

        if report.issues.len() > 5 {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!(
                    "  ... and {} more issues",
                    report.issues.len() - 5
                ))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    // Recommendations
    if !report.recommendations.is_empty() {
        widget.history_push(history_cell::PlainHistoryCell::new(
            vec![
                ratatui::text::Line::from(""),
                ratatui::text::Line::from("Recommendations:"),
            ],
            history_cell::HistoryCellType::Notice,
        ));
        for rec in &report.recommendations {
            widget.history_push(history_cell::PlainHistoryCell::new(
                vec![ratatui::text::Line::from(format!("  - {}", rec))],
                history_cell::HistoryCellType::Notice,
            ));
        }
    }

    widget.history_push(history_cell::PlainHistoryCell::new(
        vec![
            ratatui::text::Line::from(""),
            ratatui::text::Line::from("Cost savings: $0.35 (zero agents used)"),
        ],
        history_cell::HistoryCellType::Notice,
    ));
}
