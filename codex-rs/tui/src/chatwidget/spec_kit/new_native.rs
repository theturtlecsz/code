//! Native SPEC creation - eliminates all agents from /speckit.new
//!
//! FORK-SPECIFIC (just-every/code): SPEC-KIT-072 - Native tool migration Phase 2
//!
//! Replaces 2-agent consensus ($0.15) with instant native implementation ($0).
//! Pure template filling and file operations - no AI reasoning required.

#![allow(dead_code)] // Template helpers pending full integration

use chrono::Local;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::SpecKitError;
use super::spec_id_generator::{create_slug, generate_next_spec_id};

/// Result of successful SPEC creation
#[derive(Debug)]
pub struct SpecCreationResult {
    pub spec_id: String,
    pub directory: PathBuf,
    pub files_created: Vec<String>,
    pub feature_name: String,
    pub slug: String,
}

/// Create a new SPEC natively with instant template filling
///
/// Steps:
/// 1. Generate SPEC-ID (already native)
/// 2. Parse description to extract feature name
/// 3. Create directory
/// 4. Fill PRD template
/// 5. Create spec.md
/// 6. Update SPEC.md tracker
///
/// Cost: $0 (zero agents)
/// Time: <1s (instant)
pub fn create_spec(description: &str, cwd: &Path) -> Result<SpecCreationResult, SpecKitError> {
    let description = description.trim();
    if description.is_empty() {
        return Err(SpecKitError::Other(
            "Description cannot be empty".to_string(),
        ));
    }

    // Step 1: Generate SPEC-ID
    let spec_id = generate_next_spec_id(cwd)
        .map_err(|e| SpecKitError::Other(format!("Failed to generate SPEC-ID: {}", e)))?;

    // Step 2: Parse description
    let slug = create_slug(description);
    if slug.is_empty() {
        return Err(SpecKitError::Other(
            "Description must contain alphanumeric characters".to_string(),
        ));
    }

    // Extract feature name (capitalize first letter of each word in description)
    let feature_name = capitalize_words(description);

    // Step 3: Create directory
    let dir_name = format!("{}-{}", spec_id, slug);
    let spec_dir = cwd.join("docs").join(&dir_name);

    fs::create_dir_all(&spec_dir).map_err(|e| SpecKitError::DirectoryCreate {
        path: spec_dir.clone(),
        source: e,
    })?;

    // Step 4: Fill PRD template
    let prd_path = spec_dir.join("PRD.md");
    let prd_content = fill_prd_template(&spec_id, &feature_name, description)?;

    fs::write(&prd_path, prd_content).map_err(|e| SpecKitError::FileWrite {
        path: prd_path.clone(),
        source: e,
    })?;

    // Step 5: Create spec.md
    let spec_path = spec_dir.join("spec.md");
    let spec_content = fill_spec_template(&spec_id, &feature_name, description)?;

    fs::write(&spec_path, spec_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_path.clone(),
        source: e,
    })?;

    // Step 6: Update SPEC.md tracker
    update_spec_tracker(cwd, &spec_id, &feature_name, &dir_name)?;

    Ok(SpecCreationResult {
        spec_id,
        directory: spec_dir,
        files_created: vec!["PRD.md".to_string(), "spec.md".to_string()],
        feature_name,
        slug,
    })
}

