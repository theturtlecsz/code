//! UI-independent vision persistence logic (CLI reuse)
//!
//! Extracts the core vision persistence logic from vision_builder_handler.rs
//! to enable headless CLI usage without ChatWidget dependencies.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use super::error::SpecKitError;

/// Result of vision persistence operation
#[derive(Debug, Clone)]
pub struct VisionPersistenceResult {
    /// Constitution version after persistence
    pub constitution_version: u32,
    /// SHA256 hash of all vision content
    pub content_hash: String,
    /// Number of goals stored
    pub goals_count: usize,
    /// Number of non-goals stored
    pub non_goals_count: usize,
    /// Number of principles stored
    pub principles_count: usize,
    /// Number of guardrails stored
    pub guardrails_count: usize,
    /// Number of Tier 2 cache entries invalidated
    pub cache_invalidated: usize,
    /// Filesystem projections created
    pub projections: VisionProjections,
}

/// Filesystem projections created during vision persistence
#[derive(Debug, Clone)]
pub struct VisionProjections {
    /// Path to NL_VISION.md if created
    pub nl_vision_path: Option<PathBuf>,
}

/// Persist vision answers to OverlayDb (no ChatWidget dependency)
///
/// # Arguments
/// * `cwd` - Working directory for the project
/// * `answers` - Vision answers (Users, Problem, Goals, NonGoals, Principles, Guardrails)
///
/// # Returns
/// * `Ok(VisionPersistenceResult)` on success with all counts and projections
/// * `Err(SpecKitError)` on any failure (hard fail for headless mode)
pub fn persist_vision_to_overlay(
    cwd: &Path,
    answers: &HashMap<String, String>,
) -> Result<VisionPersistenceResult, SpecKitError> {
    // Extract answers by category
    let target_users = answers.get("Users").cloned().unwrap_or_default();
    let problem_statement = answers.get("Problem").cloned().unwrap_or_default();
    let goals_raw = answers.get("Goals").cloned().unwrap_or_default();
    let nongoals_raw = answers.get("NonGoals").cloned().unwrap_or_default();
    let principles_raw = answers.get("Principles").cloned().unwrap_or_default();
    let guardrails_raw = answers.get("Guardrails").cloned().unwrap_or_default();

    // Parse semicolon-separated lists
    let goals: Vec<String> = goals_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let nongoals: Vec<String> = nongoals_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let principles: Vec<String> = principles_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let guardrails: Vec<String> = guardrails_raw
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Connect to overlay DB
    let config = codex_stage0::Stage0Config::load().map_err(|e| {
        SpecKitError::VisionPersistence(format!("Failed to load Stage0 config: {}", e))
    })?;

    let db = codex_stage0::OverlayDb::connect_and_init(&config).map_err(|e| {
        SpecKitError::VisionPersistence(format!("Failed to connect to overlay DB: {}", e))
    })?;

    let mut errors: Vec<String> = Vec::new();

    // Store goals as ConstitutionType::Goal (priority 8)
    for (i, goal) in goals.iter().enumerate() {
        let memory_id = format!("vision-goal-{}", uuid::Uuid::new_v4());
        if let Err(e) =
            db.upsert_constitution_memory(&memory_id, codex_stage0::ConstitutionType::Goal, goal)
        {
            errors.push(format!("Goal {}: {}", i + 1, e));
        }
    }

    // Store non-goals as ConstitutionType::NonGoal (priority 8)
    for (i, nongoal) in nongoals.iter().enumerate() {
        let memory_id = format!("vision-nongoal-{}", uuid::Uuid::new_v4());
        if let Err(e) = db.upsert_constitution_memory(
            &memory_id,
            codex_stage0::ConstitutionType::NonGoal,
            nongoal,
        ) {
            errors.push(format!("Non-goal {}: {}", i + 1, e));
        }
    }

    // Store principles as ConstitutionType::Principle (priority 9)
    for (i, principle) in principles.iter().enumerate() {
        let memory_id = format!("vision-principle-{}", uuid::Uuid::new_v4());
        if let Err(e) = db.upsert_constitution_memory(
            &memory_id,
            codex_stage0::ConstitutionType::Principle,
            principle,
        ) {
            errors.push(format!("Principle {}: {}", i + 1, e));
        }
    }

    // Store guardrails as ConstitutionType::Guardrail (priority 10)
    for (i, guardrail) in guardrails.iter().enumerate() {
        let memory_id = format!("vision-guardrail-{}", uuid::Uuid::new_v4());
        if let Err(e) = db.upsert_constitution_memory(
            &memory_id,
            codex_stage0::ConstitutionType::Guardrail,
            guardrail,
        ) {
            errors.push(format!("Guardrail {}: {}", i + 1, e));
        }
    }

    // Compute content hash
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    target_users.hash(&mut hasher);
    problem_statement.hash(&mut hasher);
    goals_raw.hash(&mut hasher);
    nongoals_raw.hash(&mut hasher);
    principles_raw.hash(&mut hasher);
    guardrails_raw.hash(&mut hasher);
    let content_hash = format!("{:016x}", hasher.finish());

    // Increment constitution version
    let constitution_version = match db.increment_constitution_version(Some(&content_hash)) {
        Ok(v) => v,
        Err(e) => {
            errors.push(format!("Version increment: {}", e));
            0
        }
    };

    // Invalidate Tier 2 cache
    let cache_invalidated = match db.invalidate_tier2_by_constitution() {
        Ok(count) => count,
        Err(e) => {
            tracing::warn!("Failed to invalidate Tier 2 cache: {}", e);
            0
        }
    };

    // Log telemetry event
    tracing::info!(
        event_type = "VisionDefined",
        constitution_version = constitution_version,
        goals_count = goals.len(),
        nongoals_count = nongoals.len(),
        principles_count = principles.len(),
        guardrails_count = guardrails.len(),
        source = "headless_cli",
        "Vision defined for project"
    );

    // Generate NL_VISION.md
    let nl_vision_path = generate_nl_vision(
        cwd,
        &target_users,
        &problem_statement,
        &goals,
        &nongoals,
        &principles,
        &guardrails,
    )?;

    // If there were any errors storing constitution memories, return them
    if !errors.is_empty() {
        return Err(SpecKitError::VisionPersistence(format!(
            "Partial failures during vision persistence: {}",
            errors.join("; ")
        )));
    }

    Ok(VisionPersistenceResult {
        constitution_version,
        content_hash,
        goals_count: goals.len(),
        non_goals_count: nongoals.len(),
        principles_count: principles.len(),
        guardrails_count: guardrails.len(),
        cache_invalidated,
        projections: VisionProjections {
            nl_vision_path: Some(nl_vision_path),
        },
    })
}

