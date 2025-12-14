# P108 Continuation Prompt — Audit Follow-ups + P107 Research API

**Created**: 2025-12-14
**Predecessor**: P107 (deferred), Policy Audit Session
**Commit**: `4c710db2c` (MODEL-POLICY.md + spec compliance)

---

## Pre-flight

```bash
git log -1 --oneline          # Expect 4c710db2c or later
git status                    # Expect clean
./build-fast.sh               # Verify build
```

---

## Session Structure

This session has **two phases**:

| Phase | Scope | Type | Exit Condition |
|-------|-------|------|----------------|
| **A** | Audit follow-ups | Quick wins (docs + config) | All 4 items complete or tracked |
| **B** | P107 Research API & Ask Polish | Feature work | Research commands + Ask polish working |

**Rule**: Complete Phase A before starting Phase B. Keep items separable.

---

## Phase A: Audit Follow-ups (Quick Wins)

### A1. vLLM Config Verification

**Goal**: Ensure local runtime config matches `docs/MODEL-POLICY.md` v1.0.0

**Checklist**:
- [ ] Verify model IDs/aliases are consistent with policy table
- [ ] Confirm quant assumptions (especially 32B coder fits 5090 32GB)
- [ ] Confirm context defaults (planner 14B vs implementer 32B vs tutor 7B)
- [ ] Confirm vLLM is default runtime, llama.cpp fallback is wired
- [ ] Check any config files that reference models (stage0.toml, etc.)

**Exit condition**: "If I deploy local services, the policy table is actually runnable."

**Deliverable**: Config patch or verification note (no change needed if correct)

---

### A2. SPEC-KIT-099 Deprecation

**Goal**: Make 099 clearly legacy so it can't contradict policy

**Current state**: Has legacy banner (added in audit), but still active

**Actions**:
- [ ] Update status from "Legacy (Partially Superseded)" to "DEPRECATED"
- [ ] Add explicit pointer: "For current implementation, see SPEC-KIT-102"
- [ ] Review remaining consensus language and neutralize if any remains
- [ ] Consider moving to `docs/archive/` if fully superseded

**Exit condition**: Nobody can read 099 and think consensus is canonical.

**Deliverable**: One docs-only commit

---

### A3. Instruction File Parity (CLAUDE.md / AGENTS.md / GEMINI.md)

**Goal**: Sync instruction files to reference MODEL-POLICY.md consistently

**Pre-commit warning**: "CLAUDE.md and AGENTS.md are out of sync"

**Scope (DO)**:
- [ ] Pick source of truth (CLAUDE.md recommended — most complete)
- [ ] Make AGENTS.md and GEMINI.md match it
- [ ] Add "Policy Pointers" header to all three:
  - "Authoritative policy: `docs/MODEL-POLICY.md` (v1.0.0)"
  - "Guardrails: GR-001 through GR-013"
  - "No consensus by default"
  - "Kimi/DeepSeek escalation-only"
- [ ] Ensure all three reference same routing thresholds (0.75 / 2 loops)

**Out of scope (DON'T)**:
- Don't redesign agent behavior
- Don't change code or workflows
- Don't add new features

**Exit condition**: Pre-commit parity check passes.

**Deliverable**: `docs(meta): sync CLAUDE/AGENTS/GEMINI instructions with MODEL-POLICY`

---

### A4. consensus.rs Code Paths (Track Only)

**Goal**: Create tracked issue for runtime alignment — do NOT fix inline

**Current state**: Policy says "no consensus" but runtime code may still have paths

**Actions**:
- [ ] Review `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs`
- [ ] Identify: Is consensus flow disabled by default? Is critic-only pattern preserved?
- [ ] Create GitHub issue or TODO in SPEC.md tracking:
  - Disable consensus flow by default
  - Preserve critic-only as optional sidecar
  - Update CLI flags/config to match GR-001

**Exit condition**: Clear tracked path to align runtime with policy.

**Deliverable**: Issue/TODO created (code fix is separate PR)

---

## Phase B: P107 Research API & Ask Polish

**Goal**: Implement research commands and polish ask features

### B1. Research API (Full Suite)

Commands to implement:

```bash
code architect research fast "query"    # Quick research, immediate response
code architect research deep "query"    # Thorough research, may take longer
code architect research status          # Check status of running research
code architect research results         # Get results of completed research
code architect research import          # Import research into context
```

**Implementation path**:
1. Create `core/src/architect/research.rs`
2. Check notebooklm-mcp `/api/research/*` endpoints
3. Add Research subcommand to CLI
4. Wire up to NotebookLM HTTP API (v2.0.0)
5. Add caching layer for results

---

### B2. Ask Polish (All Three)

Features to add:

| Feature | Description | Default |
|---------|-------------|---------|
| Budget warnings | Warn at 80%/100% of token budget | Enabled |
| Cache TTL | Cache responses with configurable TTL | 24h default |
| `--no-cache` flag | Bypass cache for fresh response | Flag |
| Session reuse | Reuse NotebookLM session between queries | Enabled |

**Implementation path**:
1. Add budget tracking to ask command
2. Implement response cache with TTL
3. Add `--no-cache` CLI flag
4. Wire session persistence

---

## Files to Read First

```
docs/MODEL-POLICY.md                    # Policy we just created
docs/SPEC-KIT-102-notebooklm-integration/spec.md  # NotebookLM v2.0.0 spec
docs/PROMPT-P107.md                     # Original P107 context
codex-rs/tui/src/chatwidget/spec_kit/consensus.rs  # For A4 review
CLAUDE.md                               # Source of truth for A3
```

---

## Commit Strategy

| Phase | Commits |
|-------|---------|
| A1 | `chore(config): verify vLLM config matches MODEL-POLICY` (or skip if no changes) |
| A2 | `docs(SPEC-KIT-099): mark as deprecated, point to SPEC-KIT-102` |
| A3 | `docs(meta): sync CLAUDE/AGENTS/GEMINI instructions with MODEL-POLICY` |
| A4 | No commit — create issue/TODO only |
| B1 | `feat(architect): add research command suite` |
| B2 | `feat(architect): add ask polish (budget, cache, session)` |

---

## Success Criteria

**Phase A complete when**:
- [ ] vLLM config verified or patched
- [ ] SPEC-KIT-099 clearly deprecated
- [ ] Instruction files in sync (pre-commit passes)
- [ ] consensus.rs alignment tracked

**Phase B complete when**:
- [ ] `code architect research fast/deep/status/results/import` all work
- [ ] Ask command has budget warnings at 80%/100%
- [ ] Ask command has cache with TTL (default 24h)
- [ ] Ask command has `--no-cache` flag
- [ ] Ask command reuses NotebookLM session

---

## Risk Notes

| Risk | Mitigation |
|------|------------|
| vLLM config mismatch blocks local routing | Verify A1 first before any model calls |
| Instruction parity causes pre-commit failures | Fix A3 before feature commits |
| consensus.rs refactor scope creep | A4 is tracking only — separate PR |
| NotebookLM session management complexity | Start with simple session ID reuse |

---

## Quick Reference

**Policy doc**: `docs/MODEL-POLICY.md` (v1.0.0)
**Guardrails**: GR-001 (no consensus), GR-008 (citation-grounded)
**Escalation**: Kimi (Librarian hard sweeps), DeepSeek (Implementer stuck)
**Local runtime**: vLLM default, llama.cpp fallback

---

*Continue this session by running pre-flight checks, then work through Phase A items in order.*
