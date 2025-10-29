# Option A: Strategic Quality Gate Placement

**Date**: 2025-10-29
**Goal**: One quality gate per stage type, strategically placed for maximum value
**Status**: Design proposal

---

## Strategic Placement

### 1. Clarify → BEFORE specify

**Trigger**: After `/speckit.new`, before `/speckit.specify`
**Purpose**: Resolve ambiguities in initial description EARLY
**Value**: Prevents garbage-in → PRD is built on clear foundation

**What it checks**:
- Is the initial description clear?
- Are requirements contradictory?
- What needs clarification?

**Example**:
```
Input: "Add user auth"
Clarify detects: Missing details (OAuth? JWT? Email/password?)
Asks: "Which auth method? Which providers?"
Result: Clear spec before PRD generation
```

**Why here**: Cheapest place to fix (before any artifacts created)

---

### 2. Checklist → AFTER specify (BEFORE plan)

**Trigger**: After `/speckit.specify`, before `/speckit.plan`
**Purpose**: Validate PRD quality before planning work
**Value**: Ensures foundation is solid before investing in planning

**What it checks**:
- Are requirements measurable?
- Are acceptance criteria clear?
- Is scope well-defined?
- PRD quality score (0-100)

**Example**:
```
PRD has: "System should be fast"
Checklist flags: "fast" is vague (magnitude issue)
Suggests: "Response time <200ms for 95th percentile"
Auto-fix if unanimous
```

**Why here**: Last chance to fix PRD before work breakdown

---

### 3. Analyze → AFTER tasks (BEFORE implement)

**Trigger**: After `/speckit.tasks`, before `/speckit.implement`
**Purpose**: Full consistency check across ALL artifacts
**Value**: Catch contradictions before code generation

**What it checks**:
- PRD ↔ plan consistency
- plan ↔ tasks consistency
- PRD ↔ tasks consistency (transitive)
- No orphaned requirements
- No unplanned tasks

**Example**:
```
PRD requires: "Support OAuth providers: Google, GitHub"
Tasks only mention: "Implement Google OAuth"
Analyze detects: Missing GitHub OAuth task
Auto-adds: "Task: Implement GitHub OAuth integration"
```

**Why here**: Final check before expensive code generation

---

## Checkpoint Mapping

### Code Changes Required

**File**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`

**Current** (line ~883-892):
```rust
pub(super) fn determine_quality_checkpoint(
    stage: SpecStage,
    completed: &HashSet<QualityCheckpoint>,
) -> Option<QualityCheckpoint> {
    let checkpoint = match stage {
        SpecStage::Plan => QualityCheckpoint::PrePlanning,
        SpecStage::Tasks => QualityCheckpoint::PostPlan,
        SpecStage::Implement => QualityCheckpoint::PostTasks,
        _ => return None,
    };
    // ...
}
```

**New** (Option A):
```rust
pub(super) fn determine_quality_checkpoint(
    stage: SpecStage,
    completed: &HashSet<QualityCheckpoint>,
) -> Option<QualityCheckpoint> {
    let checkpoint = match stage {
        SpecStage::Specify => QualityCheckpoint::BeforeSpecify,  // NEW: Clarify
        SpecStage::Plan => QualityCheckpoint::AfterSpecify,      // NEW: Checklist
        SpecStage::Implement => QualityCheckpoint::AfterTasks,   // RENAMED: Analyze
        _ => return None,
    };

    if completed.contains(&checkpoint) {
        return None;
    }
    Some(checkpoint)
}
```

**File**: `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

**Current** (line ~627-636):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityCheckpoint {
    PrePlanning,
    PostPlan,
    PostTasks,
}
```

**New** (Option A):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QualityCheckpoint {
    BeforeSpecify,  // Runs: Clarify
    AfterSpecify,   // Runs: Checklist
    AfterTasks,     // Runs: Analyze
}

impl QualityCheckpoint {
    pub fn gates(&self) -> &[QualityGateType] {
        match self {
            Self::BeforeSpecify => &[QualityGateType::Clarify],
            Self::AfterSpecify  => &[QualityGateType::Checklist],
            Self::AfterTasks    => &[QualityGateType::Analyze],
        }
    }
}
```

