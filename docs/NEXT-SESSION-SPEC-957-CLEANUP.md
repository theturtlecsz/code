# SPEC-957: Test & Warning Cleanup Sprint

**Created**: 2025-11-25
**Priority**: P1 - Technical Debt Reduction
**Estimated Duration**: 3-4 hours
**Branch**: main
**Last Commit**: cd2b1f96a (feat(multi-provider): Update model presets and complete SPEC-947 Phase 2)

---

## Session Context

### Completed Previous Session (SPEC-947 Phase 2)
- **Opus 4.5 update**: claude-opus-4.1 → claude-opus-4.5 (10 files)
- **Gemini enabled**: Updated CLAUDE.md, all 3 providers working
- **CI fixed**: Disabled upstream-merge schedule (requires API keys)
- **SPEC-956 created**: Message interleaving bug (deferred)
- **Model validation**: All 13 presets tested and working

### Current State
- **18 failing tests** in spec_kit module
- **239 compiler warnings** across codex-tui
- **SPEC-956** backlogged (message interleaving)
- Binary functional (398MB, all providers working)

---

## Session Objectives

### Phase 1: Test Failure Analysis (30 min)

Categorize the 18 failing tests:

| Category | Count | Tests |
|----------|-------|-------|
| command_registry | 9 | Registration, aliases, counts, quality commands |
| cost_tracker | 3 | Pricing calculations for models |
| json_extractor | 2 | Markdown fence extraction |
| pipeline_config | 1 | Precedence testing |
| routing | 1 | Registry find |
| subagent_defaults | 2 | Agent config validation |

**Investigation Steps:**
```bash
# Run specific failing test with output
cd ~/code/codex-rs && cargo test -p codex-tui --lib command_registry -- --nocapture 2>&1 | head -100

# Check cost_tracker test expectations
cd ~/code/codex-rs && cargo test -p codex-tui --lib cost_tracker -- --nocapture
```

**Likely Root Causes:**
1. Model preset changes (opus-4.1 → 4.5) broke cost calculations
2. Command registry counts out of sync with actual commands
3. Subagent defaults need updating for new model names

---

### Phase 2: Fix Test Failures (1.5-2 hours)

**Priority Order:**
1. **cost_tracker** (3 tests) - Update pricing expectations for opus-4.5
2. **command_registry** (9 tests) - Sync test assertions with actual registry
3. **subagent_defaults** (2 tests) - Update agent config expectations
4. **routing** (1 test) - Likely depends on registry fixes
5. **json_extractor** (2 tests) - May be unrelated to model changes
6. **pipeline_config** (1 test) - Check precedence logic

**Acceptance Criteria:**
- [ ] All 18 failing tests pass
- [ ] No new test failures introduced
- [ ] Total: 228+ tests passing (210 + 18 fixed)

---

### Phase 3: Warning Cleanup (1-1.5 hours)

**Current State:** 239 warnings

**Categories (from build output):**
1. **Unused imports** (~30) - `cargo fix --lib -p codex-tui`
2. **Unused variables** (~20) - Prefix with `_` or remove
3. **Dead code** (~10) - Remove or `#[allow(dead_code)]`
4. **Deprecated methods** (~5) - Update to new APIs
5. **Private interface warnings** (~15) - Visibility fixes
6. **Unreachable patterns** (~5) - Clean up match arms

**Automated Fix:**
```bash
cd ~/code/codex-rs && cargo fix --lib -p codex-tui --allow-dirty
```

**Manual Review Required:**
- Deprecated ratatui methods (`.size()` → `.area()`)
- Deprecated rand methods (`thread_rng` → `rng`)
- Private interface visibility issues

**Acceptance Criteria:**
- [ ] Warnings reduced to <50 (80% reduction)
- [ ] No clippy errors
- [ ] Build still succeeds

---

### Phase 4: Validation & Commit (30 min)

```bash
# Full test suite
cd ~/code/codex-rs && cargo test -p codex-tui --lib -q

# Clippy check
cd ~/code/codex-rs && cargo clippy -p codex-tui -- -D warnings

# Build verification
~/code/build-fast.sh

# Final commit
git add -A && git commit -m "fix(spec-kit): Resolve test failures and reduce warnings"
```

---

## Success Criteria Summary

### Must Complete
- [ ] All spec_kit tests passing (currently 18 failing)
- [ ] Warnings < 50 (currently 239)
- [ ] Binary builds and runs
- [ ] Changes committed and pushed

### Should Complete
- [ ] Clippy clean (no warnings)
- [ ] Document any intentional `#[allow(...)]` additions
- [ ] Update SPEC.md if creating formal SPEC-957

### Verify Before Closing
- [ ] `cargo test -p codex-tui --lib` - all pass
- [ ] `cargo clippy -p codex-tui` - clean
- [ ] TUI binary works (`~/code/codex-rs/target/dev-fast/code`)
- [ ] Git status clean

---

## Deferred Items (Not This Session)

1. **SPEC-956**: Message interleaving bug (responses above questions)
2. **Slow snapshot tests**: 2 tests run >60s (separate optimization)
3. **Integration tests**: Focus on unit tests this session

---

## Reference Files

**Test Files:**
- `tui/src/chatwidget/spec_kit/command_registry.rs` - Registry tests
- `tui/src/chatwidget/spec_kit/cost_tracker.rs` - Pricing tests
- `tui/src/chatwidget/spec_kit/subagent_defaults.rs` - Agent config tests
- `tui/src/chatwidget/spec_kit/json_extractor.rs` - JSON extraction tests
- `tui/src/chatwidget/spec_kit/routing.rs` - Routing tests
- `tui/src/chatwidget/spec_kit/pipeline_config.rs` - Config tests

**Warning Sources:**
- `tui/src/chatwidget/spec_kit/*.rs` - Main warning concentration
- `tui/src/app.rs` - Some unused variables
- `tui/src/providers/*.rs` - Deprecated methods

---

## Session Start Command

```bash
# Load this prompt in Claude Code
cd ~/code && cat docs/NEXT-SESSION-SPEC-957-CLEANUP.md
```

**Mode**: ultrathink
