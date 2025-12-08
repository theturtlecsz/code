# P103 Continuation Prompt - Architect Sidecar Phase 2

## Session Context

**P102 Complete** (commit `e5941ef4f`): Architect Sidecar CLI implemented with budget-aware caching.

## What Was Done (P102)

| Component | Status | Commit |
|-----------|--------|--------|
| Vault structure `.codex/architect/` | ✅ | e5941ef4f |
| CLI commands (refresh/ask/audit/status/clear-cache) | ✅ | e5941ef4f |
| Budget protocol (cache-first, Y/n prompt) | ✅ | e5941ef4f |
| Python harvester scripts | ✅ | e5941ef4f |
| Documentation `docs/ARCHITECT-SIDECAR.md` | ✅ | e5941ef4f |
| NotebookLM integration (via CLI wrapper) | ✅ | e5941ef4f |

## Pre-Session Setup

```bash
# 1. Verify the CLI works
./codex-rs/target/dev-fast/code architect status

# 2. If binary is stale, rebuild
./build-fast.sh

# 3. Verify NotebookLM auth
nlm notebooks  # Should show codex-rs-architect
```

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
- `git2` crate for git log analysis
- `syn` crate for Rust AST parsing

**Target Files:**
```
codex-rs/core/src/architect/
├── mod.rs           # Module exports
├── harvester.rs     # Main harvester logic
├── churn.rs         # Git churn/coupling analysis
├── complexity.rs    # LOC + indentation metrics
└── skeleton.rs      # Public API extraction
```

**Implementation Checklist:**

1. **Add dependencies to `codex-rs/core/Cargo.toml`:**
   ```toml
   git2 = { version = "0.19", default-features = false }
   syn = { version = "2", features = ["full", "parsing"] }
   ```

2. **Implement `churn.rs`:**
   - Use `git2::Repository::open()` to access git
   - Walk commits with `repo.revwalk()`
   - Parse diff stats for each commit
   - Build churn map and coupling matrix
   - Output as `ChurnReport` struct

3. **Implement `complexity.rs`:**
   - Walk directory for `.rs` files
   - Count lines, filter comments/blanks
   - Calculate max/avg indentation
   - Output as `ComplexityReport` struct

4. **Implement `skeleton.rs`:**
   - Parse `.rs` files with `syn::parse_file()`
   - Extract `pub fn`, `pub struct`, `pub enum`, `pub trait`, `impl`
   - Output as XML or JSON

5. **Wire into CLI:**
   - Update `architect_cmd.rs` to use Rust modules
   - Remove Python script calls
   - Add `--native` flag for testing during transition

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
| `scripts/architect/generate_*.py` | Python harvesters (to be replaced) |
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
```

## Expected Outcomes

1. **NotebookLM Updated**: Fresh forensic data with 125 co-changes (not 22)
2. **Native Harvester**: `code architect refresh` runs pure Rust (no Python)
3. **Faster Refresh**: Native git2 should be 5-10x faster than shelling out
4. **Tests Added**: Unit tests for churn/complexity/skeleton parsing

## Architecture Reference

```
┌─────────────────────────────────────────────────────────────────┐
│                     Architect Sidecar                           │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │  Harvester  │───►│    Vault    │◄───│       Broker        │ │
│  │  (Rust)     │    │  (Storage)  │    │  (Query Gatekeeper) │ │
│  │  git2 + syn │    │  .codex/    │    │  Y/n confirmation   │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│        │                   │                      │             │
│        ▼                   ▼                      ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │ Git Repo    │    │   Ingest/   │    │   NotebookLM CLI    │ │
│  │ .rs Files   │    │  Answers/   │    │  codex-rs-architect │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Commits This Session

- `e5941ef4f` feat(cli): add Architect Sidecar with budget-aware caching

## Notes

- The existing Python scripts in `scripts/architect/` serve as reference implementations
- The `git2` crate handles all git operations without shelling out
- The `syn` crate provides full Rust AST parsing with item visitor patterns
- Consider using `rayon` for parallel file processing in complexity analysis
