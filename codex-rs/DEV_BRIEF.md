# DEV\_BRIEF.md

**Focus:** Trust Foundation (PM-005 + PM-006, parallel)
**Phase:** P0 — Capsule-backed PM
**Branch:** main

## Current Work

* PM-005: Change classifier (Class 0/1/2/E) — implemented, tests passing
* PM-006: Packet schema + atomic I/O + sacred anchor guard — implemented, tests passing
* Next: WP-3 (wire classifier to packet), WP-4 (PM-007 recap), WP-5 (DOGFOOD-002 exit gate)

## Constraints

* Edition 2024, workspace clippy deny
* No tui2 feature work (quarantined per ADR-002)
* Sacred anchors immutable without amendment workflow
