# ACE Testing - Fresh Start Guide

## Current Status ✅

All fixes applied and verified:
- ✅ ACE config complete (all 9 commands)
- ✅ Gemini wrapper installed and working
- ✅ Settings.json fixed and immutable
- ✅ Database has 8 bullets
- ✅ Binary built with ACE code (Oct 26 20:15)

**The migration errors you see are GOOD** - they mean Gemini can't rewrite settings.json.

---

## Problem

You're likely running an **old TUI instance** that loaded the old config.

---

## Solution: Complete Restart

### Step 1: Kill Any Running TUI

```bash
# Find and kill ALL code/codex-tui processes
pkill -f "codex-tui\|/code exec"
ps aux | grep code | grep -v grep
# Should show nothing
```

### Step 2: Start Fresh TUI

```bash
cd /home/thetu/code/codex-rs
/home/thetu/code/codex-rs/target/dev-fast/code
```

**IMPORTANT**: Use the **full path** to ensure you're running the correct binary.

### Step 3: Test ACE Status

In the TUI:
```
/speckit.ace-status
```

**Expected output**:
```
📊 ACE Playbook Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Scope      Bullets  Pinned  Avg Score
───────────────────────────────────────
global          6       6       0.00
tasks           1       1       0.00
test            1       1       0.00
───────────────────────────────────────
TOTAL           8       8       0.00
```

### Step 4: Test ACE Injection

```
/speckit.plan SPEC-KIT-069
```

**Expected output (BEFORE LLM call)**:
```
⏳ Preparing prompt with ACE context...
⏳ Fetching ACE bullets for scope: plan...
✅ Loaded N bullets from ACE playbook
⏳ Submitting prompt to LLM...
```

---

## If You Still Don't See ACE Output

### Option 1: Check Logs
```bash
# Look for ACE initialization
tail -100 ~/.code/logs/codex-tui.log 2>/dev/null | grep -i ace

# If no log file exists, ACE might not be initializing
```

### Option 2: Rebuild Binary
```bash
cd /home/thetu/code/codex-rs
cargo build --profile dev-fast -p codex-tui
ls -lh target/dev-fast/code
# Should show fresh timestamp
```

### Option 3: Run With Debug Logging
```bash
RUST_LOG=codex_tui=debug /home/thetu/code/codex-rs/target/dev-fast/code
```

Then check for ACE messages in output.

---

## About the Gemini Errors

You'll see these when gemini is called:
```
Error migrating settings file on disk: EPERM: operation not permitted
```

**This is EXPECTED and GOOD!** It means:
1. Gemini tried to rewrite settings.json
2. The immutable flag blocked it
3. Gemini continues anyway with the correct settings

The wrapper ensures `-m gemini-2.5-flash` is always passed, so Gemini works despite these errors.

---

## Verification Checklist

After restart:

- [ ] Killed all old TUI processes
- [ ] Started TUI from full path
- [ ] `/speckit.ace-status` shows 8 bullets
- [ ] `/speckit.plan` shows "Preparing prompt with ACE context..."
- [ ] No more `model.startsWith` errors
- [ ] Gemini orchestrator works (may show migration errors, that's OK)

---

## If Everything Works

**Next steps**:
1. Run 5-10 spec-kit commands this week
2. Monitor playbook growth: `./QUICK_TEST_COMMANDS.sh`
3. Check bullet quality end of week
4. Decide: keep full ACE or simplify to injector

---

## If Gemini Still Fails

The wrapper might not be getting the -m flag through. Try this test:

```bash
# Direct test
/home/thetu/.local/bin/gemini-wrapper -y "test" 2>&1 | grep -v "Error migrating"

# Should output text from Gemini
```

If that works but orchestrator fails, there might be an issue with how the TUI calls the wrapper.

---

## Summary

**All fixes are in place**:
1. ACE: All 9 commands enabled
2. Gemini: Wrapper + immutable settings
3. Binary: Has ACE code
4. Database: Ready with 8 bullets

**You just need to**: Kill old TUI, start fresh, test commands.

**Expected behavior**: ACE messages before every spec-kit command.

---

Ready to test! 🚀
