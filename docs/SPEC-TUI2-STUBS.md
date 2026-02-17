# SPEC-TUI2-STUBS: tui2 Stub Inventory

**Last Updated:** 2026-02-17
**Related:** [ADR-002](adr/ADR-002-tui2-purpose-and-future.md) | [tui2 README](../codex-rs/tui2/README.md)

***

## Purpose

This document tracks what is present/stubbed/missing in `tui2` compared to `tui`, which features are candidates for cherry-picking, and what is explicitly out of scope.

## Stub Inventory

### Missing in tui2 (vs tui)

| Feature                                             | tui Status                                 | tui2 Status | Notes                        |
| --------------------------------------------------- | ------------------------------------------ | ----------- | ---------------------------- |
| Spec-Kit system                                     | 1.3MB, 60+ files in `chatwidget/spec_kit/` | Not present | **Out of scope permanently** |
| `/speckit.auto`, `/speckit.new`, `/speckit.project` | Fully implemented                          | Missing     | Out of scope                 |
| Agent orchestrator                                  | \~90K LOC                                  | Missing     | Out of scope                 |
| Pipeline coordinator                                | \~86K LOC                                  | Missing     | Out of scope                 |
| Quality gates                                       | Full system                                | Missing     | Out of scope                 |
| Cost tracking                                       | Implemented                                | Missing     | Out of scope                 |
| `/model` selection                                  | Working                                    | Stubbed     | Cherry-pick candidate        |
| Merge train integration                             | Implemented                                | Missing     | Out of scope                 |
| Stage0 overlay engine                               | Full integration                           | Missing     | Out of scope                 |

### Present in tui2 (from upstream)

| Feature                                 | Status  | Cherry-pick Value       |
| --------------------------------------- | ------- | ----------------------- |
| Viewport-based terminal architecture    | Working | High (future)           |
| Frame scheduling / rendering separation | Working | Medium                  |
| Desktop notification behavior           | Working | Medium (good candidate) |
| Keybinding hints, pager overlay         | Working | Low (nice-to-have)      |
| Basic chat loop                         | Working | N/A (already in tui)    |
| Compat shims (`compat.rs`)              | Working | Reference only          |

### Compatibility Layer

tui2 depends on `codex-tui` (see `tui2/Cargo.toml` line 43) for CLI type conversion (`cli.rs:92`). This is an accepted coupling for now; removing it is separate scope.

## Explicitly Out of Scope (Forever)

These MUST NOT be added to tui2. Enforced by CI guardrail (`scripts/check-tui2-quarantine.sh`).

1. **Spec-Kit integration** -- `/speckit.*` commands, `chatwidget/spec_kit/`, pipeline orchestration
2. **Stage0 overlay engine** -- `codex-stage0` / `codex_stage0` imports
3. **Merge train** -- serialized execution, class 2 boundary enforcement
4. **Agent orchestrator** -- multi-agent pipeline execution
5. **Quality gates** -- evidence collection, telemetry, consensus artifacts

**Rationale:** ADR-002 constraint 2: "tui2 must not become an alternate spec-kit implementation." Constitution: "tui is primary; tui2 is upstream scaffold/reference only."

## Cherry-Pick Workflow

To propose cherry-picking a feature from tui2 to tui:

1. **Identify the feature** in the "Present in tui2" table above
2. **Create a SPEC** using `/speckit.new` describing the backport
3. **Implement in tui** (not tui2) -- adapt the upstream pattern to tui's architecture
4. **If shared logic is needed**, extract into a core crate first (ADR-002 constraint 3)
5. **Update this inventory** after the cherry-pick lands

### Decision Criteria

A cherry-pick is worthwhile when:

* The feature adds user-visible value to the production TUI
* The implementation is self-contained (does not require pulling in large upstream subsystems)
* The feature does not conflict with existing tui architecture or spec-kit integration

## Re-evaluation Trigger

Per ADR-002, only revisit "tui2 as replacement" when:

1. Spec-kit/orchestrator is extracted into shared core crates
2. tui2 can consume shared orchestration without duplication
3. tui2 achieves command parity with tui

***

Back to [KEY\_DOCS.md](KEY_DOCS.md)
