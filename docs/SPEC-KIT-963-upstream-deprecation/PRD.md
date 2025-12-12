# SPEC-KIT-963: Upstream Command Deprecation

**Status:** In Progress
**Created:** 2025-11-30
**Author:** Claude (P62 session)

---

## Problem Statement

This fork (theturtlecsz/code) retains upstream (upstream repository) slash commands that conflict with the spec-kit workflow:

| Command | Description (upstream) | Conflict |
|---------|----------------------|----------|
| `/plan` | Multi-agent planning | Conflicts with `/speckit.plan` |
| `/solve` | Multi-agent problem solving | No spec-kit equivalent, but confuses namespace |
| `/code` | Multi-agent coding | Conflicts with `/speckit.implement` |

These commands:
1. Appear in the TUI command popup alongside `/speckit.*` commands
2. Have different semantics than spec-kit equivalents
3. Confuse users about which to use
4. Add maintenance burden during upstream syncs

## Decision

**Remove all upstream prompt-expanding subagent commands** (`/plan`, `/solve`, `/code`).

This fork standardizes on `/speckit.*` namespace exclusively.

## Scope

### Files to Modify

#### 1. `codex-rs/tui/src/slash_command.rs`

**Remove enum variants (lines 118-120):**
```rust
// REMOVE:
Plan,
Solve,
Code,
```

**Remove from `description()` (lines 196-198):**
```rust
// REMOVE:
SlashCommand::Plan => "create a comprehensive plan (multiple agents)",
SlashCommand::Solve => "solve a challenging problem (multiple agents)",
SlashCommand::Code => "perform a coding task (multiple agents)",
```

**Simplify `is_prompt_expanding()` (lines 280-286):**
```rust
// BEFORE:
pub fn is_prompt_expanding(self) -> bool {
    matches!(
        self,
        SlashCommand::Plan | SlashCommand::Solve | SlashCommand::Code
    )
}

// AFTER:
pub fn is_prompt_expanding(self) -> bool {
    false // All prompt-expanding commands removed; spec-kit uses registry
}
```

**Remove from `requires_arguments()` (lines 291-295):**
```rust
// REMOVE:
SlashCommand::Plan
    | SlashCommand::Solve
    | SlashCommand::Code
```

**Remove from `expand_prompt()` (lines 353-366):**
```rust
// REMOVE entire match arms:
SlashCommand::Plan => Some(codex_core::slash_commands::format_plan_command(...)),
SlashCommand::Solve => Some(codex_core::slash_commands::format_solve_command(...)),
SlashCommand::Code => Some(codex_core::slash_commands::format_code_command(...)),
```

#### 2. `codex-rs/core/src/slash_commands.rs`

**Remove legacy wrapper functions (lines 151-182):**
- `format_plan_command()`
- `format_solve_command()`
- `format_code_command()`

**Remove `handle_slash_command()` entirely (lines 185-228):**
- Only handles `/plan`, `/solve`, `/code`
- Not used after TUI removal

**Update `default_read_only_for()` (lines 40-45):**
```rust
// BEFORE:
pub fn default_read_only_for(name: &str) -> bool {
    match name {
        "plan" | "solve" => true,
        _ => name != "code",
    }
}

// AFTER:
pub fn default_read_only_for(name: &str) -> bool {
    // Custom subagent commands default to read-only for safety
    true
}
```

**Update `default_instructions_for()` (lines 65-99):**
```rust
// BEFORE: Returns hardcoded instructions for "plan", "solve", "code"
// AFTER: Returns None for all (custom commands provide their own)
pub fn default_instructions_for(_name: &str) -> Option<String> {
    None // Built-in subagent commands removed; use spec-kit
}
```

**Remove tests (lines 230-330):**
- `test_slash_command_parsing` - tests /plan, /solve, /code
- `test_slash_commands_with_agents` - tests agent filtering for these commands

### Files NOT Modified

- `docs/slash-commands.md` - Check if this file exists; if so, update
- `CLAUDE.md` - Already documents `/speckit.*` only
- Tests in other modules - None found

## Acceptance Criteria

1. **Build passes**: `cargo build -p codex-tui -p codex-core` succeeds
2. **Tests pass**: `cargo test -p codex-tui -p codex-core` succeeds
3. **No references**: `grep -r "SlashCommand::Plan\|SlashCommand::Solve\|SlashCommand::Code" codex-rs/` returns empty
4. **TUI command list**: Only shows `/speckit.*` and `/guardrail.*` for multi-agent workflows
5. **SPEC.md updated**: SPEC-KIT-963 marked Done

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Users expect /plan | Low | Low | Fork has documented /speckit.* for months |
| Breaking change for scripts | Very Low | Low | Scripts use spec-kit commands |
| Upstream sync conflicts | Medium | Low | Clear fork-specific block comments |

## Implementation Notes

### Preserving `format_subagent_command()`

The generic `format_subagent_command()` function is retained for potential custom subagent definitions via `[[subagents.commands]]` in config. Only the built-in plan/solve/code defaults are removed.

### Upstream Sync Strategy

After this change, upstream syncs that touch `/plan`, `/solve`, `/code`:
1. Will show conflicts in fork-specific areas
2. Should be resolved by dropping upstream additions
3. Fork-specific comment blocks make this clear

## Estimated Effort

- Implementation: 30 minutes
- Testing: 15 minutes
- Documentation: 10 minutes

---

## Changelog

| Date | Change |
|------|--------|
| 2025-11-30 | PRD created |
