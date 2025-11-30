# Multi-Agent Architecture

_System documentation for the spec-kit multi-agent consensus framework._

**For operational instructions**: See `CLAUDE.md`, `AGENTS.md`, or `GEMINI.md` in repo root.

---

## Overview

Spec-kit uses a multi-agent consensus system for complex development stages. Agents from different providers (OpenAI, Anthropic, Google) collaborate to produce validated outputs.

**Key Metrics**:
- Native MCP integration: 5.3x faster than subprocess (8.7ms typical)
- 13 `/speckit.*` commands fully automated
- 75% cost reduction via tiered model strategy (SPEC-KIT-070)

## Agent Roster

| Agent | Model | Role | Used In |
|-------|-------|------|---------|
| **gemini-25-flash** | gemini-2.5-flash | Cheap research | plan, validate |
| **claude-haiku-45** | claude-3.5-haiku | Cheap validation | plan, validate, implement |
| **gpt5-low** | gpt-5 (low effort) | Simple analysis | specify, tasks |
| **gpt5-medium** | gpt-5 (medium effort) | Planning | plan, validate |
| **gpt5-high** | gpt-5 (high effort) | Critical decisions | audit, unlock |
| **gpt_codex** | gpt-5-codex (HIGH) | Code generation | implement only |
| **gemini-25-pro** | gemini-2.5-pro | Premium reasoning | audit, unlock |
| **claude-sonnet-45** | claude-4.5-sonnet | Premium analysis | audit, unlock |
| **code** | Native Rust | Zero-cost heuristics | Tier 0 commands |

## Tiered Model Strategy

### Tier 0: Native Rust ($0, <1s)
**Commands**: `/speckit.new`, `.project`, `.clarify`, `.analyze`, `.checklist`, `.status`
**Purpose**: Pattern matching, template expansion, heuristics - no AI needed

### Tier 1: Single Agent (~$0.10, 3-5 min)
**Agent**: gpt5-low
**Commands**: `/speckit.specify`, `.tasks`

### Tier 2: Multi-Agent (~$0.11-0.35, 8-12 min)
**Commands**:
- `.plan`: gemini-flash + claude-haiku + gpt5-medium
- `.validate`: gemini-flash + claude-haiku + gpt5-medium
- `.implement`: gpt_codex + claude-haiku validator

### Tier 3: Premium (~$0.80, 10-12 min)
**Agents**: gemini-pro + claude-sonnet + gpt5-high
**Commands**: `/speckit.audit`, `.unlock`

### Tier 4: Full Pipeline (~$2.70, 45-50 min)
**Command**: `/speckit.auto SPEC-ID`
Routes through all tiers strategically. (Was $11, 75% reduction via SPEC-KIT-070)

**Principle**: "Agents for reasoning, NOT transactions"
- Pattern matching → Native Rust (FREE, instant)
- Strategic decisions → Multi-agent consensus (justified cost)
- Code generation → Specialist model (gpt-5-codex)

## Consensus Workflow

**Step 1**: Spawn agents in parallel for stage
**Step 2**: Each agent stores analysis to local-memory with tags `spec:SPEC-ID`, `stage:NAME`
**Step 3**: `check_consensus_and_advance_spec_auto()` fetches via MCP (8.7ms avg)
**Step 4**: Validates participation, extracts gpt_pro consensus, checks conflicts
**Step 5**: Advance to next stage or retry (max 3x)

### Classification
- **OK**: All agents present, no conflicts → advance
- **Degraded**: 2/3 agents, no conflicts → advance with warning
- **Conflict**: Non-empty conflicts → retry or escalate
- **No consensus**: <50% participation → retry

## Retry Logic

| Trigger | Max Attempts | Backoff |
|---------|--------------|---------|
| Empty/invalid results | 3 | 100→200→400ms |
| MCP "not initialized" | 3 | 100→200→400ms |
| Validation failures | 2 | Implement→Validate cycle |

## State Machine

```
Guardrail → ExecutingAgents → CheckingConsensus → [Next Stage or Retry]
                                     ↓ (if quality gates)
                               QualityGateExecuting → ... → Next Stage
```

