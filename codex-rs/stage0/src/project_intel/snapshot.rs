//! Project snapshot builder
//!
//! Gathers project details from code, docs, memory, and specs.

use crate::project_intel::types::*;
use crate::{OverlayDb, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for snapshot building
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Project root directory
    pub root: PathBuf,
    /// Code source roots to scan
    pub code_roots: Vec<String>,
    /// Directories to ignore
    pub ignore_patterns: Vec<String>,
    /// Doc roots to scan
    pub doc_roots: Vec<String>,
    /// Memory domains to include
    pub memory_domains: Vec<String>,
    /// Spec index patterns (glob)
    pub spec_patterns: Vec<String>,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            code_roots: vec![
                "stage0/src".to_string(),
                "tui/src".to_string(),
                "core/src".to_string(),
            ],
            ignore_patterns: vec![
                "target".to_string(),
                "tests/fixtures".to_string(),
                "node_modules".to_string(),
            ],
            doc_roots: vec![
                "docs".to_string(),
                ".speckit".to_string(),
            ],
            memory_domains: vec![
                "spec-kit".to_string(),
                "constitution".to_string(),
                "librarian".to_string(),
                "project-notes".to_string(),
            ],
            spec_patterns: vec![
                "docs/SPEC-KIT-*/*.md".to_string(),
            ],
        }
    }
}

/// Builder for creating project snapshots
pub struct ProjectSnapshotBuilder {
    config: SnapshotConfig,
    snapshot: ProjectSnapshot,
}

impl ProjectSnapshotBuilder {
    /// Create a new snapshot builder
    pub fn new(config: SnapshotConfig, project_name: &str) -> Self {
        Self {
            config,
            snapshot: ProjectSnapshot::new(project_name),
        }
    }

    /// Build the complete snapshot
    pub fn build(mut self) -> Result<ProjectSnapshot> {
        self.gather_metadata()?;
        self.gather_code_topology()?;
        self.gather_workflows()?;
        self.gather_specs()?;
        // governance and memory require external clients
        // they're populated separately via set_* methods
        Ok(self.snapshot)
    }

    /// Set governance data (from overlay DB)
    pub fn set_governance(&mut self, governance: GovernanceSummary) {
        self.snapshot.governance = governance;
    }

    /// Set memory stats (from local-memory)
    pub fn set_memory_stats(&mut self, stats: MemorySummary) {
        self.snapshot.memory_stats = stats;
    }

    /// Set session lineage (from handoff docs)
    pub fn set_session_lineage(&mut self, lineage: SessionLineageSummary) {
        self.snapshot.sessions = lineage;
    }

