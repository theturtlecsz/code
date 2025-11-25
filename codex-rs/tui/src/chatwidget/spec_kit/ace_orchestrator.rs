//! ACE Orchestrator - Full reflection-curation cycle
//!
//! Coordinates the complete ACE workflow:
//! 1. Reflect: Analyze outcomes with LLM to extract patterns
//! 2. Curate: Decide playbook updates strategically with LLM
//! 3. Apply: Update playbook via MCP
//!
//! This is the intelligence layer that makes ACE more than simple scoring.
//!
//! Note: ACE integration is enabled but full orchestration pending validation.

#![allow(dead_code, unused_variables, unreachable_patterns)] // ACE integration in progress

use super::ace_client::{self, AceResult};
use super::ace_curator::{self, CurationDecision, CurationPromptBuilder};
use super::ace_learning::ExecutionFeedback;
use super::ace_reflector::{self, ReflectionPromptBuilder, ReflectionResult};
use codex_core::config_types::AceConfig;
use std::time::Instant;
use tracing::{debug, info, warn};

/// Result of full ACE reflection-curation cycle
#[derive(Debug, Clone)]
pub struct AceCycleResult {
    pub reflection: ReflectionResult,
    pub curation: Option<CurationDecision>,
    pub bullets_added: usize,
    pub bullets_deprecated: usize,
    pub elapsed_ms: u128,
}

/// Call an LLM for reflection/curation
///
/// Uses Gemini Flash 2.5 (cheap, fast) for ACE intelligence
async fn call_llm_for_ace(prompt: String) -> Result<String, String> {
    use tokio::io::AsyncWriteExt;

    // Call Gemini Flash via subprocess (using existing gemini command)
    let mut child = tokio::process::Command::new("gemini")
        .args(["-y", "-m", "gemini-2.5-flash"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn gemini: {}", e))?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| format!("Failed to write prompt: {}", e))?;
        drop(stdin); // Close stdin to signal EOF
    }

    // Read response
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("Failed to wait for gemini: {}", e))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from gemini: {}", e))
    } else {
        Err(format!(
            "gemini exited with status: {}",
            output.status.code().unwrap_or(-1)
        ))
    }
}

/// Synchronous wrapper for call_llm_for_ace (NOT USED - would panic)
#[allow(dead_code)]
fn call_llm_for_ace_sync(prompt: String) -> Result<String, String> {
    // Cannot use block_on when already on tokio runtime
    Err("Cannot block_on from within tokio runtime".to_string())
}

/// Run full ACE reflection-curation cycle
///
/// This is the complete intelligence layer:
/// 1. Reflector analyzes outcomes and extracts patterns (LLM call)
/// 2. Curator decides playbook updates strategically (LLM call)
/// 3. Apply updates via MCP (playbook.pin for new bullets)
pub async fn run_ace_cycle(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    scope: &str,
    task_title: &str,
    feedback: ExecutionFeedback,
    bullets_used_ids: Vec<i32>,
) -> Result<AceCycleResult, String> {
    let start = Instant::now();

    // STEP 1: Reflection (LLM analyzes outcome)
    info!("ACE Reflector: Analyzing execution outcome...");

    let reflection_prompt =
        ReflectionPromptBuilder::new(task_title.to_string(), scope.to_string(), feedback.clone())
            .build();

    let reflection_response = call_llm_for_ace(reflection_prompt).await?;
    let reflection = ace_reflector::parse_reflection_response(&reflection_response)
        .map_err(|e| format!("Reflection parsing failed: {}", e))?;

    info!(
        "ACE Reflector: Discovered {} patterns ({} helpful, {} harmful)",
        reflection.patterns.len(),
        reflection
            .patterns
            .iter()
            .filter(|p| matches!(p.kind, ace_reflector::PatternKind::Helpful))
            .count(),
        reflection
            .patterns
            .iter()
            .filter(|p| matches!(p.kind, ace_reflector::PatternKind::Harmful))
            .count()
    );

    // STEP 2: Check if curation is needed
    let curation_result = if ace_curator::should_curate(&reflection) {
        // Fetch current playbook for curation context
        let current_bullets = match ace_client::playbook_slice(
            repo_root.clone(),
            branch.clone(),
            scope.to_string(),
            20,   // Get more bullets for curation context
            true, // Include neutral
        )
        .await
        {
            AceResult::Ok(response) => response.bullets,
            _ => Vec::new(), // Proceed with empty if unavailable
        };

        info!("ACE Curator: Deciding playbook updates...");

        let curation_prompt =
            CurationPromptBuilder::new(reflection.clone(), current_bullets, scope.to_string())
                .build();

        let curation_response = call_llm_for_ace(curation_prompt).await?;
        let curation = ace_curator::parse_curation_response(&curation_response)
            .map_err(|e| format!("Curation parsing failed: {}", e))?;

        info!(
            "ACE Curator: +{} bullets, -{} deprecated, {} adjustments",
            curation.bullets_to_add.len(),
            curation.bullets_to_deprecate.len(),
            curation.score_adjustments.len()
        );

        Some(curation)
    } else {
        debug!("ACE Curator: Skipped (no high-confidence patterns)");
        None
    };

    // STEP 3: Apply curation decisions via MCP
    let mut bullets_added = 0;
    let bullets_deprecated = 0; // TODO: Implement deprecation via MCP

    if let Some(curation) = &curation_result {
        // Add new bullets
        if !curation.bullets_to_add.is_empty() {
            let bullet_texts: Vec<String> = curation
                .bullets_to_add
                .iter()
                .map(|b| b.text.clone())
                .collect();

            // Convert to (text, kind) tuples
            let bullet_tuples: Vec<(String, String)> = curation
                .bullets_to_add
                .iter()
                .map(|b| (b.text.clone(), b.kind.clone()))
                .collect();

            match ace_client::pin(
                repo_root.clone(),
                branch.clone(),
                "global".to_string(), // Pin curator bullets to global scope
                bullet_tuples,
            )
            .await
            {
                AceResult::Ok(response) => {
                    bullets_added = response.pinned_count;
                    info!("ACE: Pinned {} new bullets to playbook", bullets_added);
                }
                AceResult::Error(e) => {
                    warn!("Failed to pin new bullets: {}", e);
                }
                AceResult::Disabled => {}
            }
        }
    }

    let elapsed = start.elapsed();

    Ok(AceCycleResult {
        reflection,
        curation: curation_result,
        bullets_added,
        bullets_deprecated,
        elapsed_ms: elapsed.as_millis(),
    })
}

