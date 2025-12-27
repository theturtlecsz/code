# Session 32 Prompt - Tier2 Source Architecture + Dogfood Completion

**Last updated:** 2025-12-27
**Status:** Session 31 Complete - Async Stage0 working, Tier2 needs source-based architecture
**Current SPECs:** SPEC-DOGFOOD-001 (validation), SPEC-TIER2-SOURCES (new)

---

## Session 31 Summary

### Completed
1. **Async Stage0** - TUI remains responsive during Stage0 execution
   - Commit `220832042` - Full implementation
   - Tested and confirmed working

2. **Tier2 Prompt Fix** - Reduced prompt to fit ~2k chat limit
   - Commit `08d892178` - Minimal prompt format
   - Root cause identified: NotebookLM chat query limit is ~2k chars

3. **Root Cause Analysis** - NotebookLM architecture mismatch
   - Query limit: ~2,000 chars (hard limit)
   - Custom instructions: 10,000 chars (for static instructions only)
   - Correct pattern: Put dynamic content as **sources**, not in query

4. **New SPEC Created** - `SPEC-TIER2-SOURCES`
   - Source-based Tier2 architecture design
   - Requires `notebooklm-mcp` changes first

### Pending
- **SPEC-DOGFOOD-001 A2/A3** - Tier2 validation (blocked on source architecture)
- **SPEC-TIER2-SOURCES** - Implementation

---

## Session 32 Priority Options

### Option A: Complete Dogfood with Minimal Tier2 (Quick)
1. Test current minimal prompt (~1.8k chars)
2. Verify Tier2 works end-to-end
3. Mark SPEC-DOGFOOD-001 complete
4. Defer SPEC-TIER2-SOURCES to later session

### Option B: Implement Source Architecture First (Thorough)
1. Add `upsertSource` API to `notebooklm-mcp`
2. Update Stage0 to use source-based queries
3. Full Tier2 validation
4. Mark both SPECs complete

---

## Quick Start (Option A)

```bash
# Test minimal Tier2 prompt
rm -f /tmp/speckit-trace.log /tmp/tier2-prompt.txt
~/code/build-fast.sh run
# In TUI: /speckit.auto SPEC-DOGFOOD-001

# Verify
wc -c /tmp/tier2-prompt.txt  # Should be < 2000
cat /tmp/speckit-trace.log   # Should show Tier2 SUCCESS
head -50 docs/SPEC-DOGFOOD-001/evidence/DIVINE_TRUTH.md
```

---

## SPEC-TIER2-SOURCES Implementation (Option B)

### Phase 1: NotebookLM Service (`notebooklm-mcp`)

Add source upsert API:
```typescript
// POST /api/sources/upsert
{
  "notebook": "code-project-docs",
  "name": "CURRENT_SPEC.md",
  "content": "<content>"
}
```

### Phase 2: Stage0 Tier2 (`codex-rs`)

Update `stage0_adapters.rs`:
1. Call `POST /api/sources/upsert` with CURRENT_SPEC.md
2. Call `POST /api/sources/upsert` with CURRENT_TASK_BRIEF.md
3. Send minimal query (~100 chars)

New query format:
```
Generate Divine Truth Brief for {spec_id}.
Use sources CURRENT_SPEC.md and CURRENT_TASK_BRIEF.md.
Follow NL_TIER2_TEMPLATE.md format.
```

### Phase 3: Notebook Setup

Add one-time source: `NL_TIER2_TEMPLATE.md` with output format.

---

## Key Files

| File | Purpose |
|------|---------|
| `docs/SPEC-TIER2-SOURCES/spec.md` | Source architecture spec |
| `codex-rs/stage0/src/tier2.rs` | Prompt building |
| `codex-rs/tui/src/stage0_adapters.rs` | Tier2 HTTP calls |
| `~/notebooklm-client/` | NotebookLM service |

---

## Acceptance Criteria Status

### SPEC-DOGFOOD-001
| ID | Criterion | Status |
|----|-----------|--------|
| A0 | No Surprise Fan-Out | ✅ PASS |
| A1 | Doctor Ready | ✅ PASS |
| A2 | Tier2 Used | ⏳ TEST with minimal prompt |
| A3 | Evidence Exists | ⏳ TEST |
| A4 | System Pointer | ⏳ TEST |
| A5 | GR-001 Enforcement | ✅ PASS |
| A6 | Slash Dispatch Single-Shot | ✅ PASS |
| UX1 | Blocking audit | ✅ PASS |
| UX2 | Design doc | ✅ PASS |
| UX3 | Async Stage0 | ✅ PASS |

### SPEC-TIER2-SOURCES (New)
| ID | Criterion | Status |
|----|-----------|--------|
| A1 | Source upsert API | ❌ PENDING |
| A2 | CURRENT_SPEC.md upserted | ❌ PENDING |
| A3 | CURRENT_TASK_BRIEF.md upserted | ❌ PENDING |
| A4 | Query < 500 chars | ❌ PENDING |
| A5 | Valid Divine Truth | ❌ PENDING |
| A6 | Source count bounded | ❌ PENDING |

---

## Session 32 Checklist

```
[ ] 1. Decide: Option A (quick) or Option B (thorough)
[ ] 2. If Option A: Test minimal Tier2, verify DIVINE_TRUTH.md
[ ] 3. If Option B: Implement source upsert in notebooklm-mcp
[ ] 4. Validate SPEC-DOGFOOD-001 A2/A3/A4
[ ] 5. Commit and update HANDOFF.md
```

---

## NotebookLM Limits Reference

| Input Type | Limit | Use For |
|------------|-------|---------|
| Chat query | ~2,000 chars | Minimal query only |
| Custom instructions | 10,000 chars | Static persona/format |
| Source document | 500k words / 200MB | SPEC, TASK_BRIEF, templates |
| Sources per notebook | 50 (free) / 300 (premium) | Use upsert to stay bounded |
| Daily queries | 50 (free) / 500 (premium) | Cache by content hash |
