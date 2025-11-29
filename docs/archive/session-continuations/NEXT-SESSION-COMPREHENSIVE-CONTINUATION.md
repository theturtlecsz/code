# NEXT SESSION: Complete SPEC-954 + Property Tests + CI Fix

**Copy this entire prompt into your next Claude Code session**

---

## 🎯 Session Overview

**Primary Focus**: Complete SPEC-954 remaining tasks + enhancements
**Secondary**: Fix CI automation (no time limit - solve completely)
**Estimated Time**: 4-6 hours total

**Session Goals**:
1. ✅ SPEC-954 Tasks 2-4 (Drop verification, stability, docs) - **45 minutes**
2. ✅ Fix .gitignore 'core' pattern - **15 minutes**
3. ✅ Add property-based tests expansion - **60 minutes**
4. ✅ Fix CI workflows completely (no time limit) - **2-4 hours**

---

## 📋 Phase 1: Complete SPEC-954 Tasks 2-4 (45 minutes)

### Context Files to Load
```bash
cat /home/thetu/code/docs/SPEC-KIT-954-session-management-polish/spec.md
cat /home/thetu/code/codex-rs/TESTING-CRITIQUE.md
```

### Local-Memory Query
```
Search: "SPEC-KIT-954 TUI Testing"
# Will find: 49fffde8-3715-4179-8093-e97a86267a18 (comprehensive session summary)
```

### Task 2: Drop Cleanup Verification (10 minutes)

**Goal**: Verify Drop trait actually kills Claude/Gemini processes

**Manual Test Plan**:
```bash
# 1. Build and run TUI
~/code/build-fast.sh run

# 2. Send messages to Claude & Gemini models
# Note the process PIDs via /sessions command

# 3. Exit TUI (Ctrl-C or :quit)
sleep 2

# 4. Verify processes killed
ps aux | grep -E "claude|gemini"
# Expected: No orphaned CLI processes

# If processes remain, document PIDs and behavior
```

**Deliverables**:
- [ ] Test procedure documented in SPEC-954 spec.md
- [ ] Results recorded (pass/fail)
- [ ] If failing: Create bug report with PIDs and symptoms
- [ ] If passing: Mark Task 2 complete in SPEC.md

**Files to update**:
- `docs/SPEC-KIT-954-session-management-polish/spec.md` (Task 2 status)

---

### Task 3: Long Conversation Stability Testing (20 minutes)

**Goal**: Verify session-based mode handles 20+ turn conversations

**Automated Test Approach**:
```bash
# Create test script
cat > /tmp/test_long_conversation.sh << 'EOF'
#!/bin/bash
# Send 25 message pairs and verify stability

for i in {1..25}; do
    echo "Turn $i: Testing context retention..."
    echo "Can you remember what turn number this is? (Turn $i)"
    sleep 2
done
EOF

chmod +x /tmp/test_long_conversation.sh

# Run with TUI
~/code/build-fast.sh run
# Execute test script or manually send 20+ messages
```

**Metrics to Track**:
- [ ] Successfully complete 20+ turn conversation
- [ ] Context preserved across all turns (ask about turn 1 at turn 20)
- [ ] Memory stable (monitor with `top` or `htop`)
- [ ] No performance degradation
- [ ] Session files remain valid throughout

**Deliverables**:
- [ ] Test results documented
- [ ] Memory usage logged (RSS before/after)
- [ ] Any issues or degradation noted
- [ ] Task 3 marked complete in SPEC.md

---

### Task 4: Model-Switching Limitation Documentation (15 minutes)

**Goal**: Document known limitation with global providers

**Root Cause** (from SPEC-954):
```rust
// Global provider with empty model (uses CLI default)
static CLAUDE_PROVIDER: OnceLock<ClaudePipesProvider> = OnceLock::new();
CLAUDE_PROVIDER.get_or_init(|| ClaudePipesProvider::with_cwd("", &cwd))
```

**Documentation to Create**:

