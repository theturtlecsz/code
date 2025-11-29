# 🚀 Next Session: SPEC-947 Master Validation + Testing Infrastructure

**Created**: 2025-11-23
**Session Type**: Multi-Phase (Primary: SPEC-947 E2E Validation, Secondary: Test Infrastructure)
**Estimated Duration**: 8-14 hours (flexible phases)
**Status**: SPEC-947 UNBLOCKED - All dependencies complete ✅

---

## 📋 Session Objectives

### Phase 1: Quick Validation (~1h)
**Manual testing tasks to confirm functionality**

1. ✅ SPEC-954 Task 2: Drop cleanup verification (~10 min)
2. ✅ SPEC-954 Task 3: Long conversation stability (~20 min)
3. ✅ SPEC-946: Interactive model switching (~30 min)

### Phase 2: SPEC-947 Master Validation (4-6h) - **PRIMARY FOCUS**
**Comprehensive E2E testing of multi-provider system**

1. ✅ Model selection & auto-switching
2. ✅ CLI routing (Claude & Gemini)
3. ✅ ChatGPT OAuth validation
4. ✅ Config persistence
5. ✅ Error handling
6. ✅ UI validation

### Phase 3: Testing Infrastructure (3-7h)
**Selected improvements for mature test foundation**

1. ✅ Item 1: Refactor ChatWidget test layout (~1-2h)
2. ✅ Item 3: Enhanced stream-JSON parsing tests (~2h)
3. ✅ Item 4: Real CLI integration test (~1-2h)
4. ✅ Item 6: CI & coverage automation (~2-3h)

### Phase 4: Documentation Cleanup (~30 min)
**Archive outdated session handoff documents**

---

## 🎯 Phase 1: Quick Validation Tasks (1h)

### Context
These manual tests verify functionality that's difficult to automate. All features are implemented; we're just confirming they work as expected.

### Task 1.1: Drop Cleanup Verification (~10 min)

**Objective**: Verify CLI processes are killed when TUI exits

**Test Procedure**:
```bash
# 1. Start TUI
cd ~/code
./codex-rs/target/dev-fast/code

# 2. Create sessions with both providers
# In TUI:
/model claude-sonnet-4.5
"Hello from Claude"
# Wait for response

/model gemini-2.5-flash
"Hello from Gemini"
# Wait for response

# 3. Check active sessions
/sessions
# Note PIDs from output (CTRL+H to copy)

# 4. Exit TUI
# Press Ctrl-C or /exit

# 5. Wait 2 seconds
sleep 2

# 6. Verify no orphaned processes
ps aux | grep -E "claude|gemini" | grep -v grep
# Expected: No CLI processes (only grep itself)
```

**Success Criteria**:
- [ ] Multiple sessions created successfully
- [ ] PIDs visible in /sessions output
- [ ] All CLI processes terminated after TUI exit
- [ ] No zombie or orphaned processes

**If Processes Leak**:
- Document PIDs and process states
- Check logs for Drop trait errors
- Kill manually: `kill $(pgrep -f "claude|gemini")`
- Create issue in SPEC-954 notes

**Deliverable**: Update SPEC-954 spec.md Task 2 with results (pass/fail + evidence)

---

### Task 1.2: Long Conversation Stability (~20 min)

**Objective**: Verify 20+ turn conversations maintain stability

**Test Procedure**:
```bash
# 1. Start TUI
./codex-rs/target/dev-fast/code

# 2. Select a model
/model gemini-2.5-flash

# 3. Send 20-25 message pairs
# Use this script for consistency:
```

**Test Script** (run in TUI):
```
Turn 1: My name is Alice
Turn 2: What's my name?
Turn 3: I'm learning Rust
Turn 4: What language am I learning?
Turn 5: I have 3 cats named Bob, Carol, and Dave
Turn 6: How many cats do I have?
Turn 7: What are their names?
Turn 8: I live in Seattle
Turn 9: What city do I live in?
Turn 10: My favorite color is blue
Turn 11: What's my favorite color?
Turn 12: I work as a software engineer
Turn 13: What do I do for work?
Turn 14: I enjoy hiking on weekends
Turn 15: What do I enjoy doing?
Turn 16: Summarize everything you know about me
Turn 17: Did you remember my name correctly?
Turn 18: Did you remember all my cats?
Turn 19: What's the weather typically like in my city?
Turn 20: Based on everything, what hobbies would you recommend?
```

**Monitor During Test**:
```bash
# In another terminal:
# Watch memory usage
watch -n 2 "ps aux | grep 'codex-tui\|gemini' | grep -v grep"

# Check session validity
ls -lh ~/.gemini/sessions/  # or equivalent

# Monitor for errors in logs
tail -f ~/code/debug.log  # if logging enabled
```

**Success Criteria**:
- [ ] Complete all 20+ turns without errors
- [ ] Context preserved (Turn 16 mentions Alice, cats, Seattle, blue, etc.)
- [ ] Memory stable (no significant RSS growth)
- [ ] No performance degradation (response times consistent)
- [ ] Session files valid throughout

