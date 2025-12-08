# P103 Continuation Prompt - Architect Sidecar Phase 2

## Session Context

**P102 Complete** (commits `e5941ef4f`, `f6b33e071`, `2ce08736d`):
- Architect Sidecar CLI implemented with budget-aware caching
- Librarian audit trail integration added
- Old handoff docs cleaned up (P51-P94 removed)

## What Was Done (P102)

| Component | Status | Commit |
|-----------|--------|--------|
| Vault structure `.codex/architect/` | ✅ | e5941ef4f |
| CLI commands (refresh/ask/audit/status/clear-cache) | ✅ | e5941ef4f |
| Budget protocol (cache-first, Y/n prompt) | ✅ | e5941ef4f |
| Python harvester scripts | ✅ | e5941ef4f |
| Documentation `docs/ARCHITECT-SIDECAR.md` | ✅ | e5941ef4f |
| NotebookLM integration (via CLI wrapper) | ✅ | e5941ef4f |
| Librarian audit trail (overlay_db, history cmd) | ✅ | 2ce08736d |
| Docs cleanup (44 obsolete files removed) | ✅ | 2ce08736d |

## Pre-Session Setup

```bash
# 1. Verify the CLI works
./codex-rs/target/dev-fast/code architect status

# 2. If binary is stale, rebuild
./build-fast.sh

# 3. Verify NotebookLM auth
nlm notebooks  # Should show codex-rs-architect

# 4. Verify CodeGraphContext MCP is running
# (for relationship analysis integration)
```

## Design Decisions (P102)

| Decision | Choice | Rationale |
|----------|--------|-----------|
| AST Parsing | **tree-sitter** | Multi-language support (Rust, TS, Python) |
| Graph Integration | **CodeGraphContext MCP** | Leverage existing graph, avoid duplication |
| Performance | **Single-threaded** | Simplicity over speed for maintainability |

## Session Tasks (Priority Order)

### Phase 1: Refresh NotebookLM Notebook (15 min)

The `codex-rs-architect` notebook has stale forensic data (shows 22 co-changes vs 125 actual).

```bash
# Fresh forensic data is in .codex/architect/ingest/
ls -la .codex/architect/ingest/
```

**Task**: Upload fresh data to NotebookLM:
1. Delete stale sources (churn_hotspots.md, logical_coupling.md, metrics.md)
2. Upload fresh `churn_matrix.md` from `.codex/architect/ingest/`
3. Verify with a test query: "What is the strongest coupling in the codebase?"

### Phase 2: Rust Harvester Implementation (Core Task)

Replace Python scripts with native Rust modules using:
- `tree-sitter` crate for multi-language AST parsing
- `git2` crate for git log analysis
- Integration with `CodeGraphContext` MCP for relationship queries

**Target Files:**
```
codex-rs/core/src/architect/
├── mod.rs           # Module exports
├── harvester.rs     # Main harvester orchestration
├── churn.rs         # Git churn/coupling analysis (git2)
├── complexity.rs    # LOC + indentation metrics
├── skeleton.rs      # Public API extraction (tree-sitter)
└── graph_bridge.rs  # CodeGraphContext MCP integration
```

**Implementation Checklist:**

1. **Add dependencies to `codex-rs/core/Cargo.toml`:**
   ```toml
   git2 = { version = "0.19", default-features = false }
   tree-sitter = "0.22"
   tree-sitter-rust = "0.21"
   tree-sitter-typescript = "0.21"
   tree-sitter-python = "0.21"
   ```

2. **Implement `churn.rs`:**
   - Use `git2::Repository::open()` to access git
   - Walk commits with `repo.revwalk()`
   - Parse diff stats for each commit
   - Build churn map and coupling matrix
   - Output as `ChurnReport` struct

3. **Implement `complexity.rs`:**
   - Walk directory for source files (.rs, .ts, .py)
   - Count lines, filter comments/blanks
   - Calculate max/avg indentation
   - Output as `ComplexityReport` struct

