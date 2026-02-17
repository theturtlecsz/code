# SPEC-P0-TUI2-QUARANTINE: Task List

**Date:** 2026-02-17

***

## Tasks

### T001: Add `default-members` to workspace Cargo.toml

* **File:** `codex-rs/Cargo.toml`
* **Edit:** Add `default-members` array after `members`, listing all members except `"tui2"`
* **Effect:** `cargo build` / `cargo test` skip tui2 by default
* **Validation:**
  ```bash
  cd codex-rs && cargo build   # should not compile codex-tui2
  cd codex-rs && cargo build -p codex-tui2   # should still work
  ```
* **Artifacts:** Modified Cargo.toml

### T002: Create `codex-rs/tui2/README.md`

* **File:** `codex-rs/tui2/README.md` (new)
* **Content:** SCAFFOLD-ONLY banner, links to ADR-002, tui as canonical, build instructions
* **Validation:** File exists, contains "SCAFFOLD"
* **Artifacts:** New README.md

### T003: Create `docs/SPEC-TUI2-STUBS.md`

* **File:** `docs/SPEC-TUI2-STUBS.md` (new)
* **Content:** Stub inventory (tui2 vs tui), cherry-pick workflow, out-of-scope list
* **Validation:** File exists, linked from ADR-002 references section
* **Artifacts:** New SPEC-TUI2-STUBS.md

### T004: Add tui2 quarantine guardrail script

* **File:** `scripts/check-tui2-quarantine.sh` (new)
* **Content:** Grep-based check for forbidden patterns in tui2/src/
* **Forbidden patterns:**
  * `chatwidget/spec_kit` (path import)
  * `spec_kit::` (module use)
  * `codex_spec_kit` (crate import)
  * `codex-stage0` / `codex_stage0` (stage0 dep in Cargo.toml)
  * `/speckit\.` (slash command strings, excluding main.rs allowlist)
* **Allowlist:** `tui2/src/main.rs` (contains ADR-002 informational warnings)
* **Validation:**
  ```bash
  bash scripts/check-tui2-quarantine.sh   # exit 0
  ```
* **Artifacts:** New script

### T005: Wire guardrail into pre-commit hook

* **File:** `.githooks/pre-commit`
* **Edit:** Add section after config-isolation check, triggered when tui2 files are staged
* **Validation:** Hook section present
* **Artifacts:** Modified pre-commit

### T006: Add CI step for tui2 quarantine + build check

* **File:** `.github/workflows/quality-gates.yml`
* **Edit:** Add two steps:
  1. "tui2 quarantine check" running the script
  2. "Build tui2 (scaffold)" running `cargo build -p codex-tui2`
* **Validation:** Steps present in YAML
* **Artifacts:** Modified workflow

### T007: Update `docs/KEY_DOCS.md` with UI canon entry

* **File:** `docs/KEY_DOCS.md`
* **Edit:** Add rows for ADR-002 and SPEC-TUI2-STUBS.md with "tui = production, tui2 = scaffold" note
* **Validation:** Rows present
* **Artifacts:** Modified KEY\_DOCS.md

***

## Tracker Updates (post-implementation)

* `codex-rs/SPEC.md`: Add SPEC-P0-TUI2-QUARANTINE to Planned table
* `docs/PROGRAM.md`: Add maintenance note

***

## Completion Checklist

* [ ] T001: default-members
* [ ] T002: tui2 README
* [ ] T003: SPEC-TUI2-STUBS.md
* [ ] T004: quarantine script
* [ ] T005: pre-commit hook
* [ ] T006: CI quality-gates
* [ ] T007: KEY\_DOCS.md
* [ ] Tracker updates
* [ ] All validation commands pass
