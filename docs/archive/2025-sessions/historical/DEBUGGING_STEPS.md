# Debugging /speckit.constitution

## If Nothing Happens AT ALL

This suggests the command isn't executing. Check:

### Step 1: Verify Binary
```bash
which code
# Should show: /home/thetu/code/codex-rs/target/dev-fast/code

ls -lh $(which code)
# Check timestamp: Should be Oct 26 19:24 or later

# If older, rebuild:
/home/thetu/code/build-fast.sh
```

### Step 2: Enable Debug Logging
```bash
# Start with full logging
RUST_LOG=codex_tui=debug code
```

### Step 3: Test Command
```
/speckit.constitution
```

### Step 4: Check What You See

**If you see NOTHING**:
- Command isn't being dispatched
- Check if it's in SlashCommand enum (we added it)
- Check logs for "SpecKitConstitution: execute() called"

**If you see "Constitution not found"**:
- Check: ls memory/constitution.md
- We updated this file, should have 7 short bullets

**If you see "No valid bullets extracted"**:
- Constitution exists but bullets don't meet criteria
- Check bullet lengths

**If you see "Extracted N bullets, pinning to ACE..."**:
- Command is working!
- Check async task completed in logs

### Step 5: Check Logs
```bash
tail -f ~/.code/logs/codex-tui.log | grep -i "constitution\|ace"
```

Should see:
```
INFO SpecKitConstitution: execute() called
INFO SpecKitConstitution: Looking for constitution at: ...
INFO ACE: Pinned N bullets to playbook
```
