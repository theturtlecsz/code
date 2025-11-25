# CLAUDE.md — How Claude Code Works In This Repo

## Repository Context

**This Repository**: https://github.com/theturtlecsz/code (FORK)
**Upstream**: https://github.com/just-every/code (community fork of OpenAI Codex)
**NOT RELATED TO**: Anthropic's Claude Code (different product)

**Fork-Specific Additions**:
- Spec-kit automation framework (multi-agent PRD workflows)
- Native MCP integration for consensus synthesis (5.3x faster than subprocess)
- Quality gates framework
- Evidence repository and telemetry collection

---

This playbook gives Claude Code everything it needs to operate safely inside this **theturtlecsz/code fork** (based on **just-every/code** upstream). Read it before touching the tree and keep it open while you work.

## 0. Prerequisites & Known Limitations (October 19, 2025)
- **Foundation docs now exist:** `product-requirements.md` and `PLANNING.md` were added in response to guardrail audits. If either goes missing, stop and recreate or escalate.
- **Consensus automation OPERATIONAL:** Native MCP integration complete (ARCH-004/MAINT-1, 2025-10-18). All 13 /speckit.* commands fully automated with multi-agent consensus. Performance: 5.3x faster than subprocess (8.7ms typical, validated via MCP benchmark tests).
- **Cargo workspace location:** run Rust commands from `codex-rs/` (for example `cd codex-rs && cargo test -p codex-tui spec_auto`). Guardrail scripts set `SPEC_OPS_CARGO_MANIFEST` when needed, but manual commands must honour the workspace root.
- **HAL secrets:** full validation requires `HAL_SECRET_KAVEDARR_API_KEY`. If unavailable, set `SPEC_OPS_HAL_SKIP=1` (decision on default behaviour pending) and document the skip in results.
- **Evidence footprint:** keep evidence under the 25 MB per-SPEC soft limit; use `/spec-evidence-stats` after large runs. Current: All SPECs within limit ✅ (per MAINT-4 evidence automation, 2025-10-18).
- **Multi-provider model support (SPEC-KIT-952):** Claude models route through native CLI with streaming support. Gemini CLI routing disabled (see Known Limitations).

## 0.1 Model Guidance (Opus 4.5)

**Primary Model**: Claude Opus 4.5 (`claude-opus-4-5-20251101`)

### Extended Thinking

Use deep reasoning (`ultrathink`) for:
- Architecture decisions affecting >3 files
- Multi-agent consensus synthesis and conflict resolution
- Complex debugging with multiple hypotheses
- Security audit and compliance review
- Refactors touching shared interfaces

Use standard reasoning for:
- Single-file changes and bug fixes
- Documentation updates
- Status queries and diagnostics
- Direct tool operations

### Judgment Trust

Opus 4.5 has improved instruction following and nuanced decision-making. Guidelines in this document are **principles, not absolute rules**. When context clearly warrants deviation:
1. Document your reasoning briefly
2. Proceed with the appropriate action
3. The goal is quality outcomes, not mechanical compliance

### Context Efficiency

With 200K tokens available:
- Prefer loading full files over incremental reads when understanding is needed
- Agent spawning for "context preservation" is less critical than with previous models
- Focus on expertise isolation (specialized prompts) rather than context savings

### Validation Tiers

Match validation effort to change scope:
- **<50 lines**: Trust model self-check, validate after completion
- **50-200 lines**: Run fmt + clippy after completion
- **>200 lines or cross-module**: Full validation harness (fmt, clippy, build, tests)

### Multi-Provider CLI Setup (SPEC-KIT-952)

The TUI supports three model providers with different authentication methods:

| Provider | TUI Display Names | Auth Method | Status |
|----------|-------------------|-------------|--------|
| **ChatGPT** | gpt-5, gpt-5-codex | Native OAuth | ✅ Working |
| **Claude** | claude-opus-4-5, claude-sonnet-4-5, claude-haiku-4-5 | CLI routing | ✅ Working |
| **Gemini** | gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash | CLI routing | ✅ Working |

