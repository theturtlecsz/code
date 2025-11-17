# Unit Testing Guide

Comprehensive guide to writing effective unit tests.

---

## Overview

**Unit Testing Philosophy**: Test individual functions/components in isolation with no external dependencies

**Goals**:
- Fast execution (<1s for all unit tests)
- High coverage of business logic (70-80% for critical paths)
- Deterministic and isolated
- Easy to maintain

**Current Status**:
- ~380 unit tests (63% of total)
- 100% pass rate
- Average execution time: ~800ms

---

## Test Structure

### Arrange-Act-Assert Pattern

**Standard Pattern** for all unit tests:

```rust
#[test]
fn test_feature_behavior() {
    // Arrange: Setup test data
    let input = "test input";
    let expected = "expected output";

    // Act: Execute function under test
    let result = function_under_test(input);

    // Assert: Verify expectations
    assert_eq!(result, expected);
}
```

**Example from codebase** (clarify_native.rs:365):
```rust
#[test]
fn test_vague_language_detection() {
    // Arrange
    let detector = PatternDetector::default();
    let mut issues = Vec::new();

    // Act
    detector.check_vague_language("The system should be fast", 1, &mut issues);

    // Assert
    assert_eq!(issues.len(), 1);
    assert!(issues[0].question.contains("should"));
}
```

---

### Given-When-Then Pattern

**Alternative Pattern** for behavior-driven tests:

```rust
#[test]
fn test_example() {
    // Given: Initial state
    let state = StateBuilder::new("TEST").build();

    // When: Action occurs
    state.advance_stage();

    // Then: Expected outcome
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));
}
```

---

## Naming Conventions

### Test Function Names

**Format**: `test_{what}_{condition}_{expected}`

**Good Examples**:
```rust
#[test]
fn test_vague_language_detection() { }

#[test]
fn test_incomplete_markers_flagged_as_critical() { }

#[test]
fn test_quantifier_with_metrics_not_flagged() { }

#[test]
fn test_version_drift_detected_when_prd_newer() { }
```

**Bad Examples**:
```rust
#[test]
fn test1() { }  // ❌ Meaningless

#[test]
fn it_works() { }  // ❌ Too vague

#[test]
fn test_the_function() { }  // ❌ Not descriptive
```

---

### Test Module Organization

**In-Source Tests** (preferred for unit tests):

```rust
// src/chatwidget/spec_kit/clarify_native.rs

pub fn detect_ambiguities(prd_content: &str) -> Result<Vec<AmbiguityIssue>> {
    // Implementation...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vague_language_detection() {
        // Test implementation...
    }

    #[test]
    fn test_incomplete_markers() {
        // Test implementation...
    }
}
```

**Benefits**:
- ✅ Tests live next to code
- ✅ Private function access
- ✅ Excluded from release builds

---

## Testing Pure Functions

### What are Pure Functions?

**Definition**: Functions that:
1. Always return same output for same input
2. Have no side effects (no I/O, no mutations)
3. Don't depend on external state

**Why Test Them**: Easiest to test, highest value per test

---

### Example 1: Pattern Matching

**Function** (clarify_native.rs):
```rust
/// Check for vague language
fn check_vague_language(
    &self,
    line: &str,
    line_num: usize,
    issues: &mut Vec<AmbiguityIssue>,
) {
    for (pattern, severity, question, suggestion) in &self.vague_patterns {
        if let Some(mat) = Regex::new(pattern).unwrap().find(line) {
            issues.push(AmbiguityIssue {
                id: format!("AMB-{:03}", issues.len() + 1),
                severity: *severity,
                pattern_name: "vague_language".to_string(),
                question: question.to_string(),
                suggestion: suggestion.to_string(),
                // ...
            });
        }
    }
}
```

