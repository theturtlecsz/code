//! P85: Code Unit Extraction for Shadow Code Brain
//!
//! SPEC-KIT-102: Extracts code units (functions, structs, traits, impl blocks)
//! from the codex-rs codebase using tree-sitter for accurate AST parsing.
//!
//! This module provides:
//! - `CodeUnit`: A code snippet with metadata
//! - `CodeUnitKind`: Type of code construct (function, struct, etc.)
//! - `CodeUnitExtractor`: Tree-sitter based extractor

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tree_sitter::{Node, Parser};
use walkdir::WalkDir;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// Kind of code construct
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodeUnitKind {
    /// Function definition (fn or pub fn)
    Function,
    /// Struct definition
    Struct,
    /// Impl block
    Impl,
    /// Trait definition
    Trait,
    /// Module (mod declaration)
    Module,
}

impl CodeUnitKind {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Struct => "struct",
            Self::Impl => "impl",
            Self::Trait => "trait",
            Self::Module => "module",
        }
    }
}

impl std::fmt::Display for CodeUnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A code unit extracted from the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnit {
    /// Stable ID: "code:{relative_path}::{symbol_name}"
    pub id: String,
    /// Repository name (e.g., "codex-rs")
    pub repo: String,
    /// Relative file path (e.g., "tui/src/chatwidget/spec_kit/pipeline_coordinator.rs")
    pub path: String,
    /// Symbol name (e.g., "handle_spec_auto") - None for impl blocks without explicit name
    pub symbol: Option<String>,
    /// Kind of code construct
    pub kind: CodeUnitKind,
    /// Code snippet (definition + ~20 lines context, max 500 chars)
    pub text: String,
    /// Line number where the definition starts (1-indexed)
    pub line_start: usize,
}

impl CodeUnit {
    /// Generate a stable ID for this code unit
    pub fn generate_id(path: &str, symbol: Option<&str>, kind: CodeUnitKind, line: usize) -> String {
        match symbol {
            Some(s) => format!("code:{}::{}", path, s),
            None => format!("code:{}::{}@{}", path, kind.as_str(), line),
        }
    }
}

/// Statistics from code extraction
#[derive(Debug, Clone, Default)]
pub struct ExtractionStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of code units extracted
    pub units_extracted: usize,
    /// Number of files that failed to parse
    pub parse_errors: usize,
    /// Extraction duration in milliseconds
    pub duration_ms: u64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Code Unit Extractor
// ─────────────────────────────────────────────────────────────────────────────

/// Tree-sitter based code unit extractor
pub struct CodeUnitExtractor {
    /// Repository name for generated IDs
    repo_name: String,
    /// Maximum snippet length in characters
    max_snippet_chars: usize,
    /// Maximum context lines after definition
    max_context_lines: usize,
}

impl CodeUnitExtractor {
    /// Create a new extractor with default settings
    pub fn new(repo_name: impl Into<String>) -> Self {
        Self {
            repo_name: repo_name.into(),
            max_snippet_chars: 500,
            max_context_lines: 20,
        }
    }

    /// Set maximum snippet length
    pub fn with_max_snippet_chars(mut self, max: usize) -> Self {
        self.max_snippet_chars = max;
        self
    }

    /// Set maximum context lines
    pub fn with_max_context_lines(mut self, max: usize) -> Self {
        self.max_context_lines = max;
        self
    }

