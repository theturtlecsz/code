# 🚀 Next Session: Comprehensive Completion Sprint

**Created**: 2025-11-23
**Scope**: Multi-stream completion (Gemini, SPEC-954, Polish, Testing)
**Estimated Duration**: 13.5-20.5 hours (flexible, progress-based)
**Strategy**: Complete 4 work streams + full testing infrastructure improvements

---

## 📋 Session Objectives

**Primary Goals**:
1. ✅ Complete Gemini CLI integration (multi-turn test + TUI)
2. ✅ Complete SPEC-954 (session management polish - NOW UNBLOCKED)
3. ✅ Complete TUI polish items (quick UX wins)
4. ✅ Complete SPEC-946 (/model command TUI testing)
5. ✅ Complete TUI testing infrastructure (all 6 improvements)

**Success Criteria**:
- All work streams reach "Done" status in SPEC.md
- Tests: All relevant test suites passing
- Build: Successful compilation
- Documentation: Updated and accurate
- Commit: Comprehensive with clear history

---

## 🎯 Work Stream 1: Gemini CLI Integration (Priority 1)

**Status**: 90% complete, multi-turn test timing out
**Effort**: 1.5-3 hours (30-60 min debug + 1-2h TUI integration)
**Blocks**: SPEC-952 completion, Gemini model availability in TUI

### Context Files (Read First)
1. **`docs/NEXT-SESSION-START-HERE.md`** ⭐ PRIMARY - Complete overview
2. **`docs/gemini-pipes-session-complete.md`** - Session 1 summary
3. **`codex-rs/core/src/cli_executor/gemini_pipes.rs`** (620 lines) - Implementation

### Current State
✅ **Working**:
- Single-turn test: PASSES in 6.45s
- Manual multi-turn CLI: Works perfectly
- Architecture: Proven sound (one-shot + resume pattern)
- JSON parsing: Validated

❌ **Failing**:
- Multi-turn test: First turn times out after 120s
- Test location: `gemini_pipes.rs:585-618`

### Task Breakdown

**Task 1.1: Debug Multi-Turn Test Timeout** (30-60 min)

**Hypothesis**: Test harness issue, not core implementation (manual CLI works)

**Debug Steps**:
```bash
cd /home/thetu/code/codex-rs

# 1. Run with verbose logging
RUST_LOG=codex_core=trace cargo test -p codex-core --lib \
  cli_executor::gemini_pipes::tests::test_multi_turn_state \
  -- --ignored --nocapture 2>&1 | tee debug.log

# Look for:
# - "Starting turn"
# - "Read N bytes"
# - "EOF detected"
# - "Captured session_id"
# - "Turn complete"
```

**Fix Options** (in order of likelihood):
1. Process doesn't exit after result event → Add explicit `child.kill()` after result
2. Stderr handler blocks → Add timeout on stderr reader
3. BufReader waits forever → Set timeout on read operations
4. Test harness issue → Simplify test messages or increase timeout temporarily

**Success Criteria**:
- Multi-turn test passes consistently (<10s per turn)
- Both tests passing: test_single_turn_pipes, test_multi_turn_state
- No zombie processes (`ps aux | grep gemini` shows clean state)

**Task 1.2: TUI Integration** (1-2 hours)

**Prerequisites**: Multi-turn test passing ✅

**Steps**:
1. Update `tui/src/providers/gemini_streaming.rs`:
   ```rust
   use codex_core::cli_executor::GeminiPipesProvider;

   pub struct GeminiStreamingProvider {
       provider: GeminiPipesProvider,
   }

   impl GeminiStreamingProvider {
       pub fn new(model: &str) -> Self {
           Self { provider: GeminiPipesProvider::new(model) }
       }

       pub async fn send_message(&self, conv_id: String, msg: String)
           -> Result<Receiver<StreamEvent>, CliError>
       {
           self.provider.send_message(conv_id, msg).await
       }
   }
   ```

2. Register in `tui/src/model_router.rs`:
   ```rust
   if GeminiPipesProvider::is_available() {
       router.register("gemini-2.5-flash", Box::new(GeminiStreamingProvider::new));
       router.register("gemini-2.5-pro", Box::new(GeminiStreamingProvider::new));
       router.register("gemini-2.0-flash", Box::new(GeminiStreamingProvider::new));
   } else {
       eprintln!("{}", GeminiPipesProvider::install_instructions());
   }
   ```

