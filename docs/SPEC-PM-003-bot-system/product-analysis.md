# PM Product Analysis: "PM is Spec Management" (SPEC-PM-001)

**Session**: `98a1f086-4aa9-4272-be3d-12725e45fa74`
**Date**: 2026-02-06
**Source**: Claude Code plan file `temporal-seeking-giraffe.md`

*Product thinking document -- not an implementation plan.*

***

## 1. Core Insight Analysis

**The claim**: Product management *is* spec management. The activities of deciding what to build, tracking progress, managing lifecycle, and maintaining legibility are not a separate discipline layered on top of spec-kit -- they are the same activity already being performed, just without first-class tooling.

**What this collapses** (previously separate concerns become one):

* "Tracker document" vs "SoR" -- currently `SPEC.md` is manually authoritative but capsule is the execution SoR; they can drift.
* "PRD creation" vs "spec creation" -- you can create a SPEC (`/speckit.new`) and separately write a PRD, and they may not be linked.
* "Deprecation management" vs "lifecycle management" -- `DEPRECATIONS.md` is a separate register; it would become a lifecycle state in the work-item registry.

**What it does NOT collapse (and should not)**:

* **Prioritization decisions** -- the human decision of what to work on next (PRD correctly excludes this).
* **External project management** -- GitHub Issues/Projects.
* **Capsule execution semantics** -- run-level branching, checkpoints, merge-at-unlock. These are execution concerns; PM sits above them.

**Assessment**: The insight is **substantially correct** for a solo dev-PM. The unification holds cleanly for feature lifecycle tracking and document lifecycle tracking. It holds partially for PRD creation (maieutic intake captures structure, but rich narrative PRDs go beyond). The caveat: the claim risks being aspirational if "PM" is interpreted broadly (portfolio prioritization, resource allocation, stakeholder communication). For a solo dev, those are minimal, so the collapse is pragmatically valid.

***

## 2. Jobs-to-Be-Done

**Functional job**: "When I want to understand the current state of my project -- what is planned, in progress, done, deprecated -- I need to look at a single, trustworthy source that I do not have to manually synchronize, so I can make decisions about what to work on next without fighting document drift."

**Emotional job**: "I want the engineering rigor of my execution pipeline (capsule, events, immutable URIs, deterministic projections) to extend to the planning layer too. SPEC.md being manually maintained feels like a hole in an otherwise tight system."

**Social job**: "I want a legible audit trail of what I decided to build, why, and when things changed. This serves future-me (session handoff), AI agents (that need project state), and any future collaborators."

**Competing alternatives (today)**:

* `SPEC.md` edited by hand or by a dumb row-insertion function (`update_spec_tracker`, \~55 lines)
* `DEPRECATIONS.md` as a separate manually-maintained register
* PRDs as ad-hoc markdown in `docs/SPEC-*/` -- may or may not link to tracker entries
* `docs/PROGRAM.md` pins active work for a quarter; manually updated, can drift
* Capsule knows about *execution* events but has no concept of "work item created" or "status changed"
* Pain example: the 2026-02-05 deprecation cleanup required manually deprecating \~40 legacy PRDs and creating zip archives -- grunt work that should have been a state transition + capsule event

***

## 3. Opportunity Solution Tree

```
ROOT: All project state queryable from one SoR,
      rendered into human-readable projections,
      with zero manual synchronization.

+-- OPP 1: SPEC.md drifts from actual project state
|   +-- 1a: SPEC.md tables as deterministic projection from capsule work-item events
|   |   [Partially addressed: projections rebuild exists for docs/SPEC-*; SPEC.md is NOT yet a projection]
|   +-- 1b: Pre-commit lint verifying SPEC.md matches capsule state
|   +-- 1c: Real-time projection (capsule event triggers regeneration)
|
+-- OPP 2: No structured lifecycle for work items beyond "create"
|   +-- 2a: WorkItemCreated / StatusChanged / Deprecated capsule event types
|   +-- 2b: CRUD CLI/TUI commands (create/update/deprecate/archive)
|   |   [Partially addressed: /speckit.new creates; no update/deprecate/archive]
|   +-- 2c: Auto-infer status from pipeline events
|       (IntakeCompleted->Planned, first StageTransition->In Progress, BranchMerged->Completed)
|
+-- OPP 3: PRDs disconnected from spec lifecycle
|   +-- 3a: Maieutic PRD session emitting capsule artifact linked to work-item ID
|   |   [Partially addressed: PRD builder exists but produces filesystem files, not capsule artifacts]
|   +-- 3b: PRD-as-capsule-artifact with stable mv2:// URI
|
+-- OPP 4: Deprecation/archival is manual grunt work
|   +-- 4a: Deprecation as lifecycle state + capsule event -> auto-projection to DEPRECATIONS.md
|   +-- 4b: Archive packs as capsule export artifacts with canonical URIs
|   |   [Partially addressed: capsule export exists; not linked to deprecation workflow]
|   +-- 4c: Keep DEPRECATIONS.md manual (defer)
|
+-- OPP 5: Status visibility requires opening documents
    +-- 5a: CLI/TUI status querying capsule work-item registry
    |   [Partially addressed: /speckit.status reads filesystem, not capsule]
    +-- 5b: Headless status with structured JSON for CI/automation
    +-- 5c: Dashboard widget in TUI
```

