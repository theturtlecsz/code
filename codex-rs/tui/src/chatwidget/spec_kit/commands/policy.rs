//! SPEC-KIT-977: Policy CLI/TUI commands
//!
//! Commands for managing policy snapshots:
//! - `/speckit.policy list` - List all policy snapshots
//! - `/speckit.policy show <id>` - Show policy details
//! - `/speckit.policy current` - Show current active policy
//!
//! ## Decision IDs
//! - D12: PolicySnapshot is the compiled artifact
//! - D17: Dual storage (filesystem + capsule)
//! - D44: Events tagged with policy_id

use super::super::super::ChatWidget;
use super::super::command_registry::SpecKitCommand;
use crate::history_cell::{HistoryCellType, PlainHistoryCell};
use codex_stage0::{ChangeCategory, PolicyDiff, PolicyStore};
use ratatui::text::Line;

// =============================================================================
// speckit.policy
// =============================================================================

/// Command: /speckit.policy [list|show|current]
/// Manage policy snapshots.
pub struct SpecKitPolicyCommand;

impl SpecKitCommand for SpecKitPolicyCommand {
    fn name(&self) -> &'static str {
        "speckit.policy"
    }

    fn aliases(&self) -> &[&'static str] {
        &["policy"]
    }

    fn description(&self) -> &'static str {
        "manage policy snapshots (list/show/current)"
    }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        let parts: Vec<&str> = args.split_whitespace().collect();
        let subcommand = parts.first().copied().unwrap_or("help");

        match subcommand {
            "list" => execute_list(widget),
            "show" => {
                if parts.len() < 2 {
                    widget.history_push(PlainHistoryCell::new(
                        vec![Line::from("Usage: /speckit.policy show <policy_id>")],
                        HistoryCellType::Error,
                    ));
                } else {
                    execute_show(widget, parts[1]);
                }
            }
            "current" => execute_current(widget),
            "diff" => {
                if parts.len() < 3 {
                    widget.history_push(PlainHistoryCell::new(
                        vec![Line::from(
                            "Usage: /speckit.policy diff <policy_id_a> <policy_id_b>",
                        )],
                        HistoryCellType::Error,
                    ));
                } else {
                    execute_diff(widget, parts[1], parts[2]);
                }
            }
            _ => {
                let lines = vec![
                    Line::from("📋 Policy Commands (SPEC-KIT-977)"),
                    Line::from(""),
                    Line::from("/speckit.policy list           # List all policy snapshots"),
                    Line::from("/speckit.policy show <id>      # Show policy details"),
                    Line::from("/speckit.policy current        # Show current active policy"),
                    Line::from("/speckit.policy diff <a> <b>   # Compare two policies"),
                    Line::from(""),
                    Line::from("Headless CLI:"),
                    Line::from("  code speckit policy list [--json]"),
                    Line::from("  code speckit policy show <id> [--json]"),
                    Line::from("  code speckit policy current [--json]"),
                    Line::from("  code speckit policy diff <a> <b> [--json]"),
                    Line::from("  code speckit policy validate [--path <path>]"),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
            }
        }
    }

    fn requires_args(&self) -> bool {
        false
    }

    fn is_prompt_expanding(&self) -> bool {
        false
    }
}

// =============================================================================
// Command implementations
// =============================================================================

fn execute_list(widget: &mut ChatWidget) {
    let store = PolicyStore::new();

    match store.list() {
        Ok(policies) => {
            if policies.is_empty() {
                let lines = vec![
                    Line::from("📋 Policy Snapshots"),
                    Line::from(""),
                    Line::from("   No policy snapshots found."),
                    Line::from(""),
                    Line::from("   Policy snapshots are created when:"),
                    Line::from("     - A /speckit.auto run starts"),
                    Line::from("     - Policy drift is detected at stage boundaries"),
                    Line::from(""),
                    Line::from("   Location: .speckit/policies/"),
                ];
                widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
                return;
            }

            let mut lines = vec![
                Line::from("📋 Policy Snapshots"),
                Line::from(""),
                Line::from(
                    "   Created              | Policy ID                              | Hash",
                ),
                Line::from(
                    "   ---------------------|----------------------------------------|----------",
                ),
            ];

            // Show most recent first, limit to 15
            for info in policies.iter().rev().take(15) {
                let date = info.created_at.format("%Y-%m-%d %H:%M").to_string();
                let hash_display = if info.hash_short.len() > 8 {
                    &info.hash_short[..8]
                } else {
                    &info.hash_short
                };

                lines.push(Line::from(format!(
                    "   {:19} | {:38} | {}",
                    date, info.policy_id, hash_display
                )));
            }

            if policies.len() > 15 {
                lines.push(Line::from(""));
                lines.push(Line::from(format!(
                    "   Showing 15 of {} policies. Use CLI for full list.",
                    policies.len()
                )));
            }

            lines.push(Line::from(""));
            lines.push(Line::from("   Use `/speckit.policy show <id>` for details"));

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!("❌ Failed to list policies: {}", e))],
                HistoryCellType::Error,
            ));
        }
    }
}

