# Spec-Kit Workflow System - Comprehensive Architecture

## Executive Summary

The spec-kit framework is a multi-agent automation system for software specification management that orchestrates consensus from up to 5 AI agents across 6 sequential stages. It combines native Rust implementation with MCP integration for cost optimization and quality gates for autonomous resolution.

**Key Stats:**
- **13 Commands**: /speckit.* namespace (new, specify, clarify, analyze, checklist, plan, tasks, implement, validate, audit, unlock, auto, status)
- **7 Guardrail Commands**: /guardrail.* namespace (plan, tasks, implement, validate, audit, unlock, auto)
- **5 AI Agents**: Gemini, Claude, Claude Code (native), GPT-Codex, GPT-Pro
- **4 Tiers**: Tier 0 (native, $0), Tier 2-lite (dual, $0.35), Tier 2 (triple, $0.80-1.00), Tier 3 (quad, $2.00), Tier 4 (dynamic, ~$11)
- **6 Stages**: Plan → Tasks → Implement → Validate → Audit → Unlock
- **604 Tests**: 100% pass rate, 42-48% estimated coverage (exceeded 40% Q1 2026 target in Oct 2025)

---

## 1. STAGE DEFINITIONS

### Stage Enum (SpecStage)

Located: `/home/thetu/code/codex-rs/tui/src/spec_prompts.rs`

```rust
pub enum SpecStage {
    Plan,       // 1: Work breakdown & acceptance mapping
    Tasks,      // 2: Task decomposition with consensus
    Implement,  // 3: Code generation + validation
    Validate,   // 4: Test strategy consensus
    Audit,      // 5: Compliance checking
    Unlock,     // 6: Final approval
}

impl SpecStage {
    pub fn key(self) -> &'static str {
        // Returns: "spec-plan", "spec-tasks", "spec-implement", "spec-validate", "spec-audit", "spec-unlock"
    }
    pub fn all() -> [SpecStage; 6] { [...] }
    pub fn display_name(self) -> &'static str { [...] }
}
```

### Agent Tiers by Stage

| Stage | Tier | Agents | Role | Cost | Time |
|-------|------|--------|------|------|------|
| Plan | 2 | Gemini (Researcher), Claude (Synthesizer), GPT-Pro (Executor/QA) | Work breakdown, risks, acceptance mapping | ~$1.00 | 10-12 min |
| Tasks | 2 | Gemini, Claude, GPT-Pro | Decomposition, dependencies, status | ~$1.00 | 10-12 min |
| Implement | 3 | Gemini, Claude, GPT-Codex, GPT-Pro | Code design, diffs, feasibility, checklist | ~$2.00 | 15-20 min |
| Validate | 2 | Gemini, Claude, GPT-Pro | Test scenarios, analysis, decision | ~$1.00 | 10-12 min |
| Audit | 2 | Gemini, Claude, GPT-Pro | Compliance, safeguards, recommendation | ~$1.00 | 10-12 min |
| Unlock | 2 | Gemini, Claude, GPT-Pro | Branch state, justification, decision | ~$1.00 | 10-12 min |

### Quality Commands (Pre-Stage Checkpoints)

| Command | Agents | Purpose | Cost | Stage |
|---------|--------|---------|------|-------|
| /speckit.clarify | 3 (Gemini, Claude, Code) | Resolve ambiguities in SPEC | ~$0.80 | Pre-planning |
| /speckit.analyze | 3 (Gemini, Claude, Code) | Cross-artifact consistency | ~$0.80 | Any |
| /speckit.checklist | 2 (Claude, Code) | Requirement quality scoring | ~$0.35 | Pre-planning |

### Diagnostic/Utility

| Command | Tier | Purpose | Cost | Time |
|---------|------|---------|------|------|
| /speckit.status | Tier 0 (Native) | TUI dashboard, instant status | $0 | <1s |
| /spec-consensus | N/A | Inspect local-memory artifacts | $0 | <1s |
| /spec-evidence-stats | N/A | Evidence footprint monitoring | $0 | <1s |

---

## 2. MULTI-AGENT ORCHESTRATION SYSTEM

### Architecture Layers

Located: `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/`

