//! Embedded template defaults for spec-kit.
//!
//! Templates are compiled into the binary using `include_str!()`.
//! This ensures the TUI works without any external file dependencies.
//!
//! ## Resolution Order (SPEC-KIT-964: Hermetic Isolation)
//!
//! 1. **Project-local**: `./templates/{name}-template.md`
//! 2. **Embedded**: Compiled into binary (always available)
//!
//! **Note**: Global user config (`~/.config/code/templates/`) is intentionally
//! NOT checked to ensure hermetic agent isolation. Spawned agents must not
//! depend on user-specific global configurations.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use codex_tui::templates::{resolve_template, resolve_template_source, TemplateSource};
//!
//! // Get template content (automatically resolves through priority order)
//! let plan = resolve_template("plan");
//!
//! // Check where a template would be resolved from
//! match resolve_template_source("plan") {
//!     TemplateSource::ProjectLocal(path) => println!("Using project: {}", path.display()),
//!     TemplateSource::Embedded => println!("Using embedded default"),
//! }
//! ```

use std::fs;
use std::path::PathBuf;

/// Embedded template constants.
///
/// These are compiled into the binary at build time using `include_str!()`.
/// The relative paths go from `codex-rs/tui/src/templates/` up to repo root.
pub mod embedded {
    /// Plan stage template - work breakdown structure
    pub const PLAN: &str = include_str!("../../../../templates/plan-template.md");

    /// Tasks stage template - task decomposition
    pub const TASKS: &str = include_str!("../../../../templates/tasks-template.md");

    /// Implement stage template - code generation guidance
    pub const IMPLEMENT: &str = include_str!("../../../../templates/implement-template.md");

    /// Validate stage template - test strategy
    pub const VALIDATE: &str = include_str!("../../../../templates/validate-template.md");

    /// Audit stage template - compliance checking
    pub const AUDIT: &str = include_str!("../../../../templates/audit-template.md");

    /// Unlock stage template - ship decision
    pub const UNLOCK: &str = include_str!("../../../../templates/unlock-template.md");

    /// Clarify quality gate template - ambiguity detection
    pub const CLARIFY: &str = include_str!("../../../../templates/clarify-template.md");

    /// Analyze quality gate template - consistency checking
    pub const ANALYZE: &str = include_str!("../../../../templates/analyze-template.md");

    /// Checklist quality gate template - quality scoring
    pub const CHECKLIST: &str = include_str!("../../../../templates/checklist-template.md");

    /// PRD template - product requirements document
    pub const PRD: &str = include_str!("../../../../templates/PRD-template.md");

    /// Spec template - technical specification
    pub const SPEC: &str = include_str!("../../../../templates/spec-template.md");

    // SPEC-KIT-961 Phase 5: Instruction file templates for hermetic agent isolation
    /// CLAUDE.md instruction file template
    pub const CLAUDE: &str = include_str!("../../../../templates/CLAUDE-template.md");

    /// AGENTS.md instruction file template
    pub const AGENTS: &str = include_str!("../../../../templates/AGENTS-template.md");

    /// GEMINI.md instruction file template
    pub const GEMINI: &str = include_str!("../../../../templates/GEMINI-template.md");
}

/// Get embedded template by name.
///
/// Returns `None` if template name is not recognized.
///
/// # Arguments
///
/// * `name` - Template name (case-insensitive). Valid names:
///   - Stage templates: `plan`, `tasks`, `implement`, `validate`, `audit`, `unlock`
///   - Quality gate templates: `clarify`, `analyze`, `checklist`
///   - Document templates: `prd`, `spec`
///
/// # Examples
///
/// ```rust,ignore
/// let plan = get_embedded("plan").expect("plan template exists");
/// let prd = get_embedded("PRD").expect("PRD template exists");
/// ```
pub fn get_embedded(name: &str) -> Option<&'static str> {
    match name.to_lowercase().as_str() {
        "plan" => Some(embedded::PLAN),
        "tasks" => Some(embedded::TASKS),
        "implement" => Some(embedded::IMPLEMENT),
        "validate" => Some(embedded::VALIDATE),
        "audit" => Some(embedded::AUDIT),
        "unlock" => Some(embedded::UNLOCK),
        "clarify" => Some(embedded::CLARIFY),
        "analyze" => Some(embedded::ANALYZE),
        "checklist" => Some(embedded::CHECKLIST),
        "prd" => Some(embedded::PRD),
        "spec" => Some(embedded::SPEC),
        // SPEC-KIT-961 Phase 5: Instruction file templates
        "claude" => Some(embedded::CLAUDE),
        "agents" => Some(embedded::AGENTS),
        "gemini" => Some(embedded::GEMINI),
        _ => None,
    }
}

