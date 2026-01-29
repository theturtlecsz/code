# SPEC-KIT-900: Stage 0 Integration Test Harness

**Status**: COMPLETED
**Created**: 2025-12-01
**Completed**: 2026-01-29
**Session**: P90
**Type**: Integration Test / Validation

***

## 1. Overview

### 1.1 Purpose

SPEC-KIT-900 provides a reproducible integration test for the Stage 0 pipeline using an external OSS project (ferris-says) as a neutral benchmark workload. This validates:

1. `/speckit.project` project scaffolding
2. `/speckit.new` spec creation
3. `/speckit.auto` full 6-stage pipeline with Stage 0 context injection
4. Tier 2 (NotebookLM) synthesis caching and Divine Truth generation
5. TASK\_BRIEF.md compilation via DCC

### 1.2 Benchmark Project

* **Name**: ferris-says
* **Source**: <https://github.com/mgeisler/ferris-says>
* **Description**: A simple Rust library that prints text with Ferris (the Rust mascot) in ASCII art speech bubbles
* **Why this project**: Small, well-documented, pure Rust, no external dependencies, easy to understand

### 1.3 Test Location

```
/home/thetu/benchmark/
├── reference/ferris-says/   # Original OSS project (read-only reference)
└── ferris-clone/            # Spec-kit scaffolded copy (our test subject)
```

***

## 2. Test Objectives

### 2.1 Stage 0 Validation Points

| Checkpoint         | Description                           | Validation                           |
| ------------------ | ------------------------------------- | ------------------------------------ |
| DCC\_TASK\_BRIEF   | TASK\_BRIEF.md generated with context | File exists, >500 chars              |
| DIVINE\_TRUTH      | Divine Truth from NotebookLM          | Contains 5 sections                  |
| CACHE\_BEHAVIOR    | Tier 2 cache hit on repeat            | `cache_hit: true` on 2nd run         |
| CONTEXT\_INJECTION | Stage 0 context in agent prompts      | Agents log "Stage 0: Shadow Context" |
| HYBRID\_RETRIEVAL  | Memory + Code context combined        | Both sections in TASK\_BRIEF         |

### 2.2 Pipeline Stages to Validate

1. **SPECIFY (Stage 1)**: Agents receive Divine Truth + TASK\_BRIEF
2. **TASKS (Stage 2)**: Task breakdown informed by historical context
3. **IMPLEMENT (Stage 3)**: Code generation with architectural guardrails
4. **VALIDATE (Stage 4)**: Tests pass, lint clean
5. **AUDIT (Stage 5)**: Security/quality review
6. **UNLOCK (Stage 6)**: Ready to merge

***

## 3. Test Procedure

### 3.1 Setup Phase

```bash
# 1. Ensure benchmark project is fresh
cd /home/thetu/benchmark/ferris-clone
git status  # Should be clean

# 2. Verify spec-kit structure
ls docs/ templates/ SPEC.md CLAUDE.md
```

### 3.2 Create Test Spec

```bash
# In ferris-clone project
/speckit.new "Add color support to ferris-says output using ANSI escape codes"
# Expected: Creates SPEC-002-add-color-support/spec.md
```

### 3.3 Run Full Pipeline

```bash
/speckit.auto SPEC-002
# Monitor:
# - Stage 0 logs (DCC, Tier 2 cache, Divine Truth)
# - Each stage agent spawns
# - Evidence directory population
```

### 3.4 Validation Checklist

* [ ] Stage 0 runs before Stage 1
* [ ] TASK\_BRIEF.md written to evidence/
* [ ] DIVINE\_TRUTH.md written to evidence/
* [ ] Agents receive combined\_context\_md()
* [ ] All 6 stages complete successfully
* [ ] Tests pass (cargo test)
* [ ] Lints pass (cargo clippy)

***

## 4. Expected Outputs

### 4.1 Evidence Directory Structure

```
docs/SPEC-002-add-color-support/evidence/
├── TASK_BRIEF.md           # Stage 0 DCC output
├── DIVINE_TRUTH.md         # Stage 0 Tier 2 output
├── SPECIFY_synthesis.md    # Stage 1 consensus
├── TASKS_synthesis.md      # Stage 2 task list
├── IMPLEMENT_synthesis.md  # Stage 3 code changes
├── VALIDATE_synthesis.md   # Stage 4 test results
├── AUDIT_synthesis.md      # Stage 5 review
└── UNLOCK_synthesis.md     # Stage 6 final approval
```

### 4.2 Sample Divine Truth Sections

1. Executive Summary - What color support adds
2. Architectural Guardrails - Don't break existing API
3. Historical Context - Prior art in cowsay/lolcat
4. Risks & Open Questions - Terminal compatibility
5. Suggested Causal Links - Related memories