**Unit Test** (clarify_native.rs:365):
```rust
#[test]
fn test_vague_language_detection() {
    let detector = PatternDetector::default();
    let mut issues = Vec::new();

    detector.check_vague_language("The system should be fast", 1, &mut issues);

    assert_eq!(issues.len(), 1);
    assert!(issues[0].question.contains("should"));
    assert_eq!(issues[0].pattern_name, "vague_language");
}
```

**What Makes This a Good Test**:
- ✅ Tests one specific pattern (vague language)
- ✅ Verifies both detection and message content
- ✅ No external dependencies
- ✅ Fast (<1ms)

---

### Example 2: Conditional Logic

**Function** (clarify_native.rs:385):
```rust
fn check_quantifier_ambiguity(
    &self,
    line: &str,
    line_num: usize,
    issues: &mut Vec<AmbiguityIssue>,
) {
    for (pattern, question, suggestion) in &self.quantifier_patterns {
        if Regex::new(pattern).unwrap().is_match(line) {
            // Only flag if NO metrics present
            if !has_metrics(line) {
                issues.push(...);
            }
        }
    }
}
```

**Unit Tests** (clarify_native.rs:385):
```rust
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
```

**What Makes This a Good Test**:
- ✅ Tests both branches (with/without metrics)
- ✅ Clear positive and negative cases
- ✅ Reuses same detector (efficient)

---

## Testing Error Handling

### Testing Error Cases

**Pattern**: Verify function returns `Err` with expected error type

**Example 1: Missing File**:
```rust
#[test]
fn test_analyze_fails_when_prd_missing() {
    let temp_dir = TempDir::new().unwrap();
    let result = check_consistency("SPEC-TEST", temp_dir.path());

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("PRD.md not found"));
}
```

---

### Testing Error Messages

**Pattern**: Verify error messages are helpful

**Example**:
```rust
#[test]
fn test_error_message_includes_spec_id() {
    let result = find_spec_directory("SPEC-INVALID");

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("SPEC-INVALID"));
    assert!(err.to_string().contains("not found"));
}
```

---

### Testing Panic Conditions

**Use `should_panic` for panic tests**:

```rust
#[test]
#[should_panic(expected = "index out of bounds")]
fn test_invalid_index_panics() {
    let stages = vec![SpecStage::Plan];
    let _ = stages[10];  // Should panic
}
```

**Prefer `Result<()>` over panics**:
```rust
// Good: Returns error
fn validate_index(idx: usize) -> Result<()> {
    if idx >= 6 {
        return Err(anyhow!("Index {} out of range [0, 5]", idx));
    }
    Ok(())
}

// Bad: Panics
fn validate_index(idx: usize) {
    assert!(idx < 6, "Index out of range");
}
```

---

## Testing with Test Data

### Inline Test Data

**Pattern**: Small data inline in test

```rust
#[test]
fn test_requirement_extraction() {
    let prd_content = r#"
# PRD

## Requirements

- **R1**: User can log in
- **R2**: User can log out
    "#;

    let requirements = extract_requirements(prd_content);

    assert_eq!(requirements.len(), 2);
    assert_eq!(requirements[0].id, "R1");
    assert_eq!(requirements[1].id, "R2");
}
```

---

### External Test Fixtures

**Pattern**: Large data from files (see test-infrastructure.md)

```rust
#[test]
fn test_with_real_prd() -> Result<()> {
    let prd_path = "tests/fixtures/prds/SPEC-DEMO-prd.md";
    let content = std::fs::read_to_string(prd_path)?;

    let ambiguities = detect_ambiguities(&content)?;

    // Real PRD should have known ambiguities
    assert!(ambiguities.len() > 0);
    assert!(ambiguities.iter().any(|a| a.severity == Severity::Critical));

    Ok(())
}
```

---

### Generated Test Data

