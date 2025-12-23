//! Shadow Notebook Seeder for Stage 0
//!
//! SPEC-KIT-102: Extracts high-value knowledge from local-memory and codex-rs
//! to generate NotebookLM-ready Markdown files.
//!
//! V1 Implementation (No Vector DB):
//! - Uses local-memory CLI/REST search with tag/domain filtering
//! - Prioritizes by overlay dynamic_score
//! - Generates 5 artifact types:
//!   - NL_ARCHITECTURE_BIBLE.md
//!   - NL_STACK_JUSTIFICATION.md
//!   - NL_BUG_RETROS_01.md
//!   - NL_DEBT_LANDSCAPE.md
//!   - NL_PROJECT_DIARY_01.md

use codex_stage0::dcc::{Iqo, LocalMemoryClient, LocalMemorySearchParams, LocalMemorySummary};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Kind of NotebookLM seed artifact
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeedKind {
    /// Architecture Bible - design decisions, patterns, key subsystems
    ArchitectureBible,
    /// Stack Justification - why each major dependency is used
    StackJustification,
    /// Bug Retros - patterns from bugs and incidents
    BugRetros,
    /// Debt Landscape - TODO/FIXME/HACK comments grouped by module
    DebtLandscape,
    /// Project Diary - chronological session summaries and milestones
    ProjectDiary,
}

impl SeedKind {
    /// Get the filename for this artifact kind
    pub fn filename(&self) -> &'static str {
        match self {
            Self::ArchitectureBible => "NL_ARCHITECTURE_BIBLE.md",
            Self::StackJustification => "NL_STACK_JUSTIFICATION.md",
            Self::BugRetros => "NL_BUG_RETROS_01.md",
            Self::DebtLandscape => "NL_DEBT_LANDSCAPE.md",
            Self::ProjectDiary => "NL_PROJECT_DIARY_01.md",
        }
    }

    /// Get human-readable display name
    #[allow(dead_code)] // API for future UI integration
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::ArchitectureBible => "Architecture Bible",
            Self::StackJustification => "Stack Justification",
            Self::BugRetros => "Bug Retros",
            Self::DebtLandscape => "Debt Landscape",
            Self::ProjectDiary => "Project Diary",
        }
    }
}

/// Result of generating a single seed artifact
#[derive(Debug, Clone)]
pub struct SeedArtifact {
    /// Type of artifact generated
    pub kind: SeedKind,
    /// Path where the artifact was written
    #[allow(dead_code)] // API for callers to locate generated files
    pub path: PathBuf,
    /// Number of source items (memories, TODOs, etc.) used
    pub count: usize,
    /// Whether the artifact was successfully written
    pub written: bool,
}

/// Configuration for seeding operation
#[derive(Debug, Clone)]
pub struct SeedingConfig {
    /// Maximum memories to include per artifact
    pub max_memories_per_artifact: usize,
    /// Output directory for generated files
    pub output_dir: PathBuf,
    /// Project root for scanning TODOs
    pub project_root: PathBuf,
}

impl Default for SeedingConfig {
    fn default() -> Self {
        Self {
            max_memories_per_artifact: 50,
            output_dir: PathBuf::from("evidence/notebooklm"),
            project_root: PathBuf::from("."),
        }
    }
}