## Technical Architecture

**Main Files** (7,883 LOC total):
- `handler.rs` (2,038 LOC) - orchestration
- `consensus.rs` (992 LOC) - MCP native integration
- `quality.rs` (807 LOC) - gates
- `evidence.rs` (499 LOC) - persistence
- `templates/mod.rs` - template resolution

**Key Functions**:
```rust
pub async fn run_spec_consensus(...) -> Result<(Vec<Line>, bool)>
async fn fetch_memory_entries(...) -> Result<Vec<LocalMemorySearchResult>>
async fn remember_consensus_verdict(...) -> Result<()>
pub fn resolve_template(name: &str) -> Result<String>
```

**MCP Timeouts**: Search 30s, Store 10s

## Evidence Repository

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── consensus/SPEC-ID/
│   └── {stage}_{timestamp}_verdict.json
└── commands/SPEC-ID/
    └── {stage}_{timestamp}_telemetry.json
```

**Limit**: 25 MB soft limit per SPEC. Monitor with `/spec-evidence-stats`.

## Template System

Spec-kit uses a template resolution system with hermetic isolation (SPEC-KIT-964):

```
Priority Order:
1. ./templates/{name}-template.md        (project-local, highest)
2. [embedded in binary]                  (compiled defaults)

Note: Global user config (~/.config/code/templates/) is NOT checked
to ensure hermetic agent isolation and reproducible behavior.
```

### Embedded Templates (11 total)

| Template | Used By | Purpose |
|----------|---------|---------|
| `PRD-template.md` | `/speckit.new`, `.specify` | Product Requirements Document |
| `spec-template.md` | `/speckit.new` | Feature specification |
| `plan-template.md` | `/speckit.plan` | Work breakdown structure |
| `tasks-template.md` | `/speckit.tasks` | Task decomposition |
| `implement-template.md` | `/speckit.implement` | Implementation guide |
| `validate-template.md` | `/speckit.validate` | Test strategy |
| `audit-template.md` | `/speckit.audit` | Compliance checklist |
| `unlock-template.md` | `/speckit.unlock` | Ship decision |
| `evidence-template.md` | All stages | Evidence capture |
| `consensus-template.md` | Multi-agent stages | Synthesis format |
| `CLAUDE-template.md` | `/speckit.project` | Project instructions |

## Multi-IDE Integration

### Supported Environments

| IDE/CLI | Config Location | Model Default |
|---------|-----------------|---------------|
| Claude Code | `~/.claude/` | claude-opus-4-5 |
| Gemini CLI | `.gemini/settings.json` | gemini-2.5-flash |
| Codex TUI | `~/.config/code/` | gpt-5 |

### Agent Routing by IDE

| Stage | Claude Code | Gemini CLI | Codex TUI |
|-------|-------------|------------|-----------|
| Tier 0 | Native | Native | Native |
| Tier 1-2 | Spawns agents | Uses gemini-flash | Spawns agents |
| Tier 3 | Premium agents | gemini-pro | Premium agents |

**Note**: Gemini CLI uses its configured model directly; multi-agent consensus requires TUI or Claude Code.

## Debugging

### "MCP manager not initialized yet"
Auto-handled by retry logic. Verify: `local-memory --version`

### "No consensus artifacts found"
Check: `/spec-evidence-stats --spec SPEC-ID`
Check: `local-memory search "SPEC-ID stage:plan"`

### "Consensus degraded: missing agents"
2/3 agents still valid. Check TUI history for errors.

### "Evidence footprint exceeds 25MB"
Archive old SPECs. Run `/spec-evidence-stats`.

### "Template not found"
Check resolution order: `./templates/` → embedded (SPEC-KIT-964: no global config)

## Architecture Status

**Completed** (Oct-Nov 2025): ARCH-001 through ARCH-007, AR-1 through AR-4, SPEC-KIT-957 through 963
**Skipped**: ARCH-008 (YAGNI), ARCH-010 (no non-TUI clients)

See `ARCHITECTURE-TASKS.md` and `REVIEW.md` for details.

---

_Last Updated: 2025-11-30_