/// Synchronous wrapper for run_ace_cycle
///
/// Spawns async task and returns immediately (fire-and-forget).
/// Logs results but doesn't wait for completion.
pub fn run_ace_cycle_sync(
    config: &AceConfig,
    repo_root: String,
    branch: String,
    scope: &str,
    task_title: &str,
    feedback: ExecutionFeedback,
    bullets_used_ids: Vec<i32>,
) -> Result<AceCycleResult, String> {
    // Clone for move into async
    let config = config.clone();
    let scope = scope.to_string();
    let task_title = task_title.to_string();

    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // Spawn async task (don't block - already on runtime)
            handle.spawn(async move {
                match run_ace_cycle(
                    &config,
                    repo_root,
                    branch,
                    &scope,
                    &task_title,
                    feedback,
                    bullets_used_ids,
                )
                .await
                {
                    Ok(result) => {
                        info!(
                            "ACE cycle complete: {}ms, {} patterns, +{} bullets",
                            result.elapsed_ms,
                            result.reflection.patterns.len(),
                            result.bullets_added
                        );
                    }
                    Err(e) => {
                        warn!("ACE cycle failed: {}", e);
                    }
                }
            });

            // Return dummy result (actual work happens async)
            Ok(AceCycleResult {
                reflection: super::ace_reflector::ReflectionResult {
                    patterns: vec![],
                    successes: vec![],
                    failures: vec![],
                    recommendations: vec![],
                    summary: "Processing async...".to_string(),
                },
                curation: None,
                bullets_added: 0,
                bullets_deprecated: 0,
                elapsed_ms: 0,
            })
        }
        Err(_) => Err("Not on tokio runtime".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These are integration-style tests that would need actual LLM access
    // For now, they test the orchestration logic structure

    #[test]
    fn test_ace_cycle_result_construction() {
        use super::super::ace_reflector::{PatternKind, ReflectedPattern};

        let reflection = ReflectionResult {
            patterns: vec![ReflectedPattern {
                pattern: "Test".to_string(),
                rationale: "Because".to_string(),
                kind: PatternKind::Helpful,
                confidence: 0.9,
                scope: "implement".to_string(),
            }],
            successes: vec![],
            failures: vec![],
            recommendations: vec![],
            summary: "Good".to_string(),
        };

        let result = AceCycleResult {
            reflection,
            curation: None,
            bullets_added: 2,
            bullets_deprecated: 1,
            elapsed_ms: 1500,
        };

        assert_eq!(result.bullets_added, 2);
        assert_eq!(result.bullets_deprecated, 1);
        assert!(result.elapsed_ms > 0);
    }
}
