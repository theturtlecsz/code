//! ACE constitution pinning logic
//!
//! Extracts imperative bullets from constitution documents and pins them
//! to ACE playbook (global and phase-specific scopes).

use super::ace_client::{self, AceResult};
use codex_core::config_types::AceConfig;
use regex_lite::Regex;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Maximum bullet length (characters)
const MAX_BULLET_LENGTH: usize = 140;

/// Minimum bullet length (characters)
const MIN_BULLET_LENGTH: usize = 10;

/// Maximum bullets to extract
const MAX_BULLETS: usize = 12;

/// Minimum bullets to extract
const MIN_BULLETS: usize = 5;

/// Phase keywords for scope detection
const PHASE_KEYWORDS: &[(&str, &str)] = &[
    ("plan", "plan"),
    ("planning", "plan"),
    ("tasks", "tasks"),
    ("task", "tasks"),
    ("implement", "implement"),
    ("implementation", "implement"),
    ("coding", "implement"),
    ("test", "test"),
    ("testing", "test"),
    ("validation", "test"),
    ("validate", "test"),
    ("audit", "test"),
];

/// A constitution bullet with metadata
#[derive(Debug, Clone)]
pub struct ConstitutionBullet {
    pub text: String,
    pub scopes: Vec<String>,
    pub tags: Vec<String>,
}

impl ConstitutionBullet {
    pub fn new(text: String) -> Self {
        let (scopes, tags) = detect_scopes_and_tags(&text);
        Self { text, scopes, tags }
    }

    /// Check if this bullet applies to global scope
    pub fn is_global(&self) -> bool {
        self.scopes.is_empty() || self.scopes.contains(&"global".to_string())
    }
}

/// Detect phase scopes and tags from bullet text
fn detect_scopes_and_tags(text: &str) -> (Vec<String>, Vec<String>) {
    let lower = text.to_lowercase();
    let mut scopes = Vec::new();
    let mut tags = Vec::new();

    // Detect phase keywords
    for (keyword, scope) in PHASE_KEYWORDS {
        if lower.contains(keyword) {
            if !scopes.contains(&scope.to_string()) {
                scopes.push(scope.to_string());
            }
        }
    }

    // Detect common tags
    if lower.contains("template") {
        tags.push("templates".to_string());
    }
    if lower.contains("lint") || lower.contains("clippy") || lower.contains("format") {
        tags.push("lint".to_string());
    }
    if lower.contains("test") {
        tags.push("testing".to_string());
    }
    if lower.contains("evidence") || lower.contains("telemetry") {
        tags.push("evidence".to_string());
    }
    if lower.contains("doc") || lower.contains("documentation") {
        tags.push("docs".to_string());
    }

    // If no specific scope detected, it's global
    if scopes.is_empty() {
        scopes.push("global".to_string());
    }

    (scopes, tags)
}

/// Extract imperative bullets from markdown text
pub fn extract_bullets(markdown: &str) -> Vec<ConstitutionBullet> {
    let mut bullets = Vec::new();

    // Match bullet points (- or *)
    let bullet_re = Regex::new(r"^\s*[-*]\s+(.+)$").unwrap();

    for line in markdown.lines() {
        if let Some(captures) = bullet_re.captures(line) {
            let text = captures.get(1).unwrap().as_str().trim();

            // Skip if too short or too long
            if text.len() < MIN_BULLET_LENGTH || text.len() > MAX_BULLET_LENGTH {
                continue;
            }

            // Convert to imperative if needed
            let imperative = convert_to_imperative(text);

            bullets.push(ConstitutionBullet::new(imperative));

            // Stop if we have enough
            if bullets.len() >= MAX_BULLETS {
                break;
            }
        }
    }

    bullets
}

/// Convert bullet to imperative voice
fn convert_to_imperative(text: &str) -> String {
    let trimmed = text.trim();

    // Already imperative (starts with verb)
    let imperative_verbs = [
        "Keep", "Update", "Maintain", "Ensure", "Validate", "Check", "Use", "Avoid", "Never",
        "Always", "Document", "Test", "Record", "Surface", "Add", "Remove", "Extract", "Pin",
        "Call", "Run", "Execute", "Build", "Format", "Lint",
    ];

    for verb in &imperative_verbs {
        if trimmed.starts_with(verb) {
            return trimmed.to_string();
        }
    }

    // Convert common patterns
    if trimmed.contains(" must ") {
        // "X must Y" -> "Ensure X Y"
        return format!("Ensure {}", trimmed.replace(" must ", " "));
    }

    if trimmed.starts_with("All ") || trimmed.starts_with("Every ") {
        // "All X should Y" -> "Ensure all X Y"
        return format!("Ensure {}", trimmed.to_lowercase());
    }

    // Default: prepend "Follow: "
    format!("Follow: {}", trimmed)
}

