# HANDOFF-P100: God-Level Context Generation Pipeline

**Session**: P100 (Next)
**Date**: 2024-12-02
**Predecessor**: P99 (Project Intel Pipeline Implementation)
**Role**: DevOps & Data Engineering Specialist

---

## 1. Executive Summary

Build a local "God-Level" context generation pipeline that produces artifacts for Google NotebookLM upload. This complements the P99 Project Intel pipeline by adding external tool-based analysis (AST packing, complexity metrics, git forensics, call graphs).

---

## 2. Prior Session Context (P99)

### What Was Implemented

A `/stage0.project-intel` command pipeline was added:

```
stage0/src/project_intel/
├── mod.rs           # Module exports
├── types.rs         # Data model (ProjectSnapshot, CodeTopology, etc.)
└── snapshot.rs      # ProjectSnapshotBuilder + load_governance_from_db()

tui/src/chatwidget/spec_kit/commands/intel.rs  # TUI command implementation

config/project_profile.toml   # Project scanning configuration
docs/NL_MANIFEST.toml         # NotebookLM sync manifest
```

### Project Intel Commands (Already Working)

```bash
/stage0.project-intel snapshot     # Gather project details → JSON + markdown
/stage0.project-intel curate-nl    # Generate NL_* docs from feeds
/stage0.project-intel sync-nl      # Push NL_* docs to NotebookLM
/stage0.project-intel overview     # Query global mental model
```

### Key Integration Points

- **MCP Manager**: Available via `widget.mcp_manager` for NotebookLM calls
- **Stage0 Engine**: `codex_stage0::Stage0Engine::new()` for overlay DB access
- **Governance Data**: `codex_stage0::project_intel::load_governance_from_db()`
- **Block-on-Sync Pattern**: `consensus_coordinator::block_on_sync()` for async in TUI

---

## 3. New Task: God-Level Context Pipeline

### Objective

Create `generate_god_context.sh` in repo root that produces NotebookLM-ready artifacts.

### Required Tool Stack

| Tool | Runtime | Purpose | Install Command |
|------|---------|---------|-----------------|
| `repomix` | Node.js | AST-based structural packing | `npm install -g repomix` |
| `rust-code-analysis-cli` | Cargo | Cognitive complexity metrics | `cargo install rust-code-analysis-cli` |
| `hercules` | Go | Git churn + coupling forensics | `go install github.com/src-d/hercules/cmd/hercules@latest` |
| `code2flow` | Python | Call graph generation | `pip install code2flow` |

### Script Requirements

```bash
#!/bin/bash
# generate_god_context.sh

# 1. Setup output directory
mkdir -p notebooklm_context

# 2. Structural Map (AST-based)
repomix --compress --style xml --output notebooklm_context/repo_structure.xml

# 3. Git Forensics (churn + coupling)
hercules --burndown --couples --json > notebooklm_context/git_forensics.json

# 4. Code Metrics (complexity)
rust-code-analysis-cli -p . -o json > notebooklm_context/code_metrics.json

# 5. Call Graph Topology
code2flow . --output notebooklm_context/call_graph.gv

# 6. Summary
echo "Generated artifacts in notebooklm_context/"
ls -la notebooklm_context/
```

### Expected Output Structure

```
notebooklm_context/
├── repo_structure.xml      # AST-compressed repository map
├── git_forensics.json      # Churn analysis + file coupling
├── code_metrics.json       # Cognitive complexity per function
└── call_graph.gv           # Graphviz call graph (or .txt)
```

---

## 4. Implementation Steps

### Step 1: Check Runtime Prerequisites

```bash
# Check Node.js
node --version

# Check Cargo/Rust
cargo --version

# Check Go
go version

# Check Python
python3 --version
pip --version
```

### Step 2: Install Missing Tools

```bash
# Node.js tool
npm install -g repomix

# Rust tool
cargo install rust-code-analysis-cli

# Go tool (ensure $GOPATH/bin in PATH)
go install github.com/src-d/hercules/cmd/hercules@latest

# Python tool
pip install code2flow
```

### Step 3: Create the Script

Write `generate_god_context.sh` with:
- Prerequisite checks (fail fast if tool missing)
- Progress output
- Error handling
- Optional: timing for each step

### Step 4: Integration with Project Intel (Optional)

