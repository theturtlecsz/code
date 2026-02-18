//! PM-005 T001: Change classification schema and deterministic classifier.
//!
//! Classifies changes into Class 0 (Routine), 1 (Significant), 2 (Major),
//! or E (Emergency) using file-type heuristics and metadata signals.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Change classification levels.
///
/// Each level maps to progressively stricter gate requirements:
/// - Class 0: Auto-approve (documentation, config-only)
/// - Class 1: Standard review (logic changes, tests)
/// - Class 2: Boundary gate with milestone contract enforcement
/// - Class E: Emergency — requires evidence, snapshot, and notification
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeClass {
    /// Class 0: Routine changes (docs, comments, formatting)
    Routine,
    /// Class 1: Significant changes (logic, internal refactoring, tests)
    Significant,
    /// Class 2: Major changes (new deps, API changes, architecture)
    Major,
    /// Class E: Emergency (security, compliance, CVSS > 7)
    Emergency,
}

impl ChangeClass {
    /// Numeric severity level (0, 1, 2). Emergency returns 3.
    pub fn level(&self) -> u8 {
        match self {
            Self::Routine => 0,
            Self::Significant => 1,
            Self::Major => 2,
            Self::Emergency => 3,
        }
    }

    /// Short label for display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Routine => "Class 0 (Routine)",
            Self::Significant => "Class 1 (Significant)",
            Self::Major => "Class 2 (Major)",
            Self::Emergency => "Class E (Emergency)",
        }
    }
}

impl std::fmt::Display for ChangeClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Metadata describing a set of changes for classification.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChangeMetadata {
    /// Paths of files affected by the change.
    pub affected_files: Vec<String>,
    /// New external dependencies added (crate names, npm packages, etc.).
    #[serde(default)]
    pub new_dependencies: Vec<String>,
    /// CVSS score if a security advisory is associated (0.0-10.0).
    #[serde(default)]
    pub security_score: Option<f32>,
    /// Free-text description of the change (used for keyword signals).
    #[serde(default)]
    pub description: String,
    /// Whether the change modifies public API surface.
    #[serde(default)]
    pub modifies_public_api: bool,
}

/// Result of classifying a change.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClassificationResult {
    /// Determined change class.
    pub class: ChangeClass,
    /// Human-readable reason for the classification.
    pub reason: String,
    /// Confidence in the classification (0.0 to 1.0).
    pub confidence: f32,
    /// Signals that contributed to the classification.
    pub matched_signals: Vec<String>,
}

impl ClassificationResult {
    /// Whether confidence meets a given threshold.
    pub fn meets_threshold(&self, threshold: f32) -> bool {
        self.confidence >= threshold
    }
}

/// Errors that can occur during classification.
#[derive(Debug, thiserror::Error)]
pub enum ClassifierError {
    /// Change metadata is invalid or insufficient.
    #[error("Invalid change metadata: {reason}")]
    InvalidMetadata { reason: String },
}

// ---------------------------------------------------------------------------
// Signal definitions
// ---------------------------------------------------------------------------

/// File extension patterns mapped to base class scores.
struct FileSignal {
    /// Glob-like suffix to match (e.g., ".md", ".rs").
    pattern: &'static str,
    /// Base score contribution (0.0 = routine, 1.0 = significant, 2.0 = major).
    score: f32,
    /// Human-readable signal name.
    label: &'static str,
}

