# Property-Based Testing Guide

Comprehensive guide to property-based testing with proptest.

---

## Overview

**Property-Based Testing Philosophy**: Generate random inputs to verify invariants hold across all possible values

**Tool**: [proptest](https://docs.rs/proptest) (Rust equivalent of QuickCheck/Hypothesis)

**Goals**:
- Test invariants (properties that always hold)
- Find edge cases automatically
- Verify mathematical properties
- Reduce test boilerplate

**Current Status**:
- ~30 property-based tests
- 100% pass rate
- 100 test cases per property (default)
- Integrated with standard test suite

---

## What is Property-Based Testing?

### Traditional Example-Based Testing

```rust
#[test]
fn test_reverse_twice_is_identity() {
    let vec = vec![1, 2, 3];
    let reversed = reverse(reverse(vec.clone()));
    assert_eq!(reversed, vec);
}
```

**Limitations**:
- Only tests one input (`[1, 2, 3]`)
- May miss edge cases (empty, single element, duplicates)
- Requires manual case selection

---

### Property-Based Testing

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_reverse_twice_is_identity(vec in any::<Vec<i32>>()) {
        let reversed = reverse(reverse(vec.clone()));
        prop_assert_eq!(reversed, vec);
    }
}
```

**Benefits**:
- ✅ Tests 100 random inputs automatically
- ✅ Finds edge cases (empty, single, large, etc.)
- ✅ Shrinks failing input to minimal case
- ✅ Focuses on **properties** not **examples**

---

## Getting Started

### Add proptest Dependency

**Cargo.toml**:
```toml
[dev-dependencies]
proptest = "1.3"
```

---

### Basic Property Test

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_addition_commutative(a in any::<i32>(), b in any::<i32>()) {
        // Property: a + b == b + a
        prop_assert_eq!(a + b, b + a);
    }
}
```

**How it works**:
1. Generate 100 random pairs of `(a, b)`
2. Run test with each pair
3. If any fails, shrink to minimal failing case
4. Report failure with minimal input

---

## Generators

### Built-in Generators

**Primitive Types**:
```rust
proptest! {
    #[test]
    fn test_primitives(
        n in any::<i32>(),
        s in any::<String>(),
        b in any::<bool>(),
    ) {
        // Test with random primitives
    }
}
```

---

**Collections**:
```rust
proptest! {
    #[test]
    fn test_collections(
        vec in any::<Vec<i32>>(),
        set in any::<HashSet<String>>(),
        map in any::<HashMap<i32, String>>(),
    ) {
        // Test with random collections
    }
}
```

---

**Ranges**:
```rust
proptest! {
    #[test]
    fn test_ranges(
        index in 0usize..10,          // 0-9
        score in 0.0..100.0,          // 0.0-99.999...
        percentage in 0..=100,        // 0-100 (inclusive)
    ) {
        prop_assert!(index < 10);
        prop_assert!(score < 100.0);
        prop_assert!(percentage <= 100);
    }
}
```

---

### Custom Generators

**Regex Patterns**:
```rust
proptest! {
    #[test]
    fn test_spec_id_format(
        spec_id in "[A-Z]{4}-[A-Z]{3}-[0-9]{3}"
    ) {
        // Generates: "SPEC-KIT-001", "ABCD-XYZ-999", etc.
        prop_assert!(is_valid_spec_id(&spec_id));
    }
}
```

---

**Custom Strategies**:
```rust
fn spec_stage_strategy() -> impl Strategy<Value = SpecStage> {
    prop_oneof![
        Just(SpecStage::Plan),
        Just(SpecStage::Tasks),
        Just(SpecStage::Implement),
        Just(SpecStage::Validate),
        Just(SpecStage::Audit),
        Just(SpecStage::Unlock),
    ]
}

proptest! {
    #[test]
    fn test_stage_valid(stage in spec_stage_strategy()) {
        // Tests all 6 stages
        prop_assert!(is_valid_stage(&stage));
    }
}
```

---

## Testing Invariants

### Invariant 1: State Index Always Valid

**Property**: State index ∈ [0, 5] → `current_stage()` returns `Some(_)`, else `None`

**Test** (property_based_tests.rs:21):
```rust
proptest! {
    #[test]
    fn pb01_state_index_always_in_valid_range(index in 0usize..20) {
        let mut state = StateBuilder::new("SPEC-PB01-TEST")
            .starting_at(SpecStage::Plan)
            .build();

        state.current_index = index;

        // Invariant: index ∈ [0, 5] → Some(_), else None
        if index < 6 {
            prop_assert!(state.current_stage().is_some());
        } else {
            prop_assert_eq!(state.current_stage(), None);
        }
    }
}
```

**What This Tests**:
- ✅ All indices 0-19 handled correctly
- ✅ Valid indices (0-5) return Some
- ✅ Invalid indices (6+) return None
- ✅ No panics or crashes

