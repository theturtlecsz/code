# Stage 0 ↔ /speckit.auto Integration Specification

**Version**: 1.0-draft
**Status**: Design Phase
**Last Updated**: 2025-12-01

---

## 1. Overview

This document specifies how the Shadow Stage 0 engine integrates with the existing `/speckit.auto` pipeline in codex-rs. Stage 0 runs BEFORE the Plan stage and provides enriched context (Divine Truth + TASK_BRIEF) to all subsequent stages.

### Design Goals

1. **Non-invasive**: Minimal changes to existing pipeline code
2. **Graceful degradation**: Pipeline continues if Stage 0 fails
3. **Observable**: Clear logging and TUI feedback
4. **Configurable**: Toggles for Stage 0 and Tier 2 (NotebookLM)
5. **Cacheable**: Results persist for repeated runs

---

## 2. Entry Point and Types

### 2.1 Core Types

```rust
// codex-rs/stage0/src/lib.rs

/// Result of a Stage 0 execution
#[derive(Debug, Clone)]
pub struct Stage0Result {
    /// Tier 2 synthesis from NotebookLM (or Tier 1 fallback)
    pub divine_truth: String,

    /// DCC-compiled context brief in markdown
    pub task_brief_md: String,

    /// IDs of local-memory memories used (for cache invalidation)
    pub memories_used: Vec<String>,

    /// Whether Tier 2 cache was hit
    pub cache_hit: bool,

    /// Whether Tier 2 (NotebookLM) was actually used
    pub tier2_used: bool,

    /// Execution latency
    pub latency_ms: u64,

    /// Optional explainability data (when explain=true)
    pub explain: Option<Stage0Explain>,
}

/// Explainability data for debugging/tuning
#[derive(Debug, Clone)]
pub struct Stage0Explain {
    /// Generated Intent Query Object
    pub iqo: IntentQueryObject,

    /// Top-K candidates with scores
    pub candidates: Vec<ScoredCandidate>,

    /// Scoring component breakdown per candidate
    pub score_breakdown: Vec<ScoreBreakdown>,
}

/// Per-run configuration (from config + CLI flags)
#[derive(Debug, Clone)]
pub struct Stage0Config {
    /// Master enable switch
    pub enabled: bool,

    /// Enable explainability output
    pub explain: bool,

    /// Enable Tier 2 (NotebookLM) calls
    pub tier2_enabled: bool,

    /// Tier 2 cache TTL in hours
    pub tier2_cache_ttl_hours: u32,

    /// Maximum memories to retrieve
    pub max_candidates: usize,

    /// Top-K after reranking
    pub top_k: usize,
}

impl Default for Stage0Config {
    fn default() -> Self {
        Self {
            enabled: true,
            explain: false,
            tier2_enabled: true,
            tier2_cache_ttl_hours: 24,
            max_candidates: 50,
            top_k: 10,
        }
    }
}
```

### 2.2 Engine API

```rust
// codex-rs/stage0/src/lib.rs

/// Environment context passed from /speckit.auto
#[derive(Debug, Clone)]
pub struct EnvContext {
    /// Current working directory
    pub cwd: PathBuf,

    /// Git branch (if available)
    pub git_branch: Option<String>,

    /// Recent files touched (for relevance hints)
    pub recent_files: Vec<PathBuf>,

    /// SPEC directory path
    pub spec_dir: PathBuf,
}

impl Stage0Engine {
    /// Create a new Stage0Engine instance
    pub async fn new(overlay_db_path: &Path) -> anyhow::Result<Self>;

    /// Main entry point - called by /speckit.auto before Plan stage
    pub async fn run_stage0(
        &self,
        spec_id: &str,
        spec_content: &str,
        env: &EnvContext,
        config: &Stage0Config,
    ) -> anyhow::Result<Stage0Result>;

    /// Check if Stage 0 is available (overlay DB initialized, etc.)
    pub fn is_available(&self) -> bool;

    /// Get config from file (with defaults)
    pub fn load_config() -> Stage0Config;
}
```