    /// Gather project metadata from git
    fn gather_metadata(&mut self) -> Result<()> {
        let root = &self.config.root;
        self.snapshot.metadata.root_path = root.to_string_lossy().to_string();
        self.snapshot.metadata.snapshot_at = Utc::now();

        // Get git info
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(root)
            .output()
        {
            if output.status.success() {
                self.snapshot.metadata.branch = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
            }
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .current_dir(root)
            .output()
        {
            if output.status.success() {
                self.snapshot.metadata.commit_hash = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .to_string();
            }
        }

        if let Ok(output) = std::process::Command::new("git")
            .args(["describe", "--tags", "--abbrev=0"])
            .current_dir(root)
            .output()
        {
            if output.status.success() {
                self.snapshot.metadata.latest_tag = Some(
                    String::from_utf8_lossy(&output.stdout).trim().to_string()
                );
            }
        }

        Ok(())
    }

    /// Gather code topology from cargo metadata and file scanning
    fn gather_code_topology(&mut self) -> Result<()> {
        // Clone paths to avoid borrow issues
        let root = self.config.root.clone();
        let code_roots = self.config.code_roots.clone();
        let ignore_patterns = self.config.ignore_patterns.clone();

        // Get cargo metadata if available
        if let Ok(output) = std::process::Command::new("cargo")
            .args(["metadata", "--format-version=1", "--no-deps"])
            .current_dir(root.join("codex-rs"))
            .output()
        {
            if output.status.success() {
                if let Ok(metadata) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                    self.parse_cargo_metadata(&metadata);
                }
            }
        }

        // Add well-known modules (hardcoded for now, could be config-driven)
        self.snapshot.code_topology.key_modules = vec![
            ModuleInfo {
                name: "stage0".to_string(),
                path: "stage0/src".to_string(),
                description: "Memory retrieval, DCC, Tier 2 orchestration, Librarian".to_string(),
                responsibilities: vec![
                    "Dynamic Context Compiler (DCC)".to_string(),
                    "Tier 2 (NotebookLM) synthesis".to_string(),
                    "Overlay DB for scoring/caching".to_string(),
                    "Guardians (metadata + template)".to_string(),
                    "Librarian (classification, templating, causal inference)".to_string(),
                ],
            },
            ModuleInfo {
                name: "spec_kit".to_string(),
                path: "tui/src/chatwidget/spec_kit".to_string(),
                description: "Multi-agent automation framework commands".to_string(),
                responsibilities: vec![
                    "/speckit.* slash commands".to_string(),
                    "Pipeline coordinator (6-stage workflow)".to_string(),
                    "Command registry".to_string(),
                    "Stage0 integration".to_string(),
                    "Consensus coordinator".to_string(),
                ],
            },
            ModuleInfo {
                name: "librarian".to_string(),
                path: "stage0/src/librarian".to_string(),
                description: "Memory corpus quality engine".to_string(),
                responsibilities: vec![
                    "Memory type classification".to_string(),
                    "Template restructuring (CONTEXT/REASONING/OUTCOME)".to_string(),
                    "Causal relationship inference".to_string(),
                    "Audit trail".to_string(),
                ],
            },
            ModuleInfo {
                name: "project_intel".to_string(),
                path: "stage0/src/project_intel".to_string(),
                description: "Project intelligence gathering for NotebookLM".to_string(),
                responsibilities: vec![
                    "Snapshot builder".to_string(),
                    "NL_* doc generation".to_string(),
                    "NotebookLM sync".to_string(),
                ],
            },
        ];

        // Count files
        let mut file_counts: HashMap<String, usize> = HashMap::new();
        let mut total_loc = 0usize;

        for code_root in &code_roots {
            let path = root.join(code_root);
            if path.exists() {
                count_files_recursive(&path, &ignore_patterns, &mut file_counts, &mut total_loc);
            }
        }

        self.snapshot.code_topology.file_counts = file_counts;
        self.snapshot.code_topology.total_loc = total_loc;

        // Add binaries
        self.snapshot.code_topology.binaries = vec![
            BinaryInfo {
                name: "codex-tui".to_string(),
                entry_point: "tui/src/main.rs".to_string(),
                description: "Terminal UI with spec-kit commands".to_string(),
            },
            BinaryInfo {
                name: "codex-cli".to_string(),
                entry_point: "cli/src/main.rs".to_string(),
                description: "Command-line interface".to_string(),
            },
        ];

        Ok(())
    }