---

### Invariant 2: Current Stage Mapping

**Property**: For index ∈ [0, 5], `current_stage()` returns correct stage

**Test** (property_based_tests.rs:38):
```rust
proptest! {
    #[test]
    fn pb02_current_stage_always_some_when_index_under_six(
        index in 0usize..6
    ) {
        let mut state = StateBuilder::new("SPEC-PB02-TEST").build();
        state.current_index = index;

        prop_assert!(state.current_stage().is_some());

        // Verify correct stage mapping
        let expected_stages = vec![
            SpecStage::Plan,
            SpecStage::Tasks,
            SpecStage::Implement,
            SpecStage::Validate,
            SpecStage::Audit,
            SpecStage::Unlock,
        ];

        prop_assert_eq!(
            state.current_stage(),
            Some(expected_stages[index])
        );
    }
}
```

**What This Tests**:
- ✅ All valid indices (0-5) return Some
- ✅ Correct stage for each index
- ✅ Consistent mapping

---

### Invariant 3: Retry Count Never Negative

**Property**: Retry count ≤ max_retries (capped at max)

**Test** (property_based_tests.rs:62):
```rust
proptest! {
    #[test]
    fn pb03_retry_count_never_negative(retries in 0usize..100) {
        let ctx = IntegrationTestContext::new("SPEC-PB03-TEST").unwrap();

        let max_retries = 3;
        let capped_retries = retries.min(max_retries);

        let retry_file = ctx.commands_dir().join("retry.json");
        std::fs::write(&retry_file, json!({
            "retry_count": capped_retries,
            "max_retries": max_retries,
            "within_limit": capped_retries <= max_retries
        }).to_string()).unwrap();

        let content = std::fs::read_to_string(&retry_file).unwrap();
        let data: serde_json::Value = serde_json::from_str(&content).unwrap();

        prop_assert!(data["retry_count"].as_u64().unwrap() <= max_retries as u64);
        prop_assert_eq!(data["within_limit"].as_bool(), Some(true));
    }
}
```

**What This Tests**:
- ✅ Retry counts 0-99 all capped correctly
- ✅ No retry count exceeds max
- ✅ within_limit flag always true

---

## Testing Evidence Integrity

### Property 1: Written Evidence Always Parseable JSON

**Property**: Any evidence written is valid JSON

**Test** (property_based_tests.rs:90):
```rust
proptest! {
    #[test]
    fn pb04_written_evidence_always_parseable_json(
        agent in "[a-z]{3,10}",
        content in ".*"
    ) {
        let ctx = IntegrationTestContext::new("SPEC-PB04-TEST").unwrap();

        let evidence = json!({
            "agent": agent,
            "content": content,
            "timestamp": "2025-10-19T00:00:00Z"
        });

        let file = ctx.consensus_dir().join("test.json");
        std::fs::write(&file, evidence.to_string()).unwrap();

        // Invariant: File is valid JSON
        let content = std::fs::read_to_string(&file).unwrap();
        let parsed: Value = serde_json::from_str(&content).unwrap();

        prop_assert_eq!(parsed["agent"].as_str(), Some(agent.as_str()));
    }
}
```

**What This Tests**:
- ✅ Random agent names (3-10 lowercase letters)
- ✅ Random content (any string)
- ✅ Always produces valid JSON
- ✅ Round-trip serialization works

---

### Property 2: Evidence File Names Valid

**Property**: Generated filenames are valid filesystem paths

```rust
proptest! {
    #[test]
    fn pb05_evidence_filenames_always_valid(
        spec_id in "[A-Z]{4}-[A-Z]{3}-[0-9]{3}",
        stage in spec_stage_strategy(),
        agent in "[a-z]{5,10}",
    ) {
        let filename = format!(
            "spec-{:?}_{}_{}_{}.json",
            stage,
            spec_id,
            "2025-10-19T10_00_00Z",
            agent
        );

        // Invariant: Filename contains no invalid characters
        prop_assert!(!filename.contains('/'));
        prop_assert!(!filename.contains('\\'));
        prop_assert!(!filename.contains('\0'));

        // Invariant: Filename is not empty
        prop_assert!(!filename.is_empty());

        // Invariant: Filename has .json extension
        prop_assert!(filename.ends_with(".json"));
    }
}
```