---

## 3. Pipeline Integration Point

### 3.1 Where Stage 0 Runs

Stage 0 executes in `handle_spec_auto()` AFTER state creation but BEFORE the first call to `advance_spec_auto()`:

```
handle_spec_auto()
    │
    ├─ Validate config
    ├─ Load PipelineConfig
    ├─ Check evidence size limits
    ├─ Create SpecAutoState
    │
    ├─ ══════════════════════════════════════════
    │   STAGE 0 INSERTION POINT (NEW)
    │   - Check if Stage0 enabled in config
    │   - Call Stage0Engine::run_stage0()
    │   - Store Stage0Result in SpecAutoState
    │   - Write task_brief.md to SPEC directory
    │   - Display TUI status
    │  ══════════════════════════════════════════
    │
    └─ advance_spec_auto()
           └─ Plan → Tasks → Implement → ...
```

### 3.2 State Changes

Add to `SpecAutoState` in `state.rs`:

```rust
pub struct SpecAutoState {
    // ... existing fields ...

    /// Stage 0 result (Divine Truth + TASK_BRIEF)
    pub stage0_result: Option<Stage0Result>,

    /// Stage 0 config used for this run
    pub stage0_config: Stage0Config,

    /// Whether Stage 0 was skipped (disabled or failed)
    pub stage0_skipped: bool,

    /// Stage 0 skip reason (for logging/display)
    pub stage0_skip_reason: Option<String>,
}
```

### 3.3 Integration Code Location

**File**: `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`

**Function**: `handle_spec_auto()`

**Insertion point**: After line ~132 (`widget.spec_auto_state = Some(state);`), before `advance_spec_auto(widget);`

---

## 4. Context Injection

### 4.1 How Divine Truth and TASK_BRIEF are Used

Two injection points:

1. **File-based**: Write `task_brief.md` to SPEC directory (persists, inspectable)
2. **Prompt-based**: Inject Divine Truth into agent prompts (runtime)

### 4.2 File-based Injection

After Stage 0 completes, write to SPEC directory:

```
docs/SPEC-{ID}/
├── spec.md           # Original spec (existing)
├── task_brief.md     # NEW: DCC-compiled context
├── divine_truth.md   # NEW: Tier 2 synthesis (optional)
├── plan.md           # Plan stage output (existing)
└── ...
```

**File format** (`task_brief.md`):
```markdown
<!-- Generated by Stage 0 - DO NOT EDIT -->
<!-- Memories: mem_abc123, mem_def456, ... -->
<!-- Generated: 2025-12-01T12:34:56Z -->

# Task Context Brief

## Historical Decisions & Patterns

[DCC-summarized relevant memories]

## Anti-Patterns to Avoid

[From local-memory "type:anti-pattern" memories]

## Related Prior Work

[Links to related SPECs/memories]
```

**File format** (`divine_truth.md`):
```markdown
<!-- Generated by Stage 0 Tier 2 (NotebookLM) -->
<!-- Cache: hit/miss, TTL: 24h -->
<!-- Notebook: codex-rs – Shadow Stage 0 -->

# Divine Truth

[NotebookLM synthesis output]

## Key Recommendations

[Structured recommendations from NotebookLM]
```

### 4.3 Prompt-based Injection

In `build_individual_agent_prompt()` (agent_orchestrator.rs), inject Stage 0 context similar to ACE bullets:

```rust
// After spec.md content, before plan.md
if let Some(stage0_result) = &state.stage0_result {
    context.push_str("## Stage 0: Task Context Brief\n");
    context.push_str(&stage0_result.task_brief_md);
    context.push_str("\n\n");

    if stage0_result.tier2_used && !stage0_result.divine_truth.is_empty() {
        context.push_str("## Stage 0: Divine Truth (NotebookLM)\n");
        context.push_str(&stage0_result.divine_truth);
        context.push_str("\n\n");
    }
}
```

