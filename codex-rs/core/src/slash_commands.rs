use crate::config_types::AgentConfig;
use crate::config_types::SubagentCommandConfig;

// NOTE: SPEC-KIT-963 removed the built-in prompt-expanding commands (/plan, /solve, /code).
// This fork (theturtlecsz/code) uses /speckit.* namespace exclusively.
// The format_subagent_command() function is retained for custom [[subagents.commands]] config.
// If you add custom subagent commands, document them in your project's CLAUDE.md.

/// Get the list of enabled agent names from the configuration
pub fn get_enabled_agents(agents: &[AgentConfig]) -> Vec<String> {
    agents
        .iter()
        .filter(|agent| agent.enabled)
        .map(|agent| agent.get_agent_name().to_string())
        .collect()
}

/// Get default models if no agents are configured
fn get_default_models() -> Vec<String> {
    vec![
        "claude".to_string(),
        "gemini".to_string(),
        "qwen".to_string(),
        "code".to_string(),
    ]
}

/// Resolution result for a subagent command.
#[derive(Debug, Clone, PartialEq)]
pub struct SubagentResolution {
    pub name: String,
    pub read_only: bool,
    pub models: Vec<String>,
    pub orchestrator_instructions: Option<String>,
    pub agent_instructions: Option<String>,
    pub prompt: String,
}

/// Default read_only for custom subagent commands.
/// SPEC-KIT-963: Built-in plan/solve/code removed. Custom commands default to read-only.
pub fn default_read_only_for(_name: &str) -> bool {
    true // Custom subagent commands default to read-only for safety
}

fn resolve_models(explicit: &[String], agents: Option<&[AgentConfig]>) -> Vec<String> {
    if !explicit.is_empty() {
        return explicit.to_vec();
    }
    if let Some(agents) = agents {
        let enabled = get_enabled_agents(agents);
        if !enabled.is_empty() {
            return enabled;
        }
    }
    get_default_models()
}

/// Format a subagent command (custom) using optional overrides from `[[subagents.commands]]`.
/// SPEC-KIT-963: Built-in plan/solve/code removed. Custom commands provide their own instructions.
pub fn default_instructions_for(_name: &str) -> Option<String> {
    None // Built-in subagent commands removed; custom commands use config
}

pub fn format_subagent_command(
    name: &str,
    task: &str,
    agents: Option<&[AgentConfig]>,
    commands: Option<&[SubagentCommandConfig]>,
) -> SubagentResolution {
    let (user_cmd, read_only_default) = {
        let ro = default_read_only_for(name);
        let found = commands
            .and_then(|list| list.iter().find(|c| c.name.eq_ignore_ascii_case(name)))
            .cloned();
        (found, ro)
    };

    let (read_only, models, orch_extra, agent_extra) = match user_cmd {
        Some(cfg) => (
            cfg.read_only, // User-provided read_only (defaults to false unless set)
            resolve_models(&cfg.agents, agents),
            cfg.orchestrator_instructions,
            cfg.agent_instructions,
        ),
        None => (read_only_default, resolve_models(&[], agents), None, None),
    };

    // Compose unified prompt used for all subagent commands (built-ins and custom)
    let models_str = models
        .iter()
        .map(|m| format!("\"{m}\""))
        .collect::<Vec<_>>()
        .join(", ");

    let instr_text = orch_extra
        .clone()
        .or_else(|| default_instructions_for(name))
        .unwrap_or_default();

    let prompt = format!(
        "Please perform /{name} using the <tools>, <instructions> and <task> below.\n<tools>\n    To perform /{name} you must use `agent_run` to start a batch of agents with:\n    - `models`: an array containing [{models_str}]\n    - `read_only`: {read_only}\n    Provide a comprehensive description of the task and context. You may need to briefly research the code base first and to give the agents a head start of where to look. You can include one or two key files but also allow the models to look up the files they need themselves. Using `agent_run` will start all agents at once and return a `batch_id`.\n\n    Each agent uses a different LLM which allows you to gather diverse results.\n    Monitor progress using `agent_wait` with `batch_id` and `return_all: true` to wait for all agents to complete.\n    If an agent fails or times out, you can ignore it and continue with the other results. \n    Use `agent_result` to get the results, or inspect the worktree directly if `read_only` is false.\n</tools>\n<instructions>\n    Instructions for /{name}:\n    {instr_text}\n</instructions>\n<task>\n    Task for /{name}:\n    {task}\n</task>",
    );

    SubagentResolution {
        name: name.to_string(),
        read_only,
        models,
        orchestrator_instructions: orch_extra,
        agent_instructions: agent_extra,
        prompt,
    }
}

// SPEC-KIT-963: Legacy format_plan_command, format_solve_command, format_code_command removed.
// SPEC-KIT-963: handle_slash_command removed (only handled /plan, /solve, /code).
// This fork uses /speckit.* namespace via the command registry instead.
// format_subagent_command() retained for custom [[subagents.commands]] in config.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_subagent_command_basic() {
        // Test that custom subagent commands can be formatted
        let result = format_subagent_command("custom-task", "do something", None, None);
        assert_eq!(result.name, "custom-task");
        assert!(result.prompt.contains("do something"));
        assert!(result.read_only); // defaults to true for safety
    }

    #[test]
    fn test_default_read_only_for_custom() {
        // SPEC-KIT-963: All commands default to read-only
        assert!(default_read_only_for("anything"));
        assert!(default_read_only_for("custom"));
        assert!(default_read_only_for("plan")); // Even old names default to read-only now
    }

    #[test]
    fn test_default_instructions_for_returns_none() {
        // SPEC-KIT-963: No built-in instructions
        assert!(default_instructions_for("plan").is_none());
        assert!(default_instructions_for("solve").is_none());
        assert!(default_instructions_for("code").is_none());
        assert!(default_instructions_for("custom").is_none());
    }
}