fn execute_show(widget: &mut ChatWidget, policy_id: &str) {
    let store = PolicyStore::new();

    match store.load(policy_id) {
        Ok(snapshot) => {
            let mut lines = vec![
                Line::from(format!("📋 Policy: {}", snapshot.policy_id)),
                Line::from(""),
                Line::from(format!(
                    "   Created:        {}",
                    snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                )),
                Line::from(format!("   Hash:           {}", snapshot.hash)),
                Line::from(format!("   Schema Version: {}", snapshot.schema_version)),
                Line::from(""),
                Line::from("Model Config:"),
                Line::from(format!(
                    "   max_tokens:       {}",
                    snapshot.model_config.max_tokens
                )),
                Line::from(format!(
                    "   top_k:            {}",
                    snapshot.model_config.top_k
                )),
                Line::from(format!(
                    "   hybrid_enabled:   {}",
                    snapshot.model_config.hybrid_enabled
                )),
                Line::from(format!(
                    "   tier2_enabled:    {}",
                    snapshot.model_config.tier2_enabled
                )),
                Line::from(""),
                Line::from("Scoring Weights:"),
                Line::from(format!(
                    "   usage:            {:.2}",
                    snapshot.weights.usage
                )),
                Line::from(format!(
                    "   recency:          {:.2}",
                    snapshot.weights.recency
                )),
                Line::from(format!(
                    "   priority:         {:.2}",
                    snapshot.weights.priority
                )),
                Line::from(format!(
                    "   decay:            {:.2}",
                    snapshot.weights.decay
                )),
            ];

            if let Some(gov) = &snapshot.governance {
                lines.push(Line::from(""));
                lines.push(Line::from("Governance (from model_policy.toml):"));
                lines.push(Line::from(format!(
                    "   SOR primary:      {}",
                    gov.system_of_record.primary
                )));
                lines.push(Line::from(format!(
                    "   Capture mode:     {}",
                    gov.capture.mode
                )));
                lines.push(Line::from(format!(
                    "   Reflex enabled:   {}",
                    gov.routing.reflex.enabled
                )));
            }

            let verified = snapshot.verify_hash();
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Hash verified:     {}",
                if verified { "✓" } else { "✗" }
            )));

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Policy '{}' not found: {}",
                    policy_id, e
                ))],
                HistoryCellType::Error,
            ));
        }
    }
}

fn execute_current(widget: &mut ChatWidget) {
    let store = PolicyStore::new();

    match store.latest() {
        Ok(Some(snapshot)) => {
            let mut lines = vec![
                Line::from(format!("📋 Current Policy: {}", snapshot.policy_id)),
                Line::from(""),
                Line::from(format!(
                    "   Created: {}",
                    snapshot.created_at.format("%Y-%m-%d %H:%M:%S UTC")
                )),
                Line::from(format!("   Hash:    {}", snapshot.hash)),
            ];

            if let Some(gov) = &snapshot.governance {
                lines.push(Line::from(""));
                lines.push(Line::from("Governance Summary:"));
                lines.push(Line::from(format!(
                    "   SOR primary:      {}",
                    gov.system_of_record.primary
                )));
                lines.push(Line::from(format!(
                    "   Capture mode:     {}",
                    gov.capture.mode
                )));
                lines.push(Line::from(format!(
                    "   Reflex enabled:   {}",
                    gov.routing.reflex.enabled
                )));
                if gov.routing.reflex.enabled {
                    lines.push(Line::from(format!(
                        "   Reflex endpoint:  {}",
                        gov.routing.reflex.endpoint
                    )));
                }
            }

            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Use `/speckit.policy show {}` for full details",
                snapshot.policy_id
            )));

            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Ok(None) => {
            let lines = vec![
                Line::from("📋 Current Policy"),
                Line::from(""),
                Line::from("   No policy snapshots found."),
                Line::from(""),
                Line::from("   Run a /speckit.auto pipeline to create a policy snapshot."),
            ];
            widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
        }
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Failed to load current policy: {}",
                    e
                ))],
                HistoryCellType::Error,
            ));
        }
    }
}

