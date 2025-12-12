# Maintainer Answers (Evidence-Based)

## 1) Canonical spec / source of truth

The canonical development tracker is `SPEC.md` (single source of truth for work items). Product identity lives in `docs/VISION.md`. Additional canonical docs are listed in `docs/KEY_DOCS.md`.

Precedence (recommended):
1) `SPEC.md` (what to do / what’s done)
2) `product-requirements.md` + `docs/SPEC-KIT-*/PRD.md` (what to build)
3) `docs/VISION.md` + `memory/constitution.md` (why / principles)

## 2) Fork or independent distribution?

Fork. Evidence: fork-specific markers and upstream sync notes exist in-repo (see `UPSTREAM-SYNC.md`).

## 3) What does “any provider” mean today?

Unknown. Evidence: the codebase contains integrations for multiple providers/tools (e.g., device-code OAuth providers), but a single “provider contract” and explicit support matrix are not consolidated in one canonical doc.

Recommended next step: document supported providers in `docs/CONFIG.md` once confirmed from `codex-rs/*` provider modules and config schema.

## 4) How is “reasoning effort” represented?

Present in code/config types (e.g., `ReasoningEffort` appears in the TUI event model), but Spec-Kit-specific usage is not clearly documented as a requirement.

## 5) Must-not-break workflows

Unknown. Recommended next step: treat `/speckit.*` commands and their tests under `codex-rs/tui/tests/` as the workflow contract.

## 6) Threat model: tool/prompt injection boundaries

Unknown. Recommended next step: add a concise threat model section once the intended sandbox/approvals posture is specified.

## 7) Is Spec-Kit user-facing or internal?

User-facing in this fork: Spec-Kit commands are exposed as `/speckit.*` in the TUI (`codex-rs/tui/src/slash_command.rs`).

## 8) Prompt/template stability guarantees

Unknown. Evidence: templates exist project-local and embedded; stability policy is not stated canonically.

## 9) Telemetry/logging policy

Unknown. Evidence: multiple docs reference evidence/telemetry, but there is no single canonical policy document.

## 10) Are non-OpenAI providers supported directly or via CLIs?

Unknown. Evidence: optional CLI/tool integrations exist, but the supported matrix is not yet consolidated.
