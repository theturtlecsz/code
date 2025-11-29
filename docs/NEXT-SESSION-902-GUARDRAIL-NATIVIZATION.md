# SPEC-KIT-902: Nativize Guardrail Scripts - Deep Dive Session

**Created**: 2025-11-29
**Purpose**: PRD creation and scope analysis for converting shell guardrails to native Rust
**Estimated Duration**: 2-3 hours (PRD creation) + assessment

---

## Session Objectives

1. **Audit all guardrail scripts** - Understand what each does, identify dead code
2. **Assess native coverage** - What's already in Rust vs still in bash
3. **Create PRD** - Detailed requirements with effort estimates per script
4. **Decide scope** - Full nativization vs selective (cost/benefit)

---

## Context Summary

### Architecture Backlog Status
- **5/7 Complete**: 909 ✅, 903 ✅, 910 ✅ (already done), 901 ✅ (obsolete)
- **902 is the LAST remaining item** from Oct 2025 architecture review

### What Changed Since Oct 2025
- **SPEC-KIT-070** (Nov 2025): 4 quality commands now native (clarify, analyze, checklist, status)
- **SPEC-936** (Nov 2025): Tmux eliminated → DirectProcessExecutor
- **SPEC-934** (Nov 2025): MCP eliminated from hot path → SQLite
- **Evidence lifecycle** (SPEC-909): Scripts exist but could be native

### Current Guardrail Scripts (1,555 LOC total)

| Script | Lines | Purpose | Native Candidate? |
|--------|-------|---------|-------------------|
| `common.sh` | 423 | Shared helpers, JSON generation, telemetry | HIGH - core utilities |
| `consensus_runner.sh` | 456 | Agent consensus orchestration | MAYBE OBSOLETE - DirectProcessExecutor replaced? |
| `evidence_archive.sh` | 173 | Compress >30d evidence | MEDIUM - simple file ops |
| `evidence_cleanup.sh` | 182 | Purge >180d evidence | MEDIUM - simple file ops |
| `evidence_stats.sh` | 134 | Report evidence sizes | LOW - diagnostic only |
| `baseline_audit.sh` | 82 | Git baseline checks | HIGH - called per-stage |
| `log_agent_runs.sh` | 105 | Agent execution logging | MAYBE OBSOLETE - execution_logger.rs? |

### Existing Native Infrastructure

```
codex-rs/tui/src/chatwidget/spec_kit/
├── native_guardrail.rs    # Partial: clean tree, SPEC validation (~200 LOC)
├── commands/guardrail.rs  # Command wrappers (still delegate to shell)
├── evidence.rs            # Evidence handling
├── execution_logger.rs    # May overlap with log_agent_runs.sh
├── git_integration.rs     # Git operations (auto-commit)
└── pipeline_coordinator.rs # Orchestrates stages
```

---

## Investigation Tasks

### Task 1: Script-by-Script Analysis

For each script, determine:
1. **What it does** (read the code)
2. **Who calls it** (grep for usage)
3. **Is it still used?** (post-SPEC-936/934 changes)
4. **Native equivalent exists?** (check Rust code)
5. **Effort to nativize** (complexity assessment)

### Task 2: Identify Dead Code

Scripts that may be obsolete:
- `consensus_runner.sh` - DirectProcessExecutor replaced tmux-based orchestration
- `log_agent_runs.sh` - `execution_logger.rs` may cover this
- Parts of `common.sh` - HAL integration if not used

### Task 3: Create PRD Structure

```markdown
# SPEC-KIT-902 PRD

## Problem Statement
- Shell scripts add ~150ms latency per call (process spawn)
- Cross-platform issues (Windows compatibility)
- Error handling inconsistent with Rust codebase
- Maintenance burden (two languages)

## Requirements
### Must Have
- [ ] ...

### Should Have
- [ ] ...

### Won't Have (Deferral)
- [ ] ...

## Implementation Plan
### Phase 1: Foundation (Xh)
### Phase 2: Core Scripts (Xh)
### Phase 3: Evidence Management (Xh)
### Phase 4: Cleanup (Xh)

## Acceptance Criteria
...
```

---

## Key Questions to Answer

1. **Is `consensus_runner.sh` dead code?**
   - DirectProcessExecutor replaced tmux orchestration
   - If dead, that's 456 lines we don't need to port

2. **What telemetry format does `common.sh` produce?**
   - Need to maintain compatibility or migrate consumers

3. **Are evidence scripts used outside TUI?**
   - If CLI-only, can defer nativization

4. **What's the Windows story?**
   - Shell scripts don't work on Windows
   - Native Rust would enable Windows support

5. **Cost/benefit of partial vs full nativization?**
   - SPEC-070 already reduced scope 46%
   - Maybe only 2-3 scripts need native ports

---

## Files to Read

```bash
# Guardrail scripts
scripts/spec_ops_004/common.sh           # 423 lines - PRIORITY
scripts/spec_ops_004/consensus_runner.sh # 456 lines - CHECK IF DEAD
scripts/spec_ops_004/baseline_audit.sh   # 82 lines

# Evidence scripts
scripts/spec_ops_004/evidence_archive.sh
scripts/spec_ops_004/evidence_cleanup.sh
scripts/spec_ops_004/evidence_stats.sh

# Native implementations
codex-rs/tui/src/chatwidget/spec_kit/native_guardrail.rs
codex-rs/tui/src/chatwidget/spec_kit/execution_logger.rs
codex-rs/tui/src/chatwidget/spec_kit/evidence.rs
codex-rs/tui/src/chatwidget/spec_kit/git_integration.rs
```

---

## Expected Outcomes

1. **PRD document** at `docs/SPEC-KIT-902-nativize-guardrails/PRD.md`
2. **Revised effort estimate** (likely lower than 23-35h if scripts are dead)
3. **Phase breakdown** with clear priorities
4. **Decision**: Full nativization vs selective port vs close as low-value

---

## Session Commands

```bash
# Load this prompt
load ~/code/docs/NEXT-SESSION-902-GUARDRAIL-NATIVIZATION.md

# Quick script audit
wc -l ~/code/scripts/spec_ops_004/*.sh
grep -r "consensus_runner" ~/code/codex-rs/
grep -r "baseline_audit" ~/code/codex-rs/

# Check what calls the scripts
grep -r "spec_ops_004" ~/code/codex-rs/tui/src/
```

---

## Success Criteria

- [ ] All 7 scripts analyzed (purpose, usage, native equivalent)
- [ ] Dead code identified and marked for removal
- [ ] PRD created with accurate effort estimates
- [ ] Decision made: proceed, defer, or close
