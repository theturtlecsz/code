# SPEC-KIT-102 V2 Handoff: Stage 0 Overlay Engine

**Status**: Implementation In Progress | V1.1+V1.2 Complete, V1.3 Next
**Last Session**: P75 (2025-12-01)
**Architecture**: Rust Overlay Engine (treats local-memory as black-box backend)

---

## Quick Summary

SPEC-KIT-102 V2 defines a **Stage 0 Overlay Engine** in Rust that:
- Sits between `codex-rs` and the closed-source `local-memory` daemon
- Maintains its own SQLite overlay DB (scores, structure status, Tier 2 cache)
- Implements guardians, DCC, dynamic scoring, and Tier 2 (NotebookLM) orchestration
- Does NOT modify local-memory internals

**Key Architectural Pivot**: V1 assumed we could modify local-memory's schema. V2 recognizes local-memory is closed-source and builds an overlay layer instead.

**Session P73 Progress**:
- Designed `/speckit.auto` integration contract
- Created IQO and Tier 2 prompt specifications
- Identified remaining spec gaps (TASK_BRIEF template, error taxonomy, metrics)

**Session P74 Progress**:
- Completed all spec gaps: TASK_BRIEF template, error taxonomy, metrics
- All 12 spec documents now complete
- Research phase finished; ready for V1.1 implementation

**Session P75 Progress**:
- **V1.1 Complete**: Created `codex-rs/stage0/` crate with overlay DB + config
- **V1.2 Complete**: Implemented MetadataGuardian + TemplateGuardian + LlmClient trait
- 26 tests passing, clippy clean
- Ready for V1.3 Dynamic Scoring

---

## Spec Files Index

All spec files are in repo root (`/home/thetu/code/`):

### Core Architecture Specs
| File | Purpose | Status |
|------|---------|--------|
| `STAGE0_IMPLEMENTATION_GUIDE.md` | High-level architecture, phases A-F | Complete |
| `STAGE0_SCHEMA.sql` | Overlay SQLite schema | Complete |
| `STAGE0_SCORING_AND_DCC.md` | Dynamic scoring formula, DCC pipeline | Complete |
| `STAGE0_GUARDIANS_AND_ORCHESTRATION.md` | Metadata/Template Guardians, run_stage0, cache | Complete |
| `STAGE0_CONFIG_AND_PROMPTS.md` | YAML config structure | Complete |
| `STAGE0_OBSERVABILITY.md` | Structured logging schema (stage0_run events) | Complete |

### Integration & Prompt Specs (NEW - P73)
| File | Purpose | Status |
|------|---------|--------|
| `STAGE0_SPECKITAUTO_INTEGRATION.md` | /speckit.auto integration contract, API types | **Complete** |
| `STAGE0_IQO_PROMPT.md` | IQO generation prompt, schema, validation | **Complete** |
| `STAGE0_TIER2_PROMPT.md` | Divine Truth prompt, parsing, fallbacks | **Complete** |

### Spec Gaps (Completed - P74)
| File | Purpose | Status |
|------|---------|--------|
| `STAGE0_TASK_BRIEF_TEMPLATE.md` | DCC output format specification | **Complete** |
| `STAGE0_ERROR_TAXONOMY.md` | Stage0Error types, recovery strategies | **Complete** |
| `STAGE0_METRICS.md` | Telemetry, dashboards, alerting | **Complete** |

### Reference Documents
| File | Purpose |
|------|---------|
| `docs/LOCAL-MEMORY-ENVIRONMENT.md` | Local-memory API reference (MCP/REST) |
| `docs/HANDOFF-SPEC-KIT-102-DEFERRED.md` | Previous V1 handoff (superseded) |

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     codex-rs TUI/CLI                         │
│                   (/speckit.auto Stage 0)                    │
└─────────────────────────┬───────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────┐
│              Stage 0 Overlay Engine (Rust)                   │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  Guardians   │  │   Scoring    │  │     DCC      │       │
│  │ (Metadata +  │  │ (dynamic_    │  │ (IQO+hybrid  │       │
│  │  Template)   │  │  score)      │  │  +diversity) │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
│                                                              │
│  ┌──────────────────────────────────────────────────┐       │
│  │           Overlay SQLite DB                       │       │
│  │  - overlay_memories (scores, structure_status)    │       │
│  │  - tier2_synthesis_cache                          │       │
│  │  - cache_memory_dependencies                      │       │
│  └──────────────────────────────────────────────────┘       │
└─────────────────────────┬───────────────────────────────────┘
                          │
          ┌───────────────┼───────────────┐
          │               │               │
          ▼               ▼               ▼
┌─────────────┐  ┌─────────────┐  ┌─────────────┐
│local-memory │  │   Ollama    │  │ NotebookLM  │
│ (REST/MCP)  │  │  (LLM)      │  │   (MCP)     │
│ Black Box   │  │             │  │  Tier 2     │
└─────────────┘  └─────────────┘  └─────────────┘
```

---

## Key Design Decisions (Locked In)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Crate location | `codex-rs/stage0/` | Follows existing conventions |
| Memory content caching | No cache (fetch from local-memory) | local-memory is source of truth |
| Tier 2 cache | TTL (24h) + write invalidation | Simple, effective |
| Failure mode | Soft failure (log and continue) | Stage 0 shouldn't brick pipeline |
| NotebookLM | Single notebook for V1 | Committee in V2.8 |
| Template Guardian | Best-effort classify → OTHER fallback | Flexible structure |

---

## Implementation Phases (V1.1 → V1.5)

```
V1.1: Overlay DB & Basic Plumbing ✅ COMPLETE (P75)
  - Created codex-rs/stage0/ crate
  - Schema init from STAGE0_SCHEMA.sql via include_str!
  - Stage0Config loading from ~/.config/codex/stage0.toml
  - OverlayDb with full CRUD for 3 tables
  - Stage0Error taxonomy (7 categories)
  - 11 tests passing

