# Session Handoff — Dogfooding: "tui writes tui"

**Last updated:** 2025-12-25
**Status:** P0 Blockers Resolved - Ready for Dogfooding

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

## Session 14 Summary (2025-12-25) - DOGFOODING READINESS

### Completed

1. **Dogfooding Analysis**
   - Identified P0/P1 blockers
   - Created `docs/DOGFOODING-CHECKLIST.md`
   - Created `docs/DOGFOODING-BACKLOG.md`

2. **Instruction File Fixes**
   - Fixed CLAUDE.md, AGENTS.md, GEMINI.md headers
   - Added tui/tui2 callout with ADR-002 link

3. **SessionEnd Hook Fix** (in localmemory-policy)
   - Was gated on `CLAUDE_TOOL_COUNT` env var (never set by Claude Code)
   - Now parses stdin JSON for `transcript_path`
   - Analyzes transcript for decision patterns (ADR-*, SYNC-*)
   - Auto-promotes decisions (importance 9) and milestones (importance 8)

4. **NotebookLM Setup**
   - Authentication configured
   - Created `code-project-docs` notebook
   - Added 3 sources (ADR-002, Dogfooding Checklist, Golden Path)
   - Verified HTTP API works (`/api/ask` returns accurate answers)

### Key Decisions Made

| Decision | ADR | Summary |
|----------|-----|---------|
| tui2 purpose | ADR-002 | Upstream scaffold only, NOT replacement |
| Golden path | - | `/speckit.auto` + Stage0 + local-memory + NotebookLM |
| Memory access | - | CLI + REST only, no MCP |
| NotebookLM access | - | HTTP API at 127.0.0.1:3456, no MCP |

---

## Remaining Work

### P1: Should Fix Soon

| Item | Owner | Status |
|------|-------|--------|
| Create stage0.toml | `~/code` or user config | Pending |
| Consolidate config files | User config | Pending |
| Add more NotebookLM sources | `~/code` | Optional |

### P2: Nice to Have

| Item | Owner |
|------|-------|
| `/doctor` TUI command | `~/code` |
| Session hooks for `code` binary | `localmemory-policy` |

---

## Quick Commands

```bash
# Daily health check
./build-fast.sh && \
./codex-rs/target/dev-fast/code doctor && \
lm health && \
curl -s http://127.0.0.1:3456/health/ready | jq -r '.ready'

# Start dogfooding
./build-fast.sh run

# Query NotebookLM
curl -s http://127.0.0.1:3456/api/ask -X POST \
  -H "Content-Type: application/json" \
  -d '{"notebook": "code-project-docs", "question": "What is the golden path?"}' | jq '.data.answer'
```

---

## Cross-Repo Coordination

| Repo | Owns | Recent Changes |
|------|------|----------------|
| `~/code` | Stage0, spec-kit, TUI, doctor | ADR-002, dogfooding docs, instruction files |
| `~/notebooklm-client` | Auth, library, HTTP service | Fixed patchright-core issue |
| `~/infra/localmemory-policy` | Memory policy, hooks, `lm` CLI | SessionEnd hook fix |

---

## Key Files

| File | Purpose |
|------|---------|
| `docs/DOGFOODING-CHECKLIST.md` | Daily health checks |
| `docs/DOGFOODING-BACKLOG.md` | Prioritized blockers |
| `docs/adr/ADR-002-tui2-purpose-and-future.md` | tui/tui2 decision |
| `CLAUDE.md`, `AGENTS.md`, `GEMINI.md` | Agent instructions |

---

## Continuation Prompt

```
Load HANDOFF.md for full context. ultrathink

## Context
Session 14 completed dogfooding readiness analysis.
- All P0 blockers resolved
- NotebookLM notebook created with project docs
- Instruction files fixed
- SessionEnd hook fixed (in localmemory-policy)

## Session 15 Goals

### Primary: Start Dogfooding
1. Run `./build-fast.sh run` in real terminal
2. Use `code` TUI to make a small change to `~/code`
3. Run tests via TUI
4. Commit via TUI

### Secondary: P1 Items
1. Create `~/.config/codex/stage0.toml` for Tier2 domain mapping
2. Add more sources to NotebookLM notebook (optional)

## Questions for Architect
1. Should we create stage0.toml now or defer?
2. How much NotebookLM source content? (minimal/core docs/comprehensive)
3. Should we create a formal SPEC for dogfooding work?

## Success Criteria
- [ ] Complete one full development cycle using `code` TUI
- [ ] Make a change, test, commit - without using Claude Code
- [ ] Stage0.toml created (if approved)

## Key Commands
./build-fast.sh run
./codex-rs/target/dev-fast/code doctor
curl -s http://127.0.0.1:3456/api/ask -X POST -H "Content-Type: application/json" \
  -d '{"notebook": "code-project-docs", "question": "..."}'
```

---

## Session 14 Commits

| Hash | Message |
|------|---------|
| 8790efdf9 | docs(adr): ADR-002 tui2 is upstream scaffold |
| 57538569a | docs: add dogfooding checklist and fix instruction file headers |
| cc9b897 | fix(hooks): SessionEnd now uses transcript analysis (localmemory-policy) |

---

## Previous Sessions (SYNC-028)

| Session | Focus | Outcome |
|---------|-------|---------|
| S10 | Compilation | 262 → 0 errors |
| S11 | Runtime testing | --help/--version work |
| S12 | Warning cleanup | 117 → 0 warnings |
| S13 | External crates | 0 warnings all crates |
| S14 | Dogfooding readiness | All P0 blockers resolved |