#### Layer 1: Routing (routing.rs)
- Dispatches commands via `try_dispatch_spec_kit_command()`
- Merges user config with spec-kit defaults
- Applies ACE injection for orchestrator instructions
- Fallback to upstream commands if not in registry

#### Layer 2: Command Registry (command_registry.rs)
- Dynamic trait-based registry (`SpecKitCommand` trait)
- Eliminates upstream `SlashCommand` enum conflicts
- Supports prompt-expanding and direct-execution commands
- Backward compatibility through aliases (e.g., `/new-spec` → `/speckit.new`)

#### Layer 3: Pipeline Coordination (pipeline_coordinator.rs)
- State machine: Guardrail → Agents → Consensus → Quality Gate → Next Stage
- `handle_spec_auto()`: Pipeline initiation
- `advance_spec_auto()`: Stage progression loop
- Task lifecycle: `on_spec_auto_task_started()`, `on_spec_auto_task_complete()`

#### Layer 4: Agent Orchestration (agent_orchestrator.rs)
- `auto_submit_spec_stage_prompt()`: Build prompts with MCP context
- ACE routing via `ace_route_selector.rs` (SPEC-KIT-070)
- Cost tracking per agent (`cost_tracker.rs`)
- Retry logic (AR-2, AR-3): Up to 3 retries on failure
- Aggregator effort configuration (Tier escalation on conflict retry)

#### Layer 5: Consensus (consensus.rs, consensus_coordinator.rs)
- `run_consensus_with_retry()`: Validate agent agreement
- `expected_agents_for_stage()`: Quorum checking
- Local-memory synthesis (MAINT-1, native MCP)
- Consensus verdict JSON storage

#### Layer 6: Quality Gates (quality.rs, quality_gate_handler.rs)
- 3 checkpoints: Pre-planning, post-plan, post-tasks
- Auto-resolution: Unanimous (55%), 2/3 majority w/ GPT-5 validation (~10-15%)
- Escalation to user for need-human issues
- Auto-modify spec.md/plan.md/tasks.md with backups

---

## 3. PROMPT SYSTEM

### Prompts Location

File: `/home/thetu/code/docs/spec-kit/prompts.json` (28KB, included at compile-time)

### Prompt Structure (per stage)

Each stage has entries for:
1. **Gemini** (Researcher role)
2. **Claude** (Synthesizer role)
3. **Code/GPT-Codex/GPT-Pro** (varied roles)

### Prompt Template References

Prompts reference Markdown templates that guide JSON output structure:

| Stage | Template |
|-------|----------|
| Plan | plan-template.md |
| Tasks | tasks-template.md |
| Implement | implement-template.md |
| Validate | validate-template.md |
| Audit | audit-template.md |
| Unlock | unlock-template.md |
| Clarify | clarify-template.md |
| Analyze | analyze-template.md |
| Checklist | checklist-template.md |

### Template Variables

All prompts support variable substitution:
- `${SPEC_ID}` - Spec identifier (e.g., SPEC-KIT-070)
- `${CONTEXT}` - Stage-specific context (artifacts, prior outputs)
- `${PREVIOUS_OUTPUTS.gemini}` - Prior agent JSON
- `${MODEL_ID}`, `${MODEL_RELEASE}`, `${REASONING_MODE}` - Model info
- `${PROMPT_VERSION}` - Versioned prompts for A/B testing

### Consensus Synthesis

Prompts generate JSON outputs that are:
1. Persisted individually to evidence/consensus/<SPEC-ID>/ (per-agent)
2. Synthesized by gpt_pro aggregator
3. Validated via local-memory
4. Stored as consensus_verdict.json

---

## 4. TEMPLATE SYSTEM

### Template Locations

Base: `/home/thetu/code/templates/`

```
templates/
├── spec-template.md          # SPEC document structure
├── plan-template.md          # Work breakdown format
├── tasks-template.md         # Task decomposition
├── implement-template.md     # Implementation strategy
├── validate-template.md      # Test scenarios
├── audit-template.md         # Compliance checklist
├── unlock-template.md        # Final approval memo
├── clarify-template.md       # Ambiguity resolution
├── analyze-template.md       # Consistency checks
├── checklist-template.md     # Requirement quality
└── PRD-template.md           # Product Requirements Document
```