4. **Implement `skeleton.rs` (tree-sitter):**
   - Parse source files with appropriate tree-sitter grammar
   - Extract public declarations using tree-sitter queries
   - Support Rust (`pub fn`, `pub struct`), TypeScript (`export`), Python (`def`)
   - Output as structured `SkeletonReport`

5. **Implement `graph_bridge.rs`:**
   - Query CodeGraphContext MCP for existing relationships
   - `find_callers`, `find_callees`, `module_deps`
   - Merge with local churn data for enriched analysis

6. **Wire into CLI:**
   - Update `architect_cmd.rs` to use Rust modules
   - Remove Python script calls from `run_refresh()`
   - Add `--legacy` flag to fall back to Python during transition

### Phase 3: /risk TUI Integration (Optional)

If time permits, add risk assessment to the TUI:

```rust
// codex-rs/tui/src/slash_command.rs
SlashCommand::Risk { target } => {
    // Query local forensics for file risk
    // Display churn count, coupling partners, complexity score
}
```

### Phase 4: Librarian Integration (Deferred)

Feed churn data into the Librarian's index for queries like:
- "Show me the most unstable parts of the codebase"
- "What files change together with chatwidget.rs?"

## Key Files

| File | Purpose |
|------|---------|
| `codex-rs/cli/src/architect_cmd.rs` | CLI implementation (534 lines) |
| `scripts/architect/generate_*.py` | Python harvesters (reference) |
| `scripts/local-memory/*.py` | Local-memory utilities |
| `docs/ARCHITECT-SIDECAR.md` | Usage documentation |
| `.codex/architect/ingest/` | Fresh forensic data |

## Testing Checklist

```bash
# Test CLI commands
code architect status
code architect refresh
code architect ask "test query" --yes
code architect audit test-crate --yes
code architect clear-cache

# Test Rust harvester (after implementation)
cargo test -p codex-core -- architect

# Test tree-sitter parsing
cargo test -p codex-core -- architect::skeleton

# Test CodeGraphContext integration
cargo test -p codex-core -- architect::graph_bridge
```

## Expected Outcomes

1. **NotebookLM Updated**: Fresh forensic data with 125 co-changes (not 22)
2. **Native Harvester**: `code architect refresh` runs pure Rust (no Python)
3. **Multi-Language Support**: tree-sitter parses .rs, .ts, .py files
4. **Graph Integration**: Relationships from CodeGraphContext enriched with churn
5. **Tests Added**: Unit tests for all harvester modules

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────────┐
│                     Architect Sidecar                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │  Harvester  │───►│    Vault    │◄───│       Broker        │ │
│  │  (Rust)     │    │  (Storage)  │    │  (Query Gatekeeper) │ │
│  │  tree-sitter│    │  .codex/    │    │  Y/n confirmation   │ │
│  │  git2       │    │             │    │                     │ │
│  └──────┬──────┘    └─────────────┘    └─────────────────────┘ │
│         │                                         │             │
│         ▼                                         ▼             │
│  ┌─────────────┐                        ┌─────────────────────┐ │
│  │ CodeGraph   │                        │   NotebookLM CLI    │ │
│  │ Context MCP │                        │  codex-rs-architect │ │
│  │ (relations) │                        │      notebook       │ │
│  └─────────────┘                        └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Commits This Session (P102)

- `2ce08736d` chore: cleanup old docs and add librarian audit integration
- `f6b33e071` docs: add P103 continuation prompt for Architect Sidecar Phase 2
- `e5941ef4f` feat(cli): add Architect Sidecar with budget-aware caching

## Reference: Python Harvester Locations

The existing Python scripts serve as reference implementations:
- `scripts/architect/generate_churn.py` - Git forensics
- `scripts/architect/generate_complexity.py` - Complexity metrics
- `scripts/architect/generate_skeleton.py` - API extraction

## Notes

- tree-sitter provides fast, incremental parsing across languages
- CodeGraphContext MCP already indexes the codebase - leverage, don't duplicate
- Single-threaded implementation keeps debugging simple
- The `--legacy` flag allows gradual migration from Python to Rust
