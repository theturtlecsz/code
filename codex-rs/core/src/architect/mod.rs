//! Architect Sidecar - Forensic intelligence harvesting for codex-rs.
//!
//! This module provides native Rust implementations of the architect intelligence
//! gathering tools, replacing the original Python scripts.
//!
//! # Modules
//! - [`churn`] - Git forensics: churn hotspots and logical coupling analysis
//! - [`complexity`] - Code complexity metrics: LOC, indentation, function count
//! - [`skeleton`] - API extraction using tree-sitter for Rust/TS/Python
//! - [`mermaid`] - Call graph and module dependency visualization
//! - [`graph_bridge`] - CodeGraphContext MCP bridge (Python only)

pub mod churn;
pub mod complexity;
pub mod graph_bridge;
pub mod mermaid;
pub mod skeleton;

use anyhow::Result;
use std::path::Path;

/// Configuration for the harvester.
#[derive(Debug, Clone, Default)]
pub struct HarvesterConfig {
    /// Number of months to analyze for churn (default: 12)
    pub churn_months: u32,
    /// Minimum co-changes to report coupling (default: 5)
    pub min_cochanges: u32,
    /// File extensions to include in analysis
    pub extensions: Vec<String>,
}

impl HarvesterConfig {
    pub fn new() -> Self {
        Self {
            churn_months: 12,
            min_cochanges: 5,
            extensions: vec![
                "rs".into(),
                "ts".into(),
                "tsx".into(),
                "py".into(),
                "md".into(),
                "toml".into(),
            ],
        }
    }
}

/// Run the full harvester pipeline.
pub async fn run_harvest(
    repo_root: &Path,
    output_dir: &Path,
    config: &HarvesterConfig,
) -> Result<HarvestResults> {
    let mut results = HarvestResults::default();

    // Run churn analysis
    let churn_report = churn::analyze(repo_root, config)?;
    let churn_path = output_dir.join("churn_matrix.md");
    tokio::fs::write(&churn_path, &churn_report.to_markdown()).await?;
    results.churn_files = churn_report.file_count;
    results.churn_path = Some(churn_path);

    // Run complexity analysis
    let complexity_report = complexity::analyze(repo_root, config)?;
    let complexity_path = output_dir.join("complexity_map.json");
    tokio::fs::write(&complexity_path, &complexity_report.to_json()?).await?;
    results.complexity_files = complexity_report.file_count;
    results.complexity_path = Some(complexity_path);

    // Run skeleton extraction
    let skeleton_report = skeleton::extract(repo_root)?;
    let skeleton_path = output_dir.join("repo_skeleton.xml");
    tokio::fs::write(&skeleton_path, &skeleton_report.to_xml()).await?;
    results.skeleton_files = skeleton_report.file_count;
    results.skeleton_path = Some(skeleton_path);

    Ok(results)
}

/// Results from a harvest operation.
#[derive(Debug, Default)]
pub struct HarvestResults {
    pub churn_files: usize,
    pub churn_path: Option<std::path::PathBuf>,
    pub complexity_files: usize,
    pub complexity_path: Option<std::path::PathBuf>,
    pub skeleton_files: usize,
    pub skeleton_path: Option<std::path::PathBuf>,
}
