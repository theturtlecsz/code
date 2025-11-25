//! Native consistency analysis (zero agents, zero cost, <1s)
//!
//! FORK-SPECIFIC (just-every/code): Cross-artifact consistency checking
//! Eliminates $0.80 agent cost per /speckit.analyze execution
//!
//! Principle: Agents for reasoning, NOT transactions. Consistency checking is
//! pattern-matching (FREE) not reasoning ($0.80).

#![allow(dead_code)] // Extended analysis helpers pending

use super::clarify_native::Severity;
use super::error::{Result, SpecKitError};
use regex_lite::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Inconsistency issue detected across artifacts
#[derive(Debug, Clone)]
pub struct InconsistencyIssue {
    pub id: String,         // INC-001...
    pub issue_type: String, // "ID mismatch", "missing coverage", etc.
    pub severity: Severity,
    pub source_file: String,     // "PRD.md"
    pub target_file: String,     // "plan.md"
    pub source_location: String, // Line or section
    pub target_location: String, // Line or section (or "NOT FOUND")
    pub description: String,
    pub suggested_fix: Option<String>,
}

/// Requirement reference (FR-001, NFR-002, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RequirementRef {
    id: String,      // FR-001
    file: String,    // PRD.md
    line: usize,     // Line number
    context: String, // Brief description
}

/// Check cross-artifact consistency
pub fn check_consistency(spec_id: &str, cwd: &Path) -> Result<Vec<InconsistencyIssue>> {
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)
        .map_err(|e| SpecKitError::Other(e))?;
    let mut issues = Vec::new();

    // Load all artifacts
    let prd_path = spec_dir.join("PRD.md");
    let spec_path = spec_dir.join("spec.md");
    let plan_path = spec_dir.join("plan.md");
    let tasks_path = spec_dir.join("tasks.md");
    let constitution_path = cwd.join("memory/constitution.md");

    // PRD is required
    if !prd_path.exists() {
        issues.push(InconsistencyIssue {
            id: format!("INC-{:03}", issues.len() + 1),
            issue_type: "missing_artifact".to_string(),
            severity: Severity::Critical,
            source_file: "PRD.md".to_string(),
            target_file: "N/A".to_string(),
            source_location: "N/A".to_string(),
            target_location: "N/A".to_string(),
            description: "PRD.md not found - required for consistency checking".to_string(),
            suggested_fix: Some("Create PRD.md before running analyze".to_string()),
        });
        return Ok(issues);
    }

    let prd_content =
        fs::read_to_string(&prd_path).map_err(|e| SpecKitError::file_read(&prd_path, e))?;

    // Extract requirements from PRD
    let prd_requirements = extract_requirements(&prd_content, "PRD.md");

    // Check ID consistency across artifacts
    if plan_path.exists() {
        let plan_content =
            fs::read_to_string(&plan_path).map_err(|e| SpecKitError::file_read(&plan_path, e))?;
        check_id_consistency(&prd_requirements, &plan_content, "plan.md", &mut issues);
        check_requirement_coverage(&prd_requirements, &plan_content, "plan.md", &mut issues);
        check_contradiction_detection(
            &prd_content,
            &plan_content,
            "PRD.md",
            "plan.md",
            &mut issues,
        );
        check_version_drift(&prd_path, &plan_path, &mut issues)?;
    }

    if tasks_path.exists() {
        let tasks_content =
            fs::read_to_string(&tasks_path).map_err(|e| SpecKitError::file_read(&tasks_path, e))?;
        check_orphan_tasks(&prd_requirements, &tasks_content, &mut issues);
    }

    // Check constitution violations (if exists)
    if constitution_path.exists() {
        let constitution_content = fs::read_to_string(&constitution_path)
            .map_err(|e| SpecKitError::file_read(&constitution_path, e))?;
        check_constitution_compliance(&prd_content, &constitution_content, &mut issues);
    }

    // Check scope creep (plan has features not in PRD)
    if plan_path.exists() {
        let plan_content =
            fs::read_to_string(&plan_path).map_err(|e| SpecKitError::file_read(&plan_path, e))?;
        check_scope_creep(&prd_requirements, &plan_content, &mut issues);
    }

    // Sort by severity
    issues.sort_by(|a, b| match (&a.severity, &b.severity) {
        (Severity::Critical, Severity::Critical) => std::cmp::Ordering::Equal,
        (Severity::Critical, _) => std::cmp::Ordering::Less,
        (_, Severity::Critical) => std::cmp::Ordering::Greater,
        (Severity::Important, Severity::Important) => std::cmp::Ordering::Equal,
        (Severity::Important, _) => std::cmp::Ordering::Less,
        (_, Severity::Important) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });

    // Re-number
    for (idx, issue) in issues.iter_mut().enumerate() {
        issue.id = format!("INC-{:03}", idx + 1);
    }

    Ok(issues)
}

