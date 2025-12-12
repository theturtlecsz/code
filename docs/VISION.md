# Planner Vision (Project Truth)

Planner is a terminal TUI focused on **Spec-Kit workflows**.

This document is the canonical “what is this repo” reference; when in doubt, treat **runtime behavior** as truth and update docs to match.

## Product Surface Area (from code)

- **Primary binary name**: `code` (`codex-rs/cli/Cargo.toml`)
- **Primary UX**: interactive TUI (default `code` behavior; no subcommand)
- **Primary workflow contract**: Spec-Kit slash commands under the `/speckit.*` namespace
- **Deprecated legacy UX**: `/plan`, `/solve`, `/code` are removed; invoking them shows a migration message

## Where Spec-Kit Lives

- **TUI integration (slash routing, pipeline orchestration, UI)**: `codex-rs/tui/src/chatwidget/spec_kit/`
- **Shared Spec-Kit library crate (config/retry/types)**: `codex-rs/spec-kit/`
- **Templates**: project-local `./templates/*.md` (optional) plus embedded fallbacks
- **Evidence storage**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/` (guardrails/telemetry/consensus artifacts)

## Canonical Invocation

- Build and run locally via `./build-fast.sh run`
- Use `/speckit.project` (optional) to scaffold a new project, then `/speckit.new` + `/speckit.auto` for end-to-end runs