**Failure Scenarios to Document**:
- Context loss (model forgets earlier info)
- Memory leaks (RSS grows linearly)
- Session corruption (errors after N turns)
- Performance degradation (responses get slower)

**Deliverable**: Update SPEC-954 spec.md Task 3 with results + metrics

---

### Task 1.3: Model Preset Verification (~30 min)

**Objective**: Verify all 14 model presets work in TUI

**Test Procedure**:
```bash
# 1. Start TUI
./codex-rs/target/dev-fast/code

# 2. Test each preset systematically
/model
# Verify all 14 presets appear in selector

# 3. Test each model with simple prompt
```

**Test Matrix** (test each):

| Category | Model | Test Prompt | Expected |
|----------|-------|-------------|----------|
| **Gemini** | gemini-3-pro | "What's 2+2?" | Correct answer, ~5-10s |
| | gemini-2.5-pro | "Hello" | Brief response, ~5-10s |
| | gemini-2.5-flash | "Hi" | Quick response, ~3-7s |
| **Claude** | claude-opus-4.1 | "Test" | Response, ~5-15s |
| | claude-sonnet-4.5 | "Hello" | Response, ~5-15s |
| | claude-haiku-4.5 | "Hi" | Fast response, ~3-10s |
| **GPT-5.1** | gpt-5.1-mini | "2+2" | Fastest, ~2-5s |
| | gpt-5.1-minimal | "Hello" | Fast, ~3-6s |
| | gpt-5.1-low | "Hi" | Quick, ~4-7s |
| | gpt-5.1-medium | "Test" | Normal, ~5-10s |
| | gpt-5.1-high | "Puzzle" | Slower, ~8-15s |
| | gpt-5.1-codex-low | "def f():" | Code, ~4-7s |
| | gpt-5.1-codex-medium | "# function" | Code, ~5-10s |
| | gpt-5.1-codex-high | "algorithm" | Detailed, ~8-15s |

**Quick Test Script** (copy-paste each):
```
/model gemini-3-pro
What's 2+2?

/model gemini-2.5-pro
Hello

/model gemini-2.5-flash
Hi

/model claude-opus-4.1
Test

/model claude-sonnet-4.5
Hello

/model claude-haiku-4.5
Hi

/model gpt-5.1-mini
2+2

/model gpt-5.1-minimal
Hello

/model gpt-5.1-low
Hi

/model gpt-5.1-medium
Test

/model gpt-5.1-high
Puzzle

/model gpt-5.1-codex-low
def f():

/model gpt-5.1-codex-medium
# function

/model gpt-5.1-codex-high
algorithm
```

**Success Criteria**:
- [ ] All 14 presets appear in /model selector
- [ ] Each preset displays correct pricing
- [ ] Model switching works seamlessly
- [ ] Response times within expected ranges
- [ ] No errors or crashes

**Document Issues**:
- Models that fail to load
- Incorrect pricing displayed
- Unexpected response times
- Switching errors

**Deliverable**:
- Update SPEC-946 status in SPEC.md
- Create test results table with pass/fail for each model
- Store any issues to local-memory (importance ≥8)

---

## 🎯 Phase 2: SPEC-947 Master Validation (4-6h) - PRIMARY FOCUS

### Overview

**Purpose**: Comprehensive E2E validation that all 6 providers work correctly through appropriate auth methods

**Scope**:
1. Model selection & auto-switching
2. CLI routing for Claude & Gemini
3. ChatGPT OAuth validation
4. Config persistence across sessions
5. Error handling & recovery
6. UI/UX validation

### Prerequisites

**Load These Files First**:
1. `docs/SPEC-KIT-947-multi-provider-oauth-architecture/PRD.md` - Master validation spec
2. `SPEC.md` - Current project status
3. `CLAUDE.md` - Project guidelines
4. `codex-rs/common/src/model_presets.rs` - Model configuration

**Query Local-Memory**:
```
Search: "multi-provider CLI routing model switching"
Tags: ["spec:SPEC-KIT-952", "spec:SPEC-KIT-947"]
Limit: 10
```

### Task 2.1: Model Selection & Auto-Switching (1h)

**Objective**: Verify /model command works for all providers

**Test Cases**:

**TC-2.1.1: Model Selector Display**
```bash
# In TUI:
/model

# Verify:
- [ ] All 14 presets appear
- [ ] Grouped by provider (Gemini, Claude, GPT-5.1)
- [ ] Pricing displayed correctly
- [ ] Current model highlighted
```

**TC-2.1.2: Model Switching Within Provider**
```bash
# Start with GPT-5.1
/model gpt-5.1-minimal
"Test message 1"

# Switch to different GPT-5.1
/model gpt-5.1-high
"Test message 2"

# Verify:
- [ ] Switch successful
- [ ] Response uses new model (different reasoning level visible)
- [ ] No errors or warnings
```

