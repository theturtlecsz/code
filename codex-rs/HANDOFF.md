# SYNC-028 Session 9 Handoff - Continue tui2 Error Fixes

**Date**: 2024-12-24
**Session**: 9 of SYNC-028
**Last Commit**: 7743298fc (reduce errors 173→149, fix API divergences)

---

## Session 9 Summary (In Progress)

### Progress: 149 → 56 Errors (62.4% reduction)

#### Error Breakdown (56 remaining)
```
56 E0308 - mismatched types (ALL REMAINING)
```

All remaining errors are type mismatches concentrated in `chatwidget.rs`.

### Key Fixes Applied This Session

1. **ScrollConfigOverrides Type Compatibility**
   - Changed ConfigExt returns from `u32/f64` to `Option<u16/u64>`
   - Required by upstream's generic scroll config system

2. **ModelsManager Arc Wrapping**
   - Return `Arc<ModelsManager>` instead of direct struct
   - Required for upstream's shared state pattern

3. **Model Migration Stubbed Out**
   - Entirely disabled due to `ReasoningEffort` type conflicts
   - Two incompatible enums: `codex_core::config_types` vs `codex_protocol::openai_models`

4. **RateLimitSnapshot Type Switch**
   - Changed from `codex_core::protocol` to `codex_protocol` version
   - Credits display set to `None` (not available in fork)

5. **Function Signature Reductions**
   - `list_conversations`: 6 args → 3 args
   - `ConversationManager::new`: 2 args → 1 arg

6. **Enum Variant Fixes**
   - `FileChange::Delete`: Changed from struct to unit variant
   - `McpStartupStatus::Failed`: Changed from struct to tuple variant

### Build Status After Session 9

| Crate | Status |
|-------|--------|
| codex-protocol | BUILDS |
| codex-core | BUILDS |
| codex-tui (original) | BUILDS |
| codex-app-server-protocol | BUILDS |
| codex-backend-client | BUILDS |
| codex-tui2 | **56 ERRORS** (down from 149) |

---

## Session 10 Tasks

### Priority: Fix Remaining 56 E0308 Type Mismatches

All errors are in `chatwidget.rs`. Focus areas:

1. **Option<T> vs T patterns** - Fork uses direct types, upstream uses Option wrappers
2. **Integer type casts** - u8/u32/u64/i64 conversions
3. **String/&str conversions** - Clone vs reference issues

### Quick Diagnostic Commands

```bash
# Count errors
cargo check -p codex-tui2 2>&1 | grep "^error\[E" | wc -l

# See specific E0308 errors
cargo check -p codex-tui2 2>&1 | grep -A3 "E0308"

# Verify original tui still builds
cargo check -p codex-tui
```

---

## Continue Prompt for Session 10

```
Continue SYNC-028 (TUI v2 port) Session 10 - FINAL PUSH **ultrathink**

## Context
Session 9 reduced errors 149→56 (62.4%). All 56 remaining are E0308 type mismatches.

## Current State
- All E0432 import errors: FIXED
- All E0609 field access errors: FIXED
- All E0599 method errors: FIXED
- All E0061 argument count errors: FIXED
- Remaining: 56 E0308 type mismatches in chatwidget.rs

## Key Files
- tui2/src/chatwidget.rs - Main chat widget (most errors)
- tui2/src/compat.rs - Compatibility layer

## Patterns to Apply
1. Option<T> wrapping: Add .map() or unwrap_or_default()
2. Integer casts: as u16, as i64, etc.
3. String conversions: .to_string(), .clone()

## Success Criteria
- [ ] cargo build -p codex-tui2 COMPILES
- [ ] ./target/debug/codex-tui2 --help runs
- [ ] cargo build -p codex-tui still works
- [ ] Commit with `feat(tui2): complete port`
```

---

## Files Modified Session 9

| File | Changes |
|------|---------|
| `tui2/src/compat.rs` | ConfigExt Option returns, Arc<ModelsManager>, format_env_display |
| `tui2/src/app.rs` | Model migration stubbed, ConstraintResult handling |
| `tui2/src/chatwidget.rs` | RateLimitSnapshot type, integer casts, pattern fixes |
| `tui2/src/status/rate_limits.rs` | Credits set to None |
| `tui2/src/bottom_pane/approval_overlay.rs` | Pattern matching fixes |
| `tui2/src/bottom_pane/skill_popup.rs` | SkillMetadata field access |

---

## Key Patterns Documented

### ReasoningEffort Type Conflict (BLOCKER)
```rust
// Two incompatible types with same name:
codex_core::config_types::ReasoningEffort      // Fork's version
codex_protocol::openai_models::ReasoningEffort // Upstream's version

// Resolution: Stubbed out model migration entirely
async fn handle_model_migration_prompt_if_needed(...) -> Option<AppExitInfo> {
    None // Model migration not supported in fork
}
```

### Option<T> Wrapping Pattern
```rust
// Fork returns direct value:
fn scroll_config_vertical() -> u32

// Upstream expects Option:
fn scroll_config_vertical() -> Option<u16>

// Fix: Change return type and add conversion
fn scroll_config_vertical(&self) -> Option<u16> {
    Some(8) // Default value as Option
}
```

### Arc Wrapping Pattern
```rust
// Fork returns struct directly:
fn get_models_manager() -> ModelsManager

// Upstream expects Arc:
fn get_models_manager() -> Arc<ModelsManager>

// Fix: Wrap in Arc
pub fn get_models_manager(_config: &Config) -> Arc<ModelsManager> {
    Arc::new(ModelsManager { ... })
}
```

---

## Session History

| Session | Errors | Reduction | Key Work |
|---------|--------|-----------|----------|
| 7 | 262→173 | 34% | Migration docs, compat.rs foundation |
| 8 | 173→149 | 14% | API divergence fixes |
| 9 | 149→56 | 62% | Type compatibility, model migration stub |
| 10 | 56→0 | TBD | Final type mismatches |

---

_Last updated: 2024-12-24 (SYNC-028 Session 9 in progress)_
