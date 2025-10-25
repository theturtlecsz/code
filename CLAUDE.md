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
- `/speckit.new <description>` – Create new SPEC with multi-agent PRD consensus (Tier 2: 3 agents - gemini, claude, code). Uses templates for consistent structure. ~13 min, ~$0.60.
- `/speckit.specify SPEC-ID [description]` – Draft/update PRD with multi-agent analysis (Tier 2: 3 agents - gemini, claude, code). ~10-12 min, ~$0.80.

**Quality Commands:**
- `/speckit.clarify SPEC-ID` – Structured ambiguity resolution (Tier 2: 3 agents - gemini, claude, code). Identifies unclear requirements. ~8-10 min, ~$0.80.
- `/speckit.analyze SPEC-ID` – Cross-artifact consistency checking with auto-fix (Tier 2: 3 agents - gemini, claude, code). ~8-10 min, ~$0.80.
- `/speckit.checklist SPEC-ID` – Requirement quality scoring (Tier 2-lite: 2 agents - claude, code). ~5-8 min, ~$0.35.

**Development Stages:**
- `/speckit.plan SPEC-ID [context]` – Multi-agent work breakdown (Tier 2: 3 agents - gemini, claude, gpt_pro). ~10-12 min, ~$1.00.
- `/speckit.tasks SPEC-ID` – Task decomposition with consensus (Tier 2: 3 agents - gemini, claude, gpt_pro). ~10-12 min, ~$1.00.
- `/speckit.implement SPEC-ID` – Code generation + validation (Tier 3: 4 agents - gemini, claude, gpt_codex, gpt_pro). ~15-20 min, ~$2.00.
- `/speckit.validate SPEC-ID` – Test strategy consensus (Tier 2: 3 agents - gemini, claude, gpt_pro). ~10-12 min, ~$1.00.
  - **Single-flight guard**: duplicate triggers show `Validate run already active (run_id …)` and do not spawn extra agents; lifecycle telemetry lands under `stage:validate`.
- `/speckit.audit SPEC-ID` – Compliance checking (Tier 2: 3 agents - gemini, claude, gpt_pro). ~10-12 min, ~$1.00.
- `/speckit.unlock SPEC-ID` – Final approval (Tier 2: 3 agents - gemini, claude, gpt_pro). ~10-12 min, ~$1.00.

**Automation:**
- `/speckit.auto SPEC-ID` – Full 6-stage pipeline with auto-advancement (Tier 4: dynamic 3-5 agents, uses Tier 2 for most stages, Tier 3 for implement, adds arbiter if conflicts). ~60 min, ~$11.

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

### Legacy Commands (Backward Compatible)

**Deprecated but still functional (will be removed in future release):**
- `/new-spec` → use `/speckit.new`
- `/spec-plan` → use `/speckit.plan`
- `/spec-tasks` → use `/speckit.tasks`
- `/spec-implement` → use `/speckit.implement`
- `/spec-validate` → use `/speckit.validate`
- `/spec-audit` → use `/speckit.audit`
- `/spec-unlock` → use `/speckit.unlock`
- `/spec-auto` → use `/speckit.auto`
- `/spec-status` → use `/speckit.status`

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

### Tiered Model Strategy

**Tier 0: Native TUI** (0 agents, $0, <1s)
- `/speckit.status` - Pure Rust implementation

**Tier 1: Single Agent** (1 agent: code, ~$0.10, 1-3 min)
- Future optimization for deterministic scaffolding

**Tier 2-lite: Dual Agent** (2 agents: claude, code, ~$0.35, 5-8 min)
- `/speckit.checklist` - Quality evaluation without research

**Tier 2: Triple Agent** (3 agents: gemini, claude, code/gpt_pro, ~$0.80-1.00, 8-12 min)
- `/speckit.new`, `/speckit.specify`, `/speckit.clarify`, `/speckit.analyze`
- `/speckit.plan`, `/speckit.tasks`, `/speckit.validate`, `/speckit.audit`, `/speckit.unlock`
- Use for analysis, planning, consensus (no code generation)

**Tier 3: Quad Agent** (4 agents: gemini, claude, gpt_codex, gpt_pro, ~$2.00, 15-20 min)
- `/speckit.implement` only - Code generation + validation

**Tier 4: Dynamic** (3-5 agents adaptively, ~$11, 60 min)
- `/speckit.auto` - Uses Tier 2 for most stages, Tier 3 for implement, adds arbiter if conflicts

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

**Workspace reminder:** run Rust commands from `codex-rs/` (example: `cd codex-rs && cargo test -p codex-tui spec_kit`). Update `SPEC_OPS_CARGO_MANIFEST` in guardrail helpers if workspace layout changes.

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