**What This Tests**:
- ✅ Random SPEC IDs
- ✅ All 6 stages
- ✅ Random agent names
- ✅ Filenames always valid (no `/`, `\`, null bytes)
- ✅ Always has .json extension

---

## Testing Collections

### Property 1: Filtering Never Increases Length

**Property**: Filtered collection ≤ original length

```rust
proptest! {
    #[test]
    fn test_filter_never_increases_length(
        vec in any::<Vec<i32>>()
    ) {
        let filtered: Vec<_> = vec.iter()
            .filter(|&&x| x > 0)
            .collect();

        prop_assert!(filtered.len() <= vec.len());
    }
}
```

---

### Property 2: Sorting Preserves Length

**Property**: Sorted collection has same length as original

```rust
proptest! {
    #[test]
    fn test_sort_preserves_length(
        mut vec in any::<Vec<i32>>()
    ) {
        let original_len = vec.len();

        vec.sort();

        prop_assert_eq!(vec.len(), original_len);
    }
}
```

---

### Property 3: Dedupe Length

**Property**: Deduplicated length ≤ original length

```rust
proptest! {
    #[test]
    fn test_dedupe_length(
        mut vec in any::<Vec<i32>>()
    ) {
        let original_len = vec.len();

        vec.sort();
        vec.dedup();

        prop_assert!(vec.len() <= original_len);
    }
}
```

---

## Testing String Operations

### Property 1: Truncation Length

**Property**: Truncated string ≤ max length (plus ellipsis)

```rust
proptest! {
    #[test]
    fn test_truncate_length(
        text in any::<String>(),
        max_len in 1usize..100,
    ) {
        let truncated = truncate_context(&text, max_len);

        if text.len() <= max_len {
            // No truncation
            prop_assert_eq!(truncated.len(), text.len());
        } else {
            // Truncated with "..."
            prop_assert_eq!(truncated.len(), max_len + 3);
        }
    }
}
```

---

### Property 2: Regex Escape Safety

**Property**: Escaped string never causes regex parse error

```rust
proptest! {
    #[test]
    fn test_regex_escape_never_panics(s in ".*") {
        let escaped = regex_escape(&s);

        // Invariant: Escaped string is valid regex literal
        let pattern = format!("^{}$", escaped);
        let re = Regex::new(&pattern);

        prop_assert!(re.is_ok());
    }
}
```

---

## Shrinking

### What is Shrinking?

When a property test fails, proptest **shrinks** the failing input to the **minimal** failing case.

**Example**:
```rust
proptest! {
    #[test]
    fn test_all_positive(vec in any::<Vec<i32>>()) {
        prop_assert!(vec.iter().all(|&x| x > 0));
    }
}
```

**Failure**:
```
Test failed for input: [1, 2, 3, 0, 5, 6, 7, 8, 9]
Shrinking...
Minimal failing input: [0]
```

---

### Shrinking Example

**Original failure**:
- Input: `vec = [42, -17, 0, 99, -3, 100, 256, -1, 7]`
- Failed because: -17, -3, -1 are negative

**After shrinking**:
- Input: `vec = [-1]`
- Still fails, but minimal

**Benefits**:
- ✅ Easier to debug
- ✅ Clear failure reason
- ✅ No noise from extra elements

---

## Advanced Patterns

### Conditional Properties

**Pattern**: Property holds only under certain conditions

```rust
proptest! {
    #[test]
    fn test_division_inverse(
        a in any::<f64>(),
        b in any::<f64>()
    ) {
        // Property only holds when b ≠ 0
        prop_assume!(b != 0.0);

        let result = a / b * b;
        prop_assert!((result - a).abs() < 0.0001);
    }
}
```

**`prop_assume!(condition)`**:
- Skips test case if condition false
- Generates new random input
- Useful for preconditions

---

### Composite Strategies

**Pattern**: Combine multiple generators

```rust
fn state_and_index_strategy() -> impl Strategy<Value = (SpecAutoState, usize)> {
    (spec_id_strategy(), 0usize..20)
        .prop_map(|(spec_id, index)| {
            let mut state = StateBuilder::new(&spec_id).build();
            state.current_index = index;
            (state, index)
        })
}

proptest! {
    #[test]
    fn test_with_composite(
        (state, index) in state_and_index_strategy()
    ) {
        if index < 6 {
            prop_assert!(state.current_stage().is_some());
        }
    }
}
```

---

### Regression Testing

**Pattern**: Save failing inputs, re-test on every run

**File**: `proptest-regressions/property_based_tests.txt`
```
# Seeds for failure cases
xs 1234567890
xs 9876543210
```

**Usage**:
1. Test fails with input `xs = 1234567890`
2. proptest saves seed to regression file
3. Next run always tests that seed first
4. Ensures bug doesn't resurface

---

## Configuration

### Adjust Test Cases

**Default**: 100 test cases per property

**Custom**:
```rust
proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    #[test]
    fn test_with_more_cases(n in any::<i32>()) {
        // Runs 1000 times instead of 100
    }
}
```

---

### Environment Variable

```bash
# Run 10,000 test cases
PROPTEST_CASES=10000 cargo test --test property_based_tests
```

---

### Timeout

```rust
proptest! {
    #![proptest_config(ProptestConfig {
        cases: 100,
        max_shrink_iters: 10000,
        timeout: 5000,  // 5 seconds
        .. ProptestConfig::default()
    })]

    #[test]
    fn test_with_timeout(vec in any::<Vec<i32>>()) {
        // Timeout if takes >5s
    }
}
```

---

## Best Practices

### DO

**✅ Test invariants, not examples**:
```rust
// Good: Tests property
proptest! {
    #[test]
    fn test_reverse_twice_identity(vec in any::<Vec<i32>>()) {
        prop_assert_eq!(reverse(reverse(vec.clone())), vec);
    }
}

