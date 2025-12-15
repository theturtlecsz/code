//! Project Intel data types
//!
//! Defines the structured data model for project snapshots.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete project snapshot containing all gathered intelligence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    /// Snapshot metadata
    pub metadata: ProjectMetadata,
    /// Code topology analysis
    pub code_topology: CodeTopology,
    /// Spec-kit and Stage0 workflows
    pub workflows: Vec<WorkflowSummary>,
    /// SPEC-KIT-* spec summaries
    pub specs: Vec<SpecSummary>,
    /// Constitution and governance info
    pub governance: GovernanceSummary,
    /// Memory system stats
    pub memory_stats: MemorySummary,
    /// Session lineage (Pxx milestones)
    pub sessions: SessionLineageSummary,
}

impl ProjectSnapshot {
    /// Create a new empty snapshot
    pub fn new(name: &str) -> Self {
        Self {
            metadata: ProjectMetadata::new(name),
            code_topology: CodeTopology::default(),
            workflows: Vec::new(),
            specs: Vec::new(),
            governance: GovernanceSummary::default(),
            memory_stats: MemorySummary::default(),
            sessions: SessionLineageSummary::default(),
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Generate markdown feed for code topology
    pub fn code_topology_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# Code Topology\n\n");
        out.push_str(&format!("Generated: {}\n\n", self.metadata.snapshot_at));

        // Crates
        out.push_str("## Crates\n\n");
        for crate_info in &self.code_topology.crates {
            out.push_str(&format!(
                "### {}\n- Path: `{}`\n- Type: {:?}\n- Dependencies: {}\n\n",
                crate_info.name,
                crate_info.path,
                crate_info.crate_type,
                crate_info.dependencies.join(", ")
            ));
        }

        // Key modules
        out.push_str("## Key Modules\n\n");
        for module in &self.code_topology.key_modules {
            out.push_str(&format!(
                "- **{}**: {} (`{}`)\n",
                module.name, module.description, module.path
            ));
        }

        // Binaries
        out.push_str("\n## Binaries\n\n");
        for bin in &self.code_topology.binaries {
            out.push_str(&format!(
                "- **{}**: {} (`{}`)\n",
                bin.name, bin.description, bin.entry_point
            ));
        }

        out
    }

    /// Generate markdown feed for workflows
    pub fn workflows_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# Spec-Kit Workflows\n\n");

        for workflow in &self.workflows {
            out.push_str(&format!("## {}\n\n", workflow.name));
            out.push_str(&format!("**Command:** `{}`\n\n", workflow.command));
            out.push_str(&format!("{}\n\n", workflow.description));

            if !workflow.stages.is_empty() {
                out.push_str("**Stages:**\n");
                for stage in &workflow.stages {
                    out.push_str(&format!("1. {stage}\n"));
                }
                out.push('\n');
            }

            if !workflow.models_used.is_empty() {
                out.push_str(&format!(
                    "**Models:** {}\n\n",
                    workflow.models_used.join(", ")
                ));
            }
        }

        out
    }

    /// Generate markdown feed for specs
    pub fn specs_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# SPEC-KIT Specifications\n\n");

        // Group by phase
        let mut by_phase: HashMap<String, Vec<&SpecSummary>> = HashMap::new();
        for spec in &self.specs {
            by_phase.entry(spec.phase.clone()).or_default().push(spec);
        }

        for (phase, specs) in by_phase.iter() {
            out.push_str(&format!("## {phase}\n\n"));
            for spec in specs {
                out.push_str(&format!(
                    "### {}: {}\n\n**Status:** {:?}\n\n{}\n\n",
                    spec.id, spec.title, spec.status, spec.summary
                ));
            }
        }

