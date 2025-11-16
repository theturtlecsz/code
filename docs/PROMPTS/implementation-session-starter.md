# IMPLEMENTATION SESSION STARTER: SPEC-947/948/949 Progressive Development

**Session Type**: Implementation Execution (Progressive, Quality-Focused)
**Context**: 3 implementation SPECs ready to execute (94/100 readiness score post-blocker resolution)
**Approach**: Agile progressive development with continuous validation
**Quality Standard**: No compromise on testing, validation, or evidence capture

---

## SESSION INITIALIZATION PROTOCOL

### Step 1: Load Context & Assess Current State (5 minutes)

**MANDATORY CONTEXT LOADING**:

```markdown
# 1. Load Implementation Readiness Report
READ: docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md

Key sections to review:
- Executive Summary (current state, readiness score)
- Critical Findings (4 blockers - check if resolved)
- Unified Execution Plan (week-by-week schedule)
- Risk Monitoring Dashboard (active risks)
- Immediate Action Items (what to do first)

# 2. Query Current SPEC Status
READ: SPEC.md lines 137-141

Extract:
- SPEC-949-IMPL status (BACKLOG? In Progress? Done?)
- SPEC-948-IMPL status
- SPEC-947-IMPL status
- Which SPECs have branches/PRs open?
- What's the last completed milestone?

# 3. Check Git State
RUN: git status
RUN: git log --oneline --grep="spec-949\|spec-948\|spec-947" -10

Verify:
- Clean working tree (or understand dirty state)
- Current branch (should be feature branch or main)
- Recent commits (any implementation work already done?)

# 4. Query Local-Memory for Recent Implementation Work
USE: mcp__local-memory__search
PARAMETERS:
  query: "SPEC-949 SPEC-948 SPEC-947 implementation milestone progress"
  search_type: "semantic"
  use_ai: true
  limit: 10
  tags: ["type:milestone", "spec:SPEC-949", "spec:SPEC-948", "spec:SPEC-947"]
  response_format: "concise"

Extract:
- What implementation work was completed in previous sessions?
- Were any blockers encountered?
- Were any decisions made that affect current plan?
```

**OUTPUT**: Status report answering:
- Readiness report blockers: Resolved? (Yes/No/Partial)
- Current SPEC in progress: (None/SPEC-949/SPEC-948/SPEC-947)
- Current Phase/Task: (e.g., "SPEC-949 Phase 2 Task 2.1 in progress")
- Blockers encountered: (List any issues from previous session)

---

### Step 2: Determine Starting Point (5 minutes)

**DECISION TREE**:

```
IF blockers NOT resolved:
  → START: Resolve Priority 1 blockers (55 minutes, see Section 3)
  → THEN: Resume at Step 2

ELIF no SPEC in progress:
  → START: SPEC-949 Phase 1 Task 1.1 (first task in sequence)
  → CREATE: Feature branch `spec-949-extended-model-support`
  → INITIALIZE: Todo list with SPEC-949 all phases

ELIF SPEC-949 in progress:
  → RESUME: Continue from last completed task
  → LOAD: SPEC-949 implementation plan completely
  → VERIFY: Last task's validation passed (tests, compilation)

ELIF SPEC-949 complete, SPEC-948 not started:
  → START: SPEC-948 Phase 1 Task 1.1 (CRITICAL PATH)
  → CREATE: Feature branch `spec-948-modular-pipeline-logic`
  → RUN: Integration test INT-1 (GPT-5 model validation)

ELIF SPEC-948 in progress:
  → RESUME: Continue from last completed task
  → VERIFY: pipeline_config.rs exists if past Phase 1 (CRITICAL)

ELIF SPEC-948 complete, SPEC-947 not started:
  → RUN: SPEC-947 Phase 1 verification (API checklist)
  → START: SPEC-947 Phase 2 Task 2.1 (widget core)
  → CREATE: Feature branch `spec-947-pipeline-ui-configurator`

ELIF SPEC-947 in progress:
  → RESUME: Continue from last completed task
  → VERIFY: SPEC-948 pipeline_config.rs exists (hard dependency)

ELIF all 3 SPECs complete:
  → START: Integration testing (INT-1 through INT-7)
  → CREATE: Integration evidence directory
  → RUN: 7 integration checkpoints per readiness report

ELIF integration testing complete:
  → START: Manual MVP testing and final validation
  → CREATE: MVP test plan
  → EXECUTE: End-to-end user workflows
```

**OUTPUT**: Clear starting point (e.g., "Start: SPEC-949 Phase 1 Task 1.1")

---

## SECTION 3: RESOLVE BLOCKERS FIRST (If Not Done)

**Prerequisites**: Only execute if readiness report blockers NOT YET resolved

**Estimated Duration**: 55 minutes (do NOT skip - prevents 6-12h debugging waste)

### Blocker #1: Fix File Path References (30 minutes)

**Task**: Update file paths in all 3 implementation specs to reference actual implementation files

**Steps**:

1. **SPEC-949 Task 2.2 File Path Correction** (10min):
   ```bash
   # Verify location
   grep -n "agents.*gpt\|gemini\|claude" codex-rs/tui/src/chatwidget/spec_kit/subagent_defaults.rs | head -10

   # Expected: Lines 34, 41, 51, 58, 65, 72 (agent arrays in make_config() calls)
   ```

   - **File to edit**: docs/SPEC-949-extended-model-support/implementation-plan.md
   - **Find**: Line 160 (Task 2.2)
   - **Change**:
     ```markdown
     # FROM:
     - **File**: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` OR `router.rs` (depending on subagent config location)

     # TO:
     - **File**: `codex-rs/tui/src/chatwidget/spec_kit/subagent_defaults.rs`
     - **Lines to modify**: 41 (speckit.specify), 51 (speckit.plan), 58 (speckit.tasks), 65 (speckit.implement), 72 (speckit.validate), ~79 (speckit.audit/unlock)
     - **Changes**: Replace agent names in arrays:
       - Line 41: ["gemini", "claude", "code"] → ["gpt5_1_mini"]
       - Line 51: ["gemini", "claude", "gpt_pro"] → ["gemini", "claude", "gpt5_1"]
       - Line 65: [..., "gpt_codex", "gpt_pro"] → ["gpt5_1_codex", "claude"]
     ```

2. **SPEC-948 Task 3.2 CLI Parsing Location** (15min):
   ```bash
   # Find where /speckit.auto arguments are parsed
   grep -rn "handle_spec_auto\|parse.*args\|speckit\.auto" codex-rs/tui/src/chatwidget/spec_kit/*.rs

   # Likely: pipeline_coordinator.rs or command_handlers.rs
   # Find function signature, note parameter list
   ```

   - **File to edit**: docs/SPEC-948-modular-pipeline-logic/implementation-plan.md
   - **Find**: Line 582 (Task 3.2)
   - **Update**: Specify exact file and function found in grep results
   - **Example**:
     ```markdown
     - **File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
     - **Function**: `handle_spec_auto` (line 105 or wherever found)
     - **Action**: Add `cli_overrides: Option<PipelineOverrides>` parameter
     ```

3. **SPEC-947 Task 4.2 Command Registration** (5min):
   - **File to edit**: docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md
   - **Find**: Line 712 (Task 4.2)
   - **Change**:
     ```markdown
     # FROM:
     - **File**: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` (or wherever commands are registered)

     # TO:
     - **File**: `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs`
     - **Pattern**: Follow existing registration at lines 280, 302, 339
     - **Action**: Add entry for "speckit.configure" command
     ```

**Validation**: Re-read Task 2.2 (SPEC-949), Task 3.2 (SPEC-948), Task 4.2 (SPEC-947) - confirm file paths are exact

**Commit**:
```bash
git add docs/SPEC-94{7,8,9}*/implementation-plan.md
git commit -m "docs(spec-947-948-949): correct file path references

- SPEC-949: handler.rs → subagent_defaults.rs (agent array location)
- SPEC-948: Specify CLI parsing location (pipeline_coordinator.rs)
- SPEC-947: handler.rs → command_registry.rs (command registration)

Prevents 6-12h debugging waste during implementation.

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Blocker #2: Fix SPEC-936 Dependency Claim (5 minutes)

**Task**: Correct misleading dependency reference in SPEC-949

**Steps**:

1. **Verify ProviderRegistry exists**:
   ```bash
   grep -n "pub struct ProviderRegistry" codex-rs/core/src/async_agent_executor.rs
   # Expected: Line 434
   ```

2. **Update SPEC-949**:
   - **File**: docs/SPEC-949-extended-model-support/implementation-plan.md
   - **Find**: Line 6
   - **Change**:
     ```markdown
     # FROM:
     **Dependencies**: SPEC-936 ProviderRegistry infrastructure (95% complete)

     # TO:
     **Dependencies**: ProviderRegistry (exists: async_agent_executor.rs:434), Config infrastructure (exists: config_types.rs)
     ```

