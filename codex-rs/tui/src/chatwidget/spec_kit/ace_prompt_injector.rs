//! ACE prompt injection logic for spec-kit commands
//!
//! Fetches playbook heuristics from ACE and injects them into prompts
//! before submission to the orchestrator.

use super::ace_client::{self, AceResult, PlaybookBullet};
use codex_core::config_types::{AceConfig, AceMode};
use std::collections::HashSet;
use tracing::{debug, warn};

/// Check if ACE should be used for this command
pub fn should_use_ace(config: &AceConfig, command_name: &str) -> bool {
    if !config.enabled {
        return false;
    }

    match config.mode {
        AceMode::Always => true,
        AceMode::Never => false,
        AceMode::Auto => {
            // Check if command is in use_for list
            config.use_for.iter().any(|pattern| {
                // Support both exact matches and prefix patterns
                command_name == pattern || command_name.starts_with(&format!("{}.", pattern))
            })
        }
    }
}

/// Map command name to ACE scope
pub fn command_to_scope(command_name: &str) -> Option<&str> {
    // Remove "speckit." prefix if present
    let name = command_name.strip_prefix("speckit.").unwrap_or(command_name);

    match name {
        "constitution" => Some("global"),
        "specify" => Some("specify"),
        "tasks" => Some("tasks"),
        "implement" => Some("implement"),
        "test" | "validate" => Some("test"),
        _ => None,
    }
}

/// Normalize bullet text for deduplication
fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' }) // Convert all non-alphanumeric to spaces
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Deduplicate bullets by normalized text
fn dedupe_bullets(bullets: Vec<PlaybookBullet>) -> Vec<PlaybookBullet> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();

    for bullet in bullets {
        let normalized = normalize_text(&bullet.text);
        if seen.insert(normalized) {
            result.push(bullet);
        }
    }

    result
}

/// Select and cap bullets according to rules
pub fn select_bullets(mut bullets: Vec<PlaybookBullet>, slice_size: usize) -> Vec<PlaybookBullet> {
    // Deduplicate first
    bullets = dedupe_bullets(bullets);

    // Separate by type
    let helpful: Vec<_> = bullets.iter().filter(|b| b.helpful && !b.harmful).cloned().collect();
    let mut harmful: Vec<_> = bullets.iter().filter(|b| b.harmful).cloned().collect();
    let mut neutral: Vec<_> = bullets
        .iter()
        .filter(|b| !b.helpful && !b.harmful)
        .cloned()
        .collect();

    // Cap harmful and neutral
    harmful.truncate(2);
    neutral.truncate(2);

    // Build result, prioritizing helpful bullets
    let mut result = Vec::new();

    // Add helpful bullets first (these are most valuable)
    let helpful_count = slice_size.saturating_sub(harmful.len()).saturating_sub(neutral.len());
    result.extend(helpful.into_iter().take(helpful_count));

    // Add harmful bullets (warnings/anti-patterns)
    result.extend(harmful);

    // Fill remaining space with neutral if we have room
    let remaining = slice_size.saturating_sub(result.len());
    result.extend(neutral.into_iter().take(remaining));

    result
}

/// Format bullets into a text section
///
/// Returns (section_text, bullet_ids) tuple for tracking which bullets were used
pub fn format_ace_section(bullets: &[PlaybookBullet]) -> (String, Vec<i32>) {
    if bullets.is_empty() {
        return (String::new(), Vec::new());
    }

    let mut lines = vec!["### Project heuristics learned (ACE)".to_string()];
    let mut bullet_ids = Vec::new();

    for bullet in bullets {
        let marker = if bullet.harmful {
            "[avoid]"
        } else if bullet.helpful {
            "[helpful]"
        } else {
            "[note]"
        };

        lines.push(format!("- {} {}", marker, bullet.text));

        // Track bullet ID if available
        if let Some(id) = bullet.id {
            bullet_ids.push(id);
        }
    }

    lines.push(String::new()); // Blank line after section
    (lines.join("\n"), bullet_ids)
}

