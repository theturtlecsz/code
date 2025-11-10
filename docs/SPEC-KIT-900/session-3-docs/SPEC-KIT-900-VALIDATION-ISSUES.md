# SPEC-KIT-900 Validation Issues - Nov 1, 2025

**Session**: Quality gate debugging + execution logging implementation
**Branch**: debugging-session (22 commits)
**Status**: Pipeline executing but with 5 identified issues

---

## Issue Summary

| ID | Issue | Severity | Impact | Effort |
|----|-------|----------|--------|--------|
| **I-001** | Shell script guardrails still executing | P0 | Performance penalty, SPEC-KIT-066 incomplete | 15 min |
| **I-002** | JSON parsing failures in TUI | P1 | Unreliable extraction, retry delays | 30 min |
| **I-003** | Agent orchestration spawning 16 agents | P1 | Delays, wasted spawns, confusion | 1-2h |
| **I-004** | Execution logging disabled | P2 | No visibility, can't validate SPEC-KIT-070 | 1-2h |
| **I-005** | Quality gate recursion (FIXED) | P0 | Stack overflow | DONE ✓ |

---

## I-001: Shell Script Guardrails Still Executing

### Observed

```
[spec-auto] Follow-up checklist for SPEC-KIT-900 (Plan)

❯ 'scripts/env_run.sh scripts/spec_ops_004/commands/spec_ops_plan.sh SPEC-KIT-900'
  Plan guardrail executed for SPEC-KIT-900
  Telemetry: /home/thetu/code/docs/SPEC-OPS-004-.../spec-plan_2025-11-01T19:48:42Z.json

  Ran for 23.03s
```

### Expected

SPEC-KIT-066 (commit 16cbbfeab, 2025-10-24) claimed:
> "COMPLETE: Eliminated bash orchestration duplicate. /guardrail.auto now redirects to native /speckit.auto."

Guardrails should be native Rust, not bash scripts.

### Root Cause

SPEC-KIT-066 was "config-only" - updated orchestrator instructions but **did not remove bash script calls from Rust code**.

### Evidence

- Script path: `scripts/spec_ops_004/commands/spec_ops_plan.sh`
- Execution time: 23.03s (vs <1s for native commands)
- Called via: `scripts/env_run.sh` wrapper

### Location to Fix

```bash
# Find where bash scripts are called
rg "spec_ops_plan.sh|spec_ops_004/commands" codex-rs/tui/src/chatwidget/spec_kit/
rg "guardrail_for_stage|SlashCommand::SpecOps" codex-rs/tui/src/chatwidget/spec_kit/state.rs
```

**Likely**: `state.rs` has `guardrail_for_stage()` function that returns bash script commands instead of native commands.

### Fix

Replace bash script calls with native guardrail logic or native no-op if guardrails already implemented elsewhere.

### Impact

- **Performance**: 23s penalty per stage (6 stages = 2+ minutes wasted)
- **SPEC-KIT-066 validation**: Incomplete, claimed done but not implemented
- **SPEC-KIT-902 dependency**: Blocks native guardrail work (can't nativize what's already "native")

---

## I-002: JSON Parsing Failures in TUI

### Observed

```
Extracting JSON for local storage

❯ python - '<<PY'
  text = Path(.code/agents/.../result.txt).read_text()
  start = text.find('{\n  "stage": "quality-gate-clarify"')
  # Brace counting logic
  stack = 0
  if text[i] == '{': stack += 1
  elif text[i] == '}': stack -= 1
  # ERROR: TypeError: unsupported operand type(s) for +: 'NoneType' and 'int'
```

### Root Cause

TUI uses **Python inline scripts** for JSON extraction from agent result files:
1. Searches for specific JSON opening pattern
2. Counts braces to find closing brace
3. Fails when:
   - Pattern not found (returns None → crash)
   - Nested JSON in thinking blocks
   - Multiple JSON objects
   - Format variations (markdown fence vs raw)

### Current Behavior

TUI retries with "Adjusting JSON extraction logic" message, but:
- Adds delays (multiple retry attempts)
- May fail entirely on complex outputs
- Fragile (breaks on format changes)

### Fix Options

**Option A**: Use Rust extraction (recommended)
- Port `extract_json_from_markdown()` from quality_gate_handler.rs
- Handles both markdown fences and raw JSON
- Already tested and working
- 30 min effort

**Option B**: Fix Python script
- Add None checks
- Handle both formats
- More robust brace counting
- 20 min effort but still fragile

**Option C**: Use jq command-line tool
- Simpler, more reliable
- Requires jq installed
- 15 min effort

### Impact

- Reliability: Occasional failures extracting agent outputs
- Performance: Retry delays add 10-30s per failure
- Maintenance: Fragile string parsing prone to breakage

---

## I-003: Agent Orchestration Spawning 16 Agents

### Observed

```
✔ Agent Run: task="Execute /speckit.checklist for SPEC-KIT-900"
  models: [16 items]  ← WHY 16?
  batch_id: "b5b896f8-0975-44f3-9a7d-eaca2c310960"

★ Launching Gemini, Claude, and GPT Pro...

✔ Agent Wait: 4m 09s

# Then struggles:
✔ Agent Run: task="gather agent results"
  Noticing unintended agent start
✖ Timeout after 10s
✔ Cancelled

✔ Agent Run: task="get results"
  Correcting function call to agent_result
✔ Cancelled
```

