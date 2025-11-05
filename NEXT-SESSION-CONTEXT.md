# Next Session Context - SPEC-KIT-900 Validation

**Last Session**: Nov 1, 2025 (9 hours)
**Branch**: `debugging-session` (23 commits from feature/spec-kit-069-complete)
**Binary**: `/home/thetu/code/codex-rs/target/dev-fast/code` (built Nov 1 19:10 UTC)
**Status**: Quality gate working, pipeline executing, 4 issues remain

---

## Quick Start

```bash
cd /home/thetu/code
git checkout debugging-session
git log --oneline -10  # Review recent commits
```

**Read first**:
- `SESSION-HANDOFF-2025-11-01.md` - Complete session recap
- `SPEC-KIT-900-VALIDATION-ISSUES.md` - Detailed issue analysis

---

## What Works Now ✅

1. **Quality Gate Artifact Collection**
   - Direct filesystem broker (reads `.code/agents/*/result.txt`)
   - Handles both markdown fences and raw JSON
   - No local-memory dependency
   - Commit: `c51be0c50`

2. **Stack Overflow Fix**
   - Processing flag set before history_push (prevents recursion)
   - Binary launches and runs without crash
   - Commit: `4c537c7e0`

3. **SPEC-KIT-900 P0 Blockers Resolved**
   - Tech stack, consensus definition, cost schema, etc.
   - Commit: `0160502e6`

---

## What Needs Fixing (Priority Order)

### P0: Shell Script Guardrails (15 min)

**Problem**: Still calling bash scripts despite SPEC-KIT-066 claiming native implementation.

**Evidence**:
```
scripts/env_run.sh scripts/spec_ops_004/commands/spec_ops_plan.sh SPEC-KIT-900
Ran for 23.03s  ← 23 SECONDS PER STAGE!
```

**Root Cause**: `state.rs:792` guardrail_for_stage() returns `SlashCommand::SpecOpsPlan` which maps to bash script.

**Fix Location**: `codex-rs/tui/src/chatwidget/spec_kit/state.rs` line 792-805

**Fix**:
```rust
pub fn guardrail_for_stage(stage: SpecStage) -> SlashCommand {
    // Guardrails are redundant with quality gates (SPEC-KIT-068)
    // Return Noop or skip guardrail phase entirely
    match stage {
        _ => SlashCommand::Noop,
    }
}
```

**OR** remove guardrail phase from pipeline entirely if quality gates sufficient.

**Impact**: 23s × 6 stages = 2+ minutes saved per run

---

### P1: JSON Parsing Failures (30 min)

**Problem**: Python inline scripts fail to extract JSON from agent results.

**Evidence**:
```python
TypeError: unsupported operand type(s) for +: 'NoneType' and 'int'
```

**Location**: TUI code that processes agent results (likely in chatwidget/)

**Fix Options**:
1. **Port Rust function** (recommended): Use `extract_json_from_markdown()` from quality_gate_handler.rs
2. **Fix Python**: Add None checks, handle both formats
3. **Use jq**: Shell out to jq command-line tool

**Effort**: 30 minutes for Option 1

**Impact**: More reliable extraction, no retry delays

---

### P1: 16-Agent Orchestration Issue (1-2h)

**Problem**: Checklist spawns 16 agents instead of 3, result collection fails.

**Evidence**:
```
models: [16 items]
batch_id: b5b896f8...
Agent Wait: 4m 09s

task="gather agent results" → Timeout → Cancelled
task="get results" → Cancelled
```

**Root Cause**: Orchestrator prompt or config causing wrong model selection.

**Investigation Needed**:
```bash
# Check config
cat config.toml | grep -A 50 "agents\|models"

# Check checklist orchestrator prompt
rg "speckit.checklist.*prompt|quality.*checklist.*prompt" codex-rs/
```

**Effort**: 1-2 hours to debug and fix

**Impact**: 4+ min delays, wasted agent spawns, cost waste

---

### P2: Re-enable Execution Logging (1-2h)

**Status**: Disabled in commit 5b9a6fa0d

**Why Disabled**: Suspected recursion causing stack overflow (but recursion was from quality_gate_handler, not logger).

