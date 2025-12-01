# SPEC-KIT-900: Stage 0 Integration Test Harness

**Status**: IN-PROGRESS
**Created**: 2025-12-01
**Session**: P87
**Type**: Integration Test / Validation

---

## 1. Overview

### 1.1 Purpose

SPEC-KIT-900 provides a reproducible integration test for the Stage 0 pipeline using an external OSS project (ferris-says) as a neutral benchmark workload. This validates:

1. `/speckit.project` project scaffolding
2. `/speckit.new` spec creation
3. `/speckit.auto` full 6-stage pipeline with Stage 0 context injection
4. Tier 2 (NotebookLM) synthesis caching and Divine Truth generation
5. TASK_BRIEF.md compilation via DCC

### 1.2 Benchmark Project

- **Name**: ferris-says
- **Source**: https://github.com/mgeisler/ferris-says
- **Description**: A simple Rust library that prints text with Ferris (the Rust mascot) in ASCII art speech bubbles
- **Why this project**: Small, well-documented, pure Rust, no external dependencies, easy to understand

### 1.3 Test Location

```
/home/thetu/benchmark/
├── reference/ferris-says/   # Original OSS project (read-only reference)
└── ferris-clone/            # Spec-kit scaffolded copy (our test subject)
```

---

## 2. Test Objectives

### 2.1 Stage 0 Validation Points

| Checkpoint | Description | Validation |
|------------|-------------|------------|
| DCC_TASK_BRIEF | TASK_BRIEF.md generated with context | File exists, >500 chars |
| DIVINE_TRUTH | Divine Truth from NotebookLM | Contains 5 sections |
| CACHE_BEHAVIOR | Tier 2 cache hit on repeat | `cache_hit: true` on 2nd run |
| CONTEXT_INJECTION | Stage 0 context in agent prompts | Agents log "Stage 0: Shadow Context" |
| HYBRID_RETRIEVAL | Memory + Code context combined | Both sections in TASK_BRIEF |

### 2.2 Pipeline Stages to Validate

1. **SPECIFY (Stage 1)**: Agents receive Divine Truth + TASK_BRIEF
2. **TASKS (Stage 2)**: Task breakdown informed by historical context
3. **IMPLEMENT (Stage 3)**: Code generation with architectural guardrails
4. **VALIDATE (Stage 4)**: Tests pass, lint clean
5. **AUDIT (Stage 5)**: Security/quality review
6. **UNLOCK (Stage 6)**: Ready to merge

---

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

- [ ] Stage 0 runs before Stage 1
- [ ] TASK_BRIEF.md written to evidence/
- [ ] DIVINE_TRUTH.md written to evidence/
- [ ] Agents receive combined_context_md()
- [ ] All 6 stages complete successfully
- [ ] Tests pass (cargo test)
- [ ] Lints pass (cargo clippy)

---

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

---

## 5. Success Criteria

| Criterion | Required | Measured |
|-----------|----------|----------|
| Stage 0 completes | Yes | Logs show Stage0Complete |
| Divine Truth generated | Yes | 5 sections present |
| Cache hit on 2nd run | Yes | `cache_hit: true` |
| All stages complete | Yes | UNLOCK_synthesis.md exists |
| Tests pass | Yes | cargo test exit 0 |
| No regressions | Yes | Existing tests still pass |

---

## 6. Session Log

| Session | Date | Status | Notes |
|---------|------|--------|-------|
| P87 | 2025-12-01 | IN-PROGRESS | Initial setup, verification of Stage 0 flow |

---

## 7. Related

- **SPEC-KIT-102R**: Stage 0 implementation report (authoritative reference)
- **SPEC-KIT-102**: Original NotebookLM integration spec
- **docs/HANDOFF-P87.md**: Session context

---

*This spec serves as a reusable integration test harness for validating Stage 0 pipeline changes.*
