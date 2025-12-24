# ADR-002: TUI2 Purpose and Future

**Status:** Accepted
**Date:** 2025-12-24
**Deciders:** Architecture review
**Context:** SYNC-028 (TUI v2 Port) completion, Session 13

---

## Decision

**tui2 is an upstream-aligned, viewport-style UI scaffold. It is NOT a replacement for tui.**

tui2's purpose is to:
1. Reduce upstream sync friction
2. Test upstream UI behavior and contracts
3. Serve as a source for selective backports into tui

tui remains the default UI and the only UI supporting spec-kit workflows.

---

## Context

SYNC-028 ported `tui2` from upstream (just-every/code) over 13 sessions:
- 262 → 0 compilation errors
- 117 → 0 warnings
- Functional binary that launches and accepts prompts

However, tui2 is missing major local functionality:

| Feature | tui2 | tui |
|---------|------|-----|
| spec_kit system | Not present | 1.3MB, 60+ files |
| `/speckit.auto`, `/speckit.new` | Missing | Fully implemented |
| Agent orchestrator | Missing | 90K LOC |
| Pipeline coordinator | Missing | 86K LOC |
| Quality gates | Missing | Full system |
| Cost tracking | Missing | Implemented |
| `/model` selection | Stubbed | Working |

The golden path workflow (`/speckit.auto` + Stage0 + local-memory + NotebookLM) lives entirely in tui.

---

## Decision Drivers

1. **Convergence principle**: Avoid double-building orchestration
2. **Golden path stability**: spec-kit workflows must remain stable
3. **Maintenance burden**: Two full UIs is unsustainable
4. **Upstream alignment**: Value in staying close to upstream for rebases

---

## Considered Options

| Option | Effort | Risk | Outcome |
|--------|--------|------|---------|
| **A: tui2 replaces tui** | High (weeks) | High | Duplication, two partial UIs |
| **B: Cherry-pick to tui** | Medium | Low | Best of both, single primary UI |
| **C: Delete tui2** | Low | Low | Lose upstream reference |
| **D: Parallel coexistence** | Ongoing | High | Indefinite maintenance burden |

---

## Decision Outcome

**Chosen option: B (Cherry-pick to tui)** with tui2 retained as a non-default upstream scaffold.

### Constraints

1. **tui remains the default** and the spec-kit UI
2. **tui2 must not become an alternate spec-kit implementation**
3. **Any spec-kit/Stage0 capability** must live in shared core crates (single implementation) before either UI consumes it
4. **Do not port spec_kit into tui2** - that creates duplication

### tui2 Maintenance Policy

- tui2 is **non-default** and explicitly marked experimental/upstream scaffold
- Keep it **building cleanly** (0 errors, 0 warnings)
- Keep stub inventory documented (`docs/SPEC-TUI2-STUBS.md`)
- **Do not add** spec-kit/orchestrator features directly into tui2
- Any golden-path functionality must land in shared core first

### Success Criteria for tui2 (as scaffold)

- [x] Always builds
- [x] Launches reliably
- [x] Can run basic chat loops
- [ ] Stays close to upstream with minimal fork-only hacks
- [ ] Acts as reference for backports

---

## Backport Candidates

Features worth cherry-picking from tui2 to tui:

| Feature | Value | Effort | Priority |
|---------|-------|--------|----------|
| Viewport-based terminal architecture | High | High | Future |
| Frame scheduling / rendering separation | Medium | Medium | Consider |
| Desktop notification behavior | Medium | Low | Good candidate |
| Keybinding hints, pager overlay | Low | Low | Nice-to-have |

---

## Re-evaluation Trigger

Only revisit "tui2 as replacement" when:
1. spec-kit/orchestrator is extracted into shared core crates
2. tui2 can consume shared orchestration without duplication
3. tui2 achieves command parity with tui

Until then, this ADR stands.

---

## References

- `docs/SPEC-TUI2-STUBS.md` - Complete stub inventory
- `codex-rs/tui2/src/compat.rs` - Compatibility shims
- `codex-rs/tui/src/chatwidget/spec_kit/` - spec-kit system (tui-only)
- `HANDOFF.md` - SYNC-028 session history

---

## Changelog

| Date | Change |
|------|--------|
| 2025-12-24 | Initial decision after SYNC-028 S13 |
