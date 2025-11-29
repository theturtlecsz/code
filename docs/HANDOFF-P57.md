# P57 SYNC CONTINUATION PROMPT

**Generated**: 2025-11-29
**Previous Session**: P56 (prep work, SPECs created)
**Model**: Claude Opus 4.5

---

## Session Goals (Priority Order)

### 1. Commit P56 Work (5 min)
Commit the SPECs and analysis created in P56:
- `docs/SPEC-KIT-956-config-cleanup/PRD.md` (updated with Phase 2)
- `docs/SPEC-KIT-957-specify-nativization/PRD.md` (new)
- `SPEC.md` (rows 22-23 updated)

```bash
git add docs/SPEC-KIT-956-config-cleanup/ docs/SPEC-KIT-957-specify-nativization/ SPEC.md
git commit -m "docs(spec-kit): Add SPEC-KIT-957, update SPEC-KIT-956 with Phase 2"
```

### 2. SPEC-KIT-957: Nativize speckit.specify (~3 hours)

**Goal**: Complete SPEC-KIT-902's unfinished business - make speckit.specify use direct execution like the other 6 stage commands.

**Why First**: Unblocks SPEC-KIT-956 Phase 2 (full config cleanup, ~200 lines removal)

#### Implementation Phases

**Phase 1: Add SpecStage::Specify (30 min)**
- File: `codex-rs/tui/src/spec_prompts.rs`
- Add `Specify` variant to `SpecStage` enum
- Update match arms: `key()`, `command_name()`, `display_name()`
- Decide: Include in `all()` or separate (it precedes main pipeline)

**Phase 2: Prompt Building (45 min)**
- File: `codex-rs/tui/src/spec_prompts.rs`
- Add `build_specify_prompt()` function
- Port orchestrator-instructions from config.toml to Rust
- Context gathering: PRD.md, spec.md, constitution.md

**Phase 3: Routing Integration (30 min)**
- File: `codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs`
- Update `decide_stage_routing()` for Specify
- Use `AggregatorEffort::Minimal` (Tier 1: single agent)
- Add ACE scope mapping if appropriate

**Phase 4: Command Refactor (30 min)**
- File: `codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs`
- Modify `SpecKitSpecifyCommand::execute()`:
  ```rust
  fn execute(&self, widget: &mut ChatWidget, args: String) {
      execute_stage_command(widget, args, SpecStage::Specify, "speckit.specify");
  }

  fn expand_prompt(&self, _args: &str) -> Option<String> {
      None  // SPEC-KIT-957: No longer uses orchestrator pattern
  }
  ```

**Phase 5: Config Cleanup (15 min)**
- File: `~/.code/config.toml`
- Remove ALL `[[subagents.commands]]` entries for speckit.*
- Add comment: "All speckit.* commands use native Rust routing (SPEC-KIT-902, SPEC-KIT-957)"

**Phase 6: Testing (30 min)**
- Manual: `/speckit.specify SPEC-TEST-001`
- Unit test: Prompt builds correctly
- Integration: Verify single-agent routing

### 3. P56: /speckit.project Command (~2-3 hours)

**Goal**: Create command to scaffold new projects for spec-kit workflow.

**Workflow**: `/speckit.project` → `/speckit.new` → `/speckit.auto`

#### Templates to Implement (Full Set)
1. **Rust** - Cargo workspace, CLAUDE.md, memory/constitution.md
2. **Python** - uv/pyproject.toml, src layout, tests/
3. **TypeScript** - package.json, tsconfig, src/, tests/
4. **Generic** - Minimal: CLAUDE.md, docs/, SPEC.md template

#### Implementation Steps
1. Research existing patterns in `codex-rs/tui/src/slash_commands/`
2. Create SPEC-KIT-9XX for /speckit.project
3. Implement command in `spec_kit/commands/`
4. Create template files (embedded or external)
5. Validate with test project creation

---

## Key Files Reference

### SPEC-KIT-957 Implementation
```
codex-rs/tui/src/spec_prompts.rs                    # SpecStage enum
codex-rs/tui/src/chatwidget/spec_kit/commands/special.rs  # SpecKitSpecifyCommand
codex-rs/tui/src/chatwidget/spec_kit/ace_route_selector.rs  # Routing
codex-rs/tui/src/chatwidget/spec_kit/commands/plan.rs  # Reference pattern
~/.code/config.toml                                  # Cleanup target
```

### Config.toml Current State
```
Lines 277-345: [[subagents.commands]] for speckit.* (TO BE REMOVED)
  - speckit.specify (line ~278) - STILL USED, remove after 957
  - speckit.plan (~290) - DEAD CONFIG
  - speckit.tasks (~302) - DEAD CONFIG
  - speckit.implement (~314) - DEAD CONFIG
  - speckit.validate (~400) - DEAD CONFIG
  - speckit.audit (~412) - DEAD CONFIG
  - speckit.unlock (~424) - DEAD CONFIG
```

### Evidence for Dead Config
```rust
// plan.rs:35 - Same pattern in all 6 stage commands
fn expand_prompt(&self, _args: &str) -> Option<String> {
    None // SPEC-KIT-902: No longer uses orchestrator pattern
}

// special.rs:159-164 - Only speckit.specify still uses config
let formatted = codex_core::slash_commands::format_subagent_command(
    "speckit.specify", &args,
    Some(&widget.config.agents),
    Some(&widget.config.subagent_commands),  // <-- CONFIG DEPENDENCY
);
```

---

## Verification Commands

```bash
# Build TUI
~/code/build-fast.sh

# Run tests
cd ~/code/codex-rs && cargo test -p codex-tui

# Check config references
grep -n "speckit" ~/.code/config.toml | head -20

# Verify no subagent lookups for stage commands
grep -rn "subagent_commands" ~/code/codex-rs/tui/src/chatwidget/spec_kit/commands/
```

---

## Success Criteria

### SPEC-KIT-957 Complete When:
- [ ] `SpecStage::Specify` exists with all match arms
- [ ] `/speckit.specify SPEC-ID` works via direct execution
- [ ] Tier 1 routing (single agent) applies
- [ ] ALL speckit.* `[[subagents.commands]]` removed from config.toml
- [ ] TUI builds with 0 warnings
- [ ] Tests pass

### P56 Complete When:
- [ ] `/speckit.project` command exists
- [ ] Creates Rust/Python/TypeScript/Generic templates
- [ ] Generated project has: CLAUDE.md, docs/, SPEC.md template
- [ ] Works with `/speckit.new` flow

---

## Context From Previous Sessions

### P56 Original Goal (from HANDOFF-P56.md)
```
Gap identified: No command to scaffold new projects.
Target workflow: /speckit.project → /speckit.new → /speckit.auto
```

### Config Cleanup Progress (SPEC-KIT-956)
```
Phase 1 (DONE): 551→424 lines (-127, 23%)
  - Deleted kavedarr spec-ops-* commands
  - Deleted [spec_ops_004] sections
  - Deleted commented blocks
  - Deleted ~/.code/spec_ops_004/

Phase 2 (BLOCKED by 957): ~200 more lines
  - Remove ALL speckit.* [[subagents.commands]]
```

---

## Questions Resolved

| Question | Answer |
|----------|--------|
| Priority | SPEC-KIT-957 first, then P56 |
| P56 Templates | Full set (Rust + Python + TypeScript + Generic) |
| Git | Commit P56 work before starting |

---

## Start Commands

```bash
# 1. Load this handoff
cat docs/HANDOFF-P57.md

# 2. Check current state
git status
wc -l ~/.code/config.toml

# 3. Start implementation
# Follow phases in order above
```
