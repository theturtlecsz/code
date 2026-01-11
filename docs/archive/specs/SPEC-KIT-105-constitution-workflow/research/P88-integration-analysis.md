# P88 Integration Analysis: Constitution Workflow with Stage 0 + NotebookLM

**Session**: P88
**Date**: 2025-12-01
**Status**: RESEARCH COMPLETE
**Authors**: Claude (analysis), User (refinements)

---

## Executive Summary

This document captures the analysis and design decisions for integrating constitution/vision workflows into the existing Stage 0 + NotebookLM stack. The key insight is that constitution should be treated as **Tier-0 context** that feeds into Stage 0, not a bolt-on feature.

The integration is architecturally clean because Stage 0 already has the necessary hooks:
- Domain filtering in IQO
- Priority boosting in overlay_db
- Section insertion in TASK_BRIEF
- Cache invalidation on memory change
- Seeder pattern for NL artifacts

---

## 1. Problem Statement

### 1.1 Current Gap

When `/speckit.project` runs, it creates `memory/constitution.md` with placeholder content:

```markdown
## Mission
[Define the project's core purpose]

## Principles
1. [Core principle 1]
...
```

These placeholders are never filled. When Stage 0 runs for `/speckit.auto`, there's no meaningful constitution content to inject into TASK_BRIEF or seed into NotebookLM.

### 1.2 GitHub Spec-Kit Reference

GitHub's spec-kit uses a **constitution-first** approach:
```
constitution.md (9 Articles) → /specify → /plan → /tasks → /implement
```

Our current flow skips the constitution:
```
/speckit.project (placeholder) → /speckit.new → /speckit.auto
```

---

## 2. Claims

1. **DCC already supports domain-filtered, priority-weighted retrieval** - Adding `domain:constitution` with elevated `initial_priority` requires minimal code changes.

2. **TASK_BRIEF has a clear insertion point** - Section 0 (Constitution) can be added before Section 1 (Spec Snapshot) in `assemble_task_brief()` at `dcc.rs:702`.

3. **Overlay_db already tracks `initial_priority`** - Constitution memories can be boosted by storing them with priority 9-10.

4. **NotebookLM seeder pattern is established** - Extending it to generate `NL_CONSTITUTION.md` and `NL_VISION.md` follows existing artifact generation model.

5. **Phase -1 Gates are a gating check, not a pipeline change** - They can be implemented as a precondition in TUI layer without modifying Stage 0 engine.

---

## 3. Evidence

### 3.1 IQO Domain Filtering

From `dcc.rs:31-46`:
```rust
pub struct Iqo {
    /// Knowledge domains to filter (e.g., ["spec-kit", "infrastructure"])
    pub domains: Vec<String>,
    /// Tags that MUST be present (e.g., ["spec:SPEC-KIT-102"])
    pub required_tags: Vec<String>,
    // ...
}
```

The data model natively supports domain filtering. Adding `domain:constitution` is straightforward.

### 3.2 TASK_BRIEF Section Assembly

From `dcc.rs:719-846`, the assembly is linear:
```rust
// Section 1: Spec Snapshot
out.push_str("## 1. Spec Snapshot\n\n");
// ...
// Section 2: Relevant Context (Memories)
out.push_str("\n## 2. Relevant Context (Memories)\n\n");
```

Inserting `## 0. Project Constitution (Summary)` is mechanically easy.

### 3.3 Overlay Priority Mechanism

From `overlay_db.rs:49-58`:
```rust
pub struct OverlayMemory {
    pub memory_id: String,
    pub initial_priority: i32,  // Constitution items can use 9-10
    pub usage_count: i32,
    // ...
}
```

The scoring formula in `scoring.rs` weights `initial_priority`.

### 3.4 Existing Seeder Artifacts

Current implementation generates:
- `NL_ARCHITECTURE_BIBLE.md`
- `NL_STACK_JUSTIFICATION.md`
- `NL_BUG_RETROS_01.md`
- `NL_DEBT_LANDSCAPE.md`
- `NL_PROJECT_DIARY_01.md`

Adding `NL_CONSTITUTION.md` and `NL_VISION.md` follows the same pattern.

---

## 4. Counter-Checks (Stress Tests)

### 4.1 What if a project has no constitution?

**Risk**: Stage 0 might error or produce malformed TASK_BRIEF.

**Mitigations**:
- Make Section 0 conditional (omit if no constitution memories exist)
- Phase -1 Gate warns but doesn't hard-fail (configurable strictness)
- Default skeleton auto-generated on `/speckit.project`
- Log diagnostic: `stage0.constitution=missing`

### 4.2 What if constitution memories conflict with spec requirements?