Consider adding a new subcommand:
```
/stage0.project-intel god-context
```

That calls the shell script and merges output with existing NL_* docs.

---

## 5. Technical Notes

### Hercules Quirks

- Requires Git history access (won't work on shallow clones)
- May need `--pb` flag for progress bar
- JSON output can be large for repos with long history

### Repomix Considerations

- `--compress` uses gzip compression
- `--style xml` produces structured output
- May have file size limits; consider `--ignore` patterns

### rust-code-analysis-cli Notes

- Uses tree-sitter for parsing
- Outputs cognitive complexity, SLOC, cyclomatic complexity
- `-p .` scans current directory recursively

### code2flow Limitations

- Works best with Python/JavaScript
- Rust support may be limited
- Fallback: Use `cargo-call-stack` for Rust-specific call graphs

---

## 6. Codebase Reference

### Key Directories

```
codex-rs/                    # Rust workspace root
├── stage0/src/              # Stage 0 overlay engine
│   ├── project_intel/       # NEW: Project Intel module
│   ├── librarian/           # Memory quality engine
│   └── *.rs                  # DCC, Tier2, scoring, etc.
├── tui/src/                 # Terminal UI
│   └── chatwidget/spec_kit/ # Spec-kit commands
├── core/                    # Core library
└── cli/                     # CLI binary
```

### Relevant Files for Integration

- `tui/src/chatwidget/spec_kit/commands/intel.rs` - Project Intel command
- `stage0/src/project_intel/snapshot.rs` - Snapshot builder
- `config/project_profile.toml` - Project configuration
- `docs/NL_MANIFEST.toml` - NotebookLM manifest

---

## 7. Verification Checklist

- [ ] All four runtimes present (Node, Cargo, Go, Python)
- [ ] All tools installed and in PATH
- [ ] Script created with proper shebang and permissions
- [ ] Script runs without errors
- [ ] All four artifacts generated in `notebooklm_context/`
- [ ] Artifacts are valid (non-empty, parseable)
- [ ] Optional: Integration with `/stage0.project-intel` tested

---

## 8. Success Criteria

1. **Script exists**: `generate_god_context.sh` in repo root
2. **Prerequisites documented**: Clear install instructions
3. **Artifacts generated**: All four files in `notebooklm_context/`
4. **Upload-ready**: Files suitable for NotebookLM ingestion

---

## 9. References

- **repomix**: https://github.com/yamadashy/repomix
- **rust-code-analysis**: https://github.com/mozilla/rust-code-analysis
- **hercules**: https://github.com/src-d/hercules
- **code2flow**: https://github.com/scottrogowski/code2flow

---

## 10. Session Transition Notes

**From P99**: The Project Intel pipeline is complete and compiles. The workspace builds cleanly with `cargo check --workspace`.

**For P100**: Focus on the shell script first. Integration with the Rust codebase is optional/secondary. The immediate goal is working artifacts for NotebookLM.

**Git Status** (at handoff):
- Branch: `main`
- New files: `stage0/src/project_intel/*`, `tui/.../commands/intel.rs`, config files
- Status: Uncommitted changes (Project Intel implementation)

---

## 11. Current Environment Status

### Runtimes (All Present)

| Runtime | Version | Status |
|---------|---------|--------|
| Node.js | v22.18.0 | Ready |
| Cargo | 1.90.0 | Ready |
| Go | 1.25.4 | Ready |
| Python | 3.13.7 | Ready |
| Pip | 25.3 | Ready |

### Tools (Need Installation)

| Tool | Status | Install Command |
|------|--------|-----------------|
| repomix | **NOT INSTALLED** | `npm install -g repomix` |
| rust-code-analysis-cli | **NOT INSTALLED** | `cargo install rust-code-analysis-cli` |
| hercules | **NOT INSTALLED** | `go install github.com/src-d/hercules/cmd/hercules@latest` |
| code2flow | **NOT INSTALLED** | `pip install code2flow` |

### First Action in P100

```bash
# Install all tools
npm install -g repomix
cargo install rust-code-analysis-cli
go install github.com/src-d/hercules/cmd/hercules@latest
pip install code2flow

# Verify $GOPATH/bin is in PATH
export PATH=$PATH:$(go env GOPATH)/bin
```

---

*Generated by Claude Code - P99 Session*