## 9. Memory Workflow - Curated Knowledge Base

**POLICY**: Use **local-memory MCP exclusively** for high-value knowledge. See `MEMORY-POLICY.md` for complete details.

**Purpose**: Build a curated knowledge base of reusable patterns and living project handbook, NOT a complete history archive.

**Note**: Consensus artifacts (agent outputs, structured data) will migrate to separate database (SPEC-KIT-072). Local-memory is for **human-curated insights only**.

---

### Session Workflow

**1. Session Start** (REQUIRED):
```
Use mcp__local-memory__search:
- query: "project architecture recent changes"
- limit: 10
- search_type: "semantic"
```
Retrieves recent architecture decisions, bug fixes, patterns.

**2. Before Major Tasks** (REQUIRED):
```
Use mcp__local-memory__search:
- query: "test coverage phase 3 integration"
- tags: ["testing", "spec-kit"]
- limit: 5
```
Search for relevant prior work to avoid repeating research.

**3. During Work** (Store importance ≥8 ONLY):
```
Use mcp__local-memory__store_memory:
- content: "Routing bug fixed: SpecKitCommand wasn't passing config. Root cause: routing.rs line 45 passed None instead of actual config. Solution: Pass widget.config to format_subagent_command(). Pattern: Always verify config propagation in command chains."
- domain: "debugging"
- tags: ["type:bug-fix", "spec:SPEC-KIT-066", "component:routing"]
- importance: 9
```

**What to Store** (importance ≥8):
- 🏗️ Architecture decisions with rationale (why, not just what)
- 🔧 Reusable patterns and code examples
- 🚨 Critical discoveries (rate limits, cost crisis, system-breaking)
- 🐛 Non-obvious bug fixes with context
- ⚠️ Important limitations and workarounds
- ✅ Major milestones with outcomes

**What NOT to Store**:
- ❌ Session summaries (use git commits + SPEC.md instead)
- ❌ Progress updates (use SPEC.md task tracker)
- ❌ Information already in documentation (link to it instead)
- ❌ Routine operations (normal workflow)
- ❌ Transient status ("in progress", "blocked")
- ❌ Low-value observations (importance <8)
- ❌ Consensus artifacts (will use separate DB, SPEC-KIT-072)

**4. After Milestones** (Store importance ≥8):
```
Use mcp__local-memory__store_memory:
- content: "Test coverage Phase 3 complete: Added 60 integration tests (workflow, error recovery, state persistence, quality gates, concurrent ops). Total: 555 tests, 100% pass rate. Estimated coverage: 38-42% (exceeded 40% target). Pattern: IntegrationTestContext harness enables complex multi-module testing. Files: workflow_integration_tests.rs, error_recovery_integration_tests.rs"
- domain: "infrastructure"
- tags: ["type:milestone", "testing", "phase-3"]
- importance: 8
```

**5. Session End** (OPTIONAL - only if exceptional):

Store session summary ONLY if:
- Major breakthrough or discovery (rate limits, architectural insight)
- Multi-day work requiring detailed handoff context
- Critical decisions NOT captured in individual memories

Otherwise: Individual memories + git commits + SPEC.md are sufficient.

**If storing** (rare):
```
Use mcp__local-memory__store_memory:
- content: "Discovered OpenAI rate limit crisis validates SPEC-KIT-070 urgency. Hit limits during testing (1d 1h block). Changed strategy to prioritize provider diversity and aggressive cost reduction. Deployed Claude Haiku (12x cheaper), Gemini Flash (12.5x cheaper), native SPEC-ID ($0). Impact: 40-50% cost reduction ready for validation."
- domain: "infrastructure"
- tags: ["type:discovery", "spec:SPEC-KIT-070", "priority:critical"]
- importance: 10
```

---

### Tag Schema (Guided, Flexible)

**Namespaced Format** (use when applicable):
```
spec:<SPEC-ID>          Example: spec:SPEC-KIT-071
type:<category>         Example: type:bug-fix, type:pattern, type:discovery
project:<name>          Example: project:codex-rs, project:kavedarr
component:<area>        Example: component:routing, component:consensus
```

**Domain Structure** (5 primary domains):
```
spec-kit        Spec-kit automation, consensus, multi-agent workflows
infrastructure  Cost, testing, architecture, CI/CD, performance
rust            Language patterns, borrow checker, cargo, performance
documentation   Doc strategy, templates, writing, guides
debugging       Bug fixes, error patterns, workarounds, troubleshooting
```