**Validation**: Read line 6, confirm ProviderRegistry marked as existing (not pending)

**Commit**: Include in same commit as Blocker #1 (documentation corrections)

---

### Blocker #3: Add Cost Timeline Clarification (15 minutes)

**Task**: Add timeline notes to all 3 specs clarifying pre/post SPEC-949 cost baselines

**Steps**:

1. **SPEC-949 Executive Summary** (5min):
   - **File**: docs/SPEC-949-extended-model-support/implementation-plan.md
   - **Find**: Line 14 (end of Executive Summary section)
   - **Add after line 21** (after Strategic Impact bullets):
     ```markdown

     **Cost Baseline Context**:
     - **Current (Pre-SPEC-949)**: $2.71 per /speckit.auto (GPT-4 agents)
     - **Target (Post-SPEC-949)**: $2.36 per /speckit.auto (GPT-5 agents)
     - **Reduction**: -13% (-$0.35 per run)

     All cost comparisons in this spec reference the $2.71 → $2.36 migration context. SPEC-948 and SPEC-947 will use $2.36 as the new baseline after this implementation completes.
     ```

2. **SPEC-948 Executive Summary** (5min):
   - **File**: docs/SPEC-948-modular-pipeline-logic/implementation-plan.md
   - **Add after line 21**:
     ```markdown

     **Cost Baseline Note**: This spec assumes SPEC-949 GPT-5 migration is complete (baseline $2.36 per full pipeline). Pre-SPEC-949 baseline was $2.71 (GPT-4 agents). Workflow cost ranges ($0.66-$2.71) reflect partial pipeline execution from the $2.36 baseline.
     ```

3. **SPEC-947 Executive Summary** (5min):
   - **File**: docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md
   - **Add after line 21**:
     ```markdown

     **Cost Baseline Note**: Real-time cost display ranges from $0.66 (minimal workflow) to $2.36 (full pipeline, post-SPEC-949 GPT-5 migration). Pre-SPEC-949 baseline was $2.71 (GPT-4 agents).
     ```

**Validation**: Read all 3 Executive Summaries, confirm timeline context is clear

**Commit**: Include in documentation corrections commit

---

### Blocker #4: Clarify Test Count Baseline (5 minutes)

**Task**: Make SPEC-947's test count dependency on SPEC-948 explicit

**Steps**:

1. **SPEC-947 Success Criteria**:
   - **File**: docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md
   - **Find**: Line 1106
   - **Change**:
     ```markdown
     # FROM:
     2. **Tests Passing**: 100% pass rate maintained (634+ existing + 17-21 new = 651-655 total)

     # TO:
     2. **Tests Passing**: 100% pass rate maintained (634+ existing [includes SPEC-948's 24-30 tests] + 17-21 new = 651-655 total)
     ```

**Validation**: Verify math (604 current + 24-30 SPEC-948 = 628-634 ≈ 634)

**Commit**: Include in documentation corrections commit

---

### Blocker Resolution Complete (55 minutes)

**Final Validation**:
```bash
# Verify all changes staged
git status

# Review diff
git diff --staged

# Commit with comprehensive message
git commit -m "docs(spec-947-948-949): resolve 4 readiness report blockers

Blocker #1 - File Path Corrections:
- SPEC-949 Task 2.2: handler.rs → subagent_defaults.rs:41,51,58,65,72
- SPEC-948 Task 3.2: Specify CLI parsing location (grep results)
- SPEC-947 Task 4.2: handler.rs → command_registry.rs:280,302,339

Blocker #2 - SPEC-936 Dependency Claim:
- SPEC-949 line 6: Clarify ProviderRegistry exists (not pending SPEC-936)

Blocker #3 - Cost Timeline Clarification:
- All 3 specs: Add timeline notes ($2.71 GPT-4 → $2.36 GPT-5 context)

Blocker #4 - Test Count Baseline:
- SPEC-947 line 1106: Clarify 634 includes SPEC-948's tests

Readiness Score: 89/100 → 94/100 ✅ Ready for implementation.

Ref: docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

**Update Readiness Report**:
- Update score from 89 → 94
- Mark blockers as resolved
- Update recommendation to "✅ READY FOR IMPLEMENTATION"

**Store in Local-Memory**:
```
USE: mcp__local-memory__store_memory
CONTENT: "Implementation readiness blockers resolved (2025-11-16): Fixed 4 critical issues in SPEC-947/948/949 implementation plans. (1) File path corrections: handler.rs→subagent_defaults.rs for agent configs, →command_registry.rs for command registration. (2) SPEC-936 dependency corrected: ProviderRegistry already exists (async_agent_executor.rs:434), not pending completion. (3) Cost timeline clarified: $2.71 GPT-4 → $2.36 GPT-5 migration context. (4) Test count baseline explained: 634 includes SPEC-948's tests. Readiness: 89→94/100. Implementation approved. Pattern: Always verify file paths via grep before specifying in implementation plans."
TAGS: ["type:milestone", "spec:SPEC-949", "spec:SPEC-948", "spec:SPEC-947", "readiness-validation"]
IMPORTANCE: 9
DOMAIN: "spec-kit"
```

---

## SECTION 4: PROGRESSIVE IMPLEMENTATION FRAMEWORK

### Core Principles (Agile Best Practices)

**Principle 1: Continuous Validation** (Test-Driven Execution)
- Build + Test after **every task** (not just at phase end)
- Validation cycle: Code → Compile → Test → Clippy → Commit
- Never advance to next task with failing tests or compilation errors

**Principle 2: Evidence-Based Progress** (Measurable Outcomes)
- Capture evidence for every phase (test logs, telemetry, screenshots)
- Store milestones in local-memory (importance ≥8)
- Update SPEC.md task tracker after each phase completion

**Principle 3: Fail-Fast with Rollback Readiness** (Risk Mitigation)
- Commit atomically per task (enables git revert if needed)
- Test rollback procedure at each phase boundary
- Document degradations immediately (don't defer)

**Principle 4: Quality Over Speed** (No Compromises)
- 100% test pass rate mandatory at all times
- No "TODO" or placeholder code (finish task completely or don't commit)
- Peer review code examples against implementation before writing

**Principle 5: Iterative Check-ins** (Continuous Communication)
- Checkpoint after each phase (validate before advancing)
- Review risks at each milestone (update risk dashboard)
- Ask questions early (don't assume, don't guess)

---

### Definition of Done (Per Task)

**A task is DONE when ALL criteria met**:

1. ✅ **Code Complete**: All LOC written as specified in implementation plan
2. ✅ **Compiles**: `cargo build -p <package>` succeeds with 0 errors, 0 warnings
3. ✅ **Tests Written**: Unit/integration tests written per spec (not deferred)
4. ✅ **Tests Pass**: `cargo test -p <package> <module>` 100% pass rate
5. ✅ **Clippy Clean**: `cargo clippy -p <package>` 0 warnings
6. ✅ **Format Applied**: `cargo fmt --all` applied
7. ✅ **Validation Run**: Manual validation command executed (per spec's validation section)
8. ✅ **Evidence Captured**: Test output, telemetry files, or screenshots saved
9. ✅ **Committed**: Atomic git commit with conventional commit message
10. ✅ **Documented**: If task changes behavior, update inline comments or docs

**Never mark task complete if ANY criterion fails** - Fix issue or rollback task

---

### Phase Completion Checklist (Per Phase)

**A phase is DONE when ALL criteria met**:

1. ✅ **All Tasks Done**: Every task in phase meets Definition of Done (above)
2. ✅ **Phase Tests Pass**: All unit + integration tests for the phase passing
3. ✅ **Milestone Achieved**: Phase deliverable exists and validated
4. ✅ **Success Criteria Met**: Phase-specific success criteria from implementation plan confirmed
5. ✅ **No Regressions**: Existing tests still pass (`cargo test --workspace --no-fail-fast`)
6. ✅ **Evidence Collected**: Phase evidence directory populated (if applicable)
7. ✅ **SPEC.md Updated**: Task tracker shows phase complete
8. ✅ **Local-Memory Stored**: Milestone recorded (importance ≥8) if significant
9. ✅ **Committed**: Phase completion commit with summary
10. ✅ **Risks Reviewed**: Risk dashboard updated (any new risks? Any resolved?)

**Before advancing to next phase**: Review checklist, confirm 10/10 complete

---

### SPEC Completion Checklist (Per SPEC)

**A SPEC is DONE when ALL criteria met**:

1. ✅ **All Phases Done**: All 4 phases meet Phase Completion Checklist
2. ✅ **Overall Success Criteria**: SPEC's overall criteria section fully met
3. ✅ **Integration Tests**: SPEC-specific integration tests passing
4. ✅ **Documentation Complete**: User guides, rustdoc, CHANGELOG entry written
5. ✅ **Evidence Complete**: All validation evidence captured
6. ✅ **SPEC.md Updated**: Status → Done, Branch/PR filled, dated note added
7. ✅ **Local-Memory Stored**: SPEC completion milestone (importance 9)
8. ✅ **PR Created**: Feature branch → main, peer review requested
9. ✅ **Merged**: PR approved and merged to main
10. ✅ **Cleanup**: Feature branch deleted, evidence archived if needed

**Before advancing to next SPEC**: Review checklist, confirm 10/10 complete

---

## SECTION 5: EXECUTION WORKFLOW (Task-by-Task)

### Task Execution Template (Use for EVERY Task)

**BEFORE Starting Task**:

```markdown
## [TASK ID]: [Task Title]