---

## Workflow Impact

### Before (Current - Redundant)

```
/speckit.auto SPEC-ID
  ↓
Plan stage:
  - PrePlanning: Clarify + Checklist (2 gates)
  - Guardrail
  - Agents
  ↓
Tasks stage:
  - PostPlan: Analyze
  - Guardrail
  - Agents
  ↓
Implement stage:
  - PostTasks: Analyze ← DUPLICATE
  - Guardrail
  - Agents
```

**Issues**:
- Analyze runs twice (wasteful)
- Clarify + Checklist bundled (2 gates = longer wait)

---

### After (Option A - Strategic)

```
/speckit.auto SPEC-ID
  ↓
Specify stage:
  - BeforeSpecify: Clarify (resolve ambiguities EARLY)
  - Guardrail
  - Agents → generate PRD
  ↓
Plan stage:
  - AfterSpecify: Checklist (validate PRD quality)
  - Guardrail
  - Agents → generate plan
  ↓
Tasks stage:
  - Guardrail
  - Agents → generate tasks
  ↓
Implement stage:
  - AfterTasks: Analyze (full consistency check: PRD ↔ plan ↔ tasks)
  - Guardrail
  - Agents → generate code
```

**Benefits**:
- 3 gates total (same as before)
- Each gate at optimal position
- No duplicates
- Clear intent for each

---

## Value Proposition Per Gate

| Gate | When | Prevents | Fixes | Cost | ROI |
|------|------|----------|-------|------|-----|
| **Clarify** | Before specify | Ambiguous PRD | Unclear requirements | 1 gate (~8min, $0.30) | Prevents 1-2 re-specifications (~20min, $2) |
| **Checklist** | After specify | Weak PRD | Vague acceptance criteria | 1 gate (~8min, $0.30) | Prevents 1 failed planning cycle (~10min, $1) |
| **Analyze** | After tasks | Contradictions | PRD/plan/tasks mismatches | 1 gate (~8min, $0.30) | Prevents 1 failed implementation (~15min, $2) |

**Total Gate Cost**: ~24min, ~$0.90
**Total Prevention Value**: ~45min, ~$5 in avoided retries

**Net ROI**: 2x time savings, 5x cost savings

---

## ACE Integration Points (With Option A)

### Gate 1: Clarify (Before Specify)

**ACE Context**:
- Fetch scope: `clarify`
- Helpful bullets: "When X is ambiguous, ask about Y"
- Harmful bullets: "Avoid assuming Z without confirmation"

**Enhancement**:
- ACE suggests clarification questions based on past ambiguities
- Auto-resolves common ambiguities (e.g., "auth" → suggest OAuth vs JWT based on project context)

---

### Gate 2: Checklist (After Specify)

**ACE Context**:
- Fetch scope: `checklist`
- Helpful bullets: "Good PRDs include X, Y, Z"
- Harmful bullets: "Avoid vague terms like 'fast', 'soon'"

**Enhancement**:
- ACE recognizes quality patterns from past PRDs
- Suggests specific improvements based on learned best practices
- Flags patterns that previously caused implementation issues

---

### Gate 3: Analyze (After Tasks)

**ACE Context**:
- Fetch scope: `analyze`
- Helpful bullets: "Check for orphaned requirements in section X"
- Harmful bullets: "Don't overlook edge cases in Y"

**Enhancement**:
- ACE identifies inconsistency patterns from past runs
- Suggests specific cross-checks based on project patterns
- Recognizes when tasks miss requirements (learned from previous failures)

---

## Implementation Checklist

### Phase 1: Reorganize Quality Gates (2-3 hours)

