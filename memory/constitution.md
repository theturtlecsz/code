# Code Spec-Kit Constitution

## Guardrails

Hard constraints that must never be violated:

- tui is primary; tui2 is upstream scaffold/reference only (cherry-pick, never wholesale replace)
- No duplicate features across localmemory-policy / notebooklm-mcp / ~/code
- Keep tooling in this repo, project configs in product repos
- Shared tooling and templates live in this repository; project-specific configurations, telemetry, and evidence stay inside their respective product repositories.
- Keep SPEC.md canonical; one In Progress entry per thread with dated notes
- Template changes require accompanying documentation updates and passing tests

## Principles

Architectural values and design principles:

- Documents → tests → implementation (evidence-driven)
- Keep acceptance criteria, task mappings, and guardrail docs synchronized
- Every update must keep acceptance criteria, task mappings, and guardrail documentation in sync across SPEC.md, plan/tasks templates, and slash-command guidance.
- Use MCP/LLM tooling; avoid bespoke shell scripts for runtime operations
- Wrap shell scripts with TUI slash commands for user-facing operations
- Data access and automation must flow through MCP/LLM tooling; avoid bespoke shell scripts for runtime evidence or API calls unless MCP cannot satisfy the requirement.
- All user-facing operations (guardrails, telemetry snapshots, evidence collection) must be invocable via Planner TUI slash commands.
- Guardrail scripts and prompts must remain agent-friendly: record model metadata, surface telemetry artifacts, and never rely on local state that agents cannot reproduce.

## Goals

Project objectives:

- Make /speckit.auto workflow generic for any project, not just ~/code
- Dogfood ~/code to validate the system end-to-end
- Enable Tier2 (NotebookLM) by default with fail-closed semantics
- Provide consistent configuration and clean boundaries across 3 repos
- Good logging and traceability (system pointers, evidence artifacts)

## Non-Goals

What we explicitly don't build:

- No MCP-based local-memory access (CLI + REST only per localmemory-policy)
- No automatic overwrites of memory/constitution.md (import only, sync writes NL_CONSTITUTION.md)
- No monolithic mega-repo; strict boundaries between ~/code, localmemory-policy, notebooklm-mcp

---

**Version**: 2.0 | **Ratified**: 2025-12-25 | **Last Amended**: 2025-12-25

---

## ACE-Compatible Bullets (≤140 chars)

These short imperatives are extracted by `/speckit.constitution` for ACE playbook injection:

- tui is primary; tui2 is upstream scaffold/reference only
- No duplicate features across localmemory-policy / notebooklm-mcp / ~/code
- Keep acceptance criteria, task mappings, and guardrail docs synchronized
- Keep tooling in this repo, project configs in product repos
- Use MCP/LLM tooling; avoid bespoke shell scripts for runtime operations
- Wrap shell scripts with TUI slash commands for user-facing operations
- Keep SPEC.md canonical; one In Progress entry per thread with dated notes
- Update docs and pass tests when changing templates
- Keep guardrail scripts agent-friendly with model metadata

---

Back to [Key Docs](../docs/KEY_DOCS.md)
