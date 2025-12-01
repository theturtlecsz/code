//! Native clarification heuristics (zero agents, zero cost, <1s)
//!
//! FORK-SPECIFIC (just-every/code): Pattern-matching for ambiguity detection
//! Eliminates $0.80 agent cost per /speckit.clarify execution
//!
//! Principle: Agents for reasoning, NOT transactions. Ambiguity detection is
//! pattern-matching (FREE) not reasoning ($0.80).

#![allow(dead_code)] // Extended heuristics pending

use super::error::{Result, SpecKitError};
use regex_lite::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Severity levels for ambiguity issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Critical,  // Blocks implementation
    Important, // Should fix before implementation
    Minor,     // Nice to fix
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Critical => "CRITICAL",
            Severity::Important => "IMPORTANT",
            Severity::Minor => "MINOR",
        }
    }
}

/// Detected ambiguity issue
#[derive(Debug, Clone)]
pub struct Ambiguity {
    pub id: String,                 // AMB-001, AMB-002...
    pub question: String,           // What's unclear?
    pub location: String,           // "PRD.md:45" or "spec.md:## Data Model"
    pub severity: Severity,         // Critical, Important, Minor
    pub pattern: String,            // Which pattern triggered it
    pub context: String,            // Surrounding text
    pub suggestion: Option<String>, // Auto-fix if obvious
}

/// Pattern detector for ambiguity analysis
struct PatternDetector {
    vague_language: Regex,
    incomplete_markers: Regex,
    quantifier_ambiguity: Regex,
    scope_gaps: Regex,
    time_ambiguity: Regex,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self {
            // Case-insensitive vague language
            vague_language: Regex::new(r"(?i)\b(should|might|consider|probably|maybe|could)\b")
                .unwrap(),

            // Incomplete markers
            incomplete_markers: Regex::new(r"\b(TBD|TODO|FIXME|XXX|\?\?\?)\b|\[placeholder\]")
                .unwrap(),

            // Vague quantifiers
            quantifier_ambiguity: Regex::new(
                r"(?i)\b(fast|slow|quick|scalable|responsive|performant|efficient|secure|robust|simple|complex)\b"
            ).unwrap(),

            // Scope gaps
            scope_gaps: Regex::new(r"\b(etc\.|and so on|similar|other|various)\b").unwrap(),

            // Time ambiguity
            time_ambiguity: Regex::new(r"(?i)\b(soon|later|eventually|ASAP|when possible)\b")
                .unwrap(),
        }
    }
}

impl PatternDetector {
    /// Check for vague language patterns
    fn check_vague_language(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
        if let Some(mat) = self.vague_language.find(content) {
            let word = mat.as_str();
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("What is the specific requirement? '{}' is vague", word),
                location: format!("line {}", line_num),
                severity: Severity::Important,
                pattern: "vague_language".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some(format!(
                    "Replace '{}' with measurable criteria (e.g., 'must', 'will', specific metric)",
                    word
                )),
            });
        }
    }

    /// Check for incomplete markers
    fn check_incomplete_markers(
        &self,
        content: &str,
        line_num: usize,
        issues: &mut Vec<Ambiguity>,
    ) {
        if let Some(mat) = self.incomplete_markers.find(content) {
            let marker = mat.as_str();
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("Incomplete requirement marked with '{}'", marker),
                location: format!("line {}", line_num),
                severity: Severity::Critical,
                pattern: "incomplete_marker".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some("Complete this requirement before implementation".to_string()),
            });
        }
    }

    /// Check for quantifier ambiguity
    fn check_quantifier_ambiguity(
        &self,
        content: &str,
        line_num: usize,
        issues: &mut Vec<Ambiguity>,
    ) {
        // Only flag if no metrics present in same line
        if self.quantifier_ambiguity.is_match(content)
            && !has_metrics(content)
            && let Some(mat) = self.quantifier_ambiguity.find(content)
        {
            let word = mat.as_str();
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("'{}' lacks quantifiable metrics", word),
                location: format!("line {}", line_num),
                severity: Severity::Important,
                pattern: "quantifier_ambiguity".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some(
                    "Add specific metric (e.g., '<100ms', '>1000 RPS', '<1MB memory')".to_string(),
                ),
            });
        }
    }

    /// Check for scope gaps
    fn check_scope_gaps(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
        if let Some(mat) = self.scope_gaps.find(content) {
            let word = mat.as_str();
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("Scope incomplete: '{}' - what else is included?", word),
                location: format!("line {}", line_num),
                severity: Severity::Minor,
                pattern: "scope_gap".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some("List all items explicitly instead of using 'etc.'".to_string()),
            });
        }
    }

    /// Check for time ambiguity
    fn check_time_ambiguity(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
        if let Some(mat) = self.time_ambiguity.find(content) {
            let word = mat.as_str();
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("Timeline unclear: '{}'", word),
                location: format!("line {}", line_num),
                severity: Severity::Important,
                pattern: "time_ambiguity".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some("Specify concrete timeline or milestone".to_string()),
            });
        }
    }
}

