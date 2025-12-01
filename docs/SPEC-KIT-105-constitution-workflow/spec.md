# SPEC-KIT-105: Constitution & Vision Workflow Enhancement

**Status**: DESIGN COMPLETE
**Created**: 2025-12-01
**Updated**: 2025-12-01 (P88)
**Session**: P87 (created), P88 (design)
**Priority**: P1 (Should Have)
**Depends On**: SPEC-KIT-102 (Stage 0), SPEC-KIT-900 (E2E Test Harness)

---

## 1. Problem Statement

### 1.1 Current Gap

When `/speckit.project` runs, it creates `memory/constitution.md` with placeholder content that is never filled. Stage 0 runs without meaningful constitution context.

### 1.2 Impact

- No architectural guardrails enforced across specs
- No project-wide principles guiding implementation decisions
- Tier-2 synthesis lacks foundational context
- Each spec is evaluated in isolation without project vision

### 1.3 Reference

GitHub's spec-kit uses a **constitution-first** approach with 9 Articles that define immutable principles. Our integration adapts this pattern for the Stage 0 + NotebookLM stack.

---

## 2. Solution Overview

Constitution becomes **Tier-0 context** that Stage 0 always includes:

```
/speckit.project     → Scaffold with placeholder constitution
/speckit.vision      → Capture mission, users, non-goals (NEW)
/speckit.constitution → Define principles, guardrails (NEW)
/speckit.new         → Create spec (Phase -1 Gate checks)
/speckit.auto        → Stage 0 includes constitution in TASK_BRIEF
```

---

## 3. Data Model

### 3.1 Constitution Memory Schema

Constitution content stored as local-memory entries with:

| Field | Requirement |
|-------|-------------|
| **Domain** | `constitution` |
| **Types** | `principle`, `guardrail`, `goal`, `non-goal` |
| **Importance** | 8-10 (forced high priority) |
| **Template** | Guardian-structured per Template Guardian format |

### 3.2 Priority Schema

| Type | `initial_priority` | Rationale |
|------|-------------------|-----------|
| `type:guardrail` | 10 | Hard constraints, never violate |
| `type:principle` | 9 | Architectural values |
| `type:goal` | 8 | Mid-term objectives |
| `type:non-goal` | 8 | Explicit exclusions |

### 3.3 Constitution-Ready Definition

A project is "constitution-ready" when:
- At least 1 `type:guardrail` memory exists
- At least 1 `type:principle` memory exists
- `NL_CONSTITUTION.md` has been generated at least once

---

## 4. IQO Amendment

### 4.1 Domain Union (Not Replace)

Constitution domain MUST be added via union, preserving LLM-generated domains:

```rust
// In build_iqo() or compile_context()
if !iqo.domains.iter().any(|d| d == "constitution") {
    iqo.domains.push("constitution".to_string());
}
```

### 4.2 Separate Always-On Pass

Constitution retrieval MUST be a separate pass, not just IQO filtering:

```rust
compile_context():
    1. Build IQO from spec → query memories (existing flow)
    2. SEPARATELY: Always fetch top-N domain:constitution memories
    3. Merge both into candidates
    4. Apply MMR, assemble TASK_BRIEF
```

**Rationale**: Guarantees constitution is never accidentally filtered out by IQO heuristics.

### 4.3 Always Include N Rule

Section 0 MUST include at least 3 constitution memories (preferring guardrails, then principles) even if their `dynamic_score` would not place them in top-k.

**Implementation**: After MMR selection, ensure at least 3 constitution memories are present, backfilling from `domain:constitution` query if needed.

---

## 5. TASK_BRIEF Amendment

### 5.1 Section 0: Project Constitution (Summary)

Insert before Section 1 in `assemble_task_brief()`:

```markdown
## 0. Project Constitution (Summary)

### Principles
- [P1] We optimize for developer ergonomics over raw performance (mem-const-001)
- [P2] All public APIs must be documented (mem-const-002)

### Guardrails
- [G1] Never store secrets in plain text (mem-const-003)
- [G2] All file operations must be sandboxed (mem-const-004)

### Goals
- [Goal] Support 3 cloud providers by Q3 (mem-const-005)
```

### 5.2 Conditional Rendering

If no `domain:constitution` memories exist:
- Section 0 is **omitted**
- Diagnostic logged: `stage0.constitution=missing`
- Stage 0 does NOT error

### 5.3 Size Constraints

- Maximum 5 items in Section 0 (2-3 principles, 2-3 guardrails)
- Full constitution content in NotebookLM `NL_CONSTITUTION.md`
- TASK_BRIEF includes summaries + memory IDs only

---

## 6. Tier-2 Prompt Amendment

### 6.1 Constitution Awareness Clause

Add to Staff Engineer prompt:

> You have been given a project constitution (principles, guardrails, goals).
> When making recommendations, you MUST:
> 1. Respect all guardrails as hard constraints
> 2. Align suggestions with stated principles
> 3. Call out any spec details that conflict with the constitution

### 6.2 Divine Truth Constitution Alignment

Add subsection to Divine Truth output:

