# ACE /speckit.constitution Command Not Working - Root Cause

## Problem

User types `/speckit.constitution` in CODE TUI → CODE treats it as a question to AI instead of executing the command.

## Root Cause Analysis

### What We Know

✅ Command IS registered in command_registry.rs (line 157)
✅ Command IS in compiled binary (confirmed via strings)
✅ Routing IS wired in app.rs:1708 (try_dispatch_spec_kit_command)
✅ Tests verify command exists (lines 302, 418)
❌ Command NOT executing when typed

### The Missing Piece

**Hypothesis**: Upstream `SlashCommand` enum needs `/speckit.constitution` added.

The flow is likely:
1. User types `/speckit.constitution`
2. CODE tries to parse it as SlashCommand enum
3. Parse FAILS (not in enum)
4. Falls through to treating it as user input
5. Sent to AI instead of creating DispatchCommand event

### Check Needed

Look at `tui/src/slash_command.rs`:
- Is there a `SlashCommand::SpecKitConstitution` variant?
- Or does it fallthrough to spec-kit registry?

If enum doesn't have it, we need to add:
```rust
pub enum SlashCommand {
    // ... existing variants ...
    SpecKitConstitution,
    // ...
}
```

## Quick Test

Try these existing direct commands that SHOULD work:
- `/speckit.status` - Does this work?
- `/spec-consensus SPEC-KIT-069 plan` - Does this work?

If those work but `/speckit.constitution` doesn't, it confirms the enum hypothesis.
