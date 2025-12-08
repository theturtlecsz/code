//! Bridge to CodeGraphContext MCP server for semantic code analysis.
//!
//! Provides async interface to query the Neo4j code graph for:
//! - Call graph analysis (Python only)
//! - Dead code detection (Python only)
//! - Complexity metrics from indexed code (Python only)
//! - Module dependencies (Python only)
//!
//! # Language Support
//!
//! **Important**: CodeGraphContext currently only parses Python files.
//! For Rust analysis, use the native `skeleton.rs` and `mermaid.rs` modules instead.
//!
//! This bridge is useful for mixed-language repos or Python-heavy projects.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::debug;

/// Results from the code graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub success: bool,
    pub data: serde_json::Value,
    pub error: Option<String>,
}

/// Interface to CodeGraphContext MCP server.
///
/// # Architecture Note
///
/// MCP (Model Context Protocol) tools are designed to be invoked by AI assistants
/// like Claude, not directly from Rust code. This bridge provides:
///
/// 1. **Data structures** that match CodeGraphContext MCP responses
/// 2. **Helper methods** for formatting queries and parsing results
/// 3. **CLI integration** via the `cgc` helper script (if available)
///
/// For direct Rust code analysis (especially for Rust files), use the native
/// `mermaid.rs` module which uses tree-sitter parsing.
pub struct GraphBridge {
    /// Whether the MCP server appears configured
    available: bool,
    /// Path to cgc helper script (if found)
    cgc_path: Option<std::path::PathBuf>,
}

impl GraphBridge {
    /// Create a new graph bridge, checking MCP availability.
    pub fn new() -> Self {
        // Check if MCP server is configured by looking for the Neo4j connection
        let neo4j_configured = std::env::var("NEO4J_URI").is_ok();
        let claude_config = dirs::home_dir()
            .map(|h| h.join(".config/claude/claude_desktop_config.json"))
            .filter(|p| p.exists());

        // Look for cgc helper script in common locations
        let cgc_path = dirs::home_dir()
            .map(|h| h.join(".local/bin/cgc"))
            .filter(|p| p.exists());

        let available = neo4j_configured || claude_config.is_some();

        debug!(
            "GraphBridge: neo4j={}, claude_config={}, cgc={}",
            neo4j_configured,
            claude_config.is_some(),
            cgc_path.is_some()
        );

        Self {
            available,
            cgc_path,
        }
    }

    /// Check if the graph bridge is available.
    pub fn is_available(&self) -> bool {
        self.available
    }

    /// Check if the cgc CLI helper is available for direct queries.
    pub fn has_cli(&self) -> bool {
        self.cgc_path.is_some()
    }

    /// Find code related to a query (Python files only).
    ///
    /// For Rust code, use `skeleton::extract()` instead.
    pub async fn find_code(&self, query: &str) -> Result<Vec<CodeSnippet>> {
        if !self.available {
            debug!("GraphBridge not available, returning empty results");
            return Ok(Vec::new());
        }

        // Try cgc CLI if available
        if let Some(ref cgc) = self.cgc_path {
            let output = Command::new(cgc)
                .args(["find", query])
                .output()
                .context("Failed to run cgc find")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(snippets) = serde_json::from_str::<Vec<CodeSnippet>>(&stdout) {
                    return Ok(snippets);
                }
            }
        }