---

## 5. Failure and Fallback Behavior

### 5.1 Failure Modes

| Condition | Behavior | User Feedback |
|-----------|----------|---------------|
| Stage 0 disabled (config) | Skip entirely | "Stage 0: Disabled" |
| Overlay DB not found | Skip, log warning | "Stage 0: Overlay DB unavailable" |
| DCC fails (LLM error) | Skip, log error | "Stage 0: DCC failed, continuing without" |
| Tier 2 fails (NotebookLM down) | Use Tier 1 only | "Stage 0: Tier 2 unavailable, using local brief" |
| Tier 2 timeout | Use Tier 1 only | "Stage 0: Tier 2 timeout, using local brief" |

### 5.2 Fallback Implementation

```rust
// In handle_spec_auto(), Stage 0 section:

let stage0_result = if stage0_config.enabled {
    match Stage0Engine::new(&overlay_db_path).await {
        Ok(engine) => {
            let start = Instant::now();
            match engine.run_stage0(&spec_id, &spec_content, &env_ctx, &stage0_config).await {
                Ok(result) => {
                    // Success path
                    Some(result)
                }
                Err(e) => {
                    // Soft failure - log and continue
                    tracing::warn!("Stage 0 failed: {}, continuing without", e);
                    state.stage0_skip_reason = Some(format!("DCC error: {}", e));
                    None
                }
            }
        }
        Err(e) => {
            // Engine init failed - skip Stage 0
            tracing::warn!("Stage 0 engine unavailable: {}", e);
            state.stage0_skip_reason = Some(format!("Engine unavailable: {}", e));
            None
        }
    }
} else {
    state.stage0_skip_reason = Some("Disabled in config".to_string());
    None
};

state.stage0_result = stage0_result;
state.stage0_skipped = state.stage0_result.is_none();
```

### 5.3 Decision: Hard vs Soft Failure

**V1 Decision**: **Soft failure (Option B)** - Log error and continue in degraded mode.

Rationale:
- Stage 0 is an enhancement, not a requirement
- Pipeline should never be blocked by Stage 0 issues
- Users can debug via logs and retry
- Allows iterating on Stage 0 without breaking existing workflows

---

## 6. CLI and UX Touchpoints

### 6.1 CLI Flags

Add to `/speckit.auto` command parsing:

| Flag | Description | Default |
|------|-------------|---------|
| `--no-stage0` | Disable Stage 0 entirely | false |
| `--stage0-explain` | Enable explainability output | false |
| `--stage0-tier1-only` | Skip Tier 2 (NotebookLM) | false |
| `--stage0-debug` | Dump IQO/candidates to file | false |

**Example usage**:
```bash
/speckit.auto SPEC-123 --stage0-explain
/speckit.auto SPEC-123 --no-stage0
/speckit.auto SPEC-123 --stage0-tier1-only  # DCC only, no NotebookLM
```

### 6.2 TUI Feedback

Display Stage 0 status in the pipeline initiation output:

```
/spec-auto SPEC-KIT-123
Goal: Add user authentication
Resume from: Plan
HAL mode: mock (default)

Stage 0: ✓ NotebookLM (cache miss, 8.3s)
         10 memories used, 2 anti-patterns identified
         task_brief.md written to SPEC directory

🚀 Launching 3 agents in sequential pipeline mode...
```

Alternative status messages:
- `Stage 0: ✓ Local brief only (Tier 2 disabled)`
- `Stage 0: ✓ NotebookLM (cache hit, 0.1s)`
- `Stage 0: ⚠ Skipped (DCC failed: timeout)`
- `Stage 0: ⚠ Tier 2 unavailable, using local brief`
- `Stage 0: ○ Disabled`

---

## 7. Configuration

### 7.1 Config File Location

