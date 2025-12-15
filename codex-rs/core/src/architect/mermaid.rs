//! Mermaid.js diagram generation for code visualization.
//!
//! Generates Mermaid flowcharts from Rust source code using tree-sitter parsing.
//! Unlike CodeGraphContext (Python only), this works natively with Rust codebases.
//!
//! # Output Formats
//!
//! - **Call Graph**: Function call relationships as a flowchart
//! - **Module Dependencies**: Module-level import relationships
//!
//! # Usage
//!
//! ```ignore
//! let graph = extract_call_graph(repo_root)?;
//! let mermaid = graph.to_mermaid();
//! std::fs::write("call_graph.mmd", mermaid)?;
//! ```

use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use walkdir::WalkDir;

/// A call graph extracted from Rust source code.
#[derive(Debug, Default)]
pub struct CallGraph {
    /// Map of function name to its defined location
    pub functions: HashMap<String, FunctionNode>,
    /// Set of (caller, callee) edges
    pub calls: HashSet<(String, String)>,
    /// Total files processed
    pub file_count: usize,
}

/// A function node in the call graph.
#[derive(Debug, Clone)]
pub struct FunctionNode {
    /// Function name
    pub name: String,
    /// File path where defined
    pub file_path: String,
    /// Module path (e.g., "core::architect::mermaid")
    pub module_path: String,
    /// Whether it's a public function
    pub is_public: bool,
    /// Whether it's an async function
    pub is_async: bool,
}

/// Module dependency information.
#[derive(Debug, Default)]
pub struct ModuleDeps {
    /// Map of module path to its dependencies
    pub modules: HashMap<String, ModuleNode>,
    /// Set of (importer, imported) edges
    pub imports: HashSet<(String, String)>,
}

/// A module node in the dependency graph.
#[derive(Debug, Clone)]
pub struct ModuleNode {
    /// Module path
    pub path: String,
    /// File path
    pub file_path: String,
    /// Is this a re-export?
    pub is_reexport: bool,
}

/// Extract call graph from a Rust repository.
pub fn extract_call_graph(repo_root: &Path) -> Result<CallGraph> {
    let codex_rs = repo_root.join("codex-rs");
    let mut graph = CallGraph::default();

    let target_dirs = [
        codex_rs.join("core/src"),
        codex_rs.join("tui/src"),
        codex_rs.join("cli/src"),
    ];

    for target_dir in &target_dirs {
        if !target_dir.exists() {
            continue;
        }

        for entry in WalkDir::new(target_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            // Skip target directory
            let path_str = path.to_string_lossy();
            if path_str.contains("/target/") {
                continue;
            }

            extract_file_calls(path, &codex_rs, &mut graph)?;
            graph.file_count += 1;
        }
    }

    Ok(graph)
}

