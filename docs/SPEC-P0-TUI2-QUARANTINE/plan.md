# SPEC-P0-TUI2-QUARANTINE: Rollout Plan

**Date:** 2026-02-17

***

## Rollout Strategy

Single-PR, single-branch deployment. All changes are Class 0 (docs + config + scripts) and fully reversible.

### Ordering

1. Spec packet committed first (this directory)
2. `default-members` added to `Cargo.toml`
3. Documentation created (README, stubs, KEY\_DOCS update)
4. Guardrail script created and wired (pre-commit + CI)
5. Trackers updated (SPEC.md, PROGRAM.md)

### Reversibility

Every change can be reverted independently:

* `default-members` removal restores old behavior instantly
* Documentation files are additive (delete to revert)
* Guardrail script removal + hook/CI edits to revert
* Tracker entries can be moved to "Deferred"

## Risks & Mitigations

| Risk                                                           | Likelihood | Impact | Mitigation                                                                               |
| -------------------------------------------------------------- | ---------- | ------ | ---------------------------------------------------------------------------------------- |
| `default-members` breaks existing dev workflows                | Low        | Medium | `build-fast.sh` uses explicit `-p` flags, unaffected. CI targets specific packages.      |
| Quarantine grep produces false positives                       | Low        | Low    | Allowlist for known-safe references (ADR-002 warning in main.rs). Patterns are specific. |
| Future upstream merges introduce spec-kit code into tui2       | Medium     | High   | Guardrail catches this in pre-commit and CI. Visible, fast feedback.                     |
| Removing tui2 from default build hides compilation regressions | Medium     | Medium | CI quality-gates explicitly builds tui2 as a dedicated step.                             |
| Contributors bypass pre-commit with --no-verify                | Medium     | Low    | CI is the hard gate; pre-commit is convenience only.                                     |

## Dependencies

None. This spec has no dependencies on other active SPECs.

## Success Measurement

* Zero contributor confusion reports after merge
* CI passes on first attempt
* No spec-kit symbols found in tui2 by guardrail
* `cargo build` (default) no longer compiles tui2 artifacts