Create `docs/SPEC-KIT-952-cli-routing/KNOWN-LIMITATIONS.md`:
```markdown
# Known Limitations - CLI Routing (SPEC-952)

## Model Switching in Session Mode

**Limitation**: Cannot switch between Claude models (opus/sonnet/haiku) within a session.

**Root Cause**: Global provider instances initialized with empty model string,
delegate to CLI default. Single provider instance per CLI type.

**Impact**:
- Selecting `/model claude-opus-4.1` then `/model claude-sonnet-4.5` won't switch models
- CLI continues using whichever model was initially selected

**Workaround**: Use ChatGPT account for model variety (supports runtime switching)

**Fix Estimate**: 2-3 hours
- Refactor providers to be keyed by model name
- Support multiple provider instances per CLI type
- Add model switching tests

**Priority**: P3 - Low (workaround available)
```

**Deliverables**:
- [ ] KNOWN-LIMITATIONS.md created
- [ ] Linked from SPEC-952 main docs
- [ ] Task 4 marked complete in SPEC.md

---

## 📋 Phase 2: Infrastructure Cleanup (15 minutes)

### Fix .gitignore 'core' Pattern

**Problem**: .gitignore line 154 `core` matches both core dumps AND codex-rs/core/ directory
**Impact**: Required force-add for 16 files this session

**Solution**:
```bash
# Edit .gitignore
# Change line 154-155:
# FROM:
core
core.*

# TO:
/core
/core.*
**/core.dump
**/core.[0-9]*
```

**Test**:
```bash
# Create dummy file in core/
touch codex-rs/core/src/test_file.rs

# Should NOT be ignored
git check-ignore -v codex-rs/core/src/test_file.rs
# Expected: No output (not ignored)

# Core dumps should still be ignored
touch core
git check-ignore -v core
# Expected: .gitignore:154:/core    core

# Cleanup
rm codex-rs/core/src/test_file.rs core
```

**Deliverables**:
- [ ] .gitignore updated with scoped 'core' pattern
- [ ] Tested (core dumps ignored, codex-rs/core/ not ignored)
- [ ] Committed with clear explanation
- [ ] Store pattern to local-memory (importance: 8)

---

## 📋 Phase 3: Property-Based Test Expansion (60 minutes)

### Context

Previous session added basic property tests. Now expand coverage.

**Existing Property Tests**:
- `core/src/cli_executor/claude_pipes.rs`: JSON parsing with proptest
- Basic coverage exists

**Goal**: Add comprehensive property tests for:
1. OrderKey invariants (transitivity, totality, consistency)
2. Message interleaving (adversarial event orderings)
3. Stream JSON parsing (random chunk boundaries)

---

### Test 1: OrderKey Properties (20 minutes)

Create `tui/src/chatwidget/orderkey_property_tests.rs`:

```rust
use proptest::prelude::*;
use super::OrderKey;

// Strategy for generating arbitrary OrderKeys
prop_compose! {
    fn arbitrary_orderkey()
        (req in 1u64..100, out in -10i32..10, seq in 0u64..1000)
        -> OrderKey
    {
        OrderKey { req, out, seq }
    }
}

proptest! {
    #[test]
    fn prop_orderkey_transitivity(
        keys in prop::collection::vec(arbitrary_orderkey(), 3..20)
    ) {
        // Property: If a < b and b < c, then a < c
        for i in 0..keys.len() {
            for j in i+1..keys.len() {
                for k in j+1..keys.len() {
                    if keys[i] < keys[j] && keys[j] < keys[k] {
                        prop_assert!(keys[i] < keys[k],
                            "Transitivity violated: {:?} < {:?} < {:?} but {:?} >= {:?}",
                            keys[i], keys[j], keys[k], keys[i], keys[k]);
                    }
                }
            }
        }
    }

    #[test]
    fn prop_orderkey_totality(
        a in arbitrary_orderkey(),
        b in arbitrary_orderkey()
    ) {
        // Property: For any two keys, exactly one of <, >, or == holds
        let cmp = a.cmp(&b);
        prop_assert!(
            matches!(cmp, std::cmp::Ordering::Less | std::cmp::Ordering::Equal | std::cmp::Ordering::Greater),
            "Comparison must yield a valid ordering"
        );
    }

    #[test]
    fn prop_orderkey_request_dominance(
        req1 in 1u64..100,
        req2 in 1u64..100,
        out1 in -10i32..10,
        out2 in -10i32..10,
        seq1 in 0u64..1000,
        seq2 in 0u64..1000
    ) {
        // Property: Request ordinal is primary sort key
        // If req1 < req2, then OrderKey(req1, *, *) < OrderKey(req2, *, *)
        if req1 < req2 {
            let key1 = OrderKey { req: req1, out: out1, seq: seq1 };
            let key2 = OrderKey { req: req2, out: out2, seq: seq2 };
            prop_assert!(key1 < key2,
                "Request {} should come before request {}, but {:?} >= {:?}",
                req1, req2, key1, key2);
        }
    }

    #[test]
    fn prop_orderkey_sorting_stable(
        mut keys in prop::collection::vec(arbitrary_orderkey(), 5..50)
    ) {
        // Property: Sorting is deterministic and stable
        let original = keys.clone();
        keys.sort();

        // Sort again
        let first_sort = keys.clone();
        keys.sort();

        // Should be identical
        prop_assert_eq!(first_sort, keys, "Sorting should be deterministic");

        // All elements preserved
        prop_assert_eq!(original.len(), keys.len(), "Sorting should preserve all elements");
    }
}
```