/// Inject ACE playbook slice into prompt (synchronous wrapper)
///
/// This function bridges the sync/async boundary by using tokio::runtime::Handle.
/// It's safe to call from synchronous contexts that are already on a tokio runtime.
pub fn inject_ace_section(
    config: &AceConfig,
    command_name: &str,
    repo_root: Option<String>,
    branch: Option<String>,
    mut prompt: String,
) -> String {
    // Check if we should use ACE
    if !should_use_ace(config, command_name) {
        debug!("ACE injection skipped: not enabled for {}", command_name);
        return prompt;
    }

    // Get scope for this command
    let Some(scope) = command_to_scope(command_name) else {
        debug!("ACE injection skipped: no scope mapping for {}", command_name);
        return prompt;
    };

    // Get repo_root and branch (with fallbacks)
    let repo_root = repo_root.unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .and_then(|p| p.to_str().map(|s| s.to_string()))
            .unwrap_or_else(|| ".".to_string())
    });

    let branch = branch.unwrap_or_else(|| "main".to_string());

    // Note: Cannot use block_on when already on tokio runtime (TUI context)
    // For now, skip ACE injection in sync contexts
    // TODO: Make prompt assembly async or use channels
    warn!("ACE injection skipped: cannot block_on from within tokio runtime");
    warn!("This is a known limitation - ACE injection needs async prompt assembly");
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_core::config_types::AceMode;

    fn test_config(mode: AceMode, use_for: Vec<String>) -> AceConfig {
        AceConfig {
            enabled: true,
            mode,
            slice_size: 8,
            db_path: "test.db".to_string(),
            use_for,
            complex_task_files_threshold: 4,
            rerun_window_minutes: 30,
        }
    }

    /// Integration test: ACE disabled behaves like baseline
    #[test]
    fn test_integration_ace_disabled_equals_baseline() {
        let disabled_config = AceConfig {
            enabled: false,
            ..Default::default()
        };

        let never_config = AceConfig {
            enabled: true,
            mode: AceMode::Never,
            ..Default::default()
        };

        // Both should behave identically (no ACE usage)
        for command in &["speckit.specify", "speckit.tasks", "speckit.implement"] {
            assert!(!should_use_ace(&disabled_config, command));
            assert!(!should_use_ace(&never_config, command));
        }
    }

    /// Integration test: Default config has sensible values
    #[test]
    fn test_integration_default_config() {
        let config = AceConfig::default();

        assert!(config.enabled);
        assert_eq!(config.mode, AceMode::Auto);
        assert_eq!(config.slice_size, 8);
        assert!(config.use_for.contains(&"speckit.specify".to_string()));
        assert!(config.use_for.contains(&"speckit.implement".to_string()));
    }

    #[test]
    fn test_should_use_ace_always() {
        let config = test_config(AceMode::Always, vec![]);
        assert!(should_use_ace(&config, "anything"));
        assert!(should_use_ace(&config, "speckit.specify"));
    }

    #[test]
    fn test_should_use_ace_never() {
        let config = test_config(AceMode::Never, vec!["speckit.specify".to_string()]);
        assert!(!should_use_ace(&config, "speckit.specify"));
    }

    #[test]
    fn test_should_use_ace_auto() {
        let config = test_config(
            AceMode::Auto,
            vec!["speckit.specify".to_string(), "speckit.tasks".to_string()],
        );
        assert!(should_use_ace(&config, "speckit.specify"));
        assert!(should_use_ace(&config, "speckit.tasks"));
        assert!(!should_use_ace(&config, "speckit.plan"));
    }

    #[test]
    fn test_should_use_ace_disabled() {
        let mut config = test_config(AceMode::Always, vec![]);
        config.enabled = false;
        assert!(!should_use_ace(&config, "anything"));
    }

    #[test]
    fn test_command_to_scope() {
        assert_eq!(command_to_scope("speckit.specify"), Some("specify"));
        assert_eq!(command_to_scope("specify"), Some("specify"));
        assert_eq!(command_to_scope("speckit.tasks"), Some("tasks"));
        assert_eq!(command_to_scope("speckit.implement"), Some("implement"));
        assert_eq!(command_to_scope("speckit.test"), Some("test"));
        assert_eq!(command_to_scope("speckit.validate"), Some("test"));
        assert_eq!(command_to_scope("speckit.constitution"), Some("global"));
        assert_eq!(command_to_scope("speckit.unknown"), None);
    }

    #[test]
    fn test_normalize_text() {
        assert_eq!(
            normalize_text("Use X pattern"),
            "use x pattern"
        );
        assert_eq!(
            normalize_text("  USE   X   PATTERN  "),
            "use x pattern"
        );
        assert_eq!(
            normalize_text("Use-X-Pattern!!!"),
            "use x pattern"
        );
    }

    #[test]
    fn test_dedupe_bullets() {
        let bullets = vec![
            PlaybookBullet {
                id: Some(1),
                text: "Use X pattern".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.9,
                source: None,
            },
            PlaybookBullet {
                id: Some(2),
                text: "USE X PATTERN".to_string(), // Duplicate
                helpful: true,
                harmful: false,
                confidence: 0.8,
                source: None,
            },
            PlaybookBullet {
                id: Some(3),
                text: "Use Y pattern".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.7,
                source: None,
            },
        ];

        let result = dedupe_bullets(bullets);
        assert_eq!(result.len(), 2);
        assert!(result[0].text.contains("Use X pattern"));
        assert!(result[1].text.contains("Use Y pattern"));
    }

    #[test]
    fn test_select_bullets_cap() {
        let bullets = vec![
            PlaybookBullet {
                id: None,
                text: "Helpful 1".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.9,
                source: None,
            },
            PlaybookBullet {
                id: None,
                text: "Helpful 2".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.85,
                source: None,
            },
            PlaybookBullet {
                id: None,
                text: "Harmful 1".to_string(),
                helpful: false,
                harmful: true,
                confidence: 0.8,
                source: None,
            },
            PlaybookBullet {
                id: None,
                text: "Harmful 2".to_string(),
                helpful: false,
                harmful: true,
                confidence: 0.75,
                source: None,
            },
            PlaybookBullet {
                id: None,
                text: "Harmful 3".to_string(), // Should be capped
                helpful: false,
                harmful: true,
                confidence: 0.7,
                source: None,
            },
        ];

        let result = select_bullets(bullets, 8);

        // Should have at most 2 harmful
        let harmful_count = result.iter().filter(|b| b.harmful).count();
        assert!(harmful_count <= 2);

        // Total should not exceed slice_size
        assert!(result.len() <= 8);
    }

    #[test]
    fn test_format_ace_section() {
        let bullets = vec![
            PlaybookBullet {
                id: Some(1),
                text: "Use X pattern".to_string(),
                helpful: true,
                harmful: false,
                confidence: 0.9,
                source: None,
            },
            PlaybookBullet {
                id: Some(2),
                text: "Avoid Y anti-pattern".to_string(),
                helpful: false,
                harmful: true,
                confidence: 0.8,
                source: None,
            },
        ];

        let (section, ids) = format_ace_section(&bullets);

        assert!(section.contains("### Project heuristics learned (ACE)"));
        assert!(section.contains("[helpful] Use X pattern"));
        assert!(section.contains("[avoid] Avoid Y anti-pattern"));
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn test_format_ace_section_empty() {
        let bullets = vec![];
        let (section, ids) = format_ace_section(&bullets);
        assert_eq!(section, "");
        assert!(ids.is_empty());
    }
}