/// Fill PRD template with actual values
fn fill_prd_template(
    spec_id: &str,
    feature_name: &str,
    description: &str,
) -> Result<String, SpecKitError> {
    // Read template (fallback to embedded if file not found)
    let template = read_template_or_embedded("PRD-template.md")?;

    let date = Local::now().format("%Y-%m-%d").to_string();

    // Replace placeholders
    let content = template
        .replace("[SPEC_ID]", spec_id)
        .replace("[FEATURE_NAME]", feature_name)
        .replace("[DATE]", &date)
        .replace(
            "[WHAT_EXISTS_TODAY]",
            "Current implementation does not support this feature",
        )
        .replace("[USER_PAIN_1]", &format!("Need to: {}", description))
        .replace("[INEFFICIENCY_1]", "Manual workarounds required")
        .replace("[GAP_1]", "Missing automated solution")
        .replace(
            "[WHY_THIS_MATTERS]",
            "Improves user experience and productivity",
        )
        .replace("[USER_TYPE_1]", "Developer")
        .replace("[USER_DESCRIPTION]", "Software engineer using this system")
        .replace("[HOW_THEY_WORK_TODAY]", "Manual processes")
        .replace("[WHAT_FRUSTRATES_THEM]", "Lack of automation")
        .replace("[WHAT_THEY_WANT]", feature_name)
        .replace("[USER_TYPE_2]", "End User")
        .replace("[DESCRIPTION]", "Person using the final product")
        .replace(
            "[HOW_THEY_USE_THE_SYSTEM]",
            "Through the provided interface",
        )
        .replace("[GOAL_1]", &format!("Implement {}", feature_name))
        .replace("[HOW_TO_MEASURE]", "Feature is functional and tested")
        .replace("[GOAL_2]", "Maintain quality and performance")
        .replace("[METRIC]", "100% test pass rate, no regressions")
        .replace("[SECONDARY_GOAL]", "Documentation complete")
        .replace("[WHAT_WE_WONT_DO_1]", "Out of scope enhancements")
        .replace("[FUTURE_ENHANCEMENT_1]", "Future iterations")
        .replace("[RELATED_BUT_SEPARATE_CONCERN]", "Unrelated features")
        .replace(
            "[WHY_THESE_ARE_NON_GOALS]",
            "Focus on core functionality first",
        )
        .replace("[INCLUDED_FEATURE_1]", description)
        .replace("[INCLUDED_CAPABILITY_2]", "Basic implementation")
        .replace("[ASSUMPTION_1]", "Required dependencies are available")
        .replace(
            "[DEPENDENCY_ASSUMPTION_2]",
            "No breaking changes in upstream",
        )
        .replace(
            "[TECHNICAL_CONSTRAINT]",
            "Must maintain backward compatibility",
        )
        .replace("[RESOURCE_CONSTRAINT]", "Development time available")
        .replace("[TIME_CONSTRAINT]", "Target completion within sprint")
        .replace("[REQUIREMENT_DESCRIPTION]", description)
        .replace("[HOW_TO_VERIFY]", "Feature works as specified")
        .replace("[REQUIREMENT]", "Implementation complete")
        .replace("[CRITERIA]", "Tests pass")
        .replace("[LATENCY_TARGET]", "<100ms")
        .replace("[LOAD_TEST_COMMAND]", "cargo test")
        .replace("[UPTIME_TARGET]", "99.9%")
        .replace("[MONITORING_DASHBOARD]", "Local testing")
        .replace("[SECURITY_STANDARD]", "Follow project security guidelines")
        .replace("[AUDIT_PROCESS]", "Code review")
        .replace("[SCALE_TARGET]", "Handle expected load")
        .replace("[STRESS_TEST]", "Performance benchmarks")
        .replace("[PRIMARY_USER_FLOW]", &format!("Using {}", feature_name))
        .replace("[ACTION_1]", "Initiates feature")
        .replace("[RESPONSE_1]", "System responds")
        .replace("[ACTION_2]", "Completes action")
        .replace("[RESPONSE_2]", "Feature executes")
        .replace("[HAPPY_PATH_OUTCOME]", "Feature works correctly")
        .replace("[ERROR_CONDITION]", "invalid input")
        .replace("[HANDLING]", "show error message")
        .replace("[SECONDARY_FLOW]", "Error handling")
        .replace("[LIBRARY_OR_FRAMEWORK_1]", "Project dependencies")
        .replace("[SERVICE_DEPENDENCY_1]", "None")
        .replace("[TEAM_DEPENDENCY]", "Code review approval")
        .replace("[APPROVAL_REQUIREMENT]", "PRD acceptance")
        .replace("[DATA_SOURCE_1]", "Application state")
        .replace("[MIGRATION_REQUIREMENT]", "None")
        .replace("[RISK_1]", "Implementation complexity")
        .replace("[IMPACT]", "High")
        .replace("[MITIGATION_STRATEGY]", "Incremental development")
        .replace("[OWNER]", "Code")
        .replace("[RISK_2]", "Schedule slip")
        .replace("[STRATEGY]", "Regular status updates")
        .replace("[CRITERION_1]", "All tests pass")
        .replace("[CRITERION_2]", "Documentation complete")
        .replace("[KPI_1]", "Feature adoption")
        .replace("[VALUE]", "TBD")
        .replace("[KPI_2]", "User satisfaction")
        .replace("[USER_SATISFACTION]", "Positive feedback")
        .replace("[SCORE]", "≥4/5")
        .replace("[COVERAGE_TARGET]", "≥80%")
        .replace("[TEST_SCENARIOS]", "Happy path and error cases")
        .replace("[CRITICAL_PATHS]", "Main user workflows")
        .replace("[LOAD_PROFILE]", "Expected usage patterns")
        .replace("[STAKEHOLDERS]", "Team leads")
        .replace("[REVIEWERS]", "Architects")
        .replace("[PROCESS]", "Standard code review")
        .replace("[IF_APPLICABLE]", "Security audit if needed")
        .replace("[SCORE_OR_ASSESSMENT]", "Complete - all sections filled")
        .replace("[ALL_REQUIREMENTS_UNAMBIGUOUS]", "Clear and specific")
        .replace("[ALL_CRITERIA_MEASURABLE]", "Testable criteria defined")
        .replace(
            "[IF_AGENTS_DISAGREED_ON_SCOPE_OR_APPROACH]",
            "N/A - native generation",
        )
        .replace("[HOW_CONSENSUS_REACHED]", "Native template instantiation")
        .replace("[QUESTION_1]", "Are there specific edge cases to handle?")
        .replace("[WHAT_NEEDS_CLARIFICATION]", "Detailed requirements")
        .replace("[LEVEL]", "MEDIUM")
        .replace("[BLOCKER]", "NO")
        .replace("[QUESTION_2]", "Performance requirements?")
        .replace("[UNRESOLVED_DECISION]", "Specific benchmarks")
        .replace("[HOW_TO_RESOLVE]", "Define during planning")
        .replace(
            "[MAJOR_DECISIONS_MADE]",
            &format!("Created SPEC for: {}", description),
        );

    Ok(content)
}