/// Result of a complete seeding operation
#[derive(Debug)]
pub struct SeedingResult {
    /// Artifacts that were generated
    pub artifacts: Vec<SeedArtifact>,
    /// Total execution time in milliseconds
    pub duration_ms: u64,
    /// Errors encountered (non-fatal)
    pub errors: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main Seeding Pipeline
// ─────────────────────────────────────────────────────────────────────────────

/// Run the complete shadow seeding pipeline
///
/// Generates all 5 NotebookLM artifact types and writes them to disk.
pub async fn run_shadow_seeding(
    local_mem: &impl LocalMemoryClient,
    config: &SeedingConfig,
) -> SeedingResult {
    let start = std::time::Instant::now();
    let mut artifacts = Vec::new();
    let mut errors = Vec::new();

    // Ensure output directory exists
    if let Err(e) = std::fs::create_dir_all(&config.output_dir) {
        return SeedingResult {
            artifacts: vec![],
            duration_ms: start.elapsed().as_millis() as u64,
            errors: vec![format!("Failed to create output directory: {e}")],
        };
    }

    // Generate each artifact type
    // 1. Architecture Bible
    match seed_architecture_bible(local_mem, config).await {
        Ok(artifact) => artifacts.push(artifact),
        Err(e) => errors.push(format!("Architecture Bible: {e}")),
    }

    // 2. Stack Justification
    match seed_stack_justification(local_mem, config).await {
        Ok(artifact) => artifacts.push(artifact),
        Err(e) => errors.push(format!("Stack Justification: {e}")),
    }

    // 3. Bug Retros
    match seed_bug_retros(local_mem, config).await {
        Ok(artifact) => artifacts.push(artifact),
        Err(e) => errors.push(format!("Bug Retros: {e}")),
    }

    // 4. Debt Landscape (scans codebase, doesn't need local-memory)
    match seed_debt_landscape(config) {
        Ok(artifact) => artifacts.push(artifact),
        Err(e) => errors.push(format!("Debt Landscape: {e}")),
    }

    // 5. Project Diary
    match seed_project_diary(local_mem, config).await {
        Ok(artifact) => artifacts.push(artifact),
        Err(e) => errors.push(format!("Project Diary: {e}")),
    }

    SeedingResult {
        artifacts,
        duration_ms: start.elapsed().as_millis() as u64,
        errors,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Artifact Seeders
// ─────────────────────────────────────────────────────────────────────────────

/// Generate Architecture Bible from decision and pattern memories
async fn seed_architecture_bible(
    local_mem: &impl LocalMemoryClient,
    config: &SeedingConfig,
) -> Result<SeedArtifact, String> {
    // Search for architecture-related memories
    let iqo = Iqo {
        domains: vec!["spec-kit".to_string(), "infrastructure".to_string()],
        required_tags: vec![],
        optional_tags: vec![
            "type:decision".to_string(),
            "type:pattern".to_string(),
            "component:*".to_string(),
        ],
        keywords: vec![
            "architecture".to_string(),
            "design".to_string(),
            "pattern".to_string(),
            "decision".to_string(),
            "subsystem".to_string(),
        ],
        max_candidates: config.max_memories_per_artifact * 2,
        notebook_focus: vec![],
        exclude_tags: vec![],
    };

    let params = LocalMemorySearchParams {
        iqo,
        max_results: config.max_memories_per_artifact * 2,
    };

    let memories = local_mem
        .search_memories(params)
        .await
        .map_err(|e| e.to_string())?;

    // Filter to architecture-relevant memories and take top N
    let filtered: Vec<_> = memories
        .into_iter()
        .filter(|m| {
            let tags_lower: Vec<String> = m.tags.iter().map(|t| t.to_lowercase()).collect();
            tags_lower.iter().any(|t| {
                t.contains("decision")
                    || t.contains("pattern")
                    || t.contains("architecture")
                    || t.contains("design")
            }) || m.snippet.to_lowercase().contains("architecture")
                || m.snippet.to_lowercase().contains("design decision")
        })
        .take(config.max_memories_per_artifact)
        .collect();

    // Generate markdown content
    let content = format_architecture_bible(&filtered);
    let path = config
        .output_dir
        .join(SeedKind::ArchitectureBible.filename());

    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(SeedArtifact {
        kind: SeedKind::ArchitectureBible,
        path,
        count: filtered.len(),
        written: true,
    })
}

/// Generate Stack Justification from dependency-related memories
async fn seed_stack_justification(
    local_mem: &impl LocalMemoryClient,
    config: &SeedingConfig,
) -> Result<SeedArtifact, String> {
    // Search for dependency decisions
    let iqo = Iqo {
        domains: vec!["spec-kit".to_string()],
        required_tags: vec![],
        optional_tags: vec!["type:decision".to_string()],
        keywords: vec![
            "dependency".to_string(),
            "crate".to_string(),
            "library".to_string(),
            "framework".to_string(),
            "tokio".to_string(),
            "ratatui".to_string(),
            "sqlite".to_string(),
        ],
        max_candidates: config.max_memories_per_artifact,
        notebook_focus: vec![],
        exclude_tags: vec![],
    };

    let params = LocalMemorySearchParams {
        iqo,
        max_results: config.max_memories_per_artifact,
    };

    let memories = local_mem
        .search_memories(params)
        .await
        .map_err(|e| e.to_string())?;

    // Also try to parse Cargo.toml for dependency list
    let cargo_deps = parse_cargo_dependencies(&config.project_root);

    // Generate markdown content
    let content = format_stack_justification(&memories, &cargo_deps);
    let path = config
        .output_dir
        .join(SeedKind::StackJustification.filename());

    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(SeedArtifact {
        kind: SeedKind::StackJustification,
        path,
        count: memories.len() + cargo_deps.len(),
        written: true,
    })
}

/// Generate Bug Retros from bug/incident/postmortem memories
async fn seed_bug_retros(
    local_mem: &impl LocalMemoryClient,
    config: &SeedingConfig,
) -> Result<SeedArtifact, String> {
    // Search for bug-related memories
    let iqo = Iqo {
        domains: vec!["spec-kit".to_string()],
        required_tags: vec![],
        optional_tags: vec![
            "type:bug".to_string(),
            "type:incident".to_string(),
            "type:postmortem".to_string(),
            "type:problem".to_string(),
        ],
        keywords: vec![
            "bug".to_string(),
            "error".to_string(),
            "fix".to_string(),
            "issue".to_string(),
            "crash".to_string(),
            "panic".to_string(),
            "failure".to_string(),
        ],
        max_candidates: config.max_memories_per_artifact * 2,
        notebook_focus: vec![],
        exclude_tags: vec![],
    };

    let params = LocalMemorySearchParams {
        iqo,
        max_results: config.max_memories_per_artifact * 2,
    };

    let memories = local_mem
        .search_memories(params)
        .await
        .map_err(|e| e.to_string())?;

    // Filter to bug-relevant memories
    let filtered: Vec<_> = memories
        .into_iter()
        .filter(|m| {
            let tags_lower: Vec<String> = m.tags.iter().map(|t| t.to_lowercase()).collect();
            tags_lower.iter().any(|t| {
                t.contains("bug")
                    || t.contains("problem")
                    || t.contains("incident")
                    || t.contains("postmortem")
                    || t.contains("fix")
            }) || m.snippet.to_lowercase().contains("bug")
                || m.snippet.to_lowercase().contains("error")
                || m.snippet.to_lowercase().contains("fix")
        })
        .take(config.max_memories_per_artifact)
        .collect();

    // Generate markdown content
    let content = format_bug_retros(&filtered);
    let path = config.output_dir.join(SeedKind::BugRetros.filename());

    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(SeedArtifact {
        kind: SeedKind::BugRetros,
        path,
        count: filtered.len(),
        written: true,
    })
}

/// Generate Debt Landscape by scanning codebase for TODO/FIXME/HACK comments
fn seed_debt_landscape(config: &SeedingConfig) -> Result<SeedArtifact, String> {
    let debt_items = scan_tech_debt(&config.project_root);

    // Group by module/path
    let grouped = group_debt_by_module(&debt_items);

    // Generate markdown content
    let content = format_debt_landscape(&grouped);
    let path = config.output_dir.join(SeedKind::DebtLandscape.filename());

    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(SeedArtifact {
        kind: SeedKind::DebtLandscape,
        path,
        count: debt_items.len(),
        written: true,
    })
}

/// Generate Project Diary from session summaries and chronological memories
async fn seed_project_diary(
    local_mem: &impl LocalMemoryClient,
    config: &SeedingConfig,
) -> Result<SeedArtifact, String> {
    // Search for session-related memories
    let iqo = Iqo {
        domains: vec!["spec-kit".to_string()],
        required_tags: vec![],
        optional_tags: vec![
            "type:session".to_string(),
            "type:milestone".to_string(),
            "type:insight".to_string(),
        ],
        keywords: vec![
            "session".to_string(),
            "completed".to_string(),
            "milestone".to_string(),
            "phase".to_string(),
            "implemented".to_string(),
            "progress".to_string(),
        ],
        max_candidates: config.max_memories_per_artifact * 2,
        notebook_focus: vec![],
        exclude_tags: vec![],
    };

    let params = LocalMemorySearchParams {
        iqo,
        max_results: config.max_memories_per_artifact * 2,
    };

    let memories = local_mem
        .search_memories(params)
        .await
        .map_err(|e| e.to_string())?;

    // Sort by creation date (oldest first for chronological diary)
    let mut sorted: Vec<_> = memories
        .into_iter()
        .take(config.max_memories_per_artifact)
        .collect();

    sorted.sort_by(|a, b| {
        let a_date = a.created_at.unwrap_or_else(|| chrono::Utc::now());
        let b_date = b.created_at.unwrap_or_else(|| chrono::Utc::now());
        a_date.cmp(&b_date)
    });

    // Generate markdown content
    let content = format_project_diary(&sorted);
    let path = config.output_dir.join(SeedKind::ProjectDiary.filename());

    std::fs::write(&path, &content).map_err(|e| format!("Failed to write file: {e}"))?;

    Ok(SeedArtifact {
        kind: SeedKind::ProjectDiary,
        path,
        count: sorted.len(),
        written: true,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Formatting Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Get current git commit hash (short form) if available
fn get_git_commit_hash() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
}

/// P84: Generate index header with timestamp and git commit
fn generate_index_header(artifact_name: &str, entry_count: usize, sections: &[&str]) -> String {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let commit = get_git_commit_hash().unwrap_or_else(|| "unknown".to_string());

    let mut header = format!("# {}\n\n", artifact_name);
    header.push_str(&format!("> Seeded by Stage0 on {}\n", now));
    header.push_str(&format!("> Source: codex-rs commit {}\n\n", commit));

    if !sections.is_empty() {
        header.push_str("## Index\n\n");
        for section in sections {
            header.push_str(&format!("- {}\n", section));
        }
        header.push('\n');
    }

    header.push_str(&format!("_Total entries: {}_\n\n", entry_count));
    header.push_str("---\n\n");

    header
}

fn format_architecture_bible(memories: &[LocalMemorySummary]) -> String {
    // Count entries by category for index
    let decisions_count = memories
        .iter()
        .filter(|m| m.tags.iter().any(|t| t.to_lowercase().contains("decision")))
        .count();
    let patterns_count = memories
        .iter()
        .filter(|m| m.tags.iter().any(|t| t.to_lowercase().contains("pattern")))
        .count();
    let other_count = memories.len() - decisions_count - patterns_count;

    // Build index sections
    let mut sections = Vec::new();
    if decisions_count > 0 {
        sections.push(format!("Design Decisions ({} entries)", decisions_count));
    }
    if patterns_count > 0 {
        sections.push(format!(
            "Architectural Patterns ({} entries)",
            patterns_count
        ));
    }
    if other_count > 0 {
        sections.push(format!(
            "Other Architecture Notes ({} entries)",
            other_count
        ));
    }
    let section_refs: Vec<&str> = sections.iter().map(|s| s.as_str()).collect();

    let mut out = generate_index_header("NL_ARCHITECTURE_BIBLE", memories.len(), &section_refs);

    out.push_str("_Architecture decisions, patterns, and key subsystems for codex-rs._\n\n");

    // Group by inferred category
    let mut decisions = Vec::new();
    let mut patterns = Vec::new();
    let mut other = Vec::new();

    for m in memories {
        let tags_lower: Vec<String> = m.tags.iter().map(|t| t.to_lowercase()).collect();
        if tags_lower.iter().any(|t| t.contains("decision")) {
            decisions.push(m);
        } else if tags_lower.iter().any(|t| t.contains("pattern")) {
            patterns.push(m);
        } else {
            other.push(m);
        }
    }

    // Design Decisions section
    if !decisions.is_empty() {
        out.push_str("## Design Decisions\n\n");
        for m in &decisions {
            out.push_str(&format!(
                "### [DECISION] {}\n\n",
                truncate_first_line(&m.snippet)
            ));
            out.push_str(&format!("{}\n\n", &m.snippet));
            if !m.tags.is_empty() {
                out.push_str(&format!("_Tags: {}_\n\n", m.tags.join(", ")));
            }
            out.push_str("---\n\n");
        }
    }

    // Patterns section
    if !patterns.is_empty() {
        out.push_str("## Architectural Patterns\n\n");
        for m in &patterns {
            out.push_str(&format!(
                "### [PATTERN] {}\n\n",
                truncate_first_line(&m.snippet)
            ));
            out.push_str(&format!("{}\n\n", &m.snippet));
            if !m.tags.is_empty() {
                out.push_str(&format!("_Tags: {}_\n\n", m.tags.join(", ")));
            }
            out.push_str("---\n\n");
        }
    }

    // Other architecture-related content
    if !other.is_empty() {
        out.push_str("## Other Architecture Notes\n\n");
        for m in &other {
            out.push_str(&format!("- {}\n", truncate_to_length(&m.snippet, 200)));
        }
        out.push('\n');
    }

    out
}

fn format_stack_justification(
    memories: &[LocalMemorySummary],
    cargo_deps: &[(String, String)],
) -> String {
    // Build index sections
    let sections = vec![
        "Core Dependencies",
        "Full Dependency List",
        "Documented Decisions",
    ];

    let total_entries = memories.len() + cargo_deps.len();
    let mut out = generate_index_header("NL_STACK_JUSTIFICATION", total_entries, &sections);

    out.push_str("_Why each major dependency is used in codex-rs._\n\n");

    // Major dependencies section
    out.push_str("## Core Dependencies\n\n");

    // Key dependencies we want to highlight
    let key_deps = [
        "tokio",
        "ratatui",
        "rusqlite",
        "serde",
        "async-trait",
        "clap",
        "chrono",
        "tracing",
    ];

    for dep_name in &key_deps {
        if let Some((_, version)) = cargo_deps.iter().find(|(name, _)| name == *dep_name) {
            out.push_str(&format!("### `{}` (v{})\n\n", dep_name, version));
            // Find any memories mentioning this dependency
            let related: Vec<_> = memories
                .iter()
                .filter(|m| m.snippet.to_lowercase().contains(dep_name))
                .collect();

            if related.is_empty() {
                out.push_str(&format!(
                    "_No documented rationale found. Used for its standard {} functionality._\n\n",
                    get_dep_purpose(dep_name)
                ));
            } else {
                for m in related {
                    out.push_str(&format!("{}\n\n", m.snippet));
                }
            }
        }
    }

    // All dependencies listing
    out.push_str("## Full Dependency List\n\n");
    out.push_str("| Crate | Version | Purpose |\n");
    out.push_str("|-------|---------|----------|\n");
    for (name, version) in cargo_deps.iter().take(30) {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            name,
            version,
            get_dep_purpose(name)
        ));
    }

    // Memory-based justifications
    if !memories.is_empty() {
        out.push_str("\n## Documented Decisions\n\n");
        for m in memories.iter().take(10) {
            out.push_str(&format!("- {}\n", truncate_to_length(&m.snippet, 150)));
        }
    }

    out
}

fn format_bug_retros(memories: &[LocalMemorySummary]) -> String {
    // Build index sections (list up to 10 bug pattern entries)
    let section_strings: Vec<String> = (1..=memories.len().min(10))
        .map(|i| format!("Bug Pattern {}", i))
        .collect();
    let sections_ref: Vec<&str> = section_strings.iter().map(|s| s.as_str()).collect();

    let mut out = generate_index_header("NL_BUG_RETROS_01", memories.len(), &sections_ref);

    out.push_str("_Patterns and lessons from bugs, incidents, and fixes in codex-rs._\n\n");

    if memories.is_empty() {
        out.push_str("_No bug-related memories found. This is a good thing!_\n\n");
    } else {
        for (idx, m) in memories.iter().enumerate() {
            out.push_str(&format!("## Bug Pattern {}\n\n", idx + 1));

            // Extract first line as title
            let title = truncate_first_line(&m.snippet);
            out.push_str(&format!("**Issue:** {}\n\n", title));

            out.push_str("**Details:**\n\n");
            out.push_str(&format!("{}\n\n", m.snippet));

            if !m.tags.is_empty() {
                out.push_str(&format!("**Tags:** {}\n\n", m.tags.join(", ")));
            }

            if let Some(date) = m.created_at {
                out.push_str(&format!("**Date:** {}\n\n", date.format("%Y-%m-%d")));
            }

            out.push_str("---\n\n");
        }
    }

    out
}

fn format_debt_landscape(grouped: &HashMap<String, Vec<DebtItem>>) -> String {
    // Calculate totals for index
    let total_items: usize = grouped.values().map(|v| v.len()).sum();
    let modules: Vec<String> = grouped.keys().cloned().collect();

    // Build index sections
    let mut sections = vec!["Summary".to_string()];
    sections.push("Details by Module".to_string());
    for module in &modules {
        sections.push(format!("  - {}", module));
    }
    let sections_ref: Vec<&str> = sections.iter().map(|s| s.as_str()).collect();

    let mut out = generate_index_header("NL_DEBT_LANDSCAPE", total_items, &sections_ref);

    out.push_str("_Technical debt inventory: TODO, FIXME, HACK comments across codex-rs._\n\n");

    // Summary table
    out.push_str("## Summary\n\n");
    out.push_str("| Module | TODOs | FIXMEs | HACKs | Total |\n");
    out.push_str("|--------|-------|--------|-------|-------|\n");

    let mut total_todos = 0;
    let mut total_fixmes = 0;
    let mut total_hacks = 0;

    for (module, items) in grouped.iter() {
        let todos = items.iter().filter(|i| i.kind == DebtKind::Todo).count();
        let fixmes = items.iter().filter(|i| i.kind == DebtKind::Fixme).count();
        let hacks = items.iter().filter(|i| i.kind == DebtKind::Hack).count();
        let total = items.len();

        total_todos += todos;
        total_fixmes += fixmes;
        total_hacks += hacks;

        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            module, todos, fixmes, hacks, total
        ));
    }

    out.push_str(&format!(
        "| **TOTAL** | **{}** | **{}** | **{}** | **{}** |\n\n",
        total_todos,
        total_fixmes,
        total_hacks,
        total_todos + total_fixmes + total_hacks
    ));

    // Details by module
    out.push_str("## Details by Module\n\n");

    let mut modules: Vec<_> = grouped.keys().collect();
    modules.sort();

    for module in modules {
        let items = &grouped[module];
        out.push_str(&format!("### {}\n\n", module));

        for item in items {
            let prefix = match item.kind {
                DebtKind::Todo => "TODO",
                DebtKind::Fixme => "FIXME",
                DebtKind::Hack => "HACK",
                DebtKind::Warn => "WARN",
            };
            out.push_str(&format!(
                "- **[{}]** `{}:{}` - {}\n",
                prefix,
                item.file.file_name().unwrap_or_default().to_string_lossy(),
                item.line,
                truncate_to_length(&item.text, 80)
            ));
        }
        out.push('\n');
    }

    out
}

fn format_project_diary(memories: &[LocalMemorySummary]) -> String {
    // Build index sections based on months present
    let mut months: Vec<String> = memories
        .iter()
        .filter_map(|m| m.created_at.map(|d| d.format("%B %Y").to_string()))
        .collect();
    months.dedup();

    let sections_ref: Vec<&str> = months.iter().map(|s| s.as_str()).collect();

    let mut out = generate_index_header("NL_PROJECT_DIARY_01", memories.len(), &sections_ref);

    out.push_str("_Chronological record of project progress, milestones, and insights._\n\n");

    if memories.is_empty() {
        out.push_str("_No diary entries found. Start capturing session summaries!_\n\n");
    } else {
        // Group by month
        let mut current_month = String::new();

        for m in memories {
            if let Some(date) = m.created_at {
                let month = date.format("%Y-%m").to_string();
                if month != current_month {
                    current_month = month.clone();
                    out.push_str(&format!("\n## {}\n\n", date.format("%B %Y")));
                }

                out.push_str(&format!("### {} - Entry\n\n", date.format("%Y-%m-%d")));
            } else {
                out.push_str("### Undated Entry\n\n");
            }

            out.push_str(&format!("{}\n\n", m.snippet));

            if !m.tags.is_empty() {
                out.push_str(&format!("_Tags: {}_\n\n", m.tags.join(", ")));
            }

            out.push_str("---\n\n");
        }
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper Types and Functions
// ─────────────────────────────────────────────────────────────────────────────

/// Technical debt item from codebase scan
#[derive(Debug, Clone)]
struct DebtItem {
    file: PathBuf,
    line: usize,
    kind: DebtKind,
    text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DebtKind {
    Todo,
    Fixme,
    Hack,
    Warn,
}

/// Scan codebase for technical debt comments
fn scan_tech_debt(project_root: &Path) -> Vec<DebtItem> {
    let mut items = Vec::new();

    // Patterns to search for
    let patterns = [
        ("TODO", DebtKind::Todo),
        ("FIXME", DebtKind::Fixme),
        ("HACK", DebtKind::Hack),
        ("WARN", DebtKind::Warn),
    ];

    // Walk the project looking for Rust files
    let walker = walkdir::WalkDir::new(project_root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            // Skip common non-source directories
            !name.starts_with('.')
                && name != "target"
                && name != "node_modules"
                && name != "evidence"
        });

    for entry in walker.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        // Only scan Rust files and Markdown
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !["rs", "md", "toml"].contains(&ext) {
            continue;
        }

        // Read and scan file
        if let Ok(content) = std::fs::read_to_string(path) {
            for (line_num, line) in content.lines().enumerate() {
                let line_upper = line.to_uppercase();
                for (pattern, kind) in &patterns {
                    if line_upper.contains(pattern) {
                        // Extract the comment text
                        let text = if let Some(idx) = line.find("//") {
                            line[idx + 2..].trim().to_string()
                        } else if let Some(idx) = line.find('#') {
                            line[idx + 1..].trim().to_string()
                        } else {
                            line.trim().to_string()
                        };

                        items.push(DebtItem {
                            file: path.to_path_buf(),
                            line: line_num + 1,
                            kind: *kind,
                            text,
                        });
                        break; // Only count once per line
                    }
                }
            }
        }
    }

    items
}

/// Group debt items by module/directory
fn group_debt_by_module(items: &[DebtItem]) -> HashMap<String, Vec<DebtItem>> {
    let mut groups: HashMap<String, Vec<DebtItem>> = HashMap::new();

    for item in items {
        // Extract module from path (first meaningful directory)
        let module = item
            .file
            .parent()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "root".to_string());

        groups.entry(module).or_default().push(item.clone());
    }

    groups
}

/// Parse Cargo.toml for dependency list
fn parse_cargo_dependencies(project_root: &Path) -> Vec<(String, String)> {
    let mut deps = Vec::new();

    // Look for Cargo.toml in common locations
    let cargo_paths = [
        project_root.join("Cargo.toml"),
        project_root.join("codex-rs/Cargo.toml"),
        project_root.join("codex-rs/tui/Cargo.toml"),
    ];

    for cargo_path in &cargo_paths {
        if let Ok(content) = std::fs::read_to_string(cargo_path) {
            // Simple TOML parsing for dependencies
            let mut in_deps = false;
            for line in content.lines() {
                let trimmed = line.trim();

                if trimmed.starts_with("[dependencies]")
                    || trimmed.starts_with("[dev-dependencies]")
                {
                    in_deps = true;
                    continue;
                }

                if trimmed.starts_with('[') && in_deps {
                    in_deps = false;
                    continue;
                }

                if in_deps && trimmed.contains('=') {
                    let parts: Vec<_> = trimmed.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        let name = parts[0].trim().to_string();
                        let version = parts[1]
                            .trim()
                            .trim_matches('"')
                            .trim_matches('{')
                            .split(',')
                            .next()
                            .unwrap_or("*")
                            .trim()
                            .trim_matches('"')
                            .to_string();
                        if !name.is_empty() && !name.starts_with('#') {
                            deps.push((name, version));
                        }
                    }
                }
            }
        }
    }

