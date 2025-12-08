//! Artifact chunker for NotebookLM upload size limits.
//!
//! NotebookLM has a practical limit of ~200KB per text source due to browser
//! automation limitations. This module provides intelligent chunking for large
//! artifacts while preserving semantic meaning.

/// Maximum chunk size in bytes (200KB safe limit for NotebookLM).
pub const MAX_CHUNK_SIZE: usize = 200_000;

/// A chunk of content ready for upload.
///
/// Note: The `name` field does NOT include the [ARCH] prefix.
/// The caller is responsible for wrapping this in `nlm_service::Artifact`.
#[derive(Debug, Clone)]
pub struct ChunkedPart {
    /// Name for this chunk (e.g., "Repo Skeleton (Part 1/2)").
    pub name: String,
    /// Content of this chunk.
    pub content: String,
}

impl ChunkedPart {
    /// Create a single-part chunk.
    pub fn single(name: &str, content: String) -> Self {
        Self {
            name: name.to_string(),
            content,
        }
    }

    /// Create a numbered part.
    pub fn part(name: &str, part: usize, total: usize, content: String) -> Self {
        Self {
            name: format!("{} (Part {}/{})", name, part, total),
            content,
        }
    }
}

/// Check if content needs chunking.
pub fn needs_chunking(content: &str) -> bool {
    content.len() > MAX_CHUNK_SIZE
}

/// Chunk large content into smaller pieces.
///
/// Returns a vector of `ChunkedPart`. Each part's `name` field can be used
/// with `nlm_service::Artifact::new()` to create uploadable artifacts.
pub fn chunk_content(name: &str, content: &str, chunk_type: ChunkType) -> Vec<ChunkedPart> {
    if content.len() <= MAX_CHUNK_SIZE {
        return vec![ChunkedPart::single(name, content.to_string())];
    }

    match chunk_type {
        ChunkType::Xml => chunk_xml(name, content),
        ChunkType::Mermaid => chunk_mermaid(name, content),
        ChunkType::Lines => chunk_by_lines(name, content),
    }
}

/// Type of content for intelligent chunking.
#[derive(Debug, Clone, Copy)]
pub enum ChunkType {
    /// XML content - split at top-level elements.
    Xml,
    /// Mermaid diagram - split at subgraph boundaries.
    Mermaid,
    /// Generic text - split at line boundaries.
    Lines,
}

/// Chunk XML content at top-level element boundaries.
/// Falls back to line-based chunking if structure prevents clean splits.
fn chunk_xml(name: &str, content: &str) -> Vec<ChunkedPart> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut element_depth: usize = 0;

    // Find the root element and XML declaration
    let mut header = String::new();
    let mut body_start = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("<?xml") || trimmed.starts_with("<repository") {
            header.push_str(line);
            header.push('\n');
            if trimmed.contains("<repository") {
                body_start = i + 1;
                break;
            }
        } else {
            body_start = i;
            break;
        }
    }

    let footer = "</repository>\n";
    let overhead = header.len() + footer.len();
    let effective_max = MAX_CHUNK_SIZE.saturating_sub(overhead);

    // Process body elements
    for line in &lines[body_start..] {
        let trimmed = line.trim();

        // Skip closing root tag
        if trimmed == "</repository>" {
            continue;
        }

        // Track element depth
        if trimmed.starts_with("<file") || trimmed.starts_with("<module") {
            element_depth += 1;
        }
        if trimmed.ends_with("/>") && element_depth > 0 {
            element_depth -= 1;
        } else if trimmed.starts_with("</file") || trimmed.starts_with("</module") {
            element_depth = element_depth.saturating_sub(1);
        }

        let line_with_newline = format!("{}\n", line);

        // Check if we need to start a new chunk BEFORE adding this line
        // At element boundaries (depth == 0) or forced split if too large
        let would_exceed = current_chunk.len() + line_with_newline.len() > effective_max;
        let at_boundary = element_depth == 0;

        if would_exceed && !current_chunk.is_empty() {
            // Push current chunk before it gets too big
            chunks.push(format!("{}{}{}", header, current_chunk, footer));
            current_chunk = String::new();
        }

        current_chunk.push_str(&line_with_newline);

        // Also check after adding if we're at a boundary and over limit
        // This handles the case where we just closed an element
        if at_boundary && current_chunk.len() > effective_max {
            chunks.push(format!("{}{}{}", header, current_chunk, footer));
            current_chunk = String::new();
        }
    }

    // Push remaining content
    if !current_chunk.is_empty() {
        chunks.push(format!("{}{}{}", header, current_chunk, footer));
    }

    // If any chunk is still too big, fall back to line-based chunking
    let max_chunk_size = chunks.iter().map(|c| c.len()).max().unwrap_or(0);
    if max_chunk_size > MAX_CHUNK_SIZE {
        // Fall back to simple line-based chunking
        return chunk_by_lines(name, content);
    }

    // Convert to ChunkedParts with part numbers
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| ChunkedPart::part(name, i + 1, total, content))
        .collect()
}

