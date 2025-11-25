//! ACE Reflector - Deep outcome analysis with LLM
//!
//! Analyzes execution outcomes to extract patterns, identify successful strategies,
//! and discover new heuristics. This is the intelligence layer that makes ACE
//! more than just simple +/- scoring.

#![allow(dead_code)] // ACE reflection pending full integration

use super::ace_learning::ExecutionFeedback;
use serde::{Deserialize, Serialize};

/// Pattern extracted from reflection
#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PatternKind {
    Helpful,
    Harmful,
    Neutral,
}

/// Reflection analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
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
                prompt.push_str("\n");
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
    if let Some(diff) = &feedback.diff_stat {
        if diff.files > 5 || diff.insertions > 200 {
            return true;
        }
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
}