3. Test end-to-end:
   ```bash
   ~/code/build-fast.sh run
   # /model gemini-2.5-flash
   # "My name is Alice"
   # "What's my name?"
   # Expect: "Alice" in response
   ```

**Success Criteria**:
- Gemini models selectable via /model command
- Multi-turn conversations work (context maintained)
- No process leaks or errors
- Session IDs captured correctly

**Deliverables**:
- Working Gemini CLI integration in TUI
- SPEC-952 marked "Complete" in SPEC.md
- Documentation updated (CLAUDE.md section on Gemini CLI)

---

## 🎯 Work Stream 2: SPEC-954 Session Management Polish (Priority 2)

**Status**: In Progress, Tasks 2-3 blocked by SPEC-955 ✅ NOW UNBLOCKED
**Effort**: 1.5-2.5 hours remaining
**Location**: docs/SPEC-KIT-954-session-management-polish/spec.md

### Current Progress
- ✅ Task 1: Interleaving tests (complete)
- ✅ Task 4: Documentation (complete)
- ⏸️ Task 2: (blocked, now unblocked)
- ⏸️ Task 3: (blocked, now unblocked)

### Tasks to Complete

**Task 2.1: Review Task 2 Requirements** (30 min)
- Read SPEC-954 spec.md to understand Task 2 scope
- Verify SPEC-955 fixes resolved the blocking issue
- Plan implementation approach

**Task 2.2: Implement Task 2** (varies - check spec)
- Follow spec requirements
- Write tests if applicable
- Validate against acceptance criteria

**Task 2.3: Complete Task 3** (similar process)

**Success Criteria**:
- All 4 tasks marked complete
- Tests passing
- SPEC-954 marked "Done" in SPEC.md
- Documentation updated

---

## 🎯 Work Stream 3: TUI Polish (Priority 3 - Quick Wins)

**Status**: Multi-turn now working ✅, formatting issues remain
**Effort**: 1 hour total (10 min + 30-60 min)
**Location**: docs/NEXT-SESSION-TUI-FIXES.md

### Issue 1: /sessions Output Formatting ⚡ 5-10 MIN

**Problem**: Output displays as single line with escaped newlines
```
" # Active CLI Sessions1 active session(s)```CONV-ID..."
```

**Fix Location**: `tui/src/chatwidget/mod.rs:7883-7886`

**Solution**:
```rust
// Instead of:
vec![ratatui::text::Line::from(output)]

// Use:
output.lines()
    .map(|l| ratatui::text::Line::from(l.to_string()))
    .collect()
```

**Apply to**:
- `list_cli_sessions_impl()`
- `kill_cli_session_impl()`
- `kill_all_cli_sessions_impl()`

**Validation**:
```bash
~/code/build-fast.sh run
# After using Claude/Gemini:
/sessions  # Should display multi-line formatted table
```

### Issue 2: Questions/Responses Not Interleaved (UX) ⏱️ 30-60 MIN

**Problem**: "Questions and responses are separate rather than Q, R, Q, R..."

**Investigation Needed**:
1. Reproduce: Send 3-4 Q&A pairs in TUI
2. Check history_push() call sites
3. Understand intended vs actual display order
4. Fix grouping logic if needed

**Likely Causes**:
- History cell ordering issue
- Message grouping logic
- Conversation history construction

**Success Criteria**:
- Messages display as: User1, Assistant1, User2, Assistant2, etc.
- Natural conversation flow in TUI
- No visual grouping oddities

---

## 🎯 Work Stream 4: SPEC-946 /model Command Expansion (Priority 4)

**Status**: Core complete (2h), TUI testing pending
**Effort**: 1-2 hours
**Impact**: 12x cost savings (Flash $0.30 vs GPT-5 $1.25)

### Current State
- ✅ model_presets.rs expanded from 7→16 presets
- ✅ All 13 models covered (Gemini 3 Pro, 2.5 Pro, Flash, Claude Opus/Sonnet/Haiku, GPT-5.1 variants)
- ✅ Pricing added to descriptions
- ✅ Full workspace compiles

### Tasks

**Task 4.1: TUI Testing** (30-60 min)
```bash
~/code/build-fast.sh run