**Pattern**: Use proptest for fuzz testing (see property-testing-guide.md)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_regex_escape_never_panics(s in ".*") {
        // Should handle any string
        let escaped = regex_escape(&s);
        assert!(escaped.len() >= s.len());
    }
}
```

---

## Testing State Machines

### Example: SpecAutoState Transitions

**State Machine**:
- Plan → Tasks → Implement → Validate → Audit → Unlock

**Test Pattern**: Verify transitions

```rust
#[test]
fn test_stage_advancement() {
    let mut state = StateBuilder::new("SPEC-TEST")
        .starting_at(SpecStage::Plan)
        .build();

    // Initial state
    assert_eq!(state.current_stage(), Some(SpecStage::Plan));
    assert_eq!(state.current_index, 0);

    // Advance to Tasks
    state.advance_stage();
    assert_eq!(state.current_stage(), Some(SpecStage::Tasks));
    assert_eq!(state.current_index, 1);

    // Advance to Implement
    state.advance_stage();
    assert_eq!(state.current_stage(), Some(SpecStage::Implement));
    assert_eq!(state.current_index, 2);
}
```

---

### Testing Invalid Transitions

```rust
#[test]
fn test_cannot_advance_past_unlock() {
    let mut state = StateBuilder::new("SPEC-TEST")
        .starting_at(SpecStage::Unlock)
        .build();

    state.current_index = 5;  // Unlock (last stage)

    // Advancing should be no-op or return None
    state.advance_stage();
    assert_eq!(state.current_stage(), None);
}
```

---

### Testing State Invariants

```rust
#[test]
fn test_state_index_never_negative() {
    let state = StateBuilder::new("SPEC-TEST").build();

    // Type system prevents negative (usize)
    assert!(state.current_index >= 0);

    // But ensure index is valid
    assert!(state.current_index < 6);
}
```

---

## Testing Calculations

### Scoring Functions

**Example** (checklist_native.rs:350):

```rust
fn score_testability(prd_content: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 0.0;

    // Check for acceptance criteria (40%)
    let ac_re = Regex::new(r"(?mi)^###?\s+Acceptance Criteria").unwrap();
    if ac_re.is_match(prd_content) {
        score += 40.0;
    }

    // Check for test scenarios (20%)
    let test_re = Regex::new(r"(?mi)^##\s+Test (Strategy|Scenarios)").unwrap();
    if test_re.is_match(prd_content) {
        score += 20.0;
    }

    score.max(0.0)
}
```

**Unit Tests**:
```rust
#[test]
fn test_score_testability_perfect() {
    let prd = r#"
### Acceptance Criteria
- AC1: Test

## Test Strategy
- Test
    "#;

    let mut issues = Vec::new();
    let score = score_testability(prd, &mut issues);

    assert_eq!(score, 60.0);  // 40 + 20
    assert_eq!(issues.len(), 0);
}

#[test]
fn test_score_testability_missing_tests() {
    let prd = r#"
### Acceptance Criteria
- AC1: Test
    "#;

    let mut issues = Vec::new();
    let score = score_testability(prd, &mut issues);

    assert_eq!(score, 40.0);  // 40 (AC) + 0 (no tests)
    assert!(issues.iter().any(|i| i.category == "testability"));
}

#[test]
fn test_score_testability_zero() {
    let prd = "# PRD\n\nNo structure";

    let mut issues = Vec::new();
    let score = score_testability(prd, &mut issues);

    assert_eq!(score, 0.0);
    assert!(issues.len() > 0);
}
```

---

### Penalty Calculations

**Example** (checklist_native.rs:408):
```rust
fn score_consistency(issues: &[InconsistencyIssue]) -> f32 {
    let critical_count = issues.iter()
        .filter(|i| matches!(i.severity, Severity::Critical))
        .count();
    let important_count = issues.iter()
        .filter(|i| matches!(i.severity, Severity::Important))
        .count();

    let penalty = (critical_count as f32 * 20.0)
                + (important_count as f32 * 10.0);

    (100.0 - penalty).max(0.0)
}
```

**Unit Tests**:
```rust
#[test]
fn test_score_consistency_perfect() {
    let issues = vec![];
    let score = score_consistency(&issues);
    assert_eq!(score, 100.0);
}