### Template Usage

1. **Agent Context**: Prompts reference templates to guide JSON format
2. **Human Synthesis**: User reads template-aligned JSON → writes markdown
3. **Dual Purpose**: Templates accelerate both agent and human workflows (50% speed improvement validated)

### Key Template Sections

**spec-template.md** (132 lines):
- User Scenarios (P1/P2/P3 stories)
- Edge Cases
- Functional & Non-Functional Requirements
- Success Criteria
- Evidence & Validation paths

**plan-template.md** (173 lines):
- Inputs (SPEC, Constitution, PRD versions)
- Work Breakdown (7 steps typical)
- Technical Design (data model, API contracts, components)
- Acceptance Mapping (requirement → test → artifact)
- Risks & Unknowns
- Multi-Agent Consensus section
- Exit Criteria

**tasks-template.md**:
- Similar structure, focus on task decomposition
- Columns: Order, Task ID, Title, Status, Validation, Artifact

---

## 5. FILE SYSTEM STRUCTURE

### SPEC Directory Layout

```
docs/SPEC-KIT-<ID>-<slug>/
├── spec.md                  # Created by /speckit.new
├── plan.md                  # Created by /speckit.plan
├── tasks.md                 # Created by /speckit.tasks
└── PRD.md                   # Optional product requirements
```

### Evidence Repository

Root: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

#### Guardrail Telemetry
```
evidence/commands/<SPEC-ID>/
├── spec-plan_<TIMESTAMP>.json          # /guardrail.plan output
├── spec-tasks_<TIMESTAMP>.json         # /guardrail.tasks output
├── spec-implement_<TIMESTAMP>.json     # /guardrail.implement output
├── spec-validate_<TIMESTAMP>.json      # /guardrail.validate output
├── spec-audit_<TIMESTAMP>.json         # /guardrail.audit output
├── spec-unlock_<TIMESTAMP>.json        # /guardrail.unlock output
└── cost-summary.json                   # SPEC-KIT-070 cost tracking
```

#### Multi-Agent Consensus
```
evidence/consensus/<SPEC-ID>/
├── spec-plan/
│   ├── gemini_<UUID>.json              # Gemini research
│   ├── claude_<UUID>.json              # Claude synthesis
│   ├── gpt_pro_<UUID>.json             # GPT-Pro QA
│   ├── consensus_verdict.json          # Verdict structure
│   └── spec-plan_synthesis.json        # Aggregated synthesis
├── spec-tasks/
│   └── [similar structure]
├── spec-implement/
│   └── [with gpt_codex outputs]
└── [spec-validate, spec-audit, spec-unlock]
```

#### Soft Limits
- **Per-SPEC**: 25 MB (monitored via `/spec-evidence-stats`)
- **Retention**: Consensus >30d archived, >90d offloaded, >180d purged (with `--enable-purge`)

---

## 6. CONSENSUS & SYNTHESIS

### Consensus Types

#### Type 1: Agreement (Unanimous)
- All 3 agents produce same core insights
- **Confidence**: High
- **Resolution**: Auto-apply (55% typical)

#### Type 2: Majority (2/3)
- Two agents agree, one dissents
- **Validation**: GPT-5 validator checks SPEC intent
- **Resolution**: Approve (10-15% additional) or escalate

#### Type 3: Conflict (1-1-1 or other)
- No majority
- **Action**: Escalate to user (quality gate modal)
- **Rate**: <5% (0% observed in 26 completed tasks)

### Local-Memory Synthesis

Consensus artifacts stored with metadata:

```json
{
  "stage": "spec-plan",
  "spec_id": "SPEC-KIT-070",
  "timestamp": "2025-10-28T14:32:15Z",
  "agent": "claude",
  "content": { ...JSON output... },
  "consensus_ok": true,
  "missing_agents": [],
  "agreements": ["work breakdown structure", "risk assessment"],
  "conflicts": [],
  "synthesis_path": "docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-070/spec-plan_synthesis.json"
}
```

### Persistence

Located: `spec_kit/consensus.rs`, `spec_kit/consensus_coordinator.rs`

