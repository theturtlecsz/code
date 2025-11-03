//! Native guardrail validation (SPEC-KIT-066, SPEC-KIT-902)
//!
//! Lightweight checks that run before each stage:
//! - Clean tree validation
//! - SPEC ID and file structure validation
//! - Telemetry emission
//!
//! Quality gates (clarify/analyze/checklist) provide comprehensive validation.
//! Guardrails provide fast (<1s) sanity checks.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::spec_prompts::SpecStage;

/// Result of guardrail validation
#[derive(Debug, Clone)]
pub struct GuardrailResult {
    pub success: bool,
    pub stage: SpecStage,
    pub spec_id: String,
    pub checks_run: Vec<GuardrailCheck>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
    pub telemetry_path: Option<PathBuf>,
}

/// Individual guardrail check
#[derive(Debug, Clone)]
pub struct GuardrailCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CheckStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
}

impl GuardrailResult {
    pub fn new(stage: SpecStage, spec_id: String) -> Self {
        Self {
            success: true,
            stage,
            spec_id,
            checks_run: Vec::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            telemetry_path: None,
        }
    }

    pub fn add_check(&mut self, check: GuardrailCheck) {
        if check.status == CheckStatus::Failed {
            self.success = false;
        }
        self.checks_run.push(check);
    }

    pub fn add_warning(&mut self, msg: String) {
        self.warnings.push(msg);
    }

    pub fn add_error(&mut self, msg: String) {
        self.errors.push(msg);
        self.success = false;
    }
}

/// Run native guardrail validation for a stage
pub fn run_native_guardrail(
    cwd: &Path,
    spec_id: &str,
    stage: SpecStage,
    allow_dirty: bool,
) -> GuardrailResult {
    let mut result = GuardrailResult::new(stage, spec_id.to_string());

    // Check 1: SPEC ID validation
    result.add_check(validate_spec_id(cwd, spec_id));

    // Check 2: Required files exist
    result.add_check(validate_spec_files(cwd, spec_id));

    // Check 3: Clean tree (unless explicitly allowed dirty)
    if !allow_dirty && !is_allow_dirty_env() {
        result.add_check(validate_clean_tree(cwd));
    } else {
        result.add_check(GuardrailCheck {
            name: "clean-tree".to_string(),
            status: CheckStatus::Skipped,
            message: Some("Skipped (SPEC_OPS_ALLOW_DIRTY set)".to_string()),
        });
    }

    // Check 4: Stage-specific validations
    match stage {
        SpecStage::Plan => {
            result.add_check(validate_plan_stage(cwd, spec_id));
        }
        SpecStage::Tasks => {
            result.add_check(validate_tasks_stage(cwd, spec_id));
        }
        SpecStage::Implement => {
            result.add_check(validate_implement_stage(cwd, spec_id));
        }
        SpecStage::Validate => {
            result.add_check(validate_validate_stage(cwd, spec_id));
        }
        SpecStage::Audit => {
            result.add_check(validate_audit_stage(cwd, spec_id));
        }
        SpecStage::Unlock => {
            result.add_check(validate_unlock_stage(cwd, spec_id));
        }
        _ => {
            // Quality stages (Clarify, Analyze, Checklist) don't need guardrails
        }
    }

    // Emit telemetry
    if let Ok(path) = emit_telemetry(cwd, &result) {
        result.telemetry_path = Some(path);
    }

    result
}

// === Individual Check Implementations ===

fn validate_spec_id(cwd: &Path, spec_id: &str) -> GuardrailCheck {
    // Check format: SPEC-XXX-NNN or similar
    let valid_format = spec_id.starts_with("SPEC-") && spec_id.len() > 5;

    if !valid_format {
        return GuardrailCheck {
            name: "spec-id-format".to_string(),
            status: CheckStatus::Warning,
            message: Some(format!("SPEC ID '{}' doesn't match expected format SPEC-*", spec_id)),
        };
    }

    // Check if SPEC directory exists
    let spec_dir = find_spec_directory(cwd, spec_id);
    if spec_dir.is_none() {
        return GuardrailCheck {
            name: "spec-id-exists".to_string(),
            status: CheckStatus::Failed,
            message: Some(format!("SPEC directory not found for '{}'", spec_id)),
        };
    }

    GuardrailCheck {
        name: "spec-id-validation".to_string(),
        status: CheckStatus::Passed,
        message: Some(format!("SPEC ID '{}' is valid", spec_id)),
    }
}