**Primary**: `~/.config/codex/stage0.toml`
**Fallback**: Environment variables

### 7.2 Config Schema

```toml
# ~/.config/codex/stage0.toml

[stage0]
# Master enable switch
enabled = true

# Enable explainability (debugging)
explain = false

# DCC configuration
max_candidates = 50
top_k = 10

[stage0.tier2]
# Enable NotebookLM Tier 2 calls
enabled = true

# NotebookLM notebook ID (placeholder until created)
notebook_id_shadow = "YOUR-NOTEBOOKLM-NOTEBOOK-ID"

# Cache TTL in hours
cache_ttl_hours = 24

# Timeout for NotebookLM calls (seconds)
timeout_secs = 60

[stage0.scoring]
# Dynamic scoring weights (sum to 1.0)
usage_weight = 0.30
recency_weight = 0.30
priority_weight = 0.25
age_decay_weight = 0.15

# Novelty boost for fresh memories (< 7 days)
novelty_boost = 1.2

[stage0.overlay]
# Overlay DB location
db_path = "~/.config/codex/local-memory-overlay.db"
```

### 7.3 Environment Variable Overrides

| Variable | Description |
|----------|-------------|
| `CODEX_STAGE0_ENABLED` | Override enabled flag (0/1) |
| `CODEX_STAGE0_TIER2_ENABLED` | Override Tier 2 flag (0/1) |
| `CODEX_STAGE0_NOTEBOOK_ID` | Override notebook ID |
| `CODEX_STAGE0_EXPLAIN` | Enable explain mode (0/1) |

---

## 8. Observability

### 8.1 Structured Logging

Emit structured log events for Stage 0 execution:

```rust
// Event schema
struct Stage0RunEvent {
    request_id: String,       // Correlation ID (from SpecAutoState.run_id)
    spec_id: String,
    timestamp: DateTime<Utc>,

    // Timing
    dcc_latency_ms: u64,
    tier2_latency_ms: Option<u64>,
    total_latency_ms: u64,

    // Results
    memories_count: usize,
    cache_hit: bool,
    tier2_used: bool,
    tier2_error: Option<String>,

    // Config
    explain_enabled: bool,
    max_candidates: usize,
    top_k: usize,
}
```

### 8.2 Log Integration

Use existing `execution_logger` pattern from `SpecAutoState`:

```rust
// Log Stage 0 start
state.execution_logger.log_event(ExecutionEvent::Stage0Start {
    run_id: run_id.clone(),
    spec_id: spec_id.clone(),
    timestamp: ExecutionEvent::now(),
    tier2_enabled: stage0_config.tier2_enabled,
});

// Log Stage 0 complete
state.execution_logger.log_event(ExecutionEvent::Stage0Complete {
    run_id: run_id.clone(),
    spec_id: spec_id.clone(),
    timestamp: ExecutionEvent::now(),
    memories_count: result.memories_used.len(),
    cache_hit: result.cache_hit,
    tier2_used: result.tier2_used,
    latency_ms: result.latency_ms,
});
```

### 8.3 Debug Output (--stage0-debug)

When `--stage0-debug` is passed, write detailed output to:
`~/.config/codex/stage0-debug/{spec_id}-{timestamp}.json`

Contents:
```json
{
  "spec_id": "SPEC-KIT-123",
  "timestamp": "2025-12-01T12:34:56Z",
  "iqo": {
    "domains": ["spec-kit", "rust"],
    "required_tags": ["type:decision"],
    "optional_tags": ["type:pattern", "type:anti-pattern"],
    "keywords": ["authentication", "user", "session"],
    "max_candidates": 50
  },
  "candidates": [
    {
      "memory_id": "mem_abc123",
      "content_preview": "Authentication decision: Use JWT...",
      "scores": {
        "semantic_similarity": 0.89,
        "dynamic_score": 0.75,
        "combined": 0.82
      }
    }
  ],
  "task_brief_preview": "## Historical Decisions...",
  "divine_truth_preview": "## Key Recommendations..."
}
```