**General Tags** (~30-50 approved, can add new if justified):
```
Core: testing, mcp, consensus, evidence, telemetry
Concepts: cost-optimization, quality-gates, rebase-safety
Tools: borrow-checker, native-tools
```

**FORBIDDEN Tags** (auto-reject):
```
❌ Specific dates: 2025-10-20, 2025-10-14 (use date filters instead)
❌ Task IDs: t84, T12, t21 (ephemeral, not useful long-term)
❌ Status values: in-progress, blocked, done, complete (changes over time)
❌ Overly specific: 52-lines-removed, policy-final-check (not reusable)
```

**Tag Reuse**: Check existing tags before creating new. Consolidate duplicates quarterly.

---

### Importance Calibration (CRITICAL)

**Use this guide STRICTLY** to prevent inflation:

```
10: Crisis events, system-breaking discoveries
    - Rate limit discovery blocking operations
    - Critical architecture flaws found
    - Security vulnerabilities discovered
    - USE SPARINGLY: <5% of stores

9:  Major architectural decisions, critical patterns
    - Borrow checker workarounds for complex scenarios
    - Cost optimization strategies ($6,500/year savings)
    - Significant refactors (handler.rs extraction)
    - ~10-15% of stores

8:  Important milestones, valuable solutions
    - Phase completions with evidence
    - Non-obvious bug fixes with context
    - Reusable code patterns
    - ~15-20% of stores

7:  Useful context, good reference
    - Configuration changes with rationale
    - Minor optimizations
    - RARELY STORE (use docs/git instead)
    - ~10-15% of stores

6 and below:
    - DON'T STORE to local-memory
    - Use git commits, SPEC.md, or documentation instead
```

**Threshold**: Store ONLY importance ≥8 (not ≥7)
**Target Average**: 8.5-9.0 (quality-focused)
**Current Average**: 7.88 (too low, indicates over-storage at 7)

---

### Storage Examples

**GOOD Example ✅** (importance: 9):
```
content: "Native SPEC-ID generation eliminates $2.40 consensus cost per /speckit.new. Implementation: spec_id_generator.rs scans docs/, finds max ID, increments. Pattern: Use native Rust for deterministic tasks - 10,000x faster, FREE, more reliable than AI consensus. Applies to: file operations, ID generation, formatting, validation."

domain: "infrastructure"
tags: ["type:pattern", "spec:SPEC-KIT-070", "cost-optimization", "native-tools"]
importance: 9

Why Good:
- Captures WHY (pattern: native > AI for deterministic)
- Includes HOW (implementation detail)
- Generalizable (applies beyond this case)
- Proper tags (namespaced, meaningful, no dates)
- Justified importance (major pattern = 9)
```

**BAD Example ❌** (DON'T STORE):
```
content: "Session 2025-10-24: Did work on SPEC-069 and SPEC-070. Made progress. Tests passing."

domain: "session-summary"
tags: ["2025-10-24", "session-complete", "done"]
importance: 9

Why Bad:
- Redundant (git commits already capture this)
- Vague (no actionable insights)
- Date tag (useless for retrieval)
- Status tags (ephemeral)
- Wrong importance (routine session ≠ 9)
- Wrong domain (session-summary will be deprecated)
- No WHY (doesn't explain decisions)
```

**BETTER** (if session truly exceptional):
```
content: "Discovered CLAUDE.md documentation causing memory bloat through flawed guidance. Root cause: Requires session summaries (redundant), threshold ≥7 too low (inflation), date tags in examples (proliferation). Fixed by updating to ≥8 threshold, optional summaries, tag schema. Pattern: Question the documentation itself when system exhibits emergent problems."

domain: "infrastructure"
tags: ["type:discovery", "spec:SPEC-KIT-071", "priority:critical", "meta-learning"]
importance: 10

Why Better:
- Captures specific insight (docs drive bloat)
- Includes solution (how we fixed it)
- Meta-pattern (question documentation)
- No date tags (timeless insight)
- Justified importance (critical discovery = 10)
```

---

### Why This Matters

**Curated Knowledge Base**:
- ✅ High-value patterns and decisions ONLY
- ✅ Reusable insights (not one-time info)
- ✅ Findable (clean tags, proper domains)
- ✅ Scalable (quality > quantity)

**Living Project Handbook**:
- ✅ Current understanding of architecture
- ✅ Active SPEC knowledge
- ✅ Critical context for contributors
- ✅ Evolves with project (outdated info removed)

**Sustainable Growth**:
- ~40-60 stores/month (≥8 threshold)
- Quarterly cleanup (stay at 120-150 target)
- Consensus artifacts separate (SPEC-KIT-072)

**Deprecated**: byterover-mcp is no longer used (migration complete 2025-10-18).

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
