# CRITICAL FINDINGS: ACE & Quality Gate Integration Issues

**Date**: 2025-10-29
**Severity**: 🔴 HIGH - Core feature not functioning
**Impact**: ACE framework (Agentic Context Engine) is completely disabled
**Discovery**: Diagram creation + ultrathink analysis

---

## 🚨 Issue #1: ACE Is Not Being Used (BROKEN)

### The Problem

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/ace_prompt_injector.rs:174-179`

```rust
// Note: Cannot use block_on when already on tokio runtime (TUI context)
// For now, skip ACE injection in sync contexts
// TODO: Make prompt assembly async or use channels
warn!("ACE injection skipped: cannot block_on from within tokio runtime");
warn!("This is a known limitation - ACE injection needs async prompt assembly");
prompt // Returns prompt UNCHANGED
```

**Impact**:
- ❌ ACE playbook heuristics are NEVER injected into prompts
- ❌ No learned patterns from previous runs
- ❌ Agents don't benefit from historical knowledge
- ❌ Every run starts from scratch (no improvement over time)

**Evidence**:
- `inject_ace_section()` always returns prompt unchanged
- `should_use_ace()` checks are meaningless (always skipped)
- ACE client, curator, reflector, orchestrator modules exist but unused
- Config has `ace.use_for = ["speckit.specify", "speckit.implement"]` but ignored

### Why This Happens

**Technical Cause**: Async/sync boundary conflict
- TUI runs on tokio runtime (async context)
- `inject_ace_section()` is synchronous function
- Needs to call async MCP function (`playbook.slice`)
- Can't use `block_on()` when already in async context (causes panic)

**Options Tried** (none implemented):
1. Make prompt assembly async (requires changing call chain)
2. Use channels (adds complexity)
3. Pre-fetch ACE data (caching layer)

---

## 🎯 Issue #2: Duplicate Quality Systems

### The Confusion

**Two Separate Systems Exist**:

1. **Manual Quality Commands** (`/speckit.clarify`, `/speckit.analyze`, `/speckit.checklist`)
   - User-triggered optional commands
   - Implemented in `commands/quality.rs`
   - Expand prompts via `format_subagent_command`
   - NOT integrated into `/speckit.auto` pipeline

2. **Automatic Quality Gates** (Integrated into pipeline)
   - Triggered at 3 checkpoints: PrePlanning, PostPlan, PostTasks
   - Use same concepts: `QualityGateType::{Clarify, Analyze, Checklist}`
   - Implemented in `quality_gate_handler.rs` (925 lines)
   - spawn agents, auto-resolve issues, escalate to human
   - **Already automatic in /speckit.auto!**

### Current Checkpoint Mapping

From `state.rs:643-645`:
```rust
QualityCheckpoint::PrePlanning => &[QualityGateType::Clarify, QualityGateType::Checklist],
QualityCheckpoint::PostPlan    => &[QualityGateType::Analyze],
QualityCheckpoint::PostTasks   => &[QualityGateType::Analyze],
```

**So**: Clarify/Analyze/Checklist ARE already automatic gates within /speckit.auto!

**User's diagrams showed**: Optional manual commands (which is technically correct but misleading)

**Reality**: Quality gates use these checks automatically, manual commands are redundant

---

## 🔍 What's Actually Happening

### Current `/speckit.auto` Flow (Corrected)

```
1. /speckit.auto SPEC-ID
2. Stage: Plan
   a. Quality Gate: PrePlanning
      - Clarify gate (check ambiguities)
      - Checklist gate (score requirements)
      - Auto-resolve or escalate
   b. Guardrail: /guardrail.plan
   c. Agents: Gemini + Claude + GPT-Pro
   d. Consensus check
3. Stage: Tasks
   a. Quality Gate: PostPlan
      - Analyze gate (check consistency)
      - Auto-resolve or escalate
   b. Guardrail: /guardrail.tasks
   c. Agents: Gemini + Claude + GPT-Pro
   d. Consensus check
4. Stage: Implement
   a. Quality Gate: PostTasks
      - Analyze gate (check consistency)
      - Auto-resolve or escalate
   b. Guardrail: /guardrail.implement
   c. Agents: Gemini + Claude + GPT-Codex + GPT-Pro (Tier 3)
   d. Consensus check