---

## 9. Integration Checklist

### 9.1 Files to Create (Stage 0 crate)

- [ ] `codex-rs/stage0/Cargo.toml`
- [ ] `codex-rs/stage0/src/lib.rs` - Public API
- [ ] `codex-rs/stage0/src/config.rs` - Configuration loading
- [ ] `codex-rs/stage0/src/overlay_db/mod.rs` - SQLite operations
- [ ] `codex-rs/stage0/src/dcc/mod.rs` - Dynamic Context Compiler
- [ ] `codex-rs/stage0/src/tier2/mod.rs` - NotebookLM orchestration
- [ ] `codex-rs/stage0/src/scoring.rs` - Dynamic scoring
- [ ] `codex-rs/stage0/src/guardians/mod.rs` - Metadata/Template guardians

### 9.2 Files to Modify (TUI integration)

- [ ] `codex-rs/Cargo.toml` - Add stage0 to workspace
- [ ] `codex-rs/tui/Cargo.toml` - Add stage0 dependency
- [ ] `codex-rs/tui/src/chatwidget/spec_kit/state.rs` - Add Stage0 fields
- [ ] `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - Stage0 call
- [ ] `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` - Context injection
- [ ] `codex-rs/tui/src/chatwidget/spec_kit/execution_logger.rs` - Stage0 events
- [ ] `codex-rs/tui/src/slash_command.rs` - CLI flags

### 9.3 Files to Create (Documentation)

- [ ] `codex-rs/stage0/README.md` - Crate documentation
- [ ] `docs/spec-kit/STAGE0-USER-GUIDE.md` - User documentation

---

## 10. Future Considerations

### 10.1 Vector DB Integration (V2)

When vector DB is added (Phase V2), the integration point remains the same. Only DCC internals change to use hybrid retrieval.

### 10.2 Multi-Notebook Committee (V2.8)

When notebook committee is implemented, `Stage0Config` gains:
```rust
pub notebook_strategy: NotebookStrategy::Single | Committee,
pub notebook_arch_id: Option<String>,
pub notebook_bugs_id: Option<String>,
pub notebook_diary_id: Option<String>,
```

### 10.3 Learned Routing (V5)

When learned routing is added, the integration hook is in `run_stage0()` decision logic:
- Should we call Tier 2?
- Which notebooks to query?
- How much effort to spend on DCC?

---

## Appendix A: Sequence Diagram

```
┌─────────────┐     ┌──────────────┐     ┌─────────────┐     ┌────────────┐
│ /speckit.auto│     │ Stage0Engine │     │local-memory │     │ NotebookLM │
└──────┬──────┘     └──────┬───────┘     └──────┬──────┘     └─────┬──────┘
       │                   │                    │                  │
       │ run_stage0()      │                    │                  │
       │──────────────────>│                    │                  │
       │                   │                    │                  │
       │                   │ search(IQO)        │                  │
       │                   │───────────────────>│                  │
       │                   │                    │                  │
       │                   │  candidates        │                  │
       │                   │<───────────────────│                  │
       │                   │                    │                  │
       │                   │ join w/ overlay    │                  │
       │                   │ scores, rerank     │                  │
       │                   │                    │                  │
       │                   │ ask_question(spec+brief)              │
       │                   │───────────────────────────────────────>│
       │                   │                    │                  │
       │                   │        divine_truth                   │
       │                   │<───────────────────────────────────────│
       │                   │                    │                  │
       │  Stage0Result     │                    │                  │
       │<──────────────────│                    │                  │
       │                   │                    │                  │
       │ inject into       │                    │                  │
       │ agent prompts     │                    │                  │
       │                   │                    │                  │
       └───────────────────┴────────────────────┴──────────────────┘
```

---

*Document generated from research session P73 (2025-12-01)*
