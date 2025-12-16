# P117 Session Prompt

```
load HANDOFF.md **ultrathink**
```

Continue P117: MAINT-11 Phase 6 (Browser/Chrome code removal investigation).

## Context
- P116 completed: `input_helpers.rs` extraction (175 LOC, 5 tests)
- Commits: `d5c58634c` (input_helpers), `132c87752` (handoff)
- mod.rs: 22,852 LOC remaining
- **User guidance**: Browser/Chrome code is likely dead code from upstream `just-every/code` - investigate for removal

## Primary Task
Investigate and potentially **remove** browser/chrome integration code from mod.rs.

### Phase 1: Investigation
Search for browser/chrome usage patterns:
```bash
cd codex-rs

# Find all browser/chrome handler functions
grep -n "handle_browser\|handle_chrome\|browser_hud\|chrome" tui/src/chatwidget/mod.rs | head -40

# Check if browser commands are registered
grep -n "browser\|chrome" tui/src/slash_command.rs

# Check for external references to browser functionality
grep -rn "handle_browser\|handle_chrome\|browser_hud" tui/src/ --include="*.rs" | grep -v "mod.rs"

# Check app.rs for browser event handling
grep -n "browser\|chrome" tui/src/app.rs | head -20
```

### Phase 2: Assess Dependencies
Before removal, verify:
1. No slash commands reference browser functionality
2. No app events depend on browser state
3. No tests reference browser methods
4. BrowserHud struct and related types can be removed

### Phase 3: Staged Removal
If confirmed dead:
1. Remove `handle_browser_command` (~150 LOC)
2. Remove `handle_chrome_command` (~200 LOC)
3. Remove `handle_chrome_connection` (~50 LOC)
4. Remove `handle_chrome_launch_option` (~50 LOC)
5. Remove `toggle_browser_hud` and related (~50 LOC)
6. Remove unused imports and types
7. Clean up any orphaned structs

### Verification
```bash
cargo test -p codex-tui
cargo clippy -p codex-tui -- -D warnings
cargo build -p codex-tui
```

## Safety Checks
- [ ] Search for ALL usages before removing any function
- [ ] Check slash_command.rs for /browser, /chrome commands
- [ ] Check app.rs for browser-related AppEvent handling
- [ ] Verify no tests depend on browser functionality
- [ ] Confirm removal doesn't break compilation

## Expected Outcome
If browser code is confirmed dead:
- mod.rs: ~22,350 LOC (target -500 LOC)
- Zero browser/chrome related code remaining
- All tests pass
- No new warnings

---

### Session Commits So Far
- `d5c58634c` - refactor(tui): extract input_helpers module (MAINT-11 Phase 5)
- `132c87752` - docs(handoff): add P117 session handoff document

---

### P116 Session Summary

| Metric | Value |
|--------|-------|
| Commits | 2 |
| New module | `input_helpers.rs` (175 LOC, 5 tests) |
| mod.rs LOC | 22,906 → 22,852 (-54) |
| Tests | 539 pass |
| Clippy | 0 warnings |

---

### MAINT-11 Cumulative Progress

| Phase | Session | Change | mod.rs Total |
|-------|---------|--------|--------------|
| 1 | P110 | -200 extracted | 23,213 |
| 2 | P113 | -65 extracted | 23,151 |
| 3 | P114 | -300 extracted | 22,911 |
| 4 | P115 | -5 removed | 22,906 |
| 5 | P116 | -54 extracted | 22,852 |
| 6 | P117 | ~-500 (target) | ~22,350 |

---

_Generated: 2025-12-16 after P116 completion_
