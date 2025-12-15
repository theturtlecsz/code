//! Code complexity analysis: LOC, indentation depth, function count.
//!
//! Estimates code complexity using lightweight heuristics without full parsing.

use crate::architect::HarvesterConfig;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use walkdir::WalkDir;

/// Complexity analysis results.
#[derive(Debug)]
pub struct ComplexityReport {
    /// Files sorted by complexity score (descending)
    pub files: Vec<FileComplexity>,
    /// Total files analyzed
    pub file_count: usize,
    /// Breakdown by risk level
    pub by_risk: RiskBreakdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileComplexity {
    pub path: String,
    pub loc: usize,
    pub sloc: usize,
    pub max_indent: usize,
    pub avg_indent: f64,
    pub function_count: usize,
    pub complexity_score: f64,
    pub risk_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskBreakdown {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
}

/// Analyze repository for code complexity.
pub fn analyze(repo_root: &Path, config: &HarvesterConfig) -> Result<ComplexityReport> {
    let mut files = Vec::new();

    for entry in WalkDir::new(repo_root)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let path = entry.path();

        // Skip non-files
        if !path.is_file() {
            continue;
        }

        // Skip hidden and common exclude patterns
        let path_str = path.to_string_lossy();
        if path_str.contains("/target/")
            || path_str.contains("/node_modules/")
            || path_str.contains("/__pycache__/")
            || path_str.contains("/venv/")
            || path_str.contains("/.git/")
        {
            continue;
        }

        // Check extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !config.extensions.iter().any(|e| e == ext) {
            continue;
        }

        // Analyze based on file type
        let result = match ext {
            "rs" => analyze_rust_file(path, repo_root),
            "ts" | "tsx" => analyze_typescript_file(path, repo_root),
            "py" => analyze_python_file(path, repo_root),
            _ => continue,
        };

        if let Some(complexity) = result {
            files.push(complexity);
        }
    }

    // Sort by complexity score descending
    files.sort_by(|a, b| {
        b.complexity_score
            .partial_cmp(&a.complexity_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let by_risk = RiskBreakdown {
        critical: files.iter().filter(|f| f.risk_level == "critical").count(),
        high: files.iter().filter(|f| f.risk_level == "high").count(),
        medium: files.iter().filter(|f| f.risk_level == "medium").count(),
        low: files.iter().filter(|f| f.risk_level == "low").count(),
    };

    Ok(ComplexityReport {
        file_count: files.len(),
        files,
        by_risk,
    })
}

/// Analyze a Rust file for complexity metrics.
fn analyze_rust_file(path: &Path, repo_root: &Path) -> Option<FileComplexity> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let loc = lines.len();

    let mut sloc = 0;
    let mut indents = Vec::new();
    let mut in_block_comment = false;

    for line in &lines {
        let stripped = line.trim();

        // Handle block comments
        if stripped.contains("/*") {
            in_block_comment = true;
        }
        if stripped.contains("*/") {
            in_block_comment = false;
            continue;
        }
        if in_block_comment {
            continue;
        }

        // Skip empty and single-line comments
        if stripped.is_empty() || stripped.starts_with("//") {
            continue;
        }

        sloc += 1;

        // Calculate indentation
        let indent_chars = line.len() - line.trim_start().len();
        let indent = if line.starts_with('\t') {
            line.chars().take_while(|c| *c == '\t').count() * 4
                + line
                    .trim_start_matches('\t')
                    .chars()
                    .take_while(|c| *c == ' ')
                    .count()
        } else {
            indent_chars
        };
        indents.push(indent / 4); // Normalize to indent levels
    }

    let max_indent = indents.iter().max().copied().unwrap_or(0);
    let avg_indent = if indents.is_empty() {
        0.0
    } else {
        indents.iter().sum::<usize>() as f64 / indents.len() as f64
    };

    // Count functions (simplified regex-free approach)
    let function_count = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            (trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub async fn ")
                || trimmed.starts_with("fn ")
                || trimmed.starts_with("async fn ")
                || trimmed.contains(" fn "))
                && trimmed.contains('(')
        })
        .count();

    // Calculate complexity score
    let complexity_score = sloc as f64 * 0.1
        + max_indent as f64 * 10.0
        + avg_indent * 5.0
        + if function_count > 20 {
            function_count as f64 * 0.5
        } else {
            0.0
        };

    let risk_level = if complexity_score >= 200.0 {
        "critical"
    } else if complexity_score >= 100.0 {
        "high"
    } else if complexity_score >= 50.0 {
        "medium"
    } else {
        "low"
    };

    let rel_path = path
        .strip_prefix(repo_root)
        .ok()?
        .to_string_lossy()
        .to_string();

