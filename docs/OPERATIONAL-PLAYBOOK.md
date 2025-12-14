# Operational Playbook

_Extended behavioral guidance for AI agents working in this repository._

**Quick Reference Files**: See `CLAUDE.md`, `AGENTS.md`, or `GEMINI.md` for commands and project structure.

---

## When To Pause And Ask

Stop and request clarification when:

- Missing or ambiguous acceptance criteria
- Spec requires external services unavailable here
- Security/privacy implications are unclear
- Legacy `specs/**` artifact touched—plan migration before editing
- Large refactor emerges unexpectedly
- Required reference documents (`product-requirements.md`, `PLANNING.md`, relevant spec files) are absent

## Escalate Early

- State blockers, degraded guardrails, or missing telemetry explicitly
- When HAL telemetry is missing or malformed, pause and re-run the relevant guardrail command (e.g., `/guardrail.plan`) with `SPEC_OPS_TELEMETRY_HAL=1` after restoring prerequisites
- For consensus drift (agents missing, conflicting verdicts), re-run the stage or run `/spec-consensus <SPEC-ID> <stage>` and include findings in the report

## Memory Workflow

Use **local-memory MCP exclusively** for high-value knowledge (importance ≥8).

**Full protocol**: See `~/.local-memory/PROTOCOL.md` and `MEMORY-POLICY.md`.

**Quick reference**:
- Search: `local-memory search "query" --limit 5`
- Store: `mcp__local-memory__store_memory(content, tags, importance>=8, domain)`
- Health: `~/.claude/hooks/lm-dashboard.sh --compact`

**Store**: Architecture decisions, reusable patterns, critical discoveries, milestones.
**Don't store**: Session summaries, progress updates, routine operations.

**MANDATORY SESSION WORKFLOW**:
1. **Session Start**: Query local-memory for project context, recent decisions, architecture state
2. **Before Tasks**: Search local-memory for relevant prior work, patterns, solutions
3. **During Work**: Store key decisions, architecture changes, bug discoveries (importance ≥7)
4. **After Milestones**: Store outcomes, file locations, validation results, lessons learned

## NotebookLM Integration (SPEC-KIT-102)

NotebookLM provides "Tier 2" reasoning for complex synthesis queries.

**When to Use**: Stage 0 planning, deep context synthesis, "WHY" questions.
**Rate Limit**: 50 queries/day (free tier). Cache aggressively.

**Quick reference**:
```bash
# Verify service (must be running)
curl -s localhost:3456/health | jq .authenticated

# Ask a question
curl -X POST localhost:3456/api/ask \
  -H "Content-Type: application/json" \
  -d '{"notebookId": "...", "question": "..."}'
```

**Service Management**:
```bash
notebooklm service start   # Start HTTP daemon
notebooklm service status  # Check status
notebooklm health --deep   # Verify authentication
```

**Full documentation**: See `docs/SPEC-KIT-102-notebooklm-integration/`.

## Evidence & Validation Ritual

- Guardrail runs must have a clean tree unless specifically allowed (`SPEC_OPS_ALLOW_DIRTY=1`)
- Capture both success and failure artifacts; `/speckit.auto` includes automatic retry (AR-2, AR-3) but document degradations
- After `/implement`, run the full validation harness (fmt, clippy, build/tests, doc validators). Attach logs or cite evidence files in local-memory and user reports
- Evidence growth policy: 25 MB soft limit per SPEC, monitor with `/spec-evidence-stats`. See `docs/spec-kit/evidence-policy.md` for retention/archival

## Telemetry & Evidence Expectations

- Telemetry schema v1: every JSON needs `command`, `specId`, `sessionId`, `timestamp`, `schemaVersion`, `artifacts[]`
- Stage-specific fields:
  - Plan – `baseline.mode`, `baseline.artifact`, `baseline.status`, `hooks.session.start`
  - Tasks – `tool.status`
  - Implement – `lock_status`, `hook_status`
  - Validate/Audit – `scenarios[{name,status}]` (`passed|failed|skipped`)
  - Unlock – `unlock_status`
- Enable `SPEC_OPS_TELEMETRY_HAL=1` during HAL smoke tests to capture `hal.summary.{status,failed_checks,artifacts}`
- `/guardrail.auto` halts on schema violations or missing artifacts. Investigate immediately
- Evidence root: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

## Deliverable Formats

### Plans (`docs/SPEC-<id>-<slug>/plan.md`)
```markdown
# Plan: <feature / spec-id>

## Inputs
- Spec: docs/<id>-<slug>/spec.md (version/hash)
- Constitution: memory/constitution.md (version/hash)

## Work Breakdown
1. …
2. …

## Acceptance Mapping
| Requirement (Spec) | Validation Step | Test/Check Artifact |
| --- | --- | --- |
| R1: … | … | … |

## Risks & Unknowns
- …

## Consensus & Risks (Multi-AI)
- Agreement: …
- Disagreement & resolution: …

## Exit Criteria (Done)
- All acceptance checks pass
- Docs updated (list)
- Changelog/PR prepared
```

### Tasks (`docs/SPEC-<id>-<slug>/tasks.md` + SPEC.md)

- Update SPEC.md's Tasks table every time a `/tasks` or `/implement` run changes state
- Columns: Order | Task ID | Title | Status | PRD | Branch | PR | Notes
- Status ∈ {Backlog, In Progress, In Review, Blocked, Done}
- On PR open: Status → `In Review`, populate `Branch`
- On merge: Status → `Done`, fill `PR`, add dated note referencing evidence

## Multi-Agent Expectations

- **Consensus is fully automated** via native MCP integration (ARCH-002, 5.3x faster). All 13 `/speckit.*` commands operational
- **Agent roster**: Tier 2 uses gemini/claude/code (or gpt_pro for dev stages), Tier 3 adds gpt_codex, Tier 4 dynamically selects 3-5 agents
- **Degradation handling**: If agent fails, retry up to 3 times (AR-2). If still fails, continue with remaining agents (2/3 consensus still valid)
- **Consensus metadata**: Automatically records `agent`, `version`, `content` in local-memory. Synthesis includes `consensus_ok`, `degraded`, `missing_agents`, `conflicts[]`
- **Memory System**: Use local-memory MCP exclusively. Byterover deprecated 2025-10-18
- **Validation**: `/implement` runs `cargo fmt`, `cargo clippy`, build checks, tests before returning

See `docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md` for detailed system documentation.

## Config Isolation (SPEC-KIT-964)

This project uses hermetic agent isolation:

- Templates resolve: `./templates/` → embedded (NO global `~/.config/code/templates/`)
- Agents receive context ONLY from:
  - Project files (CLAUDE.md, AGENTS.md, GEMINI.md)
  - prompts.json (embedded)
  - MCP queries scoped by `project:theturtlecsz/code`

This ensures reproducible behavior regardless of user's global configuration.

## Reference Documents

Load these every session:
- `MEMORY-POLICY.md` – mandatory memory system policy
- `memory/constitution.md` – non-negotiable project charter
- `product-requirements.md` – canonical product scope
- `PLANNING.md` – high-level architecture, goals, constraints
- `SPEC.md` – single source of truth for task tracking

---

_Last Updated: 2025-12-14_
