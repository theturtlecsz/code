# Session 32 Prompt - Source-Based Tier2 Architecture

**Last updated:** 2025-12-27
**Status:** S31 Complete - Async Stage0 implemented, Tier2 needs source-based architecture
**Primary SPEC:** SPEC-TIER2-SOURCES (implements fix for SPEC-DOGFOOD-001 A2/A3)

---

## Session 31 Accomplishments

| Item | Status | Commit |
|------|--------|--------|
| Async Stage0 (TUI responsive) | ✅ IMPLEMENTED | `220832042` |
| Tier2 prompt reduction (~1.8k chars) | ✅ IMPLEMENTED | `08d892178` |
| TUI Blocking Audit | ✅ COMPLETE | `docs/SPEC-DOGFOOD-001/evidence/TUI_BLOCKING_AUDIT.md` |
| Async Stage0 Design Doc | ✅ UPDATED | `docs/SPEC-DOGFOOD-001/evidence/ASYNC_STAGE0_DESIGN.md` |
| SPEC-TIER2-SOURCES created | ✅ COMPLETE | `docs/SPEC-TIER2-SOURCES/spec.md` |
| S31 milestone in local-memory | ✅ STORED | importance=8, type=discovery |

### Key Discovery (S31)
**NotebookLM chat query limit is ~2,000 chars** (not 10k - that's Custom Instructions).
The correct pattern is **source-based**: upsert SPEC + TASK_BRIEF as sources, send minimal query.

---

## Session 32 Scope: SPEC-TIER2-SOURCES Implementation

### Decision Record
- **Tier2 Path:** Option B - Source architecture (orchestrated by codex-rs, notebooklm-mcp as primitives)
- **NL Config:** No UI Custom Instructions dependency - template as versioned source document
- **Boundary:** `notebooklm-mcp` gets primitive enhancements only; Stage0 orchestrates

### Architecture

```
BEFORE (broken - 10k+ chars):
  Query = instructions + SPEC + TASK_BRIEF → ❌ Submit disabled

AFTER (source-based - ~100 chars):
  Sources = [NL_TIER2_TEMPLATE.md, CURRENT_SPEC.md, CURRENT_TASK_BRIEF.md]
  Query = "Generate Divine Truth for SPEC-X using sources" → ✅ Works
```

---

## Implementation Phases

### Phase 1: NotebookLM Service Enhancement (`notebooklm-mcp`)

Add source upsert primitive:

```typescript
// POST /api/sources/upsert
{
  "notebook": "code-project-docs",
  "name": "CURRENT_SPEC.md",      // Fixed name for upsert
  "content": "<spec content>"
}

// Response
{
  "success": true,
  "sourceId": "...",
  "action": "created" | "updated"
}
```

Implementation notes:
- Find source by name in notebook
- If exists: update content
- If not: create new source
- Use fixed names to stay within 50-source limit

### Phase 2: Stage0 Tier2 Orchestration (`codex-rs`)

Update `stage0_adapters.rs` to:

1. **Upsert sources before query:**
   ```rust
   // POST /api/sources/upsert with CURRENT_SPEC.md
   // POST /api/sources/upsert with CURRENT_TASK_BRIEF.md
   ```

2. **Send minimal query:**
   ```
   Generate Divine Truth Brief for {spec_id}.
   Use sources CURRENT_SPEC.md and CURRENT_TASK_BRIEF.md.
   Follow NL_TIER2_TEMPLATE.md format exactly.
   Output only the 6-section brief.
   ```

3. **Cache by content hash:**
   ```rust
   cache_key = hash(spec_content + task_brief + notebook_id)
   ```

### Phase 3: One-Time Notebook Setup

Add static source to `code-project-docs` notebook:
- `NL_TIER2_TEMPLATE.md` - Contains 6-section output format (from current `tier2.rs` template)

This is a one-time manual step or can be automated via `/api/sources/upsert`.

---

## Files to Modify

| File | Changes |
|------|---------|
| `~/notebooklm-client/src/routes/sources.ts` | Add `/api/sources/upsert` endpoint |
| `codex-rs/tui/src/stage0_adapters.rs` | Call upsert before query, minimal query format |
| `codex-rs/stage0/src/tier2.rs` | Simplify `build_tier2_prompt()` to ~100 chars |

---

## Acceptance Criteria

### SPEC-TIER2-SOURCES
| ID | Criterion | Validation |
|----|-----------|------------|
| A1 | Source upsert API exists | `POST /api/sources/upsert` returns 200 |
| A2 | CURRENT_SPEC.md upserted | Trace log shows upsert call |
| A3 | CURRENT_TASK_BRIEF.md upserted | Trace log shows upsert call |
| A4 | Query < 500 chars | `wc -c /tmp/tier2-prompt.txt` < 500 |
| A5 | Valid Divine Truth returned | `DIVINE_TRUTH.md` has real content |
| A6 | Source count bounded | Notebook has ≤10 dynamic sources |

### SPEC-DOGFOOD-001 (Unblocked by above)
| ID | Criterion | Status |
|----|-----------|--------|
| A0 | No Surprise Fan-Out | ✅ PASS |
| A1 | Doctor Ready | ✅ PASS |
| A2 | Tier2 Used | ⏳ Blocked on SPEC-TIER2-SOURCES |
| A3 | Evidence Exists | ⏳ Blocked on SPEC-TIER2-SOURCES |
| A4 | System Pointer | ⏳ Needs validation |
| A5 | GR-001 Enforcement | ✅ PASS |
| A6 | Slash Dispatch Single-Shot | ✅ PASS |
| UX1-3 | Async Stage0 | ✅ PASS |

---

## Session 32 Checklist

```
Phase 1: notebooklm-mcp
[ ] 1. Add POST /api/sources/upsert endpoint
[ ] 2. Implement find-by-name + create/update logic
[ ] 3. Test manually: curl -X POST .../api/sources/upsert
[ ] 4. Restart service (systemctl --user restart notebooklm)

Phase 2: codex-rs Stage0
[ ] 5. Update stage0_adapters.rs to call upsert before query
[ ] 6. Simplify build_tier2_prompt() to minimal format
[ ] 7. Add content-hash caching
[ ] 8. Build: ~/code/build-fast.sh

Phase 3: Validation
[ ] 9. Add NL_TIER2_TEMPLATE.md to notebook (one-time)
[ ] 10. Run: /speckit.auto SPEC-DOGFOOD-001
[ ] 11. Verify: wc -c /tmp/tier2-prompt.txt < 500
[ ] 12. Verify: cat /tmp/speckit-trace.log shows SUCCESS
[ ] 13. Verify: DIVINE_TRUTH.md has real content
[ ] 14. Validate A4: lm search "SPEC-DOGFOOD-001"

Phase 4: Completion
[ ] 15. Mark SPEC-TIER2-SOURCES acceptance criteria
[ ] 16. Mark SPEC-DOGFOOD-001 A2/A3/A4
[ ] 17. Commit all changes
[ ] 18. Update HANDOFF.md for S33
```

---

## Quick Start

```bash
# Read the SPEC first
cat docs/SPEC-TIER2-SOURCES/spec.md

# Check NotebookLM service
curl -s http://127.0.0.1:3456/health/ready | jq

# Start with notebooklm-mcp changes
cd ~/notebooklm-client
# Implement /api/sources/upsert
```

---

## Reference: NotebookLM Limits

| Input Type | Limit | Use For |
|------------|-------|---------|
| Chat query | ~2,000 chars | Minimal query only |
| Custom instructions | 10,000 chars | Human convenience (not pipeline) |
| Source document | 500k words / 200MB | SPEC, TASK_BRIEF, templates |
| Sources per notebook | 50 (free) / 300 (premium) | Use upsert to stay bounded |
| Daily queries | 50 (free) / 500 (premium) | Cache by content hash |

---

## Constraints

- `notebooklm-mcp`: Primitive enhancements only (upsert API), no orchestration logic
- `codex-rs`: Orchestrates source upsert + query flow
- Template stored as **source document**, not UI Custom Instructions
- Cache Tier2 responses to respect daily quota