5. Stages: Validate, Audit, Unlock
   - No quality gates (yet)
   - Standard guardrail → agents → consensus flow
```

**Missing**: ACE injection before each stage (supposed to happen but doesn't)

---

## 🎯 What Should Be Happening

### Ideal Flow (User's Vision)

**Before Each Stage**:
1. **ACE Injection**: Inject playbook heuristics (helpful/harmful bullets)
2. **Quality Gate**: Run automatic quality checks (if checkpoint defined)
3. **Guardrail**: Validate policy compliance
4. **Agents**: Execute with ACE context + quality fixes
5. **Consensus**: Synthesize results

**ACE Purpose**:
- Learn from past mistakes (avoid harmful patterns)
- Apply proven strategies (use helpful patterns)
- Improve over time (each run teaches the playbook)

**Quality Gate Purpose**:
- Detect issues before they propagate (ambiguities, inconsistencies)
- Auto-fix when unanimous (55% of cases)
- Escalate when uncertain (5% of cases)

**Combined Power**: ACE + Quality Gates = Self-improving, self-correcting system

---

## 💡 Solutions

### Solution 1: Fix ACE Injection (Async Boundary)

**Approach A: Make Prompt Building Async** (Recommended)

```rust
// Change signature in spec_prompts.rs
pub async fn build_stage_prompt_async(
    stage: SpecStage,
    raw_args: &str,
    mcp_manager: Option<Arc<McpConnectionManager>>,
    ace_client: Option<Arc<AceClient>>, // NEW
) -> Result<String, PromptBuildError> {
    // Existing prompt assembly...

    // NEW: ACE injection
    if let Some(ace) = ace_client {
        let bullets = ace.fetch_playbook_slice(scope, repo_root, branch).await?;
        let (ace_section, bullet_ids) = format_ace_section(&bullets);
        prompt.push_str(&ace_section);

        // Store bullet_ids for learning feedback
    }

    Ok(prompt)
}
```

**Changes Required**:
1. Make `build_stage_prompt` async
2. Update `auto_submit_spec_stage_prompt` to be async
3. Use async call chain from pipeline_coordinator
4. Already on tokio runtime, so safe to await

**Complexity**: Medium (requires async refactoring of call chain)
**Benefit**: ACE actually works

---

**Approach B: Pre-fetch ACE Data** (Faster to implement)

```rust
// In pipeline_coordinator.rs before spawning agents
pub fn advance_spec_auto(widget: &mut ChatWidget) {
    // ...existing logic...

    // NEW: Pre-fetch ACE bullets for upcoming stage
    if let Some(ace_config) = &widget.config.ace {
        let bullets = block_on_sync(|| {
            let ace = widget.ace_client.clone();
            let scope = command_to_scope(stage.command_name());
            async move {
                ace.fetch_playbook_slice(scope?, repo_root, branch).await.ok()
            }
        });

        // Store in state for prompt building
        if let Some(state) = widget.spec_auto_state.as_mut() {
            state.ace_bullets_cache = bullets;
        }
    }

    // Then in agent_orchestrator, inject cached bullets synchronously
}
```

**Complexity**: Low (minimal changes)
**Benefit**: ACE works without async refactoring
**Drawback**: Caching adds state management

---

### Solution 2: Integrate ACE with Quality Gates

**Current**: Quality gates detect issues, auto-resolve or escalate
**Enhanced**: Quality gates use ACE to inform auto-resolution

```rust
// In quality_gate_handler.rs
pub fn resolve_quality_issue_with_ace(
    issue: &QualityIssue,
    ace_bullets: &[PlaybookBullet],
) -> Option<Resolution> {
    // Check if ACE has learned how to handle this issue type
    for bullet in ace_bullets.iter().filter(|b| b.helpful) {
        if issue_matches_bullet_pattern(issue, bullet) {
            return Some(Resolution {
                fix: bullet.text.clone(),
                confidence: bullet.confidence,
                source: "ACE playbook".to_string(),
            });
        }
    }

    // Fall back to existing auto-resolution logic
    should_auto_resolve(issue)
}
```

**Benefit**: ACE learns quality patterns, auto-resolves more issues over time

---

### Solution 3: Clarify Command Redundancy

**Current State**:
- `/speckit.clarify` exists as manual command
- `QualityGateType::Clarify` runs automatically at PrePlanning checkpoint
- User confusion: which one to use?

**Recommendation: Deprecate Manual Commands**

```rust
// Mark as deprecated
#[deprecated(since = "2025-10-29", note = "Use /speckit.auto instead - clarify runs automatically at PrePlanning checkpoint")]
pub struct SpecKitClarifyCommand;