**Deliverables**:
- [ ] Create `tui/src/chatwidget/orderkey_property_tests.rs`
- [ ] Add to mod.rs: `mod orderkey_property_tests;`
- [ ] Run tests: `cargo test -p codex-tui orderkey_property --lib`
- [ ] Commit: "test(tui): Add comprehensive OrderKey property tests"

---

### Test 2: Message Interleaving Properties (25 minutes)

Add to `tui/src/chatwidget/test_harness.rs`:

```rust
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    // Strategy for generating random event orderings
    prop_compose! {
        fn arbitrary_event_ordering()
            (num_requests in 1usize..5,
             events_per_request in 2usize..6)
            -> Vec<(usize, usize, String)> // (request, seq, delta)
        {
            let mut events = Vec::new();
            for req in 0..num_requests {
                for seq in 0..events_per_request {
                    events.push((req, seq, format!("req{}-seq{}", req, seq)));
                }
            }
            events
        }
    }

    proptest! {
        #[test]
        fn prop_events_never_interleave(
            event_ordering in arbitrary_event_ordering()
        ) {
            // Property: Regardless of event arrival order,
            // messages are grouped by request in history

            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                let mut harness = TestHarness::new();

                // Send user messages
                let num_requests = event_ordering.iter()
                    .map(|(req, _, _)| req)
                    .max()
                    .unwrap_or(&0) + 1;

                for i in 0..num_requests {
                    harness.send_user_message(&format!("Request {}", i));
                }

                // Send events in scrambled order
                use codex_core::protocol::{AgentMessageDeltaEvent, Event, EventMsg, OrderMeta};

                for (req, seq, delta) in event_ordering {
                    harness.send_codex_event(Event {
                        id: format!("req-{}", req),
                        event_seq: seq as u64,
                        msg: EventMsg::AgentMessageDelta(AgentMessageDeltaEvent {
                            delta: delta.clone(),
                        }),
                        order: Some(OrderMeta {
                            request_ordinal: (req + 1) as u64,
                            output_index: Some(seq as u32),
                            sequence_number: None,
                        }),
                    });
                }

                harness.drain_app_events();

                // Verify contiguity
                let (user_groups, assistant_groups) = harness.cells_by_turn();

                // Each request should form contiguous groups
                for group in user_groups.iter().chain(assistant_groups.iter()) {
                    for window in group.windows(2) {
                        prop_assert_eq!(
                            window[1], window[0] + 1,
                            "Group should be contiguous but found gap: {} -> {}",
                            window[0], window[1]
                        );
                    }
                }
            });
        }
    }
}
```

**Deliverables**:
- [ ] Add property_tests module to test_harness.rs
- [ ] Run: `cargo test -p codex-tui prop_events --lib -- --nocapture`
- [ ] Verify 256+ random scenarios tested
- [ ] Commit: "test(tui): Add property-based interleaving tests"

---

### Test 3: JSON Parsing Properties (15 minutes)

Expand `core/src/cli_executor/claude_pipes.rs` property tests:

```rust
proptest! {
    #[test]
    fn prop_json_parsing_chunk_boundaries(
        json_lines in prop::collection::vec(
            "[a-z]{10,100}",  // Random JSON content
            1usize..10
        )
    ) {
        // Property: JSON parsing works regardless of chunk boundaries
        let full_json = json_lines.join("\n");

        // Test with different chunk sizes
        for chunk_size in [1, 5, 10, 50, 100] {
            let chunks: Vec<String> = full_json
                .chars()
                .collect::<Vec<_>>()
                .chunks(chunk_size)
                .map(|c| c.iter().collect())
                .collect();

            // Parse should handle any chunking
            let result = parse_streaming_chunks(&chunks);

            // Should successfully parse or consistently fail
            // (not hang or panic)
            prop_assert!(
                result.is_ok() || result.is_err(),
                "Parser should handle chunk size {} deterministically",
                chunk_size
            );
        }
    }
}
```

**Deliverables**:
- [ ] Add to existing parsing tests
- [ ] Run: `cargo test -p codex-core prop_json --lib`
- [ ] Commit: "test(core): Expand JSON parsing property tests"

---

## 📋 Phase 2: Infrastructure Cleanup (15 minutes)

### Fix .gitignore 'core' Pattern

**Current State** (line 154-155):
```gitignore
# Core dumps (crash files)
core
core.*
```

**Problem**: Matches `codex-rs/core/` directory, required force-add for 16 files

**Solution**:
```gitignore
# Core dumps (crash files) - scoped to avoid matching codex-rs/core/
/core
/core.*
**/core.dump
**/core.[0-9]*

# Explicitly allow codex-rs/core/ directory
!codex-rs/core/
```

**Validation**:
```bash
# Test core dumps still ignored
touch core && git check-ignore -v core
# Expected: /core matches

# Test core/ directory not ignored
touch codex-rs/core/src/test.rs && git check-ignore -v codex-rs/core/src/test.rs
# Expected: No match (file not ignored)

# Cleanup
rm core codex-rs/core/src/test.rs
```

**Deliverables**:
- [ ] Update .gitignore with scoped pattern
- [ ] Test both cases (dumps ignored, directory allowed)
- [ ] Commit: "fix: Scope .gitignore 'core' pattern to avoid matching core/ directory"
- [ ] Store to local-memory (importance: 8, tags: ["type:pattern", "gitignore", "infrastructure"])

---

## 📋 Phase 3: Property-Based Test Expansion (60 minutes)

See detailed test implementations in Phase 1 above.

**Summary**:
- OrderKey property tests (transitivity, totality, request dominance, sorting stability)
- Message interleaving property tests (adversarial event orderings)
- JSON parsing property tests (random chunk boundaries)

**Expected Test Count Increase**:
- Before: 72 tests
- After: ~85-90 tests (+13-18 property tests with 256 cases each = 3,000+ scenarios)

**Validation**:
```bash
cargo test -p codex-tui --lib | grep "test result"
# Expected: 85-90 passed

cargo test -p codex-tui prop_ --lib
# Expected: All property tests pass
```

---

## 📋 Phase 4: Fix CI Workflows Completely (2-4 hours, NO LIMIT)

### Context

**Current Status**:
- Tests pass locally ✅ (72 tests, all green)
- CI fails with 85 compilation errors ❌
- Errors in unrelated files (textarea.rs, etc.)
- 9 debugging iterations attempted (documented in NEXT-SESSION-CI-DEBUGGING.md)

**Key Mystery**: Same code compiles locally but not in CI

---

### Step 1: Reproduce CI Environment Locally (30 minutes)

```bash
cd /home/thetu/code/codex-rs

# Match exact CI Rust version
rustup install 1.90.0
rustup override set 1.90.0
rustc --version
# Expected: rustc 1.90.0 (1159e78c4 2025-09-14)

# Clean build from scratch
cargo clean

# Reproduce exact CI command
cargo test --lib -p codex-tui -p codex-core -p codex-protocol --all-features 2>&1 | tee /tmp/local_ci_build.log

# Check for errors
grep "^error\[E" /tmp/local_ci_build.log | wc -l

# If errors appear: SUCCESS! We reproduced the CI environment
# If no errors: Environment difference is elsewhere (dependencies, features, etc.)
```

**Decision Point**:
- **If reproduced**: Fix the errors (now visible locally)
- **If not reproduced**: Investigate Cargo.lock, feature flags, dependencies