**SPEC**: SPEC-XXX Phase Y Task Z
**File**: [Exact file path from implementation plan]
**Estimated Duration**: [Hours from implementation plan]
**Dependencies**: [List task IDs that must be complete]
**Validation Command**: [Specific command from implementation plan]

### Pre-Flight Checklist:
- [ ] Read task description completely from implementation plan
- [ ] Verify dependencies complete (previous tasks done)
- [ ] Verify file path exists or parent directory exists (for new files)
- [ ] Review code examples in implementation plan (understand before writing)
- [ ] Check current test count: `cargo test --list | wc -l` (baseline)
```

**DURING Task Execution**:

```markdown
### Implementation Notes:
- [Document decisions made during implementation]
- [Note any deviations from spec (with rationale)]
- [Flag any issues encountered]

### Code Written:
- File: [path]
- Lines: [start-end] or [+ LOC count]
- Changes: [Brief summary]
```

**AFTER Task Completion**:

```markdown
### Validation Results:
- [ ] ✅ Compilation: `cargo build -p <package>` → SUCCESS / FAILED (details)
- [ ] ✅ Tests: `cargo test -p <package> <module>` → X/X passing / Y failing (details)
- [ ] ✅ Clippy: `cargo clippy -p <package>` → 0 warnings / Z warnings (fix)
- [ ] ✅ Format: `cargo fmt --all` → Applied
- [ ] ✅ Manual Validation: [Command from spec] → [Result]

### Evidence:
- Test output: [Paste summary or save to evidence/]
- Metrics: [If applicable: LOC count, test count, performance]

### Commit:
```bash
git add [files]
git commit -m "feat(spec-XXX): Phase Y Task Z - [Brief description]

[Details of what was implemented]

Validation: [Test results, compilation status]

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

### Definition of Done Check:
- [ ] All 10 DoD criteria met (see Section 4)
- [ ] Ready to advance to next task
```

**IF Any Validation Fails**:
1. **DO NOT** mark task complete
2. **DO NOT** advance to next task
3. Fix issue immediately (debug, revise code)
4. Re-run validation until all criteria pass
5. **THEN** mark task complete and commit

---

### Phase Transition Template (Use Between Phases)

**BEFORE Starting Next Phase**:

```markdown
## Phase [X] → Phase [Y] Transition

### Phase [X] Completion Verification:
- [ ] All Phase X tasks complete (DoD met for each)
- [ ] Phase X milestone achieved: [Specific deliverable]
- [ ] Phase X success criteria met: [List from implementation plan]
- [ ] Phase X tests passing: [Count] tests, 100% pass rate
- [ ] No regressions: `cargo test --workspace` still passing
- [ ] Committed: Phase X completion commit pushed

### Phase [Y] Readiness:
- [ ] Read Phase Y section of implementation plan completely
- [ ] Understand Phase Y objective and deliverables
- [ ] Verify Phase Y dependencies met (Phase X complete, external deps exist)
- [ ] Review Phase Y tasks (estimate time, understand approach)
- [ ] Check Phase Y risks (from implementation plan risk section)

### Checkpoint Questions:
1. Are there any blockers preventing Phase Y start? (List or "None")
2. Are Phase Y file paths validated? (Grep to confirm locations)
3. Are Phase Y LOC estimates realistic? (Compare to Phase X actual)
4. Should any Phase Y tasks be reordered or split? (Judgment call)

**IF blockers identified**: Resolve before starting Phase Y
**IF no blockers**: Proceed to Phase Y Task 1
```

---

## SECTION 6: SPEC-SPECIFIC EXECUTION GUIDES

### SPEC-949: Extended Model Support (16-24 hours, Week 1-2)

**Feature Branch**: `spec-949-extended-model-support`

**Phase Sequence**:
1. Phase 1: Model Registration (4-6h, Days 1-2)
2. Phase 2: Config Integration (6-8h, Days 3-5)
3. Phase 3: Provider Stubs (4-6h, Days 6-7)
4. Phase 4: Migration & Validation (2-4h, Week 2)

**Critical Success Factors**:
- ✅ All 5 GPT-5 models registered in model_provider_info.rs HashMap
- ✅ Agent arrays in subagent_defaults.rs updated correctly (lines 41, 51, 58, 65, 72)
- ✅ Cost reduction measured: $2.30-$2.42 (target $2.36 ± 2.5%)
- ✅ Performance validated: <2.5min single-agent stages (50% faster minimum)

**Validation Commands** (Run after each phase):
```bash
# Phase 1
cargo test -p codex-core model_provider_info::tests::test_gpt5_models

# Phase 2
cargo test -p codex-tui spec_kit::tests::test_gpt5_agent_selection
# Manual: /speckit.plan SPEC-900 → verify gpt5_1 used in consensus

# Phase 3
cargo clippy -p codex-core --all-features
# Verify: No unused code warnings (dead_code attribute works)

# Phase 4
# Manual: Run test SPEC (/speckit.auto SPEC-900 or create small test)
# Measure: Total cost, stage durations, cache behavior
# Evidence: docs/SPEC-949-extended-model-support/evidence/cost_validation.md
```

**Milestone**: SPEC-949 complete → GPT-5 models operational, SPEC-948 can use for testing

**Next**: Run integration test INT-1 (GPT-5 in multi-agent consensus), then start SPEC-948

---

### SPEC-948: Modular Pipeline Logic (20-28 hours, Week 2-3)

**Feature Branch**: `spec-948-modular-pipeline-logic`

**Phase Sequence**:
1. Phase 1: Config Data Layer (6-8h, Days 1-3) **← CRITICAL PATH**
2. Phase 2: Pipeline Execution Logic (8-10h, Days 4-6)
3. Phase 3: CLI Flag Support (4-6h, Days 1-2 Week 3)
4. Phase 4: Documentation & Examples (2-4h, Days 3-4 Week 3)

**CRITICAL MILESTONE** (Phase 1):
- **Deliverable**: pipeline_config.rs (250-300 LOC)
- **Why Critical**: HARD DEPENDENCY for SPEC-947 (cannot start without this)
- **Validation**: File exists, compiles, 10-12 unit tests passing
- **Checkpoint**: After Phase 1 complete, IMMEDIATELY notify that SPEC-947 is unblocked

**Critical Success Factors**:
- ✅ pipeline_config.rs module created with API: load(), save(), validate(), is_enabled()
- ✅ Stage filtering works (skip validate → only 6/8 stages execute)
- ✅ CLI flags parsed correctly (--skip-*, --stages=)
- ✅ 3-tier precedence works (CLI > per-SPEC > global > defaults)

**Validation Commands** (Run after each phase):
```bash
# Phase 1
cargo test -p codex-tui pipeline_config::tests
# Check: 10-12 tests passing, precedence merge correct

# Phase 2
cargo test -p codex-tui spec_kit::pipeline::tests
# Manual: Create test pipeline.toml (skip validate, audit), run /speckit.auto
# Check: Skip telemetry files written to evidence/

# Phase 3
cargo test -p codex-tui spec_kit::cli_parsing::tests
# Manual: /speckit.auto SPEC-XXX --skip-validate → verify validate skipped

# Phase 4
# Manual: Test all 4 workflow examples (copy .toml to SPEC dir, execute)
# Check: rapid-prototyping.toml → cost ~$0.66, time ~20min
```

**Milestone**: SPEC-948 complete → Backend logic operational, SPEC-947 can integrate

**Next**: Verify SPEC-947 Phase 1 API checklist, then start SPEC-947 Phase 2

---

### SPEC-947: Pipeline UI Configurator (24-32 hours, Week 3-4)

**Feature Branch**: `spec-947-pipeline-ui-configurator`

**Phase Sequence**:
1. Phase 1: API Verification (0h - checklist only)
2. Phase 2: Widget Core (8-10h, Days 2-4 Week 3)
3. Phase 3: Interactive Components (6-8h, Days 5-7 Week 3)
4. Phase 4: Command Integration (4-6h, Days 1-2 Week 4)

**MANDATORY Phase 1 Verification** (Before starting Phase 2):
- [ ] PipelineConfig::load() exists in pipeline_config.rs
- [ ] PipelineConfig::save() exists
- [ ] PipelineConfig::validate() exists
- [ ] PipelineConfig::is_enabled(stage) exists
- [ ] StageType enum exists (8 variants)
- [ ] ValidationResult struct exists (warnings list)

