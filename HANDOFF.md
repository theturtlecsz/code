# Session 33 Prompt - E2E Validation

**Last updated:** 2025-12-27
**Status:** S32 Complete - Source-based Tier2 implemented, E2E validation pending
**Primary SPEC:** SPEC-DOGFOOD-001 (A2/A3/A4 validation)

---

## Session 32 Accomplishments

| Item | Status | Commit |
|------|--------|--------|
| `POST /api/sources/upsert` API | ✅ IMPLEMENTED | notebooklm-client `3f4464b` |
| Tier2HttpAdapter upsert flow | ✅ IMPLEMENTED | codex-rs `04d042a47` |
| Minimal prompt (~350 chars) | ✅ IMPLEMENTED | codex-rs `04d042a47` |
| NL_TIER2_TEMPLATE source | ✅ ADDED | "Divine Truth Tier 2 SPEC Analysis Framework" |
| SPEC-TIER2-SOURCES A1-A4,A6 | ✅ COMPLETE | Only A5 (E2E) pending |

### Key Implementation Details (S32)

**Upsert API** (`notebooklm-client`):
- Fuzzy title matching: NotebookLM generates semantic titles from content
- Algorithm: list sources → find by key words → delete if exists → add new
- Returns `action: "created" | "updated"`

**Tier2 Flow** (`codex-rs`):
1. Upsert CURRENT_SPEC source (prepends spec_id as heading)
2. Upsert CURRENT_TASK_BRIEF source (prepends spec_id as heading)
3. Send minimal query: "Analyze SPEC-X using the CURRENT_SPEC and CURRENT_TASK_BRIEF sources"

---

## Session 33 Scope: E2E Validation

### Option A: Full Validation (Recommended)

Run `/speckit.auto SPEC-DOGFOOD-001` and validate:

| Criterion | Validation Command | Expected |
|-----------|-------------------|----------|
| A2: Tier2 Used | `grep tier2 /tmp/speckit-trace.log` | Shows upsert + query calls |
| A3: Evidence | `ls -la docs/SPEC-DOGFOOD-001/evidence/` | TASK_BRIEF.md, DIVINE_TRUTH.md exist |
| A4: System Pointer | `lm search "SPEC-DOGFOOD-001"` | Returns memory with system:true |

### Option B: Quick Smoke Test

```bash
# 1. Check trace log during Tier2
tail -f /tmp/speckit-trace.log

# 2. Verify prompt size
cat /tmp/tier2-prompt.txt | wc -c  # Should be < 500

# 3. Test upsert API directly
curl -s -X POST "http://127.0.0.1:3456/api/sources/upsert" \
  -H "Content-Type: application/json" \
  -d '{"notebook":"code-project-docs","name":"TEST_SMOKE","content":"test"}' | jq

# 4. Verify sources
curl -s "http://127.0.0.1:3456/api/sources?notebook=code-project-docs" | jq '.data.sources | map(.title)'
```

---

## Session 33 Checklist

```
Validation
[ ] 1. Run /speckit.auto SPEC-DOGFOOD-001
[ ] 2. Verify A2: Tier2 logs show upsert calls
[ ] 3. Verify A3: Evidence files exist
[ ] 4. Verify A4: lm search returns system pointer
[ ] 5. Verify A5: DIVINE_TRUTH.md has real content

Cleanup
[ ] 6. Mark SPEC-TIER2-SOURCES A5 as complete
[ ] 7. Mark SPEC-DOGFOOD-001 A2/A3/A4 as complete
[ ] 8. Update SPEC.md status

Documentation
[ ] 9. Store S32 milestone in local-memory
[ ] 10. Update HANDOFF.md for S34
```

---

## Current NotebookLM Sources

```
code-project-docs (6 sources):
1. Divine Truth Tier 2 SPEC Analysis Framework  ← NL_TIER2_TEMPLATE
2. The Essence of New Source Testing
3. NotebookLM Tier2 Architectural Decisions and Milestone Log
4. Protocol for Active Testing Specifications
5. TUI v2 Port Stub and Compatibility Tracking Document
6. The Codex TUI Dogfooding Protocol

Dynamic sources (created at runtime):
- CURRENT_SPEC (upserted before each query)
- CURRENT_TASK_BRIEF (upserted before each query)
```

---

## Key Files

| Location | Purpose |
|----------|---------|
| `~/notebooklm-client/src/service/handlers/sources.ts` | `handleUpsertSource` - fuzzy matching upsert |
| `~/code/codex-rs/tui/src/stage0_adapters.rs` | `Tier2HttpAdapter` - upsert + query flow |
| `~/code/codex-rs/stage0/src/tier2.rs` | `build_tier2_prompt()` - minimal prompt |
| `/tmp/speckit-trace.log` | Runtime trace log |
| `/tmp/tier2-prompt.txt` | Last prompt sent |

---

## Constraints

- **notebooklm-mcp = primitives only**: Upsert API, no orchestration logic
- **codex-rs = orchestration**: Manages source lifecycle, caching
- **Query limit**: ~2,000 chars (current prompt: ~350 chars)
- **Source limit**: 50 sources per notebook (using upsert to stay bounded)