/// Check if section headers are present
fn check_missing_sections(content: &str, file_path: &str, issues: &mut Vec<Ambiguity>) {
    let required_sections = vec![
        ("Acceptance Criteria", Severity::Critical),
        ("Test Strategy", Severity::Important),
        ("Non-Functional Requirements", Severity::Minor),
    ];

    for (section, severity) in required_sections {
        let pattern = format!(r"(?m)^##\s+{}", regex_escape(section));
        let re = Regex::new(&pattern).unwrap();

        if !re.is_match(content) {
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("Missing required section: '{}'", section),
                location: file_path.to_string(),
                severity,
                pattern: "missing_section".to_string(),
                context: format!("Section '{}' not found", section),
                suggestion: Some(format!(
                    "Add '## {}' section with detailed criteria",
                    section
                )),
            });
        }
    }
}

/// Check for undefined technical terms (simple heuristic)
fn check_undefined_terms(content: &str, issues: &mut Vec<Ambiguity>) {
    // Common technical patterns that should be defined
    let term_patterns = vec![
        (r"\bAPI\b", "API"),
        (r"\bREST\b", "REST"),
        (r"\bWebSocket\b", "WebSocket"),
        (r"\bOAuth\b", "OAuth"),
        (r"\bJWT\b", "JWT"),
        (r"\bSSO\b", "SSO"),
    ];

    let mut first_uses = HashSet::new();
    let lines: Vec<&str> = content.lines().collect();

    for (line_num, line) in lines.iter().enumerate() {
        for (pattern_str, term) in &term_patterns {
            let re = Regex::new(pattern_str).unwrap();
            if re.is_match(line) && !first_uses.contains(*term) {
                // Check if definition follows (naive: looks for colon or dash after term)
                let definition_re =
                    Regex::new(&format!(r"{}(\s*[:\-]|\s+is\s+)", regex_escape(term))).unwrap();

                if !definition_re.is_match(line) {
                    issues.push(Ambiguity {
                        id: format!("AMB-{:03}", issues.len() + 1),
                        question: format!("Technical term '{}' used without definition", term),
                        location: format!("line {}", line_num + 1),
                        severity: Severity::Minor,
                        pattern: "undefined_term".to_string(),
                        context: truncate_context(line, 80),
                        suggestion: Some(format!("Define '{}' on first use", term)),
                    });
                }
                first_uses.insert((*term).to_string());
            }
        }
    }
}