**IF any checklist item fails**: Extend SPEC-948 pipeline_config.rs with missing methods (2-4h), then resume

**Critical Success Factors**:
- ✅ TUI modal renders correctly (80×70% centered overlay)
- ✅ Real-time cost calculation works (toggle stage → cost updates instantly)
- ✅ /speckit.configure SPEC-ID launches modal successfully
- ✅ Saves pipeline.toml correctly (round-trip: save → load → verify)

**Validation Commands** (Run after each phase):
```bash
# Phase 1 (Verification only)
grep -n "pub fn load\|pub fn save\|pub fn validate\|pub fn is_enabled" \
  codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs
# Check: All 4 methods exist

# Phase 2
cargo test -p codex-tui pipeline_configurator::tests
# Check: 6-8 widget state tests passing

# Phase 3
cargo test -p codex-tui spec_kit::stage_selector::tests
# Manual: Launch modal in TUI, navigate with ↑↓, toggle with Space
# Check: Cost updates in real-time

# Phase 4
cargo test -p codex-tui spec_kit::commands::configure::tests
# Manual: /speckit.configure SPEC-947 → modal launches
# Manual: Toggle stages, press 'q', verify pipeline.toml written
# Check: Round-trip (load → modify → save → load → verify)
```

**Milestone**: SPEC-947 complete → User-facing TUI feature operational

**Next**: Run integration tests INT-3, INT-4, INT-5 (cross-SPEC validation)

---

## SECTION 7: PROGRESS TRACKING SYSTEM

### Todo List Management (Per Session)

**At Session Start**:
```
CREATE todo list with current SPEC's all phases:

USE: TodoWrite
EXAMPLE for SPEC-949:
[
  {"content": "SPEC-949 Phase 1: Model Registration (4-6h, 2 tasks)", "status": "pending", "activeForm": "Executing Phase 1"},
  {"content": "SPEC-949 Phase 2: Config Integration (6-8h, 3 tasks)", "status": "pending", "activeForm": "Executing Phase 2"},
  {"content": "SPEC-949 Phase 3: Provider Stubs (4-6h, 3 tasks)", "status": "pending", "activeForm": "Executing Phase 3"},
  {"content": "SPEC-949 Phase 4: Migration & Validation (2-4h, 4 tasks)", "status": "pending", "activeForm": "Executing Phase 4"}
]
```

**During Implementation**:
- Mark phase as "in_progress" when starting
- Mark phase as "completed" IMMEDIATELY after Phase Completion Checklist passes (10/10)
- Update todo list every 1-2 hours (keep user informed of progress)

**At Phase Boundaries**:
- Add sub-tasks if phase is complex (e.g., break Phase 2 into 3 sub-todos)
- Remove tasks that become irrelevant (e.g., if optional task deferred)

---

### SPEC.md Task Tracker Updates

**When to Update**:
- Phase starts → Update Status to "In Progress"
- Phase completes → Keep "In Progress" until SPEC fully done
- SPEC completes → Update Status to "Done"
- PR created → Fill "Branch" column
- PR merged → Fill "PR" column, add dated note

**Update Template** (After Each Phase):
```markdown
# Example for SPEC-949 after Phase 2 complete:

| # | SPEC ID | Status | Notes |
|---|---------|--------|-------|
| 8 | SPEC-949-IMPL | **In Progress** | Phases 1-2 complete: 5 GPT-5 models registered, agent configs updated, 11-15 unit tests passing. Phase 3 in progress (provider stubs). |
```

**Update Template** (After SPEC Complete):
```markdown
| # | SPEC ID | Status | Branch | PR | Notes |
|---|---------|--------|--------|----|----|
| 8 | SPEC-949-IMPL | **Done** | spec-949-extended-model-support | #XXX | 2025-11-[DATE] ✅ COMPLETE: 5 GPT-5 models integrated, cost reduction achieved ($2.71→$2.36, -13%). Tests: 20-24 new (100% pass rate). Evidence: cost_validation.md. Enables SPEC-948 testing. |
```

---

### Local-Memory Milestone Storage

**WHEN to Store** (Importance ≥8 ONLY):

- ✅ Phase complete with significant deliverable (e.g., SPEC-948 Phase 1 - pipeline_config.rs)
- ✅ SPEC complete (all phases done, tests passing)
- ✅ Integration test passed (cross-SPEC validation)
- ✅ Critical bug fixed (non-obvious issue with reusable solution)
- ✅ Important decision made (architectural choice, design pattern selected)

**WHEN NOT to Store** (Routine progress):
- ❌ Task complete (use git commits instead)
- ❌ Compilation success (expected outcome)
- ❌ Session summaries (redundant with git + SPEC.md)

**Storage Template** (After Phase Complete):
```
USE: mcp__local-memory__store_memory

EXAMPLE - SPEC-948 Phase 1 Complete:
CONTENT: "SPEC-948 Phase 1 Complete: pipeline_config.rs module created (289 LOC actual vs 250-300 estimated). Implements 3-tier precedence (CLI > per-SPEC > global > defaults), dependency validation (hard deps error, soft deps warn), TOML serialization. API: load(), save(), validate(), is_enabled(). Tests: 12 unit tests (100% pass). CRITICAL: Unblocks SPEC-947 (hard dependency). Pattern: Precedence merge logic uses explicit merge() method iterating config layers, enables clear debugging. File: codex-rs/tui/src/chatwidget/spec_kit/pipeline_config.rs"
TAGS: ["type:milestone", "spec:SPEC-948", "component:config", "critical-path"]
IMPORTANCE: 9
DOMAIN: "spec-kit"

EXAMPLE - SPEC Complete:
CONTENT: "SPEC-949 Implementation Complete (2025-11-[DATE]): GPT-5/5.1 family integrated (5 models), Deepseek/Kimi provider stubs ready. Actual effort: 18h (vs 16-24h estimated, 75% of realistic). Cost reduction validated: $2.71→$2.35 actual (-13.3%, target -13%). Performance: 2.1× faster single-agent stages (specify 4min→1.9min). Tests: 21 new (604→625 total, 100% pass). Files modified: model_provider_info.rs (+60), config_template.toml (+60), subagent_defaults.rs (+22/-11), async_agent_executor.rs (+124). Pattern: Provider stub pattern (dead code, commented registration) enables future integration with zero refactor."
TAGS: ["type:milestone", "spec:SPEC-949", "cost-optimization", "model-integration"]
IMPORTANCE: 9
DOMAIN: "spec-kit"
```

---

## SECTION 8: QUALITY GATES & VALIDATION

### Continuous Validation Cycle (Every Task)

**Validation Sequence** (Run in order):

1. **Syntax Check**: `cargo build -p <package>`
   - Must pass: 0 errors, 0 warnings
   - If fails: Fix compilation errors before proceeding

2. **Type Check**: `cargo check -p <package>`
   - Faster than full build, catches type issues
   - If fails: Fix type errors

3. **Lint Check**: `cargo clippy -p <package> --all-targets`
   - Must pass: 0 warnings
   - If fails: Fix clippy suggestions (most are legitimate issues)

4. **Format Check**: `cargo fmt --all -- --check`
   - If fails: Run `cargo fmt --all` to auto-format
   - Commit formatted code

5. **Unit Tests**: `cargo test -p <package> <module>::tests`
   - Must pass: 100% of unit tests for modified modules
   - If fails: Debug test failures, fix code

6. **Integration Tests**: `cargo test -p <package> <feature>::integration_tests`
   - Must pass: 100% of integration tests for the feature
   - If fails: Debug, fix

7. **Regression Tests**: `cargo test --workspace --no-fail-fast`
   - Must pass: All existing tests (no regressions)
   - If fails: Identify regressed test, fix or revert change

8. **Manual Validation**: [Specific command from implementation plan]
   - Execute the validation command listed in phase's "Validation" section
   - Verify expected behavior occurs
   - Capture evidence (output, screenshot, telemetry)

**Validation Frequency**: After EVERY task (not just phase end)

**Quality Gate**: If ANY validation step fails, task is NOT done - fix immediately

---

### Checkpoint Reviews (After Each Phase)

**Phase Checkpoint Template**:

