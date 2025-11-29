# SPEC-KIT-956: Config.toml Cleanup - Remove Stale Agent Expand Commands

**Status**: Phase 1 Complete, Phase 2 Blocked
**Created**: 2025-11-29
**Trigger**: Post SPEC-KIT-902 audit - native Rust commands replaced shell scripts but config not updated
**Blocked By**: SPEC-KIT-957 (speckit.specify nativization)

---

## Problem Statement

SPEC-KIT-902 (commit `bddd82fd7`) eliminated the orchestrator pattern and deleted shell scripts from `scripts/spec_ops_004/`. However, `~/.code/config.toml` still contains:

1. **Project command entries** pointing to deleted shell scripts
2. **Global section declarations** for obsolete spec_ops_004 paths
3. **80 lines of commented dead code** (deprecated speckit.new subagent)
4. **Orphaned directories** with stale September 2024 scripts

This creates confusion, potential runtime errors, and maintenance debt.

---

## Evidence

### Deleted by SPEC-KIT-902 (2025-11-29)
```
scripts/spec_ops_004/commands/spec_ops_audit.sh
scripts/spec_ops_004/commands/spec_ops_implement.sh
scripts/spec_ops_004/commands/spec_ops_plan.sh
scripts/spec_ops_004/commands/spec_ops_status.sh
scripts/spec_ops_004/commands/spec_ops_tasks.sh
scripts/spec_ops_004/commands/spec_ops_unlock.sh
scripts/spec_ops_004/commands/spec_ops_validate.sh
scripts/spec_ops_004/common.sh
scripts/spec_ops_004/consensus_runner.sh
scripts/spec_ops_004/baseline_audit.sh
```

### Still Referenced in config.toml
```toml
# Lines 39-73: kavedarr project commands (STALE)
[[projects."/home/thetu/kavedarr".commands]]
name = "spec-ops-plan"
run = ["bash", "scripts/spec_ops_004/commands/spec_ops_plan.sh"]  # DELETED

# Lines 74-77: Global paths (STALE)
[spec_ops_004]
hooks_dir = "/home/thetu/.code/spec_ops_004/hooks"
commands_dir = "/home/thetu/.code/spec_ops_004/commands"
validation_dir = "/home/thetu/.code/spec_ops_004/validation"

# Lines 134-136: Duplicate declaration (STALE)
[spec_ops_004_commands]
path = "/home/thetu/.code/spec_ops_004/commands"

# Lines 313-396: Commented dead code (NOISE)
# DEPRECATED: /speckit.new is now fully native (SPEC-KIT-072)
# ... 80 lines of commented TOML ...
```

### Orphaned Directories (Still Exist)
```
~/.code/spec_ops_004/commands/   # 21 files, September 24 dates
~/.code/spec_ops_004/hooks/
~/.code/spec_ops_004/validation/
~/kavedarr/scripts/spec_ops_004/ # Separate project, may keep
```

---

## Scope

### In Scope
1. Remove stale config.toml entries (lines 39-77, 134-136)
2. Delete commented speckit.new block (lines 313-396)
3. Delete `~/.code/spec_ops_004/` directory entirely
4. Update any documentation referencing old paths

### Out of Scope
- `~/kavedarr/scripts/spec_ops_004/` - separate project, separate decision
- Agent definitions (`[[agents]]`) - these are still valid
- Subagent commands (specify/plan/tasks/etc) - these define multi-agent routing, NOT shell scripts

---

## Requirements

### R1: Remove Stale Project Commands
**Delete lines 39-73** from config.toml:
- `spec-ops-plan`
- `spec-ops-tasks`
- `spec-ops-implement`
- `spec-ops-validate`
- `spec-ops-review`
- `spec-ops-unlock`

These pointed to shell scripts deleted by SPEC-KIT-902.

### R2: Remove Global spec_ops_004 Sections
**Delete lines 74-77 and 134-136**:
```toml
[spec_ops_004]  # DELETE
hooks_dir = ...
commands_dir = ...
validation_dir = ...

[spec_ops_004_commands]  # DELETE
path = ...
```

### R3: Delete Commented Dead Code
**Delete lines 313-396** - the 80-line commented `speckit.new` subagent block. This is documentation debt, not useful history (git preserves it).

### R4: Clean Up Orphaned Directory
**Delete `~/.code/spec_ops_004/`** entirely:
```bash
rm -rf ~/.code/spec_ops_004/
```