/// Fill spec.md template with metadata
fn fill_spec_template(
    spec_id: &str,
    feature_name: &str,
    description: &str,
) -> Result<String, SpecKitError> {
    let template = read_template_or_embedded("spec-template.md")?;

    let date = Local::now().format("%Y-%m-%d").to_string();

    let content = template
        .replace("[SPEC_ID]", spec_id)
        .replace("[FEATURE_NAME]", feature_name)
        .replace("[CREATION_DATE]", &date)
        .replace("[BRANCH_NAME]", "TBD")
        .replace("[OWNER]", "Code")
        .replace("[BACKGROUND_PROBLEM_STATEMENT]", description)
        .replace(
            "[HIGH_PRIORITY_USER_STORY_TITLE]",
            &format!("Implement {}", feature_name),
        )
        .replace("[USER_TYPE]", "developer")
        .replace("[GOAL]", feature_name)
        .replace("[BENEFIT]", "improved functionality")
        .replace("[WHY_THIS_IS_P1]", "Core feature requirement")
        .replace("[HOW_TO_VERIFY_INDEPENDENTLY]", "Manual testing")
        .replace("[CONTEXT]", "feature is enabled")
        .replace("[ACTION]", "user triggers feature")
        .replace("[OUTCOME]", "feature executes correctly")
        .replace("[ERROR_CONDITION]", "invalid state")
        .replace("[ERROR_HANDLING]", "error message displayed")
        .replace("[MEDIUM_PRIORITY_STORY_TITLE]", "Edge case handling")
        .replace("[WHY_P2_NOT_P1]", "Nice to have, not critical")
        .replace("[VERIFICATION_METHOD]", "Unit tests")
        .replace("[LOW_PRIORITY_STORY_TITLE]", "Performance optimization")
        .replace("[WHY_P3]", "Future enhancement")
        .replace("[VERIFICATION]", "Benchmarks")
        .replace("[BOUNDARY_CONDITION_1]", "Empty input")
        .replace("[NULL_OR_EMPTY_INPUT_HANDLING]", "Return validation error")
        .replace("[CONCURRENT_ACCESS_SCENARIO]", "Handle race conditions")
        .replace("[ERROR_RECOVERY_CASE]", "Retry on failure")
        .replace("[PERFORMANCE_LIMIT_CASE]", "Throttle under high load")
        .replace("[REQUIREMENT_WITH_ACCEPTANCE_CRITERIA]", description)
        .replace("[REQUIREMENT]", "Feature implemented")
        .replace("[METRIC_OR_CONSTRAINT]", "<100ms response time")
        .replace("[SECURITY_REQUIREMENT]", "Follow security best practices")
        .replace("[SCALE_REQUIREMENT]", "Handle expected concurrency")
        .replace("[UPTIME_OR_ERROR_RATE]", "99.9% availability")
        .replace("[MEASURABLE_OUTCOME_1]", "Feature works as specified")
        .replace("[QUANTIFIABLE_METRIC_2]", "100% test pass rate")
        .replace("[OBJECTIVE_SUCCESS_INDICATOR_3]", "No regressions")
        .replace(
            "[QUESTION_OR_AMBIGUITY]",
            "Initial spec created via /speckit.new",
        )
        .replace(
            "[ANSWER_OR_DECISION]",
            "Use /speckit.clarify to resolve ambiguities",
        )
        .replace("[WHICH_PARTS_CHANGED]", "N/A - initial creation")
        .replace("[UPSTREAM_DEPENDENCY_1]", "None identified")
        .replace("[SERVICE_REQUIREMENT_1]", "None")
        .replace("[PREREQUISITE_SPEC_1]", "None")
        .replace(
            "[ANY_ADDITIONAL_CONTEXT_OR_WARNINGS]",
            "Created via native SPEC generation. Run /speckit.clarify to fill in details.",
        );

    Ok(content)
}