/// Extract function definitions and calls from a single Rust file.
fn extract_file_calls(path: &Path, codex_root: &Path, graph: &mut CallGraph) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

    let tree = match parser.parse(&content, None) {
        Some(t) => t,
        None => return Ok(()),
    };

    let rel_path = path
        .strip_prefix(codex_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    // Derive module path from file path
    let module_path = derive_module_path(&rel_path);

    // Query for function definitions
    let fn_query = r#"
        (function_item
            (visibility_modifier)? @vis
            "async"? @async
            name: (identifier) @name
        ) @func
    "#;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), fn_query)?;
    let mut cursor = QueryCursor::new();
    let root = tree.root_node();

    // First pass: collect function definitions
    let mut matches = cursor.matches(&query, root, content.as_bytes());
    while let Some(m) = matches.next() {
        let mut fn_name = None;
        let mut is_public = false;
        let mut is_async = false;

        for capture in m.captures.iter() {
            let capture_name = query.capture_names()[capture.index as usize];
            let text = capture.node.utf8_text(content.as_bytes()).unwrap_or("");

            match capture_name {
                "name" => fn_name = Some(text.to_string()),
                "vis" => is_public = text.starts_with("pub"),
                "async" => is_async = text == "async",
                _ => {}
            }
        }

        if let Some(name) = fn_name {
            let qualified = format!("{module_path}::{name}");
            graph.functions.insert(
                qualified.clone(),
                FunctionNode {
                    name,
                    file_path: rel_path.clone(),
                    module_path: module_path.clone(),
                    is_public,
                    is_async,
                },
            );
        }
    }

    // Second pass: extract function calls
    let call_query = r#"
        (call_expression
            function: [
                (identifier) @fn_name
                (field_expression
                    field: (field_identifier) @method_name
                )
                (scoped_identifier
                    name: (identifier) @scoped_name
                )
            ]
        )
    "#;

    let call_q = Query::new(&tree_sitter_rust::LANGUAGE.into(), call_query)?;
    let mut call_cursor = QueryCursor::new();

    // Find the current function context for each call
    let ctx_query = r#"
        (function_item
            name: (identifier) @ctx_fn
            body: (block) @body
        )
    "#;

    let ctx_q = Query::new(&tree_sitter_rust::LANGUAGE.into(), ctx_query)?;
    let mut ctx_cursor = QueryCursor::new();
    let mut ctx_matches = ctx_cursor.matches(&ctx_q, root, content.as_bytes());

    while let Some(ctx_m) = ctx_matches.next() {
        let mut ctx_fn_name = None;
        let mut body_node = None;

        for capture in ctx_m.captures.iter() {
            let capture_name = ctx_q.capture_names()[capture.index as usize];
            match capture_name {
                "ctx_fn" => {
                    ctx_fn_name = Some(
                        capture
                            .node
                            .utf8_text(content.as_bytes())
                            .unwrap_or("")
                            .to_string(),
                    )
                }
                "body" => body_node = Some(capture.node),
                _ => {}
            }
        }

        let (Some(caller_name), Some(body)) = (ctx_fn_name, body_node) else {
            continue;
        };

        let caller_qualified = format!("{module_path}::{caller_name}");

        // Find calls within this function body
        let mut call_matches = call_cursor.matches(&call_q, body, content.as_bytes());
        while let Some(call_m) = call_matches.next() {
            for capture in call_m.captures.iter() {
                let callee = capture
                    .node
                    .utf8_text(content.as_bytes())
                    .unwrap_or("")
                    .to_string();

                if !callee.is_empty() && callee != caller_name {
                    // Try to resolve to qualified name or use as-is
                    let callee_qualified = if callee.contains("::") {
                        callee
                    } else {
                        // Check if it's defined in the same module
                        let same_module = format!("{module_path}::{callee}");
                        if graph.functions.contains_key(&same_module) {
                            same_module
                        } else {
                            callee
                        }
                    };

                    graph
                        .calls
                        .insert((caller_qualified.clone(), callee_qualified));
                }
            }
        }
    }

    Ok(())
}

/// Derive module path from file path.
fn derive_module_path(file_path: &str) -> String {
    // Convert "core/src/architect/mermaid.rs" to "core::architect::mermaid"
    let path = file_path
        .trim_end_matches(".rs")
        .replace("/src/", "::")
        .replace('/', "::");

    // Handle lib.rs and mod.rs
    if path.ends_with("::lib") || path.ends_with("::mod") {
        path.rsplit_once("::")
            .map(|(p, _)| p)
            .unwrap_or(&path)
            .to_string()
    } else {
        path
    }
}