- [ ] Update `QualityCheckpoint` enum in state.rs
  - Rename: PrePlanning → BeforeSpecify
  - Rename: PostPlan → AfterSpecify
  - Rename: PostTasks → AfterTasks
- [ ] Update `determine_quality_checkpoint()` in quality_gate_handler.rs
  - Change trigger: Plan → Specify (for BeforeSpecify)
  - Change trigger: Tasks → Plan (for AfterSpecify)
  - Keep trigger: Implement → AfterTasks
- [ ] Update `gates()` mapping in state.rs
  - BeforeSpecify → [Clarify]
  - AfterSpecify → [Checklist]
  - AfterTasks → [Analyze]
- [ ] Update tests to reflect new checkpoint names
- [ ] Update telemetry/evidence field names

---

### Phase 2: Fix ACE Injection (4-6 hours)

- [ ] Add to SpecAutoState in state.rs:
  - `ace_bullets_cache: Option<Vec<PlaybookBullet>>`
  - `ace_bullet_ids_used: Option<Vec<i32>>`
- [ ] In pipeline_coordinator.rs:advance_spec_auto():
  - Pre-fetch ACE bullets for upcoming stage
  - Store in state.ace_bullets_cache
- [ ] In agent_orchestrator.rs:auto_submit_spec_stage_prompt():
  - Inject cached bullets into prompt
  - Store bullet IDs in state
- [ ] After consensus in pipeline_coordinator.rs:
  - Send learning feedback via ace_learning
  - Clear cache for next stage
- [ ] Test end-to-end ACE loop

---

### Phase 3: Integrate ACE with Quality Gates (3-4 hours)

- [ ] In quality_gate_handler.rs:execute_quality_checkpoint():
  - Pass ACE bullets to gate prompt building
  - Include ACE context in quality agent prompts
- [ ] In quality.rs:should_auto_resolve():
  - Check ACE bullets for resolution patterns
  - Boost confidence if ACE confirms fix
  - Track ACE-assisted resolutions
- [ ] Test ACE-enhanced quality resolution
- [ ] Update telemetry to track ACE impact

---

### Phase 4: Update Documentation (1-2 hours)

- [ ] Update diagrams to show:
  - ACE pre-fetch before each stage
  - New quality checkpoint positions
  - ACE injection into prompts
  - Learning feedback loop
- [ ] Deprecate manual quality commands
- [ ] Create docs/spec-kit/QUALITY_GATES_GUIDE.md
- [ ] Update CLAUDE.md with corrected flow

---

## Timing Impact

### Current (3 checkpoints, analyze x2)

```
PrePlanning: clarify + checklist = ~16min
PostPlan: analyze = ~8min
PostTasks: analyze = ~8min ← duplicate
Total: ~32min in quality gates
```

### Option A (3 checkpoints, optimized)

```
BeforeSpecify: clarify = ~8min
AfterSpecify: checklist = ~8min
AfterTasks: analyze = ~8min
Total: ~24min in quality gates

Savings: 8min per /speckit.auto run
```

---

## Value Per Gate (Option A)

| Gate | Position | What It Prevents | Estimated Saves |
|------|----------|------------------|-----------------|
| **Clarify** | Before specify | Ambiguous PRD → bad plan → bad code | 1-2 re-specs (~20min, $2) |
| **Checklist** | After specify | Weak PRD → failed planning | 1 re-plan (~10min, $1) |
| **Analyze** | After tasks | Contradictions → wrong code | 1 re-implement (~15min, $2) |

**Cost**: ~$0.90 (3 gates × ~$0.30)
**Saves**: ~$5 in prevented retries
**ROI**: 5.5x

---

## Next Steps

1. Implement Quality Gate reorganization (Phase 1)
2. Fix ACE injection (Phase 2)
3. Integrate ACE with Quality (Phase 3)
4. Update docs/diagrams (Phase 4)

**Estimated Total**: 10-15 hours
**Impact**: Strategic quality + working ACE = self-improving system

Should I proceed with implementation?