- Direct native MCP calls (ARCH-004, 5.3x faster than subprocess)
- Retry logic: 3 retries with exponential backoff
- Empty result detection (AR-3): Re-prompt with storage guidance

---

## 7. QUALITY GATES

Located: `/home/thetu/code/codex-rs/tui/src/chatwidget/spec_kit/quality.rs` (830 LOC)

### 3 Checkpoints

1. **Pre-Planning** (before /speckit.plan)
   - Run /speckit.clarify (ambiguities)
   - Run /speckit.checklist (requirement quality)
   - Confidence-based auto-fix

2. **Post-Plan** (after plan.md generation)
   - Run /speckit.analyze (consistency vs spec)
   - Check acceptance mapping coverage

3. **Post-Tasks** (after tasks.md generation)
   - Run /speckit.analyze (coverage vs plan)
   - Validate task ordering, dependencies

### Issue Classification

```rust
pub struct QualityIssue {
    pub id: String,
    pub question: String,
    pub answer: Option<String>,
    pub confidence: Confidence,     // High | Medium | Low
    pub magnitude: Magnitude,        // Critical | Important | Minor
    pub resolvability: Resolvability, // AutoFix | SuggestFix | NeedHuman
}
```

### Auto-Resolution Flow

1. **Unanimous** (all 3 agents agree) → Auto-apply (no user interaction)
2. **2/3 Majority** + **Confidence high/medium** → GPT-5 validation
   - Validators check: Does answer align with SPEC intent?
   - If yes: Apply. If no: Escalate.
3. **Escalated** → Quality gate modal shows question + options
4. **User answers** → Auto-modify artifacts

---

## 8. COST TRACKING (SPEC-KIT-070)

Located: `spec_kit/cost_tracker.rs` (486 LOC, 8 tests)

### Model Pricing Database

Updated: 2025-10-24

| Model | Input | Output | Use |
|-------|-------|--------|-----|
| Claude Haiku | $0.25/M | $1.25/M | Fast analysis (default) |
| Gemini Flash 2.5 | $0.10/M | $0.40/M | Research (12.5x cheaper than Claude Opus) |
| GPT-4o | $2.50/M | $10.00/M | Backup (hit rate limits Oct 2025) |
| GPT-5 (est.) | $10.00/M | $30.00/M | Validation, codex |

### Cost Optimization Strategy (Phase 1 Deployed)

**Quick Wins (8-hour sprint, Oct 2025):**
1. Claude Haiku (12x cheaper) → Replaces Opus for analysis
2. Gemini Flash 2.5 (12.5x cheaper) → Replaces Opus for research
3. Native SPEC-ID generation (FREE) → Eliminates $2.40 consensus cost

**Results:**
- Before: $11/spec-auto run ($1,148/month at ~104/mo)
- After: $5.50-6.60 (+GPT-4o) or $3.10-4.20 (w/o GPT-4o)
- **Savings**: 40-50% ($488-598/month)

### Cost Summary Telemetry

```json
{
  "spec_id": "SPEC-KIT-070",
  "total_cost_usd": 6.47,
  "agents": {
    "gemini": { "input_tokens": 2500, "output_tokens": 1200, "cost": 0.41 },
    "claude": { "input_tokens": 3000, "output_tokens": 1500, "cost": 1.50 },
    "gpt_pro": { "input_tokens": 2800, "output_tokens": 900, "cost": 2.56 }
  },
  "stages": {
    "plan": { "cost": 1.47, "agents": ["gemini", "claude", "gpt_pro"] },
    "tasks": { "cost": 1.35, "agents": ["gemini", "claude", "gpt_pro"] },
    "implement": { "cost": 2.14, "agents": ["gemini", "claude", "gpt_codex", "gpt_pro"] }
  },
  "budget_remaining": 93.53
}
```

---

## 9. COMMAND HANDLERS

Located: `spec_kit/command_handlers.rs` (5 commands), `spec_kit/commands/`

### Direct Execution Commands (6)