        out
    }

    /// Generate markdown feed for governance
    pub fn governance_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# Governance & Constitution\n\n");

        out.push_str(&format!(
            "**Constitution Version:** {}\n\n",
            self.governance.constitution_version
        ));

        // Guardrails
        out.push_str("## Guardrails\n\n");
        for g in &self.governance.guardrails {
            out.push_str(&format!("- **{}:** {}\n", g.id, g.text));
        }

        // Principles
        out.push_str("\n## Principles\n\n");
        for p in &self.governance.principles {
            out.push_str(&format!("- **{}:** {}\n", p.id, p.text));
        }

        // Goals
        out.push_str("\n## Goals\n\n");
        for g in &self.governance.goals {
            out.push_str(&format!("- {g}\n"));
        }

        // Non-goals
        out.push_str("\n## Non-Goals\n\n");
        for ng in &self.governance.non_goals {
            out.push_str(&format!("- {ng}\n"));
        }

        // Gate mode
        out.push_str(&format!(
            "\n**Gate Mode:** {:?}\n",
            self.governance.gate_mode
        ));

        // Drift detection
        out.push_str("\n## Drift Detection\n\n");
        out.push_str(&format!(
            "- Last check: {}\n",
            self.governance
                .drift_status
                .last_check
                .map(|d| d.to_string())
                .unwrap_or_else(|| "Never".to_string())
        ));
        out.push_str(&format!(
            "- Drift detected: {}\n",
            self.governance.drift_status.drift_detected
        ));
        if !self.governance.drift_status.violations.is_empty() {
            out.push_str("- Violations:\n");
            for v in &self.governance.drift_status.violations {
                out.push_str(&format!("  - {v}\n"));
            }
        }

        out
    }

    /// Generate markdown feed for memory stats
    pub fn memory_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# Memory & Librarian\n\n");

        // Overall stats
        out.push_str("## Memory Stats\n\n");
        out.push_str(&format!(
            "- Total memories: {}\n",
            self.memory_stats.total_memories
        ));
        out.push_str(&format!(
            "- Templated: {} ({:.1}%)\n",
            self.memory_stats.templated_count, self.memory_stats.templated_percent
        ));
        out.push_str(&format!(
            "- Legacy (unstructured): {}\n",
            self.memory_stats.legacy_count
        ));

        // By domain
        out.push_str("\n## Memories by Domain\n\n");
        for (domain, count) in &self.memory_stats.by_domain {
            out.push_str(&format!("- {domain}: {count}\n"));
        }

        // Librarian status
        out.push_str("\n## Librarian Status\n\n");
        out.push_str(&format!(
            "- Last sweep: {}\n",
            self.memory_stats
                .librarian
                .last_sweep
                .map(|d| d.to_string())
                .unwrap_or_else(|| "Never".to_string())
        ));
        out.push_str(&format!(
            "- Memories processed: {}\n",
            self.memory_stats.librarian.memories_processed
        ));
        out.push_str(&format!(
            "- Causal edges created: {}\n",
            self.memory_stats.librarian.causal_edges_created
        ));

        out
    }

    /// Generate markdown feed for session lineage
    pub fn sessions_md(&self) -> String {
        let mut out = String::new();
        out.push_str("# Session Lineage\n\n");

        out.push_str(&format!(
            "**Current Session:** {}\n\n",
            self.sessions.current_session
        ));

        // Milestones
        out.push_str("## Key Milestones\n\n");
        for milestone in &self.sessions.milestones {
            out.push_str(&format!(
                "### {} ({})\n\n{}\n\n",
                milestone.session_id,
                milestone.date.format("%Y-%m-%d"),
                milestone.summary
            ));
            if !milestone.key_achievements.is_empty() {
                out.push_str("**Achievements:**\n");
                for a in &milestone.key_achievements {
                    out.push_str(&format!("- {a}\n"));
                }
                out.push('\n');
            }
        }

        // Timeline
        out.push_str("## Evolution Timeline\n\n");
        for entry in &self.sessions.timeline {
            out.push_str(&format!(
                "- **{}** ({}): {}\n",
                entry.session_id,
                entry.date.format("%Y-%m-%d"),
                entry.event
            ));
        }

        out
    }
}

/// Project metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMetadata {
    /// Project name
    pub name: String,
    /// Repository root path
    pub root_path: String,
    /// Current git branch
    pub branch: String,
    /// Latest git tag
    pub latest_tag: Option<String>,
    /// Current commit hash (short)
    pub commit_hash: String,
    /// Snapshot timestamp
    pub snapshot_at: DateTime<Utc>,
    /// Snapshot version (incremented each run)
    pub version: u32,
}

impl ProjectMetadata {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            root_path: String::new(),
            branch: String::new(),
            latest_tag: None,
            commit_hash: String::new(),
            snapshot_at: Utc::now(),
            version: 1,
        }
    }
}

/// Code topology - crates, modules, binaries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CodeTopology {
    /// Crate information
    pub crates: Vec<CrateInfo>,
    /// Key modules (stage0, librarian, spec_kit, etc.)
    pub key_modules: Vec<ModuleInfo>,
    /// Binary entry points
    pub binaries: Vec<BinaryInfo>,
    /// Total lines of code
    pub total_loc: usize,
    /// File counts by extension
    pub file_counts: HashMap<String, usize>,
}

/// Information about a Rust crate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateInfo {
    /// Crate name
    pub name: String,
    /// Path relative to workspace
    pub path: String,
    /// Type of crate
    pub crate_type: CrateType,
    /// Direct dependencies
    pub dependencies: Vec<String>,
    /// Lines of code
    pub loc: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrateType {
    Library,
    Binary,
    ProcMacro,
}

/// Information about a key module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Module path (e.g., "stage0/src/librarian")
    pub path: String,
    /// Brief description
    pub description: String,
    /// Key exports/responsibilities
    pub responsibilities: Vec<String>,
}

/// Binary entry point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinaryInfo {
    /// Binary name
    pub name: String,
    /// Entry point path
    pub entry_point: String,
    /// Brief description
    pub description: String,
}