fn execute_diff(widget: &mut ChatWidget, policy_id_a: &str, policy_id_b: &str) {
    let store = PolicyStore::new();

    // Load both snapshots
    let snapshot_a = match store.load(policy_id_a) {
        Ok(s) => s,
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Policy '{}' not found: {}",
                    policy_id_a, e
                ))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    let snapshot_b = match store.load(policy_id_b) {
        Ok(s) => s,
        Err(e) => {
            widget.history_push(PlainHistoryCell::new(
                vec![Line::from(format!(
                    "❌ Policy '{}' not found: {}",
                    policy_id_b, e
                ))],
                HistoryCellType::Error,
            ));
            return;
        }
    };

    // Compute diff
    let diff = PolicyDiff::compute(&snapshot_a, &snapshot_b);

    let mut lines = vec![
        Line::from(format!(
            "📋 Policy Diff: {} → {}",
            &diff.policy_id_a[..8.min(diff.policy_id_a.len())],
            &diff.policy_id_b[..8.min(diff.policy_id_b.len())]
        )),
        Line::from(""),
        Line::from(format!(
            "   Hash A: {}",
            &diff.hash_a[..16.min(diff.hash_a.len())]
        )),
        Line::from(format!(
            "   Hash B: {}",
            &diff.hash_b[..16.min(diff.hash_b.len())]
        )),
        Line::from(""),
    ];

    if diff.identical {
        lines.push(Line::from("   ✓ Policies are identical"));
    } else {
        lines.push(Line::from(format!(
            "   {} change(s) detected:",
            diff.changes.len()
        )));
        lines.push(Line::from(""));

        // Group by category and display
        let grouped = diff.changes_by_category();

        // Fixed category order for determinism
        let categories = [
            ChangeCategory::Governance,
            ChangeCategory::ModelConfig,
            ChangeCategory::Weights,
            ChangeCategory::SourceFiles,
            ChangeCategory::Prompts,
            ChangeCategory::Schema,
        ];

        for category in categories {
            if let Some(changes) = grouped.get(&category) {
                lines.push(Line::from(format!("   [{}]", category.as_str())));
                for change in changes {
                    // Truncate long values for display
                    let old_val = if change.old_value.len() > 20 {
                        format!("{}...", &change.old_value[..20])
                    } else {
                        change.old_value.clone()
                    };
                    let new_val = if change.new_value.len() > 20 {
                        format!("{}...", &change.new_value[..20])
                    } else {
                        change.new_value.clone()
                    };
                    lines.push(Line::from(format!(
                        "     {} : {} → {}",
                        change.path, old_val, new_val
                    )));
                }
                lines.push(Line::from(""));
            }
        }

        lines.push(Line::from("Changed keys:"));
        for key in diff.changed_keys() {
            lines.push(Line::from(format!("     - {}", key)));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(
        "Use CLI `code speckit policy diff <a> <b> --json` for machine-parseable output",
    ));

    widget.history_push(PlainHistoryCell::new(lines, HistoryCellType::Notice));
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_command_name() {
        let cmd = SpecKitPolicyCommand;
        assert_eq!(cmd.name(), "speckit.policy");
        assert!(!cmd.requires_args());
        assert!(!cmd.is_prompt_expanding());
    }

    #[test]
    fn test_policy_command_aliases() {
        let cmd = SpecKitPolicyCommand;
        assert!(cmd.aliases().contains(&"policy"));
    }
}
