# S32 Implementation Evidence: Source-Based Tier2 Architecture

**Session:** S32
**Date:** 2025-12-27
**Status:** ✅ Implementation Complete, E2E Pending

---

## Summary

Implemented source-based Tier2 architecture to fix NotebookLM's ~2,000 character chat query limit. Instead of embedding SPEC + TASK_BRIEF in the query, we now upsert them as sources and send a minimal ~350 char query.

## Commits

| Repository | Commit | Description |
|------------|--------|-------------|
| notebooklm-client | `3f4464b` | feat(api): Add POST /api/sources/upsert endpoint |
| codex-rs | `04d042a47` | feat(tier2): Implement source-based Tier2 architecture |
| codex-rs | `08e4f9657` | docs(handoff): S33 prompt |

---

## Implementation Details

### 1. Upsert API (`notebooklm-client`)

**Endpoint:** `POST /api/sources/upsert`

**Request:**
```json
{
  "notebook": "code-project-docs",
  "name": "CURRENT_SPEC",
  "content": "# SPEC: SPEC-XXX\n\n<spec content>"
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "name": "CURRENT_SPEC",
    "action": "created" | "updated",
    "sourceType": "text",
    "processingTimeMs": 9000
  }
}
```

**Fuzzy Title Matching:**
NotebookLM auto-generates semantic titles from content (e.g., "S32_TEST" → "S32 Implementation Test Source"). The upsert algorithm handles this:

1. Extract key words from name (split on `_`, `-`, `.`)
2. Find source where title contains ≥66% of key words
3. If found: delete existing source
4. Add new source with content

**Key File:** `~/notebooklm-client/src/service/handlers/sources.ts`

### 2. Tier2 Flow (`codex-rs`)

**File:** `codex-rs/tui/src/stage0_adapters.rs`

```rust
// Step 1: Upsert CURRENT_SPEC source
upsert_source_blocking(&client, &base_url, &notebook, "CURRENT_SPEC", &spec_content)?;

// Step 2: Upsert CURRENT_TASK_BRIEF source
upsert_source_blocking(&client, &base_url, &notebook, "CURRENT_TASK_BRIEF", &brief_content)?;

// Step 3: Send minimal query
let prompt = build_tier2_prompt(&spec_id, &spec_content, &task_brief_md);
// prompt is ~350 chars, references sources by name
```

### 3. Minimal Prompt (`codex-rs`)

**File:** `codex-rs/stage0/src/tier2.rs`

```rust
pub fn build_tier2_prompt(spec_id: &str, _spec_content: &str, _task_brief_md: &str) -> String {
    format!(
        r#"Analyze {} using the CURRENT_SPEC and CURRENT_TASK_BRIEF sources.

Using all sources (Architecture Bible, Bug Retros, Project Diary), provide:
1. **Summary**: 3-5 bullets on what this implements
2. **Risks**: Key risks with mitigations
3. **Architecture**: Relevant patterns from your sources
4. **History**: Related decisions or past issues

Be specific. Cite sources. Under 1000 words."#,
        spec_id
    )
}
```

**Length:** ~350 characters (well under 2k limit)

---

## Acceptance Criteria Status

| ID | Criterion | Status | Evidence |
|----|-----------|--------|----------|
| A1 | Source upsert API exists | ✅ | `curl POST /api/sources/upsert` returns 200 |
| A2 | CURRENT_SPEC.md upserted | ✅ | Trace log shows upsert call |
| A3 | CURRENT_TASK_BRIEF.md upserted | ✅ | Trace log shows upsert call |
| A4 | Query < 500 chars | ✅ | `build_tier2_prompt` returns ~350 chars |
| A5 | Valid Divine Truth returned | 🔄 | E2E test pending (S33) |
| A6 | Source count bounded | ✅ | Upsert updates in place |

---

## Test Commands

```bash
# Check service health
curl -s http://127.0.0.1:3456/health/ready | jq

# Test upsert API
curl -s -X POST "http://127.0.0.1:3456/api/sources/upsert" \
  -H "Content-Type: application/json" \
  -d '{"notebook":"code-project-docs","name":"TEST","content":"test"}' | jq

# List sources
curl -s "http://127.0.0.1:3456/api/sources?notebook=code-project-docs" | jq '.data.sources | map(.title)'

# Check trace log
tail -f /tmp/speckit-trace.log

# Check prompt size
cat /tmp/tier2-prompt.txt | wc -c
```

---

## Notebook State

**Notebook:** `code-project-docs` (ID: 4e80974f-789d-43bd-abe9-7b1e76839506)

**Static Sources (6):**
1. Divine Truth Tier 2 SPEC Analysis Framework ← NL_TIER2_TEMPLATE
2. The Essence of New Source Testing
3. NotebookLM Tier2 Architectural Decisions and Milestone Log
4. Protocol for Active Testing Specifications
5. TUI v2 Port Stub and Compatibility Tracking Document
6. The Codex TUI Dogfooding Protocol

**Dynamic Sources (created at runtime):**
- CURRENT_SPEC (upserted before each Tier2 query)
- CURRENT_TASK_BRIEF (upserted before each Tier2 query)

---

## Next Steps (S33)

1. Run `/speckit.auto SPEC-DOGFOOD-001` for E2E validation
2. Verify A5: DIVINE_TRUTH.md contains real NotebookLM output
3. Complete SPEC-DOGFOOD-001 A2/A3/A4 acceptance criteria
4. Mark both SPECs as complete
