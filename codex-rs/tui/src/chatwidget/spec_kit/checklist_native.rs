//! Native quality scoring rubric (zero agents, zero cost, <1s)
//!
//! FORK-SPECIFIC (just-every/code): Automated requirement quality evaluation
//! Eliminates $0.35 agent cost per /speckit.checklist execution
//!
//! Principle: Agents for reasoning, NOT transactions. Quality scoring is
//! pattern-matching (FREE) not reasoning ($0.35).

#![allow(dead_code)] // Extended scoring helpers pending

use super::analyze_native::check_consistency;
use super::clarify_native::Severity;
use super::error::{Result, SpecKitError};
use regex_lite::Regex;
use std::fs;
use std::path::Path;

/// Quality issue detected during rubric evaluation
#[derive(Debug, Clone)]
pub struct QualityIssue {
    pub id: String,       // CHK-001...
    pub category: String, // "completeness", "clarity", "testability", "consistency"
    pub severity: Severity,
    pub description: String,
    pub impact: String, // Impact on score
    pub suggestion: String,
}

/// Quality report with scores
#[derive(Debug, Clone)]
pub struct QualityReport {
    pub spec_id: String,
    pub overall_score: f32,        // 0-100
    pub completeness: f32,         // 0-100
    pub clarity: f32,              // 0-100
    pub testability: f32,          // 0-100
    pub consistency: f32,          // 0-100
    pub issues: Vec<QualityIssue>, // CHK-001, CHK-002...
    pub recommendations: Vec<String>,
}

impl QualityReport {
    /// Generate summary text
    pub fn summary(&self) -> String {
        format!(
            "Overall: {:.1}% | Completeness: {:.1}% | Clarity: {:.1}% | Testability: {:.1}% | Consistency: {:.1}%",
            self.overall_score, self.completeness, self.clarity, self.testability, self.consistency
        )
    }

    /// Get grade letter (A-F)
    pub fn grade(&self) -> &'static str {
        match self.overall_score {
            s if s >= 90.0 => "A",
            s if s >= 80.0 => "B",
            s if s >= 70.0 => "C",
            s if s >= 60.0 => "D",
            _ => "F",
        }
    }
}

/// Score quality of a SPEC
pub fn score_quality(spec_id: &str, cwd: &Path) -> Result<QualityReport> {
    let spec_dir = super::spec_directory::find_spec_directory(cwd, spec_id)
        .map_err(|e| SpecKitError::Other(e))?;
    let mut issues = Vec::new();

    // Load PRD (required)
    let prd_path = spec_dir.join("PRD.md");
    if !prd_path.exists() {
        return Err(SpecKitError::MissingArtifact {
            spec_id: spec_id.to_string(),
            stage: "checklist".to_string(),
            artifact: "PRD.md".to_string(),
        });
    }

    let prd_content =
        fs::read_to_string(&prd_path).map_err(|e| SpecKitError::file_read(&prd_path, e))?;

    // Score each dimension
    let completeness = score_completeness(&prd_content, &mut issues);
    let clarity = score_clarity(&prd_content, &mut issues);
    let testability = score_testability(&prd_content, &mut issues);
    let consistency = score_consistency(spec_id, cwd, &mut issues)?;

    // Overall score (weighted average)
    let overall_score =
        (completeness * 0.3) + (clarity * 0.2) + (testability * 0.3) + (consistency * 0.2);

    // Generate recommendations
    let mut recommendations = Vec::new();
    if completeness < 80.0 {
        recommendations.push("Add missing required sections to PRD".to_string());
    }
    if clarity < 70.0 {
        recommendations.push("Remove vague language and add specific metrics".to_string());
    }
    if testability < 80.0 {
        recommendations.push("Add measurable acceptance criteria for all requirements".to_string());
    }
    if consistency < 90.0 {
        recommendations.push("Fix inconsistencies between PRD and plan/tasks".to_string());
    }

    // Re-number issues
    for (idx, issue) in issues.iter_mut().enumerate() {
        issue.id = format!("CHK-{:03}", idx + 1);
    }

    Ok(QualityReport {
        spec_id: spec_id.to_string(),
        overall_score,
        completeness,
        clarity,
        testability,
        consistency,
        issues,
        recommendations,
    })
}