impl CallGraph {
    /// Generate Mermaid flowchart syntax.
    pub fn to_mermaid(&self) -> String {
        let mut lines = vec![
            "%%{ init: { 'flowchart': { 'curve': 'basis' } } }%%".to_string(),
            "flowchart LR".to_string(),
            format!(
                "    %% Generated from {} files, {} functions, {} call edges",
                self.file_count,
                self.functions.len(),
                self.calls.len()
            ),
            String::new(),
        ];

        // Group functions by module for subgraphs
        let mut by_module: HashMap<&str, Vec<&str>> = HashMap::new();
        for (qualified, node) in &self.functions {
            by_module
                .entry(&node.module_path)
                .or_default()
                .push(qualified);
        }

        // Sort modules for consistent output
        let mut modules: Vec<_> = by_module.keys().collect();
        modules.sort();

        // Generate subgraphs
        for module in modules {
            let safe_id = sanitize_mermaid_id(module);
            lines.push(format!("    subgraph {safe_id} [\"📦 {module}\"]"));

            let mut fns = by_module[module].clone();
            fns.sort();

            for qualified in fns {
                let node = &self.functions[qualified];
                let fn_id = sanitize_mermaid_id(qualified);
                let icon = if node.is_async { "⚡" } else { "fn" };
                let style = if node.is_public { "pub" } else { "priv" };
                lines.push(format!(
                    "        {}[\"{}:{} {}\"]",
                    fn_id, icon, style, node.name
                ));
            }

            lines.push("    end".to_string());
        }

        lines.push(String::new());

        // Generate edges (limit to avoid overwhelming diagrams)
        let mut edge_count = 0;
        const MAX_EDGES: usize = 200;

        // Prioritize edges where both endpoints are known functions
        let mut prioritized: Vec<_> = self
            .calls
            .iter()
            .map(|(from, to)| {
                let known_to = self.functions.contains_key(to);
                (known_to, from, to)
            })
            .collect();
        prioritized.sort_by_key(|(known, _, _)| std::cmp::Reverse(*known));

        for (_, from, to) in prioritized {
            if edge_count >= MAX_EDGES {
                lines.push(format!(
                    "    %% ... and {} more edges",
                    self.calls.len() - MAX_EDGES
                ));
                break;
            }

            let from_id = sanitize_mermaid_id(from);
            let to_id = sanitize_mermaid_id(to);

            // Only show edges where the target is a known function
            if self.functions.contains_key(to) {
                lines.push(format!("    {from_id} --> {to_id}"));
                edge_count += 1;
            }
        }

        lines.join("\n")
    }

    /// Generate a focused Mermaid diagram for a specific function and its neighbors.
    pub fn to_mermaid_focused(&self, target: &str, depth: usize) -> String {
        let mut included: HashSet<String> = HashSet::new();
        let mut frontier: HashSet<String> = HashSet::new();

        // Find functions matching the target
        for qualified in self.functions.keys() {
            if qualified.ends_with(&format!("::{target}")) || qualified == target {
                frontier.insert(qualified.clone());
            }
        }

        if frontier.is_empty() {
            return format!(
                "flowchart LR\n    error[\"Function '{target}' not found\"]"
            );
        }

        // BFS to find neighbors up to depth
        for _ in 0..depth {
            let mut next_frontier = HashSet::new();

            for node in &frontier {
                if included.insert(node.clone()) {
                    // Find callers
                    for (from, to) in &self.calls {
                        if to == node && !included.contains(from) {
                            next_frontier.insert(from.clone());
                        }
                    }
                    // Find callees
                    for (from, to) in &self.calls {
                        if from == node && !included.contains(to) {
                            next_frontier.insert(to.clone());
                        }
                    }
                }
            }

            frontier = next_frontier;
        }

        // Add final frontier layer
        included.extend(frontier);

        // Generate focused diagram
        let mut lines = vec![
            "flowchart LR".to_string(),
            format!("    %% Focused view: {} (depth {})", target, depth),
        ];

        // Nodes
        for qualified in &included {
            let fn_id = sanitize_mermaid_id(qualified);
            if let Some(node) = self.functions.get(qualified) {
                let icon = if node.is_async { "⚡" } else { "fn" };
                lines.push(format!("    {}[\"{}:{}\"]", fn_id, icon, node.name));
            } else {
                // External function
                lines.push(format!("    {fn_id}([\"🔗 {qualified}\"])"));
            }
        }

        // Edges
        for (from, to) in &self.calls {
            if included.contains(from) && included.contains(to) {
                let from_id = sanitize_mermaid_id(from);
                let to_id = sanitize_mermaid_id(to);
                lines.push(format!("    {from_id} --> {to_id}"));
            }
        }

        lines.join("\n")
    }
}

/// Sanitize a string for use as a Mermaid node ID.
fn sanitize_mermaid_id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect()
}

