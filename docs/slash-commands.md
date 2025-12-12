# Slash Commands

Planner supports many TUI slash commands, but the **product focus is Spec-Kit**.

For removed legacy commands, see `docs/DEPRECATIONS.md`.

## Spec-Kit (Primary)

Common flow:

- `/speckit.project <type> <name>` (optional)
- `/speckit.new <description>`
- `/speckit.auto SPEC-KIT-###`

Stage commands:

- `/speckit.plan SPEC-KIT-###`
- `/speckit.tasks SPEC-KIT-###`
- `/speckit.implement SPEC-KIT-###`
- `/speckit.validate SPEC-KIT-###`
- `/speckit.audit SPEC-KIT-###`
- `/speckit.unlock SPEC-KIT-###`

Quality commands:

- `/speckit.clarify SPEC-KIT-###`
- `/speckit.analyze SPEC-KIT-###`
- `/speckit.checklist SPEC-KIT-###`

Utilities:

- `/speckit.status SPEC-KIT-###`
- `/speckit.configure SPEC-KIT-###`
- `/speckit.constitution`
- `/speckit.verify SPEC-KIT-###`
- `/speckit.install-templates`
- `/speckit.template-status`

## Guardrails

- `/guardrail.plan SPEC-KIT-###`
- `/guardrail.tasks SPEC-KIT-###`
- `/guardrail.implement SPEC-KIT-###`
- `/guardrail.validate SPEC-KIT-###`
- `/guardrail.audit SPEC-KIT-###`
- `/guardrail.unlock SPEC-KIT-###`

## Evidence

- `/spec-evidence-stats [--spec SPEC-KIT-###]`

## Source of Truth

The authoritative slash command list for the UI popup is `codex-rs/tui/src/slash_command.rs`.