V1.2: Guardians ✅ COMPLETE (P75)
  - MemoryKind enum (Pattern/Decision/Problem/Insight/Other)
  - MemoryDraft (input) + GuardedMemory (output) structs
  - MetadataGuardian: strict/lenient validation
  - TemplateGuardian: LlmClient trait + async restructuring
  - Stage0Engine::guard_memory() + guard_memory_sync()
  - MockLlmClient for testing
  - 26 tests passing (15 new guardian tests)

V1.3: Dynamic Scoring ⏳ NEXT
  - Implement scoring formula (usage/recency/priority/novelty)
  - Config-driven weights from Stage0Config.scoring
  - Real-time score updates on memory selection
  - record_selected_memories_usage() for DCC integration

V1.4: DCC (Dynamic Context Compiler)
  - IQO generation (from STAGE0_IQO_PROMPT.md)
  - Query local-memory via search/analysis MCP tools
  - Join with overlay scores, MMR diversity
  - TASK_BRIEF.md assembly

V1.5: Tier 2 Orchestration
  - run_stage0() entry point (from STAGE0_SPECKITAUTO_INTEGRATION.md)
  - Cache lookup (input_hash = hash(spec + brief))
  - NotebookLM MCP calls (from STAGE0_TIER2_PROMPT.md)
  - Divine Truth parsing and injection
```

---

## Remaining Research Tasks

### Priority 1: Spec Gaps (Completed P74)
- [x] Design TASK_BRIEF.md template specification
- [x] Design Stage0 error taxonomy
- [x] Design Stage0 metrics/telemetry spec

### Priority 1.5: Pre-Implementation Setup
- [ ] Create NotebookLM notebook: "codex-rs – Shadow Stage 0"

### Priority 2: Deferred Decisions
- [ ] Bootstrap strategy for 1000+ memories (decide during V1.1)
- [ ] Embedding source: local-memory Qdrant vs overlay vectors
- [ ] Cache granularity refinement

### Priority 3: Future Phases (V2+)
- [ ] Vector DB integration (V2.1-V2.4)
- [ ] Multi-notebook committee (V2.8)
- [ ] Knowledge seeding pipeline (V2.9)
- [ ] Learned routing (V5)

---

## Related Files

| File | Relationship |
|------|--------------|
| `codex-rs/spec-kit/` | Target integration point |
| `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Stage 0 insertion point |
| `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` | Context injection point |
| `codex-rs/ollama/` | Existing Ollama client (reusable) |
| `codex-rs/mcp-client/` | Existing MCP client (reusable) |

---

## Resume Prompt (V1.3 Dynamic Scoring)

```
Load docs/HANDOFF-SPEC-KIT-102-V2.md

Resuming SPEC-KIT-102 Stage 0 Overlay Engine - V1.3 Dynamic Scoring.

Current state:
- V1.1 Complete: codex-rs/stage0/ crate with overlay DB + config (11 tests)
- V1.2 Complete: Guardians (MetadataGuardian + TemplateGuardian + LlmClient trait, 26 tests)
- V1.3 Next: Dynamic Scoring

V1.3 Implementation Tasks:
1. Create scoring.rs with ScoringInput, ScoringParams, calculate_dynamic_score()
2. Extend OverlayDb with get_overlay_memory() and record_memory_usage()
3. Add Stage0Engine::record_selected_memories_usage() for DCC integration
4. Unit tests for scoring formula behavior

Key files to reference:
- STAGE0_SCORING_AND_DCC.md (scoring formula spec)
- codex-rs/stage0/src/config.rs (ScoringConfig already defined)
- codex-rs/stage0/src/overlay_db.rs (extend with scoring updates)

Design decisions (from P75):
- Real-time scoring on memory selection (not batch)
- Defer Ollama LlmClient adapter to V1.4+
- No global background recalculation yet

[Continue with V1.3 implementation]
```

---

## Crate Structure (Post V1.2)

```
codex-rs/stage0/
├── Cargo.toml
├── STAGE0_SCHEMA.sql        # Embedded via include_str!
└── src/
    ├── lib.rs               # Stage0Engine, exports
    ├── config.rs            # Stage0Config (TOML loading)
    ├── errors.rs            # Stage0Error (7 categories)
    ├── guardians.rs         # MemoryKind, MemoryDraft, GuardedMemory, LlmClient
    ├── overlay_db.rs        # OverlayDb (SQLite wrapper)
    └── scoring.rs           # V1.3: NEW - scoring formula
```

---

*Handoff updated: 2025-12-01 (Session P75)*
*Status: V1.1+V1.2 complete; V1.3 Dynamic Scoring next*
