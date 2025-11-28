# SPEC-958 Session 12: Test Documentation & Decision Capture

## Context

**Previous Session**: Session 11 completed SPEC-957 investigation
**Focus**: Documentation of test infrastructure and architectural decisions
**Mode**: CLEARFRAME Documentation Mode

---

## Session 11 Summary

### Accomplished
- Fixed 2 tests: `prompt_tools_are_consistent_across_requests`, `prefixes_context_and_instructions_once_and_consistently_across_requests`
- Documented root causes for all 12 remaining ignored tests
- Updated SPEC-958-test-migration.md with accurate blockers

### Current Test Status
```
codex-core: 31 passed, 0 failed, 12 ignored
```

### Decision Log (Session 11)
| Item | Decision | Rationale |
|------|----------|-----------|
| Token-based auto-compact | Leave ignored | Feature not implemented; no current need |
| compact_resume_fork tests | Leave ignored | Payload structure evolved significantly |
| Primary focus | Documentation | Capture decisions, create test architecture docs |

---

## Session 12 Objectives

### Phase 1: Update SPEC-958-test-migration.md (Final State)

**Tasks**:
1. Add "Final Decisions" section capturing Session 11 decisions
2. Update test counts and categorization
3. Add "Future Work" section for deferred items
4. Mark document as **SPEC-958 COMPLETE** with final status

### Phase 2: Create TEST-ARCHITECTURE.md

**Location**: `docs/testing/TEST-ARCHITECTURE.md`

**Sections to document**:
1. **Test Organization**
   - `core/tests/suite/` structure
   - Test naming conventions
   - Fixture locations (`tests/fixtures/`)

2. **Test Categories**
   - Unit tests (inline `#[cfg(test)]` modules)
   - Integration tests (`tests/suite/*.rs`)
   - Stubbed tests (documented gaps)
   - Ignored tests (with blockers)

3. **Mock Infrastructure**
   - wiremock usage patterns
   - SSE response builders (`core_test_support::responses`)
   - Fixture loading (`load_sse_fixture_with_id`)

4. **Fork-Specific Testing Notes**
   - Tools list differences (browser, agent, web tools)
   - Payload structure evolution (5 messages vs upstream 3)
   - Role changes (`developer` → `user`)

5. **Running Tests**
   - Commands and flags
   - Environment variables (`CODEX_SANDBOX_NETWORK_DISABLED_ENV_VAR`)
   - CI considerations

### Phase 3: Document Fork Divergences

**Location**: `docs/FORK-DIVERGENCES.md`

**Purpose**: Document where fork behavior differs from upstream in ways that affect tests

**Sections**:
1. **Request Payload Structure**
   - Base instructions message
   - Environment context format (JSON vs XML evolution)
   - User instructions block
   - System status message
   - Role assignments

2. **Tool Registry**
   - Fork-added tools (browser_*, agent_*, web_*, search, history_search)
   - Removed/changed tools (apply_patch, view_image)

3. **Auto-Compact Behavior**
   - Upstream expectation: Token-count-based triggering
   - Fork reality: Error-message-based triggering only
   - `model_auto_compact_token_limit` config exists but unused

4. **API Differences**
   - `Op::GetPath` removed (use `SessionConfiguredEvent.rollout_path`)
   - `Op::UserTurn` removed (use `Op::UserInput`)
   - `Op::OverrideTurnContext` partial implementation

### Phase 4: Update CLAUDE.md Section on Testing

Add a concise testing section to CLAUDE.md:
- How to run tests
- What the ignored tests mean
- Where to find test documentation

---

## Files to Create/Update

| File | Action | Priority |
|------|--------|----------|
| `docs/SPEC-958-test-migration.md` | Update final state | P0 |
| `docs/testing/TEST-ARCHITECTURE.md` | Create new | P1 |
| `docs/FORK-DIVERGENCES.md` | Create new | P1 |
| `CLAUDE.md` | Add testing section | P2 |

---

## Verification

After documentation complete:
1. Run `cargo test -p codex-core --test all` to confirm 31 pass / 12 ignored
2. Verify all new docs are readable and cross-referenced
3. Update SPEC.md to mark SPEC-958 complete

---

## Commands

```bash
# Load context
load ~/.claude/CLEARFRAME.md and docs/SPEC-958-session-12-prompt.md

# Reference files
read docs/SPEC-958-test-migration.md
read codex-rs/core/tests/suite/mod.rs
```

---

## Success Criteria

- [ ] SPEC-958-test-migration.md marked COMPLETE with final decisions
- [ ] TEST-ARCHITECTURE.md created with comprehensive test infrastructure docs
- [ ] FORK-DIVERGENCES.md created documenting payload/tool/behavior differences
- [ ] CLAUDE.md updated with testing guidance
- [ ] All docs cross-referenced and consistent