fn validate_spec_files(cwd: &Path, spec_id: &str) -> GuardrailCheck {
    let Some(spec_dir) = find_spec_directory(cwd, spec_id) else {
        return GuardrailCheck {
            name: "spec-files".to_string(),
            status: CheckStatus::Skipped,
            message: Some("SPEC directory not found".to_string()),
        };
    };

    let spec_md = spec_dir.join("spec.md");

    if !spec_md.exists() {
        return GuardrailCheck {
            name: "spec-files".to_string(),
            status: CheckStatus::Failed,
            message: Some("spec.md not found in SPEC directory".to_string()),
        };
    }

    GuardrailCheck {
        name: "spec-files".to_string(),
        status: CheckStatus::Passed,
        message: Some("Required SPEC files present".to_string()),
    }
}

fn validate_clean_tree(cwd: &Path) -> GuardrailCheck {
    // Check git status
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(cwd)
        .output();

    let Ok(output) = output else {
        return GuardrailCheck {
            name: "clean-tree".to_string(),
            status: CheckStatus::Warning,
            message: Some("Could not check git status".to_string()),
        };
    };

    let status = String::from_utf8_lossy(&output.stdout);

    // Filter out expected stage artifacts and telemetry files that are auto-generated
    let unexpected_changes: Vec<&str> = status
        .lines()
        .filter(|line| {
            let line = line.trim();
            // Allow stage output files (plan.md, tasks.md, etc.)
            if line.contains("/plan.md") || line.contains("/tasks.md") ||
               line.contains("/validate.md") || line.contains("/implement.md") ||
               line.contains("/audit.md") || line.contains("/unlock.md") {
                return false;
            }
            // Allow evidence/telemetry files
            if line.contains("/evidence/") || line.contains("_cost_summary.json") ||
               line.contains("_telemetry.json") {
                return false;
            }
            true
        })
        .collect();

    if !unexpected_changes.is_empty() {
        return GuardrailCheck {
            name: "clean-tree".to_string(),
            status: CheckStatus::Failed,
            message: Some(format!("Working tree has {} unexpected changes (stage artifacts excluded)", unexpected_changes.len())),
        };
    }

    GuardrailCheck {
        name: "clean-tree".to_string(),
        status: CheckStatus::Passed,
        message: Some("Working tree is clean (stage artifacts excluded)".to_string()),
    }
}

fn validate_plan_stage(_cwd: &Path, _spec_id: &str) -> GuardrailCheck {
    // Plan stage: Could check if PRD exists, but let quality gates handle that
    GuardrailCheck {
        name: "plan-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Plan stage ready".to_string()),
    }
}

fn validate_tasks_stage(cwd: &Path, spec_id: &str) -> GuardrailCheck {
    // Tasks stage: Check if plan exists
    let Some(spec_dir) = find_spec_directory(cwd, spec_id) else {
        return GuardrailCheck {
            name: "tasks-stage".to_string(),
            status: CheckStatus::Skipped,
            message: Some("SPEC directory not found".to_string()),
        };
    };

    let plan_md = spec_dir.join("plan.md");
    if !plan_md.exists() {
        return GuardrailCheck {
            name: "tasks-stage".to_string(),
            status: CheckStatus::Warning,
            message: Some("plan.md not found (should run /speckit.plan first)".to_string()),
        };
    }

    GuardrailCheck {
        name: "tasks-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Plan exists, ready for tasks".to_string()),
    }
}

