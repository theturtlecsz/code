# Dogfooding Backlog: "tui writes tui"

> **Objective**: Use `code` TUI to develop `~/code` as the default workflow

## P0: Blockers (Must Fix Before Dogfooding)

### 1. NotebookLM Authentication [notebooklm-mcp]

**Status**: BLOCKED - Auth state file not found
**Impact**: Stage0 Tier2 cannot query NotebookLM
**Owner**: `~/notebooklm-mcp`

```bash
# Fix
notebooklm setup-auth
```

**Verification**:
```bash
curl -s http://127.0.0.1:3456/health/ready | jq '.checks.authenticated'
# Expected: true
```

---

### 2. Add Notebooks to Library [notebooklm-mcp]

**Status**: BLOCKED - Library empty (0 notebooks)
**Impact**: Tier2 has nothing to query
**Owner**: `~/notebooklm-mcp`

```bash
# Fix: Add at least one notebook for code docs
notebooklm add-notebook \
  --name 'code-project-docs' \
  --url 'https://notebooklm.google.com/notebook/YOUR_NOTEBOOK_ID' \
  --domain 'spec-kit'
```

**Verification**:
```bash
notebooklm library
# Expected: At least 1 notebook listed
```

---

### 3. Fix CLAUDE.md Header [~/code]

**Status**: Confusing
**Impact**: "Not related to Anthropic's Claude Code" misleads Claude Code users
**Owner**: `~/code`

**File**: `/home/thetu/code/CLAUDE.md`

**Current** (line 1-3):
```markdown
# CLAUDE.md

Not related to Anthropic's Claude Code.
```

**Change to**:
```markdown
# CLAUDE.md

Project instructions for AI agents (Claude Code, code TUI, etc.)
```

**Also update AGENTS.md header** to match (line 1):
```markdown
# AGENTS.md
```
→ keep as-is (already correct)

---

## P1: Should Fix Soon

### 4. Create stage0.toml [~/code or localmemory-policy]

**Status**: Missing
**Impact**: Tier2 uses defaults, no domain→notebook mapping
**Owner**: Could be `~/code` (repo-specific) or user config

**File**: `~/.config/codex/stage0.toml`

```toml
# Stage0 configuration
[tier2]
enabled = true
default_notebook = "code-project-docs"

[tier2.domain_mapping]
"spec-kit" = "code-project-docs"
"code" = "code-project-docs"
```

---

### 5. Consolidate Config Files [user config]

**Status**: Two configs with different models
**Impact**: Confusing which model is used

| File | Model |
|------|-------|
| `~/.codex/config.toml` | gpt-5.2 |
| `~/.code/config.toml` | gpt-5-codex |

**Recommendation**:
- Keep `~/.code/config.toml` as primary (newer binary)
- Remove or archive `~/.codex/config.toml`
- Or symlink: `ln -sf ~/.code/config.toml ~/.codex/config.toml`

---

### 6. Doctor NotebookLM Check Accuracy [~/code]

**Status**: Minor discrepancy
**Impact**: Doctor says "[OK] notebooklm: Service healthy" but auth is missing

**File**: `codex-rs/cli/src/doctor.rs` (or wherever stage0 doctor lives)

**Fix**: Make doctor check `authenticated` field from health endpoint, not just HTTP reachability.

---

## P2: Nice to Have

### 7. Add `/code` Doctor Command to TUI [~/code]

**Status**: Missing
**Impact**: No quick health check from within TUI

**Proposal**: Add `/doctor` or `/health` TUI command that runs `code doctor` inline.

---

### 8. Session Hooks for `code` Binary [localmemory-policy]

**Status**: Hooks exist for Claude Code but may not fire for `code` TUI
**Impact**: Session milestones not captured when using `code`

**Check**: Does `code` TUI invoke session_start.sh and session_end.sh?

**File**: `~/.code/config.toml` already has hook config, verify it works.

---

## Cross-Repo Coordination

| Change | Belongs To | Why |
|--------|------------|-----|
| NotebookLM auth | `~/notebooklm-mcp` | Service owns auth state |
| Notebook library | `~/notebooklm-mcp` | Service owns library.json |
| stage0.toml | User config or `~/code` | Stage0 lives in code |
| CLAUDE.md fix | `~/code` | Repo instruction file |
| Doctor accuracy | `~/code` | Doctor command in code binary |
| Session hooks | `~/infra/localmemory-policy` | Hook scripts live there |

---

## Config Assumptions (Cross-Repo Contract)

| Assumption | Defined In | Consumed By |
|------------|-----------|-------------|
| NotebookLM at `127.0.0.1:3456` | notebooklm-mcp | code Stage0 |
| Local-memory REST at default port | localmemory-policy | code Stage0, hooks |
| Domain resolution via `resolve-domain.sh` | localmemory-policy | code, hooks |
| `lm` CLI wrapper at `~/.local/bin/lm` | localmemory-policy | code, hooks |

---

## Verification Matrix

After fixing P0 items, run this matrix:

| Test | Command | Pass Criteria |
|------|---------|---------------|
| Build | `./build-fast.sh` | "Build successful!" |
| Doctor | `code doctor` | All [OK], no [WARN] |
| Local-memory | `lm health` | "ok" |
| NotebookLM auth | `curl ... \| jq '.checks.authenticated'` | `true` |
| NotebookLM library | `notebooklm library` | ≥1 notebook |
| Stage0 doctor | `code stage0 doctor` | All [OK] |
| TUI launch | `./build-fast.sh run` | TUI renders |
| /speckit.auto | In TUI: `/speckit.auto SPEC-TEST-001` | Pipeline starts |
