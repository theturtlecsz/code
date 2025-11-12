# SPEC-KIT-927 Session 2 Summary - 2025-11-12

## Status: JSON Extraction Fixed, Code Agent Issue Discovered

---

## Completed Work ✅

### 1. Industrial JSON Extraction Module (Commit: 955dcaa69)

**Created**: `codex-rs/tui/src/chatwidget/spec_kit/json_extractor.rs` (721 LOC)

**Implementation**:
- 4-strategy cascade (DirectParse → MarkdownFence → DepthTracking → SchemaMarker)
- Confidence scoring (0.80-0.95 per strategy)
- Schema template detection (rejects TypeScript annotations)
- Separated extraction from validation

**Integration**:
- Replaced 3 scattered extraction functions (460 LOC removed)
- Updated quality_gate_broker.rs, quality_gate_handler.rs, agent_orchestrator.rs
- Unified extraction logic across quality gates + regular stages

**Test coverage**: 10/10 tests passing

**Expected impact**: Agent success rate 67% → 95%+

---

### 2. Extraction Failure Diagnostics (Commit: ef07dfda0)

**Added to consensus_db.rs**:
- `extraction_error` column in agent_executions table
- `record_extraction_failure()` function (stores raw output + error)
- `query_extraction_failures()` for debugging

**Integration**:
- quality_gate_broker.rs calls record_extraction_failure() on errors
- Stores raw agent output even when JSON parsing fails

**Benefit**: Eliminates blind spot where extraction failures lost all diagnostic data

---

### 3. Claude Prompt Fixes (Commit: f119e7300)

**Updated**: 3 Claude quality gate prompts in `docs/spec-kit/prompts.json`

**Changes**:
- Added CRITICAL instruction headers
- Added concrete JSON examples (AUTH-METHOD, REQ-1, TERM-MISMATCH-1)
- Explicit "Do NOT return template" warnings

**Pattern**: Show example BEFORE schema, forbid type annotation placeholders

**Expected**: Claude success rate 50% → 85%+

**Validation**: One run showed "Schema template detected" error - prompt fix addresses this

---

### 4. Orchestrator Config Updates

**File**: `~/.code/config.toml` (not in git)

**Changes**:
- Added CRITICAL "Do NOT run quality gates" to 7 orchestrators:
  - speckit.specify, speckit.plan, speckit.tasks
  - speckit.implement, speckit.validate, speckit.audit, speckit.unlock
- Added explicit binary path to gpt_low agent config

**Fixed**: 18-agent spawn issue (orchestrators calling agent_run for quality gates)

---

## Discovered Issues ❌

### Critical: Code Agent Never Completes

**Symptom**:
- Spawns successfully (recorded in SQLite)
- Never reaches completed_at
- No output captured
- No error message
- Process not found (terminates silently)

**Attempted fixes** (all failed):
1. Added explicit binary path to gpt_low config
2. Verified args-read-only correct
3. Checked sandbox settings
4. Manual execution test (works perfectly)

**Manual vs Orchestrated**:
```
Manual execution:
$ code exec --sandbox read-only --model gpt-5 -c 'model_reasoning_effort="low"' \
  <<< "Test prompt"
✅ Completes in 6-11 seconds
✅ Returns valid JSON

Orchestrated execution:
create_agent_from_config_name("gpt_low", ..., false) // tmux_enabled=false
❌ Spawns successfully
❌ Never completes
❌ No output
❌ No error
```

**Difference**: Orchestration layer (create_agent_internal → tokio::spawn → execute_agent)

---

### Concern: Duplicate Agent Spawns

**User observation**: "Gemini running at the same time twice"

**SQLite evidence**:
- gemini: 2 spawns (quality_gate 02:38:16, regular_stage 02:39:53)
- claude: 2 spawns (quality_gate 02:38:16, regular_stage 02:40:15)

**Analysis**: This may be EXPECTED behavior:
- Quality gate checkpoint runs BEFORE plan stage
- Plan stage then runs with regular agents
- Same models used in both = appear as "duplicates"

**Needs verification**:
- Is this the intended design?
- Should quality gates use different models than regular stages?
- Or is there actual duplicate spawning bug?

---

### Concern: Prompt Consistency