/// Pin constitution bullets to ACE
pub async fn pin_constitution_to_ace(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    bullets: Vec<ConstitutionBullet>,
) -> Result<usize, String> {
    if !config.enabled {
        debug!("ACE pinning skipped: disabled");
        return Ok(0);
    }

    if bullets.is_empty() {
        warn!("No bullets to pin");
        return Ok(0);
    }

    let start = Instant::now();

    // Group bullets by scope and convert to (text, kind) tuples
    let mut scope_bullets: std::collections::HashMap<String, Vec<(String, String)>> =
        std::collections::HashMap::new();

    for bullet in bullets {
        // Determine kind based on bullet characteristics
        let kind = if bullet.text.contains("avoid") || bullet.text.contains("never") {
            "harmful"
        } else {
            "helpful"
        };

        // Add to each scope
        for scope in &bullet.scopes {
            scope_bullets
                .entry(scope.clone())
                .or_insert_with(Vec::new)
                .push((bullet.text.clone(), kind.to_string()));
        }
    }

    let mut total_pinned = 0;

    // Pin to each scope
    for (scope, scope_bullet_list) in scope_bullets {
        let result = ace_client::pin(
            repo_root.clone(),
            branch.clone(),
            scope.clone(),
            scope_bullet_list,
        )
        .await;

        match result {
            AceResult::Ok(response) => {
                total_pinned += response.pinned_count;
                debug!(
                    "Pinned {} bullets to scope {}",
                    response.pinned_count, scope
                );
            }
            AceResult::Error(e) => {
                warn!("Failed to pin to scope {}: {}", scope, e);
            }
            AceResult::Disabled => {}
        }
    }

    let elapsed = start.elapsed();
    info!(
        "ACE pin {}ms pinned={} bullets total",
        elapsed.as_millis(),
        total_pinned
    );

    Ok(total_pinned)
}

/// Synchronous wrapper for pin_constitution_to_ace
///
/// Spawns async task and returns immediately (fire-and-forget).
/// Result is logged but not returned to caller.
pub fn pin_constitution_to_ace_sync(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    bullets: Vec<ConstitutionBullet>,
) -> Result<usize, String> {
    let bullet_count = bullets.len();

    // Clone config for move into async task
    let config = config.clone();

    // Check if we're on a tokio runtime
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // Spawn async task (don't block - we're already on runtime)
            handle.spawn(async move {
                match pin_constitution_to_ace(&config, repo_root, branch, bullets).await {
                    Ok(pinned) => {
                        info!("ACE: Pinned {} bullets to playbook", pinned);
                    }
                    Err(e) => {
                        warn!("ACE pinning failed: {}", e);
                    }
                }
            });

            // Return estimated count immediately
            Ok(bullet_count)
        }
        Err(_) => {
            debug!("ACE pinning skipped: not on tokio runtime");
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bullets_basic() {
        let markdown = r#"
# Constitution

## Rules
- Keep templates synchronized
- Update documentation when changing templates
- Validate all changes with tests
- Never commit without running linters
        "#;

        let bullets = extract_bullets(markdown);
        assert!(bullets.len() >= 4);
        assert!(bullets.len() <= MAX_BULLETS);

        // Check imperatives were extracted
        assert!(bullets.iter().any(|b| b.text.contains("template")));
        assert!(bullets.iter().any(|b| b.text.contains("documentation")));
    }

    #[test]
    fn test_extract_bullets_length_filter() {
        let markdown = r#"
- OK
- This is a good bullet with reasonable length
- This is an extremely long bullet that exceeds the maximum character limit and should be filtered out because it's way too verbose and contains too much information that wouldn't be useful as a quick heuristic
        "#;

        let bullets = extract_bullets(markdown);

        // Should only have the middle bullet
        assert_eq!(bullets.len(), 1);
        assert!(bullets[0].text.contains("reasonable length"));
    }

    #[test]
    fn test_convert_to_imperative() {
        assert_eq!(
            convert_to_imperative("Keep templates in sync"),
            "Keep templates in sync"
        );
        assert_eq!(
            convert_to_imperative("Templates must be synchronized"),
            "Ensure Templates be synchronized"
        );
        assert_eq!(
            convert_to_imperative("All tests should pass"),
            "Ensure all tests should pass"
        );
        assert_eq!(
            convert_to_imperative("The system validates"),
            "Follow: The system validates"
        );
    }

    #[test]
    fn test_detect_scopes_and_tags() {
        let (scopes, tags) = detect_scopes_and_tags("Update plan template with new fields");
        assert!(scopes.contains(&"plan".to_string()));
        assert!(tags.contains(&"templates".to_string()));

        let (scopes2, tags2) = detect_scopes_and_tags("Run linters before implementing");
        assert!(scopes2.contains(&"implement".to_string()));
        assert!(tags2.contains(&"lint".to_string()));

        let (scopes3, tags3) = detect_scopes_and_tags("Keep documentation up to date");
        assert!(scopes3.contains(&"global".to_string()));
        assert!(tags3.contains(&"docs".to_string()));
    }

    #[test]
    fn test_detect_multiple_scopes() {
        let (scopes, _) = detect_scopes_and_tags("Update plan and implementation templates");
        assert!(scopes.contains(&"plan".to_string()));
        assert!(scopes.contains(&"implement".to_string()));
    }

    #[test]
    fn test_constitution_bullet_is_global() {
        let bullet1 = ConstitutionBullet::new("Keep all templates synchronized".to_string());
        assert!(bullet1.is_global());

        let bullet2 = ConstitutionBullet::new("Update plan documentation".to_string());
        assert!(bullet2.scopes.contains(&"plan".to_string()));
        // Plan-specific bullets are not global
        assert!(!bullet2.is_global());
    }
}
