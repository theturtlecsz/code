//! Tier 2 (NotebookLM) orchestration for Stage0
//!
//! V1.5: Implements Divine Truth synthesis via NotebookLM MCP.
//! See docs/stage0/STAGE0_TIER2_PROMPT.md for prompt specification.
//!
//! Key components:
//! - `Tier2Client` trait: abstraction for NotebookLM calls
//! - `DivineTruth`: parsed response with sections and causal links
//! - `build_tier2_prompt()`: assembles the Shadow Staff Engineer prompt
//! - `parse_divine_truth()`: extracts structured sections from response

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::errors::Result;

// ─────────────────────────────────────────────────────────────────────────────
// Data Types
// ─────────────────────────────────────────────────────────────────────────────

/// A suggested causal link between memories from Tier2 synthesis
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CausalLinkSuggestion {
    /// Source memory ID
    pub from_id: String,
    /// Target memory ID
    pub to_id: String,
    /// Relationship type: "causes", "solves", "contradicts", "expands", "supersedes"
    #[serde(rename = "type")]
    pub rel_type: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Brief explanation of the relationship
    pub reasoning: String,
}

impl CausalLinkSuggestion {
    /// Validate the relationship type is one of the known types
    pub fn is_valid_rel_type(&self) -> bool {
        matches!(
            self.rel_type.as_str(),
            "causes" | "solves" | "contradicts" | "expands" | "supersedes"
        )
    }
}

/// Raw response from Tier2 client
#[derive(Debug, Clone)]
pub struct Tier2Response {
    /// Full Divine Truth markdown (sections 1-6)
    pub divine_truth_md: String,
    /// Parsed causal link suggestions from Section 6 JSON
    pub suggested_links: Vec<CausalLinkSuggestion>,
}

/// P90/SPEC-KIT-105: Constitution alignment analysis from Tier-2
///
/// Extracted from Section 2 of Divine Truth output.
/// Used for P91 conflict detection (basic parsing only in P90).
#[derive(Debug, Clone, Default)]
pub struct ConstitutionAlignment {
    /// IDs of principles/guardrails this spec aligns with (e.g., ["P1", "G2"])
    pub aligned_ids: Vec<String>,
    /// Raw markdown of conflicts section (for P91 conflict detection)
    pub conflicts_raw: Option<String>,
}

/// Parsed Divine Truth with structured sections
#[derive(Debug, Clone, Default)]
pub struct DivineTruth {
    /// Section 1: Executive summary bullets
    pub executive_summary: String,
    /// Section 2: Constitution alignment analysis (P90/SPEC-KIT-105)
    pub constitution_alignment: ConstitutionAlignment,
    /// Section 3: Architectural guardrails and constraints
    pub architectural_guardrails: String,
    /// Section 4: Historical context and lessons learned
    pub historical_context: String,
    /// Section 5: Risks and open questions
    pub risks_and_questions: String,
    /// Section 6: Suggested causal links (parsed from JSON)
    pub suggested_links: Vec<CausalLinkSuggestion>,
    /// Original raw markdown (preserved for debugging/display)
    pub raw_markdown: String,
}

