# Session Handoff — Dogfooding: "tui writes tui"

**Last updated:** 2025-12-25
**Status:** P0 Blockers Resolved - Ready for SPEC-DOGFOOD-001

> **Goal**: Use `code` TUI to develop `~/code` as the default workflow.

---

## Current State

### Health Check Status (All PASS)

| Service | Status | Verification |
|---------|--------|--------------|
| Build | PASS | `./build-fast.sh` succeeds |
| Local-memory | PASS | `lm health` returns "ok" |
| NotebookLM Auth | PASS | `authenticated: true` |
| NotebookLM Ready | PASS | `ready: true` |
| NotebookLM Library | PASS | 1 notebook, 3 sources |
| Code Doctor | PASS | All [OK], 1 WARN (stage0.toml) |

### Notebook Created

| Property | Value |
|----------|-------|
| Name | `code-project-docs` |
| ID | `4e80974f-789d-43bd-abe9-7b1e76839506` |
| Sources | ADR-002, Dogfooding Checklist, Golden Path Architecture |

---

## Session 15 Plan (Architect-Approved)

### Decisions Made

| Question | Answer | Rationale |
|----------|--------|-----------|
| stage0.toml | **Yes, create it** | Removes WARN, enables Tier2 for dogfooding |
| NotebookLM sources | **Core docs only** | Tight, high-leverage set; not comprehensive |
| Formal SPEC | **Yes, create SPEC-DOGFOOD-001** | Makes dogfooding measurable via golden path |

### Critical Correction: stage0.toml Schema

**Current docs are outdated.** The actual `Stage0Config` expects:

```toml
# ~/.config/code/stage0.toml (CORRECT)
enabled = true
store_system_pointers = true
db_path = "~/.config/code/local-memory-overlay.db"

[tier2]
enabled = true
notebook = "4e80974f-789d-43bd-abe9-7b1e76839506"  # NOT "default_notebook"
base_url = "http://127.0.0.1:3456"
cache_ttl_hours = 24
call_timeout = "30s"
```

**NOT** the old format:
```toml
# WRONG - docs/examples use this but Stage0 ignores it
[tier2]
default_notebook = "code-docs"  # Stage0 looks for "notebook" not this
```

### Path Canonicalization

| Type | Canonical | Fallback (backward compat) |
|------|-----------|---------------------------|
| Config dir | `~/.config/code/` | `~/.config/codex/` |
| stage0.toml | `~/.config/code/stage0.toml` | `~/.config/codex/stage0.toml` |

---

## Session 15 Goals

### 1. Create stage0.toml (Correct Schema)

```bash
mkdir -p ~/.config/code
cat > ~/.config/code/stage0.toml << 'EOF'
enabled = true
store_system_pointers = true
db_path = "~/.config/code/local-memory-overlay.db"

[tier2]
enabled = true
notebook = "4e80974f-789d-43bd-abe9-7b1e76839506"
base_url = "http://127.0.0.1:3456"
cache_ttl_hours = 24
call_timeout = "30s"
EOF
```

### 2. Seed NotebookLM with Core Docs Only

Add these sources via HTTP API:
- `docs/DOGFOODING-CHECKLIST.md`
- `docs/SPEC-TUI2-STUBS.md`
- `docs/convergence/CONVERGENCE_OVERVIEW.md`
- `docs/stage0/STAGE0_IMPLEMENTATION_GUIDE.md` (if exists)
- NL_* files (via `/stage0.project-intel` commands if available)

### 3. Create SPEC-DOGFOOD-001

Create `docs/SPEC-DOGFOOD-001/spec.md` with:
- G1: Tier2 enabled by default
- G2: Config/docs reference `code` not `codex`
- G3: NotebookLM seeded with core docs
- G4: Stage0 writes system pointer to local-memory
- G5: Evidence artifacts produced

### 4. Run via Golden Path

```bash
./build-fast.sh run
# In TUI: /speckit.auto SPEC-DOGFOOD-001
```

---

## Acceptance Tests (From SPEC)

