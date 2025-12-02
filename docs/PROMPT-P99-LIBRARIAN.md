# P99 Continuation Prompt

**Session**: SPEC-KIT-103 Librarian v1 Integration (Continued)

Copy this to start the next session:

---

**Ultrathink** P99 Session: SPEC-KIT-103 Librarian v1 Polish

Read docs/PROMPT-P99-LIBRARIAN.md for context.

## P98 Completed Tasks

| Task | Status | Commit |
|------|--------|--------|
| 1. LocalMemoryClient trait | ✅ | feat(stage0): add LocalMemoryClient trait |
| 2. MCP adapter in TUI | ✅ | feat(tui): implement LibrarianMemoryMcpAdapter |
| 3. Audit trail schema | ✅ | feat(stage0): add Librarian audit trail schema |
| 4. --apply mode | ✅ | feat(tui): implement --apply mode |
| 5. --verbose mode | ✅ | (in same commit as Task 4) |
| 6. RelationshipsClient | ✅ | feat(stage0,tui): add RelationshipsClient |

## What's Working

- `/stage0.librarian sweep` - Dry-run with sample data
- `/stage0.librarian sweep --apply` - Write changes via MCP
- `/stage0.librarian sweep --verbose` - Detailed progress
- `/stage0.librarian sweep --json` - JSON output for CI
- Idempotent: skips memories with correct type tags
- Causal edge detection and inference
- Audit trail recording (if overlay DB available)

## What Needs Polish (Optional P99 Tasks)

1. **Wire audit trail to overlay DB in TUI**
   - Currently disabled (overlay_db not on widget state)
   - Need to connect or create on-demand

2. **Wire RelationshipsClient to sweep**
   - Trait exists, not yet used in sweep execution
   - When --apply, should create edges in local-memory

3. **Add /stage0.librarian history**
   - Command exists but overlay_db not connected
   - Shows sweep history from audit trail

4. **Integration tests**
   - Test full sweep with mock MCP
   - Test idempotency behavior

## Files Modified (P98)

```
codex-rs/stage0/src/librarian/client.rs     # LocalMemoryClient, RelationshipsClient traits
codex-rs/stage0/src/librarian/audit.rs      # LibrarianAudit API
codex-rs/stage0/src/librarian/mod.rs        # Re-exports
codex-rs/stage0/STAGE0_SCHEMA.sql           # Audit trail tables
codex-rs/tui/src/stage0_adapters.rs         # MCP adapters
codex-rs/tui/src/chatwidget/spec_kit/commands/librarian.rs  # Command impl
```

## Test Commands

```bash
cargo test -p codex-stage0 -- librarian        # 70 passing tests
~/code/build-fast.sh                           # Full build
```

Session Lineage: P89 → ... → P97 → P98 → **P99**

**ultrathink**

---
