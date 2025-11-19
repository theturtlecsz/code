Progressive Implementation - Multi-SPEC Session Continuation Prompt

  Progressive Implementation - Session Continuation (SPEC-948 → SPEC-949 → SPEC-947 → Future)

  Resume progressive implementation workflow for modular pipeline logic, extended model support, TUI configurator, and remaining implementation backlog.

  ---
  STEP 1: Load Current State

  Execute in parallel:

  USE: mcp__local-memory__search
  {
    "query": "SPEC-948 SPEC-949 SPEC-947 phase task complete milestone",
    "search_type": "semantic",
    "use_ai": true,
    "limit": 15,
    "tags": ["type:milestone"],
    "response_format": "concise"
  }

  git status --short
  git log --oneline -10
  git branch --show-current
  cargo build -p codex-tui --lib 2>&1 | grep "Finished\|error" | tail -3

  ---
  STEP 2: Determine Current Position

  Review Evidence:
  1. Local-memory: Last completed task from search results
  2. Git log: Recent commits (look for SPEC-XXX Phase X Task Y.Z patterns)
  3. Implementation plans:
     - /home/thetu/code/docs/SPEC-948-modular-pipeline-logic/implementation-plan.md
     - /home/thetu/code/docs/SPEC-949-extended-model-support/implementation-plan.md
     - /home/thetu/code/docs/SPEC-947-tui-pipeline-configurator/implementation-plan.md

  Current Known State (as of 2025-11-17 23:30 UTC):
  - ✅ SPEC-949: Phase 0/4 (Not Started)
  - ✅ SPEC-948: Phase 4/4 In Progress (25% complete)
    - ✅ Task 4.1: Pipeline configuration guide COMPLETE
    - 🎯 Task 4.2: Create 4 workflow example configs (NEXT)
    - ⏸️ Task 4.3: Document workflow patterns in guide
    - ⏸️ Task 4.4: Update CLAUDE.md command reference
  - ⏸️ SPEC-947: Phase 0/3 (Blocked on SPEC-948 complete)

  Decision Tree:
  IF SPEC-948 Phase 4 incomplete:
    → NEXT INCOMPLETE TASK in Phase 4 (check implementation-plan.md)

  ELIF SPEC-948 Phase 4 complete AND SPEC-949 not started:
    → SPEC-949 Phase 1 Task 1.1 (First task from implementation plan)
    NOTE: SPEC-949 = Extended model support (GPT-5, Deepseek, Kimi)

  ELIF SPEC-949 complete AND SPEC-947 not started:
    → SPEC-947 Phase 1 Task 1.1 (First task from implementation plan)
    NOTE: SPEC-947 depends on SPEC-948 backend being complete

  ELIF SPEC-947 complete:
    → Check SPEC.md for next implementation SPEC (SPEC-936, SPEC-940, etc.)
    → Read next SPEC's implementation-plan.md
    → Start Phase 1 Task 1.1

  Identify:
  - Last Completed: [SPEC-ID Phase X Task Y.Z - Description]
  - Commit Hash: [hash]
  - Duration: ~[X]h
  - Tests Status: [build/test status]
  - Branch: [name]
  - Next Task: [SPEC-ID Phase X Task Y.Z - Title]

  ---
  STEP 3: Load Next Task Details

  Read Implementation Plan:
  # For current SPEC (determine from Step 2)
  cat /home/thetu/code/docs/SPEC-XXX-<slug>/implementation-plan.md | grep -A 150 "Phase [X]:"

  # If starting new SPEC, read full plan
  cat /home/thetu/code/docs/SPEC-XXX-<slug>/implementation-plan.md | head -300

  # Check SPEC.md for overall status
  grep -A 5 "SPEC-XXX" /home/thetu/code/SPEC.md

  ---
  STEP 4: Execute Task

  1. Create TODO List (TodoWrite)

  Subtasks should include:
  - Research/read existing files (2-3 items) - IF code task
  - Specific implementation steps (4-8 items)
  - Test additions/updates (1-2 items) - IF code task
  - Validation steps (build, test, clippy) - IF code task
  - Commit step

  2. Implementation Standards

  ✅ Required:
  - Read files before editing (Read tool first) - IF code task
  - Build after EVERY significant change - IF code task
  - Test after implementation complete - IF code task
  - 0 warnings for changed files (clippy) - IF code task
  - Commit after task completion

  ❌ Forbidden:
  - Placeholders (TODO, unimplemented!())
  - AI references in commits
  - Skipping validation steps

  3. Validation Loop

  # IF CODE TASK:
  # After each subtask
  cargo build -p codex-tui --lib

  # After implementation complete
  cargo test -p codex-tui --lib [test_module]

  # Before commit
  cargo clippy -p codex-tui --lib -- -D warnings 2>&1 | grep "[changed_file]"

  # IF DOCS TASK:
  # Peer review for completeness and accuracy
  # Cross-reference with implementation/other docs
  # Verify examples are accurate

  4. Commit Format

  [type](spec-XXX): Phase [X] Task [Y.Z] - [Title]

  Implementation (IF code task):
  - [Change 1]
  - [Change 2]
  - [Change 3]

  Documentation (IF docs task):
  - [Section 1 created]
  - [Section 2 created]
  - [Content details]

  Tests (IF code task with tests):
  - ✅ [test_name_1] ([what it validates])
  - ✅ [test_name_2] ([what it validates])

  Build Status (IF code task):
  - ✅ cargo build -p [package] --lib (clean)
  - ✅ cargo clippy (0 warnings for [file])
  - [✅/⚠️] cargo test ([status])

  Build Status (IF docs task):
  - N/A (documentation task, no code changes)

  Accuracy (IF docs task):
  - ✅ [Technical detail 1 verified against implementation]
  - ✅ [Technical detail 2 verified against implementation]

  Changes:
  - +[N] LOC / -[M] LOC (IF code) OR +[N] lines (IF docs)
  - Modified/Created: [file paths]

  Duration: ~[X]h

  Next: [Next task description]

  5. Known Constraints

  - ⚠️ Pre-existing test failures (ignore, unrelated to current work)
  - ⚠️ Integration tests may fail (focus on lib tests for code tasks)
  - ⚠️ Work in isolated crates when possible
  - ✅ Library builds must succeed (code tasks)
  - ✅ Clippy must pass for changed files (code tasks)
  - ✅ Documentation must be accurate and complete (docs tasks)

  ---
  STEP 5: Session Report & Memory

  A. Store Milestone to Local-Memory

  USE: mcp__local-memory__store_memory
  {
    "content": "SPEC-[XXX] Phase [X] Task [Y.Z] Complete (2025-MM-DD): [2-3 sentence summary]. Implementation: [key changes and files] OR Documentation: [sections created]. Tests: [X/X passing, details] OR N/A (docs). Commit: [hash]. Build: [status] OR N/A (docs). Duration: ~[X]h. Next: [next task description]. [Critical insights if
  any].",
    "domain": "spec-kit",
    "tags": ["type:milestone", "spec:SPEC-XXX", "phase-[X]"],
    "importance": 8 or 9
  }

  Importance Guidelines:
  - 9: Phase completion, critical path unblocking, major architectural decisions, comprehensive guides, major feature implementations
  - 8: Task completion with significant implementation (>50 LOC, complex logic, substantial docs)

  B. Provide Session Summary

  ---
  📊 SESSION SUMMARY

  SPEC: SPEC-[XXX] ([Description])
  Phase: Phase [X]/[Y] - [%] complete
  Tasks Completed: [N] task(s)
  Commits: [N] commit(s) with hashes
  Duration: ~[X] hours

  ---
  ✅ Completed This Session

  1. [Task ID]: [Description] - Validation: [✅/⚠️]
  - File: [path]
  - Changes: [+X/-Y LOC, specific changes] OR [+X lines, sections]
  - Tests: [X added, status] OR Documentation: [sections] OR N/A
  - Build: [Clean / warnings status] OR N/A (docs)

  (Repeat for each task completed)

  ---
  📈 Overall Progress

  - SPEC-949: Phase [X]/4 [Status] [🔄/✅] ([%])
    - [Task status list with ✅/⏸️]
  - SPEC-948: Phase 4/4 [Status] [🔄/✅] ([%])
    - [Task status list with ✅/⏸️]
  - SPEC-947: Phase [X]/3 [Status] ⏸️
    - [Task status list with ✅/⏸️]
  - Tests: [N total] ([+X this session], [%] pass rate) OR N/A (docs only session)
  - Branch: [name]
  - Commits: [N total] ([+X this session])

  ---
  ⚠️ Blockers/Issues

  [Description] OR "None"

  ---
  💾 Local-Memory Storage

  ✅ Stored (ID: [uuid])
  [Summary of what was stored]

  ---
  🚀 NEXT TASK TO WORK ON

  Task: SPEC-[XXX] Phase [X] Task [Y.Z] - [Title]
  File: [exact/path or NEW]
  Type: [Code/Documentation]
  What: [2-3 sentences describing the task]

  Changes (IF code):
  - [Specific change 1]
  - [Specific change 2]
  - [Specific change 3]

  Content (IF docs):
  - [Section 1 to write]
  - [Section 2 to write]
  - [Key topics to cover]

  Duration: [X-Y hours]

  Validation:
  IF CODE:
    cargo build -p [package]
    cargo test -p [package] --lib [test_name]
    cargo clippy -p [package] --lib -- -D warnings

  IF DOCS:
    - Peer review for completeness and clarity
    - Cross-reference with implementation/other docs
    - Verify examples/technical details are accurate

  Success Criteria:
  - ✅ [Criterion 1]
  - ✅ [Criterion 2]
  - ✅ [Criterion 3]

  Dependencies: [What must be done first] OR "None"

  Why This Task Next: [Critical path / Sequential / Blocker resolution / etc.]

  ---
  📚 Quick Reference

  - SPEC-948 Plan: /home/thetu/code/docs/SPEC-948-modular-pipeline-logic/implementation-plan.md
  - SPEC-949 Plan: /home/thetu/code/docs/SPEC-949-extended-model-support/implementation-plan.md
  - SPEC-947 Plan: /home/thetu/code/docs/SPEC-947-tui-pipeline-configurator/implementation-plan.md
  - Integration Analysis: /home/thetu/code/docs/SPEC-947-948-949-INTEGRATION-ANALYSIS.md
  - Project Context: /home/thetu/code/CLAUDE.md
  - Memory Policy: /home/thetu/code/MEMORY-POLICY.md
  - Active SPECs: /home/thetu/code/SPEC.md

  ---
  BEGIN IMPLEMENTATION - Execute Steps 1-5, provide complete session report with NEXT TASK clearly identified at end.

  ---
  📝 Usage Notes

  What This Prompt Does:
  1. ✅ Auto-detects current state from local-memory + git across ALL implementation SPECs
  2. ✅ Determines next task using decision tree (SPEC-948 → SPEC-949 → SPEC-947 → future)
  3. ✅ Executes task with standards enforcement (code vs docs differentiation)
  4. ✅ Stores milestone to local-memory with proper importance
  5. ✅ Provides comprehensive session report with NEXT TASK clearly identified

  When to Use:
  - Starting any new session for implementation SPECs
  - After breaks/interruptions
  - When resuming progressive implementation workflow

  What You Get:
  - Current state analysis across all active implementation SPECs
  - Task execution with validation (code: build/test/clippy; docs: accuracy verification)
  - Commit with proper formatting
  - Session report with overall progress tracking
  - Local-memory continuity
  - Next task clearly identified and ready to start

  Customization:
  - Update "Current Known State" section as you progress through SPECs
  - Adjust decision tree when new SPECs are added to the implementation backlog
  - Modify validation commands per task type (code vs docs)
  - Add new SPECs to the decision tree and Quick Reference section

  ---
  🎯 Copy-Paste Ready Version

  Progressive Implementation - Session Continuation (SPEC-948 → SPEC-949 → SPEC-947 → Future)

  Resume progressive implementation workflow. Execute Steps 1-5:

  STEP 1: Load state (parallel):
  - mcp__local-memory__search: query="SPEC-948 SPEC-949 SPEC-947 phase task complete", tags=["type:milestone"], limit=15
  - git status --short
  - git log --oneline -10
  - cargo build -p codex-tui --lib

  STEP 2: Determine position from evidence + decision tree:
  Current: SPEC-948 Phase 4 Task 4.1 DONE → NEXT: Phase 4 Task 4.2 (4 workflow examples)
  OR: SPEC-948 Phase 4 DONE → NEXT: SPEC-949 Phase 1 Task 1.1
  OR: Check implementation-plan.md for current phase/task

  STEP 3: Load task details from implementation plan

  STEP 4: Execute with TODO list, validation (code: build/test/clippy; docs: accuracy check), proper commit format

  STEP 5: Store milestone + provide session report with NEXT TASK

  Standards: Read before edit (code), build after changes (code), 0 warnings (code), accuracy verification (docs), proper commits, no placeholders

  BEGIN IMPLEMENTATION

  This prompt will:
  - ✅ Auto-detect progress across SPEC-948, SPEC-949, SPEC-947, and future SPECs
  - ✅ Load next task details from implementation plans
  - ✅ Execute with proper validation (code vs docs)
  - ✅ Store milestones with correct importance
  - ✅ Always end with clear NEXT TASK identification