**Note**: Display names are TUI shortcuts. Actual API model IDs (e.g., `claude-opus-4-5-20251101`) are resolved at runtime.

**Claude CLI Setup (Working)**:
```bash
# Install from https://claude.ai/download
# Then authenticate:
claude
# Follow prompts to complete login
```

**Using Claude Models:**
```bash
# Select model via /model command
/model claude-sonnet-4-5
/model claude-opus-4-5
/model claude-haiku-4-5

# Or use model selector
/model
```

**Multi-turn conversations**: ✅ Fully supported with Claude CLI routing

**Known Limitations (SPEC-KIT-952)**:
- When selecting a Claude model without the CLI installed, you'll see installation instructions in chat history
- CLI responses may take 2-25s (variability in CLI performance)

## 1. Load These References Every Session
- `MEMORY-POLICY.md` – **mandatory** memory system policy (local-memory only)
- `memory/constitution.md` – non‑negotiable project charter and guardrail canon
- `product-requirements.md` – canonical product scope. If missing, pause and ask the user for direction.
- `PLANNING.md` – high-level architecture, goals, constraints. Same rule: request it if absent.
- `SPEC.md` – single source of truth for task tracking; only one `In Progress` row at a time.
- `docs/SPEC-<AREA>-<slug>/` – per-feature specs, plans, tasks. Treat `specs/**` as archival only.
- `AGENTS.md` (this document's partner) – Spec-Kit automation guardrails.

**MANDATORY LOCAL-MEMORY WORKFLOW**:
1. **Session Start**: Query local-memory for project context, recent decisions, architecture state
2. **Before Tasks**: Search local-memory for relevant prior work, patterns, solutions
3. **During Work**: Store key decisions, architecture changes, bug discoveries (importance ≥7)
4. **After Milestones**: Store outcomes, file locations, validation results, lessons learned

**Why Critical**: Local-memory builds living project handbook - captures architecture evolution, decision rationale, debugging solutions. Without it, knowledge is lost between sessions.

**Tool Names**: `mcp__local-memory__search`, `mcp__local-memory__store_memory`, `mcp__local-memory__analysis`

See `MEMORY-POLICY.md` for complete policy. Local-memory is the **only** knowledge persistence system.

## 2. Operating Modes & Slash Commands

### Core Spec-Kit Commands (/speckit.* namespace)

**Intake & Creation:**
- `/speckit.new <description>` – **Native SPEC creation** (Tier 0: zero agents, instant, FREE). Template-based PRD generation, directory creation, SPEC.md updates. <1s, $0.
- `/speckit.specify SPEC-ID [description]` – Draft/update PRD with single-agent analysis (Tier 1: 1 agent - gpt5-low). Strategic PRD refinement. ~3-5 min, ~$0.10.

**Quality Commands (Native Heuristics):**
- `/speckit.clarify SPEC-ID` – **Native ambiguity detection** (Tier 0: zero agents, instant, FREE). Pattern matching for vague language, missing sections, undefined terms. <1s, $0.
- `/speckit.analyze SPEC-ID` – **Native consistency checking** (Tier 0: zero agents, instant, FREE). Structural diff for ID mismatches, coverage gaps, contradictions. <1s, $0.
- `/speckit.checklist SPEC-ID` – **Native quality scoring** (Tier 0: zero agents, instant, FREE). Rubric-based evaluation (completeness, clarity, testability, consistency). <1s, $0.

**Development Stages:**
- `/speckit.plan SPEC-ID [context]` – Multi-agent work breakdown (Tier 2: 3 agents - gemini-flash, claude-haiku, gpt5-medium). Strategic planning with diverse perspectives. ~10-12 min, ~$0.35.
- `/speckit.tasks SPEC-ID` – Single-agent task decomposition (Tier 1: 1 agent - gpt5-low). Structured task breakdown from plan. ~3-5 min, ~$0.10.
- `/speckit.implement SPEC-ID` – Code generation with specialist (Tier 2: 2 agents - gpt_codex HIGH, claude-haiku validator). gpt-5-codex for code, cheap validator. ~8-12 min, ~$0.11.
- `/speckit.validate SPEC-ID` – Test strategy consensus (Tier 2: 3 agents - gemini-flash, claude-haiku, gpt5-medium). Coverage analysis and test planning. ~10-12 min, ~$0.35.
  - **Single-flight guard**: duplicate triggers show `Validate run already active (run_id …)` and do not spawn extra agents; lifecycle telemetry lands under `stage:validate`.
- `/speckit.audit SPEC-ID` – Compliance checking (Tier 3: 3 premium - gemini-pro, claude-sonnet, gpt5-high). Security and compliance validation. ~10-12 min, ~$0.80.
- `/speckit.unlock SPEC-ID` – Final approval (Tier 3: 3 premium - gemini-pro, claude-sonnet, gpt5-high). Ship/no-ship decision. ~10-12 min, ~$0.80.

**Automation:**
- `/speckit.auto SPEC-ID [--skip-STAGE] [--only-STAGE] [--stages=LIST]` – Full 6-stage pipeline with auto-advancement and quality gate checkpoints. Uses strategic agent routing: native for quality, single-agent for simple stages, multi-agent for complex decisions, premium for critical stages. ~45-50 min, **~$2.70** (was $11, 75% reduction via SPEC-KIT-070).
  - **CLI Flags (SPEC-948)**: `--skip-validate`, `--skip-audit`, `--only-plan`, `--stages=plan,tasks,implement`
  - **Precedence**: CLI flags > per-SPEC pipeline.toml > global config > defaults
  - **Cost Savings**: Skip expensive stages ($0.66-$2.70 vs $2.70 full pipeline)

**Diagnostic:**
- `/speckit.status SPEC-ID` – Native TUI dashboard (Tier 0: instant, no agents). Shows stage completion, artifacts, evidence paths. <1s, $0.

### Guardrail Commands (Shell wrappers)

- `/guardrail.plan SPEC-ID` – Baseline + policy checks for plan. Must land telemetry under `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/<SPEC-ID>/`. (note: legacy `/spec-ops-plan` still works)
- `/guardrail.tasks SPEC-ID` – Validation for tasks stage. (note: legacy `/spec-ops-tasks` still works)
- `/guardrail.implement SPEC-ID` – Pre-implementation checks. (note: legacy `/spec-ops-implement` still works)
- `/guardrail.validate SPEC-ID` – Test harness execution. (note: legacy `/spec-ops-validate` still works)
- `/guardrail.audit SPEC-ID` – Compliance scanning. (note: legacy `/spec-ops-audit` still works)
- `/guardrail.unlock SPEC-ID` – Final validation. (note: legacy `/spec-ops-unlock` still works)
- `/guardrail.auto SPEC-ID [--from STAGE]` – Full pipeline wrapper (plan→unlock). Enforces clean tree unless `SPEC_OPS_ALLOW_DIRTY=1`. (note: legacy `/spec-ops-auto` still works)

### Utility Commands

- `/spec-evidence-stats [--spec SPEC-ID]` – Evidence footprint monitoring. Wraps `scripts/spec_ops_004/evidence_stats.sh`. Use after large runs to monitor repo footprint.
- `/spec-consensus SPEC-ID STAGE` – Inspect local-memory consensus artifacts for a given stage.

### Command Usage Examples

**Quick start (new feature):**
```bash
# Create SPEC
/speckit.new Add user authentication with OAuth2

# Quality checks (optional)
/speckit.clarify SPEC-KIT-###
/speckit.analyze SPEC-KIT-###
/speckit.checklist SPEC-KIT-###

# Full automation
/speckit.auto SPEC-KIT-###

# Check status
/speckit.status SPEC-KIT-###
```

**Individual stage workflow:**
```bash
# Manual stage-by-stage
/speckit.plan SPEC-KIT-065
/speckit.tasks SPEC-KIT-065
/speckit.implement SPEC-KIT-065
/speckit.validate SPEC-KIT-065
/speckit.audit SPEC-KIT-065
/speckit.unlock SPEC-KIT-065
```

**Guardrail validation:**
```bash
# Run guardrail checks (separate from multi-agent)
/guardrail.plan SPEC-KIT-065
/guardrail.auto SPEC-KIT-065 --from plan

# Monitor evidence footprint
/spec-evidence-stats --spec SPEC-KIT-065
```

**Partial pipeline workflows (SPEC-948):**
```bash
# Rapid prototyping - skip validation stages ($0.66, ~20 min, 73% cost savings)
/speckit.auto SPEC-KIT-948 --skip-validate --skip-audit --skip-unlock

# Docs-only workflow - just planning and unlock ($1.15, ~15 min, 53% savings)
/speckit.auto SPEC-KIT-948 --stages=specify,plan,unlock

# Code refactoring - skip planning, focus on implementation ($1.06, ~25 min, 57% savings)
/speckit.auto SPEC-KIT-948 --stages=implement,validate,unlock

# Debug single stage - run only plan ($0.35, ~11 min, 86% savings)
/speckit.auto SPEC-KIT-948 --stages=plan
```

**For complete workflow patterns, cost analysis, and decision guidance:**
See `docs/spec-kit/PIPELINE_CONFIGURATION_GUIDE.md` section 6 (Common Workflows) and
`docs/spec-kit/workflow-examples/*.toml` for ready-to-use configuration files.

### Tiered Model Strategy (Updated 2025-11-01, SPEC-KIT-070 Phase 2+3)

**Tier 0: Native Rust** (0 agents, $0, <1s) **EXPANDED**
- `/speckit.new` - Template-based SPEC creation (native)
- `/speckit.clarify` - Ambiguity pattern matching (native heuristics)
- `/speckit.analyze` - Consistency structural diff (native)
- `/speckit.checklist` - Rubric-based scoring (native)
- `/speckit.status` - Status dashboard (native)

**Tier 1: Single Agent** (1 agent: gpt5-low, ~$0.10, 3-5 min) **NEW**
- `/speckit.specify` - PRD drafting (strategic refinement)
- `/speckit.tasks` - Task decomposition (structured breakdown)

**Tier 2: Multi-Agent** (2-3 agents: cheap + gpt5-medium, ~$0.35, 8-12 min) **UPDATED**
- `/speckit.plan` - Architectural planning (3 agents: gemini-flash, claude-haiku, gpt5-medium)
- `/speckit.validate` - Test strategy (3 agents: gemini-flash, claude-haiku, gpt5-medium)
- `/speckit.implement` - Code generation (2 agents: gpt_codex HIGH, claude-haiku validator)

**Tier 3: Premium** (3 premium agents: pro/sonnet/gpt5-high, ~$0.80, 10-12 min)
- `/speckit.audit` - Compliance/security (critical decisions, high reasoning)
- `/speckit.unlock` - Ship decision (quality over cost)

**Tier 4: Full Pipeline** (strategic routing, **~$2.70**, 45-50 min) **75% REDUCTION**
- `/speckit.auto` - Combines all tiers: native quality checks (FREE), single-agent simple stages ($0.10), multi-agent complex ($0.35), premium critical ($0.80)

**Principle**: "Agents for reasoning, NOT transactions" (SPEC-KIT-070)
- Pattern matching → Native Rust (FREE, instant)
- Strategic decisions → Multi-agent consensus (justified cost)
- Code generation → Specialist model (gpt-5-codex)

### Degradation & Fallbacks

If any slash command or CLI is unavailable, degrade gracefully and record which model/step was substituted. If Gemini agent fails (produces empty output), orchestrator continues with 2/3 agents - consensus still valid.

## 3. Telemetry & Evidence Expectations
- Telemetry schema v1: every JSON needs `command`, `specId`, `sessionId`, `timestamp`, `schemaVersion`, `artifacts[]`.
- Stage-specific fields:
  - Plan – `baseline.mode`, `baseline.artifact`, `baseline.status`, `hooks.session.start`.
  - Tasks – `tool.status`.
  - Implement – `lock_status`, `hook_status`.
  - Validate/Audit – `scenarios[{name,status}]` (`passed|failed|skipped`).
  - Unlock – `unlock_status`.
- Enable `SPEC_OPS_TELEMETRY_HAL=1` during HAL smoke tests to capture `hal.summary.{status,failed_checks,artifacts}`. Collect both healthy and degraded runs.
- `/guardrail.auto` (or legacy `/spec-auto`) halts on schema violations or missing artifacts. Investigate immediately.
- Evidence root: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`. Keep it under control with `/spec-evidence-stats`; propose offloading if any single SPEC exceeds 25 MB.

## 4. Deliverable Formats (No Deviations)
### Plans (`docs/SPEC-<id>-<slug>/plan.md`)
```
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
(Spec Kit docs prefer bullets over Markdown tables, but this mapping table stays for acceptance clarity.)

### Tasks (`docs/SPEC-<id>-<slug>/tasks.md` + SPEC.md)
- Update SPEC.md’s Tasks table every time a `/tasks` or `/implement` run changes state. Columns: Order | Task ID | Title | Status | PRD | Branch | PR | Notes. Status ∈ {Backlog, In Progress, In Review, Blocked, Done}.
- On PR open: Status → `In Review`, populate `Branch`.
- On merge: Status → `Done`, fill `PR`, add dated note referencing evidence (tests or files).

## 5. Multi-Agent Expectations
- **Consensus is fully automated** via native MCP integration (ARCH-002, 5.3x faster). All 13 `/speckit.*` commands operational.
- **Agent roster**: Tier 2 uses gemini/claude/code (or gpt_pro for dev stages), Tier 3 adds gpt_codex, Tier 4 dynamically selects 3-5 agents.
- **Degradation handling**: If agent fails, retry up to 3 times (AR-2). If still fails, continue with remaining agents (2/3 consensus still valid).
- **Consensus metadata**: Automatically records `agent`, `version`, `content` in local-memory. Synthesis includes `consensus_ok`, `degraded`, `missing_agents`, `conflicts[]`.
- **Memory System**: Use local-memory MCP exclusively. Byterover deprecated 2025-10-18.
- **Validation**: `/implement` runs `cargo fmt`, `cargo clippy`, build checks, tests before returning.

## 6. Tooling, Hooks, and Tests
- One-time: `bash scripts/setup-hooks.sh` to point Git at `.githooks`.
- Pre-commit (auto): `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace --no-run` (skip with `PRECOMMIT_FAST_TEST=0`), `scripts/doc-structure-validate.sh --mode=templates`.
- Pre-push (mirrors CI): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo build --workspace --all-features` (+ optional targeted test-compiles, skip with `PREPUSH_FAST=0`).
- Always invoke guardrail scripts through `scripts/spec_ops_004/*` using `scripts/env_run.sh` when `.env` exists.
- No secrets, ever. If HAL secrets are required (`HAL_SECRET_KAVEDARR_API_KEY`), ask the user to supply them.

### Building the TUI Binary

**ALWAYS use the build script instead of raw cargo commands:**

```bash
# Default fast build (dev-fast profile)
~/code/build-fast.sh

# Build and run
~/code/build-fast.sh run

# Release build
PROFILE=release ~/code/build-fast.sh

# With build tracing
TRACE_BUILD=1 ~/code/build-fast.sh
```

**DO NOT use:**
- `cargo build -p codex-tui`
- `cargo run -p codex-tui`

The build script handles profile optimization, environment sanitization, and proper target directory configuration. Always direct users to run `~/code/build-fast.sh` for building.

**Workspace reminder:** run Rust commands from `codex-rs/` (example: `cd codex-rs && cargo test -p codex-tui spec_kit`). Update `SPEC_OPS_CARGO_MANIFEST` in guardrail helpers if workspace layout changes.

### Cargo Cleanup & Disk Space Management

Rust build artifacts can consume significant disk space. For cleanup triggers, monitoring commands, and emergency procedures, see **[scripts/CLEANUP.md](scripts/CLEANUP.md)**.

**Quick reference**:
```bash
du -sh ~/code/codex-rs/target          # Check size
cd ~/code/codex-rs && cargo clean      # Full cleanup
CLEAN=1 ~/code/build-fast.sh           # Clean + build
```

## 7. Branch & Git Discipline
- Default branch name is **main**.
- Upstream sync: `git fetch upstream` then `git merge --no-ff --no-commit upstream/main` (see docs/UPSTREAM-SYNC.md).
- Do all work on short-lived feature branches; never commit directly to main.
- Stick to conventional commits: `feat(scope): …`, `fix(scope): …`, `test(scope): …`, `docs(scope): …`.
- Present diffs before applying (unified diff). Ask for approval if touching the constitution or shipping a large patch.
- One atomic commit per task unless a mechanical refactor is needed (split `refactor:` then feature commit).

## 8. When To Pause And Ask
- Missing or ambiguous acceptance criteria.
- Spec requires external services unavailable here.
- Security/privacy implications are unclear.
- Legacy `specs/**` artefact touched—plan migration before editing.
- Large refactor emerges unexpectedly.
- Required reference documents (`product-requirements.md`, `PLANNING.md`, relevant spec files) are absent.

## 9. Memory Workflow

**Policy**: Use **local-memory MCP exclusively** for high-value knowledge. See `MEMORY-POLICY.md` for complete details.

**Purpose**: Build a curated knowledge base of reusable patterns, NOT a complete history archive.

### Core Principle

Store **high-value insights only** (importance ≥8). Quality over quantity.

**Store**:
- Architecture decisions with rationale (WHY, not just what)
- Reusable patterns and non-obvious solutions
- Critical discoveries (rate limits, breaking changes)
- Major milestones with outcomes

**Don't Store**:
- Session summaries (use git commits)
- Progress updates (use SPEC.md)
- Information already in documentation
- Routine operations or transient status

### Quick Reference

```bash
# Session start - retrieve context
mcp__local-memory__search(query="project architecture", limit=10)

# Store high-value insight
mcp__local-memory__store_memory(
  content="Pattern: Use native Rust for deterministic tasks - 10,000x faster than AI consensus",
  domain="infrastructure",
  tags=["type:pattern", "spec:SPEC-KIT-070"],
  importance=9
)
```

### Tags

Use namespaced format: `spec:SPEC-KIT-071`, `type:bug-fix`, `component:routing`

**Domains**: spec-kit, infrastructure, rust, documentation, debugging

**Avoid**: date tags, task IDs, status values (ephemeral, not reusable)

### Importance Scale

| Level | Use For | Frequency |
|-------|---------|-----------|
| 10 | Crisis events, system-breaking discoveries | <5% |
| 9 | Major architecture decisions, critical patterns | 10-15% |
| 8 | Important milestones, valuable solutions | 15-20% |
| <8 | Don't store (use git/docs instead) | - |

## 10. Evidence & Validation Ritual
- Guardrail runs must have a clean tree unless specifically allowed (`SPEC_OPS_ALLOW_DIRTY=1`).
- Capture both success and failure artifacts; `/speckit.auto` includes automatic retry (AR-2, AR-3) but document degradations.
- After `/implement`, run the full validation harness (fmt, clippy, build/tests, doc validators). Attach logs or cite evidence files in local-memory and user reports.
- Evidence growth policy: 25 MB soft limit per SPEC, monitor with `/spec-evidence-stats`. See `docs/spec-kit/evidence-policy.md` for retention/archival.

## 11. Escalate Early
- Claude should explicitly state blockers, degraded guardrails, or missing telemetry.
- When HAL telemetry is missing or malformed, pause and re-run the relevant guardrail command (e.g., `/guardrail.plan`) with `SPEC_OPS_TELEMETRY_HAL=1` after restoring prerequisites. (note: legacy `/spec-ops-*` commands still work)
- For consensus drift (agents missing, conflicting verdicts), re-run the stage or run `/spec-consensus <SPEC-ID> <stage>` and include findings in the report.

Stay inside these guardrails and Claude Code will be a courteous teammate instead of an incident report.