/// Workflow summary (commands like /speckit.auto)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSummary {
    /// Workflow name
    pub name: String,
    /// Command to invoke
    pub command: String,
    /// Description
    pub description: String,
    /// Workflow stages
    pub stages: Vec<String>,
    /// Models used
    pub models_used: Vec<String>,
    /// Whether it's a multi-agent workflow
    pub is_multi_agent: bool,
}

/// Spec summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecSummary {
    /// Spec ID (e.g., "SPEC-KIT-102")
    pub id: String,
    /// Title
    pub title: String,
    /// Phase (e.g., "P85", "P91")
    pub phase: String,
    /// Implementation status
    pub status: SpecStatus,
    /// Brief summary
    pub summary: String,
    /// Key deliverables
    pub deliverables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecStatus {
    Planned,
    InProgress,
    Partial,
    Implemented,
    Deprecated,
}

/// Governance and constitution summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernanceSummary {
    /// Constitution version
    pub constitution_version: u32,
    /// Constitution hash
    pub constitution_hash: Option<String>,
    /// Guardrails (hard constraints)
    pub guardrails: Vec<ConstitutionItem>,
    /// Principles (soft preferences)
    pub principles: Vec<ConstitutionItem>,
    /// Goals
    pub goals: Vec<String>,
    /// Non-goals
    pub non_goals: Vec<String>,
    /// Gate mode
    pub gate_mode: GateMode,
    /// Drift detection status
    pub drift_status: DriftStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstitutionItem {
    /// Item ID (e.g., "G1", "P2")
    pub id: String,
    /// Item text
    pub text: String,
    /// Priority (10 = guardrail, 9 = principle, 8 = goal)
    pub priority: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum GateMode {
    #[default]
    Warn,
    Block,
    Skip,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DriftStatus {
    /// Last drift check timestamp
    pub last_check: Option<DateTime<Utc>>,
    /// Whether drift was detected
    pub drift_detected: bool,
    /// List of violations
    pub violations: Vec<String>,
}

/// Memory system summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemorySummary {
    /// Total memory count
    pub total_memories: usize,
    /// Templated memory count
    pub templated_count: usize,
    /// Templated percentage
    pub templated_percent: f32,
    /// Legacy (unstructured) count
    pub legacy_count: usize,
    /// Memories by domain
    pub by_domain: HashMap<String, usize>,
    /// Librarian status
    pub librarian: LibrarianStatus,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LibrarianStatus {
    /// Last sweep timestamp
    pub last_sweep: Option<DateTime<Utc>>,
    /// Memories processed in last sweep
    pub memories_processed: usize,
    /// Causal edges created
    pub causal_edges_created: usize,
    /// Sweep ID
    pub last_sweep_id: Option<String>,
}

/// Session lineage summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionLineageSummary {
    /// Current session ID (e.g., "P99")
    pub current_session: String,
    /// Key milestones
    pub milestones: Vec<Milestone>,
    /// Timeline entries
    pub timeline: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Session ID
    pub session_id: String,
    /// Date
    pub date: DateTime<Utc>,
    /// Summary
    pub summary: String,
    /// Key achievements
    pub key_achievements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    /// Session ID
    pub session_id: String,
    /// Date
    pub date: DateTime<Utc>,
    /// Event description
    pub event: String,
}

/// NL document manifest for NotebookLM sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlManifest {
    /// NotebookLM notebook ID
    pub notebook_id: String,
    /// Source files to sync
    pub sources: Vec<NlSourceFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NlSourceFile {
    /// File path relative to project root
    pub path: String,
    /// Document title for NotebookLM
    pub title: String,
    /// Last sync timestamp
    pub last_sync: Option<DateTime<Utc>>,
}

impl NlManifest {
    /// Default manifest with standard NL_* docs
    pub fn default_manifest() -> Self {
        Self {
            notebook_id: "codex-rs-main".to_string(),
            sources: vec![
                NlSourceFile {
                    path: "docs/NL_ARCHITECTURE_BIBLE.md".to_string(),
                    title: "Architecture Bible".to_string(),
                    last_sync: None,
                },
                NlSourceFile {
                    path: "docs/NL_WORKFLOW_MAP.md".to_string(),
                    title: "Workflow Map".to_string(),
                    last_sync: None,
                },
                NlSourceFile {
                    path: "docs/NL_GOVERNANCE_AND_DRIFT.md".to_string(),
                    title: "Governance and Drift".to_string(),
                    last_sync: None,
                },
                NlSourceFile {
                    path: "docs/NL_MEMORY_AND_LIBRARIAN.md".to_string(),
                    title: "Memory and Librarian".to_string(),
                    last_sync: None,
                },
                NlSourceFile {
                    path: "docs/NL_SESSION_LINEAGE.md".to_string(),
                    title: "Session Lineage".to_string(),
                    last_sync: None,
                },
            ],
        }
    }
}