```markdown
## Phase [X] Checkpoint Review

**Completed**: [Date/Time]
**Duration**: [Actual hours] (Estimated: [Y hours], Variance: [+/- %])

### Deliverables Verification:
- [ ] All Phase X tasks complete (check implementation plan)
- [ ] All Phase X deliverables exist (files created/modified as specified)
- [ ] All Phase X tests written and passing ([N] tests, 100% pass rate)
- [ ] Phase X milestone achieved: [Specific outcome]

### Quality Validation:
- [ ] Compilation: `cargo build --workspace` → SUCCESS
- [ ] Tests: `cargo test --workspace` → [N]/[N] passing (100%)
- [ ] Clippy: `cargo clippy --workspace --all-targets` → 0 warnings
- [ ] Format: `cargo fmt --all -- --check` → No changes needed

### Evidence Capture:
- [ ] Test output saved: evidence/phase-[X]-tests.log
- [ ] Metrics captured: [LOC actual vs estimated, test count, performance if applicable]
- [ ] Telemetry validated: [If phase generates telemetry, verify schema v1.0]

### Success Criteria Met:
[List Phase X success criteria from implementation plan]
- [ ] Criterion 1: [Result]
- [ ] Criterion 2: [Result]
- [ ] ...

### Risk Review:
- Active risks affecting this phase: [List or "None"]
- New risks identified: [List or "None"]
- Risks resolved by this phase: [List or "None"]
- Risk dashboard update needed: [Yes/No]

### Ready for Next Phase?
- [ ] All checklist items above verified ✅
- [ ] No blockers for next phase
- [ ] Next phase dependencies met

**DECISION**: ✅ Advance to Phase [Y] / ⚠️ Resolve issues first / ❌ Rollback Phase [X]
```

**IF checkpoint fails**: Do NOT advance - resolve issues or rollback

---

## SECTION 9: INTEGRATION TESTING SCHEDULE

**Integration Tests** (Run at specific milestones, not all at once):

### Week 2 Integration Checkpoints

**INT-1: GPT-5 Model in Multi-Agent Consensus** (After SPEC-949 Phase 2):
- **When**: SPEC-949 Phase 2 complete, agent arrays updated
- **Test**: Run `/speckit.plan SPEC-900` (or small test SPEC)
- **Validate**: Telemetry shows gpt5_1 used (not gpt_pro), cost ~$0.30 (vs $0.35 baseline)
- **Pass Criteria**: gpt5_1 appears in agent list, consensus synthesis works, cost within ±10%
- **Evidence**: docs/integration-test-results/INT-1-gpt5-consensus.md
- **If Fails**: Check subagent_defaults.rs line 51, verify gpt5_1 in agents array

---

### Week 3 Integration Checkpoints

**INT-2: CLI Flag Filtering with GPT-5** (After SPEC-948 Phase 3):
- **When**: SPEC-948 Phase 3 complete, CLI flags implemented
- **Test**: Run `/speckit.auto SPEC-XXX --skip-validate --skip-audit`
- **Validate**: Only 6/8 stages execute, skip telemetry written, uses gpt5 models
- **Pass Criteria**: validate and audit stages skipped, evidence shows _SKIPPED.json files
- **Evidence**: docs/integration-test-results/INT-2-cli-filtering.md

**INT-3: TUI Configurator Loads Pipeline Config** (After SPEC-947 Phase 2):
- **When**: SPEC-947 Phase 2 complete, widget core functional
- **Test**: Create docs/SPEC-XXX/pipeline.toml (validate disabled), run /speckit.configure SPEC-XXX
- **Validate**: Modal displays with validate checkbox unchecked (loads TOML correctly)
- **Pass Criteria**: Modal reflects TOML state, cost displays correctly
- **Evidence**: Screenshot + test log

---

### Week 4 Integration Checkpoints

**INT-4: TUI Saves Configuration Round-Trip** (After SPEC-947 Phase 4):
- **When**: SPEC-947 Phase 4 complete, command integration done
- **Test**: /speckit.configure SPEC-XXX → toggle stages → press 'q' → reload → verify
- **Validate**: Saved TOML matches modal state, reload shows same checkboxes
- **Pass Criteria**: Round-trip preserves all state (enabled_stages, quality_gates)
- **Evidence**: pipeline.toml diff before/after

**INT-5: End-to-End Workflow** (All 3 SPECs complete):
- **When**: SPEC-947 complete
- **Test**: /speckit.auto SPEC-XXX --configure → configure in TUI → execute → measure
- **Validate**: Full workflow with GPT-5 models, partial pipeline, evidence captured
- **Pass Criteria**: Configure → execute → GPT-5 used → correct stages execute → telemetry correct
- **Evidence**: Full telemetry set, cost breakdown, execution log

**INT-6: Cost Reduction Validation** (SPEC-949 claim):
- **Test**: Run n≥3 test SPECs with /speckit.auto (full pipeline)
- **Measure**: Total cost per run
- **Validate**: Mean cost $2.30-$2.42 (target $2.36 ± 2.5%)
- **Pass Criteria**: Cost reduction ≥10% (acceptable), ≥13% (target), ≥15% (excellent)
- **Evidence**: docs/SPEC-949-extended-model-support/evidence/cost_validation.md

**INT-7: Performance Validation** (SPEC-949 claim):
- **Test**: Run n≥10 single-agent stages (specify, tasks) with timing
- **Measure**: Mean duration
- **Validate**: <2.5min (50% faster vs ~3-4min GPT-4 baseline)
- **Pass Criteria**: 1.5-2.5× speedup (acceptable 1.5×, target 2×, excellent 2.5×)
- **Evidence**: performance_metrics.md with mean±stddev

---

## SECTION 10: SESSION END PROTOCOL

### Before Ending Session (15 minutes)

**Completion Assessment**:

```markdown
## Session Completion Checklist

### Work Completed This Session:
- SPEC: [SPEC-949/948/947]
- Phases: [List phases completed]
- Tasks: [List tasks completed]
- Duration: [Actual hours worked]
- Commits: [Number of commits made]

### Validation Status:
- [ ] Latest commit compiles successfully
- [ ] All tests passing (workspace-wide)
- [ ] No uncommitted changes (clean working tree) OR
- [ ] Uncommitted changes documented (WIP note in SPEC.md)

### Progress Tracking Updated:
- [ ] Todo list reflects current state (mark completed phases done)
- [ ] SPEC.md updated (Status, Notes column with latest progress)
- [ ] Local-memory milestone stored (if significant phase completed, importance ≥8)

### Handoff to Next Session:
- [ ] Document current blocker (if any): [Description or "None"]
- [ ] Note next task to start: [SPEC-XXX Phase Y Task Z]
- [ ] Estimate time to complete next task: [Hours]
- [ ] Flag any risks encountered: [Description or "None"]
```

**Create Handoff Note** (If session ended mid-phase):

```markdown
## SESSION HANDOFF NOTE

**Last Completed**: SPEC-949 Phase 1 Task 1.1 (GPT-5 models registered)
**Current WIP**: SPEC-949 Phase 1 Task 1.2 (writing unit tests, 3/7 tests done)
**Blocker**: None
**Next Action**: Finish remaining 4 unit tests (estimated 1.5h), then run cargo test
**Branch**: spec-949-extended-model-support
**Working Tree**: Clean / Has uncommitted changes (3 test files)
**Risks**: None encountered
**Notes**: model_provider_info.rs compiles successfully, first 3 tests passing
```

---

### Session Output (What to Provide User)

**ALWAYS provide at session end**:

```markdown
## 📊 SESSION SUMMARY

**Duration**: [X hours]
**SPEC Progress**: SPEC-XXX Phase Y [% complete]
**Tasks Completed**: [N] tasks
**Tests Added**: [N] unit, [M] integration
**Commits**: [N] commits

### ✅ Completed This Session:
1. [Task 1 description] - [Validation result]
2. [Task 2 description] - [Validation result]
3. ...

### 🎯 Current Status:
- **SPEC-949**: [Not Started / Phase X Y% / Complete]
- **SPEC-948**: [Not Started / Phase X Y% / Complete]
- **SPEC-947**: [Not Started / Phase X Y% / Complete]
- **Integration Tests**: [N]/7 passing

### ⚠️ Issues Encountered:
- [Issue 1: description, resolution]
- [Issue 2: description, resolution]
- OR "None - smooth execution"

### 🚀 NEXT TASK TO WORK ON:

**Task**: SPEC-XXX Phase Y Task Z - [Title]
**File**: [Exact file path]
**Changes**: [What to implement]
**Estimated Duration**: [Hours]
**Validation**: [Command to run after completion]
**Why This Task**: [Rationale - why this is next in sequence]

**To Start**:
1. Read implementation plan section for this task
2. Verify dependencies complete (previous tasks done)
3. [Specific preparation step if needed]
4. Begin implementation

### 📈 Overall Progress:

**Timeline**: Week [X] of 4 ([Y%] complete)
**Estimated Remaining**: [Z hours] ([W weeks])
**On Track**: ✅ Yes / ⚠️ Slightly behind / 🔴 Significantly delayed

**Critical Path Status**: [On schedule / [X] hours slack / [Y] hours behind]
```

**This output is MANDATORY** - user needs clear next steps every session

---

## SECTION 11: RISK MONITORING & ESCALATION

### Risk Review Frequency

**Daily** (If implementing full-time):
- Review active risks (check if any triggered)
- Update risk status (🟡 Active / 🔴 Triggered)
- Execute mitigation if risk materializes

**Weekly** (At week boundaries):
- Review risk dashboard completely
- Assess risk probability changes (new information?)
- Update severity based on progress (some risks may resolve)