**TC-2.1.3: Cross-Provider Switching**
```bash
# GPT → Claude
/model gpt-5.1-minimal
"Hello from GPT"

/model claude-sonnet-4.5
"Hello from Claude"

# Verify:
- [ ] Switch successful
- [ ] CLI routing activates (check /sessions)
- [ ] Response from Claude

# Claude → Gemini
/model gemini-2.5-flash
"Hello from Gemini"

# Verify:
- [ ] Switch successful
- [ ] New CLI session created
- [ ] Response from Gemini

# Gemini → GPT
/model gpt-5.1-minimal
"Back to GPT"

# Verify:
- [ ] Switch successful
- [ ] OAuth routing resumed
- [ ] Response from GPT
```

**TC-2.1.4: Rapid Switching**
```bash
# Switch 10 times quickly
/model gemini-2.5-flash
Hi
/model claude-haiku-4.5
Hi
/model gpt-5.1-mini
Hi
/model gemini-3-pro
Hi
/model claude-sonnet-4.5
Hi
/model gpt-5.1-codex-medium
Hi
/model gemini-2.5-pro
Hi
/model claude-opus-4.1
Hi
/model gpt-5.1-high
Hi
/model gemini-2.5-flash
Hi

# Verify:
- [ ] All switches successful
- [ ] No session leaks (/sessions shows clean state)
- [ ] No memory issues
- [ ] Responses from correct models
```

**Success Criteria**:
- [ ] All test cases pass
- [ ] Model switching < 1s
- [ ] No errors in any scenario
- [ ] Session management clean

**Deliverable**: Test results table with pass/fail for each TC

---

### Task 2.2: CLI Routing Validation (1-2h)

**Objective**: Verify Claude & Gemini CLI routing works correctly

**TC-2.2.1: CLI Installation Check**
```bash
# Pre-test: Verify CLIs installed
which claude
claude --version  # Should show version

which gemini
gemini --version  # Should show 0.17.0+

# If missing, install first:
# Claude: https://claude.ai/download
# Gemini: npm install -g @google/gemini-cli
```

**TC-2.2.2: Claude CLI Routing**
```bash
# In TUI:
/model claude-sonnet-4.5
"Write a Python hello world"

# Verify:
- [ ] Response streams in real-time
- [ ] Code formatted correctly
- [ ] Response time 2-25s
- [ ] /sessions shows claude session

# Multi-turn test
"Now make it print my name: Alice"

# Verify:
- [ ] Context preserved (references previous code)
- [ ] Session reused (same session ID in /sessions)
```

**TC-2.2.3: Gemini CLI Routing**
```bash
# In TUI:
/model gemini-2.5-flash
"What is 5 factorial?"

# Verify:
- [ ] Response streams
- [ ] Answer correct (120)
- [ ] Response time 7-15s
- [ ] /sessions shows gemini session with session_id

# Multi-turn test
"What did I just ask you to calculate?"

# Verify:
- [ ] Context preserved (mentions factorial)
- [ ] Session resumed (--resume flag used)
- [ ] Correct session ID in response
```

**TC-2.2.4: Session Management**
```bash
# Create multiple sessions
/model claude-opus-4.1
"Session A"

/model gemini-3-pro
"Session B"

/model claude-haiku-4.5
"Session C"

# Check sessions
/sessions

# Verify:
- [ ] 3 sessions listed (or appropriate count)
- [ ] Each has unique conv-id
- [ ] Session IDs present for Gemini
- [ ] PIDs visible
- [ ] Turn counts accurate

# Kill specific session
/sessions kill <conv-id>

# Verify:
- [ ] Session removed from list
- [ ] Process killed (ps aux | grep <provider>)
- [ ] Other sessions unaffected
```

**TC-2.2.5: Error Handling**
```bash
# Test with CLI not installed (if safe to test)
# Or test with invalid model name
/model invalid-model-name
"Test"

# Verify:
- [ ] Clear error message
- [ ] Suggestion to check model name
- [ ] No crash or hang
- [ ] Can recover and use valid model
```

**Success Criteria**:
- [ ] All CLI routing tests pass
- [ ] Multi-turn context works correctly
- [ ] Session management functional
- [ ] Error handling graceful

**Deliverable**: CLI routing validation report with test results

---

### Task 2.3: ChatGPT OAuth Validation (30 min)

**Objective**: Verify ChatGPT OAuth still works correctly

**TC-2.3.1: OAuth Flow**
```bash
# If not already authenticated:
# TUI should prompt for OAuth on first GPT model use

/model gpt-5.1-minimal
"Test OAuth"

# Verify:
- [ ] OAuth flow initiates (if needed)
- [ ] Browser opens for authentication (if needed)
- [ ] Token stored securely
- [ ] Response received
```

**TC-2.3.2: Token Persistence**
```bash
# Restart TUI
# Exit and restart: Ctrl-C, then ./codex-rs/target/dev-fast/code

/model gpt-5.1-minimal
"Test persistence"

# Verify:
- [ ] No re-authentication required
- [ ] Token loaded from storage
- [ ] Request successful
```

**TC-2.3.3: Multiple GPT Models**
```bash
# Test all GPT variants with same token
/model gpt-5.1-mini
"Mini test"

/model gpt-5.1-high
"High test"

/model gpt-5.1-codex-medium
"def test(): pass"

# Verify:
- [ ] All use same OAuth token
- [ ] No repeated auth prompts
- [ ] Reasoning levels work correctly
```