/// Main entry point: find all ambiguities in a SPEC
pub fn find_ambiguities(spec_id: &str, cwd: &Path) -> Result<Vec<Ambiguity>> {
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)
        .map_err(|e| SpecKitError::Other(e))?;
    let mut all_issues = Vec::new();

    // Files to scan (in priority order)
    let files_to_scan = vec![
        ("PRD.md", true),   // required
        ("spec.md", false), // optional
        ("plan.md", false), // optional
    ];

    let detector = PatternDetector::default();

    for (filename, required) in files_to_scan {
        let file_path = spec_dir.join(filename);

        if !file_path.exists() {
            if required {
                all_issues.push(Ambiguity {
                    id: format!("AMB-{:03}", all_issues.len() + 1),
                    question: format!("Required file '{}' not found", filename),
                    location: filename.to_string(),
                    severity: Severity::Critical,
                    pattern: "missing_file".to_string(),
                    context: format!("Expected: {}", file_path.display()),
                    suggestion: Some("Create PRD.md before proceeding".to_string()),
                });
            }
            continue;
        }

        let content =
            fs::read_to_string(&file_path).map_err(|e| SpecKitError::file_read(&file_path, e))?;

        // Check for missing sections (PRD only)
        if filename == "PRD.md" {
            check_missing_sections(&content, filename, &mut all_issues);
        }

        // Check for undefined terms (first file only to avoid duplicates)
        if filename == "PRD.md" {
            check_undefined_terms(&content, &mut all_issues);
        }

        // Line-by-line pattern checks
        for (line_num, line) in content.lines().enumerate() {
            let line_num = line_num + 1; // 1-indexed

            // Skip markdown headers and code blocks
            if line.trim_start().starts_with('#') || line.trim_start().starts_with("```") {
                continue;
            }

            detector.check_vague_language(line, line_num, &mut all_issues);
            detector.check_incomplete_markers(line, line_num, &mut all_issues);
            detector.check_quantifier_ambiguity(line, line_num, &mut all_issues);
            detector.check_scope_gaps(line, line_num, &mut all_issues);
            detector.check_time_ambiguity(line, line_num, &mut all_issues);
        }
    }

    // Sort by severity (Critical first)
    all_issues.sort_by(|a, b| match (&a.severity, &b.severity) {
        (Severity::Critical, Severity::Critical) => std::cmp::Ordering::Equal,
        (Severity::Critical, _) => std::cmp::Ordering::Less,
        (_, Severity::Critical) => std::cmp::Ordering::Greater,
        (Severity::Important, Severity::Important) => std::cmp::Ordering::Equal,
        (Severity::Important, _) => std::cmp::Ordering::Less,
        (_, Severity::Important) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    });

    // Re-number after sorting
    for (idx, issue) in all_issues.iter_mut().enumerate() {
        issue.id = format!("AMB-{:03}", idx + 1);
    }

    Ok(all_issues)
}

/// Truncate context to reasonable length
fn truncate_context(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

/// Check if line contains metrics
fn has_metrics(line: &str) -> bool {
    let metrics_re = Regex::new(r"(<|>|<=|>=)?\s*\d+\s*(ms|MB|KB|GB|%|RPS|req/s|users?)").unwrap();
    metrics_re.is_match(line)
}

/// Simple regex escape (regex_lite doesn't have escape function)
fn regex_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|' | '[' | ']' | '{' | '}' | '^' | '$' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}

// =============================================================================
// [NEEDS CLARIFICATION] Marker Resolution (SPEC-KIT-971)
// =============================================================================

/// A clarification marker found in a spec file
#[derive(Debug, Clone)]
pub struct ClarificationMarker {
    /// Unique ID for this marker (CLR-001, CLR-002, etc.)
    pub id: String,
    /// The question text from inside the marker
    pub question: String,
    /// File path where marker was found
    pub file_path: std::path::PathBuf,
    /// Line number (1-indexed)
    pub line_number: usize,
    /// The full original marker text (for replacement)
    pub original_text: String,
}

/// Find all [NEEDS CLARIFICATION: ...] markers in a SPEC
pub fn find_clarification_markers(spec_id: &str, cwd: &Path) -> Result<Vec<ClarificationMarker>> {
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)
        .map_err(|e| SpecKitError::Other(e))?;
    let mut markers = Vec::new();

    // Pattern: [NEEDS CLARIFICATION: question text here]
    let marker_re = Regex::new(r"\[NEEDS CLARIFICATION:\s*([^\]]+)\]").unwrap();

    // Files to scan
    let files_to_scan = vec!["PRD.md", "spec.md", "plan.md", "tasks.md"];

    for filename in files_to_scan {
        let file_path = spec_dir.join(filename);
        if !file_path.exists() {
            continue;
        }

        let content =
            fs::read_to_string(&file_path).map_err(|e| SpecKitError::file_read(&file_path, e))?;

        for (line_idx, line) in content.lines().enumerate() {
            for cap in marker_re.captures_iter(line) {
                let full_match = cap.get(0).unwrap().as_str();
                let question = cap.get(1).unwrap().as_str().trim();

                markers.push(ClarificationMarker {
                    id: format!("CLR-{:03}", markers.len() + 1),
                    question: question.to_string(),
                    file_path: file_path.clone(),
                    line_number: line_idx + 1,
                    original_text: full_match.to_string(),
                });
            }
        }
    }

    Ok(markers)
}

/// Resolve a clarification marker by replacing it with the answer
pub fn resolve_marker(marker: &ClarificationMarker, answer: &str) -> Result<()> {
    let content = fs::read_to_string(&marker.file_path)
        .map_err(|e| SpecKitError::file_read(&marker.file_path, e))?;

    // Replace the marker with the answer
    let updated = content.replace(&marker.original_text, answer);

    fs::write(&marker.file_path, updated)
        .map_err(|e| SpecKitError::file_write(&marker.file_path, e))?;

    Ok(())
}