**Risk**: NotebookLM Tier-2 might surface contradictions.

**Mitigations**:
- This is a **feature**, not a bug - surfacing conflicts early is the point
- Add "Constitution Alignment" subsection to Divine Truth
- Define `CONSTITUTION_CONFLICT_WARNING` error category
- Phase -1 "block" mode can escalate warnings to hard gates

### 4.3 What if constitution is too large for context?

**Risk**: Verbose constitution consumes too much TASK_BRIEF token budget.

**Mitigations**:
- Cap Section 0 to top-5 items (2-3 principles, 2-3 guardrails)
- Store detailed content in NotebookLM; TASK_BRIEF gets summaries + IDs
- Apply MMR diversity constrained to constitution memories

### 4.4 What if constitution changes mid-project?

**Risk**: Cached Tier-2 results become stale.

**Mitigations**:
- Constitution memories linked to cache via `cache_memory_dependencies`
- `invalidate_by_memory()` at `overlay_db.rs:642` handles this
- Any constitution update triggers cache invalidation

### 4.5 What if scoring formula suppresses old constitution items?

**Risk**: `age_penalty` and `recency_score` could push constitution out of top-k.

**Mitigations**:
- **Do not rely solely on `initial_priority`**
- Implement explicit rule: "Always include at least N constitution memories"
- Constitution retrieval as separate always-on pass, not just IQO filtering

---

## 5. Architectural Decisions

### 5.1 IQO Amendment: Union, Not Replace

**Decision**: Constitution domain should be added via union, not replace.

**Rationale**: Replacing `iqo.domains` would clobber LLM-generated domains.

**Implementation**:
```rust
// Preserve existing domains, ensure constitution is always considered
if !iqo.domains.iter().any(|d| d == "constitution") {
    iqo.domains.push("constitution".to_string());
}
```

### 5.2 Constitution as Separate Always-On Pass

**Decision**: Constitution retrieval should be a separate pass, not just IQO filtering.

**Rationale**: Guarantees constitution is never accidentally filtered out by IQO heuristics.

**Implementation**:
```rust
compile_context():
    1. Build IQO from spec → query memories (existing flow)
    2. SEPARATELY: Always fetch top-N domain:constitution memories
    3. Merge both into candidates
    4. Apply MMR, assemble TASK_BRIEF
```

### 5.3 Explicit "Always Include N" Rule

**Decision**: Section 0 MUST include at least 3 constitution memories regardless of scoring.

**Rationale**: Scoring formula includes age_penalty, recency_score, novelty_factor - old constitution items could be suppressed in a busy project.

**Implementation**: After MMR selection, ensure at least 3 constitution memories are present, backfilling from `domain:constitution` query if needed.

### 5.4 Gate Lives in TUI, Not Stage 0

**Decision**: Phase -1 gate belongs in `pipeline_coordinator.rs`, not Stage 0 engine.

**Rationale**: Stage 0 should remain pure (context compilation + Tier 2 synthesis). Gating is a workflow concern.

**Implementation**:
```rust
// pipeline_coordinator.rs - before calling run_stage0_for_spec()
fn check_phase_minus_1_gate(project: &Project, mode: GateMode) -> Result<()> {
    let constitution_count = count_constitution_memories(project)?;

    if constitution_count == 0 {
        match mode {
            GateMode::Skip => {},
            GateMode::Warn => tracing::warn!("No constitution defined"),
            GateMode::Block => return Err(SpecKitError::ConstitutionRequired),
        }
    }
    Ok(())
}
```

### 5.5 Dual Storage with File as Source of Truth

**Decision**: Constitution stored in both `memory/constitution.md` (file) and local-memory (MCP).

**Rationale**:
- File is human-readable, version-controlled, portable
- Local-memory enables semantic search and Stage 0 integration

**Sync strategy**: File is source of truth. On `/speckit.vision` or `/speckit.constitution`, write to file first, then sync TO local-memory.

---

## 6. Data Model

### 6.1 Constitution Memory Schema

| Field | Value |
|-------|-------|
| Domain | `constitution` |
| Types | `principle`, `guardrail`, `goal`, `non-goal` |
| Importance | 8-10 (forced high) |
| Template | Guardian-structured per Template Guardian format |

### 6.2 Priority Schema

| Type | `initial_priority` | Rationale |
|------|-------------------|-----------|
| `type:guardrail` | 10 | Hard constraints, never violate |
| `type:principle` | 9 | Architectural values |
| `type:goal` | 8 | Mid-term objectives |
| `type:non-goal` | 8 | Explicit exclusions |

### 6.3 Constitution-Ready Definition