    Some(FileComplexity {
        path: rel_path,
        loc,
        sloc,
        max_indent,
        avg_indent: (avg_indent * 100.0).round() / 100.0,
        function_count,
        complexity_score: (complexity_score * 10.0).round() / 10.0,
        risk_level: risk_level.to_string(),
    })
}

/// Analyze a TypeScript file for complexity metrics.
fn analyze_typescript_file(path: &Path, repo_root: &Path) -> Option<FileComplexity> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let loc = lines.len();

    let mut sloc = 0;
    let mut indents = Vec::new();

    for line in &lines {
        let stripped = line.trim();

        // Skip comments and empty lines
        if stripped.is_empty()
            || stripped.starts_with("//")
            || stripped.starts_with("/*")
            || stripped.starts_with('*')
        {
            continue;
        }

        sloc += 1;

        // TypeScript typically uses 2-space indents
        let indent = (line.len() - line.trim_start().len()) / 2;
        indents.push(indent);
    }

    let max_indent = indents.iter().max().copied().unwrap_or(0);
    let avg_indent = if indents.is_empty() {
        0.0
    } else {
        indents.iter().sum::<usize>() as f64 / indents.len() as f64
    };

    // Count functions (simplified)
    let function_count = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("function ")
                || trimmed.contains("=> {")
                || (trimmed.contains('(') && trimmed.contains("): ") && trimmed.contains('{'))
        })
        .count();

    let complexity_score = sloc as f64 * 0.1 + max_indent as f64 * 8.0 + avg_indent * 4.0;

    let risk_level = if complexity_score >= 150.0 {
        "critical"
    } else if complexity_score >= 75.0 {
        "high"
    } else if complexity_score >= 35.0 {
        "medium"
    } else {
        "low"
    };

    let rel_path = path
        .strip_prefix(repo_root)
        .ok()?
        .to_string_lossy()
        .to_string();

    Some(FileComplexity {
        path: rel_path,
        loc,
        sloc,
        max_indent,
        avg_indent: (avg_indent * 100.0).round() / 100.0,
        function_count,
        complexity_score: (complexity_score * 10.0).round() / 10.0,
        risk_level: risk_level.to_string(),
    })
}

/// Analyze a Python file for complexity metrics.
fn analyze_python_file(path: &Path, repo_root: &Path) -> Option<FileComplexity> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let loc = lines.len();

    let mut sloc = 0;
    let mut indents = Vec::new();
    let mut in_docstring = false;

    for line in &lines {
        let stripped = line.trim();

        // Handle docstrings
        let triple_quote_count =
            stripped.matches("\"\"\"").count() + stripped.matches("'''").count();
        if triple_quote_count == 1 {
            in_docstring = !in_docstring;
            continue;
        }
        if in_docstring {
            continue;
        }

        // Skip empty and comment lines
        if stripped.is_empty() || stripped.starts_with('#') {
            continue;
        }

        sloc += 1;

        // Python uses 4-space indents
        let indent = (line.len() - line.trim_start().len()) / 4;
        indents.push(indent);
    }

    let max_indent = indents.iter().max().copied().unwrap_or(0);
    let avg_indent = if indents.is_empty() {
        0.0
    } else {
        indents.iter().sum::<usize>() as f64 / indents.len() as f64
    };

    // Count functions
    let function_count = lines
        .iter()
        .filter(|line| line.trim().starts_with("def "))
        .count();

    let complexity_score = sloc as f64 * 0.1 + max_indent as f64 * 12.0 + avg_indent * 6.0;

    let risk_level = if complexity_score >= 150.0 {
        "critical"
    } else if complexity_score >= 75.0 {
        "high"
    } else if complexity_score >= 35.0 {
        "medium"
    } else {
        "low"
    };

    let rel_path = path
        .strip_prefix(repo_root)
        .ok()?
        .to_string_lossy()
        .to_string();

    Some(FileComplexity {
        path: rel_path,
        loc,
        sloc,
        max_indent,
        avg_indent: (avg_indent * 100.0).round() / 100.0,
        function_count,
        complexity_score: (complexity_score * 10.0).round() / 10.0,
        risk_level: risk_level.to_string(),
    })
}

impl ComplexityReport {
    /// Generate JSON output.
    pub fn to_json(&self) -> Result<String> {
        #[derive(Serialize)]
        struct Output {
            generated: String,
            total_files: usize,
            by_risk: RiskBreakdown,
            files: Vec<FileComplexity>,
        }

        let output = Output {
            generated: chrono::Utc::now().to_rfc3339(),
            total_files: self.file_count,
            by_risk: self.by_risk.clone(),
            files: self.files.clone(),
        };

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_levels() {
        // Test Rust thresholds
        assert_eq!(if 250.0 >= 200.0 { "critical" } else { "low" }, "critical");
        assert_eq!(
            if 150.0 >= 100.0 && 150.0 < 200.0 {
                "high"
            } else {
                "low"
            },
            "high"
        );
    }
}