**Re-enable Strategy**:
1. Uncomment logger.init() in state.rs:515-519
2. Test if it crashes
3. If crashes, stub out update_status_from_event() temporarily
4. Keep JSONL logging, defer status file

**Effort**: 1-2 hours with careful testing

**Priority**: Lower than shell scripts and JSON parsing, but needed for SPEC-KIT-070 validation

---

## Immediate Action Plan (30-60 min)

### Step 1: Fix Shell Scripts (15 min)

```rust
// state.rs:792
pub fn guardrail_for_stage(stage: SpecStage) -> SlashCommand {
    // Quality gates provide validation; guardrails redundant
    // Return no-op to skip 23s bash script penalty
    match stage {
        _ => SlashCommand::Noop,
    }
}
```

**OR** check if SlashCommand::Noop exists, otherwise skip guardrail phase in pipeline_coordinator.rs.

### Step 2: Build and Test (5 min)

```bash
./build-fast.sh
code
/speckit.auto SPEC-KIT-900
```

**Watch for**: No shell script execution, faster guardrail phase.

### Step 3: Let Run Complete (45 min)

Get baseline cost/evidence data for SPEC-KIT-070 validation.

**Total**: ~1 hour for quick win.

---

## Session Achievements (Don't Forget)

✅ Quality gate artifact storage working (7 fixes)
✅ Stack overflow recursion resolved
✅ Execution logging system implemented (needs re-enable)
✅ SPEC-KIT-900 specification completed (P0 blockers)
✅ Direct filesystem broker (eliminates local-memory dependency)
✅ Binary stable and functional

**Commits**: 23 on debugging-session
**LOC**: ~2,500 added
**Documentation**: 2 comprehensive handoff docs
**Local-memory**: 5 high-value memories stored

---

## Branch Status

**Current**: `debugging-session` (23 commits)
**Parent**: `feature/spec-kit-069-complete` (diverged)
**Merge Strategy**:
- Option A: Merge debugging-session → feature/spec-kit-069-complete
- Option B: Keep separate until all issues fixed
- Option C: Cherry-pick good commits (recursion fix, SPEC fixes) to feature branch

**Recommendation**: Fix P0 shell scripts, then merge to feature branch.

---

## Files to Review Next Session

**Code**:
- `state.rs:792` - guardrail_for_stage() ← FIX HERE
- `slash_command.rs:372-382` - Shell script mappings
- `quality_gate_handler.rs` - Extract JSON function (reference for JSON fix)
- `execution_logger.rs` - Re-enable this

**Docs**:
- `SESSION-HANDOFF-2025-11-01.md` - Full recap
- `SPEC-KIT-900-VALIDATION-ISSUES.md` - Issue details
- `docs/SPEC-KIT-900-generic-smoke/PRD.md` - P0 fixes applied

**Evidence**:
- `.code/agents/` - Should be clean (cleared old agents)
- `docs/SPEC-OPS-004-.../evidence/commands/SPEC-KIT-900/` - Partial evidence from today

---

## Success Criteria (Before Session End)

**Minimum** (today):
- ✅ Quality gate working
- ✅ Stack overflow fixed
- ✅ Documentation complete
- ✅ Issues catalogued

**Ideal** (next session):
- ⏳ Shell scripts removed (native guardrails)
- ⏳ JSON parsing robust
- ⏳ Agent orchestration clean (3 agents, not 16)
- ⏳ Execution logging enabled
- ⏳ One clean SPEC-KIT-900 validation run
- ⏳ SPEC-KIT-070 cost validation complete

---

## Context for Claude Next Session

**Goal**: Complete SPEC-KIT-070 validation (75% cost reduction claim).

**Method**: Run `/speckit.auto SPEC-KIT-900` successfully and analyze cost/tier/agent data.

**Blockers Fixed**:
- Quality gate artifact storage ✓
- Stack overflow recursion ✓
- SPEC-KIT-900 P0 gaps ✓

**Blockers Remaining**:
- Shell script guardrails (P0)
- JSON parsing fragility (P1)
- 16-agent orchestration (P1)
- Execution logging disabled (P2)

**Quick Win Available**: Fix shell scripts (15 min) + one clean run (45 min) = baseline validation data.

**After That**: Remaining backlog - SPEC-KIT-901 (4h), SPEC-KIT-910 (1-2d), SPEC-KIT-902 (1w).