---

### Step 2: Check and Commit Cargo.lock (10 minutes)

```bash
# Check if Cargo.lock exists and is tracked
git ls-files codex-rs/Cargo.lock

# If missing or not tracked
cd /home/thetu/code/codex-rs
cargo generate-lockfile

# Verify it exists
ls -lh Cargo.lock

# Commit it
git add Cargo.lock
git commit -m "chore: Add Cargo.lock for reproducible builds

Ensures CI uses exact same dependency versions as local development.
May resolve CI compilation errors caused by dependency version differences.

SPEC-KIT-954"

git push origin main
```

**Wait 3 minutes**, then check CI:
```bash
gh run list --limit 1
# If passing: VICTORY! Cargo.lock was the issue
# If still failing: Continue to Step 3
```

---

### Step 3: Investigate Specific Compilation Errors (30-60 minutes)

**Error 1: `cannot find value 'srep'` in textarea.rs:1745**

```bash
# Find the error
rg "srep" tui/src/bottom_pane/textarea.rs -B5 -A5

# Check if it's a typo or missing variable
# Likely should be 'rep' or 's_rep' or defined elsewhere

# Fix the error based on context
```

**Error 2: `use of undeclared type 'Line'`**

```bash
# Find missing imports
rg "^use.*Line" tui/src/chatwidget/mod.rs | head -10

# Check if Line import is missing
# Should be: use ratatui::text::Line;

# Add missing import at top of file
```

**Error 3: `cannot find function 'new_user_approval_decision'`**

```bash
# Search for where it should be defined
rg "fn new_user_approval_decision" tui/src/

# If missing, check git history
git log -p --all -S "new_user_approval_decision" | head -50

# Restore from history or implement if removed accidentally
```

**Strategy**: Fix errors one by one, commit incrementally

---

### Step 4: Simplify Workflow If Needed (15 minutes)

If errors persist, try ultra-minimal workflow:

```yaml
name: TUI Tests (Minimal)

on:
  push:
    branches: [ main ]
    paths: ['codex-rs/tui/src/chatwidget/test_harness.rs']

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4

    - name: Install Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1
      with:
        toolchain: 1.90.0

    - name: Add Cargo.lock if missing
      working-directory: codex-rs
      run: |
        if [ ! -f Cargo.lock ]; then
          cargo generate-lockfile
        fi

    - name: Run ONLY test_harness tests
      working-directory: codex-rs
      run: cargo test -p codex-tui test_harness --lib

    - name: Run ONLY orderkey tests
      working-directory: codex-rs
      run: cargo test -p codex-tui orderkey --lib
```

**Rationale**: Minimal scope, specific tests only, locked Rust version

---

### Step 5: Debug with Verbose Output (if still failing)

```yaml
- name: Debug build with verbose output
  working-directory: codex-rs
  run: |
    cargo build -p codex-tui --lib --verbose 2>&1 | tee build.log

- name: Upload build log
  if: failure()
  uses: actions/upload-artifact@v4
  with:
    name: build-debug-log
    path: codex-rs/build.log
```

**Then**: Download build.log artifact and analyze locally

---

### Step 6: Nuclear Option - Fresh Environment Test (30 minutes)

```bash
# Create completely fresh directory
mkdir /tmp/test-ci-fresh
cd /tmp/test-ci-fresh

# Clone repo fresh
git clone https://github.com/theturtlecsz/code.git
cd code/codex-rs

# Use exact CI Rust version
rustup override set 1.90.0

# Try building
cargo test --lib -p codex-tui --all-features

# Compare with main repo
# If fresh clone works: Something in local repo state causing issues
# If fresh clone fails too: Confirms it's a real code issue
```

---

### CI Debugging Decision Tree

```
START
  ↓
Reproduced locally with Rust 1.90.0?
  ├─ YES → Fix errors locally, test, commit, push
  │         Expected time: 1-2 hours
  │
  └─ NO → Check Cargo.lock
            ├─ Missing → Add it, commit, push, check CI
            │            Expected time: 15 minutes
            │
            └─ Exists → Investigate feature flags
                        Run with --no-default-features
                        Run with specific feature combos
                        Expected time: 30-60 minutes

If still failing after all above:
  → Create minimal workflow (test_harness only)
  → Get THAT passing first
  → Incrementally add complexity
  Expected time: 1-2 hours
```

