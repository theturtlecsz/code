# Branch Brief: feat/tui2-quarantine

## Focus

Quarantine tui2 to reduce contributor confusion: exclude from default builds, add scaffold docs, wire CI guardrail.

## Scope

* Add `default-members` to workspace Cargo.toml (exclude tui2)
* Create tui2/README.md with SCAFFOLD-ONLY banner
* Create docs/SPEC-TUI2-STUBS.md (stub inventory, cherry-pick workflow)
* Add quarantine guardrail script + pre-commit + CI integration
* Update KEY\_DOCS.md, SPEC.md, PROGRAM.md trackers
* Spec packet: docs/SPEC-P0-TUI2-QUARANTINE/

## Constraints

* Do NOT delete tui2
* Do NOT port spec-kit into tui2
* Class 0 changes only (docs, config, scripts)
* tui2 must remain buildable via `cargo build -p codex-tui2`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->

* Capsule checkpoint: mv2://feat-tui2-quarantine/2026-02-17
* Refreshed: 2026-02-17T20:15:00Z

<!-- END: SPECKIT_BRIEF_REFRESH -->
