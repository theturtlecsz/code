# SPEC-P0-TUI2-QUARANTINE: tui2 Quarantine

**Status:** In Progress
**Priority:** P0
**Class:** 0 (Routine — docs, config, scripts; < 15 files)
**Date:** 2026-02-17
**Owner:** Architecture Lead

***

## Problem Statement

`tui2` exists in the workspace as a full member with no differentiation from `tui`. Contributors encounter two TUI crates and cannot immediately determine which is canonical. ADR-002 established that `tui` is primary and `tui2` is an upstream scaffold, but the codebase lacks enforcement:

1. `cargo build` / `cargo test` builds tui2 by default (no `default-members`)
2. No README in `codex-rs/tui2/` explaining its role
3. `docs/SPEC-TUI2-STUBS.md` (promised in ADR-002) was never created
4. No CI or pre-commit guardrail prevents spec-kit code from drifting into tui2
5. `docs/KEY_DOCS.md` has no UI canon entry

This creates **split-brain risk**: a contributor could add spec-kit integration to tui2, duplicating the system and creating an unsustainable maintenance burden.

## Goals

1. Make tui2 non-default in `cargo build` / `cargo test` via `default-members`
2. Add clear "SCAFFOLD ONLY" documentation (README + stub inventory)
3. Prevent spec-kit drift into tui2 with automated guardrails
4. Ensure tui2 remains buildable explicitly and verified in CI
5. Update canonical doc pointers and trackers

## Non-Goals

* Deleting tui2
* Porting spec-kit into tui2
* Removing the `codex-tui` dependency from tui2 (larger scope, separate SPEC)
* Changing the canonical UI decision (tui remains primary)
* Renaming crate identifiers (cosmetic, separate SPEC if desired)

## Acceptance Criteria

| ID   | Criterion                                                                         | Validation                                         |
| ---- | --------------------------------------------------------------------------------- | -------------------------------------------------- |
| AC-1 | `cargo build` (from codex-rs/) does NOT build codex-tui2 by default               | `cargo build 2>&1 \| grep -c codex-tui2` returns 0 |
| AC-2 | `cargo build -p codex-tui2` succeeds                                              | Exit code 0                                        |
| AC-3 | `codex-rs/tui2/README.md` exists with SCAFFOLD-ONLY banner                        | File exists, contains "SCAFFOLD"                   |
| AC-4 | `docs/SPEC-TUI2-STUBS.md` exists and is linked from ADR-002 references            | File exists                                        |
| AC-5 | `scripts/check-tui2-quarantine.sh` passes (no forbidden spec-kit imports in tui2) | Exit code 0                                        |
| AC-6 | Pre-commit hook runs quarantine check on tui2 changes                             | Hook section present                               |
| AC-7 | CI quality-gates workflow includes tui2 quarantine check and explicit build       | Steps present in YAML                              |
| AC-8 | `docs/KEY_DOCS.md` has UI canon entry                                             | Row present                                        |
| AC-9 | `codex-rs/SPEC.md` lists this spec                                                | Row present in Planned table                       |

## Governance

* **Constitution alignment:** "tui is primary; tui2 is upstream scaffold/reference only" (constitution.md line 7)
* **ADR-002 alignment:** "tui2 is non-default and explicitly marked experimental/upstream scaffold" (ADR-002 line 79)
* **Change class:** Class 0 (docs, config, scripts) — auto-merge eligible when attended

## References

* `docs/adr/ADR-002-tui2-purpose-and-future.md`
* `memory/constitution.md` (guardrail: tui primary)
* `docs/VISION.md` (spec-kit lives in tui)