/// Generate NL_VISION.md artifact
fn generate_nl_vision(
    cwd: &Path,
    target_users: &str,
    problem_statement: &str,
    goals: &[String],
    nongoals: &[String],
    principles: &[String],
    guardrails: &[String],
) -> Result<PathBuf, SpecKitError> {
    let memory_dir = cwd.join("memory");
    std::fs::create_dir_all(&memory_dir).map_err(|e| {
        SpecKitError::VisionPersistence(format!("Failed to create memory directory: {}", e))
    })?;

    let mut md = String::new();
    md.push_str("# Project Vision\n\n");
    md.push_str("_Auto-generated by vision persistence. Do not edit directly._\n\n");

    md.push_str("## Target Users\n\n");
    md.push_str(target_users);
    md.push_str("\n\n");

    md.push_str("## Problem Statement\n\n");
    md.push_str(problem_statement);
    md.push_str("\n\n");

    md.push_str("## Goals\n\n");
    for goal in goals {
        md.push_str(&format!("- {}\n", goal));
    }
    md.push('\n');

    md.push_str("## Non-Goals\n\n");
    for nongoal in nongoals {
        md.push_str(&format!("- {}\n", nongoal));
    }
    md.push('\n');

    md.push_str("## Principles\n\n");
    for principle in principles {
        md.push_str(&format!("- {}\n", principle));
    }
    md.push('\n');

    md.push_str("## Guardrails\n\n");
    for guardrail in guardrails {
        md.push_str(&format!("- {}\n", guardrail));
    }

    let vision_path = memory_dir.join("NL_VISION.md");
    std::fs::write(&vision_path, &md).map_err(|e| {
        SpecKitError::VisionPersistence(format!("Failed to write NL_VISION.md: {}", e))
    })?;

    Ok(vision_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_nl_vision_creates_file() {
        let temp = TempDir::new().unwrap();
        let result = generate_nl_vision(
            temp.path(),
            "Developers",
            "Need better tooling",
            &["Fast builds".to_string(), "Good DX".to_string()],
            &["Mobile support".to_string()],
            &["Simplicity".to_string()],
            &["No breaking changes".to_string()],
        );

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# Project Vision"));
        assert!(content.contains("Developers"));
        assert!(content.contains("Fast builds"));
        assert!(content.contains("Mobile support"));
        assert!(content.contains("Simplicity"));
        assert!(content.contains("No breaking changes"));
    }
}
