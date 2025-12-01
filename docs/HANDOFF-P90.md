# P90 Handoff: Constitution TASK_BRIEF Integration

**Previous Session**: P89 (Constitution Data Model)
**Next Session**: P90 (TASK_BRIEF Section 0 + Tier-2 Updates)
**Spec**: SPEC-KIT-105

---

## P89 Session Summary

| Component | File | Status |
|-----------|------|--------|
| `ConstitutionType` enum | `overlay_db.rs:32-95` | ✅ Implemented |
| `constitution_meta` table | `STAGE0_SCHEMA.sql:48-63` | ✅ Implemented |
| Version methods | `overlay_db.rs:884-1081` | ✅ Implemented |
| IQO union logic | `dcc.rs:329-342` | ✅ Implemented |
| Always-on pass | `dcc.rs:344-365` | ✅ Implemented |
| `ensure_constitution_minimum` | `dcc.rs:814-869` | ✅ Implemented |
| Tests (16 new) | `overlay_db.rs`, `dcc.rs` | ✅ 144 tests pass |

**Commit**: (uncommitted - ready for P90 to commit with TASK_BRIEF changes)

---

## External Validation (Research Feedback)

Received validation that the SPEC-KIT-105 design is sound and aligns with GitHub spec-kit while being strictly more integrated:

### Confirmed Correct

| Design Decision | Validation |
|----------------|------------|
| Constitution as both repo doc + memory domain | "Ensure constitution exists and is managed as both a repo artifact and a memory domain" |
| 5-Articles format (adapted from 9) | "Your spec should say whether you adopt it as-is, adapt it..." - we locked it down |
| Constitution versioning first-class | "This is something GitHub spec-kit doesn't do explicitly and is a real advantage of your overlay model" |
| `/speckit.plan` (plan-only surface) | "Add a dedicated 'plan-only' surface so you can support spec-kit-like workflows" |
| Stage 0 + Tier-2 in Q&A commands | "Use Stage 0 + NotebookLM in `/speckit.specify` and `/speckit.constitution` themselves" |

### Key Quote

> "Your differences table captures the main gaps and improvements accurately: you are making constitution and project memory first-class, integrated into a Stage 0 planning layer and a Tier-2 research brain, instead of leaving them as static files and one-off prompts."

---

## P90 Scope (STRICT)

Per SPEC-KIT-105 Phase 3, P90 should implement:

### 1. TASK_BRIEF Section 0 (spec.md Section 5)

Add Section 0 before Section 1 in `assemble_task_brief()`:

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

**Rules**:
- Maximum 5 items (2-3 principles, 2-3 guardrails)
- Include memory IDs for traceability
- Conditional: omit if no constitution memories exist
- Log `stage0.constitution=missing` if omitted

### 2. TASK_BRIEF Metadata JSON (spec.md Section 5.4)

Add to TASK_BRIEF metadata:
```json
{
  "constitution_version": 3,
  "constitution_hash": "sha256:abc123..."
}
```

### 3. Tier-2 Prompt Update (spec.md Section 6.1)

Add constitution awareness clause to Staff Engineer prompt:

> You have been given a project constitution (principles, guardrails, goals).
> When making recommendations, you MUST:
> 1. Respect all guardrails as hard constraints
> 2. Align suggestions with stated principles
> 3. Call out any spec details that conflict with the constitution

### 4. Divine Truth Constitution Alignment Section (spec.md Section 6.2)

Add subsection to Divine Truth output:

```markdown
## 2. Constitution Alignment

**Aligned with:** P1 (developer ergonomics), G2 (sandboxed ops)

**Potential conflicts:**
- Spec proposes direct file writes, but G2 requires sandboxing
- Mitigation: Use VFS abstraction layer (see Pattern P-034)
```

---

## Key Files for P90

| File | Changes |
|------|---------|
| `dcc.rs` | `assemble_task_brief()` - add Section 0 |
| `dcc.rs` | Task brief metadata JSON with constitution_version |
| `tier2.rs` | `build_tier2_prompt()` - add constitution clause |
| `tier2.rs` | `parse_divine_truth()` - extract Constitution Alignment |

---

## Out of Scope (P91+)

- New slash commands (`/speckit.vision`, `/speckit.constitution`, `/speckit.plan`)
- Phase -1 Gates
- NotebookLM artifacts (`NL_CONSTITUTION.md`, `NL_VISION.md`)
- E2E ferris-test benchmark
- Constitution-aware Q&A enhancements
- Drift detection batch commands

---

## Session Lineage

```
P72-P86: Stage 0 Engine Implementation
    ↓
P87-P88: Constitution Design (spec.md created)
    ↓
P89: Constitution Data Model (overlay_db, dcc selection)
    ↓
P90: TASK_BRIEF Section 0 + Tier-2 Updates  ← YOU ARE HERE
    ↓
P91: Slash Commands
    ↓
P92: Phase -1 Gates + E2E Validation
```

---

## Test Command

```bash
cd codex-rs && cargo test -p codex-stage0 -- --test-threads=1
```

All 144 tests should pass before P90 changes.
