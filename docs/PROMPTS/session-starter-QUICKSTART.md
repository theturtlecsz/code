# QUICKSTART: Implementation Session (Copy-Paste This)

**Context**: Implementing SPEC-949 (Extended Model Support), SPEC-948 (Modular Pipeline Logic), SPEC-947 (Pipeline UI Configurator)
**Quality**: No compromises - 100% test pass rate, continuous validation, evidence-based progress
**Full Guide**: docs/PROMPTS/implementation-session-starter.md (reference for detailed workflows)

---

## SESSION INITIALIZATION

### 1. Load Context & Assess State

**Read in order**:
1. `docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md` (analysis results, execution plan)
2. `SPEC.md` lines 137-141 (current SPEC status)

**Check current state**:
```bash
git status
git log --oneline --grep="spec-949\|spec-948\|spec-947" -5
cargo test --list | wc -l  # Baseline: 555 tests
```

**Query previous work**:
```
USE: mcp__local-memory__search
query: "SPEC-949 SPEC-948 SPEC-947 implementation milestone blocker"
search_type: "semantic"
use_ai: true
limit: 10
tags: ["type:milestone", "spec:SPEC-949", "spec:SPEC-948", "spec:SPEC-947"]
```

**Answer**:
- Readiness blockers resolved? (Yes/No)
- Current SPEC in progress? (None/949/948/947)
- Current task? (SPEC-XXX Phase Y Task Z)
- Any blockers? (List or None)

---

### 2. Determine Starting Point

**Decision Tree**:

```
IF blockers NOT resolved:
  → Resolve 4 blockers first (55min, see readiness report Section 3)
  → Store in local-memory (importance 9)
  → Then start SPEC-949

ELIF no work started:
  → Branch: git checkout -b spec-949-extended-model-support
  → Read: docs/SPEC-949-extended-model-support/implementation-plan.md
  → Start: Phase 1 Task 1.1 (add GPT-5 models to model_provider_info.rs)

ELIF SPEC-949 in progress:
  → Resume from last completed task
  → Verify last validation passed

ELIF SPEC-949 done, SPEC-948 not started:
  → Integration: Run INT-1 (GPT-5 model validation)
  → Branch: git checkout -b spec-948-modular-pipeline-logic
  → Start: Phase 1 Task 1.1 (create PipelineConfig struct) **CRITICAL**

ELIF SPEC-948 in progress:
  → Resume from last completed task
  → If Phase 1 done: Verify pipeline_config.rs exists (unblocks SPEC-947)

ELIF SPEC-948 done, SPEC-947 not started:
  → Integration: Run INT-2 (CLI filtering)
  → Verify: SPEC-947 Phase 1 API checklist (6 items, see implementation plan)
  → Branch: git checkout -b spec-947-pipeline-ui-configurator
  → Start: Phase 2 Task 2.1 (create state machine)

ELIF all SPECs done:
  → Integration: Run INT-3 through INT-7 (cross-SPEC validation)
  → Then: MVP testing (5 user workflows)
```

---

## 3. EXECUTION LOOP (Per Task)

### A. Before Task

**Pre-Flight Checklist**:
- [ ] Read task section from implementation plan (understand completely)
- [ ] Verify file path (grep if uncertain)
- [ ] Check dependencies complete
- [ ] Note validation command (from plan)

### B. Implement

**Follow implementation plan**:
- Use code examples as templates
- Write tests AS YOU GO (not after)
- Compile frequently (every 15-30 min)
- Document deviations (if any)

### C. Validate (EVERY Task)

```bash
cargo fmt --all
cargo build -p <package>                    # 0 errors, 0 warnings
cargo clippy -p <package> --all-targets     # 0 warnings
cargo test -p <package> <module>::tests     # 100% pass
cargo test --workspace --no-fail-fast       # No regressions
[Manual validation command from plan]       # Verify behavior
```

**IF any validation fails**: Fix before committing (quality gate)

### D. Commit