**At Phase Boundaries** (After each phase):
- Check phase-specific risks (did any materialize?)
- Add new risks discovered during implementation
- Mark risks resolved by phase completion

### Escalation Triggers (STOP Implementation and Notify)

**CRITICAL - Immediate Escalation**:
- 🔴 2+ High-severity risks triggered simultaneously
- 🔴 Critical path task blocked >8 hours (cannot resolve)
- 🔴 Rollback procedure fails (cannot restore working state)
- 🔴 Test pass rate drops below 95% (significant regression)
- 🔴 Security vulnerability discovered in code
- 🔴 Data loss or corruption in evidence/database

**HIGH - Escalate Within 24h**:
- 🟡 Single high-severity risk triggered
- 🟡 Critical path task blocked 4-8 hours
- 🟡 Test pass rate 95-99% (minor regression)
- 🟡 Cost reduction NOT achieved (>$2.71 per run)
- 🟡 Performance degradation (slower than GPT-4 baseline)

**MEDIUM - Document and Monitor**:
- 🟢 Medium-severity risk triggered
- 🟢 Non-critical task blocked
- 🟢 Effort estimate exceeded by >50%
- 🟢 Unexpected complexity discovered

**Escalation Protocol**:
1. **STOP** current work immediately
2. **Document** issue in SPEC.md Notes column
3. **Assess** impact (blocks phase? blocks SPEC? blocks all 3 SPECs?)
4. **Notify** user with issue description, impact, proposed resolution
5. **Wait** for user decision (proceed with workaround / rollback / defer)

---

## SECTION 12: DEVIATION HANDLING

### When to Deviate from Implementation Plan

**ALLOWED Deviations** (Document in commit message):

1. **Better Implementation Found**:
   - Example: Implementation plan suggests approach A, but existing codebase has proven pattern B
   - Criteria: Pattern B is simpler, tested, or more maintainable
   - Process: Document deviation in commit message, explain rationale
   - Validation: Ensure same success criteria met (different path, same outcome)

2. **Effort Estimate Significantly Off**:
   - Example: Task estimated 4h, actually taking 8h due to unforeseen complexity
   - Criteria: Complexity genuinely underestimated (not due to implementation mistakes)
   - Process: Update task estimate in notes, flag in local-memory if pattern (importance 8)
   - Continue: Complete task properly, don't cut corners to meet estimate

3. **Dependency Missing or Insufficient**:
   - Example: SPEC-947 Phase 1 verification finds SPEC-948 API incomplete
   - Criteria: Implementation cannot proceed without dependency
   - Process: Extend dependency SPEC (e.g., add method to pipeline_config.rs), document extension
   - Validation: Ensure extension doesn't break dependency SPEC's tests

4. **Integration Issue Discovered**:
   - Example: SPEC-948 config format incompatible with SPEC-947 TUI expectations
   - Criteria: Integration test fails, requires design adjustment
   - Process: Coordinate fix across both SPECs, maintain consistency
   - Validation: Re-run integration test until passing

**FORBIDDEN Deviations** (NEVER do these):

1. ❌ **Skip tests to save time** → Quality compromise, will cause bugs later
2. ❌ **Skip validation to meet estimate** → False progress, creates technical debt
3. ❌ **Placeholder code ("TODO: implement later")** → Breaks Definition of Done
4. ❌ **Suppress clippy warnings without fixing** → Code quality degradation
5. ❌ **Skip evidence capture** → Cannot validate claims later
6. ❌ **Deviate due to laziness or impatience** → Only deviate for legitimate technical reasons

**Deviation Documentation** (If deviation made):

```markdown
## DEVIATION NOTICE

**SPEC**: SPEC-XXX Phase Y Task Z
**Original Plan**: [What implementation plan specified]
**Actual Implementation**: [What was actually done]
**Rationale**: [Why deviation was necessary - must be technical reason]
**Impact**: [Does this affect other SPECs? Other tasks?]
**Validation**: [How success criteria still met despite deviation]
**Risks**: [Any new risks introduced by deviation]

**Approved By**: [Self if minor, user if major]
```

Store deviation in local-memory if significant (importance ≥8, tag: type:deviation)

---

## SECTION 13: MVP TESTING PREPARATION

**When to Execute**: After all 3 SPECs complete + 7 integration tests passing

### MVP Test Plan Structure

**Objective**: Validate end-to-end user workflows with real SPECs (not test SPECs)

**Test SPECs** (Create or use existing):
1. **Small SPEC** (~500 lines spec.md, simple feature) - Test rapid prototyping workflow
2. **Medium SPEC** (~1,500 lines spec.md, moderate complexity) - Test full pipeline with GPT-5
3. **Large SPEC** (~3,000 lines spec.md, complex feature) - Test modular workflow (skip stages)

**User Workflows to Validate**:

1. **Workflow 1: Full Pipeline with GPT-5** (SPEC-949 validation):
   - User: Developer wants full-quality output with cost savings
   - Steps:
     1. Create SPEC: /speckit.new "Add feature X"
     2. Run full pipeline: /speckit.auto SPEC-XXX
     3. Measure: Cost, duration, output quality
   - Expected: Cost $2.30-$2.42 (-13%), duration ~45-50min, all 8 stages execute
   - Validates: SPEC-949 cost reduction, GPT-5 models work correctly

2. **Workflow 2: Rapid Prototyping** (SPEC-948 validation):
   - User: Developer prototyping, wants quick iteration
   - Steps:
     1. Create SPEC: /speckit.new "Prototype feature Y"
     2. Configure: Copy rapid-prototyping.toml → docs/SPEC-XXX/pipeline.toml
     3. Execute: /speckit.auto SPEC-XXX
   - Expected: Cost ~$0.66 (76% savings), duration ~20min, 5/8 stages execute
   - Validates: SPEC-948 stage filtering, workflow examples work

3. **Workflow 3: Interactive Configuration** (SPEC-947 validation):
   - User: Developer wants visual control over pipeline
   - Steps:
     1. Create SPEC: /speckit.new "Add feature Z"
     2. Configure via TUI: /speckit.configure SPEC-XXX
     3. Toggle stages (disable validate, audit), observe cost update
     4. Save and execute: Press 'q' → pipeline runs
   - Expected: Modal works, cost updates in real-time, saved config persists
   - Validates: SPEC-947 TUI modal, keyboard navigation, config persistence

4. **Workflow 4: CLI Flag Override** (SPEC-948 + 949 integration):
   - User: Developer wants quick one-off override without TOML
   - Steps:
     1. Create SPEC: /speckit.new "Add feature W"
     2. Execute with flags: /speckit.auto SPEC-XXX --skip-validate --skip-audit
   - Expected: 6/8 stages execute, GPT-5 models used, skip telemetry written
   - Validates: CLI precedence over defaults, flag parsing works

5. **Workflow 5: Docs-Only Generation** (SPEC-948 validation):
   - User: Documentation update, no code needed
   - Steps:
     1. Create SPEC: /speckit.new "Update API documentation"
     2. Configure: Copy docs-only.toml → docs/SPEC-XXX/pipeline.toml
     3. Execute: /speckit.auto SPEC-XXX
   - Expected: Cost ~$1.20, duration ~15min, only specify/plan/unlock execute
   - Validates: Partial workflow, dependency warnings work correctly

**MVP Validation Criteria**:
- ✅ All 5 workflows execute successfully (no crashes, no errors)
- ✅ Cost measurements match expectations (within ±10%)
- ✅ Duration measurements match expectations (within ±20%)
- ✅ Output quality acceptable (subjective review)
- ✅ Evidence captured correctly (telemetry schema v1.0, all required fields)
- ✅ User experience smooth (no confusing errors, clear warnings)

**IF MVP validation fails**: Document failure, assess severity, fix or defer to future SPEC

---

## SECTION 14: SESSION STARTER PROMPT (Copy-Paste Ready)

**Copy the text below into a new Claude Code session to begin implementation**:

---

# IMPLEMENTATION SESSION: SPEC-947/948/949 Progressive Development

**Objective**: Implement SPEC-949 (Extended Model Support), SPEC-948 (Modular Pipeline Logic), and SPEC-947 (Pipeline UI Configurator) using progressive development with continuous validation.

**Quality Standards**:
- ✅ 100% test pass rate maintained at all times
- ✅ No compilation errors or clippy warnings
- ✅ Continuous validation (build + test after every task)
- ✅ Evidence captured for all claims (cost, performance, test results)
- ✅ No shortcuts or placeholder code (Definition of Done strictly enforced)

**Approach**: Follow implementation plans precisely unless better approach found (document deviations)

---

## 1. INITIALIZE SESSION

**Step 1a - Load Context**:

READ these files in order:
1. `docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md` (readiness analysis, blockers, execution plan)
2. `SPEC.md` lines 137-141 (current status of SPEC-949/948/947)
3. `docs/SPEC-949-extended-model-support/implementation-plan.md` (full implementation plan)
4. `docs/SPEC-948-modular-pipeline-logic/implementation-plan.md` (full implementation plan)
5. `docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md` (full implementation plan)