**Success Criteria**:
- [ ] OAuth flow works (if needed)
- [ ] Token persists across restarts
- [ ] All GPT models use OAuth correctly
- [ ] No auth errors

**Deliverable**: OAuth validation results

---

### Task 2.4: Config Persistence (30 min)

**Objective**: Verify configuration persists across sessions

**TC-2.4.1: Model Selection Persistence**
```bash
# Set model
/model gemini-2.5-flash
"Test"

# Exit TUI
# Restart TUI

# Verify:
- [ ] Same model selected on restart (or default behavior documented)
- [ ] Model preference saved (if applicable)
```

**TC-2.4.2: Session State Persistence**
```bash
# Create sessions
/model claude-sonnet-4.5
"Session 1"

/model gemini-2.5-pro
"Session 2"

# Exit TUI
# Restart TUI

/sessions

# Verify behavior:
- [ ] Sessions persist (if designed to)
- [ ] OR sessions clear (if ephemeral by design)
- [ ] Behavior documented and intentional
```

**Success Criteria**:
- [ ] Config persistence behavior verified
- [ ] Behavior matches design intent
- [ ] No data loss or corruption

**Deliverable**: Config persistence validation

---

### Task 2.5: Error Handling & Recovery (1h)

**Objective**: Verify graceful handling of error scenarios

**TC-2.5.1: Network Interruption**
```bash
# Simulate network issue (if safe):
# Disconnect WiFi briefly during request

/model gpt-5.1-minimal
"Test during disconnect"
# Disconnect network now
# Wait 10s
# Reconnect network

# Verify:
- [ ] Timeout error shown (not hang)
- [ ] Clear error message
- [ ] Can retry after reconnect
- [ ] System recovers
```

**TC-2.5.2: CLI Process Crash**
```bash
# In TUI:
/model claude-sonnet-4.5
"Test"

# In another terminal:
# Kill claude process mid-response
kill -9 $(pgrep -f "claude.*--model")

# Verify in TUI:
- [ ] Error detected and reported
- [ ] No TUI crash
- [ ] Can create new session
- [ ] Recovery is clean
```

**TC-2.5.3: Rate Limiting**
```bash
# Send many rapid requests
# (If not hitting real rate limits, document expected behavior)

for i in {1..10}; do
  /model gpt-5.1-minimal
  "Request $i"
done

# Verify:
- [ ] Rate limit handling (if applicable)
- [ ] Clear error messages
- [ ] Automatic retry (if implemented)
- [ ] OR clear guidance to user
```

**TC-2.5.4: Invalid Input**
```bash
# Test edge cases
/model claude-sonnet-4.5
""  # Empty message

/model gemini-2.5-flash
"x" * 10000  # Very long message

# Verify:
- [ ] Validation errors (if applicable)
- [ ] No crashes
- [ ] Clear feedback
```

**Success Criteria**:
- [ ] All error scenarios handled gracefully
- [ ] No crashes or hangs
- [ ] Clear error messages
- [ ] System recovers correctly

**Deliverable**: Error handling validation report

---

### Task 2.6: UI/UX Validation (30 min)

**Objective**: Verify user experience is smooth and intuitive

**TC-2.6.1: Streaming Experience**
```bash
# Test streaming with each provider
/model claude-sonnet-4.5
"Write a short story about a robot"

# Observe:
- [ ] Text streams in real-time
- [ ] No buffering delays
- [ ] Smooth character-by-character display
- [ ] Final formatting correct

/model gemini-2.5-flash
"Explain quantum computing"

# Observe same criteria

/model gpt-5.1-minimal
"List 10 fruits"

# Observe same criteria
```

**TC-2.6.2: Model Indicator**
```bash
# Verify current model always visible
/model gemini-3-pro
# Check status line / indicator

/model claude-haiku-4.5
# Check indicator updated

# Verify:
- [ ] Current model clearly shown
- [ ] Provider identified
- [ ] Cost/pricing visible (if shown)
```

**TC-2.6.3: Command Discoverability**
```bash
# Test help system
/help

# Verify:
- [ ] /model command documented
- [ ] /sessions command documented
- [ ] Examples provided
```

**Success Criteria**:
- [ ] Smooth user experience
- [ ] Clear model indication
- [ ] Good discoverability
- [ ] Professional polish

**Deliverable**: UX validation report

---

### SPEC-947 Completion Criteria

**All Must Pass**:
- [ ] All 6 model types functional (Gemini 3, Claude 3, GPT 8 variants)
- [ ] Provider routing correct (OAuth for GPT, CLI for Claude/Gemini)
- [ ] Model switching seamless
- [ ] Session management working
- [ ] Error handling graceful
- [ ] Config persistence verified
- [ ] UI/UX polished

**Deliverables**:
1. Comprehensive test report (all TCs with pass/fail)
2. Update SPEC-947 status to COMPLETE in SPEC.md
3. Store validation results to local-memory (importance: 9)
4. Document any issues found
5. Create follow-up SPECs for any failures