#[test]
fn test_score_consistency_one_critical() {
    let issues = vec![
        InconsistencyIssue {
            severity: Severity::Critical,
            // ...
        }
    ];
    let score = score_consistency(&issues);
    assert_eq!(score, 80.0);  // 100 - 20
}

#[test]
fn test_score_consistency_multiple_issues() {
    let issues = vec![
        InconsistencyIssue { severity: Severity::Critical, /* ... */ },
        InconsistencyIssue { severity: Severity::Critical, /* ... */ },
        InconsistencyIssue { severity: Severity::Important, /* ... */ },
    ];
    let score = score_consistency(&issues);
    assert_eq!(score, 50.0);  // 100 - (2*20 + 1*10)
}

#[test]
fn test_score_consistency_floor_at_zero() {
    let issues = vec![
        InconsistencyIssue { severity: Severity::Critical, /* ... */ }; 10
    ];
    let score = score_consistency(&issues);
    assert_eq!(score, 0.0);  // Floor at 0 (would be -100)
}
```

---

## Testing Collections

### Testing Filters

```rust
#[test]
fn test_filter_critical_issues() {
    let issues = vec![
        AmbiguityIssue { severity: Severity::Critical, /* ... */ },
        AmbiguityIssue { severity: Severity::Important, /* ... */ },
        AmbiguityIssue { severity: Severity::Minor, /* ... */ },
    ];

    let critical: Vec<_> = issues.iter()
        .filter(|i| matches!(i.severity, Severity::Critical))
        .collect();

    assert_eq!(critical.len(), 1);
}
```

---

### Testing Sorting

**Example** (clarify_native.rs:313):
```rust
fn sort_by_severity(issues: &mut Vec<AmbiguityIssue>) {
    issues.sort_by(|a, b| match (&a.severity, &b.severity) {
        (Severity::Critical, Severity::Critical) => Ordering::Equal,
        (Severity::Critical, _) => Ordering::Less,
        (_, Severity::Critical) => Ordering::Greater,
        // ...
    });
}
```

**Unit Test**:
```rust
#[test]
fn test_sort_by_severity() {
    let mut issues = vec![
        AmbiguityIssue { severity: Severity::Minor, id: "1".into(), /* ... */ },
        AmbiguityIssue { severity: Severity::Critical, id: "2".into(), /* ... */ },
        AmbiguityIssue { severity: Severity::Important, id: "3".into(), /* ... */ },
    ];

    sort_by_severity(&mut issues);

    assert_eq!(issues[0].severity, Severity::Critical);
    assert_eq!(issues[1].severity, Severity::Important);
    assert_eq!(issues[2].severity, Severity::Minor);
}
```

---

### Testing Aggregations

```rust
#[test]
fn test_count_by_severity() {
    let issues = vec![
        AmbiguityIssue { severity: Severity::Critical, /* ... */ },
        AmbiguityIssue { severity: Severity::Critical, /* ... */ },
        AmbiguityIssue { severity: Severity::Important, /* ... */ },
    ];

    let counts = count_by_severity(&issues);

    assert_eq!(counts.critical, 2);
    assert_eq!(counts.important, 1);
    assert_eq!(counts.minor, 0);
}
```

---

## Testing String Manipulation

### Regex Matching

```rust
#[test]
fn test_requirement_id_extraction() {
    let line = "- **R42**: User can authenticate";
    let re = Regex::new(r"\*\*R(\d+)\*\*").unwrap();

    let cap = re.captures(line).unwrap();
    let id = &cap[1];

    assert_eq!(id, "42");
}
```

---

### String Transformations

**Example** (clarify_native.rs:334):
```rust
fn truncate_context(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}
```

**Unit Tests**:
```rust
#[test]
fn test_truncate_short_text() {
    let text = "Short";
    let result = truncate_context(text, 10);
    assert_eq!(result, "Short");
}