/// Read template file or use minimal fallback
fn read_template_or_embedded(filename: &str) -> Result<String, SpecKitError> {
    // Try reading from templates directory
    let templates_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("templates");

    let template_path = templates_dir.join(filename);

    if let Ok(content) = fs::read_to_string(&template_path) {
        return Ok(content);
    }

    // Fallback to minimal embedded templates (build-time include not available)
    match filename {
        "PRD-template.md" => Ok(EMBEDDED_PRD_TEMPLATE.to_string()),
        "spec-template.md" => Ok(EMBEDDED_SPEC_TEMPLATE.to_string()),
        _ => Err(SpecKitError::Other(format!(
            "Unknown template: {}",
            filename
        ))),
    }
}

// Minimal embedded templates as fallback
const EMBEDDED_PRD_TEMPLATE: &str = r#"# PRD: [FEATURE_NAME]

**SPEC-ID**: [SPEC_ID]
**Status**: Draft
**Created**: [DATE]
**Author**: Native SPEC generation

---

## Problem Statement

**Current State**: [WHAT_EXISTS_TODAY]

**Pain Points**:
- [USER_PAIN_1]
- [INEFFICIENCY_1]

**Impact**: [WHY_THIS_MATTERS]

---

## Requirements

### Functional Requirements

- **FR1**: [REQUIREMENT_DESCRIPTION]

### Non-Functional Requirements

- **NFR1**: Performance - [LATENCY_TARGET]
- **NFR2**: Reliability - [UPTIME_TARGET]

---

## Success Criteria

- [CRITERION_1]
- [CRITERION_2]

---

## Next Steps

Use `/speckit.clarify [SPEC_ID]` to fill in details and resolve ambiguities.
"#;

const EMBEDDED_SPEC_TEMPLATE: &str = r#"**SPEC-ID**: [SPEC_ID]
**Feature**: [FEATURE_NAME]
**Status**: Backlog
**Created**: [CREATION_DATE]
**Branch**: [BRANCH_NAME]
**Owner**: [OWNER]

**Context**: [BACKGROUND_PROBLEM_STATEMENT]

---

## Requirements

### Functional Requirements

- **FR1**: [REQUIREMENT_WITH_ACCEPTANCE_CRITERIA]

### Non-Functional Requirements

- **Performance**: [METRIC_OR_CONSTRAINT]

---

## Success Criteria

- [MEASURABLE_OUTCOME_1]

---

## Notes

Created via native SPEC generation. Run `/speckit.clarify [SPEC_ID]` to fill in details.
"#;

