//! Git forensics: churn hotspots and logical coupling analysis.
//!
//! Uses git2 to analyze repository history without spawning subprocesses.

use crate::architect::HarvesterConfig;
use anyhow::{Context, Result};
use git2::{Repository, Time};
use std::collections::HashMap;
use std::path::Path;

/// Churn analysis results.
#[derive(Debug)]
pub struct ChurnReport {
    /// Files sorted by commit count (descending)
    pub hotspots: Vec<FileChurn>,
    /// File pairs sorted by co-change count (descending)
    pub coupling: Vec<FileCoupling>,
    /// Total files analyzed
    pub file_count: usize,
    /// Total commits analyzed
    pub commit_count: usize,
    /// Analysis period in months
    pub months_analyzed: u32,
}

#[derive(Debug, Clone)]
pub struct FileChurn {
    pub path: String,
    pub commits: usize,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct FileCoupling {
    pub file_a: String,
    pub file_b: String,
    pub cochanges: usize,
    pub strength: CouplingStrength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskLevel {
    Critical,
    High,
    Medium,
    Low,
}

impl RiskLevel {
    fn from_commits(count: usize) -> Self {
        if count >= 50 {
            Self::Critical
        } else if count >= 25 {
            Self::High
        } else if count >= 10 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    fn emoji(self) -> &'static str {
        match self {
            Self::Critical => "🔴 Critical",
            Self::High => "🟠 High",
            Self::Medium => "🟡 Medium",
            Self::Low => "🟢 Low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouplingStrength {
    VeryStrong,
    Strong,
    Moderate,
    Weak,
}

impl CouplingStrength {
    fn from_cochanges(count: usize) -> Self {
        if count >= 15 {
            Self::VeryStrong
        } else if count >= 10 {
            Self::Strong
        } else if count >= 7 {
            Self::Moderate
        } else {
            Self::Weak
        }
    }

    fn emoji(self) -> &'static str {
        match self {
            Self::VeryStrong => "🔴 Very Strong",
            Self::Strong => "🟠 Strong",
            Self::Moderate => "🟡 Moderate",
            Self::Weak => "🟢 Weak",
        }
    }
}

/// Analyze repository for churn and coupling.
pub fn analyze(repo_root: &Path, config: &HarvesterConfig) -> Result<ChurnReport> {
    let repo = Repository::open(repo_root).context("Failed to open git repository")?;

    // Calculate cutoff time (months ago)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("system time before UNIX_EPOCH: {e}"))?
        .as_secs() as i64;
    let seconds_per_month = 30 * 24 * 60 * 60i64;
    let cutoff = Time::new(now - (config.churn_months as i64 * seconds_per_month), 0);

    // Collect commits with their changed files
    let commits = collect_commits(&repo, cutoff, &config.extensions)?;

    // Calculate churn (commit count per file)
    let churn = calculate_churn(&commits);

    // Calculate coupling (files that change together)
    let coupling = calculate_coupling(&commits, config.min_cochanges as usize);

    // Build hotspots list
    let mut hotspots: Vec<FileChurn> = churn
        .into_iter()
        .map(|(path, commits)| FileChurn {
            risk_level: RiskLevel::from_commits(commits),
            path,
            commits,
        })
        .collect();
    hotspots.sort_by(|a, b| b.commits.cmp(&a.commits));

    // Build coupling list
    let mut coupling_list: Vec<FileCoupling> = coupling
        .into_iter()
        .map(|((file_a, file_b), cochanges)| FileCoupling {
            strength: CouplingStrength::from_cochanges(cochanges),
            file_a,
            file_b,
            cochanges,
        })
        .collect();
    coupling_list.sort_by(|a, b| b.cochanges.cmp(&a.cochanges));

    Ok(ChurnReport {
        file_count: hotspots.len(),
        commit_count: commits.len(),
        months_analyzed: config.churn_months,
        hotspots,
        coupling: coupling_list,
    })
}

/// Collect commits with their changed files since cutoff.
fn collect_commits(
    repo: &Repository,
    cutoff: Time,
    extensions: &[String],
) -> Result<Vec<Vec<String>>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let mut commits = Vec::new();

    for oid in revwalk {
        let oid = oid?;
        let commit = repo.find_commit(oid)?;

        // Skip if before cutoff
        if commit.time() < cutoff {
            break;
        }

        // Skip merge commits (they usually have multiple parents)
        if commit.parent_count() > 1 {
            continue;
        }

        // Get changed files
        let files = get_changed_files(repo, &commit, extensions)?;
        if !files.is_empty() {
            commits.push(files);
        }
    }

    Ok(commits)
}

/// Get files changed in a commit, filtered by extension.
fn get_changed_files(
    repo: &Repository,
    commit: &git2::Commit,
    extensions: &[String],
) -> Result<Vec<String>> {
    let tree = commit.tree()?;
    let parent_tree = if commit.parent_count() > 0 {
        Some(commit.parent(0)?.tree()?)
    } else {
        None
    };

    let diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)?;

    let mut files = Vec::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(path) = delta.new_file().path() {
                let path_str = path.to_string_lossy();
                // Filter by extension
                if let Some(ext) = path.extension()
                    && extensions.iter().any(|e| e == ext.to_str().unwrap_or("")) {
                        files.push(path_str.to_string());
                    }
            }
            true
        },
        None,
        None,
        None,
    )?;

    Ok(files)
}

