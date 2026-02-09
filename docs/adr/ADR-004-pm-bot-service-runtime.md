# ADR-004: PM Bot Service Runtime + Systemd Resume

**Status:** Accepted\
**Date:** 2026-02-09\
**Deciders:** Product + Architecture\
**Context:** PM bot runs that may span hours/days and must survive reboots

***

## Decision

This ADR is locked via D135 in `docs/DECISIONS.md`.

Adopt a **service-first runtime** for PM bot execution (`NeedsResearch` / `NeedsReview`):

- A lightweight local service manages bot run lifecycle (start/stop/status/cancel/resume).
- A **systemd user unit** ensures incomplete runs can be resumed after reboot **without interactive prompts**.
- The **CLI remains canonical** for Tier‑1 parity: CLI commands talk to the service when available and preserve identical semantics across TUI/CLI/headless.
- An **ephemeral CLI runner** remains as a fallback/debug mode, but is not the primary runtime for long-lived runs.

## Why this is needed

Long-running bot runs (hours → days) must be resilient to:

- TUI restarts / terminal disconnects,
- process crashes,
- machine reboots.

A service + systemd-managed resume makes “run continuation” a product guarantee rather than a best-effort behavior.

## Compatibility with locked decisions (resolved)

This ADR intersects with:

- **D38 Operational footprint** (“prefer single-binary, no-daemon design; daemons only as optional legacy”).
- **D126 Maintenance posture** (“no permanent daemon”).

This intersection is resolved by **D135** (“Bot job management service”), which clarifies the scope of acceptable persistence:

- *Allowed:* a lightweight, user-scoped service that is idle when no jobs exist and exists solely for job management/resume.
- *Disallowed:* heavy frameworks, always-processing background agents, or services that become a second source of truth.

## Non-negotiables

- **Capsule (`mv2://`) remains the system of record** for bot run artifacts and evidence (see D114).
- **Headless never prompts**; missing inputs/prereqs produce structured “needs input/blocked” outcomes (D133).
- **Maieutic context is mandatory pre-execution**; resume must not bypass gates (D130).
- **Capture mode governs persistence** and over-capture is hard-blocked (D131/D119).

## Consequences

- TUI can degrade gracefully when the service is down (read-only status from capsule), and provide explicit service management actions.
- Bot run execution becomes resumable and observable independently of the UI process lifetime.
- We must define what “resume” means for determinism: what is snapshotted, what is allowed to drift, and how those facts are recorded as evidence.

## Open Questions

- Service posture: socket-activated + idle-exit vs continuously running while jobs exist.
- Resume triggers: timer-based, state-based, or explicit “resume all incomplete” action.
- Determinism boundary: what inputs must be snapshotted vs what drift is permitted during multi-day runs (especially for review worktrees).

## References

- Canon tracker: `codex-rs/SPEC.md` (Planned: `SPEC-PM-001`/`002`/`003`)
- Bot runner contract: `docs/SPEC-PM-002-bot-runner/spec.md`
- Bot system design: `docs/SPEC-PM-003-bot-system/spec.md`