| Command | Handler | Effect |
|---------|---------|--------|
| /speckit.status | status.rs | Instant TUI dashboard (Tier 0) |
| /spec-consensus | special.rs | Query local-memory artifacts |
| /spec-evidence-stats | special.rs | Monitor footprint |
| /guardrail.* | guardrail.rs | Validate stages (shell wrappers) |

### Prompt-Expanding Commands (13)

These generate prompts and fan out to agents:

| Command | Route | Agents |
|---------|-------|--------|
| /speckit.new | plan.rs | 3 (Tier 2) |
| /speckit.specify | plan.rs | 3 (Tier 2) |
| /speckit.clarify | quality.rs | 3 (Tier 2) |
| /speckit.analyze | quality.rs | 3 (Tier 2) |
| /speckit.checklist | quality.rs | 2 (Tier 2-lite) |
| /speckit.plan | plan.rs | 3 (Tier 2) |
| /speckit.tasks | plan.rs | 3 (Tier 2) |
| /speckit.implement | plan.rs | 4 (Tier 3) |
| /speckit.validate | plan.rs | 3 (Tier 2) |
| /speckit.audit | plan.rs | 3 (Tier 2) |
| /speckit.unlock | plan.rs | 3 (Tier 2) |
| /speckit.auto | pipeline_coordinator.rs | Dynamic 3-5 (Tier 4) |

---

## 10. STATE MACHINE

Located: `spec_kit/state.rs` (828 LOC)

### SpecAutoPhase Enum

```rust
pub enum SpecAutoPhase {
    Guardrail,
    ExecutingAgents {
        expected_agents: Vec<String>,
        completed_agents: HashSet<String>,
    },
    CheckingConsensus,
    QualityGateExecuting {
        checkpoint: QualityCheckpoint,
        gates: Vec<QualityGateType>,
        active_gates: HashSet<QualityGateType>,
        expected_agents: Vec<String>,
        completed_agents: HashSet<String>,
        results: HashMap<String, Value>,
    },
    QualityGateProcessing { ... },
    QualityGateValidating { ... },
    QualityGateAwaitingHuman { ... },
}
```

### SpecAutoState Structure

```rust
pub struct SpecAutoState {
    pub spec_id: String,
    pub goal: String,
    pub stages: Vec<SpecStage>,
    pub current_index: usize,
    pub phase: SpecAutoPhase,
    pub waiting_guardrail: Option<GuardrailWait>,
    pub spec_auto_start_time: SystemTime,
    // Quality Gates
    pub quality_gate_state: HashMap<QualityCheckpoint, QualityGateState>,
    // Cost tracking
    pub cost_summary: CostSummary,
    pub aggregator_effort_notes: HashMap<SpecStage, String>,
    pub escalation_reason_notes: HashMap<SpecStage, String>,
    // Lifecycle
    pub validate_lifecycle: Option<ValidateRunInfo>,
}
```

### State Transitions

```
[Start] → Guardrail (block if failed)
  ↓
ExecutingAgents (fan out to 3-4 agents)
  ↓
CheckingConsensus (quorum, verdict, local-memory)
  ↓
QualityGateExecuting (if checkpoint active)
  ↓
QualityGateProcessing (classify: auto/majority/escalate)
  ↓
QualityGateValidating (GPT-5 for 2/3, if needed)
  ↓
QualityGateAwaitingHuman (if escalated, wait for answers)
  ↓
[Advance to next stage or complete]
```

---

## 11. VALIDATION LIFECYCLE TRACKING (SPEC-KIT-069)

Located: `spec_kit/validation_lifecycle.rs` (5 events, 186 LOC)

### Lifecycle Events

```rust
pub enum ValidateLifecycleEvent {
    Queued,                // /speckit.validate triggered
    Dispatched,            // Guardrail shell started
    CheckingConsensus,     // Waiting for multi-agent results
    Completed,             // Pipeline finished successfully
    Cancelled,             // User cancelled
    Failed,                // Error occurred
    Reset,                 // State cleared
    Deduped,               // Duplicate detected (single-flight guard)
}
```

### Single-Flight Guard (SPEC-KIT-069)

Prevents duplicate agent spawning:

```
Request 1: /speckit.validate SPEC-KIT-065 → Dispatch + track run_id
Request 2: /speckit.validate SPEC-KIT-065 (same payload) → Return "run already active"
Response: ✓ Same run_id, zero duplicate agents
```