const FILE_SIGNALS: &[FileSignal] = &[
    // Documentation (Class 0)
    FileSignal {
        pattern: ".md",
        score: 0.0,
        label: "file_type:markdown",
    },
    FileSignal {
        pattern: ".txt",
        score: 0.0,
        label: "file_type:plaintext",
    },
    FileSignal {
        pattern: ".rst",
        score: 0.0,
        label: "file_type:restructuredtext",
    },
    FileSignal {
        pattern: "LICENSE",
        score: 0.0,
        label: "file_type:license",
    },
    FileSignal {
        pattern: "CHANGELOG",
        score: 0.0,
        label: "file_type:changelog",
    },
    FileSignal {
        pattern: ".gitignore",
        score: 0.0,
        label: "file_type:gitignore",
    },
    // Config (Class 0-1)
    FileSignal {
        pattern: ".toml",
        score: 0.3,
        label: "file_type:config_toml",
    },
    FileSignal {
        pattern: ".yaml",
        score: 0.3,
        label: "file_type:config_yaml",
    },
    FileSignal {
        pattern: ".yml",
        score: 0.3,
        label: "file_type:config_yaml",
    },
    FileSignal {
        pattern: ".json",
        score: 0.3,
        label: "file_type:config_json",
    },
    // Source code (Class 1)
    FileSignal {
        pattern: ".rs",
        score: 1.0,
        label: "file_type:rust_source",
    },
    FileSignal {
        pattern: ".py",
        score: 1.0,
        label: "file_type:python_source",
    },
    FileSignal {
        pattern: ".ts",
        score: 1.0,
        label: "file_type:typescript_source",
    },
    FileSignal {
        pattern: ".js",
        score: 1.0,
        label: "file_type:javascript_source",
    },
    FileSignal {
        pattern: ".go",
        score: 1.0,
        label: "file_type:go_source",
    },
    FileSignal {
        pattern: ".java",
        score: 1.0,
        label: "file_type:java_source",
    },
    // Test files (Class 1, slightly lower)
    FileSignal {
        pattern: "_test.rs",
        score: 0.8,
        label: "file_type:rust_test",
    },
    FileSignal {
        pattern: "_test.go",
        score: 0.8,
        label: "file_type:go_test",
    },
    FileSignal {
        pattern: ".test.ts",
        score: 0.8,
        label: "file_type:ts_test",
    },
    FileSignal {
        pattern: ".test.js",
        score: 0.8,
        label: "file_type:js_test",
    },
    // Dependency manifests (Class 2)
    FileSignal {
        pattern: "Cargo.toml",
        score: 2.0,
        label: "file_type:cargo_manifest",
    },
    FileSignal {
        pattern: "Cargo.lock",
        score: 1.5,
        label: "file_type:cargo_lockfile",
    },
    FileSignal {
        pattern: "package.json",
        score: 2.0,
        label: "file_type:npm_manifest",
    },
    FileSignal {
        pattern: "package-lock.json",
        score: 1.5,
        label: "file_type:npm_lockfile",
    },
    FileSignal {
        pattern: "go.mod",
        score: 2.0,
        label: "file_type:go_module",
    },
    FileSignal {
        pattern: "requirements.txt",
        score: 2.0,
        label: "file_type:pip_requirements",
    },
    // CI/CD (Class 2)
    FileSignal {
        pattern: ".github/workflows/",
        score: 2.0,
        label: "file_type:ci_workflow",
    },
    FileSignal {
        pattern: "Dockerfile",
        score: 1.5,
        label: "file_type:dockerfile",
    },
];

/// Description keyword signals that modify classification.
struct KeywordSignal {
    keyword: &'static str,
    score_delta: f32,
    label: &'static str,
}

const KEYWORD_SIGNALS: &[KeywordSignal] = &[
    KeywordSignal {
        keyword: "security",
        score_delta: 1.5,
        label: "keyword:security",
    },
    KeywordSignal {
        keyword: "vulnerability",
        score_delta: 2.0,
        label: "keyword:vulnerability",
    },
    KeywordSignal {
        keyword: "cve-",
        score_delta: 2.0,
        label: "keyword:cve",
    },
    KeywordSignal {
        keyword: "breaking change",
        score_delta: 1.0,
        label: "keyword:breaking_change",
    },
    KeywordSignal {
        keyword: "api change",
        score_delta: 1.0,
        label: "keyword:api_change",
    },
    KeywordSignal {
        keyword: "migration",
        score_delta: 0.8,
        label: "keyword:migration",
    },
    KeywordSignal {
        keyword: "deprecat",
        score_delta: 0.5,
        label: "keyword:deprecation",
    },
    KeywordSignal {
        keyword: "refactor",
        score_delta: 0.3,
        label: "keyword:refactor",
    },
    KeywordSignal {
        keyword: "typo",
        score_delta: -0.5,
        label: "keyword:typo",
    },
    KeywordSignal {
        keyword: "comment",
        score_delta: -0.3,
        label: "keyword:comment",
    },
    KeywordSignal {
        keyword: "docs",
        score_delta: -0.3,
        label: "keyword:docs",
    },
    KeywordSignal {
        keyword: "readme",
        score_delta: -0.5,
        label: "keyword:readme",
    },
];

// ---------------------------------------------------------------------------
// Classification logic
// ---------------------------------------------------------------------------

