# Rolling Decisions Log

This file is a canonical source for NotebookLM Tier2. It captures major decisions,
ADRs, and milestone markers that inform the Divine Truth synthesis.

**Update Policy:**
- Append new decisions when: merging significant PRs, closing SPECs, or making architectural decisions
- Look for markers like: "Decision:", "ADR:", "SYNC-### closed", merge commits
- Keep entries concise (1-3 sentences each)
- Do NOT create new NotebookLM sources per decision; append here instead

---

## 2025-12-25: Session 15 - Dogfooding Setup

**Decision:** Route `/speckit.auto` to native pipeline handler instead of legacy subagent format.
- Root cause: `format_subagent_command("spec-auto")` fell back to ALL 18 agents when no config existed.
- Fix: `ProcessedCommand::SpecAuto` now calls `handle_spec_auto_command()` directly (GR-001 compliant).

**Decision:** Stage0 config path resolution prefers `~/.config/code/` over `~/.config/codex/`.
- `CODE_STAGE0_CONFIG` env var (new, preferred)
- `CODEX_STAGE0_CONFIG` env var (legacy fallback)
- `~/.config/code/stage0.toml` (preferred path)
- `~/.config/codex/stage0.toml` (legacy fallback)

**Decision:** Constitution import from `memory/constitution.md` via `/speckit.constitution import`.
- Populates overlay DB from existing markdown
- Does NOT overwrite source file
- Enables warning-free `/speckit.auto` runs

**Decision:** Remove premature quality gate triggering from `history_push()` function.
- Root cause: Every `history_push` call during `QualityGateExecuting` phase triggered the broker.
- Fix: Removed lines 3972-3987 from `mod.rs`. Broker now only triggered by `QualityGateNativeAgentsComplete` event.
- Result: Quality gates wait for agents to complete before running broker collection.

---

## Template for New Entries

```markdown
## YYYY-MM-DD: [Session/Context]

**Decision:** [Brief description of what was decided]
- [Why this decision was made]
- [Key implications or follow-ups]
```