### Analysis

**Expected**: 3 agents (gemini, claude, gpt_pro) for checklist quality gate

**Actual**: 16 agents spawned in batch

**Possible Causes**:

1. **Batch API misuse**: Orchestrator spawning all available models instead of selected 3
2. **Config issue**: Model list includes 16 variants, all being spawned
3. **Batch vs individual**: Code expects 3 individual agent_run calls, got 1 batch with 16

**Result Collection Failures**:
- Orchestrator doesn't know how to get batch results
- Tries tool calls: "gather agent results", "get results"
- Neither tool exists or works correctly
- Timeouts and cancellations

### Root Cause Hypothesis

**Orchestrator prompt** tells it to spawn agents, but:
- Doesn't specify HOW MANY or WHICH models
- Says "models: [...]" generically
- Orchestrator interprets as "spawn all available models"
- Then can't collect results properly

**Config file** might list 16 model variants for different use cases.

### Evidence Needed

```bash
# Check model config
cat /home/thetu/code/config.toml | grep -A 50 "\[agents\]\|\[models\]"

# Check orchestrator prompt for checklist
rg "speckit.checklist.*prompt\|quality.*checklist" codex-rs/
```

### Fix

**Orchestrator prompt** needs to explicitly state:
```
Spawn EXACTLY 3 agents:
1. models: ["gemini-25-flash"] for gemini
2. models: ["claude-haiku-45"] for claude
3. models: ["code"] for code

Do NOT spawn other models. Use individual agent_run calls, not batch.
```

**Or**: Fix batch result collection if batching is intended.

### Impact

- Time: 4+ minutes for checklist (should be 30-60s)
- Cost: 16 agents × cost (significant waste)
- Reliability: Result collection fails, requires retries/cancellations
- Confusion: Unclear which agents actually matter

---

## I-004: Execution Logging Disabled

### Status

Disabled in commit 5b9a6fa0d:
```rust
// DISABLED: Execution logger causes stack overflow (investigating)
```

### Impact on SPEC-KIT-070 Validation

**Cannot validate**:
- ✗ Stage-by-stage cost breakdown
- ✗ Tier assignments (Tier 0-4 routing)
- ✗ Agent model usage per stage
- ✗ Duration per stage
- ✗ Quality gate timing and resolution stats

**Missing files**:
- ✗ `.code/execution_logs/*.jsonl` (structured timeline)
- ✗ `.code/spec_auto_status.json` (real-time monitoring)
- ✗ Post-run summary (human-readable report)

### Why Disabled

**Initial hypothesis**: logger.init() or update_status_from_event() causing recursion.

**But**: Logger only initialized during SpecAutoState creation (when `/speckit.auto` runs), not on every render.

**More likely**: Logger's log_event() calls being triggered during render loop somehow.

### Re-enable Strategy

**Step 1**: Enable logging but stub out update_status_from_event():
```rust
fn update_status_from_event(&self, event: &ExecutionEvent) {
    // TODO: Implement without recursion
    // Just log to JSONL file for now, skip status file
}
```

**Step 2**: Test if JSONL-only logging works without crash

**Step 3**: Gradually re-enable status file updates

**Effort**: 1-2 hours with careful testing

---

## I-005: Quality Gate Recursion (FIXED ✓)

### Issue

Stack overflow from infinite loop:
```
on_quality_gate_agents_complete()
  → history_push()
    → mod.rs:4167 (checks quality_gate_processing == None)
      → on_quality_gate_agents_complete() AGAIN
        → INFINITE RECURSION
```

### Fix Applied

Commit 4c537c7e0: Set `quality_gate_processing` flag **before** any `history_push()` calls.

```rust
// Line 66: Set flag FIRST
state.quality_gate_processing = Some(checkpoint);

// Line 72: THEN safe to call history_push
widget.history_push(...);
```

### Validation

✅ Binary runs without crash
✅ Quality gate executes
✅ Agents complete
✅ No stack overflow

**Status**: RESOLVED

---

## Session Summary

**Duration**: ~9 hours
**Branch**: debugging-session
**Commits**: 22 total

**Major Work**:
1. Quality gate artifact storage (7 iterative fixes)
2. Execution logging system (implemented but disabled)
3. SPEC-KIT-900 P0 blockers resolved
4. Stack overflow recursion fixed

**Current State**:
- ✅ Quality gate not crashing
- ✅ Pipeline executing
- ⚠ Shell scripts still used (not native)
- ⚠ JSON parsing fragile
- ⚠ Agent orchestration issues (16 instead of 3)
- ❌ Execution logging disabled

**Next Session Focus**:
1. Fix shell script guardrails (P0)
2. Fix JSON parsing (P1)
3. Debug agent orchestration (P1)
4. Re-enable execution logging (P2)
5. Complete SPEC-KIT-070 validation

---

## Recommendations

**Short-term** (today if time):
- Fix shell script issue (15 min)
- Let one full run complete to get baseline cost data

**Medium-term** (next session):
- Fix JSON parsing (30 min)
- Debug 16-agent spawn issue (1-2h)
- Re-enable logging carefully (1-2h)

**Long-term**:
- Full SPEC-KIT-070 validation with clean execution logs
- SPEC-KIT-901 (MCP docs)
- SPEC-KIT-910 (Consensus DB)
- SPEC-KIT-902 (Native guardrails - now unblocked if we fix shell scripts)