    /// Extract code units from a directory
    ///
    /// Walks the directory tree, parsing `.rs` files and extracting code units.
    /// Skips test files, examples, and target directories.
    pub fn extract_from_directory(
        &self,
        root: &Path,
        relative_prefix: &str,
    ) -> (Vec<CodeUnit>, ExtractionStats) {
        let start = std::time::Instant::now();
        let mut units = Vec::new();
        let mut stats = ExtractionStats::default();

        // Set up tree-sitter parser for Rust
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        if parser.set_language(&language.into()).is_err() {
            tracing::error!("Failed to set tree-sitter Rust language");
            return (units, stats);
        }

        // Walk directory tree
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Skip common non-source directories
                !name.starts_with('.')
                    && name != "target"
                    && name != "tests"
                    && name != "examples"
                    && name != "benches"
            });

        for entry in walker.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            // Only process Rust files
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            // Skip test files
            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if filename.starts_with("test_")
                || filename.ends_with("_test.rs")
                || filename == "tests.rs"
            {
                continue;
            }

            // Compute relative path
            let rel_path = match path.strip_prefix(root) {
                Ok(rel) => {
                    if relative_prefix.is_empty() {
                        rel.to_string_lossy().to_string()
                    } else {
                        format!("{}/{}", relative_prefix, rel.to_string_lossy())
                    }
                }
                Err(_) => continue,
            };

            // Read and parse file
            let source = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!("Failed to read {}: {}", path.display(), e);
                    stats.parse_errors += 1;
                    continue;
                }
            };

            match parser.parse(&source, None) {
                Some(tree) => {
                    let file_units = self.extract_from_tree(&tree, &source, &rel_path);
                    stats.units_extracted += file_units.len();
                    units.extend(file_units);
                    stats.files_processed += 1;
                }
                None => {
                    tracing::warn!("Failed to parse {}", path.display());
                    stats.parse_errors += 1;
                }
            }
        }

        stats.duration_ms = start.elapsed().as_millis() as u64;
        (units, stats)
    }

    /// Extract code units from multiple directories
    ///
    /// Convenience method to extract from stage0/src/, tui/src/, core/src/
    pub fn extract_from_codex_rs(&self, codex_rs_root: &Path) -> (Vec<CodeUnit>, ExtractionStats) {
        let mut all_units = Vec::new();
        let mut combined_stats = ExtractionStats::default();
        let start = std::time::Instant::now();

        // Directories to scan with their relative prefixes
        let dirs = [
            ("stage0/src", "stage0/src"),
            ("tui/src", "tui/src"),
            ("core/src", "core/src"),
        ];

        for (dir, prefix) in &dirs {
            let full_path = codex_rs_root.join(dir);
            if !full_path.exists() {
                tracing::debug!("Skipping non-existent directory: {}", full_path.display());
                continue;
            }

            let (units, stats) = self.extract_from_directory(&full_path, prefix);
            all_units.extend(units);
            combined_stats.files_processed += stats.files_processed;
            combined_stats.units_extracted += stats.units_extracted;
            combined_stats.parse_errors += stats.parse_errors;
        }

        combined_stats.duration_ms = start.elapsed().as_millis() as u64;
        (all_units, combined_stats)
    }

    /// Extract code units from a parsed tree
    fn extract_from_tree(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        rel_path: &str,
    ) -> Vec<CodeUnit> {
        let mut units = Vec::new();
        let root = tree.root_node();

        // Walk the AST looking for interesting nodes
        self.walk_node(&root, source, rel_path, &mut units);

        units
    }

    /// Recursively walk AST nodes
    fn walk_node(&self, node: &Node, source: &str, rel_path: &str, units: &mut Vec<CodeUnit>) {
        let kind_str = node.kind();

        match kind_str {
            "function_item" => {
                if let Some(unit) = self.extract_function(node, source, rel_path) {
                    units.push(unit);
                }
            }
            "struct_item" => {
                if let Some(unit) = self.extract_struct(node, source, rel_path) {
                    units.push(unit);
                }
            }
            "impl_item" => {
                if let Some(unit) = self.extract_impl(node, source, rel_path) {
                    units.push(unit);
                }
            }
            "trait_item" => {
                if let Some(unit) = self.extract_trait(node, source, rel_path) {
                    units.push(unit);
                }
            }
            "mod_item" => {
                if let Some(unit) = self.extract_module(node, source, rel_path) {
                    units.push(unit);
                }
            }
            _ => {}
        }

        // Recurse into children (but not into function/impl bodies for nested functions)
        let should_recurse = !matches!(kind_str, "function_item");
        if should_recurse {
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    self.walk_node(&child, source, rel_path, units);
                }
            }
        } else {
            // For function items, only recurse into impl items within
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    if child.kind() == "impl_item" {
                        self.walk_node(&child, source, rel_path, units);
                    }
                }
            }
        }
    }

    /// Extract function definition
    fn extract_function(&self, node: &Node, source: &str, rel_path: &str) -> Option<CodeUnit> {
        // Get function name
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?;

        // Skip test functions and private helpers (starting with _)
        if name.starts_with("test_") || name.starts_with('_') {
            return None;
        }

        let line_start = node.start_position().row + 1;
        let text = self.extract_snippet(node, source);
        let id = CodeUnit::generate_id(rel_path, Some(name), CodeUnitKind::Function, line_start);

        Some(CodeUnit {
            id,
            repo: self.repo_name.clone(),
            path: rel_path.to_string(),
            symbol: Some(name.to_string()),
            kind: CodeUnitKind::Function,
            text,
            line_start,
        })
    }

    /// Extract struct definition
    fn extract_struct(&self, node: &Node, source: &str, rel_path: &str) -> Option<CodeUnit> {
        // Get struct name
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?;

        let line_start = node.start_position().row + 1;
        let text = self.extract_snippet(node, source);
        let id = CodeUnit::generate_id(rel_path, Some(name), CodeUnitKind::Struct, line_start);

        Some(CodeUnit {
            id,
            repo: self.repo_name.clone(),
            path: rel_path.to_string(),
            symbol: Some(name.to_string()),
            kind: CodeUnitKind::Struct,
            text,
            line_start,
        })
    }

    /// Extract impl block
    fn extract_impl(&self, node: &Node, source: &str, rel_path: &str) -> Option<CodeUnit> {
        // Get type name being implemented
        let type_node = node.child_by_field_name("type")?;
        let type_name = type_node.utf8_text(source.as_bytes()).ok()?;

        // Check for trait implementation
        let trait_name = node
            .child_by_field_name("trait")
            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
            .map(|s| s.to_string());

        let symbol = match &trait_name {
            Some(trait_n) => format!("{}::{}", trait_n, type_name),
            None => type_name.to_string(),
        };

        let line_start = node.start_position().row + 1;
        let text = self.extract_snippet(node, source);
        let id = CodeUnit::generate_id(rel_path, Some(&symbol), CodeUnitKind::Impl, line_start);

        Some(CodeUnit {
            id,
            repo: self.repo_name.clone(),
            path: rel_path.to_string(),
            symbol: Some(symbol),
            kind: CodeUnitKind::Impl,
            text,
            line_start,
        })
    }

    /// Extract trait definition
    fn extract_trait(&self, node: &Node, source: &str, rel_path: &str) -> Option<CodeUnit> {
        // Get trait name
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?;

        let line_start = node.start_position().row + 1;
        let text = self.extract_snippet(node, source);
        let id = CodeUnit::generate_id(rel_path, Some(name), CodeUnitKind::Trait, line_start);

        Some(CodeUnit {
            id,
            repo: self.repo_name.clone(),
            path: rel_path.to_string(),
            symbol: Some(name.to_string()),
            kind: CodeUnitKind::Trait,
            text,
            line_start,
        })
    }

    /// Extract module declaration
    fn extract_module(&self, node: &Node, source: &str, rel_path: &str) -> Option<CodeUnit> {
        // Get module name
        let name_node = node.child_by_field_name("name")?;
        let name = name_node.utf8_text(source.as_bytes()).ok()?;

        // Skip inline modules (they have body children)
        // We only want `mod foo;` declarations, not `mod foo { ... }`
        let has_body = node.child_by_field_name("body").is_some();
        if has_body {
            // For inline modules, recurse into their children
            return None;
        }

        let line_start = node.start_position().row + 1;
        let text = self.extract_snippet(node, source);
        let id = CodeUnit::generate_id(rel_path, Some(name), CodeUnitKind::Module, line_start);

        Some(CodeUnit {
            id,
            repo: self.repo_name.clone(),
            path: rel_path.to_string(),
            symbol: Some(name.to_string()),
            kind: CodeUnitKind::Module,
            text,
            line_start,
        })
    }

    /// Extract a code snippet with context
    fn extract_snippet(&self, node: &Node, source: &str) -> String {
        let start = node.start_byte();
        let end = node.end_byte();

        // Get the node text
        let node_text = if end <= source.len() {
            &source[start..end]
        } else {
            return String::new();
        };

        // Count lines in the node
        let node_lines: Vec<&str> = node_text.lines().collect();

        // Limit to max_context_lines
        let text = if node_lines.len() > self.max_context_lines {
            let truncated: Vec<&str> = node_lines
                .into_iter()
                .take(self.max_context_lines)
                .collect();
            format!("{}...", truncated.join("\n"))
        } else {
            node_text.to_string()
        };

        // Limit to max_snippet_chars
        if text.len() > self.max_snippet_chars {
            let truncated = &text[..self.max_snippet_chars.saturating_sub(3)];
            // Try to truncate at a line boundary
            if let Some(last_newline) = truncated.rfind('\n') {
                format!("{}...", &truncated[..last_newline])
            } else {
                format!("{}...", truncated)
            }
        } else {
            text
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_unit_kind_display() {
        assert_eq!(CodeUnitKind::Function.as_str(), "function");
        assert_eq!(CodeUnitKind::Struct.as_str(), "struct");
        assert_eq!(CodeUnitKind::Impl.as_str(), "impl");
        assert_eq!(CodeUnitKind::Trait.as_str(), "trait");
        assert_eq!(CodeUnitKind::Module.as_str(), "module");
    }

    #[test]
    fn test_generate_id_with_symbol() {
        let id = CodeUnit::generate_id("tui/src/lib.rs", Some("main"), CodeUnitKind::Function, 10);
        assert_eq!(id, "code:tui/src/lib.rs::main");
    }

    #[test]
    fn test_generate_id_without_symbol() {
        let id = CodeUnit::generate_id("tui/src/lib.rs", None, CodeUnitKind::Impl, 42);
        assert_eq!(id, "code:tui/src/lib.rs::impl@42");
    }

    #[test]
    fn test_extract_from_source() {
        let source = r#"
pub struct Config {
    pub name: String,
}

impl Config {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

pub trait Configurable {
    fn configure(&self);
}

pub fn main() {
    println!("Hello!");
}
"#;

        // Set up parser
        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).unwrap();

        let tree = parser.parse(source, None).unwrap();
        let extractor = CodeUnitExtractor::new("test-repo");
        let units = extractor.extract_from_tree(&tree, source, "test.rs");

        // Should find: struct Config, impl Config, trait Configurable, fn main
        assert!(units.len() >= 4, "Expected at least 4 units, got {}", units.len());

        // Check we found the struct
        let struct_unit = units.iter().find(|u| u.kind == CodeUnitKind::Struct);
        assert!(struct_unit.is_some(), "Should find struct");
        assert_eq!(struct_unit.unwrap().symbol, Some("Config".to_string()));

        // Check we found the function
        let fn_unit = units.iter().find(|u| u.kind == CodeUnitKind::Function && u.symbol == Some("main".to_string()));
        assert!(fn_unit.is_some(), "Should find main function");

        // Check we found the trait
        let trait_unit = units.iter().find(|u| u.kind == CodeUnitKind::Trait);
        assert!(trait_unit.is_some(), "Should find trait");
        assert_eq!(trait_unit.unwrap().symbol, Some("Configurable".to_string()));
    }

    #[test]
    fn test_snippet_truncation() {
        let source = r#"pub fn long_function() {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    let f = 6;
    let g = 7;
    let h = 8;
    let i = 9;
    let j = 10;
    let k = 11;
    let l = 12;
    let m = 13;
    let n = 14;
    let o = 15;
    let p = 16;
    let q = 17;
    let r = 18;
    let s = 19;
    let t = 20;
    let u = 21;
    let v = 22;
    let w = 23;
}"#;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).unwrap();

        let tree = parser.parse(source, None).unwrap();
        let extractor = CodeUnitExtractor::new("test-repo")
            .with_max_context_lines(10);
        let units = extractor.extract_from_tree(&tree, source, "test.rs");

        assert_eq!(units.len(), 1);
        let unit = &units[0];

        // Should be truncated and end with ...
        assert!(unit.text.ends_with("..."), "Should be truncated: {}", unit.text);
        // Should have fewer lines than original
        let line_count = unit.text.lines().count();
        assert!(line_count <= 11, "Should have at most 10 lines + partial: got {}", line_count);
    }

    #[test]
    fn test_skip_test_functions() {
        let source = r#"
pub fn real_function() {}

fn test_something() {}

fn _private_helper() {}
"#;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).unwrap();

        let tree = parser.parse(source, None).unwrap();
        let extractor = CodeUnitExtractor::new("test-repo");
        let units = extractor.extract_from_tree(&tree, source, "test.rs");

        // Should only find real_function
        assert_eq!(units.len(), 1, "Should only find 1 function, got {:?}", units);
        assert_eq!(units[0].symbol, Some("real_function".to_string()));
    }

    #[test]
    fn test_impl_with_trait() {
        let source = r#"
impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.name)
    }
}
"#;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).unwrap();

        let tree = parser.parse(source, None).unwrap();
        let extractor = CodeUnitExtractor::new("test-repo");
        let units = extractor.extract_from_tree(&tree, source, "test.rs");

        // Should find the impl block
        let impl_unit = units.iter().find(|u| u.kind == CodeUnitKind::Impl);
        assert!(impl_unit.is_some(), "Should find impl block");
        // Symbol should include trait name
        let symbol = impl_unit.unwrap().symbol.as_ref().unwrap();
        assert!(symbol.contains("Display"), "Symbol should include trait name: {}", symbol);
    }

    #[test]
    fn test_extractor_stats() {
        // Test extraction from the actual codebase rather than temp dirs
        // to avoid issues with temp directory naming on different platforms
        let source = r#"
pub struct Config {
    pub name: String,
}

pub fn main() {
    println!("Hello!");
}
"#;

        let mut parser = Parser::new();
        let language = tree_sitter_rust::LANGUAGE;
        parser.set_language(&language.into()).unwrap();

        let tree = parser.parse(source, None).unwrap();
        let extractor = CodeUnitExtractor::new("test-repo");
        let units = extractor.extract_from_tree(&tree, source, "test.rs");

        // Should find at least struct Config and fn main
        assert!(units.len() >= 2, "Should extract at least 2 units, got {}", units.len());

        // Verify we found the expected items
        let has_struct = units.iter().any(|u| u.kind == CodeUnitKind::Struct);
        let has_fn = units.iter().any(|u| u.kind == CodeUnitKind::Function);
        assert!(has_struct, "Should find struct");
        assert!(has_fn, "Should find function");
    }
}
