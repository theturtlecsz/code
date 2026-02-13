# PRD: TUI PM UX/UI (SPEC-PM-004)

**SPEC-ID**: SPEC-PM-004
**Status**: Draft
**Created**: 2026-02-12
**Author**: Design Q\&A session 2

***

## Problem Statement

The PM layer (PM-001/002/003) defines a data model, bot interface, and service runtime — but no specification for how the primary user interacts with it. The TUI is the single control surface (north star), yet the interaction design is scattered across PM-001's "TUI Interaction Design" subsection and informal notes. Without a dedicated UX spec, the costliest risk (getting the PM experience wrong) has no spec to validate against.

***

## Goals

1. Define the TUI PM views (list + detail) that make "what's the state of my project?" answerable in 30 seconds.
2. Define navigation, filtering, and keyboard-driven workflows for solo-developer efficiency.
3. Define run configuration UX (presets + scope toggles) as the interface between the user and bot runs.
4. Define degraded-mode behavior so the TUI never breaks when the service is down.
5. Define status indicators that make holding states, active runs, and safety signals immediately visible.

***

## Non-Goals

* Visual design system (colors, fonts, spacing) — that's implementation detail.
* Web UI (deferred to v2).
* CLI UX (owned by PM-002; TUI must achieve Tier-1 parity but the UX is distinct).

***

## Success Metrics

* User can determine project state (what's in progress, what's blocked, what needs attention) from the PM list view in under 30 seconds.
* User can launch, monitor, and inspect bot runs entirely from TUI without switching to CLI.
* When service is down, TUI degrades gracefully (read-only) with no confusion about what's available.

***

## Dependencies

* PM-001: work item schema (which fields exist determines what's shown).
* PM-002: run config surface (preset + scope names).
* PM-003: service status + checkpoint data format.

***

## References

* Spec: `docs/SPEC-PM-004-tui-ux/spec.md`
* PM-003 design transcript: `docs/SPEC-PM-003-bot-system/design-qa-transcript.md`
* PM-004 design transcript: `docs/SPEC-PM-004-tui-ux/design-qa-transcript.md`
