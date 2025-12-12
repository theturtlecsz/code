# Architecture

Planner is the `code` binary with an interactive TUI. Spec-Kit is implemented as slash commands under `/speckit.*`.

## High-Level Flow

```
User input (TUI)
  -> slash parsing (codex-rs/tui/src/slash_command.rs)
  -> dispatch (codex-rs/tui/src/app.rs)
  -> Spec-Kit routing/registry (codex-rs/tui/src/chatwidget/spec_kit/)
  -> native pipeline + guardrails + evidence (writes under docs/SPEC-OPS-004-.../evidence/)
  -> shared Spec-Kit crate (codex-rs/spec-kit/) for config/retry/types
```

## Key Boundaries

- UX + orchestration: `codex-rs/tui/src/chatwidget/spec_kit/`
- Shared library: `codex-rs/spec-kit/`
- Templates: `./templates/` (project-local) + embedded fallbacks

