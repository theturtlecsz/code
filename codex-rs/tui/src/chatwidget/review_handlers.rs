// MAINT-11 Phase 7: Review handler functions extracted from mod.rs
// Contains PR review and code review functionality

use codex_core::git_info::CommitLogEntry;
use codex_core::protocol::Op;
use codex_core::protocol::ReviewOutputEvent;
use codex_core::protocol::{ReviewContextMetadata, ReviewRequest};
use codex_core::review_format::format_review_findings_block;

// AppEvent accessed via crate::app_event::AppEvent inline
use crate::bottom_pane::CustomPromptView;
use crate::bottom_pane::list_selection_view::{ListSelectionView, SelectionItem};
use crate::history_cell;

use super::ChatWidget;

impl ChatWidget<'_> {
    pub(crate) fn open_review_dialog(&mut self) {
        if self.is_task_running() {
            self.history_push(crate::history_cell::new_error_event(
                "`/review` — complete or cancel the current task before starting a new review."
                    .to_string(),
            ));
            self.request_redraw();
            return;
        }

        let mut items: Vec<SelectionItem> = Vec::new();

        items.push(SelectionItem {
            name: "Review current workspace changes".to_string(),
            description: Some("Include staged, unstaged, and untracked files".to_string()),
            is_current: false,
            actions: vec![Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(crate::app_event::AppEvent::RunReviewCommand(String::new()));
            })],
        });

        items.push(SelectionItem {
            name: "Review a specific commit".to_string(),
            description: Some("Pick from recent commits".to_string()),
            is_current: false,
            actions: vec![Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(crate::app_event::AppEvent::StartReviewCommitPicker);
            })],
        });

        items.push(SelectionItem {
            name: "Review against a base branch".to_string(),
            description: Some("Diff current branch against another".to_string()),
            is_current: false,
            actions: vec![Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(crate::app_event::AppEvent::StartReviewBranchPicker);
            })],
        });

        items.push(SelectionItem {
            name: "Custom review instructions".to_string(),
            description: Some("Describe exactly what to audit".to_string()),
            is_current: false,
            actions: vec![Box::new(|tx: &crate::app_event_sender::AppEventSender| {
                tx.send(crate::app_event::AppEvent::OpenReviewCustomPrompt);
            })],
        });

        let view: ListSelectionView = ListSelectionView::new(
            " Review options ".to_string(),
            Some("Choose what scope to review".to_string()),
            Some("Enter select · Esc cancel".to_string()),
            items,
            self.app_event_tx.clone(),
            6,
        );

        self.bottom_pane
            .show_list_selection("Review options".to_string(), None, None, view);
    }

    pub(crate) fn show_review_custom_prompt(&mut self) {
        let submit_tx = self.app_event_tx.clone();
        let on_submit: Box<dyn Fn(String) + Send + Sync> = Box::new(move |text: String| {
            submit_tx.send(crate::app_event::AppEvent::RunReviewCommand(text));
        });
        let view = CustomPromptView::new(
            "Custom review instructions".to_string(),
            "Describe the files or changes you want reviewed".to_string(),
            Some("Press Enter to submit · Esc cancel".to_string()),
            self.app_event_tx.clone(),
            None,
            on_submit,
        );
        self.bottom_pane.show_custom_prompt(view);
    }

    pub(crate) fn show_review_commit_loading(&mut self) {
        let loading_item = SelectionItem {
            name: "Loading recent commits…".to_string(),
            description: None,
            is_current: true,
            actions: Vec::new(),
        };
        let view = ListSelectionView::new(
            " Select a commit ".to_string(),
            Some("Fetching recent commits from git".to_string()),
            Some("Esc cancel".to_string()),
            vec![loading_item],
            self.app_event_tx.clone(),
            6,
        );
        self.bottom_pane
            .show_list_selection("Select a commit".to_string(), None, None, view);
    }

    pub(crate) fn present_review_commit_picker(&mut self, commits: Vec<CommitLogEntry>) {
        if commits.is_empty() {
            self.bottom_pane
                .flash_footer_notice("No recent commits found for review".to_string());
            self.request_redraw();
            return;
        }

        let mut items: Vec<SelectionItem> = Vec::with_capacity(commits.len());
        for entry in commits {
            let subject = entry.subject.trim().to_string();
            let sha = entry.sha.trim().to_string();
            if sha.is_empty() {
                continue;
            }
            let short_sha: String = sha.chars().take(7).collect();
            let title = if subject.is_empty() {
                short_sha.clone()
            } else {
                format!("{short_sha} — {subject}")
            };
            let prompt = if subject.is_empty() {
                format!(
                    "Review the code changes introduced by commit {sha}. Provide prioritized, actionable findings."
                )
            } else {
                format!(
                    "Review the code changes introduced by commit {sha} (\"{subject}\"). Provide prioritized, actionable findings."
                )
            };
            let hint = format!("commit {short_sha}");
            let preparation = format!("Preparing code review for commit {short_sha}");
            let prompt_closure = prompt.clone();
            let hint_closure = hint.clone();
            let prep_closure = preparation.clone();
            let metadata_option = Some(ReviewContextMetadata {
                scope: Some("commit".to_string()),
                commit: Some(sha.clone()),
                ..Default::default()
            });
            items.push(SelectionItem {
                name: title,
                description: None,
                is_current: false,
                actions: vec![Box::new(
                    move |tx: &crate::app_event_sender::AppEventSender| {
                        tx.send(crate::app_event::AppEvent::RunReviewWithScope {
                            prompt: prompt_closure.clone(),
                            hint: hint_closure.clone(),
                            preparation_label: Some(prep_closure.clone()),
                            metadata: metadata_option.clone(),
                        });
                    },
                )],
            });
        }

        if items.is_empty() {
            self.bottom_pane
                .flash_footer_notice("No recent commits found for review".to_string());
            self.request_redraw();
            return;
        }

        let view = ListSelectionView::new(
            " Select a commit ".to_string(),
            Some("Choose a commit to review".to_string()),
            Some("Enter select · Esc cancel".to_string()),
            items,
            self.app_event_tx.clone(),
            10,
        );

        self.bottom_pane.show_list_selection(
            "Select a commit to review".to_string(),
            None,
            None,
            view,
        );
    }

    pub(crate) fn show_review_branch_loading(&mut self) {
        let loading_item = SelectionItem {
            name: "Loading local branches…".to_string(),
            description: None,
            is_current: true,
            actions: Vec::new(),
        };
        let view = ListSelectionView::new(
            " Select a base branch ".to_string(),
            Some("Fetching local branches".to_string()),
            Some("Esc cancel".to_string()),
            vec![loading_item],
            self.app_event_tx.clone(),
            6,
        );
        self.bottom_pane
            .show_list_selection("Select a base branch".to_string(), None, None, view);
    }

    pub(crate) fn present_review_branch_picker(
        &mut self,
        current_branch: Option<String>,
        branches: Vec<String>,
    ) {
        let current_trimmed = current_branch.as_ref().map(|s| s.trim().to_string());
        let mut items: Vec<SelectionItem> = Vec::new();
        for branch in branches {
            let branch_trimmed = branch.trim();
            if branch_trimmed.is_empty() {
                continue;
            }
            if current_trimmed
                .as_ref()
                .is_some_and(|current| current == branch_trimmed)
            {
                continue;
            }

            let title = if let Some(current) = current_trimmed.as_ref() {
                format!("{current} → {branch_trimmed}")
            } else {
                format!("Compare against {branch_trimmed}")
            };

            let prompt = if let Some(current) = current_trimmed.as_ref() {
                format!(
                    "Review the code changes between the current branch '{current}' and '{branch_trimmed}'. Identify bugs, regressions, risky patterns, and missing tests before merging."
                )
            } else {
                format!(
                    "Review the code changes that would merge into '{branch_trimmed}'. Identify bugs, regressions, risky patterns, and missing tests before merge."
                )
            };
            let hint = format!("against {branch_trimmed}");
            let preparation = format!("Preparing code review against {branch_trimmed}");
            let prompt_closure = prompt.clone();
            let hint_closure = hint.clone();
            let prep_closure = preparation.clone();
            let metadata_option = Some(ReviewContextMetadata {
                scope: Some("branch_diff".to_string()),
                base_branch: Some(branch_trimmed.to_string()),
                current_branch: current_trimmed.clone(),
                ..Default::default()
            });
            items.push(SelectionItem {
                name: title,
                description: None,
                is_current: false,
                actions: vec![Box::new(
                    move |tx: &crate::app_event_sender::AppEventSender| {
                        tx.send(crate::app_event::AppEvent::RunReviewWithScope {
                            prompt: prompt_closure.clone(),
                            hint: hint_closure.clone(),
                            preparation_label: Some(prep_closure.clone()),
                            metadata: metadata_option.clone(),
                        });
                    },
                )],
            });
        }

        if items.is_empty() {
            self.bottom_pane
                .flash_footer_notice("No alternative branches found for review".to_string());
            self.request_redraw();
            return;
        }

        let subtitle = current_trimmed
            .as_ref()
            .map(|current| format!("Current branch: {current}"));

        let view = ListSelectionView::new(
            " Select a base branch ".to_string(),
            subtitle,
            Some("Enter select · Esc cancel".to_string()),
            items,
            self.app_event_tx.clone(),
            10,
        );

        self.bottom_pane.show_list_selection(
            "Compare against a branch".to_string(),
            None,
            None,
            view,
        );
    }

    /// Handle `/review [focus]` command by starting a dedicated review session.
    pub(crate) fn handle_review_command(&mut self, args: String) {
        if self.is_task_running() {
            self.history_push(crate::history_cell::new_error_event(
                "`/review` — complete or cancel the current task before starting a new review."
                    .to_string(),
            ));
            self.request_redraw();
            return;
        }

        let trimmed = args.trim();
        if trimmed.is_empty() {
            let metadata = ReviewContextMetadata {
                scope: Some("workspace".to_string()),
                ..Default::default()
            };
            self.start_review_with_scope(
                "Review the current workspace changes and highlight bugs, regressions, risky patterns, and missing tests before merge.".to_string(),
                "current workspace changes".to_string(),
                Some("Preparing code review request...".to_string()),
                Some(metadata),
            );
        } else {
            let value = trimmed.to_string();
            let preparation = format!("Preparing code review for {value}");
            let metadata = ReviewContextMetadata {
                scope: Some("custom".to_string()),
                ..Default::default()
            };
            self.start_review_with_scope(value.clone(), value, Some(preparation), Some(metadata));
        }
    }

    pub(crate) fn start_review_with_scope(
        &mut self,
        prompt: String,
        hint: String,
        preparation_label: Option<String>,
        metadata: Option<ReviewContextMetadata>,
    ) {
        self.active_review_hint = None;
        self.active_review_prompt = None;

        let trimmed_hint = hint.trim();
        let preparation_notice = preparation_label.unwrap_or_else(|| {
            if trimmed_hint.is_empty() {
                "Preparing code review request...".to_string()
            } else {
                format!("Preparing code review for {trimmed_hint}")
            }
        });

        self.insert_background_event_early(preparation_notice);
        self.request_redraw();

        let review_request = ReviewRequest {
            prompt,
            user_facing_hint: hint,
            metadata,
        };

        self.submit_op(Op::Review { review_request });
    }

    /// Check if a review flow is currently active.
    /// Used by event handling to determine special review-mode behavior.
    pub(crate) fn is_review_flow_active(&self) -> bool {
        self.active_review_hint.is_some() || self.active_review_prompt.is_some()
    }

    /// Build a summary cell from review output for display in history.
    pub(crate) fn build_review_summary_cell(
        &self,
        hint: Option<&str>,
        prompt: Option<&str>,
        output: &ReviewOutputEvent,
    ) -> history_cell::AssistantMarkdownCell {
        let mut sections: Vec<String> = Vec::new();
        let title = match hint {
            Some(h) if !h.trim().is_empty() => {
                let trimmed = h.trim();
                format!("**Review summary — {trimmed}**")
            }
            _ => "**Review summary**".to_string(),
        };
        sections.push(title);

        if let Some(p) = prompt {
            let trimmed_prompt = p.trim();
            if !trimmed_prompt.is_empty() {
                sections.push(format!("**Prompt:** {trimmed_prompt}"));
            }
        }

        let explanation = output.overall_explanation.trim();
        if !explanation.is_empty() {
            sections.push(explanation.to_string());
        }
        if !output.findings.is_empty() {
            sections.push(
                format_review_findings_block(&output.findings, None)
                    .trim()
                    .to_string(),
            );
        }
        let correctness = output.overall_correctness.trim();
        if !correctness.is_empty() {
            sections.push(format!("**Overall correctness:** {correctness}"));
        }
        if output.overall_confidence_score > 0.0 {
            let score = output.overall_confidence_score;
            sections.push(format!("**Confidence score:** {score:.1}"));
        }
        if sections.len() == 1 {
            sections.push("No detailed findings were provided.".to_string());
        }

        let markdown = sections
            .into_iter()
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        history_cell::AssistantMarkdownCell::new(markdown, &self.config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test that review context metadata defaults properly
    #[test]
    fn test_review_context_metadata_defaults() {
        let metadata = ReviewContextMetadata::default();
        assert!(metadata.scope.is_none());
        assert!(metadata.commit.is_none());
        assert!(metadata.base_branch.is_none());
        assert!(metadata.current_branch.is_none());
    }

    // Test that review request can be constructed
    #[test]
    fn test_review_request_construction() {
        let request = ReviewRequest {
            prompt: "Test prompt".to_string(),
            user_facing_hint: "test hint".to_string(),
            metadata: Some(ReviewContextMetadata {
                scope: Some("workspace".to_string()),
                ..Default::default()
            }),
        };
        assert_eq!(request.prompt, "Test prompt");
        assert_eq!(request.user_facing_hint, "test hint");
        assert!(request.metadata.is_some());
    }
}
