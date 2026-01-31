//! Shared agent config resolution for TUI and headless parity (SPEC-KIT-981)
//!
//! Provides a unified mechanism to resolve agent names to config names,
//! ensuring both TUI and headless paths use the same resolution logic.

use codex_core::config_types::AgentConfig;

/// Resolve a user-facing agent name (from prompts.json) to a configured agent name.
///
/// Resolution order:
/// 1) Exact match on `AgentConfig.name`
/// 2) Match on `AgentConfig.canonical_name` (preferred for portability)
/// 3) Error with available agents list
///
/// SPEC-KIT-981: Unified resolution for TUI/headless parity.
pub fn resolve_agent_config_name(
    agent_name: &str,
    agent_configs: &[AgentConfig],
) -> Result<String, String> {
    // 1) Exact match on name
    if agent_configs.iter().any(|cfg| cfg.name == agent_name) {
        return Ok(agent_name.to_string());
    }

    // 2) Match on canonical_name
    if let Some(cfg) = agent_configs
        .iter()
        .find(|cfg| cfg.canonical_name.as_deref() == Some(agent_name))
    {
        return Ok(cfg.name.clone());
    }

    // 3) Error with available agents
    let available: Vec<&str> = agent_configs
        .iter()
        .filter(|cfg| cfg.enabled)
        .map(|cfg| cfg.canonical_name.as_deref().unwrap_or(&cfg.name))
        .collect();

    Err(format!(
        "No config found for agent '{}'. Available enabled agents: {:?}",
        agent_name, available
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_agent_config(name: &str, canonical: Option<&str>, enabled: bool) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            canonical_name: canonical.map(|s| s.to_string()),
            command: name.to_string(),
            args: vec![],
            read_only: false,
            enabled,
            description: None,
            env: None,
            args_read_only: None,
            args_write: None,
            instructions: None,
            model: None,
        }
    }

    #[test]
    fn test_exact_name_match() {
        let configs = vec![
            make_agent_config("claude-haiku", Some("claude"), true),
            make_agent_config("gemini-flash", Some("gemini"), true),
        ];

        let result = resolve_agent_config_name("claude-haiku", &configs);
        assert_eq!(result.unwrap(), "claude-haiku");
    }

    #[test]
    fn test_canonical_name_match() {
        let configs = vec![
            make_agent_config("claude-haiku", Some("claude"), true),
            make_agent_config("gemini-flash", Some("gemini"), true),
        ];

        // "claude" should resolve to "claude-haiku" via canonical_name
        let result = resolve_agent_config_name("claude", &configs);
        assert_eq!(result.unwrap(), "claude-haiku");
    }

    #[test]
    fn test_gpt_canonical_names() {
        let configs = vec![
            make_agent_config("gpt-5.2-architect", Some("gpt_pro"), true),
            make_agent_config("gpt-5.2-codex", Some("gpt_codex"), true),
        ];

        let result = resolve_agent_config_name("gpt_pro", &configs);
        assert_eq!(result.unwrap(), "gpt-5.2-architect");

        let result = resolve_agent_config_name("gpt_codex", &configs);
        assert_eq!(result.unwrap(), "gpt-5.2-codex");
    }

    #[test]
    fn test_agent_not_found() {
        let configs = vec![make_agent_config("claude-haiku", Some("claude"), true)];

        let result = resolve_agent_config_name("nonexistent", &configs);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("No config found for agent 'nonexistent'"));
        assert!(err.contains("claude"));
    }

    #[test]
    fn test_disabled_agent_not_listed() {
        let configs = vec![
            make_agent_config("claude-haiku", Some("claude"), true),
            make_agent_config("disabled-agent", Some("disabled"), false),
        ];

        let result = resolve_agent_config_name("nonexistent", &configs);
        let err = result.unwrap_err();
        // disabled agent should not be in the available list
        assert!(!err.contains("disabled"));
        assert!(err.contains("claude"));
    }
}