#[test]
fn test_truncate_long_text() {
    let text = "This is a very long text that should be truncated";
    let result = truncate_context(text, 10);
    assert_eq!(result, "This is a ...");
    assert_eq!(result.len(), 13);  // 10 + "..."
}

#[test]
fn test_truncate_exact_length() {
    let text = "Exactly10!";  // 10 chars
    let result = truncate_context(text, 10);
    assert_eq!(result, "Exactly10!");
}
```

---

### Regex Escaping

**Function** (clarify_native.rs:349):
```rust
fn regex_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\\' | '.' | '+' | '*' | '?' | '(' | ')' | '|'
            | '[' | ']' | '{' | '}' | '^' | '$' => {
                format!("\\{}", c)
            }
            _ => c.to_string(),
        })
        .collect()
}
```

**Unit Tests**:
```rust
#[test]
fn test_regex_escape_special_chars() {
    assert_eq!(regex_escape("a.b"), "a\\.b");
    assert_eq!(regex_escape("a*b"), "a\\*b");
    assert_eq!(regex_escape("a?b"), "a\\?b");
    assert_eq!(regex_escape("a(b)"), "a\\(b\\)");
}

#[test]
fn test_regex_escape_normal_chars() {
    assert_eq!(regex_escape("abc"), "abc");
    assert_eq!(regex_escape("123"), "123");
}

#[test]
fn test_regex_escape_multiple_special() {
    assert_eq!(regex_escape("a.b*c?"), "a\\.b\\*c\\?");
}
```

---

## Testing File Operations (with TempDir)

### Setup Pattern

```rust
use tempfile::TempDir;

#[test]
fn test_write_and_read_prd() -> Result<()> {
    // Arrange: Create temp directory
    let temp_dir = TempDir::new()?;
    let spec_dir = temp_dir.path().join("docs/SPEC-TEST-test");
    std::fs::create_dir_all(&spec_dir)?;

    let prd_path = spec_dir.join("PRD.md");
    let content = "# PRD\n\n## Goal\nTest";

    // Act: Write file
    std::fs::write(&prd_path, content)?;

    // Assert: Read and verify
    let read_content = std::fs::read_to_string(&prd_path)?;
    assert_eq!(read_content, content);

    Ok(())
    // TempDir auto-cleaned on drop
}
```

---

### Testing Directory Creation

```rust
#[test]
fn test_create_spec_directory() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let spec_id = "SPEC-TEST-001";

    let spec_dir = create_spec_directory(temp_dir.path(), spec_id)?;

    assert!(spec_dir.exists());
    assert!(spec_dir.is_dir());
    assert!(spec_dir.ends_with("SPEC-TEST-001-test"));

    Ok(())
}
```

---

## Testing with Mocks

### MockMcpManager Usage

**Pattern**: Replace real MCP with mock

```rust
#[tokio::test]
async fn test_consensus_fetch() -> Result<()> {
    // Arrange: Setup mock
    let mut mock = MockMcpManager::new();
    mock.add_fixture(
        "local-memory",
        "search",
        Some("SPEC-TEST plan"),
        json!({"memory": {"content": "Agent response"}})
    );

    // Act: Call function that uses MCP
    let results = fetch_consensus("SPEC-TEST", SpecStage::Plan, &mock).await?;

    // Assert: Verify results
    assert_eq!(results.len(), 1);

    Ok(())
}
```

See [test-infrastructure.md](test-infrastructure.md#mockmcpmanager) for details.

---

## Table-Driven Tests

### Pattern: Multiple Test Cases

```rust
#[test]
fn test_stage_index_mapping() {
    let test_cases = vec![
        (0, Some(SpecStage::Plan)),
        (1, Some(SpecStage::Tasks)),
        (2, Some(SpecStage::Implement)),
        (3, Some(SpecStage::Validate)),
        (4, Some(SpecStage::Audit)),
        (5, Some(SpecStage::Unlock)),
        (6, None),
        (100, None),
    ];

    for (index, expected) in test_cases {
        let mut state = StateBuilder::new("SPEC-TEST").build();
        state.current_index = index;

        assert_eq!(
            state.current_stage(),
            expected,
            "Failed for index {}",
            index
        );
    }
}
```

**Benefits**:
- ✅ Compact (many cases in one test)
- ✅ Easy to add new cases
- ✅ Clear failure messages

---

### Parameterized Tests (with rstest)

**Add to `Cargo.toml`**:
```toml
[dev-dependencies]
rstest = "0.18"
```

**Usage**:
```rust
use rstest::rstest;