**Validation**: 25/25 E2E tests passing, 0% duplicate dispatch rate (target <0.1%)

---

## 12. KEY MODULES & RESPONSIBILITIES

| Module | Lines | Purpose |
|--------|-------|---------|
| handler.rs | 1,491 | Re-exports (backward compat) |
| pipeline_coordinator.rs | 27K | Pipeline state machine, stage progression |
| agent_orchestrator.rs | 18K | Agent spawning, ACE routing, cost tracking |
| consensus.rs | 35K | Consensus validation, artifact handling |
| consensus_coordinator.rs | 7K | Native MCP integration (ARCH-004) |
| quality_gate_handler.rs | 45K | Quality checkpoint execution, auto-resolution |
| quality.rs | 30K | Issue classification, confidence metrics |
| state.rs | 28K | State machine, lifecycle tracking |
| cost_tracker.rs | 18K | Model pricing, cost calculation |
| ace_route_selector.rs | 22K | Aggregator effort configuration |
| ace_curator.rs | 9K | Strategic playbook curation |
| ace_prompt_injector.rs | 13K | ACE instruction injection |
| routing.rs | 6K | Command dispatch, config merging |
| command_registry.rs | 17K | Dynamic command registration |
| evidence.rs | 22K | Evidence abstraction (trait + filesystem) |
| error.rs | 7K | SpecKitError enum, error handling |
| context.rs | 11K | SpecKitContext trait, ChatWidget abstraction |

**Total**: ~375 KB of Rust code, 604 tests, 42-48% coverage

---

## 13. ENTRY POINTS & DISPATCH FLOW

### User Input Flow

```
User: "/speckit.plan SPEC-KIT-070"
  ↓ [Routing] try_dispatch_spec_kit_command() in routing.rs
  ↓ [Registry] SPEC_KIT_REGISTRY.find("speckit.plan")
  ↓ [Config] Merge user config + subagent_defaults
  ↓ [Prompt Expansion] format_subagent_command()
  ↓ [ACE Injection] submit_prompt_with_ace()
  ↓ [MCP Manager] Send to agent system
  ↓ [Agent Dispatch] Codex spawns Gemini/Claude/GPT-Pro
  ↓ [Callback] on_spec_auto_agents_complete()
  ↓ [Consensus] run_consensus_with_retry()
  ↓ [Quality Gate] determine_quality_checkpoint()
  ↓ [Storage] write_consensus_synthesis()
  ↓ [Advance] advance_spec_auto() or return verdict
```

### State Persistence Points

1. **Input**: `widget.spec_auto_state = Some(SpecAutoState::new(...))`
2. **Per-Agent**: `on_spec_auto_agents_complete()` tracks completion
3. **Consensus**: `run_consensus_with_retry()` updates verdict
4. **Quality**: `on_quality_gate_agents_complete()` modifies state
5. **Output**: `write_consensus_synthesis()`, `write_cost_summary()`

---

## 14. BACKWARD COMPATIBILITY & ALIASES

### Legacy Commands (Still Supported)

```
/new-spec              → /speckit.new
/spec-plan             → /speckit.plan
/spec-tasks            → /speckit.tasks
/spec-implement        → /speckit.implement
/spec-validate         → /speckit.validate
/spec-audit            → /speckit.audit
/spec-unlock           → /speckit.unlock
/spec-auto             → /speckit.auto
/spec-status           → /speckit.status
/spec-ops-plan         → /guardrail.plan
/spec-ops-tasks        → /guardrail.tasks
/spec-ops-implement    → /guardrail.implement
/spec-ops-validate     → /guardrail.validate
/spec-ops-audit        → /guardrail.audit
/spec-ops-unlock       → /guardrail.unlock
/spec-ops-auto         → /guardrail.auto
```

Implemented via registry aliases in `command_registry.rs`.

---

## 15. FORK-SPECIFIC MARKERS

**Total: 80+ markers across 33 files**

