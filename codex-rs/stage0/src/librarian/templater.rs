//! Memory templating for SPEC-KIT-103 Librarian
//!
//! Restructures memory content into canonical CONTEXT/REASONING/OUTCOME/TAGS format.
//! Preserves original content while adding structure for better retrieval.

use super::classifier::MemoryType;

/// Result of applying a template to memory content
#[derive(Debug, Clone)]
pub struct TemplatedMemory {
    /// The restructured content in canonical format
    pub content: String,
    /// The memory type used for templating
    pub memory_type: MemoryType,
    /// Whether original content was preserved (vs restructured)
    pub preserved_original: bool,
    /// Warnings generated during templating
    pub warnings: Vec<String>,
}

impl TemplatedMemory {
    /// Check if templating produced any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Configuration for the templater
#[derive(Debug, Clone)]
pub struct TemplaterConfig {
    /// Whether to preserve original content when restructuring fails
    pub preserve_on_failure: bool,
    /// Whether to add section headers even if content doesn't fit well
    pub force_structure: bool,
    /// Maximum length for each section (0 = unlimited)
    pub max_section_length: usize,
}

impl Default for TemplaterConfig {
    fn default() -> Self {
        Self {
            preserve_on_failure: true,
            force_structure: false,
            max_section_length: 0,
        }
    }
}

/// Apply canonical template structure to memory content
///
/// Transforms content into CONTEXT/REASONING/OUTCOME/TAGS format.
/// If content already has structure, preserves it. If unstructured,
/// attempts to infer sections from content.
pub fn apply_template(content: &str, memory_type: MemoryType) -> TemplatedMemory {
    apply_template_with_config(content, memory_type, &TemplaterConfig::default())
}

/// Apply template with custom configuration
pub fn apply_template_with_config(
    content: &str,
    memory_type: MemoryType,
    config: &TemplaterConfig,
) -> TemplatedMemory {
    let mut warnings = Vec::new();

    // Check if content already has canonical structure
    if has_canonical_structure(content) {
        // Just ensure type tag is present
        let content_with_tag = ensure_type_tag(content, memory_type);
        return TemplatedMemory {
            content: content_with_tag,
            memory_type,
            preserved_original: true,
            warnings,
        };
    }

    // Try to extract sections from unstructured content
    let sections = extract_sections(content, memory_type);

    // Build structured content
    let structured = build_structured_content(&sections, memory_type, config, &mut warnings);

    TemplatedMemory {
        content: structured,
        memory_type,
        preserved_original: false,
        warnings,
    }
}

/// Check if content already has CONTEXT/REASONING/OUTCOME structure
fn has_canonical_structure(content: &str) -> bool {
    let content_lower = content.to_lowercase();

    // Need at least 2 of the 3 main sections
    let has_context = content_lower.contains("## context") || content_lower.contains("# context");
    let has_reasoning =
        content_lower.contains("## reasoning") || content_lower.contains("# reasoning");
    let has_outcome = content_lower.contains("## outcome") || content_lower.contains("# outcome");

    let section_count = [has_context, has_reasoning, has_outcome]
        .iter()
        .filter(|&&x| x)
        .count();

    section_count >= 2
}

/// Ensure type tag is present in content
fn ensure_type_tag(content: &str, memory_type: MemoryType) -> String {
    let type_tag = memory_type.as_tag();
    let content_lower = content.to_lowercase();

    // Check if any type tag exists
    if content_lower.contains("type:") {
        return content.to_string();
    }

    // Check if there's a TAGS section to append to
    if let Some(tags_pos) = content_lower.find("## tags") {
        let after_tags = &content[tags_pos..];
        if let Some(next_section) = after_tags[7..].find("\n##") {
            // Insert before next section
            let insert_pos = tags_pos + 7 + next_section;
            let mut result = content[..insert_pos].to_string();
            result.push_str(&format!("\n- {}", type_tag));
            result.push_str(&content[insert_pos..]);
            return result;
        } else {
            // Append to end of TAGS section
            return format!("{}\n- {}", content, type_tag);
        }
    }

    // No TAGS section, append one
    format!("{}\n\n## TAGS\n- {}", content.trim_end(), type_tag)
}

/// Sections extracted from content
#[derive(Debug, Default)]
struct ExtractedSections {
    context: Option<String>,
    reasoning: Option<String>,
    outcome: Option<String>,
    original: String,
}

/// Extract sections from unstructured content using heuristics
fn extract_sections(content: &str, memory_type: MemoryType) -> ExtractedSections {
    let mut sections = ExtractedSections {
        original: content.to_string(),
        ..Default::default()
    };

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return sections;
    }