### R5: Verify No Breakage
After cleanup, verify:
- TUI starts without config errors
- `/speckit.plan` routes through subagent config (lines 412-419), not deleted shell scripts
- No runtime references to spec_ops_004 paths

---

## Acceptance Criteria

- [ ] config.toml reduced by ~100 lines
- [ ] No `spec_ops_004` string in config.toml except comments explaining native replacement
- [ ] `~/.code/spec_ops_004/` directory deleted
- [ ] TUI builds and runs without error
- [ ] `/speckit.status SPEC-KIT-956` shows this SPEC

---

## Implementation Notes

### What to Keep (Phase 1)
- `[[agents]]` definitions (gemini, claude, gpt_*, etc.)
- `[[subagents.commands]]` - **TEMPORARILY** (see Phase 2 below)
- Project trust/sandbox settings for kavedarr, code, etc.
- kavedarr hooks (lines 21-37) - these may still be needed

**NOTE**: Phase 2 will remove ALL speckit.* subagent commands after SPEC-KIT-957.
6 of 7 are already dead config (plan/tasks/implement/validate/audit/unlock use native Rust).

### Native Rust Replacements (Already Done)
| Old Shell Script | Native Rust |
|------------------|-------------|
| spec_ops_plan.sh | `chatwidget/spec_kit/commands/plan.rs` |
| spec_ops_tasks.sh | Pipeline coordinator + subagent routing |
| spec_ops_implement.sh | Pipeline coordinator + subagent routing |
| evidence_stats.sh | `spec_kit/evidence.rs::check_spec_evidence_limit()` |
| consensus_runner.sh | `spec_kit/agent_orchestrator.rs` |

---

## Risks

1. **kavedarr breakage**: If kavedarr project actively uses these hooks, removing them from config would break workflows
   - **Mitigation**: Keep project hooks (lines 21-37), only remove command entries

2. **Hidden dependencies**: Some tool may parse config.toml sections we think are unused
   - **Mitigation**: Search codebase for `spec_ops_004` string before deletion

---

## Cost/Benefit

- **Effort**: ~30 minutes (Phase 1), ~15 minutes (Phase 2 after SPEC-KIT-957)
- **Benefit**:
  - ~100 lines removed from config (Phase 1)
  - ~200 additional lines removed (Phase 2)
  - ~20 MB freed from ~/.code/ (old scripts + evidence)
  - Reduced confusion when editing config
  - No runtime path resolution errors for deleted scripts

---

## Phase 2: Full Subagent Cleanup (BLOCKED)

**Blocked by**: SPEC-KIT-957 (speckit.specify nativization)

### Discovery (2025-11-29)

Deep analysis revealed that SPEC-KIT-902 nativized 6 of 7 speckit subagent commands:

| Command | Config Used? | Reason |
|---------|--------------|--------|
| speckit.specify | **YES** | Still uses `format_subagent_command` |
| speckit.plan | **NO** | SPEC-KIT-902: Direct execution |
| speckit.tasks | **NO** | SPEC-KIT-902: Direct execution |
| speckit.implement | **NO** | SPEC-KIT-902: Direct execution |
| speckit.validate | **NO** | SPEC-KIT-902: Direct execution |
| speckit.audit | **NO** | SPEC-KIT-902: Direct execution |
| speckit.unlock | **NO** | SPEC-KIT-902: Direct execution |

**Evidence**:
- `plan.rs:35`: `None // SPEC-KIT-902: No longer uses orchestrator pattern`
- `special.rs:159-164`: Only speckit.specify calls `format_subagent_command`

### Phase 2 Scope (After SPEC-KIT-957)

Once SPEC-KIT-957 nativizes speckit.specify:

1. Delete ALL `[[subagents.commands]]` entries for speckit.*
2. Remove ~200 lines of subagent config
3. Add comment: "All speckit.* commands use native Rust routing"

### Phase 2 Acceptance Criteria

- [ ] Zero `[[subagents.commands]]` entries for speckit.*
- [ ] config.toml reduced by additional ~200 lines
- [ ] TUI builds and all speckit commands work

---

## Completion Summary

### Phase 1 (COMPLETE - 2025-11-29)
- Removed 127 lines (23% reduction)
- Deleted kavedarr spec-ops-* commands, [spec_ops_004] sections, commented blocks
- Deleted ~/.code/spec_ops_004/ directory (120KB)

### Phase 2 (BLOCKED by SPEC-KIT-957)
- Will remove ~200 additional lines
- Full [[subagents.commands]] cleanup for speckit.*
- Requires speckit.specify nativization first