Key files with spec-kit isolation:
- `codex-rs/tui/src/chatwidget/spec_kit/` (15 modules, 100% isolation)
- `codex-rs/tui/src/chatwidget/mod.rs` (SpecKitContext integration, <1%)
- `codex-rs/tui/src/app.rs` (MCP spawn, routing integration)
- `codex-rs/core/src/client.rs` (Agent timeout AR-1)

**Rebase Strategy**: Marker locations documented in `docs/UPSTREAM-SYNC.md`. Monthly/quarterly syncs with merge conflict resolution matrix.

---

## 16. TELEMETRY SCHEMA

### Command Telemetry (guardrail runs)

```json
{
  "command": "spec-plan",
  "specId": "SPEC-KIT-070",
  "sessionId": "uuid",
  "timestamp": "2025-10-28T14:32:15Z",
  "schemaVersion": "1",
  "baseline": {
    "mode": "plan",
    "artifact": "docs/SPEC-KIT-070-model-cost-optimization/spec.md",
    "status": "ready"
  },
  "hooks": {
    "session": {
      "start": "2025-10-28T14:32:00Z",
      "duration_ms": 15000
    }
  },
  "artifacts": [
    {
      "name": "spec.md",
      "path": "docs/SPEC-KIT-070-model-cost-optimization/spec.md",
      "hash": "sha256:abc123..."
    }
  ]
}
```

### Consensus Synthesis

```json
{
  "spec_id": "SPEC-KIT-070",
  "stage": "spec-plan",
  "timestamp": "2025-10-28T14:32:15Z",
  "consensus_ok": true,
  "missing_agents": [],
  "agreements": [
    "Work breakdown structure is sound",
    "Risk assessment covers deployment concerns"
  ],
  "conflicts": [],
  "artifacts": [
    {
      "agent": "gemini",
      "version": "gemini-2.0-flash",
      "path": "docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-070/spec-plan/gemini_uuid.json",
      "memory_id": "spec-tracker-1234"
    }
  ]
}
```

---

## 17. ENVIRONMENT & CONFIG

### Configuration (from Rust config.toml)

```toml
[agents]
gemini = "gemini-2.0-flash"
claude = "claude-haiku-3.5"
code = "code"
gpt_codex = "gpt-5-codex"
gpt_pro = "gpt-4o"

[[subagent_commands]]
name = "speckit.plan"
command = "/mcp_call spec_prompts build_stage_prompt"
models = ["gemini", "claude", "gpt_pro"]

[spec_kit]
evidence_base = "docs/SPEC-OPS-004-integrated-coder-hooks/evidence"
max_spec_evidence_mb = 25
enable_quality_gates = true
```

### Environment Variables

```bash
HAL_SECRET_KAVEDARR_API_KEY      # Optional: HAL validation
SPEC_OPS_TELEMETRY_HAL=1         # Enable HAL telemetry
SPEC_OPS_ALLOW_DIRTY=0           # Require clean tree for guardrails
SPEC_OPS_CARGO_MANIFEST          # Workspace root (auto-set)
PRECOMMIT_FAST_TEST=0            # Skip tests in pre-commit
PREPUSH_FAST=0                   # Skip checks in pre-push
```

---

## 18. ERROR HANDLING & RESILIENCE

### Retry Strategy (AR-2, AR-3)

Located: `agent_orchestrator.rs`

```rust
const SPEC_AUTO_AGENT_RETRY_ATTEMPTS: u32 = 3;

// Retry on:
// 1. Timeout (30-minute total)
// 2. HTTP error
// 3. Malformed JSON (AR-3)
// 4. Empty result
```

### Error Types

Located: `error.rs` (275 LOC, 15 variants)

```rust
pub enum SpecKitError {
    ConfigValidation(String),
    EvidenceRead(String),
    ConsensusFailed(String),
    QualityGateFailed(String),
    FileOperation(String),
    Serialization(String),
    Io(std::io::Error),
    // ... 7 more
}

impl From<String> for SpecKitError { ... }
impl From<serde_json::Error> for SpecKitError { ... }
impl Display for SpecKitError { ... }
```

### Degradation Handling

- **If agent fails**: Continue with 2/3 consensus (still valid)
- **If MCP unavailable**: Fall back to subprocess (slower)
- **If local-memory fails**: Store JSON in evidence/ directory
- **If quality gate fails**: Escalate to user modal