| Test | Command | Pass Criteria |
|------|---------|---------------|
| A1: Doctor ready | `code doctor` | No stage0.toml warning |
| A2: Tier2 used | `/speckit.auto SPEC-DOGFOOD-001` | tier2_used=true in logs |
| A3: Evidence exists | `ls docs/SPEC-DOGFOOD-001/evidence/` | TASK_BRIEF.md + DIVINE_TRUTH.md |
| A4: System pointer | `lm search "SPEC-DOGFOOD-001"` | system:true artifact exists |

---

## Cross-Repo Coordination

| Repo | Owns | Session 15 Changes |
|------|------|-------------------|
| `~/code` | Stage0, spec-kit, TUI, doctor | SPEC-DOGFOOD-001, stage0.toml template |
| `~/notebooklm-client` | Auth, library, HTTP service | Add core doc sources |
| `~/infra/localmemory-policy` | Memory policy, hooks, `lm` CLI | (no changes expected) |

---

## Continuation Prompt

```
Load HANDOFF.md for full context. ultrathink

## Context
Session 14 completed dogfooding readiness. Architect approved:
- Create stage0.toml (with CORRECT schema - see HANDOFF.md)
- Add core docs only to NotebookLM
- Create formal SPEC-DOGFOOD-001

CRITICAL: Current stage0.toml docs are WRONG. Stage0Config expects:
- `tier2.notebook` (not `default_notebook`)
- `tier2.base_url` (not nested domain_mapping)

## Session 15 Goals

### 1. Create stage0.toml
Path: ~/.config/code/stage0.toml
Use notebook ID: 4e80974f-789d-43bd-abe9-7b1e76839506
Verify: `code doctor` shows no stage0.toml warning

### 2. Seed NotebookLM with Core Docs
Add via HTTP API (POST /api/sources):
- docs/DOGFOODING-CHECKLIST.md
- docs/SPEC-TUI2-STUBS.md
- docs/convergence/CONVERGENCE_OVERVIEW.md
- docs/stage0/STAGE0_IMPLEMENTATION_GUIDE.md (if exists)

### 3. Create SPEC-DOGFOOD-001
Create docs/SPEC-DOGFOOD-001/spec.md with acceptance tests:
- A1: Doctor ready
- A2: Tier2 used
- A3: Evidence exists
- A4: System pointer stored

### 4. Run SPEC via Golden Path
./build-fast.sh run
In TUI: /speckit.auto SPEC-DOGFOOD-001

## Success Criteria
- [ ] code doctor passes with no warnings
- [ ] Tier2 query works (tier2_used=true)
- [ ] SPEC-DOGFOOD-001 evidence artifacts exist
- [ ] System pointer in local-memory

## Key Commands
./build-fast.sh run
./codex-rs/target/dev-fast/code doctor
curl -s http://127.0.0.1:3456/api/sources -X POST -H "Content-Type: application/json" -d '{...}'
```

---

## Session 14 Summary

### Completed

1. **Dogfooding Analysis** - Created DOGFOODING-CHECKLIST.md, DOGFOODING-BACKLOG.md
2. **Instruction File Fixes** - Fixed CLAUDE.md, AGENTS.md, GEMINI.md headers
3. **SessionEnd Hook Fix** - Now parses transcript, auto-promotes decisions/milestones
4. **NotebookLM Setup** - Auth configured, notebook created, 3 sources added
5. **Architect Review** - Approved stage0.toml, core docs, formal SPEC

### Key Decisions

| Decision | ADR/Source |
|----------|------------|
| tui2 is scaffold only | ADR-002 |
| Golden path: /speckit.auto + Stage0 + Tier2 | Architect |
| Memory: CLI + REST only | Policy |
| NotebookLM: HTTP API at 127.0.0.1:3456 | Policy |
| Config path: ~/.config/code/ canonical | Architect |

---

## Previous Sessions

| Session | Focus | Outcome |
|---------|-------|---------|
| S10-S13 | SYNC-028 tui2 port | Compiles, 0 warnings |
| S14 | Dogfooding readiness | All P0 resolved, architect review |
| S15 | SPEC-DOGFOOD-001 | (next) |