**Step 1b - Check Current State**:

```bash
# Git status
git status
git log --oneline --grep="spec-949\|spec-948\|spec-947" -10

# Baseline metrics
cargo test --list | wc -l  # Current test count (baseline: 555)
```

**Step 1c - Query Recent Implementation Work**:

USE: mcp__local-memory__search
```json
{
  "query": "SPEC-949 SPEC-948 SPEC-947 implementation milestone blocker",
  "search_type": "semantic",
  "use_ai": true,
  "limit": 10,
  "tags": ["type:milestone", "spec:SPEC-949", "spec:SPEC-948", "spec:SPEC-947"],
  "response_format": "concise"
}
```

**Step 1d - Assess Readiness**:

Answer these questions:
- Readiness report blockers resolved? (Check Section 3 of readiness report)
- Current SPEC in progress? (Check SPEC.md Status column)
- Any blockers from previous session? (Check local-memory results)

**OUTPUT**: Status report with clear starting point

---

## 2. DETERMINE STARTING POINT

Use decision tree from readiness report:

**IF blockers NOT resolved**:
→ Execute Section 3 of readiness report (resolve 4 blockers, 55min)
→ Commit documentation fixes
→ Store blocker resolution in local-memory
→ THEN start implementation

**IF SPEC-949 not started**:
→ CREATE feature branch: `git checkout -b spec-949-extended-model-support`
→ CREATE todo list with SPEC-949 phases 1-4
→ START: SPEC-949 Phase 1 Task 1.1
→ FOLLOW: Section 6 SPEC-949 execution guide (from readiness report)

**IF SPEC-949 in progress**:
→ RESUME from last completed task
→ VERIFY last commit's validation passed
→ CONTINUE with next task in sequence

**IF SPEC-949 complete, SPEC-948 not started**:
→ RUN integration test INT-1 (GPT-5 model validation)
→ CREATE feature branch: `git checkout -b spec-948-modular-pipeline-logic`
→ START: SPEC-948 Phase 1 Task 1.1 (**CRITICAL PATH**)

**IF SPEC-948 in progress**:
→ RESUME from last completed task
→ IF Phase 1 complete: VERIFY pipeline_config.rs exists (critical dependency)

**IF SPEC-948 complete, SPEC-947 not started**:
→ RUN integration test INT-2 (CLI filtering validation)
→ VERIFY SPEC-947 Phase 1 checklist (API requirements met)
→ CREATE feature branch: `git checkout -b spec-947-pipeline-ui-configurator`
→ START: SPEC-947 Phase 2 Task 2.1

**IF all 3 SPECs complete**:
→ START integration testing (INT-1 through INT-7)
→ CREATE evidence directory: docs/integration-test-results/
→ EXECUTE 7 integration checkpoints per Section 9

**IF integration testing complete**:
→ START MVP testing (Section 13)
→ CREATE MVP test plan
→ VALIDATE 5 user workflows

---

## 3. EXECUTION LOOP (Per Task)

For EACH task in implementation plan:

### A. Pre-Task Setup (5 minutes)

```markdown
## Starting Task: [SPEC-XXX Phase Y Task Z]

**Read**: Implementation plan section for this task (read completely, don't skim)

**Pre-Flight Checklist**:
- [ ] Task description understood (know exactly what to implement)
- [ ] File path verified (grep to confirm location if new file)
- [ ] Dependencies complete (previous tasks done, external deps exist)
- [ ] Code examples reviewed (understand before writing)
- [ ] Validation command noted (know how to test when done)
```

### B. Implementation (Task Duration)

```markdown
**Follow Implementation Plan**:
- Use code examples as templates (adapt to actual codebase)
- Maintain consistent style (match existing code formatting)
- Add inline comments for non-obvious logic
- Write tests AS YOU GO (not after all code written)

**Continuous Validation**:
- Compile frequently: `cargo build -p <package>` every 15-30 minutes
- Run tests incrementally: Test each function as you write it
- Fix errors immediately: Don't accumulate compilation errors

**Document Decisions**:
- If deviating from plan: Note rationale (will include in commit message)
- If encountering issues: Document problem and resolution
- If discovering risks: Add to risk dashboard
```

### C. Post-Task Validation (10-15 minutes)

```bash
# Validation Sequence (run ALL commands):

# 1. Format
cargo fmt --all

# 2. Build
cargo build -p <package>
# Must pass: 0 errors, 0 warnings

# 3. Clippy
cargo clippy -p <package> --all-targets
# Must pass: 0 warnings

# 4. Unit Tests (Modified Module)
cargo test -p <package> <module>::tests
# Must pass: 100% of new tests

# 5. Regression Tests (Workspace)
cargo test --workspace --no-fail-fast
# Must pass: All existing tests (no regressions)

# 6. Manual Validation (From Implementation Plan)
[Specific command from task's "Validation" section]
# Example: cargo test -p codex-core model_provider_info::tests::test_gpt5_models

# 7. Evidence Capture (If Applicable)
[Save test output, telemetry files, screenshots to evidence/]
```

**Validation Result**:
- ✅ ALL validations passed → Proceed to commit
- ⚠️ Some validations failed → Fix issues, re-run validation
- ❌ Critical validation failed → Consider rollback, reassess approach

### D. Commit Task (5 minutes)

```bash
git add [files modified in this task]

git commit -m "feat(spec-XXX): Phase Y Task Z - [Brief title]

[Detailed description of what was implemented]

Changes:
- File: [path] ([+X/-Y LOC])
- [Specific changes made]

Validation:
- Compilation: ✅ Success
- Tests: ✅ [N]/[N] passing (100%)
- Clippy: ✅ 0 warnings
- [Manual validation result]

[If deviation]: Deviation: [Explain why deviated from plan]

Ref: docs/SPEC-XXX-[slug]/implementation-plan.md Phase Y Task Z

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

### E. Update Progress (5 minutes)

```markdown
**Todo List**: Mark task complete (if last task in phase, mark phase complete)

**SPEC.md**: Update Notes column with progress (if phase complete)

**Local-Memory**: Store milestone if significant (importance ≥8):
- Phase complete with major deliverable (e.g., pipeline_config.rs created)
- Critical bug fixed with reusable pattern
- Important architectural decision made

**Risk Dashboard**: Update if risks affected (new/resolved/triggered)
```

### F. Determine Next Task

```markdown
**Decision**:

IF current phase has more tasks:
  → NEXT: Next task in phase sequence
  → CONTINUE: Repeat execution loop (Step 3A)

ELIF current phase complete:
  → CHECKPOINT: Run Phase Completion Checklist (Section 4)
  → IF checklist passes: Advance to next phase
  → IF checklist fails: Fix issues before advancing

ELIF current SPEC complete:
  → CHECKPOINT: Run SPEC Completion Checklist (Section 4)
  → RUN: SPEC-specific integration tests
  → CREATE: Pull request
  → NEXT: Start next SPEC in sequence

ELIF all SPECs complete:
  → START: Integration testing (Section 9)
```

---

## 4. PROVIDE NEXT TASK (End of Every Session)

**MANDATORY OUTPUT** at end of session:

```markdown
---

## 🚀 NEXT TASK TO WORK ON

**Task ID**: SPEC-XXX Phase Y Task Z
**Title**: [Task title from implementation plan]
**File**: [Exact file path]
**Estimated Duration**: [Hours]

**What to Implement**:
[2-3 sentence summary of what this task does]

**Changes Required**:
- [Bullet list of specific changes from implementation plan]

**Code Example** (from implementation plan):
```rust
// [Paste relevant code example from implementation plan]
```

**Validation Command**:
```bash
[Exact validation command from implementation plan]
```

**Success Criteria**:
- [List criteria from implementation plan]

**Dependencies**:
- [Previous tasks that must be complete: list or "None"]

**Risks**:
- [Relevant risks from risk dashboard: list or "None"]

**Why This Task**:
[1-2 sentences explaining why this is next - critical path? enables next phase? etc.]

---

**To Start This Task**:
1. Read implementation plan section: docs/SPEC-XXX-.../implementation-plan.md Phase Y
2. Verify file path: [Command to verify file exists or parent dir exists]
3. Review code example: Understand logic before implementing
4. Begin implementation: Follow task steps from implementation plan
5. Validate continuously: Build + test every 15-30 minutes