# Test each model preset:
/model
# Verify all 16 presets appear
# Select each, verify description shows pricing
# Send test message with 2-3 models to verify switching works

# Test categories:
# - Gemini: 3 Pro ($2), 2.5 Pro ($1.25), Flash ($0.30)
# - Claude: Opus ($15), Sonnet ($3), Haiku ($1)
# - GPT-5.1: Default ($1.25), Mini ($0.30), Medium ($3), High ($15), Pro ($75)
```

**Task 4.2: Optional Modal UI Enhancements** (30-60 min)
- Add model family grouping in selector
- Show cost comparison
- Add "recommended" tags

**Success Criteria**:
- All 16 presets functional
- Model switching seamless
- Pricing displayed correctly
- SPEC-946 marked "Done" in SPEC.md

---

## 🎯 Work Stream 5: TUI Testing Infrastructure - Full Improvements (Priority 5)

**Status**: Core complete (41 tests, 35+ passing), 6 improvements identified
**Effort**: 8-12 hours total
**Location**: docs/NEXT-SESSION-TUI-TESTING-HANDOFF.md

### Item 1: Refactor ChatWidget Test Layout (HIGH - 1-2h)

**Problem**: Tests embedded in 22k-line `mod.rs`
**Impact**: Maintainability, merge conflicts

**Actions**:
- Create `tui/src/chatwidget/tests.rs`
- Create `tui/src/chatwidget/test_support.rs`
- Move 14 tests from mod.rs to tests.rs (~370 lines)
- Move helpers to test_support.rs (~100 lines)
- Update mod.rs: `#[cfg(test)] mod tests;`

**Validation**:
```bash
cargo test -p codex-tui test_key --lib
cargo test -p codex-tui prop_orderkey --lib
```

**Success**: mod.rs reduced by ~370 lines, all tests still pass

### Item 2: Strengthen Interleaving Invariants (HIGH - 1h)

**Problem**: `test_three_overlapping_turns` only checks counts, not contiguity
**Impact**: Correctness - might miss subtle ordering bugs

**Actions**:
- Add helper: `history_by_req() -> HashMap<String, Vec<usize>>`
  - Groups history cell indices by request ID
  - Returns map of req_id → cell indices

- Update test assertions:
  ```rust
  let by_req = harness.history_by_req();
  for (req_id, indices) in by_req {
      // Assert indices are contiguous: [5,6,7] not [5,6,9]
      assert_contiguous(&indices);
      // Assert first cell is user, later cells are assistant
      assert_ordering(&harness.widget.history_cells, &indices);
  }
  ```

**Files**: test_harness.rs (add helper + update test)

**Success**: Test catches violations if OrderKey breaks

### Item 3: Enhance Stream-JSON Parsing Tests (MEDIUM - 2h)

**Goal**: Add real Claude CLI output samples and property tests

**Actions**:
1. Capture real samples:
   ```bash
   claude --print --output-format stream-json "test" > tests/samples/claude_stream_sample.jsonl
   gemini --output-format stream-json "test" > tests/samples/gemini_stream_sample.jsonl
   ```

2. Add tests with real samples:
   ```rust
   #[test]
   fn test_parse_real_claude_output() {
       let sample = include_str!("../../../tests/samples/claude_stream_sample.jsonl");
       // Parse and assert structure
   }
   ```

3. Add property tests:
   ```rust
   proptest! {
       #[test]
       fn prop_parse_handles_arbitrary_whitespace(
           events in prop::collection::vec(sample_json_lines(), 1..10)
       ) {
           // Validate no panics, sensible output
       }
   }
   ```

**Files**:
- `core/src/cli_executor/claude_pipes.rs` (+tests)
- `core/src/cli_executor/gemini_pipes.rs` (+tests)
- NEW: `tests/samples/*.jsonl` (real CLI output)

**Success**: Comprehensive edge case coverage, real-world validation

### Item 4: Add Real CLI Integration Test (MEDIUM - 1-2h)

**Goal**: One real stdin/stdout integration test (not PTY)

