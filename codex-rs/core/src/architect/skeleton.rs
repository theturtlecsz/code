//! Repository skeleton extraction using tree-sitter.
//!
//! Extracts public API surface from Rust, TypeScript, and Python files.

use anyhow::Result;
use std::path::Path;
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};
use walkdir::WalkDir;

/// Skeleton extraction results.
#[derive(Debug)]
pub struct SkeletonReport {
    /// Modules containing file declarations
    pub modules: Vec<ModuleSkeleton>,
    /// Total files processed
    pub file_count: usize,
    /// Total declarations extracted
    pub declaration_count: usize,
}

#[derive(Debug)]
pub struct ModuleSkeleton {
    pub name: String,
    pub files: Vec<FileSkeleton>,
}

#[derive(Debug)]
pub struct FileSkeleton {
    pub path: String,
    pub declarations: Vec<String>,
}

/// Extract public API skeleton from repository.
pub fn extract(repo_root: &Path) -> Result<SkeletonReport> {
    let codex_rs = repo_root.join("codex-rs");

    let target_dirs = vec![
        ("core", codex_rs.join("core")),
        ("tui", codex_rs.join("tui")),
        ("cli", codex_rs.join("cli")),
    ];

    let mut modules = Vec::new();
    let mut total_files = 0;
    let mut total_decls = 0;

    for (module_name, target_dir) in target_dirs {
        if !target_dir.exists() {
            continue;
        }

        let mut files = Vec::new();

        for entry in WalkDir::new(&target_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            // Only process .rs files
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }

            // Skip test files and target directory
            let path_str = path.to_string_lossy();
            if path_str.contains("/target/") || path_str.contains("/tests/") {
                continue;
            }

            let declarations = extract_rust_declarations(path)?;
            if declarations.is_empty() {
                continue;
            }

            total_files += 1;
            total_decls += declarations.len();

            let rel_path = path
                .strip_prefix(&codex_rs)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            files.push(FileSkeleton {
                path: rel_path,
                declarations,
            });
        }

        // Sort files by path
        files.sort_by(|a, b| a.path.cmp(&b.path));

        if !files.is_empty() {
            modules.push(ModuleSkeleton {
                name: module_name.to_string(),
                files,
            });
        }
    }

    Ok(SkeletonReport {
        modules,
        file_count: total_files,
        declaration_count: total_decls,
    })
}

/// Extract public API declarations from a Rust file using tree-sitter.
fn extract_rust_declarations(path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(path)?;
    let mut parser = Parser::new();

    parser.set_language(&tree_sitter_rust::LANGUAGE.into())?;

    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse"))?;

    let mut declarations = Vec::new();

    // Query for public items
    let query_source = r#"
        (function_item
            (visibility_modifier)? @vis
            name: (identifier) @name
        ) @func

        (struct_item
            (visibility_modifier)? @vis
            name: (type_identifier) @name
        ) @struct

        (enum_item
            (visibility_modifier)? @vis
            name: (type_identifier) @name
        ) @enum

        (trait_item
            (visibility_modifier)? @vis
            name: (type_identifier) @name
        ) @trait

        (impl_item) @impl

        (mod_item
            (visibility_modifier)? @vis
            name: (identifier) @name
        ) @mod

        (type_item
            (visibility_modifier)? @vis
            name: (type_identifier) @name
        ) @type

        (const_item
            (visibility_modifier)? @vis
            name: (identifier) @name
        ) @const
    "#;

    let query = Query::new(&tree_sitter_rust::LANGUAGE.into(), query_source)?;
    let mut cursor = QueryCursor::new();

    let root = tree.root_node();
    let mut matches = cursor.matches(&query, root, content.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures.iter() {
            let node = capture.node;
            let capture_name = query.capture_names()[capture.index as usize];

            // Only process full item captures (not sub-captures like @vis, @name)
            if matches!(
                capture_name,
                "func" | "struct" | "enum" | "trait" | "impl" | "mod" | "type" | "const"
            ) {
                let start = node.start_byte();
                let text = &content[start..];

                // Extract declaration (up to opening brace or semicolon)
                let decl = extract_declaration_head(text);

                // Check if public (has visibility modifier or is an impl)
                let is_public = capture_name == "impl"
                    || text.trim_start().starts_with("pub")
                    || node
                        .child_by_field_name("visibility")
                        .map(|v| v.utf8_text(content.as_bytes()).unwrap_or("").starts_with("pub"))
                        .unwrap_or(false);

                if is_public || capture_name == "impl" {
                    declarations.push(decl);
                }
            }
        }
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    declarations.retain(|d| seen.insert(d.clone()));

    Ok(declarations)
}

/// Extract the declaration head (up to { or ;), cleaned up.
fn extract_declaration_head(text: &str) -> String {
    // Find the first { or ; or newline with no continuation
    let mut depth = 0;
    let mut in_string = false;
    let mut end_pos = text.len().min(300); // Max 300 chars

    for (i, c) in text.char_indices() {
        if i >= end_pos {
            break;
        }

        match c {
            '"' if !in_string => in_string = true,
            '"' if in_string => in_string = false,
            '<' if !in_string => depth += 1,
            '>' if !in_string && depth > 0 => depth -= 1,
            '{' if !in_string && depth == 0 => {
                end_pos = i;
                break;
            }
            ';' if !in_string && depth == 0 => {
                end_pos = i;
                break;
            }
            _ => {}
        }
    }

    let decl = &text[..end_pos];

    // Clean up: collapse whitespace, remove doc comments
    let cleaned: String = decl
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.starts_with("///") && !l.starts_with("//!") && !l.starts_with("#["))
        .collect::<Vec<_>>()
        .join(" ");

    // Collapse multiple spaces
    let mut result = String::new();
    let mut prev_space = false;
    for c in cleaned.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
            }
            prev_space = true;
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    // Truncate if still too long
    if result.len() > 200 {
        format!("{}...", &result[..197])
    } else {
        result.trim().to_string()
    }
}

impl SkeletonReport {
    /// Generate XML output.
    pub fn to_xml(&self) -> String {
        let mut lines = vec![
            r#"<?xml version="1.0" encoding="UTF-8"?>"#.to_string(),
            format!(
                "<!-- Repository Skeleton - Generated {} -->",
                chrono::Utc::now().to_rfc3339()
            ),
            format!(
                "<!-- Summary: {} files, {} declarations -->",
                self.file_count, self.declaration_count
            ),
            r#"<repository name="codex-rs">"#.to_string(),
        ];

        for module in &self.modules {
            lines.push(format!(r#"  <module name="{}">"#, module.name));

            for file in &module.files {
                lines.push(format!(r#"    <file path="{}">"#, file.path));

                for decl in &file.declarations {
                    let escaped = escape_xml(decl);
                    lines.push(format!("      <decl>{}</decl>", escaped));
                }

                lines.push("    </file>".to_string());
            }

            lines.push("  </module>".to_string());
        }

        lines.push("</repository>".to_string());
        lines.join("\n")
    }
}

/// Escape XML special characters.
fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("a < b"), "a &lt; b");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml(r#"a "b" c"#), "a &quot;b&quot; c");
    }

    #[test]
    fn test_extract_declaration_head() {
        let decl = extract_declaration_head("pub fn foo(x: i32) -> bool {");
        assert_eq!(decl, "pub fn foo(x: i32) -> bool");

        let decl = extract_declaration_head("pub struct Foo<T> {");
        assert_eq!(decl, "pub struct Foo<T>");

        let decl = extract_declaration_head("impl Foo for Bar {");
        assert_eq!(decl, "impl Foo for Bar");
    }
}
