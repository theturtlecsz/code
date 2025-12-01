# P88 Session Handoff: Constitution Workflow Enhancement & E2E Testing

**Previous Session**: P87
**Date**: 2025-12-01
**Status**: RESEARCH → IMPLEMENTATION

---

## 1. Session Context

P87 set out to test Stage 0 integration via `/speckit.auto` but identified a foundational gap: **no workflow exists to capture product vision and architectural principles before creating feature specs**.

### 1.1 What Was Accomplished (P87)

| Task | Status | Artifact |
|------|--------|----------|
| Stage 0 code verification | ✅ Done | Validated pipeline_coordinator.rs, dcc.rs, lib.rs |
| Test suites passing | ✅ Done | stage0: 127, tui: 507 |
| SPEC-KIT-900 created | ✅ Done | `docs/SPEC-KIT-900-stage0-integration-test/spec.md` |
| ferris-test benchmark setup | ✅ Done | `/home/thetu/benchmark/ferris-test/` |
| GitHub spec-kit research | ✅ Done | 9 Articles framework, Phase -1 Gates documented |
| Gap identified | ✅ Done | Missing `/speckit.vision` and `/speckit.constitution` |
| SPEC-KIT-105 created | ✅ Done | `docs/SPEC-KIT-105-constitution-workflow/spec.md` |

### 1.2 Key Finding

GitHub's spec-kit uses a **constitution-first** approach:
```
constitution.md (9 Articles) → /specify → /plan → /tasks → /implement
```

Our current flow skips the constitution:
```
/speckit.project (placeholder constitution) → /speckit.new → /speckit.auto
```

---

## 2. Primary Goal: Research & Design

### 2.1 Deep Dive GitHub Spec-Kit

Clone and analyze the repository:
```bash
cd /home/thetu/benchmark
git clone https://github.com/github/spec-kit.git github-spec-kit
```

Research tasks:
1. Document the 9 Articles framework in detail
2. Understand template embedding mechanism
3. Analyze Phase -1 Gates enforcement
4. Study their Q&A flow for `/specify`

### 2.2 Design Decisions Required

| Decision | Options | Notes |
|----------|---------|-------|
| Framework approach | Adopt 9 Articles / Adapt / Custom | Research first |
| Command structure | Separate commands / Combined / Flags | `/speckit.vision` + `/speckit.constitution` proposed |
| Constitution scope | Per-project / Global defaults | Consider both |
| Backward compat | Required / Optional | Should work without constitution |

---

## 3. Secondary Goal: E2E Test Completion

### 3.1 Benchmark Project Status

**Location**: `/home/thetu/benchmark/ferris-test/`

**Current State**:
- ✅ Source code copied from ferris-says
- ✅ `/speckit.project rust ferris-test` executed
- ⚠️ Cargo.toml reset to minimal (dependencies removed)
- ⚠️ No spec created yet

**Required Fixes**:
```bash
# Restore dependencies in Cargo.toml
[dependencies]
regex = "1.10.4"
smallvec = "1.11.2"
textwrap = "0.16.0"
unicode-width = "0.1.11"
```

### 3.2 E2E Test Sequence (After Research)

1. Populate constitution manually (interim solution)
2. `/speckit.new "Add ANSI color support for terminal output"`
3. `/speckit.auto SPEC-002`
4. Verify Stage 0 context injection
5. Document findings

---

## 4. Files to Reference

### 4.1 New SPECs Created

| SPEC | Path | Purpose |
|------|------|---------|
| SPEC-KIT-900 | `docs/SPEC-KIT-900-stage0-integration-test/spec.md` | E2E test harness |
| SPEC-KIT-105 | `docs/SPEC-KIT-105-constitution-workflow/spec.md` | Constitution enhancement |

### 4.2 Stage 0 Implementation (Verified P87)

| Component | Path | Key Functions |
|-----------|------|---------------|
| Pipeline coordinator | `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | `run_stage0_for_spec()` (line 158) |
| Stage 0 engine | `stage0/src/lib.rs` | `run_stage0()`, `combined_context_md()` |
| DCC | `stage0/src/dcc.rs` | `compile_context()` |
| Tier 2 | `stage0/src/tier2.rs` | `build_tier2_prompt()`, `parse_divine_truth()` |
| Agent injection | `tui/src/chatwidget/spec_kit/agent_orchestrator.rs` | Lines 1377-1393, 121-140 |

### 4.3 External Resources

- [GitHub Spec-Kit](https://github.com/github/spec-kit)
- [Spec-Driven Development Guide](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- [GitHub Blog](https://github.blog/ai-and-ml/generative-ai/spec-driven-development-with-ai-get-started-with-a-new-open-source-toolkit/)

---

## 5. Quick Start Commands

```bash
# Tests (from codex-rs/)
cargo test -p codex-stage0   # 127 tests
cargo test -p codex-tui      # 507 tests

# Benchmark project
cd /home/thetu/benchmark/ferris-test
cargo check  # Will fail until deps restored

# Clone GitHub spec-kit for research
cd /home/thetu/benchmark
git clone https://github.com/github/spec-kit.git github-spec-kit
```

---

## 6. Session Prompt for P88

Copy this prompt to start the next session:

```
P88 Session: Constitution Workflow Enhancement

Read docs/HANDOFF-P88.md for full context.

Primary Goal: Research GitHub spec-kit and design constitution workflow.

Key Tasks:
1. Clone github/spec-kit to /home/thetu/benchmark/github-spec-kit
2. Document 9 Articles framework in detail
3. Analyze template embedding and Phase -1 Gates
4. Design /speckit.vision and /speckit.constitution commands
5. Update SPEC-KIT-105 with design decisions
6. (If time) Complete E2E test with ferris-test benchmark

Quick Reference:
- New SPECs: SPEC-KIT-105 (constitution), SPEC-KIT-900 (E2E test)
- Benchmark: /home/thetu/benchmark/ferris-test/ (needs Cargo.toml deps)
- Tests: cargo test -p codex-stage0 (127), cargo test -p codex-tui (507)

Session Lineage: P72-P87 (Stage 0), P88+ (Constitution workflow)
```

---

## 7. Open Questions

1. Should constitution be interactive Q&A or template-based?
2. How strict should Phase -1 Gates be? (blocking vs. warning)
3. Should we support multiple constitution "profiles" (strict, minimal, custom)?
4. Integration with Stage 0 Divine Truth - how much constitution context?

---

## 8. Session Lineage

```
P72-P86: Stage 0 Implementation (SPEC-KIT-102)
    └── P87: Integration Testing + Gap Discovery
        ├── SPEC-KIT-900: E2E Test Harness
        ├── SPEC-KIT-105: Constitution Enhancement
        └── P88+: Constitution Implementation
```

---

*Handoff prepared by P87 session. NotebookLM authentication required at session start.*