**Actions**:
1. Add dependencies to Cargo.toml:
   ```toml
   [dev-dependencies]
   assert_cmd = "2"
   predicates = "3"
   ```

2. Create `tests/cli_basic_integration.rs`:
   ```rust
   use assert_cmd::Command;

   #[test]
   #[ignore] // Run with: cargo test -- --ignored
   fn test_cli_multi_turn_conversation() {
       // Test that binary handles multi-turn without hanging
       let mut cmd = Command::cargo_bin("code").unwrap();
       cmd.write_stdin("Hello\n")
          .write_stdin("What did I just say?\n")
          .timeout(std::time::Duration::from_secs(30))
          .assert()
          .success();
   }
   ```

**Success**: Real end-to-end validation, CI-ready test

### Item 5: Tighten Snapshot Tests (LOW - 30 min)

**Goal**: Add structural assertions alongside snapshots

**Actions**: Update each snapshot test:
```rust
// Before snapshot
assert_eq!(harness.history_cell_count(), 4, "Expected 4 cells");
assert!(harness.widget.history_cells[0].kind() == HistoryCellType::User);
assert!(harness.widget.history_cells[2].kind() == HistoryCellType::Assistant);

// Then snapshot
let snapshot = render_widget_to_snapshot(&harness.widget);
insta::assert_snapshot!("name", snapshot);
```

**Files**: test_harness.rs (3 snapshot tests)

**Success**: Snapshots fail if structure breaks, not just rendering

### Item 6: Wire into CI & Coverage (LOW but valuable - 2-3h)

**Goal**: Automated testing + coverage tracking

**Actions**:

1. **GitHub Actions Workflow** (1h):
   Create `.github/workflows/tui-tests.yml`:
   ```yaml
   name: TUI Tests
   on: [push, pull_request]
   jobs:
     test:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v3
         - uses: dtolnay/rust-toolchain@stable
         - name: Run TUI tests
           run: |
             cd codex-rs
             cargo test -p codex-tui --lib
         - name: Run integration tests
           run: |
             cd codex-rs
             cargo test -p codex-tui --lib -- --ignored
   ```

2. **Coverage Tracking** (1-2h):
   ```bash
   # Install tarpaulin (if not present)
   cargo install cargo-tarpaulin

   # Generate coverage
   cd codex-rs
   cargo tarpaulin --lib -p codex-tui --out Html --output-dir coverage

   # Review: open coverage/index.html
   # Target: >60% for critical modules (chatwidget, streaming)
   ```

3. **Coverage Workflow** (optional):
   Create `.github/workflows/coverage.yml` with tarpaulin run

**Success**:
- Tests run on every PR
- Coverage tracked over time
- Regressions caught automatically

---

## 🎯 Work Stream Progress Tracking

### Priority Order

**Phase 1: Critical Path** (4-6h)
1. Gemini CLI multi-turn debug (30-60 min)
2. Gemini TUI integration (1-2h)
3. SPEC-954 Task 2-3 (1.5-2.5h)

**Phase 2: Quick Wins** (2-3h)
4. /sessions formatting fix (10 min)
5. Message interleaving UX (30-60 min)
6. SPEC-946 TUI testing (1-2h)

**Phase 3: Testing Infrastructure** (8-12h)
7. Refactor test layout (1-2h)
8. Strengthen invariants (1h)
9. Stream-JSON parsing tests (2h)
10. CLI integration test (1-2h)
11. Tighten snapshots (30 min)
12. CI & coverage (2-3h)

**Flexible Stopping Points**:
- After Phase 1: 3 major items complete
- After Phase 2: All completion work done
- After Phase 3: Infrastructure mature and automated

---

## 📁 Essential Files Reference

### Quick Access Map

**Gemini CLI**:
- Implementation: `codex-rs/core/src/cli_executor/gemini_pipes.rs`
- Tests: Same file, lines 585-650
- TUI Provider: `codex-rs/tui/src/providers/gemini_streaming.rs`
- Router: `codex-rs/tui/src/model_router.rs`

**SPEC-954**:
- Spec: `docs/SPEC-KIT-954-session-management-polish/spec.md`
- PRD: `docs/SPEC-KIT-954-session-management-polish/PRD.md`