#[rstest]
#[case("should", Severity::Important)]
#[case("must", Severity::Critical)]
#[case("TBD", Severity::Critical)]
#[case("TODO", Severity::Important)]
fn test_vague_language_severity(#[case] pattern: &str, #[case] expected: Severity) {
    let detector = PatternDetector::default();
    let mut issues = Vec::new();

    detector.check_vague_language(
        &format!("The system {} work", pattern),
        1,
        &mut issues
    );

    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].severity, expected);
}
```

---

## Common Assertions

### Equality

```rust
assert_eq!(actual, expected);
assert_ne!(actual, unexpected);
```

---

### Boolean

```rust
assert!(condition);
assert!(!condition);
```

---

### Contains

```rust
assert!(vec.contains(&item));
assert!(string.contains("substring"));
```

---

### Custom Messages

```rust
assert_eq!(
    actual,
    expected,
    "Expected {}, got {} (context: {})",
    expected,
    actual,
    context
);
```

---

### Floating Point

```rust
// Don't use assert_eq! for floats
// Use approx crate instead

use approx::assert_relative_eq;

assert_relative_eq!(actual, expected, epsilon = 0.001);
```

---

## Best Practices

### DO

**✅ Test one thing per test**:
```rust
#[test]
fn test_vague_language_detection() {
    // Only tests vague language, nothing else
}

#[test]
fn test_incomplete_markers() {
    // Only tests incomplete markers
}
```

---

**✅ Use descriptive names**:
```rust
#[test]
fn test_quantifier_with_metrics_not_flagged() {
    // Clear what's being tested
}
```

---

**✅ Test edge cases**:
```rust
#[test]
fn test_truncate_empty_string() {
    assert_eq!(truncate_context("", 10), "");
}

#[test]
fn test_score_consistency_floor_at_zero() {
    // Test penalty doesn't go negative
}
```

---

**✅ Keep tests independent**:
```rust
#[test]
fn test_a() {
    let state = StateBuilder::new("TEST-A").build();
    // Uses own state, doesn't affect other tests
}

#[test]
fn test_b() {
    let state = StateBuilder::new("TEST-B").build();
    // Independent
}
```

---

**✅ Use setup functions for common data**:
```rust
fn create_test_prd() -> String {
    r#"
# PRD
## Requirements
- **R1**: Test
    "#.to_string()
}

#[test]
fn test_with_prd() {
    let prd = create_test_prd();
    // Use prd...
}
```

---

### DON'T

**❌ Test implementation details**:
```rust
// Bad: Tests internal regex pattern
#[test]
fn test_regex_pattern_is_correct() {
    assert_eq!(VAGUE_PATTERN, r"(should|could|might)");
}

// Good: Tests behavior
#[test]
fn test_vague_language_detected() {
    // Tests that "should" is flagged
}
```

---

**❌ Rely on test execution order**:
```rust
// Bad: test_b depends on test_a running first
static mut SHARED_STATE: i32 = 0;

#[test]
fn test_a() {
    unsafe { SHARED_STATE = 42; }
}

#[test]
fn test_b() {
    unsafe { assert_eq!(SHARED_STATE, 42); }  // ❌ Flaky
}
```

---

**❌ Use magic numbers**:
```rust
// Bad
assert_eq!(score, 42.0);

// Good
const EXPECTED_SCORE: f32 = 42.0;
assert_eq!(score, EXPECTED_SCORE);