/// Score completeness dimension (0-100%)
fn score_completeness(prd_content: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 0.0;
    let max_score = 100.0;

    // Required sections (20% each)
    let required_sections = vec![
        ("Problem Statement", 20.0),
        ("Goals", 20.0),
        ("Requirements", 20.0),
        ("Acceptance Criteria", 20.0),
        ("Test Strategy", 20.0),
    ];

    for (section, points) in required_sections {
        let pattern = format!(r"(?mi)^##\s+{}", regex_escape(section));
        let re = Regex::new(&pattern).unwrap();

        if re.is_match(prd_content) {
            score += points;
        } else {
            issues.push(QualityIssue {
                id: format!("CHK-{:03}", issues.len() + 1),
                category: "completeness".to_string(),
                severity: if section == "Requirements" || section == "Acceptance Criteria" {
                    Severity::Critical
                } else {
                    Severity::Important
                },
                description: format!("Missing required section: '{}'", section),
                impact: format!("-{:.0}%", points),
                suggestion: format!("Add '## {}' section to PRD", section),
            });
        }
    }

    score
}

/// Score clarity dimension (0-100%)
fn score_clarity(prd_content: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 100.0;

    // Count requirements
    let req_re = Regex::new(r"(?m)^(FR|NFR|R|AC)-\d{3}:").unwrap();
    let requirement_count = req_re.find_iter(prd_content).count();

    if requirement_count == 0 {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Critical,
            description: "No structured requirements found (FR-XXX, NFR-XXX format)".to_string(),
            impact: "-50%".to_string(),
            suggestion: "Use FR-001, NFR-001 format for all requirements".to_string(),
        });
        score -= 50.0;
    }

    // Check requirement length (50-150 words = ideal)
    let lines: Vec<&str> = prd_content.lines().collect();
    let mut long_requirements = 0;
    let mut short_requirements = 0;

    for line in &lines {
        if req_re.is_match(line) {
            let word_count = line.split_whitespace().count();
            if word_count > 200 {
                long_requirements += 1;
            } else if word_count < 20 {
                short_requirements += 1;
            }
        }
    }

    if long_requirements > 0 {
        let penalty = (long_requirements as f32 * 5.0).min(20.0);
        score -= penalty;
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Minor,
            description: format!(
                "{} requirements are too long (>200 words)",
                long_requirements
            ),
            impact: format!("-{:.0}%", penalty),
            suggestion: "Break long requirements into smaller, focused statements".to_string(),
        });
    }

    if short_requirements > 0 {
        let penalty = (short_requirements as f32 * 3.0).min(15.0);
        score -= penalty;
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Minor,
            description: format!(
                "{} requirements are too short (<20 words)",
                short_requirements
            ),
            impact: format!("-{:.0}%", penalty),
            suggestion: "Add more detail to short requirements".to_string(),
        });
    }

    // Check for vague language
    let vague_re = Regex::new(r"(?i)\b(should|might|consider|probably|maybe|could)\b").unwrap();
    let vague_count = vague_re.find_iter(prd_content).count();

    if vague_count > 5 {
        let penalty = (vague_count as f32).min(20.0);
        score -= penalty;
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Important,
            description: format!(
                "{} instances of vague language (should, might, could, etc.)",
                vague_count
            ),
            impact: format!("-{:.0}%", penalty),
            suggestion: "Replace vague language with definitive statements (must, will)"
                .to_string(),
        });
    }

    // Check for undefined technical terms
    let terms = vec!["API", "REST", "WebSocket", "OAuth", "JWT", "SSO"];
    let mut undefined_terms = Vec::new();

    for term in terms {
        let term_re = Regex::new(&format!(r"\b{}\b", regex_escape(term))).unwrap();
        let definition_re =
            Regex::new(&format!(r"{}(\s*[:\-]|\s+is\s+)", regex_escape(term))).unwrap();

        if term_re.is_match(prd_content) && !definition_re.is_match(prd_content) {
            undefined_terms.push(term);
        }
    }

    if !undefined_terms.is_empty() {
        let penalty = (undefined_terms.len() as f32 * 3.0).min(15.0);
        score -= penalty;
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Minor,
            description: format!(
                "{} technical terms used without definition: {:?}",
                undefined_terms.len(),
                undefined_terms
            ),
            impact: format!("-{:.0}%", penalty),
            suggestion: "Define technical terms on first use".to_string(),
        });
    }

    score.max(0.0)
}

