# HANDOFF: Upstream Backport Program

**Generated**: 2025-12-22
**Last Commit**: 12a3259f7 (PRD skeletons for SYNC-019 to SYNC-031)
**Branch**: main

---

## Session Summary (2025-12-22)

### Completed This Session

| Task | Status | Evidence |
|------|--------|----------|
| P7-A: speckit run batch command | ✅ Done | 64 CLI tests |
| P7-B: spec.md→PRD.md migration tooling | ✅ Done | 70 CLI tests |
| P7-C: Architectural watch-out fixes | ✅ Done | Watch-outs A, C, D |
| SYNC-019 to SYNC-031 roadmap | ✅ Added | SPEC.md updated |
| PRD skeletons (13 files) | ✅ Added | docs/SYNC-0XX-*/PRD.md |

### Key Commits
```
12a3259f7 docs(sync): add SYNC-019 to SYNC-031 PRD skeletons
6f759772e docs(spec-kit): update HANDOFF.md for upstream backport program
400e07916 docs(spec): add SYNC-019 to SYNC-031 upstream backport roadmap
ceb5b46ad docs: fix SPEC.md status line for P7-B completion
e7ff5dc1a feat(spec-kit): P7-B migration tooling + P7-C architectural fixes
```

### Policy Decision (Documented)

**Legacy packets are BLOCKED until migrated** (not warn-and-proceed).
- `legacy_detected` field (renamed from `legacy_fallback`)
- Migration path: `code speckit migrate --spec <ID>`
- Creates PRD.md with migration header, leaves spec.md intact

---

## Continue: Upstream Backport Program

Copy this prompt to start the next session:

```
# Continue Upstream Backport Program (SYNC-019+)

Read docs/spec-kit/HANDOFF.md and SPEC.md section "Upstream Backport Program (2025-12-22)"

## Context
- SPEC-KIT-921 P7 complete (70 CLI tests, speckit run + migrate commands)
- SYNC-019 to SYNC-031 roadmap in SPEC.md (13 items, 0/13 started)
- PRD skeletons available: docs/SYNC-0XX-*/PRD.md (all 13 files)
- Index: docs/SYNC-019-031-UPGRADE-INDEX.md

## Session Scope (Confirmed)

### SYNC Items (in order)
1. SYNC-019: Central Feature Registry (P0, enabler)
2. SYNC-023: TUI v1 UX/perf backports (P0, immediate benefit)
3. SYNC-020: Skills v1 (P0, depends on SYNC-019)
4. SYNC-025: Exec hardening (P0, reliability)

### Architectural Debt (pair with SYNC work)
- AD-006: Event channel backpressure (pairs with SYNC-023/025)
  Scope: Bounded queues + drop/coalesce policy in UI/event hot paths
- AD-001: Async blocking in TUI event loop (pairs with SYNC-023)
  Scope: Targeted "no blocking on runtime threads" pass, fix top offenders

### Docs Alignment (micro-task with SYNC-019)
- Fix product-requirements.md narrative mismatch
- Remove "consensus/arbiter" language, align with single-owner pipeline
- Small diff, high leverage

## First Task: SYNC-019 Feature Registry

### PRD
docs/SYNC-019-features-registry/PRD.md (skeleton exists, fill in details)

### Goal
Introduce centralized `Features` enum + config mapping as enabler for all feature-gated work.

### Acceptance Criteria
- [ ] Create `spec-kit/src/features.rs` with Features enum
- [ ] Feature gates: skills, tui2, remote_models, bridge, unified_exec, exec_policy, otel
- [ ] Config mapping from TOML/env to feature toggles
- [ ] Feature checks at edges (CLI/TUI), not deep in domain logic
- [ ] Unknown feature keys warn but don't crash
- [ ] "Enabled features" visible in /debug or logs
- [ ] Update PRD.md with implementation details

### Architectural Notes
- Pattern: enum-based feature gates with "stage" metadata
- Resolution: config file < env var (standard layering)
- Boundary: features.rs owns enum + resolution, adapters query it

## Start Commands
```bash
git log --oneline -5
cargo test -p codex-cli --test speckit -- --test-threads=1
cat docs/SYNC-019-features-registry/PRD.md
```

## Test Baseline
- 70 CLI tests passing (speckit + helpers)
- Clippy clean
```