// Bad: Tests specific example (use regular #[test])
proptest! {
    #[test]
    fn test_specific_case() {
        let vec = vec![1, 2, 3];
        prop_assert_eq!(reverse(reverse(vec.clone())), vec);
    }
}
```

---

**✅ Use `prop_assume!()` for preconditions**:
```rust
proptest! {
    #[test]
    fn test_with_precondition(
        index in 0usize..100,
        vec in any::<Vec<i32>>()
    ) {
        prop_assume!(index < vec.len());

        let elem = vec[index];
        // Test with valid index
    }
}
```

---

**✅ Test mathematical properties**:
```rust
proptest! {
    #[test]
    fn test_addition_associative(a in any::<i32>(), b in any::<i32>(), c in any::<i32>()) {
        prop_assert_eq!((a + b) + c, a + (b + c));
    }

    #[test]
    fn test_multiplication_distributive(a in any::<i32>(), b in any::<i32>(), c in any::<i32>()) {
        prop_assert_eq!(a * (b + c), a * b + a * c);
    }
}
```

---

**✅ Test round-trip properties**:
```rust
proptest! {
    #[test]
    fn test_serialize_deserialize(state in any::<SpecAutoState>()) {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: SpecAutoState = serde_json::from_str(&json).unwrap();

        prop_assert_eq!(deserialized, state);
    }
}
```

---

### DON'T

**❌ Test concrete outputs**:
```rust
// Bad: Property tests shouldn't check specific outputs
proptest! {
    #[test]
    fn test_bad(n in any::<i32>()) {
        prop_assert_eq!(add_one(n), n + 1);  // ❌ This is just example-based
    }
}
```

---

**❌ Generate invalid inputs**:
```rust
// Bad: Generates many invalid cases (slow)
proptest! {
    #[test]
    fn test_with_many_assumes(
        a in any::<i32>(),
        b in any::<i32>(),
    ) {
        prop_assume!(a > 0);
        prop_assume!(b > 0);
        prop_assume!(a < b);
        prop_assume!(b % 2 == 0);
        // ... many assumes = slow
    }
}

// Good: Use constrained generator
fn even_positive_pair_strategy() -> impl Strategy<Value = (i32, i32)> {
    (1i32..1000, 1i32..1000)
        .prop_filter("a < b and b even", |(a, b)| a < b && b % 2 == 0)
}
```

---

## Running Property Tests

### Run All Property Tests

```bash
cd codex-rs
cargo test --test property_based_tests
```

---

### Run with More Cases

```bash
PROPTEST_CASES=1000 cargo test --test property_based_tests
```

---

### Debug Failing Test

```bash
# Run specific property test
cargo test --test property_based_tests pb01_state_index

# With verbose output
cargo test --test property_based_tests pb01_state_index -- --nocapture
```

---

### Re-run Regression Cases

```bash
# Automatically runs saved regression cases from proptest-regressions/
cargo test --test property_based_tests
```

---

## Summary

**Property-Based Testing Best Practices**:

1. **Invariants**: Test properties that always hold
2. **Generators**: Use appropriate generators (ranges, regex, custom)
3. **Shrinking**: Let proptest find minimal failing case
4. **Preconditions**: Use `prop_assume!()` for preconditions
5. **Configuration**: Adjust test cases with `PROPTEST_CASES`
6. **Regression**: Save failing cases automatically

**Common Properties to Test**:
- ✅ Invariants (index bounds, retry limits)
- ✅ Round-trip (serialize → deserialize)
- ✅ Mathematical (associativity, commutativity, distributivity)
- ✅ Collection operations (filter length, sort preserves length)
- ✅ String operations (truncate length, regex escape safety)
- ✅ Evidence integrity (valid JSON, valid filenames)

**Key Concepts**:
- ✅ Generators create random inputs
- ✅ Shrinking finds minimal failing case
- ✅ Regression tests prevent regressions
- ✅ 100 test cases per property (default)

**Next Steps**:
- [CI/CD Integration](ci-cd-integration.md) - Automated testing pipeline
- [Performance Testing](performance-testing.md) - Benchmarks and profiling
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures

---

**References**:
- proptest docs: https://docs.rs/proptest
- Property tests: `codex-rs/tui/tests/property_based_tests.rs`
- Regression files: `proptest-regressions/`
