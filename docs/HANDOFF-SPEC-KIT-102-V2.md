# SPEC-KIT-102 V2 Handoff: Stage 0 Overlay Engine

**Status**: V1 Core Complete | Integration Next
**Last Session**: P77 (2025-12-01)
**Architecture**: Rust Overlay Engine (treats local-memory as black-box backend)

---

## Quick Summary

SPEC-KIT-102 V2 defines a **Stage 0 Overlay Engine** in Rust that:
- Sits between `codex-rs` and the closed-source `local-memory` daemon
- Maintains its own SQLite overlay DB (scores, structure status, Tier 2 cache)
- Implements guardians, DCC, dynamic scoring, and Tier 2 (NotebookLM) orchestration
- Does NOT modify local-memory internals

**Key Architectural Pivot**: V1 assumed we could modify local-memory's schema. V2 recognizes local-memory is closed-source and builds an overlay layer instead.

---

## Session Progress

| Session | Progress |
|---------|----------|
| P73 | Designed `/speckit.auto` integration, IQO/Tier2 prompts, identified spec gaps |
| P74 | Completed all spec docs (12 total), research phase finished |
| P75 | **V1.1 + V1.2**: Crate setup, overlay DB, guardians (26 tests) |
| P76 | **V1.3 + V1.4**: Scoring + DCC (53 tests) |
| P77 | **V1.5**: Tier 2 Orchestration (74 tests) ✅ |

---

## V1.5 Completion Summary (P77)

### New Files
- `codex-rs/stage0/src/tier2.rs` - Tier2Client trait, DivineTruth, CausalLinkSuggestion, prompt builder, parser

### Modified Files
- `lib.rs` - Stage0Result, run_stage0() entry point
- `overlay_db.rs` - TTL-aware cache methods, batch dependency storage
- `errors.rs` - tier2(), local_memory(), dcc() error constructors

### Key Types

```rust
// Stage0Result - main return type from run_stage0()
pub struct Stage0Result {
    pub spec_id: String,
    pub divine_truth: DivineTruth,
    pub task_brief_md: String,
    pub memories_used: Vec<String>,
    pub cache_hit: bool,
    pub tier2_used: bool,
    pub latency_ms: u64,
    pub explain_scores: Option<ExplainScores>,
}

// DivineTruth - parsed Tier 2 response
pub struct DivineTruth {
    pub executive_summary: String,
    pub architectural_guardrails: String,
    pub historical_context: String,
    pub risks_and_questions: String,
    pub suggested_links: Vec<CausalLinkSuggestion>,
    pub raw_markdown: String,
}

// Tier2Client trait - NotebookLM abstraction
#[async_trait]
pub trait Tier2Client: Send + Sync {
    async fn generate_divine_truth(
        &self,
        spec_id: &str,
        spec_content: &str,
        task_brief_md: &str,
    ) -> Result<Tier2Response>;
}
```

### Test Coverage
- 74 tests total, all passing
- Clippy clean

---

## Crate Structure (Post V1.5)

```
codex-rs/stage0/
├── Cargo.toml
├── STAGE0_SCHEMA.sql
└── src/
    ├── lib.rs          # Stage0Engine, Stage0Result, run_stage0()
    ├── config.rs       # Stage0Config
    ├── errors.rs       # Stage0Error (7 categories)
    ├── guardians.rs    # MetadataGuardian, TemplateGuardian, LlmClient
    ├── overlay_db.rs   # OverlayDb (SQLite + scoring + cache)
    ├── scoring.rs      # Dynamic scoring formula
    ├── dcc.rs          # IQO, LocalMemoryClient, compile_context(), MMR
    └── tier2.rs        # Tier2Client, DivineTruth, run_stage0 helpers
```

---

## Next Steps (Choose in Next Session)

### Option A: codex-rs Integration
Wire Stage0Engine into `/speckit.auto` pipeline:
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- Create real `LocalMemoryClient` implementation (MCP adapter)
- Create real `Tier2Client` implementation (NotebookLM MCP adapter)

### Option B: Phase F - Observability
Complete V1 with:
- Push suggested causal links to local-memory via relationships API
- Structured `stage0_run` event logging
- Metrics emission

### Option C: End-to-End Test
Manual validation:
- Create NotebookLM notebook ("codex-rs Shadow Stage 0")
- Seed with architecture docs
- Run `/speckit.auto` with real Stage 0

### Option D: V2 Planning
Design next iteration:
- V2.1-V2.4: Vector DB integration
- V2.8: Multi-notebook committee
- V2.9: Knowledge seeding pipeline

---

## Spec Files Index

All spec files in repo root (`/home/thetu/code/`):

| File | Purpose |
|------|---------|
| `STAGE0_IMPLEMENTATION_GUIDE.md` | High-level architecture, phases A-F |
| `STAGE0_SCHEMA.sql` | Overlay SQLite schema |
| `STAGE0_SCORING_AND_DCC.md` | Dynamic scoring formula, DCC pipeline |
| `STAGE0_GUARDIANS_AND_ORCHESTRATION.md` | Guardians, run_stage0, cache |
| `STAGE0_CONFIG_AND_PROMPTS.md` | YAML config structure |
| `STAGE0_OBSERVABILITY.md` | Structured logging schema |
| `STAGE0_SPECKITAUTO_INTEGRATION.md` | /speckit.auto integration contract |
| `STAGE0_IQO_PROMPT.md` | IQO generation prompt |
| `STAGE0_TIER2_PROMPT.md` | Divine Truth prompt |
| `STAGE0_TASK_BRIEF_TEMPLATE.md` | DCC output format |
| `STAGE0_ERROR_TAXONOMY.md` | Error types, recovery |
| `STAGE0_METRICS.md` | Telemetry spec |

---

## Resume Prompt

```
ultrathink Load docs/HANDOFF-SPEC-KIT-102-V2.md

Resuming SPEC-KIT-102 Stage 0 Overlay Engine.

Current state:
- V1.1-V1.5 COMPLETE (74 tests, clippy clean)
- Stage 0 crate has full DCC + Tier 2 orchestration
- run_stage0() returns Stage0Result with divine_truth + task_brief
- Cache with TTL, fallback on Tier 2 failure
- Traits defined: LocalMemoryClient, LlmClient, Tier2Client

Integration points needed:
- codex-rs: Real MCP adapters for local-memory and NotebookLM
- codex-rs: Wire into /speckit.auto pipeline
- Optional: Phase F observability (structured logging, link ingestion)

What would you like to focus on?
1. codex-rs Integration (MCP adapters + pipeline wiring)
2. Phase F Observability (logging + causal links)
3. End-to-End Test (NotebookLM notebook + real run)
4. V2 Planning (vector DB, multi-notebook)
```

---

*Handoff updated: 2025-12-01 (Session P77)*
*Status: V1 Core Complete (V1.1-V1.5); Integration phase next*