impl DivineTruth {
    /// Check if this is a fallback (Tier1-only) response
    pub fn is_fallback(&self) -> bool {
        self.raw_markdown.contains("(Fallback)")
            || self.raw_markdown.contains("Tier2 unavailable")
            || self.raw_markdown.contains("Tier 2 unavailable")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tier2 Client Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Trait for Tier2 (NotebookLM) synthesis
///
/// Implementations should:
/// 1. Build the prompt using `build_tier2_prompt()`
/// 2. Call NotebookLM via MCP (mcp__notebooklm__ask_question)
/// 3. Parse the response using `parse_divine_truth()`
///
/// The concrete implementation lives in codex-rs; Stage0 only uses this trait.
#[async_trait]
pub trait Tier2Client: Send + Sync {
    /// Generate Divine Truth synthesis from spec and task brief
    ///
    /// # Arguments
    /// * `spec_id` - SPEC identifier (e.g., "SPEC-KIT-102")
    /// * `spec_content` - Full spec.md content
    /// * `task_brief_md` - DCC-generated TASK_BRIEF.md content
    ///
    /// # Returns
    /// * `Ok(Tier2Response)` - Successful synthesis
    /// * `Err(Stage0Error::Tier2)` - NotebookLM call failed
    async fn generate_divine_truth(
        &self,
        spec_id: &str,
        spec_content: &str,
        task_brief_md: &str,
    ) -> Result<Tier2Response>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Prompt Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Build the Tier2 prompt for NotebookLM
///
/// Implements the "Shadow Staff Engineer" prompt from docs/stage0/STAGE0_TIER2_PROMPT.md
///
/// P84: Updated to explicitly reference NL_* artifact names for better
/// NotebookLM retrieval (seeded artifacts use these exact filenames).
///
/// P90/SPEC-KIT-105: Added constitution awareness clause and Section 2.
/// S31: Chat query limit is ~2,000 chars. Using minimal prompt - full instructions
/// should be in NotebookLM's "Custom Instructions" (10k limit).
pub fn build_tier2_prompt(spec_id: &str, spec_content: &str, _task_brief_md: &str) -> String {
    const MAX_QUERY_CHARS: usize = 1800;
    const TEMPLATE_OVERHEAD: usize = 400; // Approx chars for template text
    let max_spec_chars = MAX_QUERY_CHARS.saturating_sub(TEMPLATE_OVERHEAD);

    // Truncate spec content to fit
    let spec_truncated: String = if spec_content.len() > max_spec_chars {
        let truncated: String = spec_content.chars().take(max_spec_chars - 50).collect();
        format!("{}...[truncated]", truncated)
    } else {
        spec_content.to_string()
    };

    // Minimal prompt - relies on NotebookLM's seeded sources for context
    format!(
        r#"Analyze {spec_id} for the codex-rs project.

SPEC:
{spec_truncated}

Using your sources (Architecture Bible, Bug Retros, Project Diary), provide:
1. **Summary**: What this spec does (3-5 bullets)
2. **Risks**: Key risks and mitigations
3. **Architecture**: Relevant patterns/constraints from sources
4. **History**: Related past issues or decisions

Keep response under 1000 words. Reference specific source documents."#,
        spec_id = spec_id,
        spec_truncated = spec_truncated
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Response Parsing
// ─────────────────────────────────────────────────────────────────────────────

/// Parse Divine Truth markdown into structured sections
///
/// P90/SPEC-KIT-105: Updated to parse Section 2 (Constitution Alignment)
/// and handle renumbered sections (3-6 instead of 2-5).
pub fn parse_divine_truth(response: &str) -> DivineTruth {
    let raw_markdown = response.to_string();
    let sections = extract_sections_by_header(response);

    let executive_summary = sections
        .get("1. Executive Summary")
        .cloned()
        .unwrap_or_default();

    // P90: Parse Section 2 (Constitution Alignment)
    let constitution_alignment = sections
        .get("2. Constitution Alignment")
        .map(|s| parse_constitution_alignment(s))
        .unwrap_or_default();

    // P90: Section numbers shifted (was 2-5, now 3-6)
    let architectural_guardrails = sections
        .get("3. Architectural Guardrails")
        .cloned()
        .unwrap_or_default();
    let historical_context = sections
        .get("4. Historical Context & Lessons")
        .cloned()
        .unwrap_or_default();
    let risks_and_questions = sections
        .get("5. Risks & Open Questions")
        .cloned()
        .unwrap_or_default();

    let suggested_links = extract_causal_links(
        sections
            .get("6. Suggested Causal Links")
            .map(String::as_str)
            .unwrap_or(""),
    );

    DivineTruth {
        executive_summary,
        constitution_alignment,
        architectural_guardrails,
        historical_context,
        risks_and_questions,
        suggested_links,
        raw_markdown,
    }
}

/// P90/SPEC-KIT-105: Parse Constitution Alignment section
///
/// Extracts aligned IDs from "**Aligned with:**" line and stores
/// conflicts as raw markdown for P91 conflict detection.
fn parse_constitution_alignment(section: &str) -> ConstitutionAlignment {
    let mut aligned_ids: Vec<String> = Vec::new();
    let mut conflicts_raw: Option<String> = None;

    // Find "**Aligned with:**" line and extract IDs
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("**Aligned with:**") || trimmed.starts_with("Aligned with:") {
            // Extract IDs like P1, G2, etc. from the line
            let after_label = trimmed
                .trim_start_matches("**Aligned with:**")
                .trim_start_matches("Aligned with:")
                .trim();

            // Parse comma-separated IDs, handling various formats:
            // "P1, G2" or "P1 (developer ergonomics), G2 (sandboxed ops)"
            for part in after_label.split(',') {
                let part = part.trim();
                // Extract just the ID (first word, or word before parenthesis)
                if let Some(id) = part.split_whitespace().next() {
                    let id = id.trim_matches(|c: char| !c.is_alphanumeric());
                    if !id.is_empty() {
                        aligned_ids.push(id.to_string());
                    }
                }
            }
        }
    }

    // Extract conflicts section (everything after "**Potential conflicts:**")
    if let Some(conflicts_start) = section.find("**Potential conflicts:**") {
        let after_conflicts = &section[conflicts_start..];
        // Take until next ** or end of section
        let conflicts_end = after_conflicts[24..] // Skip "**Potential conflicts:**"
            .find("**")
            .map(|i| i + 24)
            .unwrap_or(after_conflicts.len());
        let conflicts_text = after_conflicts[24..conflicts_end].trim();
        if !conflicts_text.is_empty() && conflicts_text != "None identified." {
            conflicts_raw = Some(conflicts_text.to_string());
        }
    }

    ConstitutionAlignment {
        aligned_ids,
        conflicts_raw,
    }
}

/// Extract sections by header (## N. Title)
fn extract_sections_by_header(md: &str) -> std::collections::HashMap<String, String> {
    let mut sections = std::collections::HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_content = String::new();

    for line in md.lines() {
        // Match section headers like "## 1. Executive Summary"
        if line.starts_with("## ") {
            // Save previous section
            if let Some(ref section_name) = current_section {
                sections.insert(section_name.clone(), current_content.trim().to_string());
            }

            // Extract section name (remove "## " prefix)
            let header = line.trim_start_matches('#').trim();
            current_section = Some(header.to_string());
            current_content = String::new();
        } else if current_section.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last section
    if let Some(section_name) = current_section {
        sections.insert(section_name, current_content.trim().to_string());
    }

    sections
}

/// Extract causal links from Section 5 JSON
fn extract_causal_links(section: &str) -> Vec<CausalLinkSuggestion> {
    // Look for JSON code fence
    let json_content = if let Some(start) = section.find("```json") {
        let start = start + 7; // Skip "```json"
        section[start..]
            .find("```")
            .map(|end| section[start..start + end].trim())
    } else if let Some(start) = section.find("```") {
        // Try bare code fence
        let start = start + 3;
        section[start..]
            .find("```")
            .map(|end| section[start..start + end].trim())
    } else {
        None
    };

    if let Some(json_str) = json_content {
        match serde_json::from_str::<Vec<CausalLinkSuggestion>>(json_str) {
            Ok(links) => {
                // Filter to valid link types and clamp confidence
                links
                    .into_iter()
                    .filter(CausalLinkSuggestion::is_valid_rel_type)
                    .map(|mut l| {
                        l.confidence = l.confidence.clamp(0.0, 1.0);
                        l
                    })
                    .collect()
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to parse causal links JSON");
                vec![]
            }
        }
    } else {
        // Try parsing section as raw JSON (no code fence)
        match serde_json::from_str::<Vec<CausalLinkSuggestion>>(section.trim()) {
            Ok(links) => links
                .into_iter()
                .filter(CausalLinkSuggestion::is_valid_rel_type)
                .map(|mut l| {
                    l.confidence = l.confidence.clamp(0.0, 1.0);
                    l
                })
                .collect(),
            Err(_) => vec![],
        }
    }
}

/// Validate causal links against known memory IDs
pub fn validate_causal_links(
    links: Vec<CausalLinkSuggestion>,
    valid_memory_ids: &std::collections::HashSet<String>,
) -> Vec<CausalLinkSuggestion> {
    links
        .into_iter()
        .filter(|link| {
            valid_memory_ids.contains(&link.from_id) && valid_memory_ids.contains(&link.to_id)
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// Fallback Builder
// ─────────────────────────────────────────────────────────────────────────────

/// Build a Tier1-only fallback Divine Truth when Tier2 is unavailable
///
/// P90/SPEC-KIT-105: Updated to include Section 2 (Constitution Alignment)
pub fn build_fallback_divine_truth(
    spec_id: &str,
    spec_content: &str,
    _task_brief_md: &str,
) -> DivineTruth {
    let mut raw_markdown = String::new();

    raw_markdown.push_str(&format!("# Divine Truth Brief (Fallback): {spec_id}\n\n"));
    raw_markdown.push_str("## 1. Executive Summary\n\n");
    raw_markdown.push_str(
        "- Tier2 (NotebookLM) was unavailable. This brief is generated from local context only.\n",
    );
    raw_markdown.push_str("- See Task Brief for detailed context from local-memory.\n");

    // Extract first few lines from spec as bullets
    raw_markdown.push_str("- Spec overview:\n");
    for line in spec_content.lines().take(3) {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            raw_markdown.push_str(&format!("  - {trimmed}\n"));
        }
    }

    // P90: Section 2 (Constitution Alignment)
    raw_markdown.push_str("\n## 2. Constitution Alignment\n\n");
    raw_markdown.push_str("**Aligned with:** _Unable to analyze (Tier2 unavailable)_\n\n");
    raw_markdown.push_str("**Potential conflicts:** _Unable to analyze (Tier2 unavailable)_\n");
    raw_markdown.push_str("- See Section 0 of TASK_BRIEF.md for constitution details.\n");

    // P90: Section numbers shifted (was 2-5, now 3-6)
    raw_markdown.push_str("\n## 3. Architectural Guardrails\n\n");
    raw_markdown.push_str("- See TASK_BRIEF.md for relevant memories and historical decisions.\n");
    raw_markdown.push_str("- Architectural analysis requires Tier2 (NotebookLM) access.\n");

    raw_markdown.push_str("\n## 4. Historical Context & Lessons\n\n");
    raw_markdown.push_str("- Historical analysis requires Tier2 (NotebookLM) access.\n");
    raw_markdown.push_str("- Relevant memories are included in TASK_BRIEF.md.\n");

    raw_markdown.push_str("\n## 5. Risks & Open Questions\n\n");
    raw_markdown.push_str("- Risk analysis requires Tier2 (NotebookLM) access.\n");
    raw_markdown.push_str("- Consider reviewing related SPECs and patterns manually.\n");

    raw_markdown.push_str("\n## 6. Suggested Causal Links\n\n");
    raw_markdown.push_str("```json\n[]\n```\n");
    raw_markdown.push_str("_Causal link suggestions require Tier2 access._\n");

    DivineTruth {
        executive_summary: "Tier2 unavailable. See TASK_BRIEF.md for local context.".to_string(),
        constitution_alignment: ConstitutionAlignment::default(),
        architectural_guardrails: "Tier2 unavailable.".to_string(),
        historical_context: "Tier2 unavailable.".to_string(),
        risks_and_questions: "Tier2 unavailable.".to_string(),
        suggested_links: vec![],
        raw_markdown,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Sample Divine Truth with P90 6-section format
    fn sample_divine_truth_md() -> &'static str {
        r#"# Divine Truth Brief: SPEC-KIT-102

## 1. Executive Summary

- This spec adds NotebookLM as Tier 2 synthesis layer.
- Provides architectural guidance from seeded knowledge.
- Results are cached in overlay DB.

## 2. Constitution Alignment

**Aligned with:** P1 (developer ergonomics), G2 (sandboxed ops)

**Potential conflicts:**
- Spec proposes direct file writes, but G2 requires sandboxing
- Mitigation: Use VFS abstraction layer (see Pattern P-034)

## 3. Architectural Guardrails

- Overlay pattern required: local-memory is closed-source.
- MCP-only access: use public MCP tools.
- Cache-first: always check cache before NotebookLM.

## 4. Historical Context & Lessons

- Prior daemon modification attempts failed.
- MCP integrations have been reliable.
- Rate limits are real and must be respected.

## 5. Risks & Open Questions

- Risk: NotebookLM rate limits. Mitigation: aggressive caching.
- Risk: Response format instability. Mitigation: robust parsing.
- Open question: single vs committee notebooks?

## 6. Suggested Causal Links

```json
[
  {
    "from_id": "mem-abc123",
    "to_id": "mem-def456",
    "type": "causes",
    "confidence": 0.85,
    "reasoning": "Overlay decision was made because modifying daemon failed"
  },
  {
    "from_id": "mem-ghi789",
    "to_id": "mem-abc123",
    "type": "expands",
    "confidence": 0.75,
    "reasoning": "MCP architecture decision informed overlay implementation"
  }
]
```
"#
    }

    #[test]
    fn test_parse_divine_truth_extracts_sections() {
        let dt = parse_divine_truth(sample_divine_truth_md());

        assert!(dt.executive_summary.contains("NotebookLM"));
        // P90: Section numbers shifted
        assert!(dt.architectural_guardrails.contains("Overlay pattern"));
        assert!(dt.historical_context.contains("daemon modification"));
        assert!(dt.risks_and_questions.contains("rate limits"));
        assert!(!dt.raw_markdown.is_empty());
    }

    #[test]
    fn test_parse_divine_truth_extracts_constitution_alignment() {
        let dt = parse_divine_truth(sample_divine_truth_md());

        // P90: Check constitution alignment extraction
        assert_eq!(dt.constitution_alignment.aligned_ids, vec!["P1", "G2"]);
        assert!(dt.constitution_alignment.conflicts_raw.is_some());
        let conflicts = dt.constitution_alignment.conflicts_raw.as_ref().unwrap();
        assert!(conflicts.contains("direct file writes"));
        assert!(conflicts.contains("G2 requires sandboxing"));
    }

    #[test]
    fn test_parse_divine_truth_extracts_links() {
        let dt = parse_divine_truth(sample_divine_truth_md());

        assert_eq!(dt.suggested_links.len(), 2);

        let first = &dt.suggested_links[0];
        assert_eq!(first.from_id, "mem-abc123");
        assert_eq!(first.to_id, "mem-def456");
        assert_eq!(first.rel_type, "causes");
        assert!((first.confidence - 0.85).abs() < 0.001);

        let second = &dt.suggested_links[1];
        assert_eq!(second.rel_type, "expands");
    }

    #[test]
    fn test_parse_constitution_alignment_empty() {
        let section = "No constitution defined in this project.";
        let alignment = parse_constitution_alignment(section);
        assert!(alignment.aligned_ids.is_empty());
        assert!(alignment.conflicts_raw.is_none());
    }

    #[test]
    fn test_parse_constitution_alignment_no_conflicts() {
        let section = r#"
**Aligned with:** P1, P2, G1

**Potential conflicts:**
None identified.
"#;
        let alignment = parse_constitution_alignment(section);
        assert_eq!(alignment.aligned_ids, vec!["P1", "P2", "G1"]);
        assert!(alignment.conflicts_raw.is_none()); // "None identified." is filtered
    }

    #[test]
    fn test_parse_constitution_alignment_with_descriptions() {
        let section = "**Aligned with:** P1 (developer ergonomics), G2 (sandboxed ops), Goal1";
        let alignment = parse_constitution_alignment(section);
        assert_eq!(alignment.aligned_ids, vec!["P1", "G2", "Goal1"]);
    }

    #[test]
    fn test_extract_causal_links_empty_array() {
        let section = "```json\n[]\n```";
        let links = extract_causal_links(section);
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_causal_links_no_fence() {
        let section = r#"[{"from_id": "a", "to_id": "b", "type": "causes", "confidence": 0.5, "reasoning": "test"}]"#;
        let links = extract_causal_links(section);
        assert_eq!(links.len(), 1);
    }

    #[test]
    fn test_extract_causal_links_filters_invalid_type() {
        let section = r#"```json
[
  {"from_id": "a", "to_id": "b", "type": "invalid_type", "confidence": 0.5, "reasoning": "test"},
  {"from_id": "c", "to_id": "d", "type": "causes", "confidence": 0.8, "reasoning": "valid"}
]
```"#;
        let links = extract_causal_links(section);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].rel_type, "causes");
    }

    #[test]
    fn test_extract_causal_links_clamps_confidence() {
        let section = r#"```json
[{"from_id": "a", "to_id": "b", "type": "causes", "confidence": 1.5, "reasoning": "over"}]
```"#;
        let links = extract_causal_links(section);
        assert_eq!(links.len(), 1);
        assert!((links[0].confidence - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_validate_causal_links() {
        let links = vec![
            CausalLinkSuggestion {
                from_id: "mem-1".to_string(),
                to_id: "mem-2".to_string(),
                rel_type: "causes".to_string(),
                confidence: 0.9,
                reasoning: "test".to_string(),
            },
            CausalLinkSuggestion {
                from_id: "mem-1".to_string(),
                to_id: "mem-invalid".to_string(),
                rel_type: "causes".to_string(),
                confidence: 0.9,
                reasoning: "test".to_string(),
            },
        ];

        let valid_ids: std::collections::HashSet<String> =
            ["mem-1", "mem-2"].into_iter().map(String::from).collect();

        let validated = validate_causal_links(links, &valid_ids);
        assert_eq!(validated.len(), 1);
        assert_eq!(validated[0].to_id, "mem-2");
    }

    #[test]
    fn test_build_tier2_prompt_contains_required_sections() {
        let prompt = build_tier2_prompt("SPEC-TEST", "Test spec content", "Test brief");

        assert!(prompt.contains("Shadow Staff Engineer"));
        assert!(prompt.contains("Divine Truth"));
        assert!(prompt.contains("SPEC ID: SPEC-TEST"));
        assert!(prompt.contains("Test spec content"));
        assert!(prompt.contains("Test brief"));
        assert!(prompt.contains("Executive Summary"));
        assert!(prompt.contains("Constitution Alignment")); // P90: New section
        assert!(prompt.contains("Architectural Guardrails"));
        assert!(prompt.contains("Historical Context"));
        assert!(prompt.contains("Risks & Open Questions"));
        assert!(prompt.contains("Suggested Causal Links"));

        // P84: Verify NL_* artifact names are explicitly referenced
        assert!(prompt.contains("NL_ARCHITECTURE_BIBLE.md"));
        assert!(prompt.contains("NL_STACK_JUSTIFICATION.md"));
        assert!(prompt.contains("NL_BUG_RETROS_01.md"));
        assert!(prompt.contains("NL_DEBT_LANDSCAPE.md"));
        assert!(prompt.contains("NL_PROJECT_DIARY_01.md"));

        // P90: Verify constitution awareness clause
        assert!(prompt.contains("CONSTITUTION AWARENESS"));
        assert!(prompt.contains("Principles"));
        assert!(prompt.contains("Guardrails"));
        assert!(prompt.contains("hard constraints"));
        assert!(prompt.contains("Section 0"));
    }

    #[test]
    fn test_build_fallback_divine_truth() {
        let fallback = build_fallback_divine_truth(
            "SPEC-TEST",
            "# Test Spec\n\nThis is a test.",
            "Task brief content",
        );

        assert!(fallback.is_fallback());
        assert!(fallback.raw_markdown.contains("Fallback"));
        assert!(fallback.raw_markdown.contains("SPEC-TEST"));
        assert!(fallback.suggested_links.is_empty());
        assert!(fallback.executive_summary.contains("Tier2 unavailable"));
    }

    #[test]
    fn test_divine_truth_is_fallback() {
        let normal = DivineTruth {
            raw_markdown: "# Divine Truth Brief: SPEC-1".to_string(),
            ..Default::default()
        };
        assert!(!normal.is_fallback());

        let fallback = DivineTruth {
            raw_markdown: "# Divine Truth Brief (Fallback): SPEC-1".to_string(),
            ..Default::default()
        };
        assert!(fallback.is_fallback());
    }

    #[test]
    fn test_causal_link_is_valid_rel_type() {
        let valid_types = ["causes", "solves", "contradicts", "expands", "supersedes"];
        for t in valid_types {
            let link = CausalLinkSuggestion {
                from_id: "a".to_string(),
                to_id: "b".to_string(),
                rel_type: t.to_string(),
                confidence: 0.5,
                reasoning: "test".to_string(),
            };
            assert!(link.is_valid_rel_type(), "Type {t} should be valid");
        }

        let invalid = CausalLinkSuggestion {
            from_id: "a".to_string(),
            to_id: "b".to_string(),
            rel_type: "unknown".to_string(),
            confidence: 0.5,
            reasoning: "test".to_string(),
        };
        assert!(!invalid.is_valid_rel_type());
    }
}