A project is "constitution-ready" when:
- At least 1 `type:guardrail` memory exists
- At least 1 `type:principle` memory exists
- `NL_CONSTITUTION.md` has been generated at least once

---

## 7. Integration Points

| Integration Point | Location | Change Required |
|-------------------|----------|-----------------|
| Constitution scaffold | `project_native.rs:504-552` | Add prompt about /speckit.vision |
| New commands | `commands/mod.rs` | Add VisionCommand, ConstitutionCommand |
| Local-memory storage | New | Store constitution as memories via MCP |
| File sync | New | Update `memory/constitution.md` from memories |
| Phase -1 Gate | `pipeline_coordinator.rs` | Check constitution before Stage 0 |
| TASK_BRIEF Section 0 | `dcc.rs:702` | Add constitution section |
| IQO domain include | `dcc.rs:266-273` | Push "constitution" to domains |
| Always-on constitution pass | `dcc.rs` (new) | Separate query for constitution memories |
| Tier-2 prompt | `tier2.rs` | Add constitution awareness clause |
| Divine Truth output | `tier2.rs` | Add Constitution Alignment section |
| NL artifact generation | Seeder | Add NL_CONSTITUTION.md, NL_VISION.md |
| Cache invalidation | `overlay_db.rs:642` | Already works via memory dependencies |

---

## 8. Design Questions Resolved

| Question | Decision | Rationale |
|----------|----------|-----------|
| Auto-include vs opt-in? | **Auto-include** | Constitution is the whole point |
| How many items in Section 0? | **Top-5** (2-3 principles, 2-3 guardrails) | Full list in NotebookLM |
| Default `phase1_gate_mode`? | **`warn`** | Backward compat; recommend `block` for production |
| Separate vision command or merged? | **Both** | Merged into project optionally, standalone for iteration |

---

## 9. Artifact Naming

| Artifact | Purpose |
|----------|---------|
| `NL_CONSTITUTION.md` | Formal principles, guardrails, constraints |
| `NL_VISION.md` | High-level product vision narrative |

These follow the existing pattern:
- `NL_ARCHITECTURE_BIBLE.md`
- `NL_STACK_JUSTIFICATION.md`
- `NL_BUG_RETROS_01.md`
- `NL_DEBT_LANDSCAPE.md`
- `NL_PROJECT_DIARY_01.md`

---

## 10. Workflow Integration

### Current Flow
```
/speckit.project rust myapp
    └── Creates placeholder constitution
    └── Says: "Next: /speckit.new"

/speckit.new "Add X"
    └── Creates SPEC-002

/speckit.auto SPEC-002
    └── Stage 0 runs (no constitution context)
```

### Proposed Flow
```
/speckit.project rust myapp
    └── Creates placeholder constitution
    └── Says: "Recommended: /speckit.vision, /speckit.constitution"
    └── Says: "Then: /speckit.new"

/speckit.vision (optional but encouraged)
    └── Q&A: mission, users, non-goals
    └── Stores domain:constitution memories
    └── Updates memory/constitution.md
    └── Generates NL_VISION.md

/speckit.constitution (optional but encouraged)
    └── Q&A: principles, guardrails, constraints
    └── Stores domain:constitution memories
    └── Updates memory/constitution.md
    └── Generates NL_CONSTITUTION.md

/speckit.new "Add X"
    └── Phase -1 Gate: check constitution
        - Missing → warn (or block if configured)
    └── Creates SPEC-002

/speckit.auto SPEC-002
    └── Stage 0 DCC includes:
        - Section 0: Project Constitution (real content)
        - domain:constitution memories in context
    └── Tier-2 prompt includes constitution awareness
    └── Divine Truth includes Constitution Alignment
```

---

## 11. Session Lineage

```
P72-P86: Stage 0 Implementation (SPEC-KIT-102)
    └── P87: Integration Testing + Gap Discovery
        ├── SPEC-KIT-900: E2E Test Harness
        ├── SPEC-KIT-105: Constitution Enhancement
        └── P88: Integration Analysis (this document)
            └── P89+: Implementation
```

---

## 12. References

- `codex-rs/stage0/src/dcc.rs` - DCC pipeline
- `codex-rs/stage0/src/overlay_db.rs` - Overlay database
- `codex-rs/stage0/src/tier2.rs` - Tier 2 synthesis
- `codex-rs/tui/src/chatwidget/spec_kit/project_native.rs` - Project scaffolding
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - Pipeline coordination
- GitHub spec-kit: https://github.com/github/spec-kit
- SPEC-KIT-102R: Stage 0 Implementation Report

---

*Analysis complete. Ready for implementation in SPEC-KIT-105.*