/// List all available template names.
///
/// Returns a static slice of all recognized template identifiers.
pub fn template_names() -> &'static [&'static str] {
    &[
        "plan",
        "tasks",
        "implement",
        "validate",
        "audit",
        "unlock",
        "clarify",
        "analyze",
        "checklist",
        "prd",
        "spec",
        // SPEC-KIT-961 Phase 5: Instruction file templates
        "claude",
        "agents",
        "gemini",
    ]
}

/// Source location for a resolved template.
///
/// SPEC-KIT-964: Only project-local and embedded sources are supported.
/// Global user config is intentionally excluded for hermetic isolation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemplateSource {
    /// Template found in project-local `./templates/` directory
    ProjectLocal(PathBuf),
    /// Template using embedded default compiled into binary
    Embedded,
}

impl std::fmt::Display for TemplateSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateSource::ProjectLocal(p) => write!(f, "{}", p.display()),
            TemplateSource::Embedded => write!(f, "[embedded]"),
        }
    }
}

/// Resolve a template by name, checking locations in priority order.
///
/// Resolution order (SPEC-KIT-964: Hermetic Isolation):
/// 1. **Project-local**: `./templates/{name}-template.md`
/// 2. **Embedded**: Compiled-in default (always succeeds for known templates)
///
/// **Note**: Global user config is NOT checked to ensure hermetic agent isolation.
///
/// # Arguments
///
/// * `name` - Template name (case-insensitive)
///
/// # Returns
///
/// Template content as a String. Returns empty string if template name is unknown.
///
/// # Examples
///
/// ```rust,ignore
/// // Always works - falls back to embedded
/// let plan = resolve_template("plan");
/// assert!(!plan.is_empty());
///
/// // Unknown template returns empty
/// let unknown = resolve_template("nonexistent");
/// assert!(unknown.is_empty());
/// ```
pub fn resolve_template(name: &str) -> String {
    let normalized = name.to_lowercase();
    let filename = format!("{}-template.md", normalized);

    // 1. Project-local
    let local_path = PathBuf::from("templates").join(&filename);
    if local_path.exists() {
        if let Ok(content) = fs::read_to_string(&local_path) {
            tracing::debug!(
                template = %name,
                source = %local_path.display(),
                "Template resolved from project-local"
            );
            return content;
        }
    }

    // 2. Embedded fallback (SPEC-KIT-964: skip global user config)
    tracing::debug!(template = %name, "Template resolved from embedded defaults");
    get_embedded(&normalized)
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            tracing::warn!(template = %name, "Unknown template, returning empty");
            String::new()
        })
}

/// Get the source location where a template would be resolved from.
///
/// Useful for diagnostics and the `/speckit.template-status` command.
///
/// SPEC-KIT-964: Only returns ProjectLocal or Embedded (no global user config).
///
/// # Arguments
///
/// * `name` - Template name (case-insensitive)
///
/// # Returns
///
/// [`TemplateSource`] indicating where the template would be loaded from.
pub fn resolve_template_source(name: &str) -> TemplateSource {
    let normalized = name.to_lowercase();
    let filename = format!("{}-template.md", normalized);

    // 1. Project-local
    let local_path = PathBuf::from("templates").join(&filename);
    if local_path.exists() {
        return TemplateSource::ProjectLocal(local_path);
    }

    // 2. Embedded (default) - SPEC-KIT-964: skip global user config
    TemplateSource::Embedded
}

/// Template status for diagnostics.
#[derive(Debug, Clone)]
pub struct TemplateStatus {
    /// Template name
    pub name: String,
    /// Where this template resolves from
    pub source: TemplateSource,
    /// Whether the template content is available
    pub available: bool,
}