---

## Upstream Backport Roadmap (Reference)

| Priority | Task ID | Title | Feature Flag | Depends On |
|----------|---------|-------|--------------|------------|
| P0 | SYNC-019 | Central Feature Registry | n/a (enabler) | — |
| P0 | SYNC-020 | Skills v1 | `features.skills` | SYNC-019 |
| P0 | SYNC-023 | TUI v1 UX/perf backports | n/a | — |
| P0 | SYNC-025 | Exec hardening | `features.unified_exec` | — |
| P1 | SYNC-021 | Skills v2 (SkillsManager) | `features.skills` | SYNC-020 |
| P1 | SYNC-022 | Code-Bridge v2 consumer | `features.bridge` | SYNC-019 |
| P1 | SYNC-024 | `/ps` + background terminal | n/a | SYNC-023 |
| P1 | SYNC-026 | Retention/compaction hardening | n/a | SYNC-025 |
| P1 | SYNC-028 | TUI v2 scaffold | `features.tui2` | SYNC-019 |
| P2 | SYNC-027 | ModelsManager + remote models | `features.remote_models` | SYNC-019 |
| P2 | SYNC-029 | TUI v2 parity pass | `features.tui2` | SYNC-028 |
| P2 | SYNC-030 | Governance (requirements.toml) | `features.exec_policy` | SYNC-019 |
| P3 | SYNC-031 | Minimal OTel (optional) | `features.otel` | — |

**Architectural Notes**:
- Patchable: Skills, `/ps`, exec hardening, retention, feature registry
- Bigger lift: TUI2 (new frontend), code-bridge (privacy considerations)
- Feature checks at edges, not in domain logic
- Skills boundary: metadata-only injection (body stays on disk)

---

## Architectural Debt Items (from SPEC.md)

| ID | Priority | Description | Pairs With |
|----|----------|-------------|------------|
| AD-001 | P0 | Async blocking in TUI event loop | SYNC-023 |
| AD-006 | P0 | Event channel backpressure | SYNC-025/026 |
| AD-002 | P1 | Inconsistent error handling | General cleanup |
| AD-003 | P1 | Lack of structured logging | SYNC-031 |

---

## Files Reference

### Key Files for SYNC-019
| File | Purpose |
|------|---------|
| `spec-kit/src/features.rs` | NEW: Features enum + resolution |
| `spec-kit/src/config/loader.rs` | Config loading (add features section) |
| `cli/src/main.rs` | CLI entry (query features at startup) |
| `tui/src/app.rs` | TUI entry (query features at startup) |

### Completed P7 Files
| File | Changes |
|------|---------|
| `spec-kit/src/executor/mod.rs` | execute_run(), execute_migrate(), legacy_detected |
| `spec-kit/src/executor/command.rs` | Run, Migrate variants |
| `spec-kit/src/executor/status.rs` | Unified resolve_spec_dir() usage |
| `cli/src/speckit_cmd.rs` | run, migrate subcommands |
| `cli/tests/speckit.rs` | 70 tests (64 + 6 migrate) |

---

## Exit Code Contract

| Code | Meaning |
|------|---------|
| 0 | Success / Ready |
| 2 | Blocked / Escalation |
| 3 | Infrastructure error |

---

## Invariants to Preserve

1. **No env/config reads in executor core** — resolve at adapter boundary
2. **All paths repo-relative** in JSON output and evidence refs
3. **Deterministic evidence selection** — tests cover ordering logic
4. **Feature checks at edges** — CLI/TUI/tool registry, not domain logic
5. **TUI is an adapter** — slash commands call executor
6. **Advisory by default** — missing prereqs warn unless --strict-prereqs
7. **Legacy packets blocked** — spec.md requires migration to PRD.md
