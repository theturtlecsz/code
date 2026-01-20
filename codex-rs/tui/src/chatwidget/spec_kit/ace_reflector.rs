//! ACE Reflector - Deep outcome analysis with LLM
//!
//! Analyzes execution outcomes to extract patterns, identify successful strategies,
//! and discover new heuristics. This is the intelligence layer that makes ACE
//! more than just simple +/- scoring.

use super::ace_learning::ExecutionFeedback;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Current ACE Frame schema version
pub const ACE_FRAME_SCHEMA_VERSION: &str = "ace_frame@1.0";

fn default_schema_version() -> String {
    ACE_FRAME_SCHEMA_VERSION.to_string()
}

/// Pattern extracted from reflection
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectedPattern {
    /// Short description of the pattern
    pub pattern: String,

    /// Why this pattern matters
    pub rationale: String,

    /// Whether this is a positive or negative pattern
    pub kind: PatternKind,

    /// Confidence in this pattern (0.0-1.0)
    pub confidence: f64,

    /// Suggested scope (global, specify, tasks, implement, test)
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum PatternKind {
    Helpful,
    Harmful,
    Neutral,
}

/// Reflection analysis result - the ACE Frame
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReflectionResult {
    /// Schema version for forward compatibility
    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    /// Patterns discovered from this execution
    pub patterns: Vec<ReflectedPattern>,

    /// Summary of what worked
    pub successes: Vec<String>,

    /// Summary of what failed
    pub failures: Vec<String>,

    /// Recommended focus areas for future runs
    pub recommendations: Vec<String>,

    /// Overall assessment
    pub summary: String,
}

/// Reflection prompt builder
pub struct ReflectionPromptBuilder {
    task_title: String,
    scope: String,
    feedback: ExecutionFeedback,
    code_snippet: Option<String>,
}

impl ReflectionPromptBuilder {
    pub fn new(task_title: String, scope: String, feedback: ExecutionFeedback) -> Self {
        Self {
            task_title,
            scope,
            feedback,
            code_snippet: None,
        }
    }

    #[allow(dead_code)] // Pending: code snippet analysis in reflection
    pub fn with_code_snippet(mut self, snippet: String) -> Self {
        self.code_snippet = Some(snippet);
        self
    }

    /// Build reflection prompt for LLM
    pub fn build(&self) -> String {
        let mut prompt = format!(
            r#"ROLE: ACE Reflector - Pattern extraction and outcome analysis

TASK: Analyze this spec-kit execution and extract reusable patterns.

## Execution Context
- Task: {task_title}
- Scope: {scope}
- Compile: {compile_status}
- Tests: {test_status}
- Lint issues: {lint_count}

## Execution Feedback
"#,
            task_title = self.task_title,
            scope = self.scope,
            compile_status = if self.feedback.compile_ok {
                "✅ OK"
            } else {
                "❌ FAILED"
            },
            test_status = if self.feedback.tests_passed {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            },
            lint_count = self.feedback.lint_issues,
        );

        // Add failing tests
        if !self.feedback.failing_tests.is_empty() {
            prompt.push_str("\n### Failing Tests\n");
            for test in &self.feedback.failing_tests {
                prompt.push_str(&format!("- {}\n", test));
            }
        }

        // Add stack traces
        if !self.feedback.stack_traces.is_empty() {
            prompt.push_str("\n### Error Traces\n```\n");
            for trace in &self.feedback.stack_traces {
                prompt.push_str(trace);
                prompt.push('\n');
            }
            prompt.push_str("```\n");
        }

        // Add code snippet if available
        if let Some(snippet) = &self.code_snippet {
            prompt.push_str(&format!("\n### Code Snippet\n```rust\n{}\n```\n", snippet));
        }

        prompt.push_str(
            r#"
## Your Task

Extract 1-5 actionable patterns from this execution. For each pattern:

1. **Pattern**: Short imperative statement (≤140 chars)
   Example: "Use tokio::sync::Mutex instead of std::sync::Mutex in async contexts"

2. **Rationale**: Why this pattern matters (1-2 sentences)
   Example: "std::sync::Mutex blocks threads, causing deadlocks in async runtime"

3. **Kind**: helpful|harmful|neutral
   - helpful: Positive pattern that led to success
   - harmful: Anti-pattern that caused failure
   - neutral: Observation worth noting

4. **Confidence**: 0.0-1.0 based on evidence strength

5. **Scope**: global|specify|tasks|implement|test

## Output Format

Return JSON:
```json
{
  "patterns": [
    {
      "pattern": "Use tokio::sync::Mutex in async contexts",
      "rationale": "Prevents thread blocking and deadlocks",
      "kind": "helpful",
      "confidence": 0.9,
      "scope": "implement"
    }
  ],
  "successes": ["Clear error messages", "Fast compilation"],
  "failures": ["Borrow checker conflicts", "Missing test coverage"],
  "recommendations": ["Add more integration tests", "Review async patterns"],
  "summary": "Execution succeeded but revealed async/await pattern gaps"
}
```

## Guidelines

- Focus on GENERALIZABLE patterns (not task-specific)
- Prefer helpful patterns from successes
- Extract harmful patterns from failures
- Only neutral if genuinely informative
- Confidence based on evidence strength
- Scope based on where pattern applies

## Begin Analysis
"#,
        );

        prompt
    }
}

/// Check if reflection should be triggered
pub fn should_reflect(feedback: &ExecutionFeedback) -> bool {
    // Reflect on interesting outcomes, not routine successes

    // Always reflect on failures (learn from mistakes)
    if !feedback.compile_ok || !feedback.tests_passed {
        return true;
    }

    // Reflect if there were lint issues (patterns to extract)
    if feedback.lint_issues > 0 {
        return true;
    }

    // Reflect on large changes (likely interesting patterns)
    if let Some(diff) = &feedback.diff_stat
        && (diff.files > 5 || diff.insertions > 200)
    {
        return true;
    }

    // Skip routine successes (no patterns to extract)
    false
}

/// Parse LLM response into ReflectionResult
pub fn parse_reflection_response(response: &str) -> Result<ReflectionResult, String> {
    // Try to extract JSON from response
    let json_start = response.find('{');
    let json_end = response.rfind('}');

    if let (Some(start), Some(end)) = (json_start, json_end) {
        let json_str = &response[start..=end];
        serde_json::from_str(json_str)
            .map_err(|e| format!("Failed to parse reflection JSON: {}", e))
    } else {
        Err("No JSON found in reflection response".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// D131: ACE Frame Persistence (Stub)
// ─────────────────────────────────────────────────────────────────────────────

use crate::memvid_adapter::LLMCaptureMode;
use std::path::{Path, PathBuf};

/// Persist ACE reflection result based on capture mode (D131)
///
/// - `capture=none`: Returns Ok(None), no file written (in-memory only)
/// - `capture=prompts_only` or `capture=full_io`: Writes to evidence directory
///
/// This follows the pattern established in maieutic.rs::persist_maieutic_spec.
pub fn persist_ace_frame(
    spec_id: &str,
    result: &ReflectionResult,
    capture_mode: LLMCaptureMode,
    cwd: &Path,
) -> Result<Option<PathBuf>, String> {
    match capture_mode {
        LLMCaptureMode::None => {
            // D131: capture=none runs in-memory only
            tracing::info!(
                spec_id = %spec_id,
                "ACE frame not persisted (capture_mode=none)"
            );
            Ok(None)
        }
        LLMCaptureMode::PromptsOnly | LLMCaptureMode::FullIo => {
            // D131: Persist ACE frame to evidence directory
            let evidence_dir = super::evidence::evidence_base_for_spec(cwd, spec_id);
            std::fs::create_dir_all(&evidence_dir).map_err(|e| {
                format!(
                    "Failed to create evidence directory {}: {}",
                    evidence_dir.display(),
                    e
                )
            })?;

            let filename = format!(
                "ace_milestone_{}.json",
                chrono::Local::now().format("%Y%m%d_%H%M%S")
            );
            let path = evidence_dir.join(&filename);

            // Ensure schema_version is populated before serialization
            let mut result_with_version = result.clone();
            if result_with_version.schema_version.is_empty() {
                result_with_version.schema_version = ACE_FRAME_SCHEMA_VERSION.to_string();
            }

            let json = serde_json::to_string_pretty(&result_with_version)
                .map_err(|e| format!("Failed to serialize ACE frame: {}", e))?;

            std::fs::write(&path, &json)
                .map_err(|e| format!("Failed to write ACE frame to {}: {}", path.display(), e))?;

            tracing::info!(
                spec_id = %spec_id,
                path = %path.display(),
                "ACE milestone frame persisted"
            );
            Ok(Some(path))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_reflect_on_failure() {
        let feedback = ExecutionFeedback::new().with_compile_ok(false);

        assert!(should_reflect(&feedback));
    }

    #[test]
    fn test_should_reflect_on_test_failure() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(false);

        assert!(should_reflect(&feedback));
    }

    #[test]
    fn test_should_not_reflect_on_routine_success() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(true)
            .with_lint_issues(0);

        assert!(!should_reflect(&feedback));
    }

    #[test]
    fn test_should_reflect_on_large_change() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(true)
            .with_tests_passed(true)
            .with_diff_stat(super::super::ace_route_selector::DiffStat::new(10, 300, 50));

        assert!(should_reflect(&feedback));
    }

    #[test]
    fn test_reflection_prompt_includes_context() {
        let feedback = ExecutionFeedback::new()
            .with_compile_ok(false)
            .with_failing_tests(vec!["test_async_deadlock".to_string()])
            .with_stack_traces(vec!["Error at mutex.rs:42".to_string()]);

        let builder = ReflectionPromptBuilder::new(
            "Add async mutex handling".to_string(),
            "implement".to_string(),
            feedback,
        );

        let prompt = builder.build();

        assert!(prompt.contains("Add async mutex handling"));
        assert!(prompt.contains("implement"));
        assert!(prompt.contains("test_async_deadlock"));
        assert!(prompt.contains("Error at mutex.rs:42"));
    }

    #[test]
    fn test_parse_reflection_response() {
        let response = r#"
Here's my analysis:

```json
{
  "patterns": [
    {
      "pattern": "Use tokio::sync::Mutex in async",
      "rationale": "Prevents blocking",
      "kind": "helpful",
      "confidence": 0.9,
      "scope": "implement"
    }
  ],
  "successes": ["Clear errors"],
  "failures": ["Borrow checker"],
  "recommendations": ["Add tests"],
  "summary": "Good progress"
}
```

Hope this helps!
"#;

        let result = parse_reflection_response(response).unwrap();
        assert_eq!(result.patterns.len(), 1);
        assert_eq!(
            result.patterns[0].pattern,
            "Use tokio::sync::Mutex in async"
        );
        assert_eq!(result.patterns[0].kind, PatternKind::Helpful);
    }

    /// D131: capture=none should not persist any artifacts
    #[test]
    fn test_capture_none_no_persisted_artifacts() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-PRIVATE";

        let reflection = ReflectionResult {
            schema_version: ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![],
            successes: vec!["Test".to_string()],
            failures: vec![],
            recommendations: vec![],
            summary: "Test".to_string(),
        };

        let result = persist_ace_frame(spec_id, &reflection, LLMCaptureMode::None, temp_dir.path());

        assert!(result.is_ok(), "persist_ace_frame should succeed");
        assert!(
            result.unwrap().is_none(),
            "capture=none should not create file"
        );

        // Verify no ACE files created
        let evidence_dir = super::super::evidence::evidence_base_for_spec(temp_dir.path(), spec_id);
        let has_ace = evidence_dir.exists()
            && std::fs::read_dir(&evidence_dir)
                .map(|entries| {
                    entries.filter_map(|e| e.ok()).any(|e| {
                        e.file_name()
                            .to_str()
                            .map(|n| n.starts_with("ace_milestone_"))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
        assert!(
            !has_ace,
            "No ACE files should be created when capture_mode=None"
        );
    }

    /// D131: capture=prompts_only should persist ACE frames
    #[test]
    fn test_ace_frame_persisted_for_prompts_only() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let spec_id = "SPEC-TEST-PERSIST";

        let reflection = ReflectionResult {
            schema_version: ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![],
            successes: vec!["Test".to_string()],
            failures: vec![],
            recommendations: vec![],
            summary: "Test".to_string(),
        };

        let result = persist_ace_frame(
            spec_id,
            &reflection,
            LLMCaptureMode::PromptsOnly,
            temp_dir.path(),
        );

        assert!(result.is_ok(), "persist_ace_frame should succeed");
        let path = result.unwrap();
        assert!(path.is_some(), "Should return file path");
        let path = path.unwrap();
        assert!(path.exists(), "File should exist");

        // Verify filename pattern
        let filename = path.file_name().unwrap().to_str().unwrap();
        assert!(
            filename.starts_with("ace_milestone_") && filename.ends_with(".json"),
            "Filename should match ace_milestone_*.json pattern"
        );
    }
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use schemars::schema_for;

    /// Test that schema generation produces stable output matching committed schema
    #[test]
    fn test_ace_frame_schema_generation_stable() {
        let schema = schema_for!(ReflectionResult);
        let generated = serde_json::to_value(&schema).unwrap();

        let committed_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../spec-kit/src/config/schemas/ace_frame.schema.v1.json"
        );

        let committed: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(committed_path).expect("schema file exists"),
        )
        .unwrap();

        // Compare definitions and properties (ignoring metadata like $schema, title)
        assert_eq!(
            generated.get("definitions"),
            committed.get("definitions"),
            "Schema definitions changed. Regenerate with: cargo run --bin ace-schema-gen -p codex-tui"
        );

        assert_eq!(
            generated.get("properties"),
            committed.get("properties"),
            "Schema properties changed. Regenerate with: cargo run --bin ace-schema-gen -p codex-tui"
        );
    }

    /// Test that schema_version field defaults correctly
    #[test]
    fn test_schema_version_field_required() {
        // JSON without schema_version should deserialize with default
        let json = r#"{
            "patterns": [],
            "successes": [],
            "failures": [],
            "recommendations": [],
            "summary": "test"
        }"#;

        let result: ReflectionResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.schema_version, ACE_FRAME_SCHEMA_VERSION);
    }

    /// Test that ACE Frame examples validate against schema
    #[test]
    fn test_ace_frame_examples_validate() {
        let frame = ReflectionResult {
            schema_version: ACE_FRAME_SCHEMA_VERSION.to_string(),
            patterns: vec![ReflectedPattern {
                pattern: "Use tokio::sync::Mutex".to_string(),
                rationale: "Prevents blocking".to_string(),
                kind: PatternKind::Helpful,
                confidence: 0.9,
                scope: "implement".to_string(),
            }],
            successes: vec!["Clear errors".to_string()],
            failures: vec![],
            recommendations: vec!["Add tests".to_string()],
            summary: "Good progress".to_string(),
        };

        let schema_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../spec-kit/src/config/schemas/ace_frame.schema.v1.json"
        );
        let schema: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).unwrap()).unwrap();

        let compiled = jsonschema::JSONSchema::compile(&schema).unwrap();
        let instance = serde_json::to_value(&frame).unwrap();

        let result = compiled.validate(&instance);
        assert!(result.is_ok(), "ACE Frame should validate against schema");
    }

    /// Test backward compatibility - old frames without schema_version should deserialize
    #[test]
    fn test_backward_compatibility_no_version() {
        let old_json = r#"{
            "patterns": [
                {
                    "pattern": "Old pattern",
                    "rationale": "Old reason",
                    "kind": "helpful",
                    "confidence": 0.8,
                    "scope": "global"
                }
            ],
            "successes": ["old success"],
            "failures": [],
            "recommendations": [],
            "summary": "old summary"
        }"#;

        let result: Result<ReflectionResult, _> = serde_json::from_str(old_json);
        assert!(
            result.is_ok(),
            "Old ACE frames without schema_version should deserialize"
        );

        let frame = result.unwrap();
        assert_eq!(frame.schema_version, ACE_FRAME_SCHEMA_VERSION);
        assert_eq!(frame.patterns.len(), 1);
    }
}
