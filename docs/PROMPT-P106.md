# P106 Continuation Prompt - NotebookLM Integration Completion

## Pre-flight

```bash
# 1. Read the bug report fix status
cat ~/notebooklm-mcp/NOTEBOOKLM-README.md

# 2. Verify notebooklm-mcp has been updated
cd ~/notebooklm-mcp && git log -1 --oneline

# 3. Rebuild notebooklm-mcp if needed
npm run build

# 4. Build codex-rs
cd ~/code && ./build-fast.sh

# 5. Test the fix
./codex-rs/target/dev-fast/code architect service start
./codex-rs/target/dev-fast/code architect sources upload -y
./codex-rs/target/dev-fast/code architect service stop
```

---

## Context Summary

### P105 Completed (Commit TBD)
| Component | Status | Location |
|-----------|--------|----------|
| Budget tracking | DONE | `core/src/architect/budget.rs` |
| HTTP service client | DONE | `core/src/architect/nlm_service.rs` |
| Service commands | DONE | `code architect service start/stop/status` |
| Sources commands | DONE | `code architect sources list/upload` |
| Size filtering | DONE | 500KB limit, complexity filtering |

### Blocking Issue (P105)
- **Bug**: `add-source --type text` fails with "Could not find text input area"
- **Location**: notebooklm-mcp `src/lib/source-manager.ts`
- **Tracking**: `docs/NOTEBOOKLM-BUG-REPORT.md`
- **Status**: Check `NOTEBOOKLM-README.md` for fix status

---

## P106 Scope (User Confirmed)

### Phase 1: Verify Bug Fix
```bash
# Test sources upload after notebooklm-mcp fix
code architect service start
code architect sources upload -y
code architect sources list  # Should show [ARCH] prefixed sources
code architect service stop
```

### Phase 2: Research API Integration
Add support for NotebookLM's research capabilities:

**New Commands:**
```bash
code architect research fast "query"   # Quick parallel search
code architect research deep "query"   # Multi-step autonomous research
code architect research status         # Check progress
code architect research import         # Import results as sources
```

**Files to Create/Modify:**
- `core/src/architect/research.rs` - Research API client
- `cli/src/architect_cmd.rs` - Add Research subcommand

**API Endpoints:**
```
POST /api/research/fast   { query, notebook, wait?, timeout_ms? }
POST /api/research/deep   { query, notebook, wait?, edit_plan?, timeout_ms? }
GET  /api/research/status { notebook }
GET  /api/research/results { notebook, format? }
POST /api/research/import { notebook }
```

### Phase 3: Ask Command Polish
Enhance the budget-tracked ask flow:

**Improvements:**
1. Cache-first with TTL awareness
2. Budget warning at 80% with confirmation
3. Session reuse for faster queries
4. Answer quality metrics

**Files to Modify:**
- `cli/src/architect_cmd.rs` - Enhanced ask flow
- `core/src/architect/nlm_service.rs` - Session management

### Phase 4: Artifact Chunking
Smart splitting of large files into multiple sources:

**Strategy:**
```
complexity_map.json (9MB) → Split by risk level:
  - [ARCH] Complexity Critical (files with risk=critical)
  - [ARCH] Complexity High (files with risk=high)
  - [ARCH] Complexity Summary (stats only)

call_graph.mmd (568KB) → Split by module:
  - [ARCH] Call Graph - Core
  - [ARCH] Call Graph - TUI
  - [ARCH] Call Graph - CLI
```

**Files to Create:**
- `core/src/architect/chunker.rs` - Intelligent artifact splitting

---

## Success Criteria

1. **Sources Upload Works**
   - `code architect sources upload` completes without error
   - `code architect sources list` shows [ARCH] prefixed sources
   - Atomic swap works (old [ARCH] sources deleted, new uploaded)

2. **Research Integration**
   - `code architect research fast` returns results
   - Results can be imported as notebook sources
   - Budget tracking applies to research queries

3. **Ask Flow Polish**
   - Cache hits are instant (no budget used)
   - Budget warnings at 80% threshold
   - Clear error messages at 100%

4. **Chunking**
   - Large files split into <500KB chunks
   - Chunks maintain semantic meaning
   - Total source count stays under NotebookLM limit (300)

---

## Test Plan

```bash
# 1. Verify bug fix
code architect service start
code architect sources upload -y
code architect sources list | grep "\[ARCH\]"

# 2. Test ask with budget
code architect ask "What is the main architecture?"
code architect status  # Should show 1 query used

# 3. Test research (Phase 2)
code architect research fast "Rust best practices for error handling"
code architect research status
code architect research import

# 4. Test chunking (Phase 4)
code architect refresh --graph
code architect sources upload -y  # Should chunk large files
code architect sources list       # Count [ARCH] sources

# 5. Cleanup
code architect service stop
```

---

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────────┐
│                         codex-rs CLI                            │
│  code architect ask / refresh / sources / service / research    │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   core/src/architect/                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  budget.rs   │  │ nlm_service  │  │    research.rs       │  │
│  │  (P105) ✓    │  │  (P105) ✓    │  │    (P106) ← NEW      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  churn.rs    │  │  mermaid.rs  │  │    chunker.rs        │  │
│  │  (P103) ✓    │  │  (P104) ✓    │  │    (P106) ← NEW      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼ HTTP (localhost:3456)
┌─────────────────────────────────────────────────────────────────┐
│                    notebooklm-mcp service                       │
│                    (Browser automation)                         │
└─────────────────────────────────────────────────────────────────┘
```

---

## Files Changed in P105

| File | Action | Description |
|------|--------|-------------|
| `core/src/architect/budget.rs` | CREATE | Hourly usage tracking |
| `core/src/architect/nlm_service.rs` | CREATE | HTTP client + service manager |
| `core/src/architect/mod.rs` | MODIFY | Added module exports |
| `core/Cargo.toml` | MODIFY | Added `urlencoding` dependency |
| `cli/src/architect_cmd.rs` | MODIFY | Service, Sources subcommands |
| `docs/NOTEBOOKLM-BUG-REPORT.md` | CREATE | Upstream bug documentation |
| `docs/PROMPT-P106.md` | CREATE | This continuation prompt |