/// Chunk Mermaid diagram at subgraph boundaries.
/// Falls back to line-based chunking if structure prevents clean splits.
fn chunk_mermaid(name: &str, content: &str) -> Vec<ChunkedPart> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut in_subgraph = false;

    // Extract header (graph type declaration)
    let header = if let Some(first) = lines.first() {
        if first.trim().starts_with("graph") || first.trim().starts_with("flowchart") {
            format!("{}\n", first)
        } else {
            String::new()
        }
    } else {
        return vec![ChunkedPart::single(name, content.to_string())];
    };

    let start_idx = if header.is_empty() { 0 } else { 1 };
    let effective_max = MAX_CHUNK_SIZE.saturating_sub(header.len());

    for line in &lines[start_idx..] {
        let trimmed = line.trim();

        // Track subgraph boundaries
        if trimmed.starts_with("subgraph") {
            in_subgraph = true;
        }
        if trimmed == "end" && in_subgraph {
            in_subgraph = false;
        }

        let line_with_newline = format!("{}\n", line);

        // Check BEFORE adding if this would exceed the limit
        let would_exceed = current_chunk.len() + line_with_newline.len() > effective_max;

        if would_exceed && !current_chunk.trim().is_empty() {
            // Push current chunk before it gets too big
            chunks.push(format!("{}{}", header, current_chunk));
            current_chunk = String::new();
        }

        current_chunk.push_str(&line_with_newline);

        // Also check after adding if we're at a boundary and over limit
        if !in_subgraph && current_chunk.len() > effective_max && !current_chunk.trim().is_empty() {
            chunks.push(format!("{}{}", header, current_chunk));
            current_chunk = String::new();
        }
    }

    // Push remaining content
    if !current_chunk.trim().is_empty() {
        chunks.push(format!("{}{}", header, current_chunk));
    }

    // If any chunk is still too big, fall back to line-based chunking
    let max_chunk_size = chunks.iter().map(|c| c.len()).max().unwrap_or(0);
    if max_chunk_size > MAX_CHUNK_SIZE {
        return chunk_by_lines(name, content);
    }

    // Convert to ChunkedParts with part numbers
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| ChunkedPart::part(name, i + 1, total, content))
        .collect()
}

/// Chunk generic text content at line boundaries.
fn chunk_by_lines(name: &str, content: &str) -> Vec<ChunkedPart> {
    let lines: Vec<&str> = content.lines().collect();
    let mut chunks = Vec::new();
    let mut current_chunk = String::new();

    for line in lines {
        let line_with_newline = format!("{}\n", line);

        if current_chunk.len() + line_with_newline.len() > MAX_CHUNK_SIZE
            && !current_chunk.is_empty()
        {
            chunks.push(current_chunk);
            current_chunk = String::new();
        }

        current_chunk.push_str(&line_with_newline);
    }

    // Push remaining content
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    // Convert to ChunkedParts with part numbers
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(i, content)| ChunkedPart::part(name, i + 1, total, content))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_content_no_chunk() {
        let content = "Small content that doesn't need chunking";
        let parts = chunk_content("Test", content, ChunkType::Lines);
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].name, "Test");
    }

    #[test]
    fn test_chunked_part_single() {
        let part = ChunkedPart::single("Churn Matrix", "content".to_string());
        assert_eq!(part.name, "Churn Matrix");
    }

    #[test]
    fn test_chunked_part_numbered() {
        let part = ChunkedPart::part("Repo Skeleton", 2, 3, "content".to_string());
        assert_eq!(part.name, "Repo Skeleton (Part 2/3)");
    }

    #[test]
    fn test_needs_chunking_fn() {
        assert!(!needs_chunking(&"x".repeat(100)));
        assert!(needs_chunking(&"x".repeat(MAX_CHUNK_SIZE + 1)));
    }

    #[test]
    fn test_chunk_by_lines() {
        // Create content larger than MAX_CHUNK_SIZE
        let line = "x".repeat(1000);
        let lines: Vec<&str> = (0..300).map(|_| line.as_str()).collect();
        let content = lines.join("\n");

        let parts = chunk_content("Large", &content, ChunkType::Lines);

        // Should produce multiple chunks
        assert!(parts.len() > 1);

        // Each chunk should be under the limit
        for part in &parts {
            assert!(part.content.len() <= MAX_CHUNK_SIZE + 1000); // Allow some margin for last line
        }

        // Names should be numbered
        assert!(parts[0].name.contains("Part 1/"));
    }
}