    // Type-specific extraction strategies
    match memory_type {
        MemoryType::Decision => extract_decision_sections(&lines, &mut sections),
        MemoryType::Problem => extract_problem_sections(&lines, &mut sections),
        MemoryType::Pattern => extract_pattern_sections(&lines, &mut sections),
        MemoryType::Insight => extract_insight_sections(&lines, &mut sections),
        MemoryType::Exception => extract_exception_sections(&lines, &mut sections),
        MemoryType::Reference => extract_reference_sections(&lines, &mut sections),
        MemoryType::Unknown => {
            // For unknown, just use the whole content as context
            sections.context = Some(content.to_string());
        }
    }

    sections
}

fn extract_decision_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Decision pattern: [WHAT was decided] [WHY/rationale] [RESULT/trade-offs]
    let content = lines.join("\n");

    // Look for rationale markers
    let rationale_markers = ["because", "rationale:", "reason:", "trade-off", "tradeoff"];
    let mut rationale_start = None;

    for marker in rationale_markers {
        if let Some(pos) = content.to_lowercase().find(marker) {
            rationale_start = Some(pos);
            break;
        }
    }

    if let Some(pos) = rationale_start {
        sections.context = Some(content[..pos].trim().to_string());
        sections.reasoning = Some(content[pos..].trim().to_string());
    } else {
        // No clear rationale, use first sentence as context, rest as reasoning
        let first_period = content.find('.').unwrap_or(content.len());
        sections.context = Some(content[..first_period + 1].trim().to_string());
        if first_period + 1 < content.len() {
            sections.reasoning = Some(content[first_period + 1..].trim().to_string());
        }
    }
}

fn extract_problem_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Problem pattern: [WHAT was the issue] [WHY it happened] [HOW it was fixed]
    let content = lines.join("\n");

    // Look for fix/resolution markers
    let fix_markers = ["fixed", "resolved", "solution:", "workaround", "fix:"];

    for marker in fix_markers {
        if let Some(pos) = content.to_lowercase().find(marker) {
            sections.context = Some(content[..pos].trim().to_string());
            sections.outcome = Some(content[pos..].trim().to_string());
            return;
        }
    }

    // No fix marker, use whole content as context
    sections.context = Some(content);
}

fn extract_pattern_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Pattern: [WHEN to use] [HOW to apply] [EXPECTED result]
    let content = lines.join("\n");

    sections.context = Some(format!("Applies when: {}", content.lines().next().unwrap_or("")));
    if content.lines().count() > 1 {
        sections.reasoning = Some(
            content
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
                .to_string(),
        );
    }
}

fn extract_insight_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Insight: [OBSERVATION] [IMPLICATION/learning]
    let content = lines.join("\n");

    sections.context = Some(format!("Observed: {}", content.lines().next().unwrap_or("")));
    if content.lines().count() > 1 {
        sections.outcome = Some(format!(
            "Implication: {}",
            content
                .lines()
                .skip(1)
                .collect::<Vec<_>>()
                .join("\n")
                .trim()
        ));
    }
}

fn extract_exception_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Exception: [WHAT rule is bypassed] [WHY it's allowed] [SCOPE]
    let content = lines.join("\n");

    let bypass_markers = [
        "bypass",
        "override",
        "exemption",
        "exception",
        "sanctioned",
    ];

    for marker in bypass_markers {
        if let Some(pos) = content.to_lowercase().find(marker) {
            sections.context = Some(format!("Exception for: {}", content[..pos].trim()));
            sections.reasoning = Some(content[pos..].trim().to_string());
            return;
        }
    }

    sections.context = Some(content);
}

fn extract_reference_sections(lines: &[&str], sections: &mut ExtractedSections) {
    // Reference: [WHAT it references] [WHY it's useful]
    let content = lines.join("\n");

    // Extract URL if present
    if let Some(url_start) = content.find("http") {
        let url_end = content[url_start..]
            .find(|c: char| c.is_whitespace())
            .map(|pos| url_start + pos)
            .unwrap_or(content.len());
        let url = &content[url_start..url_end];
        sections.outcome = Some(format!("Link: {}", url));

        let non_url = format!(
            "{}{}",
            content[..url_start].trim(),
            content[url_end..].trim()
        );
        if !non_url.is_empty() {
            sections.context = Some(non_url);
        }
    } else {
        sections.context = Some(content);
    }
}

/// Build structured content from extracted sections
fn build_structured_content(
    sections: &ExtractedSections,
    memory_type: MemoryType,
    config: &TemplaterConfig,
    warnings: &mut Vec<String>,
) -> String {
    let mut output = String::new();

    // CONTEXT section (required)
    output.push_str("## CONTEXT\n");
    if let Some(ref ctx) = sections.context {
        let ctx = maybe_truncate(ctx, config.max_section_length);
        output.push_str(&ctx);
    } else if config.force_structure {
        output.push_str(&sections.original);
        warnings.push("Could not extract CONTEXT, used original content".to_string());
    } else {
        output.push_str(&sections.original);
        warnings.push("Content structure unclear, preserved original".to_string());
    }
    output.push_str("\n\n");

    // REASONING section (optional)
    if let Some(ref reasoning) = sections.reasoning {
        output.push_str("## REASONING\n");
        let reasoning = maybe_truncate(reasoning, config.max_section_length);
        output.push_str(&reasoning);
        output.push_str("\n\n");
    }

    // OUTCOME section (optional)
    if let Some(ref outcome) = sections.outcome {
        output.push_str("## OUTCOME\n");
        let outcome = maybe_truncate(outcome, config.max_section_length);
        output.push_str(&outcome);
        output.push_str("\n\n");
    }

    // TAGS section (always added)
    output.push_str("## TAGS\n");
    output.push_str(&format!("- {}", memory_type.as_tag()));

    output
}