---

## 📊 Success Criteria

### Phase 1: SPEC-954 Complete
- [ ] Task 2: Drop verification tested and documented
- [ ] Task 3: Long conversation stability verified
- [ ] Task 4: Model-switching limitation documented
- [ ] SPEC-954 marked COMPLETE in SPEC.md

### Phase 2: Cleanup
- [ ] .gitignore 'core' pattern fixed
- [ ] Tested (dumps ignored, core/ allowed)
- [ ] Pattern stored to local-memory

### Phase 3: Property Tests
- [ ] OrderKey property tests (4+ tests)
- [ ] Interleaving property tests (1+ test)
- [ ] JSON parsing property tests (1+ test)
- [ ] All tests passing locally
- [ ] Test count: 85-90 total

### Phase 4: CI Working
- [ ] TUI Tests workflow passing ✅
- [ ] All steps green (fmt, tests, snapshots)
- [ ] Badge shows passing status
- [ ] Documented what fixed it

---

## 🚀 Quick Start Commands

```bash
# Load context
cd /home/thetu/code

# Check current state
git log --oneline -5
cargo test -p codex-tui test_harness --lib

# Query local-memory
# Search: "SPEC-KIT-954 TUI Testing"
# ID: 49fffde8-3715-4179-8093-e97a86267a18

# Read handoffs
cat docs/SPEC-KIT-954-session-management-polish/spec.md
cat docs/NEXT-SESSION-CI-DEBUGGING.md
cat docs/NEXT-SESSION-TUI-IMPROVEMENTS-CONTINUE.md
```

---

## 📚 Essential Context

**Previous Session Commits** (15 commits, 2025-11-23):
```
Main work:
- c639126a3: Fix test_harness.rs (28 errors)
- c0f8f8eeb: Strengthen invariants
- 6f1a88d38: Tighten snapshots

Security:
- 66960b17c: OAuth env vars (google, anthropic, openai)

CI (9 iterations, not yet passing):
- 9872d571d through 536b32098: Various CI fixes
```

**Current Test Status**:
- 72 tests total (all passing locally ✅)
- Test categories: harness (9), orderkey (14), parsing (25), integration (6), snapshot (3)
- Local compilation: Clean ✅
- CI compilation: Failing ❌ (85 errors, environment difference)

**Files Modified This Session**: 20+ files, +6000 lines

---

## 💡 Execution Strategy

**Hour 1**: SPEC-954 Tasks 2-4 + gitignore fix (fast wins)
**Hour 2**: Property-based test expansion (solid progress)
**Hour 3-6**: CI debugging with full commitment (solve completely)

**Checkpoints**:
- After Phase 1: Confirm SPEC-954 complete
- After Phase 2: Confirm gitignore fixed
- After Phase 3: Confirm property tests passing
- During Phase 4: Every 30 minutes assess CI progress

**Flexibility**: If CI solved quickly (<1h), add more property tests. If CI takes full 4h, accept that and document thoroughly.

---

## 🎯 Session Completion Criteria

**Minimum** (if CI proves intractable):
- ✅ SPEC-954 complete
- ✅ gitignore fixed
- ✅ Property tests added
- ⏸️ CI documented for future (already done)

**Ideal** (full completion):
- ✅ SPEC-954 complete
- ✅ gitignore fixed
- ✅ Property tests added (85-90 total tests)
- ✅ CI workflows passing
- ✅ Badges green
- ✅ Production-ready automation

---

## 📝 Handoff for Next-Next Session

**If CI Still Blocked After This Session**:
Create `NEXT-SESSION-CI-NUCLEAR-OPTIONS.md`:
- Consider removing CI from main branch
- Set up on separate ci-testing branch
- Or accept manual testing workflow
- Or use external CI service (CircleCI, etc.)

**If Everything Complete**:
Celebrate and move to new SPECs! Testing infrastructure is production-ready.

---

**END OF PROMPT**

**To start next session, paste this entire file and say**:
> "I'm ready to complete SPEC-954, add property tests, and fix CI. Let's start with Phase 1: SPEC-954 Tasks 2-4. **ultrathink**"
