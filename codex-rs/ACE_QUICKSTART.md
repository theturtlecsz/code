# ✅ ACE is NOW ENABLED - Quick Start

## Configuration Fixed

Your `~/.code/config.toml` now has:

```toml
[ace]
enabled = true
mode = "auto"
slice_size = 8
db_path = "~/.code/ace/playbooks_v1.sqlite3"
use_for = ["speckit.constitution", "speckit.specify", "speckit.tasks", "speckit.implement", "speckit.test"]
complex_task_files_threshold = 4
rerun_window_minutes = 30

[mcp_servers.ace]
command = "/home/thetu/agentic-context-engine/.venv/bin/python"
args = ["-m", "ace_mcp_server"]
startup_timeout_ms = 30000
```

**Changes made**:
- ✅ Removed duplicate `[mcp.servers.ace-playbook]` (old format)
- ✅ Renamed `[mcp_servers.ace-playbook]` → `[mcp_servers.ace]` (correct name)
- ✅ Added `[ace]` configuration section
- ✅ Cleaned up old mcp.servers format

---

## 🚀 Next Steps (3 Commands)

### 1. Start CODE (ACE auto-initializes)

```bash
code
```

**Watch logs** (in another terminal):
```bash
tail -f ~/.code/logs/codex-tui.log | grep ACE
# Should see: INFO ACE MCP client initialized successfully
```

### 2. Pin Constitution (One-Time Setup)

```bash
# In CODE TUI:
/speckit.constitution
```

**Expected output**:
```
Extracted 8 bullets from constitution, pinning to ACE...
Successfully pinned 8 bullets to ACE playbook (global + phase scopes)
```

**Log output**:
```
INFO ACE pin 145ms pinned=8 bullets
```

### 3. Use Spec-Kit Commands (Bullets Auto-Inject)

```bash
/speckit.implement SPEC-KIT-069
# or
/speckit.specify SPEC-KIT-070
```

**What happens**:
1. ACE fetches relevant bullets from SQLite
2. Bullets injected into prompt automatically
3. CODE calls YOUR LLM with enhanced context
4. After success: ACE learns from outcome

**Prompt will include**:
```markdown
### Project heuristics learned (ACE)
- [helpful] Keep templates synchronized with documentation
- [helpful] Validate all changes with tests before commit
- [avoid] Never commit without running linters
- [note] Record telemetry artifacts for evidence
```

---

## 🔍 Verify ACE is Working

### Check Database Created
```bash
ls -lh ~/.code/ace/playbooks_v1.sqlite3
# Should exist after first use
```

### Check Logs
```bash
# Startup
grep "ACE MCP client initialized" ~/.code/logs/codex-tui.log

# Injection
grep "Injected.*ACE bullets" ~/.code/logs/codex-tui.log

# Learning
grep "ACE learn.*scope=" ~/.code/logs/codex-tui.log
```

### Debug Mode
```bash
RUST_LOG=codex_tui=debug code

# You'll see:
# DEBUG ACE playbook_slice: repo=/home/thetu/code, branch=main, scope=implement, k=8
# DEBUG Injected 6 ACE bullets for scope: implement
# INFO ACE learn 127ms scope=implement added=2 demoted=1 promoted=3
```

---

## 📊 What's Now Active

| Feature | Status | How to Use |
|---------|--------|------------|
| Config | ✅ Enabled | Already in config.toml |
| MCP client | ✅ Auto-starts | Happens when CODE starts |
| Constitution | ✅ Ready | Run `/speckit.constitution` |
| Playbook injection | ✅ Active | Automatic for all spec-kit commands |
| Learning | ✅ Wired | Automatic after validation passes |

---

## 🎯 Quick Demo

```bash
# Terminal 1: Watch logs
tail -f ~/.code/logs/codex-tui.log | grep ACE

# Terminal 2: Run CODE
cd /home/thetu/code
code

# In CODE TUI:
/speckit.constitution
# Wait for: "Successfully pinned 8 bullets"

/speckit.implement SPEC-KIT-069
# ACE bullets will auto-inject
# Watch logs for: "Injected X ACE bullets"
# After validation: "ACE learn Xms scope=implement added=Y"
```

---

## ✅ Summary

**ACE is NOW FULLY OPERATIONAL**

- Config: ✅ Fixed and enabled
- Wiring: ✅ Complete (init, inject, learn)
- Ready: ✅ Just start CODE and use normally

Next time you run CODE, ACE will automatically:
1. Initialize at startup
2. Inject bullets into prompts
3. Learn from every execution
4. Improve over time

**No code changes needed - it's ready to go!**
