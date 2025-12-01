# SPEC-KIT-102 V2 Handoff: Stage 0 Overlay Engine

**Status**: Research Phase - Integration & Prompts Complete | Spec Gaps Remaining
**Last Session**: P73 (2025-12-01)
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

### Spec Gaps (To Be Created)
| File | Purpose | Status |
|------|---------|--------|
| `STAGE0_TASK_BRIEF_TEMPLATE.md` | DCC output format specification | **TODO** |
| `STAGE0_ERROR_TAXONOMY.md` | Stage0Error types, recovery strategies | **TODO** |
| `STAGE0_METRICS.md` | Telemetry, dashboards, alerting | **TODO** |

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
V1.1: Overlay DB & Basic Plumbing
  - Create codex-rs/stage0/ crate
  - Initialize schema from STAGE0_SCHEMA.sql
  - Basic CRUD operations

V1.2: Guardians
  - MetadataGuardian (timestamp/agent_type validation)
  - TemplateGuardian (LLM restructuring via Ollama)
  - OTHER category with best-effort classification

V1.3: Dynamic Scoring
  - Implement scoring formula (usage/recency/priority/novelty)
  - Config-driven weights
  - Usage tracking on DCC retrievals

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

### Priority 1: Spec Gaps (Next Session)
- [ ] Create NotebookLM notebook: "codex-rs – Shadow Stage 0"
- [ ] Design TASK_BRIEF.md template specification
- [ ] Design Stage0 error taxonomy
- [ ] Design Stage0 metrics/telemetry spec

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

## Resume Prompt

```
Load docs/HANDOFF-SPEC-KIT-102-V2.md

Resuming SPEC-KIT-102 Stage 0 Overlay Engine research.

Current state:
- Core architecture specs complete
- Integration contract designed (STAGE0_SPECKITAUTO_INTEGRATION.md)
- IQO and Tier 2 prompts specified
- Implementation deferred (completing other SPECs first)

Remaining design tasks:
1. Create NotebookLM notebook "codex-rs – Shadow Stage 0"
2. Design TASK_BRIEF.md template specification
3. Design Stage0 error taxonomy
4. Design Stage0 metrics/telemetry spec

Key spec files to reference:
- STAGE0_SPECKITAUTO_INTEGRATION.md (integration contract)
- STAGE0_IQO_PROMPT.md (IQO generation)
- STAGE0_TIER2_PROMPT.md (Divine Truth prompt)
- STAGE0_IMPLEMENTATION_GUIDE.md (phases A-F)

[Continue with remaining design tasks]
```

---

*Handoff updated: 2025-12-01 (Session P73)*
*Status: Research phase - completing spec gaps before implementation*