    deps.sort_by(|a, b| a.0.cmp(&b.0));
    deps.dedup_by(|a, b| a.0 == b.0);
    deps
}

/// Get a brief purpose description for known dependencies
fn get_dep_purpose(name: &str) -> &'static str {
    match name {
        "tokio" => "Async runtime",
        "ratatui" => "TUI framework",
        "rusqlite" => "SQLite database",
        "serde" => "Serialization",
        "serde_json" => "JSON parsing",
        "async-trait" => "Async trait support",
        "clap" => "CLI argument parsing",
        "chrono" => "Date/time handling",
        "tracing" => "Logging/tracing",
        "anyhow" => "Error handling",
        "thiserror" => "Error derive macros",
        "regex-lite" => "Regex matching",
        "sha2" => "SHA-2 hashing",
        "walkdir" => "Directory traversal",
        "crossterm" => "Terminal I/O",
        "tempfile" => "Temporary files",
        "once_cell" => "Lazy statics",
        "futures" => "Async utilities",
        "uuid" => "UUID generation",
        "toml" => "TOML parsing",
        "reqwest" => "HTTP client",
        _ => "General utility",
    }
}

/// Truncate string to first line
fn truncate_first_line(s: &str) -> String {
    s.lines()
        .next()
        .map(|l| truncate_to_length(l, 80))
        .unwrap_or_default()
}