**TUI Polish**:
- /sessions impl: `codex-rs/tui/src/chatwidget/mod.rs:7883+`
- Formatting helper: Same file, `list_cli_sessions_impl()`

**SPEC-946**:
- Presets: `codex-rs/common/src/model_presets.rs`
- TUI Model Selector: `codex-rs/tui/src/chatwidget/mod.rs` (search for "/model")

**Testing Infrastructure**:
- Test Harness: `codex-rs/tui/src/chatwidget/test_harness.rs`
- Main tests: `codex-rs/tui/src/chatwidget/mod.rs:18363+`
- Integration: `codex-rs/tests/`

---

## 🚀 Session Start Commands

### Load Context
```bash
# Navigate
cd /home/thetu/code

# Query local-memory for context
# Use mcp__local-memory__search with:
# - query: "SPEC-955 test deadlock Gemini CLI session management"
# - limit: 10

# Review this prompt
cat docs/NEXT-SESSION-COMPREHENSIVE.md

# Check current state
git log --oneline -3
git status --short
```

### Validate Baseline
```bash
cd /home/thetu/code/codex-rs

# Verify SPEC-955 tests still pass
cargo test -p codex-tui --lib test_harness::tests
# Expect: 7 passed, 0 failed, 2 ignored

# Verify build works
~/code/build-fast.sh
# Expect: ✅ Build successful

# Check current Gemini test state
cargo test -p codex-core --lib gemini_pipes::tests::test_single_turn_pipes -- --ignored --nocapture
# Expect: PASSES in ~6-7s

cargo test -p codex-core --lib gemini_pipes::tests::test_multi_turn_state -- --ignored --nocapture
# Expect: FAILS with timeout (this is what we'll fix first)
```

### Start Work Stream 1 (Gemini CLI)
```bash
# Begin debugging multi-turn test
cd /home/thetu/code/codex-rs

RUST_LOG=codex_core=trace cargo test -p codex-core --lib \
  cli_executor::gemini_pipes::tests::test_multi_turn_state \
  -- --ignored --nocapture 2>&1 | tee debug.log

# Analyze output for:
# - Process lifecycle events
# - JSON parsing issues
# - EOF detection
# - Timeout location
```

---

## 📊 Success Metrics by Work Stream

### Gemini CLI
- [ ] Multi-turn test passing (<10s)
- [ ] Gemini models in TUI /model selector
- [ ] Multi-turn conversations working in TUI
- [ ] No process leaks
- [ ] SPEC-952 marked "Complete" in SPEC.md

### SPEC-954
- [ ] All 4 tasks complete
- [ ] Tests passing
- [ ] Documentation updated
- [ ] SPEC-954 marked "Done" in SPEC.md

### TUI Polish
- [ ] /sessions formatted correctly
- [ ] Message interleaving displays naturally
- [ ] User testing validates improvements

### SPEC-946
- [ ] All 16 model presets tested in TUI
- [ ] Model switching seamless
- [ ] Pricing displayed accurately
- [ ] SPEC-946 marked "Done" in SPEC.md

### Testing Infrastructure
- [ ] Tests extracted from mod.rs
- [ ] Invariant tests strengthened
- [ ] Stream-JSON edge cases covered
- [ ] CLI integration test added
- [ ] Snapshots have structural assertions
- [ ] CI workflow running
- [ ] Coverage tracked (target: >60%)

---

## 🧠 Local-Memory Context IDs

**Query at session start**:
```
Search recent SPEC-955 work:
- Memory ID: eb3b2293-5cd1-4629-b578-4a4ba0fe52cc (Bug fixes)
- Memory ID: f7c8eb38-be5c-4c13-82a0-9359e3a71898 (StreamController limitation)

Search Gemini CLI context:
- Query: "Gemini CLI pipes one-shot resume"
- Tags: ["spec:SPEC-KIT-952", "gemini"]

Search testing patterns:
- Query: "TUI testing TestHarness OrderKey"
- Tags: ["testing", "rust"]
```

---

## ⚠️ Known Issues & Constraints

