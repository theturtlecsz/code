# 🔧 ACE Configuration Fix Applied

**Issue**: `/speckit.plan` showed no ACE output
**Root Cause**: "speckit.plan" was missing from `use_for` config list
**Status**: Fixed ✅

---

## What Was Wrong

In `~/.code/config.toml`, the ACE `use_for` setting only included 5 commands:
```toml
# OLD (broken):
use_for = ["speckit.constitution", "speckit.specify", "speckit.tasks", "speckit.implement", "speckit.test"]
```

**Missing commands**:
- ❌ speckit.plan
- ❌ speckit.validate
- ❌ speckit.audit
- ❌ speckit.unlock

This caused `should_use_ace()` to return false, skipping ACE injection entirely.

**Code flow**:
1. `routing.rs:115` → calls `submit_prompt_with_ace()`
2. `mod.rs:13521` → checks `should_use_ace(config, "speckit.plan")`
3. `ace_prompt_injector.rs` → checks if command in `use_for` list
4. Returns `false` → skips ACE, no output shown

---

## What Was Fixed

Updated `~/.code/config.toml` line 12:
```toml
# NEW (fixed):
use_for = [
  "speckit.constitution",
  "speckit.specify",
  "speckit.plan",          # ← ADDED
  "speckit.tasks",
  "speckit.implement",
  "speckit.validate",      # ← ADDED
  "speckit.test",
  "speckit.audit",         # ← ADDED
  "speckit.unlock"         # ← ADDED
]
```

Now **all 9 main spec-kit workflow commands** use ACE.

---

## How to Test

**IMPORTANT**: You need to **restart the TUI** for config changes to take effect.

### 1. Restart TUI
```bash
# Exit current TUI session (Ctrl+C or /quit)
cd /home/thetu/code/codex-rs
code
```

### 2. Test ACE Status
```
/speckit.ace-status
```

**Expected**: Table showing 8 bullets (global:6, tasks:1, test:1)

### 3. Test Plan Command (Previously Broken)
```
/speckit.plan SPEC-KIT-069
```

**Expected output (should now appear)**:
```
⏳ Preparing prompt with ACE context...
⏳ Fetching ACE bullets for scope: plan...
✅ Loaded N bullets from ACE playbook
⏳ Submitting prompt to LLM...
```

### 4. Test Other Commands
```
/speckit.validate SPEC-KIT-069  # Should now show ACE output
/speckit.audit SPEC-KIT-069     # Should now show ACE output
/speckit.unlock SPEC-KIT-069    # Should now show ACE output
```

All should show:
- "⏳ Preparing prompt with ACE context..."
- Bullet loading messages
- No silent failures

---

## Verification

After running any command, check logs:
```bash
# Check that ACE is actually working:
tail -50 ~/.code/logs/codex-tui.log 2>/dev/null | grep -i ace

# If log file doesn't exist yet, ACE might create it on first use
# or check stderr output from TUI
```

**Look for**:
- "ACE MCP client initialized" (at startup)
- "Fetching ACE bullets for scope: [scope]"
- "Injected N ACE bullets for scope: [scope]"

---

## Why This Happened

The config was created with only the **initial test commands** from early development:
- constitution (special command)
- specify (PRD stage)
- tasks (task decomposition)
- implement (code generation)
- test (validation)

But **4 workflow commands were never added**:
- plan (work breakdown) ← This is what you tested
- validate (quality gates)
- audit (compliance)
- unlock (final approval)

The code worked correctly - it just wasn't configured to use ACE for those commands.

---

## Impact

**Before fix**:
- 5/9 commands used ACE (56%)
- No ACE for plan/validate/audit/unlock
- Silent failure (no error, just no ACE)

**After fix**:
- 9/9 commands use ACE (100%)
- All workflow stages enhanced
- Consistent UX across all commands

---

## Next Steps

1. **Restart TUI** (required for config to load)
2. **Test /speckit.plan** (should now work)
3. **Test other commands** (validate, audit, unlock)
4. **Monitor playbook growth** (run 5-10 commands)
5. **Assess value** (end of week decision)

---

## Alternative: Test Without Restarting

If you want to test without restarting, you can use a command that was already working:

```
/speckit.implement SPEC-KIT-069
```

This was already in the `use_for` list, so it should show ACE output even before restart.

But to test `/speckit.plan` specifically, **restart is required**.

---

## Files Modified

- `~/.code/config.toml` - Added 4 commands to `use_for` array

**No code changes needed** - the framework was already correct, just misconfigured.

---

**Ready to test again!** Restart the TUI and try `/speckit.plan` - you should now see ACE output. 🚀