/// Resolve multiple markers at once (batch update)
pub fn resolve_markers(resolutions: &[(ClarificationMarker, String)]) -> Result<()> {
    use std::collections::HashMap;

    // Group by file path for efficiency
    let mut by_file: HashMap<std::path::PathBuf, Vec<(&ClarificationMarker, &str)>> =
        HashMap::new();
    for (marker, answer) in resolutions {
        by_file
            .entry(marker.file_path.clone())
            .or_default()
            .push((marker, answer.as_str()));
    }

    // Process each file
    for (file_path, replacements) in by_file {
        let mut content =
            fs::read_to_string(&file_path).map_err(|e| SpecKitError::file_read(&file_path, e))?;

        for (marker, answer) in replacements {
            content = content.replace(&marker.original_text, answer);
        }

        fs::write(&file_path, content).map_err(|e| SpecKitError::file_write(&file_path, e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vague_language_detection() {
        let detector = PatternDetector::default();
        let mut issues = Vec::new();

        detector.check_vague_language("The system should be fast", 1, &mut issues);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].question.contains("should"));
    }

    #[test]
    fn test_incomplete_markers() {
        let detector = PatternDetector::default();
        let mut issues = Vec::new();

        detector.check_incomplete_markers("TBD: Add authentication", 1, &mut issues);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Critical);
    }

    #[test]
    fn test_quantifier_ambiguity() {
        let detector = PatternDetector::default();
        let mut issues = Vec::new();

        // Should flag: no metrics
        detector.check_quantifier_ambiguity("Must be fast", 1, &mut issues);
        assert_eq!(issues.len(), 1);

        // Should NOT flag: has metrics
        issues.clear();
        detector.check_quantifier_ambiguity("Must be fast (<100ms)", 1, &mut issues);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_has_metrics() {
        assert!(has_metrics("Response time <100ms"));
        assert!(has_metrics("Must handle >1000 RPS"));
        assert!(has_metrics("Memory usage ≤ 512MB"));
        assert!(!has_metrics("Must be fast"));
    }

    #[test]
    fn test_severity_ordering() {
        let critical = Ambiguity {
            id: "AMB-001".to_string(),
            question: "test".to_string(),
            location: "test".to_string(),
            severity: Severity::Critical,
            pattern: "test".to_string(),
            context: "test".to_string(),
            suggestion: None,
        };

        let minor = Ambiguity {
            id: "AMB-002".to_string(),
            question: "test".to_string(),
            location: "test".to_string(),
            severity: Severity::Minor,
            pattern: "test".to_string(),
            context: "test".to_string(),
            suggestion: None,
        };

        let mut issues = vec![minor.clone(), critical.clone()];
        issues.sort_by(|a, b| match (&a.severity, &b.severity) {
            (Severity::Critical, Severity::Critical) => std::cmp::Ordering::Equal,
            (Severity::Critical, _) => std::cmp::Ordering::Less,
            (_, Severity::Critical) => std::cmp::Ordering::Greater,
            _ => std::cmp::Ordering::Equal,
        });

        assert_eq!(issues[0].severity, Severity::Critical);
    }

    #[test]
    fn test_clarification_marker_regex() {
        let marker_re = Regex::new(r"\[NEEDS CLARIFICATION:\s*([^\]]+)\]").unwrap();

        // Should match
        let test1 = "[NEEDS CLARIFICATION: Should we use sync or async?]";
        let cap = marker_re.captures(test1).unwrap();
        assert_eq!(
            cap.get(1).unwrap().as_str().trim(),
            "Should we use sync or async?"
        );

        // Should match with extra whitespace
        let test2 = "[NEEDS CLARIFICATION:   What is the latency target?  ]";
        let cap = marker_re.captures(test2).unwrap();
        assert_eq!(
            cap.get(1).unwrap().as_str().trim(),
            "What is the latency target?"
        );

        // Should NOT match incomplete markers
        let test3 = "[NEEDS CLARIFICATION]";
        assert!(marker_re.captures(test3).is_none());

        // Multiple markers in one line
        let test4 = "Choice: [NEEDS CLARIFICATION: A or B?] and [NEEDS CLARIFICATION: X or Y?]";
        let matches: Vec<_> = marker_re.captures_iter(test4).collect();
        assert_eq!(matches.len(), 2);
    }
}