### Pre-existing Test Failures (Not Blocking)
- 22 spec_kit tests failing (pre-existing, unrelated to SPEC-955)
- 30 codex-core tests failing (pre-existing)
- These are tracked separately, don't block this work

### Gemini CLI Requirements
- Gemini CLI version 0.17.0+ installed (`which gemini && gemini --version`)
- Authenticated (`gemini` launches successfully)
- Working `--output-format stream-json` support

### StreamController Limitation (Documented, Deferred)
- Single buffer per StreamKind
- 3+ concurrent streams not supported
- Tests marked #[ignore] with FIXME comments
- Fix estimated: 4-8h (optional future work)

---

## 🎯 Decision Points & Flexibility

### When to Pause and Ask

**After Phase 1** (Gemini + SPEC-954):
- Review progress, decide if continuing to Phase 2
- Commit checkpoint if needed

**After Phase 2** (All completions):
- Major milestone reached - all SPECs done
- Consider committing before starting Phase 3

**During Phase 3** (Testing improvements):
- Each item is independent - can pick and choose
- Can spread across multiple sessions if needed

### Adaptive Priorities

**If Gemini debug takes >2h**:
- Skip to SPEC-954 (quick win)
- Return to Gemini later with fresh perspective

**If hitting test failures**:
- Don't get stuck - document and move forward
- Use #[ignore] with good comments
- Return to debug when fresher

**If time runs out**:
- Commit completed work streams
- Create handoff doc for remaining items
- Update SPEC.md with accurate status

---

## 🏁 Session End Criteria

### Minimum Success (Phase 1)
- Gemini CLI working in TUI
- SPEC-954 complete
- 2 major SPECs closed

### Target Success (Phase 1+2)
- All 4 work streams complete
- TUI polish items done
- 4 SPECs closed, major UX improvements

### Stretch Success (All Phases)
- Testing infrastructure mature
- CI/coverage automated
- All identified improvements complete
- Technical debt significantly reduced

---

## 📝 Commit Strategy

### Per Work Stream
- Commit after each major completion (Gemini, SPEC-954, etc.)
- Use conventional commits: `feat(tui): Complete Gemini CLI integration`
- Include test evidence and validation details

### Or Comprehensive
- One large commit at session end
- Group related changes
- Detailed message covering all work streams

**Recommendation**: Commit after each Phase for checkpointing

---

## 🧭 Navigation Quick Reference

**Start Here**:
1. Read this file (NEXT-SESSION-COMPREHENSIVE.md)
2. Query local-memory for recent context
3. Run validation commands (see "Session Start Commands")
4. Choose starting work stream (recommend: Gemini CLI)

**If Blocked**:
- Check "Known Issues & Constraints"
- Review relevant docs/SPEC files
- Ask clarifying questions
- Skip to next work stream if needed

**Progress Tracking**:
- Use TodoWrite tool for each work stream
- Update SPEC.md as SPECs complete
- Store key findings to local-memory (importance ≥8)

---

## 📚 Documentation Index

**Handoff Docs** (Session continuity):
- NEXT-SESSION-COMPREHENSIVE.md ← THIS FILE
- NEXT-SESSION-START-HERE.md (Gemini CLI)
- NEXT-SESSION-TUI-FIXES.md (Polish items)
- NEXT-SESSION-TUI-TESTING-HANDOFF.md (Testing improvements)

**SPEC Docs** (Requirements):
- docs/SPEC-KIT-952-cli-routing-multi-provider/PRD.md (Gemini CLI)
- docs/SPEC-KIT-954-session-management-polish/spec.md (SPEC-954)
- docs/SPEC-KIT-946-model-command-expansion/PRD.md (Model presets)
- docs/SPEC-KIT-955-tui-test-deadlock/spec.md (Testing - COMPLETE ✅)

**Testing Docs** (Patterns):
- codex-rs/TESTING-QUICK-START.md
- codex-rs/TESTING.md
- codex-rs/TESTING-CRITIQUE.md

**Completion Docs** (Evidence):
- docs/SPEC-955-SESSION-2-COMPLETE.md ⭐ Recent success!
- docs/gemini-pipes-session-complete.md (Gemini status)

---

## 🎯 Recommended Session Flow

