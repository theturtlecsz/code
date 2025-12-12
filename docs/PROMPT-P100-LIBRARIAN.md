# P100 Continuation Prompt

**Session**: SPEC-KIT-103 Librarian v1 - Phase 2 Tests & Phase 3 Python SDK

Copy this to start the next session:

---

**Ultrathink** P100 Session: SPEC-KIT-103 Phase 2 & 3

Read docs/PROMPT-P100-LIBRARIAN.md for full context.

## Session Goals (Priority Order)

1. **Phase 2: Integration Tests** - Mock tests for CI + real local-memory tests
2. **Phase 3: Python SDK** - Standalone local-memory client (zero MCP)

## P99 Completed Summary

| Task | Status | Files |
|------|--------|-------|
| Wire audit trail to TUI | ✓ | `librarian.rs:118-189, 300-324, 388-402, 453-471` |
| Wire RelationshipsClient to sweep | ✓ | `librarian.rs:195-198, 347-404` |
| Add history subcommand | ✓ | `librarian.rs:509-628` |
| OverlayDb.conn() method | ✓ | `overlay_db.rs:224-229` |

## P100 Task Breakdown

### Phase 2: Integration Tests (Est. 2-3 tasks)

**2.1 Mock MCP Tests (CI-safe)**
- Create test module in `stage0/src/librarian/tests/`
- Use `MockLocalMemoryClient` from `client.rs`
- Test cases:
  - sweep with retype (verify tags updated)
  - sweep with template (verify content restructured)
  - idempotency (re-sweep doesn't duplicate changes)
  - causal edge detection
- File: `stage0/src/librarian/tests/sweep_test.rs`

**2.2 Integration Tests (Manual)**
- Create test script or integration test
- Requires running local-memory daemon
- Test: full sweep against real corpus
- Verify audit trail entries created
- File: `scripts/test-librarian-integration.sh` or `tui/tests/librarian_integration.rs`

### Phase 3: Python SDK (Est. 4-5 tasks)

**3.1 Create Package Structure**
```
local-memory-sdk/
├── pyproject.toml
├── README.md
├── src/local_memory/
│   ├── __init__.py
│   ├── client.py      # REST API wrapper
│   ├── types.py       # Memory, Relationship Pydantic models
│   └── exceptions.py
└── tests/
    ├── test_client.py
    └── conftest.py
```

**3.2 Implement REST Client**
- Wrap http://localhost:4080/api/v1/* endpoints
- Methods: search, store, get, update, delete
- Relationships: create, discover, related

**3.3 Add Type Hints & Docs**
- Pydantic models for Memory, Relationship
- Docstrings with examples
- README with quickstart

**3.4 Package for pip install**
- `pip install -e .` from local
- Optional: publish to PyPI

## Hard Constraints

- Algorithms FROZEN (classifier.rs, templater.rs, causal.rs)
- Dry-run default (--apply required for writes)
- Python SDK must work with ZERO MCP dependency
- Mock tests must not require MCP server or local-memory daemon

## Test Commands

```bash
# Librarian unit tests
cargo test -p codex-stage0 -- librarian

# Full build
~/code/build-fast.sh

# Manual librarian test (requires TUI + local-memory)
/stage0.librarian sweep --verbose
/stage0.librarian sweep --apply --limit 5
/stage0.librarian history --detail

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

# Mock client for tests
codex-rs/stage0/src/librarian/client.rs:213  # MockLocalMemoryClient

# Local-memory protocol
~/.local-memory/PROTOCOL.md
~/.claude/hooks/lm-*.sh                  # Existing shell wrappers
```

## SDK Location Decision

**Recommended**: `codex-rs/python/local-memory-sdk/` (monorepo)
- Keeps tooling together
- Easy to reference from Rust tests
- Single CI pipeline

## Session Lineage

P89 → P90 → P91 → P92 → P93 → P94 → P95 → P96 → P97 → P98 → P99 → **P100**

---

**ultrathink**