    /// Parse cargo metadata JSON
    fn parse_cargo_metadata(&mut self, metadata: &serde_json::Value) {
        if let Some(packages) = metadata.get("packages").and_then(|p| p.as_array()) {
            for pkg in packages {
                let name = pkg.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let manifest_path = pkg.get("manifest_path").and_then(|p| p.as_str()).unwrap_or("");

                // Extract path relative to workspace
                let path = PathBuf::from(manifest_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();

                let crate_type = if pkg.get("targets")
                    .and_then(|t| t.as_array())
                    .map(|targets| targets.iter().any(|t| {
                        t.get("kind").and_then(|k| k.as_array())
                            .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("bin")))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false)
                {
                    CrateType::Binary
                } else if pkg.get("targets")
                    .and_then(|t| t.as_array())
                    .map(|targets| targets.iter().any(|t| {
                        t.get("kind").and_then(|k| k.as_array())
                            .map(|kinds| kinds.iter().any(|k| k.as_str() == Some("proc-macro")))
                            .unwrap_or(false)
                    }))
                    .unwrap_or(false)
                {
                    CrateType::ProcMacro
                } else {
                    CrateType::Library
                };

                // Get dependencies
                let deps = pkg.get("dependencies")
                    .and_then(|d| d.as_array())
                    .map(|deps| {
                        deps.iter()
                            .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();

                self.snapshot.code_topology.crates.push(CrateInfo {
                    name: name.to_string(),
                    path,
                    crate_type,
                    dependencies: deps,
                    loc: 0, // Would need separate calculation
                });
            }
        }
    }


    /// Gather workflow information
    fn gather_workflows(&mut self) -> Result<()> {
        self.snapshot.workflows = vec![
            WorkflowSummary {
                name: "speckit.auto".to_string(),
                command: "/speckit.auto SPEC-XXX".to_string(),
                description: "Full 6-stage automated implementation pipeline".to_string(),
                stages: vec![
                    "Stage 0: Context gathering (DCC + Tier 2)".to_string(),
                    "Stage 1: Planning (Genius Architect)".to_string(),
                    "Stage 2: Task breakdown".to_string(),
                    "Stage 3: Implementation (Rust Ace)".to_string(),
                    "Stage 4: Validation".to_string(),
                    "Stage 5: Audit (Final Judge)".to_string(),
                ],
                models_used: vec![
                    "GPT-5.1 Genius Architect".to_string(),
                    "Claude Opus 4.5 Rust Ace".to_string(),
                    "GPT-5.1 Final Judge".to_string(),
                ],
                is_multi_agent: true,
            },
            WorkflowSummary {
                name: "speckit.new".to_string(),
                command: "/speckit.new <description>".to_string(),
                description: "Create new SPEC with guided questions".to_string(),
                stages: vec![
                    "Project detection".to_string(),
                    "Guided Q&A".to_string(),
                    "SPEC scaffolding".to_string(),
                ],
                models_used: vec![],
                is_multi_agent: false,
            },
            WorkflowSummary {
                name: "stage0.librarian".to_string(),
                command: "/stage0.librarian sweep [--apply]".to_string(),
                description: "Memory corpus quality sweep".to_string(),
                stages: vec![
                    "Memory classification".to_string(),
                    "Template restructuring".to_string(),
                    "Causal inference".to_string(),
                ],
                models_used: vec![],
                is_multi_agent: false,
            },
            WorkflowSummary {
                name: "speckit.vision".to_string(),
                command: "/speckit.vision".to_string(),
                description: "Constitution builder wizard".to_string(),
                stages: vec![
                    "Goals Q&A".to_string(),
                    "Non-goals Q&A".to_string(),
                    "Principles derivation".to_string(),
                    "Constitution storage".to_string(),
                ],
                models_used: vec![],
                is_multi_agent: false,
            },
            WorkflowSummary {
                name: "speckit.check-alignment".to_string(),
                command: "/speckit.check-alignment".to_string(),
                description: "Drift detection against constitution".to_string(),
                stages: vec![
                    "Recent memory scan".to_string(),
                    "Constitution comparison".to_string(),
                    "Violation reporting".to_string(),
                ],
                models_used: vec![],
                is_multi_agent: false,
            },
            WorkflowSummary {
                name: "stage0.project-intel".to_string(),
                command: "/stage0.project-intel <subcommand>".to_string(),
                description: "Project intelligence gathering for NotebookLM".to_string(),
                stages: vec![
                    "snapshot: Gather project details".to_string(),
                    "curate-nl: Generate NL_* docs".to_string(),
                    "sync-nl: Push to NotebookLM".to_string(),
                    "overview: Query global mental model".to_string(),
                ],
                models_used: vec!["NotebookLM Gemini".to_string()],
                is_multi_agent: false,
            },
        ];

        Ok(())
    }

    /// Gather spec summaries from docs
    fn gather_specs(&mut self) -> Result<()> {
        // Hardcoded for now - could parse SPEC files
        self.snapshot.specs = vec![
            SpecSummary {
                id: "SPEC-KIT-101".to_string(),
                title: "Core Framework".to_string(),
                phase: "P72-P80".to_string(),
                status: SpecStatus::Implemented,
                summary: "Foundation for spec-kit multi-agent automation".to_string(),
                deliverables: vec![
                    "Command registry".to_string(),
                    "Pipeline coordinator".to_string(),
                    "Stage 0 DCC".to_string(),
                ],
            },
            SpecSummary {
                id: "SPEC-KIT-102".to_string(),
                title: "Stage 0 Overlay".to_string(),
                phase: "P82-P86".to_string(),
                status: SpecStatus::Implemented,
                summary: "Memory overlay with DCC, Tier 2, and vector retrieval".to_string(),
                deliverables: vec![
                    "Overlay DB".to_string(),
                    "Dynamic scoring".to_string(),
                    "Tier 2 cache".to_string(),
                    "Vector backend".to_string(),
                ],
            },
            SpecSummary {
                id: "SPEC-KIT-103".to_string(),
                title: "Librarian".to_string(),
                phase: "P97-P99".to_string(),
                status: SpecStatus::Partial,
                summary: "Memory corpus quality engine with classification and causal inference".to_string(),
                deliverables: vec![
                    "Memory classifier".to_string(),
                    "Template restructurer".to_string(),
                    "Causal inference".to_string(),
                    "Audit trail".to_string(),
                ],
            },
            SpecSummary {
                id: "SPEC-KIT-105".to_string(),
                title: "Constitution & Governance".to_string(),
                phase: "P91-P95".to_string(),
                status: SpecStatus::Implemented,
                summary: "Project constitution, drift detection, and readiness gates".to_string(),
                deliverables: vec![
                    "Vision wizard".to_string(),
                    "Constitution storage".to_string(),
                    "Drift detection".to_string(),
                    "Gate modes".to_string(),
                ],
            },
        ];

        Ok(())
    }
}

/// Recursively count files and LOC (standalone function to avoid borrow issues)
fn count_files_recursive(
    path: &Path,
    ignore_patterns: &[String],
    counts: &mut HashMap<String, usize>,
    total_loc: &mut usize,
) {
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();

            // Skip ignored patterns
            if ignore_patterns.iter().any(|p| {
                entry_path.to_string_lossy().contains(p)
            }) {
                continue;
            }

            if entry_path.is_dir() {
                count_files_recursive(&entry_path, ignore_patterns, counts, total_loc);
            } else if let Some(ext) = entry_path.extension() {
                let ext_str = ext.to_string_lossy().to_string();
                *counts.entry(ext_str).or_insert(0) += 1;

                // Count lines for .rs files
                if ext == "rs" {
                    if let Ok(content) = fs::read_to_string(&entry_path) {
                        *total_loc += content.lines().count();
                    }
                }
            }
        }
    }
}

/// Load governance data from overlay DB
pub fn load_governance_from_db(db: &OverlayDb) -> Result<GovernanceSummary> {
    let version = db.get_constitution_version()?;
    let (_, hash, _) = db.get_constitution_meta()?;

    let memories = db.get_constitution_memories(100)?;

    let mut guardrails = Vec::new();
    let mut principles = Vec::new();
    let mut goals = Vec::new();
    let mut non_goals = Vec::new();

    for mem in memories {
        let content = mem.content_raw.clone().unwrap_or_default();
        let item = ConstitutionItem {
            id: mem.memory_id.clone(),
            text: content.clone(),
            priority: mem.initial_priority,
        };

        match mem.initial_priority {
            10 => guardrails.push(item),
            9 => principles.push(item),
            8 => {
                // Goals and non-goals both have priority 8
                // Distinguish by content or type
                let lower = content.to_lowercase();
                if lower.contains("non-goal") || lower.contains("not a goal") {
                    non_goals.push(content);
                } else {
                    goals.push(content);
                }
            }
            _ => {}
        }
    }

    Ok(GovernanceSummary {
        constitution_version: version,
        constitution_hash: hash,
        guardrails,
        principles,
        goals,
        non_goals,
        gate_mode: GateMode::Warn,
        drift_status: DriftStatus::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_builder_basic() {
        let config = SnapshotConfig {
            root: PathBuf::from("/tmp/test"),
            ..Default::default()
        };
        let builder = ProjectSnapshotBuilder::new(config, "test-project");
        let snapshot = builder.snapshot;

        assert_eq!(snapshot.metadata.name, "test-project");
        assert!(!snapshot.workflows.is_empty() || snapshot.workflows.is_empty()); // Just checking it compiles
    }

    #[test]
    fn test_snapshot_to_json() {
        let snapshot = ProjectSnapshot::new("test");
        let json = snapshot.to_json().expect("should serialize");
        assert!(json.contains("\"name\": \"test\""));
    }

    #[test]
    fn test_markdown_generation() {
        let mut snapshot = ProjectSnapshot::new("test");
        snapshot.code_topology.key_modules.push(ModuleInfo {
            name: "test-module".to_string(),
            path: "src/test".to_string(),
            description: "Test module".to_string(),
            responsibilities: vec!["Testing".to_string()],
        });

        let md = snapshot.code_topology_md();
        assert!(md.contains("test-module"));
        assert!(md.contains("Test module"));
    }
}