**User observation**: "prompts seemed much different for each running llm agent"

**Expected**: Each agent gets agent-specific prompt from prompts.json, but same SPEC context

**Needs verification**:
- Capture actual prompts sent to gemini, claude, code
- Compare ${SPEC_ID}, ${CONTEXT}, ${MODEL_ID} substitution
- Check if context loading is consistent

---

## Session Statistics

**Duration**: ~4 hours
**Commits**: 4
**LOC added**: +722 (json_extractor.rs + consensus_db.rs)
**LOC removed**: -460 (old extraction functions)
**Net**: +262 LOC
**Tests**: 10 new (all passing)
**Files modified**: 7

---

## Next Session Priorities

### P0: Diagnose Code Agent Execution Path

**Goal**: Understand why code agent spawns but never completes

**Approach**:
1. Enable tmux for quality gates (creates result.txt for debugging)
2. Add comprehensive execution logging (spawn → execute → complete)
3. Capture actual command executed
4. Check for suppressed errors in async spawn
5. Compare working agents (gemini/claude) vs failing (code)

**Options if not fixable**:
- Exclude code from quality gates (use gemini+claude only)
- Use 2/2 consensus instead of 3/3
- Accept degraded mode as permanent

---

### P1: Verify Workflow Ordering

**Goal**: Document and validate actual execution flow

**Tasks**:
1. Trace pipeline_coordinator advance_spec_auto() logic
2. Document when quality gates run (before which stages?)
3. Verify gemini/claude 2x spawns are expected
4. Create workflow diagram (actual vs intended)

---

### P2: Validate Prompt Consistency

**Goal**: Verify all agents get consistent prompts with correct context

**Tasks**:
1. Add prompt capture to filesystem (/tmp/prompt-{agent}-{id}.txt)
2. Run test, compare prompts
3. Verify template variable substitution
4. Check context loading (${ARTIFACTS}, ${CONTEXT})

---

## Files for Next Session

**Read first**:
- docs/SPEC-KIT-928-orchestration-chaos/spec.md (this investigation)
- docs/SPEC-KIT-927-premature-output-collection/SESSION-SUMMARY-2025-11-11.md (session 1)
- docs/SPEC-KIT-927-premature-output-collection/spec.md (original bug)

**Code locations**:
- Agent spawn: `tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs:97`
- Execution: `core/src/agent_tool.rs:600` (execute_agent)
- Model execution: `core/src/agent_tool.rs:862` (execute_model_with_permissions)

**Diagnostic queries**:
```bash
# Agent spawn timeline
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, phase_type, spawned_at, completed_at
FROM agent_executions
WHERE spec_id='SPEC-KIT-900'
ORDER BY spawned_at;
"

# Extraction failures
sqlite3 ~/.code/consensus_artifacts.db "
SELECT agent_name, extraction_error, substr(response_text, 1, 500)
FROM agent_executions
WHERE spec_id='SPEC-KIT-900' AND extraction_error IS NOT NULL;
"
```

---

## Recommendations for Next Session

### Quick Win: Enable Tmux for Quality Gates

**Change**: `native_quality_gate_orchestrator.rs:104`
```rust
true, // tmux_enabled - needed for output capture
```

**Benefit**: Creates result.txt files, enables tmux pane inspection

**Risk**: Low (tmux already used for regular stages)

**Time**: 5 minutes + rebuild

---

### Strategic Decision: Code Agent Inclusion

**Evidence for exclusion**:
- 100% failure rate across 4 runs (0/4 completions)
- Manual execution works (not a model issue)
- Orchestration complexity causing failure
- gemini+claude sufficient for 2/2 consensus

**Evidence for continued investigation**:
- Represents 33% of quality gate perspective
- May uncover deeper orchestration bugs
- Manual test proves model/config viable

**Recommendation**: Try tmux enablement (5 min). If still fails, exclude from quality gates.

---

## Git State

**Branch**: main (43 commits ahead of origin)
**Working tree**: Clean
**Latest commit**: f119e7300 (Claude prompt fixes)

**Untracked** (cleaned):
- ~~docs/AGENT-RELIABILITY-*.md~~ (research docs, removed)

---

**End of Session 2**