***

## 4. Assumption Mapping

| #      | Assumption                                                                                                                                                      | Importance | Uncertainty | Risk          |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | :--------: | :---------: | ------------- |
| A1     | Capsule event log is the right home for work-item lifecycle events (not a separate DB)                                                                          |      H     |      M      | Medium        |
| **A2** | **Deterministic projection from capsule -> SPEC.md is feasible without losing hand-authored content (invariants, gating chains, phase gates, quick reference)** |    **H**   |    **H**    | **RISKY BET** |
| **A3** | **A solo dev-PM will actually use lifecycle commands (`/speckit.deprecate`, `/speckit.archive`) instead of just editing markdown**                              |    **H**   |    **H**    | **RISKY BET** |
| A4     | Current capsule event model can extend with PM events without schema conflicts or perf degradation                                                              |      M     |      L      | Low           |
| **A5** | **Auto-inferring status from pipeline events is reliable enough to be default**                                                                                 |    **M**   |    **H**    | **RISKY BET** |
| A6     | Work-item schema can be simple (feature/spec/task + lifecycle state) without rich relationships                                                                 |      H     |      M      | Medium        |
| A7     | DEPRECATIONS.md can eventually be a projection                                                                                                                  |      L     |      M      | Low           |

### How to test risky bets cheaply:

**A2**: Walk through `codex-rs/SPEC.md` and tag each section as "registry" (derivable from events) vs "authored" (human prose). The file is 233 lines; only the Active Tasks / Completed / Blocked tables are registry data. Everything else is hand-authored. If the projection model must be "regenerate tables + passthrough authored sections," that's a more constrained (and harder) problem than "regenerate whole file." **30 minutes to test.**

**A3**: Before building commands, manually track one full lifecycle using capsule events emitted by hand. If the ceremony feels heavier than editing SPEC.md, the abstraction is wrong. Alternatively: count how many SPEC.md status transitions you make per week. If <5, automation payoff is low. **1 week to observe.**

**A5**: Replay the event log for 3-5 completed specs. Check whether IntakeCompleted + StageTransition + BranchMerged events reliably map to the lifecycle states in SPEC.md. If there are specs that were manually created or completed without running the pipeline, auto-inference will produce incorrect state. **1 hour to test.**

***

## 5. Socratic Stress Test

**Q1: What happens to the hand-authored sections of SPEC.md?**
SPEC.md contains invariants, gating chains, replay truth tables, phase gates, policy references, and build commands. These are NOT derivable from capsule events. If SPEC.md becomes a projection, where do these live? Separate document? Header template? "Managed/unmanaged sections" parser? This is the single hardest design question and the PRD does not address it.