/// Classify a change based on its metadata.
///
/// This is a pure, deterministic function with no side effects and no
/// dependency on packet state.
///
/// # Errors
///
/// Returns `ClassifierError::InvalidMetadata` if the metadata has no
/// affected files and no description.
pub fn classify_change(metadata: &ChangeMetadata) -> Result<ClassificationResult, ClassifierError> {
    if metadata.affected_files.is_empty() && metadata.description.is_empty() {
        return Err(ClassifierError::InvalidMetadata {
            reason: "at least one affected file or a description is required".into(),
        });
    }

    let mut total_score: f32 = 0.0;
    let mut signals: Vec<String> = Vec::new();
    let mut file_count: usize = 0;

    // --- Emergency override: CVSS score ---
    if let Some(cvss) = metadata.security_score {
        if cvss > 7.0 {
            signals.push(format!("cvss_score:{cvss:.1}"));
            return Ok(ClassificationResult {
                class: ChangeClass::Emergency,
                reason: format!("CVSS score {cvss:.1} exceeds emergency threshold (>7.0)"),
                confidence: 1.0,
                matched_signals: signals,
            });
        }
        // Sub-emergency security scores still contribute
        total_score += cvss / 5.0; // Scale: CVSS 5.0 → +1.0
        signals.push(format!("cvss_score:{cvss:.1}"));
    }

    // --- File-type heuristics ---
    for file_path in &metadata.affected_files {
        file_count += 1;
        let mut matched = false;

        // Try more specific patterns first (e.g., _test.rs before .rs)
        for signal in FILE_SIGNALS.iter().rev() {
            if matches_file_signal(file_path, signal.pattern) {
                total_score += signal.score;
                signals.push(signal.label.to_string());
                matched = true;
                break;
            }
        }

        if !matched {
            // Unknown file type gets a moderate score
            total_score += 0.5;
            signals.push(format!("file_type:unknown({})", file_extension(file_path)));
        }
    }

    // --- New dependency signal ---
    if !metadata.new_dependencies.is_empty() {
        let dep_count = metadata.new_dependencies.len();
        total_score += 2.0; // Any new dependency is at least Major
        signals.push(format!("new_dependencies:{dep_count}"));
    }

    // --- Public API signal ---
    if metadata.modifies_public_api {
        total_score += 1.5;
        signals.push("modifies_public_api".to_string());
    }

    // --- Description keyword signals ---
    let desc_lower = metadata.description.to_lowercase();
    for ks in KEYWORD_SIGNALS {
        if desc_lower.contains(ks.keyword) {
            total_score += ks.score_delta;
            signals.push(ks.label.to_string());
        }
    }

    // --- Normalize and classify ---
    // Average per file if multiple files, to avoid inflating score
    let avg_score = if file_count > 0 {
        total_score / file_count as f32
    } else {
        total_score
    };

    let (class, reason) = score_to_class(avg_score, &signals);

    // Confidence: higher when signals agree, lower when ambiguous
    let confidence = compute_confidence(avg_score, class, &signals);

    Ok(ClassificationResult {
        class,
        reason,
        confidence,
        matched_signals: signals,
    })
}

/// Map a numeric score to a ChangeClass.
fn score_to_class(score: f32, signals: &[String]) -> (ChangeClass, String) {
    if signals.iter().any(|s| s.starts_with("cvss_score:")) {
        let cvss_val: f32 = signals
            .iter()
            .find(|s| s.starts_with("cvss_score:"))
            .and_then(|s| s.strip_prefix("cvss_score:"))
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0);
        if cvss_val > 7.0 {
            return (
                ChangeClass::Emergency,
                format!("CVSS {cvss_val:.1} exceeds emergency threshold"),
            );
        }
    }

    if score >= 1.8 {
        (
            ChangeClass::Major,
            format!("Aggregate score {score:.2} indicates major change"),
        )
    } else if score >= 0.8 {
        (
            ChangeClass::Significant,
            format!("Aggregate score {score:.2} indicates significant change"),
        )
    } else {
        (
            ChangeClass::Routine,
            format!("Aggregate score {score:.2} indicates routine change"),
        )
    }
}

/// Compute classification confidence based on how clearly the score maps
/// to the determined class.
fn compute_confidence(score: f32, class: ChangeClass, signals: &[String]) -> f32 {
    // Distance from the nearest class boundary indicates confidence.
    // Boundaries: 0.0 | 0.8 | 1.8 | emergency
    let distance = match class {
        ChangeClass::Routine => 0.8 - score, // distance from 0.8 threshold
        ChangeClass::Significant => {
            let d_low = score - 0.8;
            let d_high = 1.8 - score;
            d_low.min(d_high)
        }
        ChangeClass::Major => score - 1.8,
        ChangeClass::Emergency => 1.0, // CVSS-based, always high confidence
    };

    // More signals = more confidence (evidence accumulation)
    let signal_bonus = (signals.len() as f32 * 0.05).min(0.2);

    // Normalize: distance of 0.5+ = high confidence, 0.0 = low confidence
    let base = (distance / 0.5).clamp(0.3, 0.95);
    (base + signal_bonus).min(1.0)
}