/// Score testability dimension (0-100%)
fn score_testability(prd_content: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 0.0;

    // Check for acceptance criteria section
    let ac_section_re = Regex::new(r"(?mi)^##\s+Acceptance Criteria").unwrap();
    if ac_section_re.is_match(prd_content) {
        score += 20.0;
    } else {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "testability".to_string(),
            severity: Severity::Critical,
            description: "Missing 'Acceptance Criteria' section".to_string(),
            impact: "-20%".to_string(),
            suggestion: "Add '## Acceptance Criteria' section with measurable criteria".to_string(),
        });
    }

    // Count requirements
    let req_re = Regex::new(r"(?m)^(FR|NFR|R)-\d{3}:").unwrap();
    let requirement_count = req_re.find_iter(prd_content).count();

    // Count acceptance criteria
    let ac_re = Regex::new(r"(?m)^AC-\d{3}:").unwrap();
    let ac_count = ac_re.find_iter(prd_content).count();

    if requirement_count > 0 {
        let coverage_ratio = ac_count as f32 / requirement_count as f32;
        let coverage_score = (coverage_ratio * 40.0).min(40.0);
        score += coverage_score;

        if coverage_ratio < 0.8 {
            issues.push(QualityIssue {
                id: format!("CHK-{:03}", issues.len() + 1),
                category: "testability".to_string(),
                severity: Severity::Important,
                description: format!(
                    "Low acceptance criteria coverage: {}/{} requirements ({:.0}%)",
                    ac_count,
                    requirement_count,
                    coverage_ratio * 100.0
                ),
                impact: format!("-{:.0}%", 40.0 - coverage_score),
                suggestion:
                    "Add acceptance criteria for all requirements (target: 1+ AC per requirement)"
                        .to_string(),
            });
        }
    }

    // Check if criteria are measurable (have numbers/metrics)
    let metrics_re =
        Regex::new(r"(<|>|<=|>=)?\s*\d+\s*(ms|MB|KB|GB|%|RPS|req/s|users?|\d+)").unwrap();
    let measurable_count = prd_content
        .lines()
        .filter(|line| ac_re.is_match(line) && metrics_re.is_match(line))
        .count();

    if ac_count > 0 {
        let measurable_ratio = measurable_count as f32 / ac_count as f32;
        let measurable_score = (measurable_ratio * 20.0).min(20.0);
        score += measurable_score;

        if measurable_ratio < 0.5 {
            issues.push(QualityIssue {
                id: format!("CHK-{:03}", issues.len() + 1),
                category: "testability".to_string(),
                severity: Severity::Important,
                description: format!(
                    "Only {}/{} acceptance criteria are measurable ({:.0}%)",
                    measurable_count,
                    ac_count,
                    measurable_ratio * 100.0
                ),
                impact: format!("-{:.0}%", 20.0 - measurable_score),
                suggestion: "Add metrics to acceptance criteria (e.g., <100ms, >90% accuracy)"
                    .to_string(),
            });
        }
    }

    // Check for test scenarios
    let test_section_re = Regex::new(r"(?mi)^##\s+Test (Strategy|Scenarios|Plan)").unwrap();
    if test_section_re.is_match(prd_content) {
        score += 20.0;
    } else {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "testability".to_string(),
            severity: Severity::Important,
            description: "Missing test strategy/scenarios section".to_string(),
            impact: "-20%".to_string(),
            suggestion: "Add '## Test Strategy' section with test scenarios".to_string(),
        });
    }

    score.max(0.0)
}

