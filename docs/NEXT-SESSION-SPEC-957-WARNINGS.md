# SPEC-957 Phase 2: Zero Warnings Sprint

**Created**: 2025-11-25
**Priority**: P2 - Code Quality
**Estimated Duration**: 1.5-2 hours
**Branch**: main
**Previous Commit**: 8aa786f89 (fix(spec-kit): Resolve test failures from model/registry updates)
**Target**: Zero compiler warnings in codex-tui

---

## Session Context

### Completed Phase 1 (This Session)
- **19 failing tests fixed** → All passing
- **Root causes identified and resolved**:
  - Registry count mismatch (23→27) caused mutex poison cascade
  - Model pricing outdated (Haiku 0.25→1.0, Gemini 0.10→0.30)
  - Agent configs changed (implement: 4-agent→2-agent)
  - JSON extractor method changed (MarkdownFence→DepthTracking)
- **Commit**: `8aa786f89`

### Current State
- **208 compiler warnings** in codex-tui
- All tests passing
- Binary functional

---

## Phase 2 Objectives: Zero Warnings

### Step 1: Create Formal SPEC Entry (5 min)

Update `SPEC.md` with SPEC-957:
```markdown
| SPEC-957 | Warning Cleanup | In Progress | P2 | - | - | Phase 1 complete (test fixes), Phase 2: zero warnings |
```

### Step 2: Warning Inventory (10 min)

Run diagnostic to get exact counts:
```bash
cd ~/code/codex-rs && cargo build -p codex-tui 2>&1 | grep "warning:" | wc -l
```

**Known Warning Categories (from Phase 1 analysis):**

| Category | Count | Fix Strategy |
|----------|-------|--------------|
| Unused variables | ~35 | Prefix with `_` or remove |
| Unused imports | ~20 | Remove import statements |
| Deprecated rand methods | ~5 | `thread_rng`→`rng`, `gen_range`→`random_range` |
| Deprecated ratatui | ~1 | `frame.size()`→`frame.area()` (DONE) |
| Unreachable patterns | ~5 | Remove or add `#[allow]` |
| Dead code | ~15 | Remove or `#[allow(dead_code)]` |
| Private interface | ~10 | Fix visibility or `#[allow]` |
| Value never read | ~10 | Remove assignment or use value |

### Step 3: High-Impact Files (Priority Order)

| File | Warnings | Primary Issues |
|------|----------|----------------|
| `ace_route_selector.rs` | 16 | Unused variables, unreachable patterns |
| `consensus.rs` | 13 | Unused variables, dead code |
| `consensus_db.rs` | 11 | Unused imports, variables |
| `evidence_cleanup.rs` | 10 | Unused variables |
| `model_router.rs` | 9 | Unused variables |
| `native_consensus_executor.rs` | 9 | Unused variables |
| `file_modifier.rs` | 6 | Unused variables |
| `chatwidget/mod.rs` | 6 | Unused imports |
| `provider_login.rs` | 5 | Unused variables |
| `spawn_metrics.rs` | 5 | Unused variables |

### Step 4: Systematic Fix Process (1-1.5 hours)

For each file, in priority order:

1. **Read file** to understand context
2. **Fix unused imports** - Remove or consolidate
3. **Fix unused variables** - Prefix with `_` if needed for API, remove if truly unused
4. **Fix deprecated methods** - Update to new API
5. **Fix unreachable patterns** - Remove or document why allowed
6. **Verify build** - `cargo build -p codex-tui`
7. **Run tests** - `cargo test -p codex-tui --lib <module_name>`

**Rules:**
- Prefer removing dead code over `#[allow(dead_code)]`
- Document any intentional `#[allow(...)]` with comment
- Don't remove code that's part of planned features (check TODOs)

### Step 5: Deprecated Method Updates

**Rand crate (if upgrading to 0.9+):**
```rust
// Old
use rand::thread_rng;
let x = thread_rng().gen_range(0..10);

// New
use rand::rng;
let x = rng().random_range(0..10);
```

**Ratatui:**
```rust
// Old (ALREADY FIXED)
frame.size()

// New
frame.area()
```

### Step 6: Validation (15 min)

```bash
# Zero warnings check
cd ~/code/codex-rs && cargo build -p codex-tui 2>&1 | grep -c "warning:"
# Expected: 0

# Clippy clean
cd ~/code/codex-rs && cargo clippy -p codex-tui -- -D warnings
# Expected: No errors

# Test suite (skip slow snapshots)
cd ~/code/codex-rs && cargo test -p codex-tui --lib -- --skip snapshot
# Expected: All pass

# Build verification
~/code/build-fast.sh
# Expected: Success
```

### Step 7: Commit (5 min)

```bash
git add -A
git commit -m "fix(spec-kit): Achieve zero warnings in codex-tui

- Remove unused imports across 15+ files
- Prefix/remove unused variables
- Update deprecated method calls
- Remove dead code paths
- Add targeted #[allow] where justified (with comments)

Reduces warnings from 208 to 0.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Acceptance Criteria

### Must Complete
- [ ] Zero compiler warnings (`cargo build` output clean)
- [ ] Clippy clean (`-D warnings` passes)
- [ ] All tests passing
- [ ] Binary builds and runs
- [ ] SPEC.md updated with SPEC-957 status

### Quality Gates
- [ ] No `#[allow(...)]` without justification comment
- [ ] No removal of code marked with TODO/FIXME for future use
- [ ] Test coverage unchanged (no test removals)

---

## Deferred Items (Not This Session)

1. **SPEC-956**: Message interleaving bug (separate SPEC)
2. **Slow snapshot tests**: 2 tests >60s (optimization task)
3. **Warning cleanup in other crates**: codex-core, codex-linux-sandbox

---

## Reference Commands

```bash
# Count warnings by category
cargo build -p codex-tui 2>&1 | grep "warning:" | sed 's/warning: //' | cut -d':' -f1 | sort | uniq -c | sort -rn

# Find warnings in specific file
cargo build -p codex-tui 2>&1 | grep "ace_route_selector"

# Quick test specific module
cargo test -p codex-tui --lib ace_route_selector -- --nocapture

# Cargo fix (safe mode)
cargo fix --lib -p codex-tui --allow-dirty
```

---

## Session Start Command

```bash
# Load this prompt in Claude Code
cd ~/code && cat docs/NEXT-SESSION-SPEC-957-WARNINGS.md
```

**Mode**: ultrathink