/// Extract module dependencies from a Rust repository.
pub fn extract_module_deps(repo_root: &Path) -> Result<ModuleDeps> {
    let codex_rs = repo_root.join("codex-rs");
    let mut deps = ModuleDeps::default();

    let target_dirs = [
        codex_rs.join("core/src"),
        codex_rs.join("tui/src"),
        codex_rs.join("cli/src"),
    ];

    for target_dir in &target_dirs {
        if !target_dir.exists() {
            continue;
        }

        for entry in WalkDir::new(target_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let path = entry.path();

            if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            let path_str = path.to_string_lossy();
            if path_str.contains("/target/") {
                continue;
            }

            extract_file_imports(path, &codex_rs, &mut deps)?;
        }
    }

    Ok(deps)
}

/// Extract imports from a single Rust file.
fn extract_file_imports(path: &Path, codex_root: &Path, deps: &mut ModuleDeps) -> Result<()> {
    let content = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

    let tree = match parser.parse(&content, None) {
        Some(t) => t,
        None => return Ok(()),
    };

    let rel_path = path
        .strip_prefix(codex_root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let module_path = derive_module_path(&rel_path);

    deps.modules.insert(
        module_path.clone(),
        ModuleNode {
            path: module_path.clone(),
            file_path: rel_path,
            is_reexport: false,
        },
    );

    // Query for use statements
    let use_query = r#"
        (use_declaration
            argument: [
                (scoped_identifier) @import
                (use_wildcard) @wildcard
                (scoped_use_list) @list
                (identifier) @ident
            ]
        )
    "#;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), use_query)?;
    let mut cursor = QueryCursor::new();
    let root = tree.root_node();

    let mut matches = cursor.matches(&query, root, content.as_bytes());
    while let Some(m) = matches.next() {
        for capture in m.captures.iter() {
            let import_text = capture
                .node
                .utf8_text(content.as_bytes())
                .unwrap_or("")
                .to_string();

            if !import_text.is_empty() {
                // Extract the base module (before ::*)
                let base = import_text
                    .split("::{")
                    .next()
                    .unwrap_or(&import_text)
                    .trim_end_matches("::*");

                // Only track internal dependencies (crate::, super::, or local modules)
                if base.starts_with("crate::")
                    || base.starts_with("super::")
                    || base.starts_with("core::")
                    || base.starts_with("tui::")
                    || base.starts_with("cli::")
                {
                    deps.imports.insert((module_path.clone(), base.to_string()));
                }
            }
        }
    }

    Ok(())
}

impl ModuleDeps {
    /// Generate Mermaid flowchart for module dependencies.
    pub fn to_mermaid(&self) -> String {
        let mut lines = vec![
            "flowchart TD".to_string(),
            format!(
                "    %% Module dependencies: {} modules, {} imports",
                self.modules.len(),
                self.imports.len()
            ),
        ];

        // Nodes
        for path in self.modules.keys() {
            let id = sanitize_mermaid_id(path);
            lines.push(format!("    {id}[\"📁 {path}\"]"));
        }

        // Edges
        for (from, to) in &self.imports {
            let from_id = sanitize_mermaid_id(from);
            let to_id = sanitize_mermaid_id(to);
            lines.push(format!("    {from_id} -.-> {to_id}"));
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_module_path() {
        assert_eq!(
            derive_module_path("core/src/architect/mermaid.rs"),
            "core::architect::mermaid"
        );
        assert_eq!(derive_module_path("tui/src/lib.rs"), "tui");
        assert_eq!(derive_module_path("cli/src/main.rs"), "cli::main");
    }

    #[test]
    fn test_sanitize_mermaid_id() {
        assert_eq!(sanitize_mermaid_id("foo::bar"), "foo__bar");
        assert_eq!(sanitize_mermaid_id("my-fn"), "my_fn");
    }

    #[test]
    fn test_empty_call_graph_to_mermaid() {
        let graph = CallGraph::default();
        let mermaid = graph.to_mermaid();
        assert!(mermaid.contains("flowchart LR"));
        assert!(mermaid.contains("0 functions"));
    }
}