/// Score consistency dimension (0-100%)
fn score_consistency(spec_id: &str, cwd: &Path, issues: &mut Vec<QualityIssue>) -> Result<f32> {
    // Use analyze_native to check consistency
    let inconsistencies = check_consistency(spec_id, cwd)?;

    if inconsistencies.is_empty() {
        return Ok(100.0);
    }

    // Count by severity
    let critical_count = inconsistencies
        .iter()
        .filter(|i| matches!(i.severity, Severity::Critical))
        .count();
    let important_count = inconsistencies
        .iter()
        .filter(|i| matches!(i.severity, Severity::Important))
        .count();
    let minor_count = inconsistencies
        .iter()
        .filter(|i| matches!(i.severity, Severity::Minor))
        .count();

    // Penalty calculation
    let penalty = (critical_count as f32 * 20.0)
        + (important_count as f32 * 10.0)
        + (minor_count as f32 * 5.0);
    let score = (100.0 - penalty).max(0.0);

    // Add summary issue
    issues.push(QualityIssue {
        id: format!("CHK-{:03}", issues.len() + 1),
        category: "consistency".to_string(),
        severity: if critical_count > 0 {
            Severity::Critical
        } else if important_count > 0 {
            Severity::Important
        } else {
            Severity::Minor
        },
        description: format!(
            "{} consistency issues found ({} critical, {} important, {} minor)",
            inconsistencies.len(),
            critical_count,
            important_count,
            minor_count
        ),
        impact: format!("-{:.0}%", penalty),
        suggestion: "Run /speckit.analyze for detailed consistency report".to_string(),
    });

    Ok(score)
}

/// Find SPEC directory from SPEC-ID

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_report_grade() {
        let report = QualityReport {
            spec_id: "SPEC-TEST-001".to_string(),
            overall_score: 92.0,
            completeness: 100.0,
            clarity: 85.0,
            testability: 90.0,
            consistency: 95.0,
            issues: vec![],
            recommendations: vec![],
        };

        assert_eq!(report.grade(), "A");
    }

    #[test]
    fn test_score_completeness_full() {
        let prd = r#"
## Problem Statement
Test problem

## Goals
Test goals

## Requirements
FR-001: Test requirement

## Acceptance Criteria
AC-001: Test criteria

## Test Strategy
Test strategy
"#;

        let mut issues = Vec::new();
        let score = score_completeness(prd, &mut issues);
        assert_eq!(score, 100.0);
        assert_eq!(issues.len(), 0);
    }

    #[test]
    fn test_score_completeness_partial() {
        let prd = r#"
## Problem Statement
Test problem

## Requirements
FR-001: Test requirement
"#;

        let mut issues = Vec::new();
        let score = score_completeness(prd, &mut issues);
        assert_eq!(score, 40.0); // Only 2/5 sections
        assert_eq!(issues.len(), 3); // Missing 3 sections
    }

    #[test]
    fn test_score_clarity_vague_language() {
        let prd = r#"
FR-001: System should be fast
FR-002: System might need caching
FR-003: We could add logging
FR-004: System should handle errors
FR-005: We might consider optimization
FR-006: System could benefit from caching
"#;

        let mut issues = Vec::new();
        let score = score_clarity(prd, &mut issues);
        assert!(score < 100.0); // Penalized for vague language (6 instances)
        assert!(
            issues
                .iter()
                .any(|i| i.description.contains("vague language"))
        );
    }

    #[test]
    fn test_score_testability() {
        let prd = r#"
## Acceptance Criteria
AC-001: Response time <100ms
AC-002: Supports >1000 users

## Requirements
FR-001: Fast response
FR-002: High capacity

## Test Strategy
Integration tests required
"#;

        let mut issues = Vec::new();
        let score = score_testability(prd, &mut issues);
        assert!(score >= 80.0); // Good testability
    }
}