---

## 🎯 Phase 3: Testing Infrastructure Improvements (3-7h)

### Overview

Selected test improvements to create a robust testing foundation:
- Item 1: Refactor test layout (~1-2h)
- Item 3: Stream-JSON parsing tests (~2h)
- Item 4: CLI integration tests (~1-2h)
- Item 6: CI & coverage (~2-3h)

### Task 3.1: Refactor ChatWidget Test Layout (1-2h)

**Objective**: Extract tests from 22k-line mod.rs for maintainability

**Current State**:
- 14 tests embedded in `tui/src/chatwidget/mod.rs`
- ~370 lines of test code
- ~100 lines of test helpers

**Target Structure**:
```
tui/src/chatwidget/
├── mod.rs (22k → 21.6k lines)
├── tests.rs (NEW - 14 tests, ~370 lines)
├── test_support.rs (EXISTING - helpers, ~100 lines)
└── orderkey_tests.rs (EXISTING - OrderKey tests)
```

**Implementation Steps**:

1. **Create tests.rs module**:
```bash
cd ~/code/codex-rs/tui/src/chatwidget

# Read current test code
# Extract tests from mod.rs lines ~18363-18733
```

2. **Move tests to tests.rs**:
```rust
// tui/src/chatwidget/tests.rs
use super::*;
use crate::chatwidget::test_harness::TestHarness;

#[cfg(test)]
mod chatwidget_tests {
    use super::*;

    #[test]
    fn test_send_user_message() {
        // Moved from mod.rs
    }

    // ... move all 14 tests
}
```

3. **Update mod.rs**:
```rust
// At top of file with other modules
#[cfg(test)]
mod tests;
```

4. **Verify tests still pass**:
```bash
cd ~/code/codex-rs
cargo test -p codex-tui --lib chatwidget::tests --no-fail-fast
```

**Success Criteria**:
- [ ] All 14 tests moved to tests.rs
- [ ] mod.rs reduced by ~370 lines
- [ ] All tests pass: `cargo test -p codex-tui --lib`
- [ ] No compilation errors

**Deliverable**:
- Commit: "refactor(tui): Extract ChatWidget tests from mod.rs"
- Update test documentation

---

### Task 3.2: Enhanced Stream-JSON Parsing Tests (2h)

**Objective**: Add real CLI samples and property tests for robust parsing

**Current State**:
- Basic stream-JSON parsing tests exist
- No real CLI output samples
- Limited edge case coverage

**Implementation**:

**Step 1: Capture Real CLI Output** (30 min)
```bash
# Claude samples
mkdir -p ~/code/codex-rs/tests/samples

claude --print --output-format stream-json "test" > \
  ~/code/codex-rs/tests/samples/claude_stream_sample.jsonl

claude --print --output-format stream-json "write a haiku" > \
  ~/code/codex-rs/tests/samples/claude_stream_haiku.jsonl

# Gemini samples
gemini --output-format stream-json "test" > \
  ~/code/codex-rs/tests/samples/gemini_stream_sample.jsonl

gemini --output-format stream-json "what is 2+2" > \
  ~/code/codex-rs/tests/samples/gemini_stream_math.jsonl
```

**Step 2: Add Real Sample Tests** (30 min)
```rust
// codex-rs/core/src/cli_executor/claude_pipes.rs (in tests module)

#[test]
fn test_parse_real_claude_output() {
    let sample = include_str!("../../../tests/samples/claude_stream_sample.jsonl");

    let events: Vec<_> = sample.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .collect();

    assert!(!events.is_empty(), "Should parse real Claude output");

    // Verify event structure
    for event in events {
        assert!(event.get("type").is_some(), "Event should have type field");
        // Add more structural assertions
    }
}

#[test]
fn test_parse_real_gemini_output() {
    let sample = include_str!("../../../tests/samples/gemini_stream_sample.jsonl");
    // Similar assertions
}
```

**Step 3: Add Property Tests** (1h)
```rust
// Add to Cargo.toml dev-dependencies:
// proptest = "1.0"

use proptest::prelude::*;

proptest! {
    #[test]
    fn prop_parse_handles_arbitrary_whitespace(
        events in prop::collection::vec(
            prop::string::string_regex(r#"\{"type":".*"\}"#).unwrap(),
            1..10
        )
    ) {
        // Test that parser handles various whitespace scenarios
        let input = events.join("\n");

        // Should not panic
        let _ = parse_stream_json_lines(&input);
    }

    #[test]
    fn prop_parse_handles_malformed_gracefully(
        malformed in prop::string::string_regex(".*").unwrap()
    ) {
        // Should return error, not panic
        let result = parse_stream_json_line(&malformed);

        // Either Ok or Err, but no panic
        drop(result);
    }
}
```

**Success Criteria**:
- [ ] 4 real CLI samples captured
- [ ] 2 real sample tests added (Claude + Gemini)
- [ ] 2+ property tests added
- [ ] All tests pass
- [ ] Edge cases covered

