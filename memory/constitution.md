# Code Spec-Kit Constitution

## Core Principles

### Evidence-Driven Templates
- Keep acceptance criteria, task mappings, and guardrail docs synchronized
- Every update must keep acceptance criteria, task mappings, and guardrail documentation in sync across SPEC.md, plan/tasks templates, and slash-command guidance.

### Cross-Repo Separation
- Keep tooling in this repo, project configs in product repos
- Shared tooling and templates live in this repository; project-specific configurations, telemetry, and evidence stay inside their respective product repositories.

### Tooling Discipline
- Use MCP/LLM tooling; avoid bespoke shell scripts for runtime operations
- Wrap shell scripts with TUI slash commands for user-facing operations
- Data access and automation must flow through MCP/LLM tooling; avoid bespoke shell scripts for runtime evidence or API calls unless MCP cannot satisfy the requirement.
- All user-facing operations (guardrails, telemetry snapshots, evidence collection) must be invocable via Planner TUI slash commands; add wrappers when a capability is only available as a shell script.

## Governance & Workflow
- Keep SPEC.md canonical; one In Progress entry per thread with dated notes
- Update docs and pass tests when changing templates
- Keep guardrail scripts agent-friendly with model metadata
- SPEC.md is the canonical tracker; keep one `In Progress` entry per active thread and update notes with dated evidence references.
- Template changes require accompanying documentation updates (RESTART.md, docs/slash-commands.md, etc.) and passing `cargo test -p codex-tui spec_auto`.
- Guardrail scripts and prompts must remain agent-friendly: record model metadata, surface telemetry artifacts, and never rely on local state that agents cannot reproduce.

**Version**: 1.1 | **Ratified**: 2025-09-28 | **Last Amended**: 2025-10-26

---

## ACE-Compatible Bullets (≤140 chars)

These short imperatives are extracted by `/speckit.constitution` for ACE playbook injection:

- Keep acceptance criteria, task mappings, and guardrail docs synchronized
- Keep tooling in this repo, project configs in product repos
- Use MCP/LLM tooling; avoid bespoke shell scripts for runtime operations
- Wrap shell scripts with TUI slash commands for user-facing operations
- Keep SPEC.md canonical; one In Progress entry per thread with dated notes
- Update docs and pass tests when changing templates
- Keep guardrail scripts agent-friendly with model metadata

---

Back to [Key Docs](../docs/KEY_DOCS.md)