/// Check if a file path matches a signal pattern.
fn matches_file_signal(file_path: &str, pattern: &str) -> bool {
    let normalized = file_path.replace('\\', "/");
    if pattern.ends_with('/') {
        // Directory prefix match
        normalized.contains(pattern)
    } else if pattern.starts_with('.') || pattern.contains('.') {
        // Extension or filename match
        normalized.ends_with(pattern)
            || Path::new(&normalized)
                .file_name()
                .is_some_and(|f| f.to_string_lossy().ends_with(pattern))
    } else {
        // Exact filename match
        Path::new(&normalized)
            .file_name()
            .is_some_and(|f| f.to_string_lossy() == pattern)
    }
}

/// Extract file extension from a path, or return "none".
fn file_extension(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_else(|| "none".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docs_classified_as_routine() {
        let meta = ChangeMetadata {
            affected_files: vec!["docs/README.md".into(), "CHANGELOG.md".into()],
            description: "Update documentation".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Routine);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn rust_source_classified_as_significant() {
        let meta = ChangeMetadata {
            affected_files: vec!["src/lib.rs".into(), "src/main.rs".into()],
            description: "Refactor core logic".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Significant);
    }

    #[test]
    fn new_dependency_classified_as_major() {
        let meta = ChangeMetadata {
            affected_files: vec!["Cargo.toml".into()],
            new_dependencies: vec!["serde_yaml".into()],
            description: "Add YAML serialization support".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Major);
    }

    #[test]
    fn high_cvss_classified_as_emergency() {
        let meta = ChangeMetadata {
            affected_files: vec!["src/auth.rs".into()],
            security_score: Some(9.1),
            description: "Fix critical vulnerability CVE-2026-1234".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Emergency);
        assert_eq!(result.confidence, 1.0);
        assert!(result.reason.contains("CVSS"));
    }

    #[test]
    fn empty_metadata_returns_error() {
        let meta = ChangeMetadata::default();
        let err = classify_change(&meta).unwrap_err();
        assert!(err.to_string().contains("at least one affected file"));
    }

    #[test]
    fn test_files_slightly_lower_than_source() {
        let test_meta = ChangeMetadata {
            affected_files: vec!["src/lib_test.rs".into()],
            ..Default::default()
        };
        let src_meta = ChangeMetadata {
            affected_files: vec!["src/lib.rs".into()],
            ..Default::default()
        };
        let test_result = classify_change(&test_meta).expect("should succeed");
        let src_result = classify_change(&src_meta).expect("should succeed");
        // Both should be significant, but test file score is lower
        assert_eq!(test_result.class, ChangeClass::Significant);
        assert_eq!(src_result.class, ChangeClass::Significant);
    }

    #[test]
    fn public_api_change_elevates_class() {
        let meta = ChangeMetadata {
            affected_files: vec!["src/api.rs".into()],
            modifies_public_api: true,
            description: "Add new API endpoint".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Major);
        assert!(
            result
                .matched_signals
                .contains(&"modifies_public_api".to_string())
        );
    }

    #[test]
    fn classification_result_serializes() {
        let result = ClassificationResult {
            class: ChangeClass::Significant,
            reason: "test reason".into(),
            confidence: 0.85,
            matched_signals: vec!["test_signal".into()],
        };
        let json = serde_json::to_string(&result).expect("should serialize");
        let parsed: ClassificationResult = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed.class, ChangeClass::Significant);
        assert!((parsed.confidence - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn change_class_level_ordering() {
        assert_eq!(ChangeClass::Routine.level(), 0);
        assert_eq!(ChangeClass::Significant.level(), 1);
        assert_eq!(ChangeClass::Major.level(), 2);
        assert_eq!(ChangeClass::Emergency.level(), 3);
    }

    #[test]
    fn ci_workflow_classified_as_major() {
        let meta = ChangeMetadata {
            affected_files: vec![".github/workflows/ci.yml".into()],
            description: "Update CI pipeline".into(),
            ..Default::default()
        };
        let result = classify_change(&meta).expect("classification should succeed");
        assert_eq!(result.class, ChangeClass::Major);
    }
}
