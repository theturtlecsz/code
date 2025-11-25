# Spec-Kit Automation Agents - codex-rs

**Project**: codex-rs (theturtlecsz/code)
**Last Updated**: 2025-11-25 (Opus 4.5 optimization)
**Architecture Status**: Production Ready - 75% Cost Optimized

---

## Opus 4.5 Note

Guidelines below are principles, not rigid rules. With 200K context and improved judgment:
- Use extended thinking for architecture decisions, multi-file refactors, complex debugging
- Prefer expertise isolation over context savings when spawning agents
- Deviate from guidelines when context clearly warrants it

---

## Project Context

**Repository**: https://github.com/theturtlecsz/code (FORK of just-every/code)
**NOT RELATED TO**: Anthropic's Claude Code (different product)

**Fork-Specific Features**: Spec-Kit multi-agent PRD workflows, consensus synthesis via local-memory MCP, quality gates framework, native MCP integration (5.3x faster).

---

## Memory System Policy

**Use**: local-memory MCP exclusively (policy effective 2025-10-18)

| Rule | Value |
|------|-------|
| Importance threshold | ≥8 |
| Target memories | 120-150 curated |
| Tags | Namespaced: `spec:`, `type:`, `component:` |
| Forbidden tags | Dates, task IDs, status values |

**Purpose**: Reusable patterns + living handbook, NOT complete history.

See `MEMORY-POLICY.md` and `CLAUDE.md` Section 9 for details.

---

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

---

## Multi-Agent Tiers

### Tier 0: Native Rust ($0, <1s)
**Commands**: `/speckit.new`, `.clarify`, `.analyze`, `.checklist`, `.status`
**Purpose**: Pattern matching, heuristics - no AI needed

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

---

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

---

## Retry Logic

| Trigger | Max Attempts | Backoff |
|---------|--------------|---------|
| Empty/invalid results | 3 | 100→200→400ms |
| MCP "not initialized" | 3 | 100→200→400ms |
| Validation failures | 2 | Implement→Validate cycle |

---

## Technical Architecture

**Main Files** (7,883 LOC total):
- `handler.rs` (2,038 LOC) - orchestration
- `consensus.rs` (992 LOC) - MCP native integration
- `quality.rs` (807 LOC) - gates
- `evidence.rs` (499 LOC) - persistence

**Key Functions**:
```rust
pub async fn run_spec_consensus(...) -> Result<(Vec<Line>, bool)>
async fn fetch_memory_entries(...) -> Result<Vec<LocalMemorySearchResult>>
async fn remember_consensus_verdict(...) -> Result<()>
```

**MCP Timeouts**: Search 30s, Store 10s

---

## State Machine

```
Guardrail → ExecutingAgents → CheckingConsensus → [Next Stage or Retry]
                                     ↓ (if quality gates)
                               QualityGateExecuting → ... → Next Stage
```

---

## Evidence Repository

```
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
├── consensus/SPEC-ID/
│   └── {stage}_{timestamp}_verdict.json
└── commands/SPEC-ID/
    └── {stage}_{timestamp}_telemetry.json
```

**Limit**: 25 MB soft limit per SPEC. Monitor with `/spec-evidence-stats`.

---

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

---

## Documentation Reference

| Doc | Purpose |
|-----|---------|
| `CLAUDE.md` | Operational playbook |
| `MEMORY-POLICY.md` | Memory system policy |
| `SPEC.md` | Task tracker |
| `docs/spec-kit/prompts.json` | Agent prompts |
| `docs/spec-kit/model-strategy.md` | Model selection |

---

## Architecture Status

**Completed** (Oct 2025): ARCH-001 through ARCH-007, AR-1 through AR-4
**Skipped**: ARCH-008 (YAGNI), ARCH-010 (no non-TUI clients)

See `ARCHITECTURE-TASKS.md` and `REVIEW.md` for details.

---

**Maintainer**: theturtlecsz | **Last Verified**: 2025-11-25
