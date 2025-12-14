# P107 Continuation Prompt - Research API & Ask Polish

## Pre-flight

```bash
# 1. Verify P106 commit
cd ~/code && git log -1 --oneline
# Expected: b8f98b1b6 feat(architect): add artifact chunking...

# 2. Build
./build-fast.sh

# 3. Verify chunking works (optional smoke test)
./codex-rs/target/dev-fast/code architect service start
./codex-rs/target/dev-fast/code architect sources upload -y
./codex-rs/target/dev-fast/code architect service stop
```

---

## Context Summary

### P106 Completed (Commit b8f98b1b6)
| Component | Status | Location |
|-----------|--------|----------|
| Artifact chunking | DONE | `core/src/architect/chunker.rs` |
| XML/Mermaid/Line strategies | DONE | 200KB safe limit |
| Sources upload with chunking | DONE | `cli/src/architect_cmd.rs` |
| Bug documentation | DONE | `docs/NOTEBOOKLM-BUG-REPORT.md` |

### Upstream Bug Status
- **notebooklm-mcp**: Size-dependent text upload failure (>250KB)
- **Root cause**: `page.fill()` performance with large text
- **Workaround**: Chunking at 200KB (implemented in P106)
- **Fix suggested**: Direct DOM manipulation in `enterText()`

---

## P107 Scope (User Confirmed)

### Phase 1: Research API Integration (Full Suite)

Add support for NotebookLM's research capabilities:

**New Commands:**
```bash
code architect research fast "query"     # Quick parallel search
code architect research deep "query"     # Multi-step autonomous research
code architect research status           # Check research progress
code architect research results          # Get latest research results
code architect research import           # Import results as notebook sources
```

**Files to Create/Modify:**
- `core/src/architect/research.rs` - Research API client (NEW)
- `core/src/architect/mod.rs` - Add research export
- `cli/src/architect_cmd.rs` - Add Research subcommand

**notebooklm-mcp API Endpoints:**
```
POST /api/research/fast    { query, notebook, wait?, timeout_ms? }
POST /api/research/deep    { query, notebook, wait?, edit_plan?, timeout_ms? }
GET  /api/research/status  { notebook }
GET  /api/research/results { notebook, format? }
POST /api/research/import  { notebook, source_ids[]? }
```

### Phase 2: Ask Command Polish (All Three)

**2a. Budget Warnings**
- Warning at 80% hourly budget consumed
- Block at 100% with clear error message
- Show remaining queries in status output

**2b. Cache TTL**
- Configurable TTL for cached answers (default: 24h)
- `--no-cache` flag to bypass cache
- Cache invalidation on source upload

**2c. Session Reuse**
- Keep NotebookLM session warm between queries
- Session timeout handling (re-auth if expired)
- Connection pooling for service requests

**Files to Modify:**
- `core/src/architect/budget.rs` - Add warning thresholds
- `core/src/architect/nlm_service.rs` - Session management, cache TTL
- `cli/src/architect_cmd.rs` - Enhanced ask flow with warnings

---

## Implementation Order

1. **Research API** (Phase 1)
   - [ ] Create `research.rs` with API client
   - [ ] Add Research subcommand to CLI
   - [ ] Implement `fast` command
   - [ ] Implement `deep` command
   - [ ] Implement `status` command
   - [ ] Implement `results` command
   - [ ] Implement `import` command

2. **Ask Polish** (Phase 2)
   - [ ] Add budget warning thresholds (80%/100%)
   - [ ] Implement cache TTL with expiration
   - [ ] Add `--no-cache` flag
   - [ ] Implement session reuse/warming
   - [ ] Update status output with remaining queries

---

## Success Criteria

1. **Research API**
   - `code architect research fast "query"` returns results
   - `code architect research deep "query"` runs autonomous research
   - Research results can be imported as notebook sources
   - Budget tracking applies to research queries

2. **Ask Polish**
   - Warning displayed at 80% budget
   - Block with clear message at 100%
   - Cache respects TTL (stale answers re-fetched)
   - `--no-cache` bypasses cache entirely
   - Repeated queries reuse session (faster)

---

## Test Plan

```bash
# Phase 1: Research API
code architect service start
code architect research fast "Rust error handling best practices"
code architect research status
code architect research results
code architect research deep "How should we structure the TUI module?"
code architect research import
code architect sources list  # Should show imported research

# Phase 2: Ask Polish
code architect ask "What is the architecture?" --no-cache
code architect status  # Check budget remaining

# Simulate 80% usage (manually edit budget.json for testing)
code architect ask "Another question"  # Should show warning

code architect service stop
```

---

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────────┐
│                         codex-rs CLI                            │
│  code architect ask / research / sources / service / status     │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                   core/src/architect/                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  budget.rs   │  │ nlm_service  │  │    research.rs       │  │
│  │  (P105) ✓    │  │  (P105) ✓    │  │    (P107) ← NEW      │  │
│  │  + warnings  │  │  + session   │  │                      │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │
│  │  chunker.rs  │  │  mermaid.rs  │  │    churn.rs          │  │
│  │  (P106) ✓    │  │  (P104) ✓    │  │    (P103) ✓          │  │
│  └──────────────┘  └──────────────┘  └──────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼ HTTP (localhost:3456)
┌─────────────────────────────────────────────────────────────────┐
│                    notebooklm-mcp service                       │
│              /api/ask  /api/research/*  /api/sources            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Files Summary

| File | Action | Description |
|------|--------|-------------|
| `core/src/architect/research.rs` | CREATE | Research API client |
| `core/src/architect/mod.rs` | MODIFY | Add research export |
| `core/src/architect/budget.rs` | MODIFY | Add warning thresholds |
| `core/src/architect/nlm_service.rs` | MODIFY | Session management, cache TTL |
| `cli/src/architect_cmd.rs` | MODIFY | Research subcommand, ask polish |

---

## Quick Start

```
P107 - Research API & Ask Polish

## Pre-flight
1. Verify: git log -1 --oneline (expect b8f98b1b6)
2. Build: ./build-fast.sh
3. Read: docs/PROMPT-P107.md

## Phase 1: Research API
Create core/src/architect/research.rs with:
- ResearchClient struct
- fast(), deep(), status(), results(), import() methods
- Budget integration

Add Research subcommand to CLI

## Phase 2: Ask Polish
- Budget warnings at 80%/100%
- Cache TTL (default 24h, --no-cache flag)
- Session reuse between queries
```