// Add migration guide
// Old: /speckit.clarify SPEC-ID
// New: /speckit.auto SPEC-ID (clarify runs automatically)
// Or:  /speckit.auto SPEC-ID --from=plan (if already created)
```

**Alternative: Manual Commands as "Force Re-Run"**

Keep commands but change semantics:
- `/speckit.clarify` → Force re-run clarify gate even if checkpoint passed
- Useful for iterating on quality before committing to full pipeline

---

## 📋 Proposed Changes

### Phase 1: Fix ACE Injection (HIGH PRIORITY)

**Deliverables**:
- [ ] Implement Approach B (pre-fetch ACE bullets)
- [ ] Add `ace_bullets_cache` to SpecAutoState
- [ ] Update agent_orchestrator to inject cached bullets
- [ ] Test ACE bullets appear in prompts
- [ ] Verify ACE learning feedback loop works

**Estimated Effort**: 4-6 hours
**Risk**: Low (caching pattern is well-understood)

---

### Phase 2: Enhance Quality Gates with ACE (MEDIUM PRIORITY)

**Deliverables**:
- [ ] Pass ACE bullets to quality gate resolution logic
- [ ] Implement `resolve_quality_issue_with_ace()`
- [ ] Track ACE-based resolutions in telemetry
- [ ] Send learning feedback when ACE resolution succeeds/fails

**Estimated Effort**: 3-4 hours
**Risk**: Low (extends existing system)

---

### Phase 3: Clarify Manual vs Automatic (LOW PRIORITY)

**Deliverables**:
- [ ] Update documentation to explain quality gates are automatic
- [ ] Deprecate manual quality commands OR repurpose as "force re-run"
- [ ] Add migration guide for users
- [ ] Update diagrams to show automatic integration

**Estimated Effort**: 2 hours
**Risk**: None (documentation only)

---

## 🎨 Updated Diagram Concepts

### Before (Current - Incorrect)

```
User runs /speckit.auto
  ↓
Stages run WITHOUT ACE
  ↓
Quality commands are optional manual steps
```

### After (Proposed - Correct)

```
User runs /speckit.auto
  ↓
FOR EACH STAGE:
  1. Pre-fetch ACE bullets (learned heuristics)
  2. Quality Gate checkpoint (if applicable)
     - Clarify: detect ambiguities
     - Checklist: score requirements
     - Analyze: check consistency
     - Use ACE to auto-resolve issues
  3. Guardrail validation
  4. Inject ACE bullets into agent prompts
  5. Spawn agents (Gemini, Claude, GPT, etc.)
  6. Consensus check
  7. Send learning feedback to ACE
  8. Advance to next stage
```

**Key Addition**: ACE is active at steps 1, 2 (via quality), 4, 7

---

## 📊 Impact Analysis

### Current State (Broken ACE)

**Pros**:
- System works without ACE (degraded but functional)
- Quality gates still catch issues
- No ACE database corruption risk

**Cons**:
- ❌ No learning from past runs
- ❌ Repeats same mistakes
- ❌ No harmful pattern avoidance
- ❌ Agents start from zero every time
- ❌ ACE infrastructure is dead weight (unused modules)

---

### Fixed State (Working ACE + Enhanced Quality Gates)

**Benefits**:
1. **Self-Improving System**: Each run teaches the playbook
2. **Avoided Mistakes**: Harmful patterns blocked automatically
3. **Faster Resolutions**: ACE suggests fixes for known issues
4. **Better Quality**: Quality gates + ACE = stronger checks
5. **Lower Costs**: Fewer retries when agents have good context

**Risks**:
1. **ACE Database Bloat**: Playbook could grow unbounded
   - Mitigation: Prune low-confidence bullets periodically
2. **Bad Heuristics**: ACE could learn incorrect patterns
   - Mitigation: Human review of new bullets, confidence thresholds
3. **Async Complexity**: Making prompt building async touches many modules
   - Mitigation: Use Approach B (caching) for quick win

---

## 🔬 Technical Deep-Dive

### Why ACE Injection Fails

**Call Stack**:
```
ChatWidget::submit_user_message (sync)
  ↓