fn validate_implement_stage(cwd: &Path, spec_id: &str) -> GuardrailCheck {
    // Implement stage: Check if tasks exist
    let Some(spec_dir) = find_spec_directory(cwd, spec_id) else {
        return GuardrailCheck {
            name: "implement-stage".to_string(),
            status: CheckStatus::Skipped,
            message: Some("SPEC directory not found".to_string()),
        };
    };

    let tasks_md = spec_dir.join("tasks.md");
    if !tasks_md.exists() {
        return GuardrailCheck {
            name: "implement-stage".to_string(),
            status: CheckStatus::Warning,
            message: Some("tasks.md not found (should run /speckit.tasks first)".to_string()),
        };
    }

    GuardrailCheck {
        name: "implement-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Tasks exist, ready for implementation".to_string()),
    }
}

fn validate_validate_stage(_cwd: &Path, _spec_id: &str) -> GuardrailCheck {
    // Validate stage: Minimal checks (quality gates handle validation)
    GuardrailCheck {
        name: "validate-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Ready for validation".to_string()),
    }
}

fn validate_audit_stage(_cwd: &Path, _spec_id: &str) -> GuardrailCheck {
    // Audit stage: Minimal checks
    GuardrailCheck {
        name: "audit-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Ready for audit".to_string()),
    }
}

fn validate_unlock_stage(_cwd: &Path, _spec_id: &str) -> GuardrailCheck {
    // Unlock stage: Final checks before unlock
    GuardrailCheck {
        name: "unlock-stage".to_string(),
        status: CheckStatus::Passed,
        message: Some("Ready for unlock".to_string()),
    }
}

// === Helper Functions ===

fn find_spec_directory(cwd: &Path, spec_id: &str) -> Option<PathBuf> {
    let docs_dir = cwd.join("docs");
    if !docs_dir.exists() {
        return None;
    }

    // Try exact match first
    let entries = std::fs::read_dir(&docs_dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Match SPEC-XXX-NNN-slug format
        if name_str.starts_with(spec_id) && entry.path().is_dir() {
            return Some(entry.path());
        }
    }

    None
}

fn is_allow_dirty_env() -> bool {
    std::env::var("SPEC_OPS_ALLOW_DIRTY")
        .map(|v| v == "1")
        .unwrap_or(false)
}

fn emit_telemetry(cwd: &Path, result: &GuardrailResult) -> Result<PathBuf, std::io::Error> {
    use std::io::Write;

    let evidence_dir = cwd.join("docs")
        .join("SPEC-OPS-004-integrated-coder-hooks")
        .join("evidence")
        .join("commands")
        .join(&result.spec_id);

    std::fs::create_dir_all(&evidence_dir)?;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let session_id = std::process::id();
    let filename = format!("guardrail-{}-{}_{}.json",
        result.stage.display_name().to_lowercase(),
        timestamp,
        session_id
    );
    let telemetry_path = evidence_dir.join(filename);

    let telemetry = serde_json::json!({
        "schemaVersion": 1,
        "command": format!("guardrail.{}", result.stage.display_name().to_lowercase()),
        "specId": result.spec_id,
        "sessionId": session_id.to_string(),
        "timestamp": timestamp.to_string(),
        "success": result.success,
        "stage": result.stage.display_name(),
        "checks": result.checks_run.iter().map(|check| {
            serde_json::json!({
                "name": check.name,
                "status": format!("{:?}", check.status).to_lowercase(),
                "message": check.message,
            })
        }).collect::<Vec<_>>(),
        "warnings": result.warnings,
        "errors": result.errors,
        "artifacts": [
            { "path": telemetry_path.to_string_lossy() }
        ]
    });

    let mut file = std::fs::File::create(&telemetry_path)?;
    file.write_all(serde_json::to_string_pretty(&telemetry)?.as_bytes())?;

    Ok(telemetry_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spec_id_format_validation() {
        let check = validate_spec_id(Path::new("."), "SPEC-KIT-900");
        // Format should pass even if directory doesn't exist (that's a separate check)
        assert!(check.status == CheckStatus::Passed || check.status == CheckStatus::Failed);
    }

    #[test]
    fn test_invalid_spec_id_format() {
        let check = validate_spec_id(Path::new("."), "INVALID");
        assert_eq!(check.status, CheckStatus::Warning);
    }
}