/// Truncate content if max_length > 0
fn maybe_truncate(content: &str, max_length: usize) -> String {
    if max_length > 0 && content.len() > max_length {
        format!("{}...", &content[..max_length])
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_canonical_structure() {
        let structured = "## CONTEXT\nSome context\n\n## REASONING\nSome reasoning\n\n## OUTCOME\nSome outcome";
        assert!(has_canonical_structure(structured));

        let partial = "## CONTEXT\nSome context\n\n## OUTCOME\nSome outcome";
        assert!(has_canonical_structure(partial));

        let unstructured = "Just some plain text without any structure.";
        assert!(!has_canonical_structure(unstructured));
    }

    #[test]
    fn test_ensure_type_tag_new() {
        let content = "Some content without tags";
        let result = ensure_type_tag(content, MemoryType::Pattern);
        assert!(result.contains("type:pattern"));
        assert!(result.contains("## TAGS"));
    }

    #[test]
    fn test_ensure_type_tag_existing() {
        let content = "Content with type:decision tag";
        let result = ensure_type_tag(content, MemoryType::Pattern);
        // Should not add another type tag
        assert!(!result.contains("type:pattern"));
        assert!(result.contains("type:decision"));
    }

    #[test]
    fn test_apply_template_already_structured() {
        let content = "## CONTEXT\nWhen to use\n\n## REASONING\nWhy\n\n## OUTCOME\nResult";
        let result = apply_template(content, MemoryType::Pattern);
        assert!(result.preserved_original);
        assert!(result.content.contains("type:pattern"));
    }

    #[test]
    fn test_apply_template_unstructured_decision() {
        let content = "Decision: Use SQLite because we need embedded storage. Trade-off is less scalability.";
        let result = apply_template(content, MemoryType::Decision);
        assert!(!result.preserved_original);
        assert!(result.content.contains("## CONTEXT"));
        assert!(result.content.contains("## REASONING") || result.content.contains("## OUTCOME"));
        assert!(result.content.contains("type:decision"));
    }

    #[test]
    fn test_apply_template_unstructured_problem() {
        let content = "Bug in cache layer causing memory leak. Fixed by adding explicit cleanup on timeout.";
        let result = apply_template(content, MemoryType::Problem);
        assert!(!result.preserved_original);
        assert!(result.content.contains("## CONTEXT"));
        assert!(result.content.contains("type:problem"));
    }

    #[test]
    fn test_apply_template_with_config() {
        let config = TemplaterConfig {
            max_section_length: 50,
            ..Default::default()
        };
        let long_content = "A".repeat(100);
        let result = apply_template_with_config(&long_content, MemoryType::Unknown, &config);
        assert!(result.content.contains("..."));
    }

    #[test]
    fn test_templated_memory_has_warnings() {
        let result = TemplatedMemory {
            content: String::new(),
            memory_type: MemoryType::Unknown,
            preserved_original: false,
            warnings: vec!["Test warning".to_string()],
        };
        assert!(result.has_warnings());

        let no_warnings = TemplatedMemory {
            content: String::new(),
            memory_type: MemoryType::Unknown,
            preserved_original: true,
            warnings: vec![],
        };
        assert!(!no_warnings.has_warnings());
    }

    #[test]
    fn test_extract_reference_with_url() {
        let content = "Check the docs at https://example.com/api for more info.";
        let result = apply_template(content, MemoryType::Reference);
        assert!(result.content.contains("https://example.com/api"));
        assert!(result.content.contains("type:reference"));
    }

    #[test]
    fn test_extract_insight() {
        let content = "TIL: Rust's borrow checker prevents data races at compile time.";
        let result = apply_template(content, MemoryType::Insight);
        assert!(result.content.contains("## CONTEXT"));
        assert!(result.content.contains("type:insight"));
    }

    #[test]
    fn test_unknown_type_preserves_content() {
        let content = "Random text that doesn't match any pattern clearly.";
        let result = apply_template(content, MemoryType::Unknown);
        // Should preserve original in CONTEXT
        assert!(result.content.contains(content) || result.content.contains("Random text"));
        assert!(result.content.contains("type:unknown"));
    }
}
