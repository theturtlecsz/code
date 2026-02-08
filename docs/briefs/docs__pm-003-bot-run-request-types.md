# Session Brief — docs/pm-003-bot-run-request-types

**Date**: 2026-02-08

## Goal

Consolidate the PM bot-system design into `SPEC-PM-003` and add a minimal, IPC-friendly `codex-core` type definition for bot runs (`BotRunRequest`) that references the spec contracts.

## Scope

- Docs:
  - Expand `docs/SPEC-PM-003-bot-system/spec.md` to capture locked constraints and the architecture option space (ephemeral CLI runner baseline).
  - Add a NotebookLM dev-notebook “decision register” roadmap note for Spec‑Kit development.
- Code:
  - Add `codex-rs/core/src/pm/bot.rs` with `BotRunRequest` + enums + validation (research cannot request worktree writes).
  - Export via `codex-rs/core/src/pm/mod.rs` and `codex-rs/core/src/lib.rs`.

## Non-Goals

- Implementing the runner/service, IPC, queue, or Linear bridge.
- Changing `SPEC-PM-002` caller-facing semantics beyond cross-links.

## Validation

- `python3 scripts/doc_lint.py`
- `cargo fmt --all -- --check` (from `codex-rs/`)
- `cargo clippy -p codex-core --all-targets --all-features -- -D warnings` (from `codex-rs/`)
- `cargo test -p codex-core` (from `codex-rs/`)

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-003 bot system runner BotRunRequest`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260208T183702Z/artifact/briefs/docs__pm-003-bot-run-request-types/20260208T183702Z.md`
- Capsule checkpoint: `brief-docs__pm-003-bot-run-request-types-20260208T183702Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
