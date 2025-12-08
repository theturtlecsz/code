# Architect Sidecar

Budget-aware forensic intelligence module for codex-rs development.

## Overview

The Architect Sidecar provides deep codebase analysis through a **Cache-First, Ask-Later** protocol. It mines git history for behavioral patterns (churn, coupling) and queries NotebookLM for architectural insights—while ensuring you never pay for the same insight twice.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Architect Sidecar                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │  Harvester  │───►│    Vault    │◄───│       Broker        │ │
│  │  (Local)    │    │  (Storage)  │    │  (Query Gatekeeper) │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│        │                   │                      │             │
│        ▼                   ▼                      ▼             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────┐ │
│  │ Git History │    │   Ingest/   │    │   NotebookLM CLI    │ │
│  │ AST Walker  │    │  Answers/   │    │  (codex-rs-architect│ │
│  │ (git2, syn) │    │  Audits/    │    │      notebook)      │ │
│  └─────────────┘    └─────────────┘    └─────────────────────┘ │
│                                                                 │
│  Cost: $0              Cost: $0         Cost: 1 query/miss     │
└─────────────────────────────────────────────────────────────────┘
```

## Installation

The Architect CLI is built into codex-rs. No additional setup required.

```bash
# Verify installation
code architect --help
```

## Vault Structure

```
.codex/architect/
├── ingest/              # Raw forensic data (machine-generated)
│   ├── churn_matrix.md      # Git churn analysis
│   ├── complexity_map.json  # LOC + indentation complexity
│   ├── repo_skeleton.xml    # Public API declarations
│   └── .repo_hash           # Freshness checksum (git HEAD)
├── answers/             # Cached Q&A responses (slugified filenames)
│   └── what-are-the-churn-hotspots.md
└── audits/              # Cached crate security audits
    └── tokio.md
```

## Commands

### `code architect status`

Show vault location, data freshness, and cache statistics.

```bash
$ code architect status
Architect Vault Status
======================

Location: /home/user/codex-rs/.codex/architect
Ingest data: FRESH
Cached answers: 3
Cached audits: 1

Recent answers:
  - what-are-the-churn-hotspots.md
  - how-to-refactor-chatwidget.md
  - where-should-i-add-slash-commands.md
```

### `code architect refresh`

Regenerate forensic maps from git history. **Cost: $0** (local computation).

```bash
$ code architect refresh
Refreshing forensic data in .codex/architect/ingest
  [1/3] Generating churn matrix...
  [2/3] Generating complexity map...
  [3/3] Generating repo skeleton...
Refresh complete.
```

Options:
- `--skip-git` - Skip churn/coupling analysis
- `--skip-complexity` - Skip LOC/indent analysis
- `--skip-skeleton` - Skip API extraction

### `code architect ask <query>`

Query the Architect notebook. Uses cache first; prompts before API call.

```bash
$ code architect ask "What are the top churn hotspots?"
Answer not cached. This will use 1 NotebookLM query. Proceed? [Y/n] y
Querying Architect notebook...

Based on the analysis of volatility data, the top three churn hotspots...

(answer cached to: .codex/architect/answers/what-are-the-top-churn-hotspots.md)
```

Options:
- `-f, --force` - Bypass cache, force fresh query
- `-y, --yes` - Skip confirmation prompt

### `code architect audit <crate>`

Investigate a Rust crate for security, maintenance, and alternatives.

```bash
$ code architect audit tokio
Audit for 'tokio' not cached. This will use 1 NotebookLM query. Proceed? [Y/n] y
Auditing crate: tokio...

# Crate Audit: tokio
...

(audit cached to: .codex/architect/audits/tokio.md)
```

### `code architect clear-cache`

Remove all cached answers and audits. Keeps ingest data.

```bash
$ code architect clear-cache
Clear 3 answers and 1 audits? [Y/n] y
Cache cleared.
```

## Budget Protocol

The Architect follows a strict cost-control protocol:

1. **Cache Check** - Slugify query → check `answers/{slug}.md`
2. **User Confirmation** - Prompt `[Y/n]` before any API call
3. **Stream + Cache** - Display result AND write to cache simultaneously
4. **Freshness Tracking** - Compare `.repo_hash` to detect stale data

| Operation | Cost |
|-----------|------|
| `status` | $0 |
| `refresh` | $0 |
| `ask` (cached) | $0 |
| `ask` (miss) | 1 NotebookLM query |
| `audit` (cached) | $0 |
| `audit` (miss) | 1 NotebookLM query |
| `clear-cache` | $0 |

## Forensic Data Sources

### Churn Matrix (`churn_matrix.md`)

Analyzes git history (last 12 months) to identify:
- **Churn Hotspots** - Files with highest commit counts
- **Logical Coupling** - Files that change together (>= 5 co-changes)
- **Coupling Clusters** - Groups of related files

### Complexity Map (`complexity_map.json`)

Static analysis of all `.rs`, `.ts`, `.py` files:
- Lines of Code (LOC)
- Source Lines (SLOC, excluding comments/blanks)
- Maximum indentation depth
- Average indentation level
- Function count
- Composite complexity score

### Repo Skeleton (`repo_skeleton.xml`)

Public API surface extraction from `core/` and `tui/`:
- `pub fn`, `pub struct`, `pub enum`, `pub trait`
- `impl` blocks (inherent and trait implementations)
- Module declarations

## Integration with NotebookLM

The Architect uses the `codex-rs-architect` NotebookLM notebook, which contains:
- Architectural risk assessments
- Code structure documentation
- Churn and coupling analysis
- Call graphs and metrics

Notebook URL: `https://notebooklm.google.com/notebook/3f5f0ff0-896e-434e-872e-a130757ea41e`

## Extending the Harvester

The current harvester uses Python scripts in `scripts/architect/`:
- `generate_churn.py` - Git log analysis
- `generate_complexity.py` - Static complexity metrics
- `generate_skeleton.py` - AST-based API extraction

Future versions will rewrite these in Rust using:
- `git2` - Native git operations
- `syn` - Rust AST parsing
- `tree-sitter` - Multi-language parsing

## Example Workflows

### Pre-Flight Check Before Editing

```bash
# Check if your target file is high-risk
code architect ask "Risk assessment for modifying chatwidget/mod.rs"

# Review cached answer instantly on subsequent calls
code architect ask "Risk assessment for modifying chatwidget/mod.rs"
```

### Dependency Audit Before Adding Crate

```bash
# Investigate before adding to Cargo.toml
code architect audit serde_json
code architect audit reqwest
```

### Understanding Codebase Architecture

```bash
# Get high-level orientation
code architect ask "What are the main architectural patterns in this codebase?"
code architect ask "How does event flow from TUI to core?"
```

## Troubleshooting

### "No .codex/architect/ vault found"

Create the vault structure:
```bash
mkdir -p .codex/architect/{ingest,answers,audits}
```

### "NotebookLM CLI not found"

Install the CLI:
```bash
cd ~
git clone https://github.com/anthropics/notebooklm-mcp
cd notebooklm-mcp
npm install && npm run build
```

### "Ingest data: STALE"

Refresh the forensic maps:
```bash
code architect refresh
```

## Related Documentation

- [NotebookLM CLI Integration](/.notebooklm/README.md)
- [Operational Playbook](/docs/OPERATIONAL-PLAYBOOK.md)
- [Model Guidance](/docs/MODEL-GUIDANCE.md)