**Estimated Time to Complete**: [Hours]
**Expected Completion**: [If working full-time, date/time estimate]
```

**This output ensures continuity across sessions** - user always knows exactly what to do next

---

## SECTION 15: QUALITY ASSURANCE FRAMEWORK

### Test-Driven Development Approach

**ALWAYS write tests as you implement** (not after):

1. **Before implementing function**: Write test skeleton
   ```rust
   #[test]
   fn test_feature_x() {
       // Given: [Setup]
       // When: [Execute]
       // Then: [Assert]
       todo!("Implement test after function written")
   }
   ```

2. **Implement function**: Write actual code

3. **Complete test**: Fill in assertions
   ```rust
   #[test]
   fn test_feature_x() {
       // Given
       let input = ...;

       // When
       let result = feature_x(input);

       // Then
       assert_eq!(result, expected);
       assert!(result.is_ok());
   }
   ```

4. **Run test**: `cargo test test_feature_x`

5. **Fix until passing**: Iterate on implementation until test passes

**Benefits**: Catches issues early, ensures code is testable, prevents "forgot to write tests" problem

---

### Code Quality Standards

**MUST follow** (enforced by validation):

1. **No Warnings**: 0 clippy warnings, 0 compiler warnings
2. **No Dead Code**: Except provider stubs (marked with #[allow(dead_code)])
3. **No Unwraps**: Use Result<T, E> and ? operator, handle errors explicitly
4. **No Panics**: No panic!(), expect(), or assert!() in production code (tests OK)
5. **No TODOs**: Complete implementation or don't commit (TODOs indicate incomplete work)
6. **Consistent Formatting**: cargo fmt applied to all files
7. **Inline Documentation**: Public APIs have rustdoc comments
8. **Error Messages**: Descriptive error messages (not just "Error" or "Failed")

**IF quality standard violated**: Fix before committing (quality gate fails task)

---

### Regression Prevention

**After EVERY commit**:

```bash
# Full regression test suite
cargo test --workspace --no-fail-fast

# Check for degradation
cargo test --workspace --no-fail-fast 2>&1 | grep -i "FAILED\|error"

# If ANY tests fail:
# 1. Identify which test failed
# 2. Understand why (did your change break it?)
# 3. Fix your code OR fix the test (if test was wrong)
# 4. Re-run until 100% pass rate restored
```

**NEVER commit with failing tests** - Broken main branch is unacceptable

---

## SECTION 16: EXAMPLE TASK EXECUTION (SPEC-949 Phase 1 Task 1.1)

**Demonstration of complete task execution flow**:

### Task: Add 5 GPT-5 Models to model_provider_info.rs

**Pre-Task Setup**:
```markdown
## SPEC-949 Phase 1 Task 1.1

**Read**: Implementation plan lines 32-91 (Task 1.1 complete description)

**Pre-Flight**:
- [x] Task understood: Add 5 HashMap entries to model_provider_info.rs
- [x] File exists: `ls codex-rs/core/src/model_provider_info.rs` ✅
- [x] Dependencies: None (extends existing HashMap)
- [x] Code example reviewed: Lines 44-62 of implementation plan (example HashMap entry)
- [x] Validation: `cargo test -p codex-core model_provider_info`

**Estimated Duration**: 4 hours
**Current Time**: [Start time]
```

**Implementation**:
```markdown
**Step 1**: Read existing model_provider_info.rs to understand structure

READ: codex-rs/core/src/model_provider_info.rs

Find:
- Where HashMap is created (likely in default_model_provider_info() function)
- Existing GPT-4 entries (use as template)
- Line number to insert after (after existing GPT-4 models)

**Step 2**: Add 5 GPT-5 model entries

EDIT: codex-rs/core/src/model_provider_info.rs
```rust
// Insert after existing GPT-4 models (line ~200-250):

// GPT-5 Family (SPEC-949)
map.insert("gpt-5".to_string(), ModelProviderInfo {
    provider: "openai".to_string(),
    model_id: "gpt-5".to_string(),
    supports_responses_api: true,
    heartbeat_interval_ms: Some(30000),
    agent_total_timeout_ms: Some(1800000),
});

map.insert("gpt-5.1".to_string(), ModelProviderInfo {
    provider: "openai".to_string(),
    model_id: "gpt-5.1".to_string(),
    supports_responses_api: true,
    heartbeat_interval_ms: Some(30000),
    agent_total_timeout_ms: Some(1800000),
});

// [Continue for gpt-5-codex, gpt-5.1-codex, gpt-5.1-codex-mini]
```

**Step 3**: Build and verify

```bash
cargo build -p codex-core
# Expected: Success with 0 warnings
```

**Step 4**: Write 5-7 unit tests

EDIT: codex-rs/core/src/model_provider_info.rs (test module)
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_gpt5_model_lookup() {
        let info_map = default_model_provider_info();
        let gpt5 = info_map.get("gpt-5").expect("gpt-5 should exist");
        assert_eq!(gpt5.provider, "openai");
        assert_eq!(gpt5.model_id, "gpt-5");
        assert_eq!(gpt5.supports_responses_api, true);
        assert_eq!(gpt5.agent_total_timeout_ms, Some(1800000));
    }

    // [Continue for all 5 models + edge cases]
}
```
```

**Validation**:
```bash
# Run all validation commands
cargo fmt --all
cargo build -p codex-core  # ✅ Success
cargo clippy -p codex-core  # ✅ 0 warnings
cargo test -p codex-core model_provider_info::tests  # ✅ 7/7 passing
cargo test --workspace --no-fail-fast  # ✅ 555/555 passing (no regressions)

# Evidence
cargo test -p codex-core model_provider_info::tests 2>&1 | tee evidence/phase1-task1-tests.log
```

**Commit**:
```bash
git add codex-rs/core/src/model_provider_info.rs
git commit -m "feat(spec-949): add 5 GPT-5 models to model registry

Added GPT-5 family models to default_model_provider_info HashMap:
- gpt-5: Flagship reasoning model (30min timeout)
- gpt-5.1: Adaptive reasoning with extended caching
- gpt-5-codex: Agentic software engineering variant
- gpt-5.1-codex: Enhanced agentic with tool use
- gpt-5.1-codex-mini: Cost-optimized for high-volume

Changes:
- File: model_provider_info.rs (+60 LOC model entries, +84 LOC tests)
- Tests: 7 new unit tests (model lookup, timeout validation, provider check)

Validation:
- Compilation: ✅ Success (0 errors, 0 warnings)
- Tests: ✅ 7/7 new tests passing
- Clippy: ✅ 0 warnings
- Regression: ✅ 555/555 existing tests still passing

Ref: docs/SPEC-949-extended-model-support/implementation-plan.md Phase 1 Task 1.1

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

**Update Progress**:
- Todo: Keep Phase 1 as "in_progress" (Task 1.2 still pending)
- SPEC.md: Update notes "SPEC-949 Phase 1 Task 1.1 complete (model registration)"
- Local-Memory: Not yet (wait for phase complete)

**NEXT TASK**: SPEC-949 Phase 1 Task 1.2 (validation logic - but spec says "no changes needed")

---

## 5. END OF SESSION

**Before ending session, PROVIDE**:

```markdown
## 📊 SESSION SUMMARY

**Duration**: [X hours]
**SPEC**: SPEC-XXX Phase Y ([Z]% complete)
**Tasks Completed**: [N]
**Commits**: [N]
**Tests Added**: [N] (+[M]% of SPEC target)

### ✅ Completed:
- [Task 1]: [Validation result]
- [Task 2]: [Validation result]

### 🎯 Current Status:
- SPEC-949: [Status] ([X]/4 phases complete)
- SPEC-948: [Status] ([X]/4 phases complete)
- SPEC-947: [Status] ([X]/4 phases complete)
- Tests: [N]/[Target] passing (100% pass rate)

### ⚠️ Issues:
[List issues encountered OR "None"]

### 🚀 NEXT TASK:

**Task**: SPEC-XXX Phase Y Task Z - [Title]
**File**: [path]
**Duration**: [hours]
**Why**: [Rationale for why this is next]

**To Start**:
1. [Specific first step]
2. [Second step]
3. Begin implementation

### 📈 Progress:
**Timeline**: Week [X] of 4 ([Y%] complete)
**On Track**: ✅ / ⚠️ / 🔴
**Estimated Remaining**: [Z hours] ([W weeks])
```

---

## READY TO BEGIN?

Execute this prompt to start implementation session:

1. **Initialize** (Section 1): Load context, check state, assess readiness
2. **Determine Starting Point** (Section 2): Use decision tree to find where to begin
3. **Execute** (Section 3-5): Follow task-by-task execution loop
4. **Provide Next Task** (Section 5): Always end session with clear next steps

**Remember**:
- ✅ Quality over speed (no shortcuts)
- ✅ Continuous validation (test after every task)
- ✅ Evidence-based progress (capture proof of claims)
- ✅ Fail-fast with rollback (commit atomically, revert if needed)
- ✅ Document deviations (explain why if departing from plan)

**Estimated Total Duration**: 2.5-4 weeks (60-94 hours with buffer)

**Goal**: Close all 3 implementation SPECs, validate with integration tests, deliver production-ready MVP

**Quality Standard**: 100% test pass rate, 0 clippy warnings, comprehensive evidence, no compromises

---

**BEGIN IMPLEMENTATION NOW** ✅