```markdown
## 2. Constitution Alignment

**Aligned with:** P1 (developer ergonomics), G2 (sandboxed ops)

**Potential conflicts:**
- Spec proposes direct file writes, but G2 requires sandboxing
- Mitigation: Use VFS abstraction layer (see Pattern P-034)
```

### 6.3 Conflict Error Category

Define `CONSTITUTION_CONFLICT_WARNING` with:
- `spec_id`
- `conflicting_guardrail_ids`
- `brief_explanation` (from NotebookLM)

---

## 7. Phase -1 Gates

### 7.1 Gate Location

Gate lives in TUI layer (`pipeline_coordinator.rs`), NOT Stage 0 engine.

**Rationale**: Stage 0 remains pure (context compilation + Tier 2 synthesis). Gating is a workflow concern.

### 7.2 Gate Checks

Before `/speckit.new` or `/speckit.auto`:

```rust
fn check_phase_minus_1_gate(project: &Project, mode: GateMode) -> Result<()> {
    let constitution_count = count_constitution_memories(project)?;

    if constitution_count == 0 {
        match mode {
            GateMode::Skip => {},
            GateMode::Warn => {
                tracing::warn!("No constitution defined for project");
                // Continue execution
            },
            GateMode::Block => {
                return Err(SpecKitError::ConstitutionRequired(
                    "Run /speckit.constitution first".to_string()
                ));
            },
        }
    }
    Ok(())
}
```

### 7.3 Configuration

```toml
[spec_kit]
phase1_gate_mode = "warn"  # warn | block | skip
```

- **Default**: `warn` (backward compatibility)
- **Recommended for production**: `block`
- When `block` mode: abort BEFORE calling Stage 0 to avoid consuming Tier 2 quota

---

## 8. New Commands

### 8.1 `/speckit.vision`

**Purpose**: Capture high-level product vision via guided Q&A

**Flow**:
1. Present Q&A wizard:
   - "What is this project's core mission?" (1-2 sentences)
   - "Who are the target users?" (list)
   - "What does success look like?" (metrics)
   - "What are explicit non-goals?" (list)
2. Store answers as `domain:constitution` memories:
   - Mission → `type:goal`
   - Users → `type:goal`
   - Non-goals → `type:non-goal`
3. Update `memory/constitution.md` Mission section
4. Generate `NL_VISION.md` artifact
5. Seed NotebookLM (if configured)

**Output**: Updated constitution file, local-memory entries, NL artifact

### 8.2 `/speckit.constitution`

**Purpose**: Define/refine formal principles and guardrails

**Flow**:
1. If no constitution exists:
   - Generate skeleton from vision (via Stage 0 + NotebookLM if available)
   - Present 9-Articles format for user fill-in
   - Store each Article as structured memory
2. If constitution exists:
   - Load current memories
   - Show current state, allow additions/edits
   - Update memories + regenerate artifacts
3. Store as `domain:constitution` memories:
   - Architectural values → `type:principle`
   - Hard constraints → `type:guardrail`
4. Update `memory/constitution.md` Principles/Constraints sections
5. Generate `NL_CONSTITUTION.md` artifact
6. Re-seed NotebookLM with updated artifacts

**Output**: Updated constitution file, local-memory entries, NL artifact

### 8.3 Post-Project Prompt Update

Update `/speckit.project` output message:

```
✓ Created Rust project: my-rust-lib

   Directory: /path/to/my-rust-lib
   Files created: 12

Switching to project directory...

Recommended:
  /speckit.vision        - Define mission and goals
  /speckit.constitution  - Set principles and guardrails

Then: /speckit.new <feature description>

Cost: $0 (zero agents, instant)
```

---

## 9. NotebookLM Artifacts

### 9.1 New Artifacts

| Artifact | Purpose | Source |
|----------|---------|--------|
| `NL_CONSTITUTION.md` | Formal principles, guardrails, constraints | `domain:constitution` memories |
| `NL_VISION.md` | High-level product vision narrative | `type:goal` + `type:non-goal` memories |

### 9.2 Artifact Structure

**NL_CONSTITUTION.md**:
```markdown
# Project Constitution

## Principles
1. **Developer Ergonomics First** - We optimize for developer experience...
2. **API Documentation Required** - All public APIs must be documented...

## Guardrails
1. **No Plaintext Secrets** - Never store secrets in plain text...
2. **Sandboxed File Operations** - All file operations must be sandboxed...

## Goals
- Support 3 cloud providers by Q3
- Achieve <100ms p99 latency

## Non-Goals
- We are NOT building a general-purpose database
- We are NOT targeting mobile platforms
```

**NL_VISION.md**:
```markdown
# Product Vision

## Mission
[One paragraph mission statement]

## Target Users
- Developer building X
- Team maintaining Y

## Success Metrics
- Metric 1: Target
- Metric 2: Target

## What We're NOT Building
- Explicit non-goal 1
- Explicit non-goal 2
```

### 9.3 Seeding Integration