/// Get status of all templates.
///
/// Used by `/speckit.template-status` command.
pub fn all_template_status() -> Vec<TemplateStatus> {
    template_names()
        .iter()
        .map(|name| {
            let source = resolve_template_source(name);
            let available = get_embedded(name).is_some()
                || matches!(&source, TemplateSource::ProjectLocal(p) if p.exists());
            TemplateStatus {
                name: (*name).to_string(),
                source,
                available,
            }
        })
        .collect()
}

/// Install result from `/speckit.install-templates`.
#[derive(Debug)]
pub struct InstallResult {
    /// Templates that were installed
    pub installed: Vec<String>,
    /// Templates that were skipped (already exist)
    pub skipped: Vec<String>,
    /// Path where templates were installed
    pub path: PathBuf,
}

/// Copy embedded templates to project-local directory for customization.
///
/// Creates `./templates/` with all embedded templates.
///
/// SPEC-KIT-964: Templates are installed to project-local directory only,
/// not global user config, to ensure hermetic agent isolation.
///
/// # Arguments
///
/// * `force` - If true, overwrite existing templates
///
/// # Returns
///
/// [`InstallResult`] with details of what was installed/skipped.
///
/// # Errors
///
/// Returns error if templates directory cannot be created.
pub fn install_templates(force: bool) -> anyhow::Result<InstallResult> {
    // SPEC-KIT-964: Install to project-local ./templates/ (not global)
    let templates_dir = PathBuf::from("templates");
    fs::create_dir_all(&templates_dir)?;

    let mut installed = Vec::new();
    let mut skipped = Vec::new();

    for name in template_names() {
        let filename = format!("{}-template.md", name);
        let dest = templates_dir.join(&filename);

        if dest.exists() && !force {
            skipped.push(filename);
            continue;
        }

        if let Some(content) = get_embedded(name) {
            fs::write(&dest, content)?;
            installed.push(filename);
        }
    }

    Ok(InstallResult {
        installed,
        skipped,
        path: templates_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_templates_embedded() {
        for name in template_names() {
            assert!(
                get_embedded(name).is_some(),
                "Missing embedded template: {}",
                name
            );
        }
    }

    #[test]
    fn test_template_content_not_empty() {
        for name in template_names() {
            let content = get_embedded(name).unwrap();
            assert!(!content.is_empty(), "Empty template: {}", name);
            assert!(
                content.contains('#'),
                "Template missing markdown header: {}",
                name
            );
        }
    }

    #[test]
    fn test_case_insensitive_lookup() {
        assert!(get_embedded("PLAN").is_some());
        assert!(get_embedded("Plan").is_some());
        assert!(get_embedded("plan").is_some());
        assert!(get_embedded("PRD").is_some());
        assert!(get_embedded("prd").is_some());
    }

    #[test]
    fn test_unknown_template_returns_none() {
        assert!(get_embedded("nonexistent").is_none());
        assert!(get_embedded("").is_none());
    }

    #[test]
    fn test_resolve_falls_back_to_embedded() {
        // In test environment without local templates, should get embedded
        let content = resolve_template("plan");
        assert!(!content.is_empty());
        assert!(content.contains("Plan"));
    }

    #[test]
    fn test_resolve_unknown_returns_empty() {
        let content = resolve_template("nonexistent");
        assert!(content.is_empty());
    }

    #[test]
    fn test_template_source_display() {
        let embedded = TemplateSource::Embedded;
        assert_eq!(format!("{}", embedded), "[embedded]");

        let local = TemplateSource::ProjectLocal(PathBuf::from("templates/plan-template.md"));
        assert!(format!("{}", local).contains("plan-template.md"));
    }

    #[test]
    fn test_all_template_status() {
        let status = all_template_status();
        // 11 original + 3 instruction file templates = 14
        assert_eq!(status.len(), 14);
        for s in &status {
            assert!(s.available, "Template {} should be available", s.name);
        }
    }

    #[test]
    fn test_instruction_file_templates() {
        // SPEC-KIT-961: Verify instruction file templates exist
        assert!(get_embedded("claude").is_some(), "claude template missing");
        assert!(get_embedded("agents").is_some(), "agents template missing");
        assert!(get_embedded("gemini").is_some(), "gemini template missing");
    }
}
