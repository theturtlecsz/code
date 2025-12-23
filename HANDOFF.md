# Convergence Alignment — Session Handoff

**Last updated:** 2025-12-23
**Status:** Stage0 Convergence Phase 2 COMPLETE — System Pointer Memory Wired In

---

## Session Summary (2025-12-23)

### Completed This Session (Phase 2 - Integration)

| Task | Status | Notes |
|------|--------|-------|
| Wire `system_memory` into Stage0 | ✅ | Config option + background storage |
| Add `store_system_pointers` config | ✅ | Defaults to `true` |
| Create `store_stage0_system_pointer()` TUI helper | ✅ | Non-blocking, best-effort |
| Add `tier2_skip_reason` to Stage0ExecutionResult | ✅ | For diagnostics and pointer |
| Create convergence acceptance tests | ✅ | 6 tests in `convergence_acceptance.rs` |
| Surface Tier2 diagnostics in TUI output | ✅ | Shows skip reason inline |
| Fix clippy issues | ✅ | Format strings + collapsible if |

### Key Files Modified

| File | Changes |
|------|---------|
| `codex-rs/stage0/src/config.rs` | Added `store_system_pointers: bool` (default: true) |
| `codex-rs/stage0/src/system_memory.rs` | Added `Tier2Status`, `Stage0PointerInfo`, `store_stage0_pointer()` |
| `codex-rs/stage0/src/lib.rs` | Export new types |
| `codex-rs/stage0/tests/convergence_acceptance.rs` | NEW - 6 acceptance tests |
| `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs` | Added `tier2_skip_reason`, `store_stage0_system_pointer()` |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Call pointer storage, display diagnostics |
| `codex-rs/stage0/src/librarian/client.rs` | Clippy fix (collapsible if) |

---

## Phase 2 Architecture Summary

### System Pointer Memory Flow

```
User runs: code /speckit.auto SPEC-ID
    ↓
Stage0 Orchestration (Tier1 + Tier2)
    ↓
Artifacts written to disk (TASK_BRIEF.md, DIVINE_TRUTH.md)
    ↓
store_stage0_system_pointer() called (best-effort, non-blocking)
    └─ Spawns background thread
    └─ POST to local-memory REST API
    └─ Tags: system:true, spec:<id>, stage:0, type:milestone, tier2:<status>
    └─ Content: hashes, paths, summary bullets, commit SHA
```

### Pointer Memory Format

```yaml
domain: spec-tracker
importance: 8
tags:
  - system:true          # Excluded from normal retrieval
  - spec:<SPEC-ID>
  - stage:0
  - type:milestone
  - tier2:success|skipped|error
  - notebook:<id>        # Optional, if Tier2 used
content: |
  ## Stage0 Execution Pointer: SPEC-ID
  **Task Brief Hash**: abc123...
  **Divine Truth Hash**: def456...  # If Tier2 used
  **Task Brief Path**: /path/to/TASK_BRIEF.md
  **Commit**: 3387b0d
  **Tier2**: ✓ Success | ⊘ Skipped (reason) | ✗ Error

  ### Summary
  - Bullet 1 from divine truth
  - Bullet 2
```

### TUI Output

When Stage0 completes, users now see:
```
Stage 0: Context compiled (5 memories, tier2=yes, 250ms)
Stage 0: Context compiled (3 memories, tier2=skipped (No notebook configured), 150ms)
```

---

## Test Coverage

### Convergence Acceptance Tests

`codex-rs/stage0/tests/convergence_acceptance.rs`:

1. `test_tier1_excludes_system_memories` - system:true filtered out
2. `test_tier2_skipped_when_not_configured` - fail-closed with NoopClient
3. `test_tier2_runs_when_configured` - Tier2 called and content used
4. `test_tier2_failure_graceful` - Tier2 error → fallback divine truth
5. `test_store_system_pointers_default` - config defaults to true
6. `test_store_system_pointers_config_parsing` - TOML parsing works

### Run Tests

```bash
cargo test -p codex-stage0 -- convergence    # Convergence tests only
cargo test -p codex-stage0 -- system_memory  # System memory tests
cargo test -p codex-stage0                   # All stage0 tests
```

---

## Next Session: Refinements & Downstream Integration

### Priority 1: NotebookLM Doctor Command

**Repo:** `notebooklm-mcp`
**Goal:** Add `notebooklm doctor` CLI for deep auth readiness check

