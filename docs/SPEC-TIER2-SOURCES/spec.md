# Spec: SPEC-TIER2-SOURCES - NotebookLM Source-Based Tier2 Architecture

## Context

The current Tier2 implementation sends the entire prompt (static instructions + SPEC + TASK_BRIEF) as a single chat query to NotebookLM. This exceeds NotebookLM's ~2,000 character chat query limit, causing the submit button to remain disabled.

## Problem

- NotebookLM chat query limit: ~2,000 characters
- Current Tier2 prompt: ~10,000+ characters
- Result: Queries fail silently (submit button disabled)

## Root Cause Analysis

NotebookLM is designed to reason over **sources** and answer with citations. The proper pattern is:
1. Store context as **sources** (up to 500k words each)
2. Store static instructions in **Custom Chat** config (10k limit)
3. Send minimal **queries** that reference sources (~100 chars)

## Solution

### Architecture Change

**Before (broken):**
```
Query = instructions (3k) + SPEC (4k) + TASK_BRIEF (3k) = 10k+ ❌
```

**After (correct):**
```
Sources = CURRENT_SPEC.md + CURRENT_TASK_BRIEF.md (upserted each run)
Custom Instructions = Static "Shadow Staff Engineer" template
Query = "Generate Divine Truth for SPEC-X" (~100 chars) ✅
```

### Required Changes

#### 1. NotebookLM Service (`notebooklm-mcp`)

Add source management API:
```typescript
// Upsert a source document (create or update by name)
POST /api/sources/upsert
{
  "notebook": "code-project-docs",
  "name": "CURRENT_SPEC.md",
  "content": "<spec content>"
}
```

Implementation:
- Find source by name in notebook
- If exists: update content
- If not: create new source
- Use fixed source names to avoid hitting 50-source limit

#### 2. Stage0 Tier2 (`codex-rs/stage0` + `tui/src/stage0_adapters.rs`)

Before sending query:
1. `POST /api/sources/upsert` with CURRENT_SPEC.md
2. `POST /api/sources/upsert` with CURRENT_TASK_BRIEF.md
3. `POST /api/ask` with minimal query

New query format:
```
Generate Divine Truth Brief for {spec_id}.

Use sources CURRENT_SPEC.md and CURRENT_TASK_BRIEF.md.
Follow the output format in NL_TIER2_TEMPLATE.md.
Output only the 6-section brief, no preamble.
```

#### 3. Notebook Setup (one-time provisioning)

Add tracked source document:
- `NL_TIER2_TEMPLATE.md` - Contains the 6-section output format and guardrails

Optionally configure Custom Chat with persona instructions.

### Caching Strategy

Cache Tier2 responses by:
```
cache_key = hash(spec_content + task_brief + constitution_hash + notebook_id)
```

Avoid re-querying during debugging iterations.

## Acceptance Criteria

| ID | Criterion | Validation |
|----|-----------|------------|
| A1 | Source upsert API exists | `POST /api/sources/upsert` returns 200 |
| A2 | CURRENT_SPEC.md upserted before query | Trace log shows upsert call |
| A3 | CURRENT_TASK_BRIEF.md upserted before query | Trace log shows upsert call |
| A4 | Query is < 500 chars | `wc -c /tmp/tier2-prompt.txt` < 500 |
| A5 | Tier2 returns valid Divine Truth | DIVINE_TRUTH.md has real content |
| A6 | Source count stays bounded | Notebook has <= 10 dynamic sources |

## Out of Scope

- Automatic Custom Chat configuration (manual one-time setup)
- Source versioning/history
- Multi-notebook support

## References

- [NotebookLM FAQ](https://support.google.com/notebooklm/answer/16269187)
- [NotebookLM 10k Custom Instructions](https://www.androidauthority.com/notebooklm-chat-customization-upgrade-3622570/)
- [Reddit: Bypassing 2k limit](https://www.reddit.com/r/notebooklm/comments/1jfkvpo/bypassing_notebooklms_chat_window_2000_character/)