        // Without CLI, return empty - MCP integration happens at AI layer
        debug!("No cgc CLI, MCP integration deferred to AI layer");
        Ok(Vec::new())
    }

    /// Analyze code relationships (Python files only).
    ///
    /// For Rust code, use `mermaid::extract_call_graph()` instead.
    pub async fn analyze_relationships(
        &self,
        target: &str,
        query_type: RelationshipQuery,
    ) -> Result<RelationshipResult> {
        if !self.available {
            return Ok(RelationshipResult::default());
        }

        let query_type_str = query_type.as_str();

        // Try cgc CLI if available
        if let Some(ref cgc) = self.cgc_path {
            let output = Command::new(cgc)
                .args(["analyze", query_type_str, target])
                .output()
                .context("Failed to run cgc analyze")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(entries) = serde_json::from_str::<Vec<RelationshipEntry>>(&stdout) {
                    return Ok(RelationshipResult {
                        target: target.to_string(),
                        query_type,
                        results: entries,
                    });
                }
            }
        }

        Ok(RelationshipResult {
            target: target.to_string(),
            query_type,
            results: Vec::new(),
        })
    }

    /// Find dead code across the indexed codebase (Python files only).
    pub async fn find_dead_code(&self, exclude_decorators: &[&str]) -> Result<Vec<DeadCodeEntry>> {
        if !self.available {
            return Ok(Vec::new());
        }

        if let Some(ref cgc) = self.cgc_path {
            let mut args = vec!["dead-code".to_string()];
            for dec in exclude_decorators {
                args.push("--exclude".to_string());
                args.push(dec.to_string());
            }

            let output = Command::new(cgc)
                .args(&args)
                .output()
                .context("Failed to run cgc dead-code")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(entries) = serde_json::from_str(&stdout) {
                    return Ok(entries);
                }
            }
        }

        Ok(Vec::new())
    }

    /// Get complexity metrics for a function (Python files only).
    pub async fn get_complexity(&self, function_name: &str) -> Result<Option<ComplexityInfo>> {
        if !self.available {
            return Ok(None);
        }

        if let Some(ref cgc) = self.cgc_path {
            let output = Command::new(cgc)
                .args(["complexity", function_name])
                .output()
                .context("Failed to run cgc complexity")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(info) = serde_json::from_str(&stdout) {
                    return Ok(Some(info));
                }
            }
        }

        Ok(None)
    }

    /// Find the most complex functions in the codebase (Python files only).
    pub async fn find_most_complex(&self, limit: usize) -> Result<Vec<ComplexityInfo>> {
        if !self.available {
            return Ok(Vec::new());
        }

        if let Some(ref cgc) = self.cgc_path {
            let output = Command::new(cgc)
                .args(["most-complex", "--limit", &limit.to_string()])
                .output()
                .context("Failed to run cgc most-complex")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(entries) = serde_json::from_str(&stdout) {
                    return Ok(entries);
                }
            }
        }

        Ok(Vec::new())
    }

    /// Execute a raw Cypher query (for advanced use cases).
    pub async fn execute_cypher(&self, query: &str) -> Result<serde_json::Value> {
        if !self.available {
            return Ok(serde_json::Value::Null);
        }

        if let Some(ref cgc) = self.cgc_path {
            let output = Command::new(cgc)
                .args(["cypher", query])
                .output()
                .context("Failed to run cgc cypher")?;

            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Ok(value) = serde_json::from_str(&stdout) {
                    return Ok(value);
                }
            }
        }

        Ok(serde_json::Value::Null)
    }
}

impl Default for GraphBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// A code snippet from the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSnippet {
    pub file_path: String,
    pub function_name: Option<String>,
    pub class_name: Option<String>,
    pub code: String,
    pub line_start: usize,
    pub line_end: usize,
}

/// Type of relationship query.
#[derive(Debug, Clone, Copy)]
pub enum RelationshipQuery {
    FindCallers,
    FindCallees,
    FindAllCallers,
    FindAllCallees,
    FindImporters,
    ClassHierarchy,
    ModuleDeps,
}

impl RelationshipQuery {
    /// Get the MCP parameter string for this query type.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FindCallers => "find_callers",
            Self::FindCallees => "find_callees",
            Self::FindAllCallers => "find_all_callers",
            Self::FindAllCallees => "find_all_callees",
            Self::FindImporters => "find_importers",
            Self::ClassHierarchy => "class_hierarchy",
            Self::ModuleDeps => "module_deps",
        }
    }
}

/// Result of a relationship query.
#[derive(Debug, Clone, Default)]
pub struct RelationshipResult {
    pub target: String,
    pub query_type: RelationshipQuery,
    pub results: Vec<RelationshipEntry>,
}

impl Default for RelationshipQuery {
    fn default() -> Self {
        Self::FindCallers
    }
}

/// A single relationship entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipEntry {
    pub source: String,
    pub target: String,
    pub relationship_type: String,
    pub file_path: Option<String>,
}

/// Dead code entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadCodeEntry {
    pub name: String,
    pub kind: String, // "function", "class", "method"
    pub file_path: String,
    pub line: usize,
}

/// Complexity information for a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityInfo {
    pub function_name: String,
    pub file_path: String,
    pub cyclomatic_complexity: usize,
    pub line_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_bridge_creation() {
        let bridge = GraphBridge::new();
        // Just verify it doesn't panic
        let _ = bridge.is_available();
    }

    #[tokio::test]
    async fn test_find_code_empty() {
        let bridge = GraphBridge {
            available: false, // Force unavailable for test
            cgc_path: None,
        };
        let results = bridge.find_code("test").await.unwrap();
        assert!(results.is_empty());
    }
}
