# Dogfooding Readiness Checklist

> **Goal**: Use `code` TUI to develop `~/code` ("tui writes tui")

## Quick Health Check (Run Daily)

```bash
# One-liner health check
./build-fast.sh && ./codex-rs/target/dev-fast/code doctor && lm health && curl -s http://127.0.0.1:3456/health/ready | jq -r '.ready'
```

## Detailed Checklist

| Check | Command | Expected | Blocker? |
|-------|---------|----------|----------|
| Build | `./build-fast.sh` | "Build successful!" | P0 |
| Binary exists | `ls ./codex-rs/target/dev-fast/code` | File exists | P0 |
| Doctor passes | `./codex-rs/target/dev-fast/code doctor` | All [OK] | P0 |
| Local-memory | `lm health` | "ok" | P0 |
| NotebookLM auth | `curl -s http://127.0.0.1:3456/health/ready \| jq '.checks.authenticated'` | `true` | P1 |
| NotebookLM notebooks | `notebooklm library` | At least 1 notebook | P1 |
| Tests compile | `cd codex-rs && cargo test -p codex-core --no-run` | Compiles | P0 |
| TUI launches | `./build-fast.sh run` (in real terminal) | TUI appears | P0 |

## Current Blockers

### P0: Must Fix Before Dogfooding

| Blocker | Status | Fix Command |
|---------|--------|-------------|
| NotebookLM not authenticated | BLOCKED | `notebooklm setup-auth` |
| No notebooks in library | BLOCKED | `notebooklm add-notebook --name 'code-docs' --url '...'` |
| CLAUDE.md confusing header | Minor | Edit file (see below) |

### P1: Should Fix Soon

| Issue | Impact | Fix |
|-------|--------|-----|
| Missing stage0.toml | Tier2 uses defaults | Create `~/.config/codex/stage0.toml` |
| Dual config files | Model confusion | Consolidate `.codex/` and `.code/` |

## Acceptance Test

**Scenario**: Implement a small change using `code` TUI

```bash
# 1. Start TUI
./build-fast.sh run

# 2. In TUI, make a small change
> Add a comment to build-fast.sh explaining what dev-fast profile does

# 3. Verify the change was made
git diff build-fast.sh

# 4. Run tests
cd codex-rs && cargo test -p codex-core

# 5. Commit (in TUI)
> Commit the change with message "docs: explain dev-fast profile"

# 6. Verify commit
git log -1 --oneline
```

**Pass criteria**: All steps complete without needing Claude Code.

## Service Health Commands

```bash
# Local-memory
lm health
lm domain

# NotebookLM
notebooklm doctor
curl -s http://127.0.0.1:3456/health/ready | jq .

# Stage0
./codex-rs/target/dev-fast/code stage0 doctor
```

## Recovery Commands

```bash
# If NotebookLM fails
notebooklm restart
notebooklm setup-auth  # If auth expired

# If local-memory fails
systemctl --user restart local-memory  # Or: lm restart

# If build fails
cargo clean -p codex-tui && ./build-fast.sh
```

---

## Setup Instructions (One-Time)

### 1. NotebookLM Authentication

```bash
# On headless server, need X11 forwarding for initial auth
ssh -X user@server
chromium --user-data-dir=~/.config/chromium \
         --password-store=basic --no-first-run https://notebooklm.google.com
# Login, close browser, then:
notebooklm setup-auth
```

### 2. Add Notebooks to Library

```bash
# Add project documentation notebook
notebooklm add-notebook \
  --name 'code-docs' \
  --url 'https://notebooklm.google.com/notebook/YOUR_NOTEBOOK_ID' \
  --domain 'spec-kit'
```

### 3. Create stage0.toml

```bash
cat > ~/.config/codex/stage0.toml << 'EOF'
# Stage0 configuration for Tier2 (NotebookLM) integration
[tier2]
default_notebook = "code-docs"
fallback_enabled = true

[tier2.domain_mapping]
"spec-kit" = "code-docs"
"code" = "code-docs"
EOF
```

### 4. Fix Instruction Files

See doc edits section in backlog.