Artifacts generated by `/speckit.vision` and `/speckit.constitution` should be:
1. Written to `docs/SPEC-KIT-artifacts/` (local)
2. Uploaded to project's NotebookLM notebook (manual or via `/speckit.seed`)

---

## 10. Dual Storage Strategy

### 10.1 Storage Locations

| Location | Purpose |
|----------|---------|
| `memory/constitution.md` | Human-readable, version-controlled, portable |
| Local-memory (MCP) | Semantic search, Stage 0 integration |

### 10.2 Sync Strategy

**File is source of truth**:
1. Q&A commands write to `memory/constitution.md` first
2. Parse file content into structured memories
3. Sync TO local-memory via MCP
4. On conflict, file wins

### 10.3 Cache Invalidation

Any write/update to a `domain:constitution` memory MUST trigger cache invalidation via existing `invalidate_by_memory()` mechanism.

---

## 11. Acceptance Criteria

- [ ] `/speckit.vision` command implemented with Q&A flow
- [ ] `/speckit.constitution` command implemented with Q&A flow
- [ ] `memory/constitution.md` populated with real content after commands
- [ ] Constitution stored in local-memory as `domain:constitution`
- [ ] Stage 0 DCC includes constitution in TASK_BRIEF Section 0
- [ ] IQO always includes "constitution" domain via union
- [ ] Separate always-on pass for constitution retrieval
- [ ] At least 3 constitution memories always included in Section 0
- [ ] Tier-2 prompt includes constitution awareness clause
- [ ] Divine Truth includes Constitution Alignment section
- [ ] Phase -1 Gate implemented with warn/block/skip modes
- [ ] `NL_CONSTITUTION.md` artifact generated
- [ ] `NL_VISION.md` artifact generated
- [ ] `/speckit.project` updated with recommended next steps
- [ ] Backward compatible (works without constitution)
- [ ] Documentation updated

---

## 12. Test Plan

### 12.1 Integration Test (SPEC-KIT-900)

Use `ferris-test` benchmark project at `/home/thetu/benchmark/ferris-test/`:

1. Run `/speckit.vision` with mission: "A library for printing text with Ferris"
2. Run `/speckit.constitution` to establish principles
3. Verify `memory/constitution.md` has real content
4. Verify local-memory has `domain:constitution` entries
5. Run `/speckit.new "Add ANSI color support"`
6. Verify Phase -1 Gate passes (constitution exists)
7. Run `/speckit.auto` and verify:
   - TASK_BRIEF includes Section 0 with constitution
   - Divine Truth includes Constitution Alignment

### 12.2 Edge Cases

| Test Case | Expected Behavior |
|-----------|-------------------|
| No constitution, gate=warn | Warning logged, Stage 0 proceeds |
| No constitution, gate=block | Error returned, Stage 0 not called |
| No constitution, gate=skip | No warning, Stage 0 proceeds |
| Constitution exists | Section 0 rendered with content |
| Constitution conflict | Divine Truth shows conflict + mitigation |
| Constitution update | Cache invalidated, fresh synthesis |

---

## 13. Implementation Phases

### Phase 1: Research (P87-P88) ✅
- Gap identified
- GitHub spec-kit analyzed
- Integration points documented
- Design decisions made
- Research document: `research/P88-integration-analysis.md`

### Phase 2: Data Model
- Define constitution memory schema in local-memory
- Add `domain:constitution` filtering to DCC
- Implement IQO union logic
- Add separate always-on constitution pass

### Phase 3: TASK_BRIEF & Tier-2
- Add Section 0 to `assemble_task_brief()`
- Update Tier-2 prompt with constitution awareness
- Add Constitution Alignment to Divine Truth parser

### Phase 4: Commands
- Implement `/speckit.vision` with Q&A
- Implement `/speckit.constitution` with Q&A
- Update `/speckit.project` output message

### Phase 5: Phase -1 Gates
- Add gate check to `pipeline_coordinator.rs`
- Add configuration for gate mode
- Test warn/block/skip behaviors

### Phase 6: NotebookLM Integration
- Generate `NL_CONSTITUTION.md` artifact
- Generate `NL_VISION.md` artifact
- Integrate with seeding pipeline

### Phase 7: E2E Validation
- Run full pipeline with ferris-test benchmark
- Document findings
- Update SPEC-KIT-900 with constitution test cases

---

## 14. References

- [GitHub Spec-Kit Repository](https://github.com/github/spec-kit)
- [Spec-Driven Development Guide](https://github.com/github/spec-kit/blob/main/spec-driven.md)
- SPEC-KIT-102R: Stage 0 Implementation Report
- SPEC-KIT-900: Integration Test Harness
- Research: `research/P88-integration-analysis.md`

---

## 15. Session Log

| Session | Date | Status | Notes |
|---------|------|--------|-------|
| P87 | 2025-12-01 | RESEARCH | Initial spec created, gap identified |
| P88 | 2025-12-01 | DESIGN COMPLETE | Integration analysis, design decisions, spec updated |

---

*This spec enhances the foundational workflow for spec-driven development by ensuring all specs are grounded in project-wide principles and guardrails.*