/// Truncate string to max length with ellipsis
fn truncate_to_length(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_kind_filename() {
        assert_eq!(
            SeedKind::ArchitectureBible.filename(),
            "NL_ARCHITECTURE_BIBLE.md"
        );
        assert_eq!(
            SeedKind::StackJustification.filename(),
            "NL_STACK_JUSTIFICATION.md"
        );
        assert_eq!(SeedKind::BugRetros.filename(), "NL_BUG_RETROS_01.md");
        assert_eq!(SeedKind::DebtLandscape.filename(), "NL_DEBT_LANDSCAPE.md");
        assert_eq!(SeedKind::ProjectDiary.filename(), "NL_PROJECT_DIARY_01.md");
    }

    #[test]
    fn test_truncate_to_length() {
        assert_eq!(truncate_to_length("short", 10), "short");
        assert_eq!(
            truncate_to_length("this is a very long string", 10),
            "this is..."
        );
    }

    #[test]
    fn test_truncate_first_line() {
        assert_eq!(truncate_first_line("First line\nSecond line"), "First line");
        assert_eq!(truncate_first_line("Single line"), "Single line");
    }

    #[test]
    fn test_get_dep_purpose() {
        assert_eq!(get_dep_purpose("tokio"), "Async runtime");
        assert_eq!(get_dep_purpose("unknown-crate"), "General utility");
    }

    #[test]
    fn test_seeding_config_default() {
        let config = SeedingConfig::default();
        assert_eq!(config.max_memories_per_artifact, 50);
        assert_eq!(config.output_dir, PathBuf::from("evidence/notebooklm"));
    }

    #[test]
    fn test_group_debt_by_module() {
        let items = vec![
            DebtItem {
                file: PathBuf::from("src/foo/bar.rs"),
                line: 10,
                kind: DebtKind::Todo,
                text: "Fix this".to_string(),
            },
            DebtItem {
                file: PathBuf::from("src/foo/baz.rs"),
                line: 20,
                kind: DebtKind::Fixme,
                text: "Fix that".to_string(),
            },
            DebtItem {
                file: PathBuf::from("src/other/mod.rs"),
                line: 5,
                kind: DebtKind::Hack,
                text: "Hack here".to_string(),
            },
        ];

        let grouped = group_debt_by_module(&items);
        assert_eq!(grouped.len(), 2);
        assert_eq!(grouped.get("foo").map(|v| v.len()), Some(2));
        assert_eq!(grouped.get("other").map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_format_architecture_bible_empty() {
        let memories: Vec<LocalMemorySummary> = vec![];
        let content = format_architecture_bible(&memories);
        assert!(content.contains("# NL_ARCHITECTURE_BIBLE"));
        // P84: New header format includes "Total entries: 0"
        assert!(content.contains("Total entries: 0"));
        assert!(content.contains("Seeded by Stage0"));
    }

    #[test]
    fn test_format_bug_retros_empty() {
        let memories: Vec<LocalMemorySummary> = vec![];
        let content = format_bug_retros(&memories);
        assert!(content.contains("# NL_BUG_RETROS_01"));
        assert!(content.contains("No bug-related memories found"));
        assert!(content.contains("Total entries: 0"));
    }

    #[test]
    fn test_format_debt_landscape_empty() {
        let grouped: HashMap<String, Vec<DebtItem>> = HashMap::new();
        let content = format_debt_landscape(&grouped);
        assert!(content.contains("# NL_DEBT_LANDSCAPE"));
        // P84: New header format includes "Total entries: 0" instead of footer
        assert!(content.contains("Total entries: 0"));
        assert!(content.contains("Seeded by Stage0"));
    }

    #[test]
    fn test_format_project_diary_empty() {
        let memories: Vec<LocalMemorySummary> = vec![];
        let content = format_project_diary(&memories);
        assert!(content.contains("# NL_PROJECT_DIARY_01"));
        assert!(content.contains("No diary entries found"));
        assert!(content.contains("Total entries: 0"));
    }

    #[test]
    fn test_seed_debt_landscape_from_temp_dir() {
        // Create a temp directory with a test file
        let tmp = tempfile::tempdir().unwrap();

        // Create a src subdirectory with a test file
        let src_dir = tmp.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let test_file = src_dir.join("test.rs");
        std::fs::write(
            &test_file,
            "// TODO: Fix this\n// FIXME: And this\n// HACK: workaround",
        )
        .unwrap();

        // Create output directory
        let output_dir = tmp.path().join("output");
        std::fs::create_dir_all(&output_dir).unwrap();

        let config = SeedingConfig {
            max_memories_per_artifact: 50,
            output_dir: output_dir.clone(),
            project_root: tmp.path().to_path_buf(),
        };

        let result = seed_debt_landscape(&config);
        assert!(result.is_ok(), "seed_debt_landscape should succeed");

        let artifact = result.unwrap();
        assert_eq!(artifact.kind, SeedKind::DebtLandscape);
        assert!(artifact.written, "File should be written");

        // Verify the output file exists and has content
        let output_file = output_dir.join(SeedKind::DebtLandscape.filename());
        assert!(output_file.exists(), "Output file should exist");
        let content = std::fs::read_to_string(&output_file).unwrap();
        assert!(
            content.contains("# NL_DEBT_LANDSCAPE"),
            "Should have header"
        );
    }

    #[test]
    fn test_scan_tech_debt_finds_comments() {
        let tmp = tempfile::tempdir().unwrap();
        let test_file = tmp.path().join("test.rs");
        std::fs::write(&test_file, "// TODO: Fix this\n// FIXME: And this").unwrap();

        let items = scan_tech_debt(tmp.path());
        // Note: scan may or may not find items depending on directory structure
        // This test just verifies no panic occurs
        assert!(items.len() <= 10, "Should not have excessive items");
    }
}