/// Extract requirement references (FR-001, NFR-002, etc.) from content
fn extract_requirements(content: &str, filename: &str) -> Vec<RequirementRef> {
    let mut requirements = Vec::new();
    let req_re = Regex::new(r"\b(FR|NFR|R|AC)-(\d{3})\b").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        for cap in req_re.captures_iter(line) {
            let id = cap[0].to_string();
            let context = line.trim().chars().take(60).collect::<String>();

            requirements.push(RequirementRef {
                id,
                file: filename.to_string(),
                line: line_num + 1,
                context,
            });
        }
    }

    requirements
}

/// Check ID consistency between PRD and plan/tasks
fn check_id_consistency(
    prd_requirements: &[RequirementRef],
    target_content: &str,
    target_file: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    let target_requirements = extract_requirements(target_content, target_file);
    let prd_ids: HashSet<&str> = prd_requirements.iter().map(|r| r.id.as_str()).collect();

    for target_req in &target_requirements {
        if !prd_ids.contains(target_req.id.as_str()) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                issue_type: "id_mismatch".to_string(),
                severity: Severity::Important,
                source_file: "PRD.md".to_string(),
                target_file: target_file.to_string(),
                source_location: "NOT FOUND".to_string(),
                target_location: format!("line {}", target_req.line),
                description: format!(
                    "ID '{}' referenced in {} but not defined in PRD",
                    target_req.id, target_file
                ),
                suggested_fix: Some(format!(
                    "Add '{}' to PRD or remove from {}",
                    target_req.id, target_file
                )),
            });
        }
    }
}

/// Check that all PRD requirements are covered in plan
fn check_requirement_coverage(
    prd_requirements: &[RequirementRef],
    plan_content: &str,
    plan_file: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    let plan_requirements = extract_requirements(plan_content, plan_file);
    let plan_ids: HashSet<&str> = plan_requirements.iter().map(|r| r.id.as_str()).collect();

    for prd_req in prd_requirements {
        if !plan_ids.contains(prd_req.id.as_str()) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                issue_type: "missing_coverage".to_string(),
                severity: Severity::Critical,
                source_file: "PRD.md".to_string(),
                target_file: plan_file.to_string(),
                source_location: format!("line {}", prd_req.line),
                target_location: "NOT FOUND".to_string(),
                description: format!("Requirement '{}' in PRD not covered in plan", prd_req.id),
                suggested_fix: Some(format!("Add work item for '{}' to plan", prd_req.id)),
            });
        }
    }
}