// Or explain inline
assert_eq!(score, 60.0);  // 40 (AC) + 20 (test strategy)
```

---

**❌ Test too much in one test**:
```rust
// Bad: Tests everything at once
#[test]
fn test_entire_quality_system() {
    // 100 lines of setup
    // Tests clarify, analyze, checklist
    // Hard to debug when fails
}

// Good: Split into focused tests
#[test]
fn test_clarify_detects_vague_language() { }

#[test]
fn test_analyze_finds_missing_requirements() { }

#[test]
fn test_checklist_scores_completeness() { }
```

---

**❌ Skip cleanup (use TempDir)**:
```rust
// Bad: Leaves files behind
#[test]
fn test_write_file() {
    std::fs::write("/tmp/test.txt", "data")?;
    // File persists after test
}

// Good: Auto-cleanup
#[test]
fn test_write_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    std::fs::write(temp_dir.path().join("test.txt"), "data")?;
    Ok(())
    // temp_dir dropped, files deleted
}
```

---

## Running Tests

### Run All Unit Tests

```bash
cd codex-rs
cargo test --lib
```

**Explanation**:
- `--lib`: Only library tests (no integration tests)
- Runs all `#[cfg(test)] mod tests { }` blocks

---

### Run Specific Module

```bash
cargo test -p codex-tui --lib clarify_native
```

**Breakdown**:
- `-p codex-tui`: Package
- `--lib`: Unit tests only
- `clarify_native`: Module filter

---

### Run Specific Test

```bash
cargo test -p codex-tui test_vague_language_detection
```

**Output**:
```
running 1 test
test chatwidget::spec_kit::clarify_native::tests::test_vague_language_detection ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured
```

---

### Run with Output

```bash
cargo test -- --nocapture
```

**Shows** `println!()` output even for passing tests.

---

### Run with Threads

```bash
# Single-threaded (for debugging)
cargo test -- --test-threads=1

# Parallel (default)
cargo test -- --test-threads=8
```

---

## Test Coverage

### Measure Coverage

**Using tarpaulin**:
```bash
cargo tarpaulin -p codex-tui --lib
```

**Output**:
```
|| Tested/Total Lines:
|| tui/src/chatwidget/spec_kit/clarify_native.rs: 89/120
||
|| Coverage: 74.2%
```

---

### Improve Coverage

**Identify Untested Lines**:
```bash
cargo tarpaulin -p codex-tui --lib --out Html
open target/tarpaulin/index.html
```

**HTML Report** shows:
- ✅ Green: Covered
- ❌ Red: Not covered
- ⚠️ Yellow: Partially covered

---

## Summary

**Unit Testing Best Practices**:

1. **Structure**: Use Arrange-Act-Assert pattern
2. **Naming**: `test_{what}_{condition}_{expected}`
3. **Scope**: One thing per test
4. **Independence**: No shared state
5. **Speed**: Fast (<1ms typical)
6. **Coverage**: 70-80% for critical paths
7. **Cleanup**: Use `TempDir` for filesystem tests
8. **Mocks**: Use `MockMcpManager` for MCP

**Test Types Covered**:
- ✅ Pure functions (pattern matching, calculations)
- ✅ Error handling (missing files, invalid input)
- ✅ State machines (transitions, invariants)
- ✅ Collections (filtering, sorting, aggregation)
- ✅ String manipulation (regex, truncation, escaping)
- ✅ File operations (with TempDir)

**Next Steps**:
- [Integration Testing Guide](integration-testing-guide.md) - Cross-module tests
- [Property Testing Guide](property-testing-guide.md) - Generative testing
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures

---

**References**:
- Rust testing guide: https://doc.rust-lang.org/book/ch11-00-testing.html
- Example tests: `codex-rs/tui/src/chatwidget/spec_kit/*/tests.rs`
- Test infrastructure: `codex-rs/tui/tests/common/`