/// Update SPEC.md tracker with new row
fn update_spec_tracker(
    cwd: &Path,
    spec_id: &str,
    feature_name: &str,
    dir_name: &str,
) -> Result<(), SpecKitError> {
    let spec_md_path = cwd.join("SPEC.md");

    let content = fs::read_to_string(&spec_md_path).map_err(|e| SpecKitError::FileRead {
        path: spec_md_path.clone(),
        source: e,
    })?;

    // Find the task table section
    let lines: Vec<&str> = content.lines().collect();
    let mut insert_index = None;

    // Look for the task table (should have "| Order | Task ID |" header)
    for (i, line) in lines.iter().enumerate() {
        if line.contains("| Order | Task ID |") || line.contains("|-------|---------|") {
            // Insert after the separator line
            if line.contains("|-------|") {
                insert_index = Some(i + 1);
                break;
            }
        }
    }

    let date = Local::now().format("%Y-%m-%d").to_string();

    // Find the next order number by parsing existing rows
    let next_order = find_next_order_number(&lines);

    let new_row = format!(
        "| {} | {} | {} | Backlog | Code | docs/{}/PRD.md | | | {} | | | Created via /speckit.new (native, $0) |",
        next_order, spec_id, feature_name, dir_name, date
    );

    let updated_content = if let Some(index) = insert_index {
        let mut new_lines = lines.clone();
        new_lines.insert(index, &new_row);
        new_lines.join("\n")
    } else {
        // Append to end if table not found
        format!("{}\n{}\n", content, new_row)
    };

    fs::write(&spec_md_path, updated_content).map_err(|e| SpecKitError::FileWrite {
        path: spec_md_path.clone(),
        source: e,
    })?;

    Ok(())
}

/// Find the next order number by parsing existing rows
fn find_next_order_number(lines: &[&str]) -> usize {
    let mut max_order = 0;

    for line in lines {
        if line.starts_with('|') && line.contains("SPEC-KIT-") {
            // Parse order number (first column)
            let parts: Vec<&str> = line.split('|').collect();
            if parts.len() > 1
                && let Ok(order) = parts[1].trim().parse::<usize>()
            {
                max_order = max_order.max(order);
            }
        }
    }

    max_order + 1
}

/// Capitalize first letter of each word
fn capitalize_words(s: &str) -> String {
    s.split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_spec_basic() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        // Create SPEC.md with minimal table
        let spec_md = temp.path().join("SPEC.md");
        fs::write(
            &spec_md,
            "# SPEC\n\n| Order | Task ID | Title |\n|-------|---------|-------|\n",
        )
        .unwrap();

        let result = create_spec("Add user authentication", temp.path()).unwrap();

        assert_eq!(result.spec_id, "SPEC-KIT-001");
        assert_eq!(result.feature_name, "Add User Authentication");
        assert_eq!(result.slug, "add-user-authentication");
        assert!(result.directory.exists());
        assert!(result.directory.join("PRD.md").exists());
        assert!(result.directory.join("spec.md").exists());
    }

    #[test]
    fn test_create_spec_increments_id() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();
        fs::create_dir(docs.join("SPEC-KIT-005-existing")).unwrap();

        let spec_md = temp.path().join("SPEC.md");
        fs::write(
            &spec_md,
            "# SPEC\n\n| Order | Task ID |\n|-------|---------||\n",
        )
        .unwrap();

        let result = create_spec("Test feature", temp.path()).unwrap();
        assert_eq!(result.spec_id, "SPEC-KIT-006");
    }

    #[test]
    fn test_create_spec_empty_description() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let result = create_spec("", temp.path());
        assert!(result.is_err());
        assert!(matches!(result, Err(SpecKitError::Other(_))));
    }

    #[test]
    fn test_create_spec_special_chars() {
        let temp = TempDir::new().unwrap();
        let docs = temp.path().join("docs");
        fs::create_dir(&docs).unwrap();

        let spec_md = temp.path().join("SPEC.md");
        fs::write(
            &spec_md,
            "# SPEC\n\n| Order | Task ID |\n|-------|---------||\n",
        )
        .unwrap();

        let result = create_spec("Fix: Error in @parser (v2.0)", temp.path()).unwrap();
        assert_eq!(result.slug, "fix-error-in-parser-v2-0");
    }

    #[test]
    fn test_capitalize_words() {
        assert_eq!(
            capitalize_words("add user authentication"),
            "Add User Authentication"
        );
        assert_eq!(capitalize_words("fix bug"), "Fix Bug");
        assert_eq!(capitalize_words("OAuth2 integration"), "OAuth2 Integration");
    }

    #[test]
    fn test_find_next_order_number() {
        let lines = vec![
            "| Order | Task ID |",
            "|-------|---------|",
            "| 1 | SPEC-KIT-001 | Test |",
            "| 5 | SPEC-KIT-005 | Another |",
            "| 3 | SPEC-KIT-003 | Middle |",
        ];

        assert_eq!(find_next_order_number(&lines), 6);
    }

    #[test]
    fn test_find_next_order_number_empty() {
        let lines = vec!["| Order | Task ID |", "|-------|---------|"];

        assert_eq!(find_next_order_number(&lines), 1);
    }
}