### Hour 1-2: Gemini CLI (Critical)
- Debug multi-turn test (should be quick - likely simple fix)
- Integrate into TUI
- Test end-to-end with multiple models
- **Checkpoint**: Commit Gemini CLI completion

### Hour 3-5: Completions (High Value)
- SPEC-954 Tasks 2-3 (now unblocked)
- /sessions formatting (10 min quick win)
- Message interleaving UX fix
- **Checkpoint**: Commit polish + SPEC-954

### Hour 6-7: SPEC-946 (High Impact)
- TUI testing of all 16 model presets
- Validate model switching
- Document cost savings
- **Checkpoint**: Commit SPEC-946 completion

### Hour 8-15: Testing Infrastructure (Quality)
- Work through Items 1-6 systematically
- Each item is independent - can pause between
- Commit after major milestones (Item 2, Item 4, Item 6)

### Hour 16+: Polish & Documentation
- Update SPEC.md for all completed items
- Archive/cleanup handoff docs
- Final comprehensive commit if needed
- Session summary to local-memory

---

## 💡 Pro Tips

### Efficiency Maximizers
- Run independent tests in parallel (use multiple terminal tabs)
- Use `cargo check` before `cargo test` (faster feedback)
- Keep build-fast.sh running with `--watch` during TUI development
- Commit frequently - don't lose work to rare crashes

### Quality Checks
- Run `cargo fmt` before each commit
- Use `cargo clippy` to catch issues early
- Test TUI manually after each major change (build-fast.sh run)
- Keep test suite green - don't accumulate failures

### Debugging Accelerators
- Use `RUST_LOG=trace` liberally for async issues
- Add targeted `println!` for test debugging (remove before commit)
- Use `--nocapture` to see test output
- Check `ps aux | grep <process>` for zombie processes

---

## 🚨 Escalation Triggers

**Stop and reassess if**:
- Any single task exceeds 2x time estimate
- Test failures multiply (>5 new failures)
- Architectural issues emerge (like StreamController)
- Build breaks and can't recover quickly

**When blocked**:
1. Document the blocker clearly
2. Mark item as "Blocked" with reason
3. Move to next work stream
4. Return with fresh eyes later

---

## 📈 Expected Outcomes

**Conservative Estimate** (13.5h):
- Gemini CLI: ✅ Complete
- SPEC-954: ✅ Complete
- TUI Polish: ✅ Complete
- SPEC-946: ✅ Complete
- Testing Items 1-3: ✅ Complete
- Testing Items 4-6: ⏸️ Deferred

**Realistic Estimate** (16-18h):
- All work streams: ✅ Complete
- Testing Items 1-5: ✅ Complete
- Testing Item 6: ⏸️ CI setup started

**Optimistic Estimate** (20h):
- Everything: ✅ Complete
- CI/Coverage: ✅ Fully automated
- Bonus: Additional polish and optimization

---

## 🎁 Bonus Items (If Time Permits)

### Low-Hanging Fruit
- Fix `render_widget_to_snapshot` unused warning (make it used or remove)
- Clean up untracked test files (`codex-rs/core/tests/*.rs`)
- Archive obsolete handoff docs to `docs/archive/`

### Quick Optimizations
- Add keyboard shortcuts for /model selector
- Improve model preset descriptions
- Add model family icons in selector

### Documentation
- Create TESTING-FINAL-REPORT.md synthesizing all testing work
- Update CLAUDE.md with Gemini CLI notes
- Create SPEC-952-COMPLETE.md comprehensive summary

---

## 🔄 Relationship to Other Work

**Unblocks**:
- SPEC-954 completion unblocks session management improvements
- Gemini CLI completion unblocks SPEC-952 closure
- Testing infrastructure enables future TUI development

**Blocked By**:
- Nothing! SPEC-955 was the last blocker
- All work streams are ready to execute

**Enables**:
- Future model provider additions (pattern established)
- Confident TUI refactoring (tests protect against regressions)
- Faster development cycles (CI catches issues early)

---

**Created**: 2025-11-23
**Author**: Claude (SPEC-955 Session 2)
**Status**: Ready for execution
**Estimated Value**: 4 SPEC completions + mature testing infrastructure

**🚀 Ready to start? Load local-memory context and begin with Gemini CLI debugging!**