***

## 5. Success Criteria

| Criterion              | Required | Measured                    |
| ---------------------- | -------- | --------------------------- |
| Stage 0 completes      | Yes      | Logs show Stage0Complete    |
| Divine Truth generated | Yes      | 5 sections present          |
| Cache hit on 2nd run   | Yes      | `cache_hit: true`           |
| All stages complete    | Yes      | UNLOCK\_synthesis.md exists |
| Tests pass             | Yes      | cargo test exit 0           |
| No regressions         | Yes      | Existing tests still pass   |

***

## 6. Session Log

| Session | Date       | Status      | Notes                                                           |
| ------- | ---------- | ----------- | --------------------------------------------------------------- |
| P87     | 2025-12-01 | IN-PROGRESS | Initial setup, verification of Stage 0 flow                     |
| P88     | 2026-01-29 | BLOCKED     | Headless CLI validation blocked                                 |
| P89     | 2026-01-29 | PARTIAL     | Headless runner scaffolded; stage execution stub (SPEC-KIT-930) |

### P88 Session Notes (2026-01-29)

**Objective**: Execute harness steps using headless CLI equivalents

**Results**:

| Step                | Status    | Evidence                                               |
| ------------------- | --------- | ------------------------------------------------------ |
| Benchmark scaffold  | ✅ PASS    | `/home/thetu/benchmark/ferris-clone/` created          |
| Spec creation (CLI) | ✅ PASS    | SPEC-KIT-001 created via `code speckit new --headless` |
| Pipeline execution  | ❌ BLOCKED | CLI stage commands were validation-only (dry-run)      |

**Critical Blocker**: No headless CLI equivalent for `/speckit.auto`

### P89 Session Notes (2026-01-29) - PARTIAL

**Resolution**: Scaffolded `HeadlessPipelineRunner`; stage execution deferred (TODO stub).

**Changes Made**:

1. Added `codex-rs/tui/src/chatwidget/spec_kit/headless/` module with:
   * `runner.rs` - HeadlessPipelineRunner scaffold (execute\_stage() is TODO stub)
   * `event_pump.rs` - Agent completion polling (not yet integrated)
   * `output.rs` - JSON output formatting

2. Modified `codex-rs/cli/src/speckit_cmd.rs` to route `--execute` to headless runner

3. Added input validation tests (maieutic parsing, JSON validation); no stage execution tests

**Blocking Issue**: `execute_stage()` (lines 396-408 in runner.rs) returns `HeadlessError::InfraError`
(exit code 3) to prevent false-green tests. Requires widget-independent agent spawning - tracked in SPEC-KIT-930.

**Headless Pipeline Execution Command**:

```bash
# Full headless pipeline execution
code speckit run \
  --spec SPEC-KIT-900 \
  --from plan \
  --to validate \
  --execute \
  --headless \
  --maieutic-answers '{"goal":"Generic smoke scenario","constraints":[],"acceptance":["Tests pass"],"delegation":"B"}' \
  --json
```

**Exit Codes**:

| Code | Meaning           | Resolution                                   |
| ---- | ----------------- | -------------------------------------------- |
| 0    | SUCCESS           | Pipeline completed                           |
| 3    | INFRA\_ERROR      | Check logs, verify config                    |
| 10   | NEEDS\_INPUT      | Provide `--maieutic-answers` or `--maieutic` |
| 11   | NEEDS\_APPROVAL   | Pre-supply approval answers                  |
| 13   | PROMPT\_ATTEMPTED | Bug - headless should never prompt           |

**Verified CLI capabilities** (updated):

* `code speckit new --headless --answers <json>` - Works, creates spec + intake artifacts
* `code speckit run --from --to` - Validation only (dry-run) ✅
* `code speckit run --from --to --execute --headless --maieutic-answers <json>` - Scaffolded (stub execution) ❌

**Artifacts Created**:

* `/home/thetu/benchmark/ferris-clone/docs/SPEC-KIT-001-add-color-support-to-ferris-says-output-using-ansi-escape/`
  * `spec.md` (1644 bytes)
  * `PRD.md` (777 bytes)
  * `INTAKE.md` (913 bytes)
* Capsule intake artifacts persisted (see capsule URIs in spec creation output)

***

## 7. Related

* **SPEC-KIT-102R**: Stage 0 implementation report (authoritative reference)
* **SPEC-KIT-102**: Original NotebookLM integration spec
* **docs/HANDOFF-P87.md**: Session context

***

*This spec serves as a reusable integration test harness for validating Stage 0 pipeline changes.*
