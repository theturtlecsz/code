# P99 Continuation Prompt

**Session**: SPEC-KIT-103 Librarian v1 Polish + Local-Memory SDK

Copy this to start the next session:

---

**Ultrathink** P99 Session: SPEC-KIT-103 Librarian Polish & Python SDK

Read docs/PROMPT-P99-LIBRARIAN.md for full context.

## Session Goals (Priority Order)

1. **Librarian Polish** - Complete P98 integration gaps
2. **Integration Tests** - Mock for CI + real local-memory tests
3. **Python SDK** - Standalone local-memory client (zero MCP)

## P98 Completed Summary

| Task | Files |
|------|-------|
| LocalMemoryClient trait | `stage0/src/librarian/client.rs` |
| LibrarianMemoryMcpAdapter | `tui/src/stage0_adapters.rs` |
| Audit trail schema | `STAGE0_SCHEMA.sql`, `audit.rs` |
| --apply/--verbose modes | `commands/librarian.rs` |
| RelationshipsClient | `client.rs`, `stage0_adapters.rs` |

## P99 Task Breakdown

### Phase 1: Librarian Polish (Est. 3-4 tasks)

**1.1 Wire Audit Trail to TUI**
- Current: overlay_db not accessible from librarian command
- Solution: Create OverlayDb on-demand in execute_sweep()
- Pattern: See `commands/special.rs:467` for existing pattern
- File: `tui/src/chatwidget/spec_kit/commands/librarian.rs`

**1.2 Wire RelationshipsClient to Sweep**
- Current: Causal edges detected but not written to local-memory
- Solution: When --apply, call create_relationship() for each edge
- File: `tui/src/chatwidget/spec_kit/commands/librarian.rs`

**1.3 Fix /stage0.librarian history Command**
- Current: Command exists but overlay_db not connected
- Solution: Same fix as 1.1 (create OverlayDb on-demand)
- File: `tui/src/chatwidget/spec_kit/commands/librarian.rs`

### Phase 2: Integration Tests (Est. 2-3 tasks)

**2.1 Mock MCP Tests (CI-safe)**
- Create test module in `stage0/src/librarian/tests/`
- Use MockLocalMemoryClient from client.rs
- Test: sweep with retype, sweep with template, idempotency
- File: `stage0/src/librarian/tests/sweep_test.rs`

**2.2 Integration Tests (Manual)**
- Create test script or Rust integration test
- Requires running local-memory daemon
- Test: full sweep against real corpus
- File: `tui/tests/librarian_integration.rs` or script

### Phase 3: Python SDK (Est. 4-5 tasks)

**3.1 Create Package Structure**
```
local-memory-sdk/
├── pyproject.toml
├── src/local_memory/
│   ├── __init__.py
│   ├── client.py      # REST API wrapper
│   ├── cli.py         # CLI binary wrapper
│   ├── types.py       # Memory, Relationship types
│   └── exceptions.py
└── tests/
```

**3.2 Implement REST Client**
- Wrap http://localhost:4080/api/v1/* endpoints
- Search, store, get, update, delete
- Relationships: create, discover, related

**3.3 Implement CLI Wrapper**
- Shell out to `local-memory` binary for speed
- Fallback to REST if binary not found
- Parse JSON output

**3.4 Add Type Hints & Docs**
- Pydantic models for Memory, Relationship
- Docstrings with examples
- README with quickstart

**3.5 Publish to PyPI (Optional)**
- Or just document pip install from git

## Hard Constraints

- Algorithms FROZEN (classifier.rs, templater.rs, causal.rs)
- Dry-run default (--apply required for writes)
- Python SDK must work with ZERO MCP dependency
- Tests must not require MCP server

## Test Commands

```bash
# Librarian tests
cargo test -p codex-stage0 -- librarian

# Full build
~/code/build-fast.sh

# Manual librarian test (requires TUI)
/stage0.librarian sweep --verbose
/stage0.librarian sweep --apply --limit 5
/stage0.librarian history

# Local-memory health (for SDK testing)
~/.claude/hooks/lm-dashboard.sh --compact
curl http://localhost:4080/api/v1/health
```

## Files to Reference

```
# Librarian implementation
codex-rs/stage0/src/librarian/           # All librarian modules
codex-rs/tui/src/stage0_adapters.rs      # MCP adapters
codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs

# OverlayDb pattern
codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs:467

# Local-memory protocol
~/.local-memory/PROTOCOL.md
~/.claude/hooks/lm-*.sh                  # Existing shell wrappers
```

## Session Lineage

P89 → P90 → P91 → P92 → P93 → P94 → P95 → P96 → P97 → P98 → **P99**

---

**ultrathink**