auto_submit_spec_stage_prompt (sync)
  ↓
build_stage_prompt_with_mcp (sync)
  ↓
inject_ace_section (sync) ← WANTS TO CALL ASYNC MCP
  ↓
ace_client.fetch_playbook_slice (async) ← CAN'T CALL FROM SYNC
```

**Problem**: Can't call async from sync when already on tokio runtime
**Solution**: Either make entire chain async OR pre-fetch data

---

### Why Caching Works

**Pre-fetch in Pipeline Coordinator**:
```rust
// pipeline_coordinator.rs:advance_spec_auto()
// This runs in async context, can await
let stage = state.stages[state.current_index];
let ace_bullets = if let Some(ace_client) = &widget.ace_client {
    block_on_sync(|| {
        let client = ace_client.clone();
        let scope = command_to_scope(stage.command_name())?;
        async move {
            client.fetch_playbook_slice(scope, repo, branch).await.ok()
        }
    })
} else {
    None
};

// Store in state
state.ace_bullets_cache = ace_bullets;
```

**Then use cached data synchronously**:
```rust
// agent_orchestrator.rs:auto_submit_spec_stage_prompt()
// This is sync, but uses pre-fetched data
if let Some(bullets) = &state.ace_bullets_cache {
    let (ace_section, bullet_ids) = format_ace_section(bullets);
    prompt.push_str(&ace_section);
    state.ace_bullet_ids_used = Some(bullet_ids);
}
```

**Benefit**: No async refactoring needed, works with existing architecture

---

## 📐 Architectural Recommendations

### Short Term (Fix ACE - 1 week)

1. **Implement Pre-fetch Caching**
   - Add `ace_bullets_cache: Option<Vec<PlaybookBullet>>` to SpecAutoState
   - Pre-fetch in `advance_spec_auto()` before guardrail phase
   - Inject cached bullets in `auto_submit_spec_stage_prompt()`
   - Clear cache after stage completes

2. **Test ACE Learning Loop**
   - Verify bullets appear in prompts
   - Test learning feedback (ace_learning.rs)
   - Confirm playbook grows over time
   - Validate bulletin pruning logic

3. **Update Documentation**
   - Explain ACE is now active
   - Show example ACE bullets in prompts
   - Document learning feedback mechanism

---

### Medium Term (ACE + Quality - 2 weeks)

4. **Integrate ACE with Quality Gate Resolution**
   - Pass ACE bullets to `should_auto_resolve()`
   - Check if ACE has seen this issue before
   - Use ACE confidence to boost auto-resolution rate
   - Target: 55% → 70% auto-resolution

5. **Add ACE Reflection to Quality Escalations**
   - When human resolves escalated issue, ask ACE to learn
   - Store resolution pattern for future auto-resolution
   - Build corpus of quality patterns

6. **Implement ACE Curator Integration**
   - Use ace_curator to strategically manage playbook
   - Prune low-confidence bullets
   - Consolidate duplicate patterns
   - Keep playbook size bounded (<100 bullets per scope)

---

### Long Term (Full Integration - 1 month)

7. **Make Prompt Building Fully Async**
   - Refactor `build_stage_prompt()` to async
   - Update call chain through agent_orchestrator
   - Remove caching workaround
   - Cleaner architecture

8. **Add Quality Gates to Remaining Stages**
   - PostImplement: Code quality checks
   - PostValidate: Test coverage analysis
   - PostAudit: Compliance verification
   - Complete 6-checkpoint coverage

9. **Implement ACE Orchestrator Full Cycle**
   - After each stage: reflect on outcomes
   - Extract patterns automatically
   - Curate playbook strategically
   - Continuous improvement loop

---

## 🚨 Why This Matters

### The Promise of ACE

**Vision**: "Learn from every run, never repeat mistakes"

**Current Reality**: "Start from scratch every time"

**With Working ACE**:
- Run 1: Generate code, encounter borrow checker error
- ACE learns: "Avoid X pattern, use Y instead"
- Run 2: Agents see bullet "[avoid] X pattern causes borrow errors"
- Result: No borrow error, faster success

**Without ACE** (current state):
- Run 1: Borrow error
- Run 2: Same borrow error
- Run 3: Same borrow error
- Manual fix required every time

---

## 📊 Cost Impact

### ACE Overhead

**Pre-fetch Cost**: Minimal
- `playbook.slice` is local SQLite query
- ~10ms per call
- No API costs

**Injection Overhead**: ~50-100 tokens per prompt
- 8 bullets × ~12 tokens = ~96 tokens
- At $0.15/1M input tokens = $0.000015 per stage
- **Negligible cost**

**Learning Feedback**: Free
- Local database write
- Async fire-and-forget
- No blocking

**ROI**:
- Cost: ~$0.0001 per run
- Benefit: Potentially avoid 1 retry = save ~$1-2
- **10,000x return on investment**

---

## 🎯 Immediate Action Items

### Must Do (This Week)

1. **Implement ACE Pre-fetch Caching**
   - File: `pipeline_coordinator.rs`
   - Function: `advance_spec_auto()`
   - Add to SpecAutoState

2. **Enable ACE Injection in Prompts**
   - File: `agent_orchestrator.rs`
   - Function: `auto_submit_spec_stage_prompt()`
   - Use cached bullets

3. **Test End-to-End**
   - Run `/speckit.auto` with ACE enabled
   - Verify bullets in prompt
   - Check learning feedback fires
   - Confirm playbook updates

4. **Update Diagram 2 (Pipeline)**
   - Add "Pre-fetch ACE bullets" step
   - Show ACE injection before agents
   - Show learning feedback after consensus

---

### Should Do (Next Sprint)

5. **Integrate ACE with Quality Gates**
   - File: `quality.rs`
   - Function: `should_auto_resolve()`
   - Add ACE pattern matching

6. **Document ACE Activation**
   - Create docs/spec-kit/ACE_INTEGRATION.md
   - Explain how ACE works
   - Show examples of learned patterns

7. **Monitor ACE Database Growth**
   - Add `/speckit.ace-stats` command
   - Show bullets per scope
   - Highlight low-confidence candidates for pruning

---

## 🔗 Related Issues

- **SPEC-KIT-070**: ACE-aligned routing (partially implemented, ACE itself broken)
- **SPEC-KIT-072**: Migrate consensus to dedicated DB (would help ACE performance)
- **T85**: Quality gates implementation (working but could use ACE)

---

## 🎓 Lessons Learned

### How This Went Unnoticed

1. **Tests Pass**: No test validates ACE injection actually happens
2. **System Works**: Quality gates provide value without ACE
3. **TODO Comments**: Marked as TODO but never prioritized
4. **Module Exists**: ACE code is there, just not called

### What Caught It

✅ **Diagram Creation**: Forcing visualization revealed the gap
✅ **User Intuition**: "It's not clear that ACE is being used" - correct!
✅ **Code Reading**: Following call chain showed the warning logs
✅ **Ultrathink Mode**: Deep analysis vs surface-level validation

**This is exactly why diagram-driven analysis is powerful**

---

## 📝 Conclusion

**Status**: 🔴 Critical feature broken but system still functional

**Root Cause**: Async/sync boundary issue left unresolved

**Fix Complexity**: Medium (4-6 hours for caching approach)

**Impact of Fix**: Transform spec-kit from "dumb automation" to "self-improving AI system"

**Next Steps**:
1. Implement ACE pre-fetch caching (Approach B)
2. Test end-to-end learning loop
3. Integrate ACE with quality gate resolution
4. Update diagrams to reflect correct flow
5. Document ACE activation for users

**This is a high-value fix** - relatively small effort for major capability unlock.

---

**Document Version**: 1.0
**Severity**: HIGH (feature completely disabled)
**Priority**: Should fix before next release
**Owner**: Needs assignment