**Deliverable**:
- Commit: "test(core): Add real CLI samples and property tests for stream-JSON parsing"
- Update TESTING.md with new test categories

---

### Task 3.3: Real CLI Integration Test (1-2h)

**Objective**: End-to-end validation with actual stdin/stdout

**Current State**:
- Unit tests mock CLI interactions
- No real subprocess tests

**Implementation**:

**Step 1: Add Test Dependencies** (5 min)
```toml
# codex-rs/Cargo.toml
[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

**Step 2: Create Integration Test** (1h)
```rust
// codex-rs/tests/cli_basic_integration.rs

use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;

#[test]
#[ignore] // Run with: cargo test -- --ignored
fn test_claude_cli_basic_invocation() {
    let mut cmd = Command::new("claude");

    cmd.arg("--print")
        .arg("--output-format").arg("stream-json")
        .arg("Hello")
        .timeout(Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type""#));
}

#[test]
#[ignore]
fn test_gemini_cli_basic_invocation() {
    let mut cmd = Command::new("gemini");

    cmd.arg("--output-format").arg("stream-json")
        .arg("Hello")
        .timeout(Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains(r#""type""#));
}

#[test]
#[ignore]
fn test_claude_multi_turn_session() {
    // Test using --resume flag
    let mut cmd1 = Command::new("claude");
    let output1 = cmd1
        .arg("--print")
        .arg("--output-format").arg("stream-json")
        .arg("My name is Alice")
        .timeout(Duration::from_secs(30))
        .output()
        .expect("Failed to execute");

    // Extract session ID from output
    let output_str = String::from_utf8_lossy(&output1.stdout);
    let session_id = extract_session_id(&output_str)
        .expect("Should have session ID");

    // Resume session
    let mut cmd2 = Command::new("claude");
    cmd2.arg("--print")
        .arg("--output-format").arg("stream-json")
        .arg("--resume").arg(&session_id)
        .arg("-p").arg("What's my name?")
        .timeout(Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"));
}

fn extract_session_id(output: &str) -> Option<String> {
    // Parse JSON output to find session_id
    // Implementation details...
    None // Placeholder
}
```

**Step 3: Add Test Documentation** (15 min)
```bash
# Update codex-rs/TESTING.md

## Integration Tests

Real CLI integration tests validate end-to-end functionality:

```bash
# Run integration tests (requires CLIs installed)
cargo test -p codex-tui --test cli_basic_integration -- --ignored

# Run specific test
cargo test -p codex-tui --test cli_basic_integration \
  test_claude_multi_turn_session -- --ignored --nocapture
```

**Success Criteria**:
- [ ] 3+ integration tests created
- [ ] Tests pass with CLIs installed
- [ ] Graceful skip if CLIs missing
- [ ] Documentation updated

**Deliverable**:
- Commit: "test: Add real CLI integration tests with assert_cmd"
- CI-ready tests (optional execution)

---

### Task 3.4: CI & Coverage Automation (2-3h)

**Objective**: Automated testing on every PR + coverage tracking

**Implementation**:

**Step 1: Create GitHub Actions Workflows** (1h)

```yaml
# .github/workflows/tui-tests.yml
name: TUI Tests

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy

    - name: Cache cargo registry
      uses: actions/cache@v3
      with:
        path: ~/.cargo/registry
        key: ${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo index
      uses: actions/cache@v3
      with:
        path: ~/.cargo/git
        key: ${{ runner.os }}-cargo-git-${{ hashFiles('**/Cargo.lock') }}

    - name: Cache cargo build
      uses: actions/cache@v3
      with:
        path: codex-rs/target
        key: ${{ runner.os }}-cargo-build-target-${{ hashFiles('**/Cargo.lock') }}

    - name: Check formatting
      run: cd codex-rs && cargo fmt --all -- --check

    - name: Run clippy
      run: cd codex-rs && cargo clippy --workspace --all-targets --all-features -- -D warnings

    - name: Run tests
      run: cd codex-rs && cargo test -p codex-tui --lib

    - name: Run snapshot tests
      run: cd codex-rs && cargo insta test --review
```

```yaml
# .github/workflows/coverage.yml
name: Coverage

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  coverage:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Install Rust
      uses: dtolnay/rust-toolchain@stable

    - name: Install tarpaulin
      run: cargo install cargo-tarpaulin

    - name: Generate coverage
      run: |
        cd codex-rs
        cargo tarpaulin --lib -p codex-tui \
          --out Xml --output-dir coverage

    - name: Upload coverage to Codecov
      uses: codecov/codecov-action@v3
      with:
        files: codex-rs/coverage/cobertura.xml
        flags: unittests
        name: codecov-umbrella

    - name: Generate HTML report
      run: |
        cd codex-rs
        cargo tarpaulin --lib -p codex-tui \
          --out Html --output-dir coverage

    - name: Upload HTML coverage
      uses: actions/upload-artifact@v3
      with:
        name: coverage-report
        path: codex-rs/coverage/
```

**Step 2: Add CI Badges to README** (15 min)
```markdown
# codex-rs

[![TUI Tests](https://github.com/theturtlecsz/code/workflows/TUI%20Tests/badge.svg)](https://github.com/theturtlecsz/code/actions/workflows/tui-tests.yml)
[![Coverage](https://codecov.io/gh/theturtlecsz/code/branch/main/graph/badge.svg)](https://codecov.io/gh/theturtlecsz/code)

Multi-provider TUI for ChatGPT, Claude, and Gemini.
```

**Step 3: Configure Coverage Targets** (30 min)
```bash
# Set up codecov.yml
cat > ~/code/.codecov.yml <<'EOF'
coverage:
  status:
    project:
      default:
        target: 60%
        threshold: 5%
    patch:
      default:
        target: 70%

ignore:
  - "codex-rs/tests/"
  - "codex-rs/**/tests/"
  - "**/*_test.rs"

comment:
  layout: "diff, flags, files"
  behavior: default
EOF
```

**Step 4: Local Coverage Testing** (30 min)
```bash
# Install tarpaulin locally
cargo install cargo-tarpaulin

# Generate coverage
cd ~/code/codex-rs
cargo tarpaulin --lib -p codex-tui --out Html --output-dir coverage

# Open report
open coverage/index.html  # or xdg-open on Linux

# Target: >60% coverage on critical modules
# - chatwidget/mod.rs (core logic)
# - test_harness.rs (test infrastructure)
# - providers/* (CLI routing)
```

**Success Criteria**:
- [ ] GitHub Actions workflows created
- [ ] Tests run on every PR
- [ ] Coverage tracked automatically
- [ ] Badges added to README
- [ ] Local coverage > 60% on critical modules

**Deliverable**:
- Commit: "ci: Add GitHub Actions for TUI tests and coverage tracking"
- Codecov integration (if account available)
- Coverage report in artifacts

---

## 🎯 Phase 4: Documentation Cleanup (30 min)

### Objective
Archive outdated session handoff documents to clean up repo while preserving history.

### Task 4.1: Archive Handoff Documents (30 min)

**Files to Archive**:
```
docs/NEXT-SESSION-PROMPT.md
docs/NEXT-SESSION-START-HERE.md
docs/NEXT-SESSION-TUI-FIXES.md
docs/NEXT-SESSION-TUI-TESTING-HANDOFF.md
docs/NEXT-SESSION-TUI-TESTING-PROMPT.md
docs/SESSION-HANDOFF-PROCESS-MGMT-COMPLETE.md
docs/gemini-cli-pipes-status.md
docs/gemini-pipes-session-complete.md
docs/gemini-pty-design.md
```

**Implementation**:
```bash
cd ~/code

# Create archive directory
mkdir -p docs/archive/session-handoffs-nov-2025

# Move handoff documents
mv docs/NEXT-SESSION-*.md docs/archive/session-handoffs-nov-2025/
mv docs/SESSION-HANDOFF-*.md docs/archive/session-handoffs-nov-2025/
mv docs/gemini-*.md docs/archive/session-handoffs-nov-2025/

# Create archive README
cat > docs/archive/session-handoffs-nov-2025/README.md <<'EOF'
# Session Handoff Documents (November 2025)

This directory contains session handoff documents from the multi-provider
CLI integration work (SPEC-952, SPEC-954, SPEC-955, SPEC-946).

## Context

These documents were created to hand off work between development sessions
during the implementation of:
- Gemini CLI integration (SPEC-952)
- Session management polish (SPEC-954)
- TUI test infrastructure (SPEC-955)
- Model preset expansion (SPEC-946)

## Status

All work documented here is **COMPLETE** as of 2025-11-23.

## Files

- `NEXT-SESSION-*.md`: Various next-session prompts and handoffs
- `SESSION-HANDOFF-*.md`: Completed session summaries
- `gemini-*.md`: Gemini CLI implementation notes

## Final Outcomes

- SPEC-952: ✅ COMPLETE (6 models via CLI routing)
- SPEC-954: ✅ COMPLETE (testing & polish)
- SPEC-955: ✅ COMPLETE (test infrastructure)
- SPEC-946: ✅ COMPLETE (14 model presets)

See `SPEC.md` in repo root for current project status.
EOF

# Commit archive
git add docs/archive/
git commit -m "docs: Archive session handoff documents from Nov 2025 work

All handoff documents from SPEC-952/954/955/946 work moved to archive.
Work is complete; documents preserved for historical reference.

Archived files:
- NEXT-SESSION-*.md (9 files)
- SESSION-HANDOFF-*.md
- gemini-*.md (3 files)

See docs/archive/session-handoffs-nov-2025/README.md for context.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

**Success Criteria**:
- [ ] All handoff docs moved to archive/
- [ ] Archive README created
- [ ] Committed with clear message
- [ ] docs/ directory clean

**Deliverable**: Clean repository with archived history

---

## 📊 Session Progress Tracking

### Phase Completion Checklist

**Phase 1: Quick Validation** (1h)
- [ ] Task 1.1: Drop cleanup (~10 min)
- [ ] Task 1.2: Long conversation (~20 min)
- [ ] Task 1.3: Model presets (~30 min)
- [ ] Update SPEC-954 & SPEC-946 in SPEC.md

**Phase 2: SPEC-947 Validation** (4-6h)
- [ ] Task 2.1: Model selection & switching (1h)
- [ ] Task 2.2: CLI routing validation (1-2h)
- [ ] Task 2.3: ChatGPT OAuth (30 min)
- [ ] Task 2.4: Config persistence (30 min)
- [ ] Task 2.5: Error handling (1h)
- [ ] Task 2.6: UI/UX validation (30 min)
- [ ] Create comprehensive validation report
- [ ] Update SPEC-947 to COMPLETE in SPEC.md
- [ ] Store results to local-memory

**Phase 3: Testing Infrastructure** (3-7h)
- [ ] Task 3.1: Refactor test layout (1-2h)
- [ ] Task 3.2: Stream-JSON parsing tests (2h)
- [ ] Task 3.3: CLI integration test (1-2h)
- [ ] Task 3.4: CI & coverage (2-3h)
- [ ] Update TESTING.md documentation
- [ ] Verify all tests passing

**Phase 4: Documentation Cleanup** (30 min)
- [ ] Archive handoff documents
- [ ] Create archive README
- [ ] Commit cleanup

### Flexible Stopping Points

**After Phase 1** (1h): Quick validation complete
**After Phase 2** (5-7h): SPEC-947 complete, major validation done
**After Phase 3.1-3.2** (8-10h): Core test improvements done
**After Phase 3** (10-14h): Full testing infrastructure mature
**After Phase 4** (10-15h): Complete session, everything done

---

## 🚀 Session Start Commands

### Load Context
```bash
# Navigate
cd ~/code

# Query local-memory
# Use: mcp__local-memory__search
# - query: "SPEC-947 multi-provider validation"
# - tags: ["spec:SPEC-KIT-947", "spec:SPEC-KIT-952"]
# - limit: 10

# Review this prompt
cat docs/NEXT-SESSION-SPEC-947-VALIDATION.md

# Check project status
cat SPEC.md | grep -A2 "SPEC-KIT-947\|SPEC-KIT-952\|SPEC-KIT-954\|SPEC-KIT-946"

# Verify build
~/code/build-fast.sh
```

### Start Phase 1: Quick Validation
```bash
# Launch TUI
./codex-rs/target/dev-fast/code

# Begin Task 1.1: Drop cleanup verification
# Follow test procedure in this document
```

---

## 📝 Success Metrics

### Must Complete
- [ ] SPEC-947 validated and marked COMPLETE
- [ ] All 6 provider types tested and working
- [ ] At least 2 testing infrastructure items complete

### Should Complete
- [ ] All 3 quick validation tasks done
- [ ] Comprehensive SPEC-947 validation report
- [ ] All 4 testing infrastructure items complete
- [ ] Documentation cleanup done

### Could Complete
- [ ] Bonus test coverage (>70%)
- [ ] Performance benchmarks
- [ ] Additional error scenarios

---

## 🎯 Local-Memory Storage Guidelines

**Store After Each Phase** (importance ≥8):

**Phase 1 Completion**:
```
Content: "Manual validation complete: Drop cleanup [PASS/FAIL], long conversations [metrics], model presets [14/14 or issues]. Pattern: [what worked, what didn't]. Files: SPEC-954 updated, SPEC-946 updated."
Tags: ["type:validation", "spec:SPEC-KIT-954", "spec:SPEC-KIT-946", "manual-testing"]
Importance: 8
```

**Phase 2 Completion**:
```
Content: "SPEC-947 Master Validation COMPLETE: All 6 providers tested [results summary]. Key findings: [issues found or all-pass]. Test coverage: [X/6 model types, Y/25 test cases]. Pattern: [validation approach that worked]. Files: SPEC-947 marked complete, validation report created."
Tags: ["type:completion", "spec:SPEC-KIT-947", "validation", "multi-provider"]
Importance: 9
```

**Phase 3 Completion**:
```
Content: "Testing infrastructure complete: [Item 1 refactor, Item 3 parsing, Item 4 integration, Item 6 CI]. Coverage: X%. Pattern: [test organization strategy]. Impact: [maintainability/CI benefits]."
Tags: ["type:infrastructure", "testing", "ci", "coverage"]
Importance: 8
```

---

## ⚠️ Known Risks & Mitigation

### Risk 1: CLI Not Installed
**Mitigation**: Check CLI availability first, document if missing, skip those tests

### Risk 2: Manual Tests Take Longer
**Mitigation**: Set timer for each task, skip if exceeding 2x estimate

### Risk 3: Test Infrastructure Complexity
**Mitigation**: Complete items in order, each is independent

### Risk 4: SPEC-947 Uncovers Issues
**Mitigation**: Document thoroughly, create follow-up SPECs, don't block on fixes

---

**Session Created**: 2025-11-23
**Ready to Execute**: Yes - All dependencies complete
**Expected Duration**: 8-14 hours (flexible phases)
**Success Probability**: High - All blocking work complete

🚀 **Ready to begin comprehensive validation and testing infrastructure work!**
