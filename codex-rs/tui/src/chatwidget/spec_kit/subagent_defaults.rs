//! Built-in subagent presets for Spec-Kit commands.
//!
//! These defaults keep Spec-Kit slash commands aligned with the tiered
//! routing and prompt policies in docs/spec-kit/model-strategy.md so that each
//! command launches the intended agent mix with the correct scope.

use codex_core::config_types::SubagentCommandConfig;

fn make_config(
    name: &str,
    read_only: bool,
    agents: &[&str],
    orchestrator: &str,
    agent: &str,
) -> SubagentCommandConfig {
    SubagentCommandConfig {
        name: name.to_string(),
        read_only,
        agents: agents.iter().map(|a| a.to_string()).collect(),
        orchestrator_instructions: Some(orchestrator.to_string()),
        agent_instructions: Some(agent.to_string()),
    }
}

/// Return the default subagent configuration for a Spec-Kit command.
///
/// The caller should only use this when the active configuration is missing an
/// explicit entry for the command. These defaults preserve the documented
/// multi-agent line-up and memory tagging discipline.
pub fn default_for(name: &str) -> Option<SubagentCommandConfig> {
    match name {
        "speckit.new" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "code"],
            "Derive the spec_id from the command argument (e.g., SPEC-KIT-###). Provide the SPEC packet and current PRD excerpts only. Launch gemini, claude, and code in read-only mode and require each agent to persist its JSON artifact to local-memory with tags `spec:<spec_id>` and `stage:new`. Capture at most five actionable clarifications before returning.",
            "Produce the stage JSON described in docs/spec-kit/prompts.json for `speckit.new`. Include spec_id, prompt_version, model metadata, and persist the result via `local-memory store_memory` using tags `spec:<spec_id>`, `stage:new`, `consensus-artifact`. Do not modify files or plan tasks.",
        )),
        "speckit.specify" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "code"],
            "Use tier-2 routing for specification elaboration. Limit context to the provided spec_id artefacts and any prior clarify outputs. Ensure each agent writes its JSON update back to local-memory with tags `spec:<spec_id>` and `stage:specify` (importance >= 8).",
            "Follow the `speckit.specify` JSON schema in docs/spec-kit/prompts.json. Reference prior clarify artifacts when needed and store the final JSON via `local-memory store_memory` with tags `spec:<spec_id>`, `stage:specify`, `consensus-artifact`.",
        )),
        "speckit.clarify" => None, // Native execution - no orchestrator defaults needed
        "speckit.analyze" => None, // Native execution - no orchestrator defaults needed
        "speckit.checklist" => None, // Native execution - no orchestrator defaults needed
        "speckit.plan" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "gpt_pro"],
            "Run the plan stage using gemini, claude, and gpt_pro. Provide the SPEC packet and clarify/checklist outputs as context, then ensure each agent emits the spec-plan JSON and records it via local-memory with tags `spec:<spec_id>` and `stage:plan`. Capture consensus agreements/conflicts before replying.",
            "Follow the `spec-plan` JSON structure in docs/spec-kit/prompts.json (work_breakdown, acceptance_mapping, risks, consensus metadata). After emitting the JSON, store it with `local-memory store_memory` tagged `spec:<spec_id>`, `stage:plan`, `consensus-artifact`, importance >= 8.",
        )),
        "speckit.tasks" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "gpt_pro"],
            "Launch gemini, claude, and gpt_pro with read-only true to build the task list for the given spec_id. Feed in SPEC, plan outputs, and relevant evidence. Persist every agent artifact to local-memory tagged `spec:<spec_id>` and `stage:tasks` before synthesising consensus.",
            "Use the `spec-tasks` JSON schema in docs/spec-kit/prompts.json (tasks array, acceptance coverage, followups, consensus). Store the JSON via `local-memory store_memory` tags `spec:<spec_id>`, `stage:tasks`, `consensus-artifact`.",
        )),
        "speckit.implement" => Some(make_config(
            name,
            false,
            &["gemini", "claude", "gpt_codex", "gpt_pro"],
            "Execute the implementation stage with the quad-agent ensemble: gemini (research), claude (strategy), gpt_codex (diffs), gpt_pro (arbiter). Allow write access in isolated worktrees. Supply prior plan/tasks context and require each agent to push its JSON artifact to local-memory tagged `spec:<spec_id>` and `stage:implement`.",
            "Emit the `spec-implement` JSON payload (code_paths, operations, diff proposals, validation plan, consensus) defined in docs/spec-kit/prompts.json. Store results via `local-memory store_memory` with tags `spec:<spec_id>`, `stage:implement`, `consensus-artifact`, and document commanded shell steps to reproduce diffs.",
        )),
        "speckit.validate" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "gpt_pro"],
            "Run validation with gemini, claude, and gpt_pro. Provide telemetry logs, test outputs, and plan/tasks context. Ensure each agent stores its validation JSON to local-memory tagged `spec:<spec_id>` and `stage:validate` with importance >= 8.",
            "Follow the `spec-validate` JSON schema (scenarios, analysis, decision, consensus) in docs/spec-kit/prompts.json. Persist via `local-memory store_memory` using tags `spec:<spec_id>`, `stage:validate`, `consensus-artifact` and highlight failing evidence.",
        )),
        "speckit.audit" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "gpt_pro"],
            "Launch gemini, claude, and gpt_pro to complete the audit stage. Scope context to diffs, telemetry, and evidence directory footprint. Require local-memory persistence tagged `spec:<spec_id>` and `stage:audit` for each agent artifact before synthesising the decision.",
            "Produce the `spec-audit` JSON schema (diff_summary, telemetry, risks, checks, recommendation, consensus). Store via `local-memory store_memory` with tags `spec:<spec_id>`, `stage:audit`, `consensus-artifact`.",
        )),
        "speckit.unlock" => Some(make_config(
            name,
            true,
            &["gemini", "claude", "gpt_pro"],
            "Use the unlock stage routing with gemini, claude, and gpt_pro. Provide branch state, outstanding work, and guardrail results for the spec_id. Persist each unlock JSON artifact to local-memory tagged `spec:<spec_id>` and `stage:unlock` before summarising the final decision.",
            "Return the `spec-unlock` JSON schema (branch_state, safeguards, decision, consensus) per docs/spec-kit/prompts.json. Store it using `local-memory store_memory` with tags `spec:<spec_id>`, `stage:unlock`, `consensus-artifact` and note any conditions.",
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clarify_defaults_match_expected_agents() {
        let clarify = default_for("speckit.clarify").expect("clarify default");
        assert!(clarify.read_only);
        assert_eq!(clarify.agents, vec!["gemini", "claude", "code"]);
    }

    #[test]
    fn implement_defaults_are_write_enabled_and_quad_agent() {
        let implement = default_for("speckit.implement").expect("implement default");
        assert!(!implement.read_only);
        assert_eq!(
            implement.agents,
            vec!["gemini", "claude", "gpt_codex", "gpt_pro"]
        );
    }
}
