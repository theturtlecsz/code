# NotebookLM CLI Integration

This directory contains project-specific configuration for the NotebookLM CLI integration.

## Overview

The Codex-RS project uses NotebookLM as an "Architect Sidecar" - providing god-level forensic context for development decisions. Instead of the deprecated MCP server, we use the CLI directly.

## Architecture

```
Developer → scripts/architect_cli.py → NotebookLM CLI → Codex-RS Architect Notebook
                                              ↓
                               Custom Templates (crate-audit, arch-review, etc.)
```

## Notebooks

| ID | Name | Purpose |
|----|------|---------|
| `codex-rs-architect` | Codex-RS Architect | Forensic context (AST, churn, coupling, metrics) |

Notebook URL: https://notebooklm.google.com/notebook/3f5f0ff0-896e-434e-872e-a130757ea41e

## Custom Templates

Located at `~/.config/notebooklm-mcp/templates/codex-rs.yaml` (also copied here for reference):

| Template | Mode | Use Case |
|----------|------|----------|
| `crate-audit` | deep | Audit Rust crates before adding dependencies |
| `arch-review` | deep | Research architectural patterns for implementations |
| `tui-patterns` | fast | ratatui/TUI best practices |
| `upstream-sync` | deep | Migration and upgrade strategies |
| `risk-assessment` | fast | Cross-reference changes with forensic data |

## Usage

### Quick Commands (with aliases)

```bash
# Ask the Architect
arch ask "Where should I add a new slash command?"

# Audit a crate before adding
arch audit sqlx

# Risk assessment before modifying a file
arch risk chatwidget.rs

# Research architectural patterns
arch arch "event handling patterns"

# TUI-specific research
arch tui "split pane layouts"
```

### Direct CLI

```bash
# List notebooks
nlm notebooks

# Ask a question
nlm ask -n codex-rs-architect "What are the churn hotspots?"

# Deep research with template
nlm deep-research -n codex-rs-architect "tokio vs async-std" -t crate-audit

# Fast research
nlm fast-research -n codex-rs-architect "ratatui event loop patterns" -t tui-patterns
```

## Pre-Flight Workflow

Before making significant changes:

1. **Risk Check**: `arch risk <file>` - Cross-reference with churn/coupling data
2. **Dependency Audit**: `arch audit <crate>` - Before adding new dependencies
3. **Pattern Research**: `arch arch "<topic>"` - Find idiomatic approaches

## Refreshing Forensic Data

When the codebase changes significantly:

```bash
# 1. Regenerate artifacts
./generate_god_context.sh --diet

# 2. Convert to markdown
python3 convert_artifacts.py

# 3. Split large files
python3 split_structure.py

# 4. Upload new files to NotebookLM (manual drag-drop)
# 5. Paste bootloader prompt from docs/notebooklm_bootloader.md
```

## Authentication

Chrome profile authentication is handled globally. If auth expires:

```bash
# SSH with X11 forwarding
ssh -Y user@server

# Open Chrome to NotebookLM
chromium --user-data-dir=~/.local/share/notebooklm-mcp/chrome_profile \
         --password-store=basic --no-first-run https://notebooklm.google.com

# Login, close browser
```

## Files

| Path | Purpose |
|------|---------|
| `templates/codex-rs.yaml` | Reference copy of custom templates |
| `~/.config/notebooklm-mcp/templates/` | Active template directory |
| `~/.local/share/notebooklm-mcp/library.json` | Notebook registry |
| `scripts/architect_cli.py` | Project wrapper script |