```bash
git commit -m "feat(spec-XXX): Phase Y Task Z - [title]

[Description]

Changes: [files, LOC]
Validation: ✅ Build, ✅ Tests [N]/[N], ✅ Clippy

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

### E. Update Progress

- **Todo**: Mark task done, mark phase in_progress/completed
- **SPEC.md**: Update Notes (if phase complete)
- **Local-Memory**: Store milestone (if importance ≥8)

### F. Next Task

**Phase has more tasks**: Continue to next task (repeat 3A)
**Phase complete**: Run Phase Completion Checklist (10 criteria), then next phase
**SPEC complete**: Run SPEC Completion Checklist (10 criteria), then integration tests
**All done**: Run 7 integration tests, then MVP testing

---

## 4. MANDATORY SESSION OUTPUT

**At session end, ALWAYS provide**:

```markdown
---

## 🚀 NEXT TASK TO WORK ON

**Task**: SPEC-XXX Phase Y Task Z - [Title]
**File**: [Exact path]
**Duration**: [Hours]

**What to Do**:
[2-3 sentence summary]

**Changes**:
- [Specific changes from implementation plan]

**Validation**:
```bash
[Command from plan]
```

**Success Criteria**:
- [List from plan]

**Why This Task**:
[Explain why this is next - critical path? enables something?]

---

**To Start**:
1. Read: [Implementation plan section]
2. Verify: [File exists or dependencies met]
3. Implement: [Follow plan]
4. Validate: [Run commands above]

**Estimated**: [Hours] to complete
```

---

## 5. QUALITY PRINCIPLES (Non-Negotiable)

**Always**:
- ✅ Test after every task (not just phase end)
- ✅ 100% pass rate maintained (never commit with failing tests)
- ✅ Fix validation issues immediately (don't defer)
- ✅ Capture evidence (test logs, telemetry, metrics)
- ✅ Follow implementation plan (deviate only with good reason + documentation)

**Never**:
- ❌ Skip tests to save time
- ❌ Commit with warnings/errors
- ❌ Placeholder code (TODO, unimplemented!())
- ❌ Assume without validating
- ❌ Shortcut quality for speed

---

## 6. REFERENCE MATERIALS

**Implementation Plans** (Read completely before starting SPEC):
- SPEC-949: docs/SPEC-949-extended-model-support/implementation-plan.md
- SPEC-948: docs/SPEC-948-modular-pipeline-logic/implementation-plan.md
- SPEC-947: docs/SPEC-947-pipeline-ui-configurator/implementation-plan.md

**Readiness Report** (Execution schedule, risk dashboard):
- docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md

**Full Session Guide** (Detailed workflows, checklists):
- docs/PROMPTS/implementation-session-starter.md

**Project Standards**:
- CLAUDE.md (spec-kit commands, slash commands, git discipline)
- MEMORY-POLICY.md (local-memory usage, importance calibration)
- memory/constitution.md (project charter, guardrails)

---

## READY TO START

**Copy text below into new session**:

---

# BEGIN IMPLEMENTATION: SPEC-949/948/947

I'm ready to implement SPEC-949 (Extended Model Support), SPEC-948 (Modular Pipeline Logic), and SPEC-947 (Pipeline UI Configurator) using progressive development with continuous validation.

**Quality Standards**:
- 100% test pass rate maintained
- Build + test after every task
- No shortcuts or placeholder code
- Evidence captured for all claims

**Approach**: Follow implementation plans precisely, deviate only when better option exists (document why).

**Initialize session**:

1. Load context:
   - READ: docs/IMPLEMENTATION-READINESS-REPORT-2025-11-16.md
   - READ: SPEC.md lines 137-141
   - QUERY local-memory for recent SPEC-949/948/947 work

2. Check current state:
   - Git status, recent commits
   - Current test count baseline
   - Readiness blockers resolved?

3. Determine starting point:
   - Use decision tree (readiness report OR session-starter.md Section 2)
   - Identify next task to work on

4. Begin execution:
   - Create todo list for current SPEC
   - Start first/next task
   - Follow task execution loop (pre-flight → implement → validate → commit → update progress)

5. At session end:
   - Provide clear NEXT TASK summary
   - Update SPEC.md tracker
   - Store milestones in local-memory (importance ≥8)

**Reference**: docs/PROMPTS/implementation-session-starter.md (full guide with checklists, examples, workflows)

**Let's begin. What's the current state?**

---

**END OF QUICKSTART PROMPT** (Paste above into new session)
