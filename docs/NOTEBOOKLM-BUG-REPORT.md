# Bug Report: notebooklm-mcp `add-source` with `type: text` fails

**Status**: OPEN
**Reported**: 2025-12-08
**Affects**: notebooklm-mcp v1.3.0
**Blocking**: P105 sources upload feature

---

## Summary

The `POST /api/sources` endpoint and `add-source --type text` CLI command fail with error "Could not find text input area" when attempting to add text sources to a notebook.

---

## Environment

```
notebooklm-mcp version: 1.3.0
Service mode: HTTP daemon on port 3456
Node.js: (standard installation)
OS: Linux 6.8.12-8-pve
Browser: Chromium (headless)
```

---

## Steps to Reproduce

**Via HTTP API:**
```bash
# Start service
notebooklm service start

# Attempt to add text source
curl -X POST http://127.0.0.1:3456/api/sources \
  -H "Content-Type: application/json" \
  -d '{
    "source_type": "text",
    "content": "This is a simple test source for debugging.",
    "notebook": "codex-rs-architect"
  }'
```

**Via CLI:**
```bash
notebooklm add-source -n codex-rs-architect --type text "Test content" --json
```

---

## Expected Behavior

Source should be added to the notebook successfully.

---

## Actual Behavior

**HTTP Response (200 OK but success: false):**
```json
{
  "success": false,
  "data": {
    "success": false,
    "sourceType": "text",
    "content": "This is a simple test source for debugging.",
    "error": "Could not find text input area"
  }
}
```

**CLI Output:**
```json
{
  "success": false,
  "data": {
    "success": false,
    "sourceType": "text",
    "content": "Test content via CLI",
    "error": "Could not find text input area"
  }
}
```

---

## Working Operations (Same Session)

The following operations work correctly in the same service session:

**1. Health check:**
```json
{
  "status": "ok",
  "service": "notebooklm",
  "version": "1.3.0",
  "sessions": { "active_sessions": 1 }
}
```

**2. List sources:**
```json
{
  "success": true,
  "data": {
    "sourceCount": 7,
    "sources": [
      { "index": 1, "title": "Architectural Decisions & Risks", "status": "ready" },
      { "index": 2, "title": "structure_part_3.md", "status": "ready" },
      ...
    ]
  }
}
```

---

## Suspected Cause

The browser automation in `SourceManager.addSource()` cannot locate the text input area element in the NotebookLM UI. Possible causes:

1. **UI selector changed** - Google may have updated the NotebookLM DOM structure
2. **Modal not opening** - The "Add source" dialog may not be triggering correctly
3. **Race condition** - Element may not be rendered before selector lookup
4. **Text source specific** - The text input flow may differ from URL/file sources

---

## Relevant Code Location

Likely in: `src/lib/source-manager.ts` - the `addSource()` method's text handling branch

---

## Additional Context

- Service was freshly started before testing
- Notebook exists and is accessible (list-sources works)
- Same notebook has existing sources that can be listed
- No browser crashes reported in health check

---

## Workaround

Currently none for text sources via service mode. Manual upload through NotebookLM web UI works.

---

## Impact on codex-rs

This bug blocks the `code architect sources upload` command from uploading generated artifacts (churn matrix, complexity map, repo skeleton) to NotebookLM for context-aware queries.

The P105 infrastructure (budget tracking, service lifecycle, artifact generation) is complete and working. Only the final upload step is blocked by this upstream issue.