/// Calculate commit count per file.
fn calculate_churn(commits: &[Vec<String>]) -> HashMap<String, usize> {
    let mut churn: HashMap<String, usize> = HashMap::new();
    for files in commits {
        for file in files {
            *churn.entry(file.clone()).or_insert(0) += 1;
        }
    }
    churn
}

/// Calculate file co-change coupling.
fn calculate_coupling(
    commits: &[Vec<String>],
    min_cochanges: usize,
) -> HashMap<(String, String), usize> {
    let mut cochange_count: HashMap<(String, String), usize> = HashMap::new();

    for files in commits {
        // Only consider commits with 2-10 files (larger commits are usually bulk changes)
        if files.len() >= 2 && files.len() <= 10 {
            let mut sorted_files: Vec<_> = files.iter().collect();
            sorted_files.sort();

            // Count pairs
            for i in 0..sorted_files.len() {
                for j in (i + 1)..sorted_files.len() {
                    let pair = (sorted_files[i].clone(), sorted_files[j].clone());
                    *cochange_count.entry(pair).or_insert(0) += 1;
                }
            }
        }
    }

    // Filter to significant coupling
    cochange_count
        .into_iter()
        .filter(|(_, count)| *count >= min_cochanges)
        .collect()
}

impl ChurnReport {
    /// Generate markdown report.
    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# Forensic Churn & Coupling Analysis".to_string(),
            String::new(),
            format!("_Generated: {}_", chrono::Utc::now().to_rfc3339()),
            String::new(),
            "## Churn Hotspots (Top 30)".to_string(),
            String::new(),
            format!(
                "Files with the highest number of commits in the last {} months.",
                self.months_analyzed
            ),
            "High churn indicates active development, potential instability, or design issues."
                .to_string(),
            String::new(),
            "| Rank | File | Commits | Risk Level |".to_string(),
            "|------|------|---------|------------|".to_string(),
        ];

        for (i, hotspot) in self.hotspots.iter().take(30).enumerate() {
            lines.push(format!(
                "| {} | `{}` | {} | {} |",
                i + 1,
                hotspot.path,
                hotspot.commits,
                hotspot.risk_level.emoji()
            ));
        }

        lines.extend([
            String::new(),
            "## Logical Coupling (Top 20)".to_string(),
            String::new(),
            format!(
                "Files that frequently change together (>= {} co-changes).",
                5 // min_cochanges default
            ),
            "High coupling may indicate hidden dependencies or shared responsibility.".to_string(),
            String::new(),
            "| Rank | File A | File B | Co-Changes | Coupling Strength |".to_string(),
            "|------|--------|--------|------------|-------------------|".to_string(),
        ]);

        for (i, coupling) in self.coupling.iter().take(20).enumerate() {
            let file_a = truncate_path(&coupling.file_a, 47);
            let file_b = truncate_path(&coupling.file_b, 47);
            lines.push(format!(
                "| {} | `{}` | `{}` | {} | {} |",
                i + 1,
                file_a,
                file_b,
                coupling.cochanges,
                coupling.strength.emoji()
            ));
        }

        // Coupling clusters
        lines.extend([
            String::new(),
            "## Coupling Clusters".to_string(),
            String::new(),
            "Files grouped by their coupling relationships:".to_string(),
            String::new(),
        ]);

        // Build clusters
        let mut clusters: HashMap<String, Vec<(String, usize)>> = HashMap::new();
        for c in self.coupling.iter().take(30) {
            clusters
                .entry(c.file_a.clone())
                .or_default()
                .push((c.file_b.clone(), c.cochanges));
            clusters
                .entry(c.file_b.clone())
                .or_default()
                .push((c.file_a.clone(), c.cochanges));
        }

        let mut sorted_clusters: Vec<_> = clusters.into_iter().collect();
        sorted_clusters.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

        for (file, connections) in sorted_clusters.into_iter().take(10) {
            let file_short = truncate_path(&file, 57);
            lines.push(format!("### `{file_short}`"));
            lines.push(String::new());

            let mut sorted_conns = connections;
            sorted_conns.sort_by(|a, b| b.1.cmp(&a.1));
            for (conn, count) in sorted_conns.into_iter().take(5) {
                let conn_short = truncate_path(&conn, 47);
                lines.push(format!("- `{conn_short}` ({count} co-changes)"));
            }
            lines.push(String::new());
        }

        // Summary statistics
        lines.extend([
            String::new(),
            "## Summary Statistics".to_string(),
            String::new(),
            format!("- Total files analyzed: {}", self.file_count),
            format!("- Total commits analyzed: {}", self.commit_count),
            format!(
                "- Total coupled pairs (>= 5 co-changes): {}",
                self.coupling.len()
            ),
            format!(
                "- Highest churn: {} commits",
                self.hotspots.first().map(|h| h.commits).unwrap_or(0)
            ),
            format!(
                "- Strongest coupling: {} co-changes",
                self.coupling.first().map(|c| c.cochanges).unwrap_or(0)
            ),
            String::new(),
            "## Risk Assessment".to_string(),
            String::new(),
            "Files appearing in BOTH high churn AND high coupling are **critical risk zones**:"
                .to_string(),
            String::new(),
        ]);

        // Find critical files
        let high_churn_files: std::collections::HashSet<_> = self
            .hotspots
            .iter()
            .filter(|h| h.commits >= 20)
            .map(|h| &h.path)
            .collect();

        let high_coupling_files: std::collections::HashSet<_> = self
            .coupling
            .iter()
            .filter(|c| c.cochanges >= 8)
            .flat_map(|c| [&c.file_a, &c.file_b])
            .collect();

        let critical_files: Vec<_> = high_churn_files
            .intersection(&high_coupling_files)
            .collect();

        if critical_files.is_empty() {
            lines.push("_No files in critical risk zone._".to_string());
        } else {
            for file in critical_files {
                let churn_count = self
                    .hotspots
                    .iter()
                    .find(|h| &h.path == *file)
                    .map(|h| h.commits)
                    .unwrap_or(0);
                let coupling_total: usize = self
                    .coupling
                    .iter()
                    .filter(|c| &c.file_a == *file || &c.file_b == *file)
                    .map(|c| c.cochanges)
                    .sum();
                lines.push(format!(
                    "- `{file}` (churn: {churn_count}, total coupling: {coupling_total})"
                ));
            }
        }

        lines.join("\n")
    }
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_level() {
        assert_eq!(RiskLevel::from_commits(100), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_commits(50), RiskLevel::Critical);
        assert_eq!(RiskLevel::from_commits(30), RiskLevel::High);
        assert_eq!(RiskLevel::from_commits(15), RiskLevel::Medium);
        assert_eq!(RiskLevel::from_commits(5), RiskLevel::Low);
    }

    #[test]
    fn test_coupling_strength() {
        assert_eq!(
            CouplingStrength::from_cochanges(20),
            CouplingStrength::VeryStrong
        );
        assert_eq!(
            CouplingStrength::from_cochanges(12),
            CouplingStrength::Strong
        );
        assert_eq!(
            CouplingStrength::from_cochanges(8),
            CouplingStrength::Moderate
        );
        assert_eq!(CouplingStrength::from_cochanges(5), CouplingStrength::Weak);
    }

    #[test]
    fn test_truncate_path() {
        assert_eq!(truncate_path("short.rs", 50), "short.rs");
        let long = "some/very/long/path/to/a/deeply/nested/file.rs";
        let truncated = truncate_path(long, 20);
        assert!(truncated.starts_with("..."));
        assert!(truncated.len() <= 20);
    }
}