---

## 19. TESTING INFRASTRUCTURE

### Test Suites

**Counts (as of Oct 2025):**
- Unit tests: 256 new + 178 baseline = 434 total
- Integration tests: 19 spec-kit modules
- E2E tests: 21 pipeline tests
- Property-based (proptest): 10 tests × 256 generative cases = 2,560 test cases
- Total assertions: 604+ passing tests, 100% pass rate

**Coverage**: 42-48% (spec-kit only, estimated)

### Key Test Fixtures

Located: `tests/common/mock_mcp.rs` (240 LOC)

```rust
pub struct MockMcpManager {
    pub calls: Vec<MCP CallRecord>,
    pub fixtures: Vec<ConsensusArtifact>,
}

// Real artifacts extracted from 3 SPECs (DEMO, 025, 045)
// Supports: plan, tasks, implement stages
// Agents: gemini, claude, code, gpt_codex, gpt_pro
```

### Test Organization

```
codex-rs/tui/tests/
├── spec_kit_handler_orchestration_tests.rs     (58 tests)
├── spec_kit_consensus_logic_tests.rs           (42 tests)
├── spec_kit_quality_resolution_tests.rs        (33 tests)
├── spec_kit_evidence_tests.rs                  (24 tests)
├── spec_kit_guardrail_tests.rs                 (25 tests)
├── spec_kit_state_tests.rs                     (27 tests)
├── spec_kit_schemas_tests.rs                   (21 tests)
├── spec_kit_error_tests.rs                     (26 tests)
├── spec_kit_edge_cases_tests.rs                (25 tests, EC01-EC25)
└── spec_kit_property_based_tests.rs            (10 tests, PB01-PB10)
```

---

## 20. DOCUMENTATION ECOSYSTEM

### Key Documents

| Document | Location | Purpose |
|----------|----------|---------|
| CLAUDE.md | /home/thetu/code/CLAUDE.md | Operator runbook (500 lines) |
| SPEC.md | /home/thetu/code/SPEC.md | Task tracker (row per task) |
| PLANNING.md | /home/thetu/code/PLANNING.md | Architecture & constraints |
| product-requirements.md | /home/thetu/code/ | Product scope |
| MEMORY-POLICY.md | /home/thetu/code/codex-rs/ | Local-memory policy |
| COMMAND_INVENTORY.md | docs/spec-kit/ | 22 command reference |
| COMMAND_REGISTRY_DESIGN.md | docs/spec-kit/ | Dynamic registry pattern |
| QUALITY_GATES_DESIGN.md | docs/spec-kit/ | 3-checkpoint architecture |
| spec-auto-full-automation-plan.md | docs/spec-kit/ | Tier strategy, stage plan |
| UPSTREAM-SYNC.md | docs/ | Rebase strategy, markers |
| evidence-policy.md | docs/spec-kit/ | Retention, archival |
| testing-policy.md | docs/spec-kit/ | Coverage roadmap |

---

## SUMMARY TABLE

| Aspect | Value |
|--------|-------|
| **Commands** | 13 /speckit.* + 7 /guardrail.* |
| **Stages** | 6 (Plan, Tasks, Implement, Validate, Audit, Unlock) |
| **Agents** | 5 (Gemini, Claude, Code, GPT-Codex, GPT-Pro) |
| **Tiers** | 4 (Tier 2-lite, Tier 2, Tier 3, Tier 4) |
| **Quality Checkpoints** | 3 (pre-plan, post-plan, post-tasks) |
| **Cost/run** | $11 → $5.50-6.60 (40-50% reduction) |
| **Consensus Latency** | ~8.7ms (native MCP) |
| **Tests** | 604 (100% pass rate, 42-48% coverage) |
| **Time to auto run** | ~60 min, ~$11 cost |
| **Template System** | 11 markdown guides, 50% speed gain |
| **Evidence Retention** | 25 MB soft limit, 30d archive, 90d offload, 180d purge |
| **Lines of Code** | ~375 KB Rust (spec-kit module) |
| **Fork Isolation** | 80+ markers, 98.8% isolated from upstream |