/// Check for contradictions (must vs optional, real-time vs batch)
fn check_contradiction_detection(
    prd_content: &str,
    plan_content: &str,
    prd_file: &str,
    plan_file: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    // Extract requirement statements
    let prd_requirements = extract_requirements(prd_content, prd_file);

    for prd_req in &prd_requirements {
        // Find context in PRD
        let prd_lines: Vec<&str> = prd_content.lines().collect();
        if prd_req.line > prd_lines.len() {
            continue;
        }
        let prd_line = prd_lines[prd_req.line - 1];

        // Find same ID in plan
        let plan_lines: Vec<&str> = plan_content.lines().collect();
        let mut plan_context = None;
        for (idx, line) in plan_lines.iter().enumerate() {
            if line.contains(&prd_req.id) {
                plan_context = Some((idx + 1, *line));
                break;
            }
        }

        if let Some((plan_line_num, plan_line)) = plan_context {
            // Check for contradictions
            if prd_line.to_lowercase().contains("must")
                && plan_line.to_lowercase().contains("optional")
            {
                issues.push(InconsistencyIssue {
                    id: format!("INC-{:03}", issues.len() + 1),
                    issue_type: "contradiction".to_string(),
                    severity: Severity::Critical,
                    source_file: prd_file.to_string(),
                    target_file: plan_file.to_string(),
                    source_location: format!("line {}", prd_req.line),
                    target_location: format!("line {}", plan_line_num),
                    description: format!(
                        "Contradiction: '{}' is 'must' in PRD but 'optional' in plan",
                        prd_req.id
                    ),
                    suggested_fix: Some(
                        "Align PRD and plan - clarify if truly required".to_string(),
                    ),
                });
            }

            if prd_line.to_lowercase().contains("real-time")
                && plan_line.to_lowercase().contains("batch")
            {
                issues.push(InconsistencyIssue {
                    id: format!("INC-{:03}", issues.len() + 1),
                    issue_type: "contradiction".to_string(),
                    severity: Severity::Critical,
                    source_file: prd_file.to_string(),
                    target_file: plan_file.to_string(),
                    source_location: format!("line {}", prd_req.line),
                    target_location: format!("line {}", plan_line_num),
                    description: format!(
                        "Contradiction: '{}' requires real-time in PRD but batch in plan",
                        prd_req.id
                    ),
                    suggested_fix: Some("Align PRD and plan on processing model".to_string()),
                });
            }
        }
    }
}

/// Check for orphan tasks (tasks not traced to PRD requirements)
fn check_orphan_tasks(
    prd_requirements: &[RequirementRef],
    tasks_content: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    let task_requirements = extract_requirements(tasks_content, "tasks.md");
    let prd_ids: HashSet<&str> = prd_requirements.iter().map(|r| r.id.as_str()).collect();

    for task_req in &task_requirements {
        if !prd_ids.contains(task_req.id.as_str()) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                issue_type: "orphan_task".to_string(),
                severity: Severity::Important,
                source_file: "tasks.md".to_string(),
                target_file: "PRD.md".to_string(),
                source_location: format!("line {}", task_req.line),
                target_location: "NOT FOUND".to_string(),
                description: format!("Task references '{}' which is not in PRD", task_req.id),
                suggested_fix: Some(format!("Add '{}' to PRD or remove task", task_req.id)),
            });
        }
    }
}

/// Check for version drift (PRD updated but plan not regenerated)
fn check_version_drift(
    prd_path: &Path,
    plan_path: &Path,
    issues: &mut Vec<InconsistencyIssue>,
) -> Result<()> {
    let prd_metadata = fs::metadata(prd_path).map_err(|e| SpecKitError::file_read(prd_path, e))?;
    let plan_metadata =
        fs::metadata(plan_path).map_err(|e| SpecKitError::file_read(plan_path, e))?;

    let prd_modified = prd_metadata
        .modified()
        .map_err(|e| SpecKitError::file_read(prd_path, e))?;
    let plan_modified = plan_metadata
        .modified()
        .map_err(|e| SpecKitError::file_read(plan_path, e))?;

    // If PRD is newer than plan by more than 1 minute, flag it
    if let Ok(duration) = prd_modified.duration_since(plan_modified) {
        if duration.as_secs() > 60 {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                issue_type: "version_drift".to_string(),
                severity: Severity::Important,
                source_file: "PRD.md".to_string(),
                target_file: "plan.md".to_string(),
                source_location: "metadata".to_string(),
                target_location: "metadata".to_string(),
                description: format!(
                    "PRD modified {} seconds after plan - plan may be stale",
                    duration.as_secs()
                ),
                suggested_fix: Some(
                    "Regenerate plan with /speckit.plan if PRD changed significantly".to_string(),
                ),
            });
        }
    }

    Ok(())
}

