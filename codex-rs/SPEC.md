# SPEC.md - Codex-RS / Spec-Kit Task Tracking

**Version:** V7 Contract Realignment\
**Last Updated:** 2026-02-17

***

## Doc Precedence Order

When resolving conflicts or ambiguity, documents take precedence in this order:

1. **`codex-rs/SPEC.md`** (this file) - canonical task tracking and active execution status
2. **`docs/PROGRAM.md`** - active 30/60/90 program DAG and phase gates
3. **`docs/VISION.md` + `docs/adr/ADR-005..ADR-012`** - governing product contract
4. **`memory/constitution.md`** - guardrails and operating principles
5. **Individual `docs/SPEC-*` packets** - implementation details for each deliverable

***

## Contract Invariants

These invariants are active for the current epoch and must not be violated.

### Product Contract

* Packet contract is authoritative: `.speckit/packet.yaml` is execution source-of-truth.
* Sacred anchors (`intent_summary`, `success_criteria`) are immutable except via explicit epoch amendment.
* Recap is mandatory before merge/execution shifts: intent, plan, gates, rollback.

### Threading + Merge Safety

* One primary merge train per project.
* Research/review threads never merge directly.
* Unattended mode performs no merges.

### Change Governance

* Class 2 changes may be adopted only at milestone boundaries.
* Class E bypass is allowed only with emergency trigger + snapshot + rollback + immediate notification.
* No silent drift from packet contract.

### Autonomy Quality

* Proposal inbox is ranked and bounded (top-3 default; top-10 discoverable).
* Hysteresis blocks plan churn unless dominance margin and confidence gates are met.
* Self-correction retries build failures before human escalation.
* Template feedback promotions require evidence and explicit approval.

### Constitution Guardrails

* `tui` is primary; `tui2` is scaffold/reference only.
* Avoid second-system effects and parallel rewrites.
* Keep docs, tasks, and validation evidence synchronized.
* Maintain one active `In Progress` row per thread.

***

## Active Tasks

### In Progress

| Spec        | Status      | Owner             | Next Action                                             |
| ----------- | ----------- | ----------------- | ------------------------------------------------------- |
| SPEC-PM-005 | In Progress | Architecture Lead | Implement shared gate classifier + boundary block tests |

### Planned

| Spec                    | Description                                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------------ |
| SPEC-P0-TUI2-QUARANTINE | tui2 quarantine: default-members exclusion, scaffold docs, CI guardrail (P0 maintenance)         |
| SPEC-PM-006             | Packet persistence: durable `.speckit/packet.yaml` + sacred anchor protections + restart restore |
| SPEC-PM-007             | Recap enforcement: hard "Explain before Act" gate across TUI/CLI/headless                        |
| SPEC-PM-008             | Unattended stacking + Morning Brief with strict no-merge semantics                               |
| SPEC-PM-009             | Proposal ranking and pruning (top-3 default, dedupe, archive policy)                             |
| SPEC-PM-010             | Reverse sync: code/packet drift detection with explicit patch proposals                          |
| SPEC-PM-011             | Hysteresis engine: stability bias for plan replacement                                           |
| SPEC-PM-012             | Self-correction: bounded build/test retry before escalation                                      |
| SPEC-PM-013             | Template feedback: promote successful patterns into shared templates                             |

### Deferred / Historical (Not Active)

| Item                                | Status     | Notes                                                                           |
| ----------------------------------- | ---------- | ------------------------------------------------------------------------------- |
| Memvid-first 2026-Q1 program stream | Historical | Superseded by `docs/PROGRAM.md` v2.0.0 active contract                          |
| SPEC-PM-001                         | Deferred   | PRD/design material exists; execution split into PM-005..PM-013 backlog         |
| SPEC-PM-002                         | Deferred   | Bot runner contract remains reference input; not current 30/60/90 critical path |
| SPEC-PM-003                         | Deferred   | Bot system design remains reference input; not current 30/60/90 critical path   |
| SPEC-PM-004                         | Deferred   | TUI PM UX design remains reference input; not current 30/60/90 critical path    |

### Completed (Recent)

| Spec         | Completion Date | Key Deliverables                              |
| ------------ | --------------- | --------------------------------------------- |
| MAINT-930    | 2026-02-01      | Headless gating/parity hardening              |
| SPEC-KIT-983 | 2026-02-01      | Stage-to-agent defaults modal and persistence |
| SPEC-KIT-982 | 2026-01-31      | ACE + maieutic prompt injection pipeline      |
| SPEC-KIT-981 | 2026-01-31      | Config-driven stage-agent mapping defaults    |

### Blocked

| Spec   | Blocker | Unblocks |
| ------ | ------- | -------- |
| (none) | -       | -        |

***

## Program Gates

| Phase            | Gate Criteria                                                      | Status      |
| ---------------- | ------------------------------------------------------------------ | ----------- |
| Trust Foundation | Packet durability + recap gate + class boundary enforcement active | In Progress |
| Autonomous Lab   | Unattended stacking + ranked proposals + reverse sync operational  | Planned     |
| Learning Loop    | Hysteresis + self-correction + template feedback operational       | Planned     |

***

## Quick Reference

### Validation Commands

```bash
cd /home/thetu/code
python3 scripts/doc_lint.py
cd codex-rs
cargo test -p codex-core
cargo test -p codex-tui --lib
```

### Canonical Paths

```text
docs/PROGRAM.md
docs/VISION.md
docs/adr/ADR-005-consultant-ux-and-packet.md
memory/constitution.md
```

***

Maintained as the canonical execution tracker for the active epoch.
