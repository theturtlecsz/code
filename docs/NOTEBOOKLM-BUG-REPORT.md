# Bug Report: notebooklm-mcp `add-source` with `type: text` fails for large content

**Status**: PARTIALLY FIXED (size-dependent)
**Reported**: 2025-12-08
**Updated**: 2025-12-08
**Affects**: notebooklm-mcp v1.3.0
**Blocking**: P106 sources upload feature (large files)

---

## Summary

The `POST /api/sources` endpoint with `type: text` **fails for content larger than ~250KB** with error "Could not find text input area". Small content (<200KB) works reliably.

### Size-Dependent Behavior (P106 Testing)

| Content Size | Result | Processing Time |
|-------------|--------|-----------------|
| Small (~1KB) | ✅ Works | ~14 seconds |
| 100KB | ✅ Works | ~21 seconds |
| 200KB | ✅ Works | ~38 seconds |
| 250KB | ✅ Works | ~91 seconds |
| 300KB | ❌ Fails | ~137 seconds (timeout) |
| 330KB | ❌ Fails | ~182 seconds (timeout) |

**Safe limit**: 200KB (reliable, reasonable timing)

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

The browser automation in `SourceManager.addSource()` has issues with large text content. Likely causes:

1. **Playwright `fill()` timeout** - Large text causes the fill operation to exceed element timeouts
2. **Browser memory pressure** - Processing 300KB+ of text in a textarea may cause UI lag
3. **DOM update delays** - NotebookLM UI may struggle to render large content in real-time
4. **Selector timing** - After filling large content, the save button selector lookup may race

### Evidence

- Small content (1KB) works reliably in ~14 seconds
- Processing time scales non-linearly: 200KB=38s, 250KB=91s
- At ~300KB, the modal appears to become unresponsive (selector lookup fails)
- Error message "Could not find text input area" suggests modal state corruption

---

## Relevant Code Location

`src/sources/source-manager.ts` - the `addTextSource()` method

Key suspect: The `page.fill()` call with large content may need:
1. Chunked input with delays
2. `page.evaluate()` to set textarea value directly
3. Clipboard paste instead of typing

---

## Additional Context

- Service was freshly started before testing
- Notebook exists and is accessible (list-sources works)
- Same notebook has existing sources that can be listed
- No browser crashes reported in health check
- Processing time increases non-linearly with content size

---

## Workaround

**For codex-rs (P106)**:
- Implement chunking at 200KB limit
- Split large artifacts into multiple sources
- Use `[ARCH-1]`, `[ARCH-2]` prefix pattern

**For direct API users**:
- Keep content under 200KB per source
- Split large files before upload

---

## Impact on codex-rs

This limits the `code architect sources upload` command to 200KB per source. Large artifacts must be chunked:

| Artifact | Size | Strategy |
|----------|------|----------|
| Churn Matrix | 12KB | Single source |
| Complexity Map | 9MB | Filter to critical/high only |
| Repo Skeleton | 330KB | Chunk into 2 parts |
| Module Deps | 95KB | Single source |
| Call Graph | 568KB | Chunk into 3 parts |

---

## Upstream Fix Request

**Priority**: Medium (workaround exists)

**Suggested fixes for notebooklm-mcp**:

1. **Quick fix**: Add content size validation with helpful error
   ```typescript
   if (content.length > 250000) {
     throw new Error(`Content too large (${content.length} bytes). Max: 250KB`);
   }
   ```

2. **Better fix**: Use clipboard paste for large content
   ```typescript
   if (content.length > 50000) {
     await page.evaluate((text) => navigator.clipboard.writeText(text), content);
     await textArea.focus();
     await page.keyboard.press('Control+V');
   }
   ```

3. **Best fix**: Direct DOM manipulation
   ```typescript
   await page.evaluate((text) => {
     const textarea = document.querySelector('textarea.mat-mdc-input-element');
     textarea.value = text;
     textarea.dispatchEvent(new Event('input', { bubbles: true }));
   }, content);
   ```