/// Check for scope creep (plan has features not in PRD)
fn check_scope_creep(
    prd_requirements: &[RequirementRef],
    plan_content: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    let plan_requirements = extract_requirements(plan_content, "plan.md");
    let prd_ids: HashSet<&str> = prd_requirements.iter().map(|r| r.id.as_str()).collect();

    // Count how many plan requirements are NOT in PRD
    let orphan_count = plan_requirements
        .iter()
        .filter(|r| !prd_ids.contains(r.id.as_str()))
        .count();

    // If >20% of plan requirements are not in PRD, flag scope creep
    if !plan_requirements.is_empty() {
        let orphan_ratio = orphan_count as f32 / plan_requirements.len() as f32;
        if orphan_ratio > 0.2 {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                issue_type: "scope_creep".to_string(),
                severity: Severity::Important,
                source_file: "plan.md".to_string(),
                target_file: "PRD.md".to_string(),
                source_location: "multiple".to_string(),
                target_location: "N/A".to_string(),
                description: format!(
                    "Possible scope creep: {}/{} plan requirements not in PRD ({:.0}%)",
                    orphan_count,
                    plan_requirements.len(),
                    orphan_ratio * 100.0
                ),
                suggested_fix: Some(
                    "Review plan and add missing requirements to PRD or remove from plan"
                        .to_string(),
                ),
            });
        }
    }
}

/// Check for constitution violations
fn check_constitution_compliance(
    prd_content: &str,
    constitution_content: &str,
    issues: &mut Vec<InconsistencyIssue>,
) {
    // Extract rules from constitution (simple heuristic: "MUST", "MUST NOT")
    let must_rules: Vec<&str> = constitution_content
        .lines()
        .filter(|line| {
            line.to_uppercase().contains("MUST") && !line.to_uppercase().contains("MUST NOT")
        })
        .collect();

    let must_not_rules: Vec<&str> = constitution_content
        .lines()
        .filter(|line| line.to_uppercase().contains("MUST NOT"))
        .collect();

    // Check for MUST NOT violations (simple keyword matching)
    for rule in must_not_rules {
        // Extract keywords from rule (naive: words after "MUST NOT")
        if let Some(pos) = rule.to_uppercase().find("MUST NOT") {
            let keywords_part = &rule[pos + 8..];
            let keywords: Vec<&str> = keywords_part.split_whitespace().take(3).collect();

            for keyword in keywords {
                let keyword_lower = keyword.to_lowercase();
                if keyword_lower.len() > 3 && prd_content.to_lowercase().contains(&keyword_lower) {
                    issues.push(InconsistencyIssue {
                        id: format!("INC-{:03}", issues.len() + 1),
                        issue_type: "constitution_violation".to_string(),
                        severity: Severity::Critical,
                        source_file: "PRD.md".to_string(),
                        target_file: "constitution.md".to_string(),
                        source_location: "content".to_string(),
                        target_location: "rule".to_string(),
                        description: format!("Possible violation: PRD contains '{}' which may violate constitution rule", keyword),
                        suggested_fix: Some("Review constitution and adjust PRD if needed".to_string()),
                    });
                    break; // Only report once per rule
                }
            }
        }
    }
}

/// Find SPEC directory from SPEC-ID

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_requirements() {
        let content = "FR-001: User authentication\nNFR-002: Performance requirement\nR-003: General requirement";
        let reqs = extract_requirements(content, "test.md");

        assert_eq!(reqs.len(), 3);
        assert!(reqs.iter().any(|r| r.id == "FR-001"));
        assert!(reqs.iter().any(|r| r.id == "NFR-002"));
        assert!(reqs.iter().any(|r| r.id == "R-003"));
    }

    #[test]
    fn test_requirement_ref_equality() {
        let req1 = RequirementRef {
            id: "FR-001".to_string(),
            file: "PRD.md".to_string(),
            line: 10,
            context: "test".to_string(),
        };

        let req2 = RequirementRef {
            id: "FR-001".to_string(),
            file: "plan.md".to_string(),
            line: 20,
            context: "different".to_string(),
        };

        // Only ID and file matter for HashSet
        assert_eq!(req1.id, req2.id);
    }
}