This is required before the full convergence path works end-to-end.
The current `check_tier2_service_health()` only checks HTTP health endpoint.

### Priority 2: Local-Memory Policy Skill

**Repo:** `localmemory-policy`
**Goal:** Ensure `lm recall` and `lm ask` exclude system:true by default

Currently the CLI adapter in codex-rs does client-side filtering, but the
upstream local-memory skill should also respect this.

### Priority 3: Pointer Recall

**Goal:** Add command to recall Stage0 pointers for a SPEC

```bash
code stage0 recall SPEC-KIT-102
# Shows stored pointer memories for this SPEC
```

---

## Next Session Start Prompt

Copy this into a new session:

```
load HANDOFF.md **ultrathink**

## Session Context (2025-12-23)

Previous session completed Stage0 Convergence Phase 2:
- ✅ `store_system_pointers` config option (default: true)
- ✅ `store_stage0_system_pointer()` helper with background storage
- ✅ `tier2_skip_reason` in Stage0ExecutionResult
- ✅ Convergence acceptance tests (6 tests)
- ✅ TUI Tier2 diagnostics surfaced
- ✅ Clippy fixes

## Downstream Work (Different Repos)

### notebooklm-mcp
- Add `notebooklm doctor` command
- Deep auth readiness check (not just health endpoint)
- Surface auth issues with actionable guidance

### localmemory-policy
- Update `lm recall` / `lm ask` to exclude system:true by default
- Add CONVERGENCE_MATRIX.yaml validation

## Optional: Pointer Recall Command

Add ability to query stored Stage0 pointers:
```bash
code stage0 recall SPEC-KIT-102
```

## Quick Verify
```bash
cargo test -p codex-stage0 -- convergence
code doctor  # Verify Stage0 health checks
```

## Reference Docs
- docs/convergence/MEMO_codex-rs.md — Required behaviors
- codex-rs/docs/convergence/README.md — Pointer to canonical docs
```

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `codex-rs/docs/convergence/README.md` | Pointer to canonical convergence docs |
| `codex-rs/stage0/src/config.rs` | Stage0Config with `store_system_pointers` |
| `codex-rs/stage0/src/system_memory.rs` | Pointer memory types and storage |
| `codex-rs/stage0/src/lib.rs` | Module exports |
| `codex-rs/stage0/tests/convergence_acceptance.rs` | Acceptance tests |
| `codex-rs/tui/src/stage0_adapters.rs` | LocalMemoryCliAdapter with filtering |
| `codex-rs/tui/src/chatwidget/spec_kit/stage0_integration.rs` | Tier2 health check, pointer storage |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | TUI output with diagnostics |
| `codex-rs/cli/src/main.rs` | `code doctor` Stage0 checks |

---

## Convergence Architecture Summary

### Golden Path

```
User runs: code /speckit.auto SPEC-ID
    ↓
Stage0 Orchestration (Tier1 + Tier2)
    ├─ Tier1 (DCC): local-memory search → TASK_BRIEF.md
    │   └─ Excludes system:true by default
    │
    └─ Tier2 (NotebookLM): enabled by default, fail-closed
        ├─ If ready + notebook mapped → call NotebookLM → DIVINE_TRUTH.md
        └─ If not ready → skip with diagnostic, continue Tier1 only
    ↓
System Pointer Memory (best-effort, non-blocking)
    └─ domain:spec-tracker, system:true, spec:<id>, stage:0
    ↓
6-Stage Pipeline (Plan → Tasks → Implement → Validate → Audit → Unlock)
```

### Fail-Closed Semantics

- Tier2 **enabled by default** but must have:
  - NotebookLM service reachable AND healthy
  - Notebook mapping configured
- If either missing → skip Tier2, continue Tier1, emit diagnostic
- **Never** fall back to "general notebook" or create notebooks silently

### Exclusion Compliance

- `system:true` tag excluded from Tier1 retrieval by default
- `normalize_iqo()` adds exclusion automatically
- Client-side filtering in `LocalMemoryCliAdapter`
- Override: `LM_INCLUDE_SYSTEM=1` (for debugging only)

---

## CI Status

- Quality Gates: ✅ Passing (stage0, tui build + test)
- Convergence Tests: ✅ 6/6 passing
- Note: codex-spec-kit has pre-existing clippy issues (not blocking)