**Q2: How do you handle the cold-start problem?**
\~20 completed specs and 4 planned items in SPEC.md have no corresponding capsule work-item events (the event type doesn't exist). Do you backfill? Start fresh? The answer determines whether the projection is trustworthy from day one.

**Q3: What is the failure mode when capsule is unavailable?**
The capsule is gitignored (`.speckit/`). If corrupted, deleted, or cloned fresh, the work-item registry is gone. SPEC.md currently survives this because it's committed to git. Making it a projection means it's regenerable -- but only if the capsule is present. Do you commit the generated projection (dual-surface) or treat it as ephemeral?

**Q4: Why not just auto-sync SPEC.md at pre-commit?**
Keep SPEC.md committed. Capsule is SoR. Pre-commit regenerates tables from capsule state. Drift caught automatically. This is simpler than full projection and might deliver 80% of the value.

**Q5: Is the maieutic PRD session actually different from existing intake?**
`/speckit.new` already asks structured questions via the PRD builder modal. The maieutic module already captures goal, constraints, acceptance criteria, risks, delegation bounds. FR3 says "maieutic PRD session that produces a PRD file and registers it in capsule linked to its work item ID." How is this different from what already exists, other than capsule linkage? If the answer is "just add capsule linkage," FR3 is much smaller than it appears.

**Q6: What is the actual CRUD surface area?**
Create is partially handled. Read is `/speckit.status`. What does Update mean concretely -- changing status? Re-linking PRDs? If most updates are automated (pipeline events -> status change), the manual CRUD surface is small. If updates are primarily manual, you're building a CLI-first PM tool -- a much larger product.

**Q7: Does this make the system harder to onboard?**
Right now, someone can read SPEC.md and understand the project. If SPEC.md is generated from capsule state, they need to understand the capsule, the projection model, and regeneration semantics. Is the added indirection worth it for a solo project?

***

## 6. Smallest Valuable Increment

**Make SPEC.md table sections regenerable from capsule state, preserving hand-authored sections as passthrough.**

Concretely:

1. Add `WorkItemRegistered` event type to capsule (emitted on `/speckit.new`)
2. Add `WorkItemStatusChanged` event type (emitted from pipeline events or explicit `/speckit.transition <ID> <status>`)
3. Add `/speckit.projections rebuild-tracker` that reads these events and regenerates only table sections of SPEC.md
4. Wire into pre-commit hook as a staleness check (warning, not block)

This is NOT the full SPEC-PM-001 vision. No DEPRECATIONS.md projection, no archive packs, no multi-surface status queries, no deep maieutic PRD sessions. But it solves the core pain: "SPEC.md drifts from reality" and "I have to manually edit tables."

***

## 7. Risk & Anti-Patterns

**Over-engineering**: Building a full work-item CRUD system with schema versioning, relationship graphs, and rich query APIs for a project with 3-5 active items at any time. At this scale, the capsule-backed registry is architecturally elegant but may cost more keystrokes than it saves.

**Abstraction leak**: SPEC.md contains both registry data and authored prose. A "managed/unmanaged sections" parser becomes a maintenance burden and source of subtle bugs.

**Second-system effect**: The current `update_spec_tracker` is 55 lines of string manipulation. The proposed replacement (capsule events + projection engine + pre-commit validation) is 10-20x more code for equivalent functionality.

**"Good enough" alternative**:

* `doc_lint.py` check that validates SPEC.md against `docs/SPEC-*/` directories
* `/speckit.transition <ID> <status>` that edits SPEC.md directly (no capsule, just file manipulation)
* Keep DEPRECATIONS.md manual

Less principled, but 80% of the value at 20% of the effort. The question: does the remaining 20% (capsule auditability, deterministic rebuilds, headless status queries) justify the full architecture?

***

## 8. Sharpened Problem Statement

**Before**: "PM is spec management."

**After**: "The project tracker (SPEC.md) and the execution engine (capsule) are maintained separately, causing status drift. When a spec is created, runs through the pipeline, or is deprecated, SPEC.md must be manually updated. The capsule already records execution lifecycle events. Adding work-item lifecycle events to the capsule and projecting them into SPEC.md table sections would eliminate manual synchronization, make project state queryable from a single SoR, and extend the 'filesystem is projection' principle that already governs docs/SPEC-\* directories."

**Testable prediction**: After shipping the smallest increment, manual SPEC.md edits for status transitions should drop to zero. SPEC.md tables should be regenerable from capsule + `projections rebuild-tracker`, producing output identical to the committed version.

***

## 9. Recommended Next Actions

1. **Audit SPEC.md structure** -- classify each section as "registry" (derivable from events) vs "authored" (human prose). Determines if projection is "regenerate tables + passthrough" or "regenerate whole file." 30 minutes.

2. **Prototype WorkItemRegistered / WorkItemStatusChanged events** -- add two variants to `EventType` enum, define payloads, emit one manually for an existing spec, query it back. Validates capsule extensibility (A4). Under 1 hour.

3. **Run a shadow projection test** -- attempt to reconstruct SPEC.md tables purely from existing capsule events. Diff against committed file. Validates auto-inference reliability (A5) and reveals the gap. 1 hour.

4. **Decide the dual-surface policy** -- is the projected SPEC.md committed to git (checked at pre-commit) or purely ephemeral? This is a governance decision. Write it down as a D-number before coding.

5. **Defer DEPRECATIONS.md** -- it changes infrequently (\~40 entries created in a single day). Ship tracker projection first, use it for a month, then evaluate deprecation automation ROI.

***

*Extracted from Claude Code plan file `temporal-seeking-giraffe.md` on 2026-02-12. Original session ran 2026-02-06.*
