# SPEC-KIT-900: Gold Run Pipeline Validation

> **Status**: Deprecated.
>
> `SPEC-KIT-900` is tracked as completed in `codex-rs/SPEC.md` (headless CLI parity work). The gold-run validation effort is tracked separately in `codex-rs/SPEC.md` (see planned `SPEC-DOGFOOD-002`).

## Status: DEPRECATED

## Overview

Canonical gold run SPEC for validating the `/speckit.auto` pipeline produces a complete evidence chain with zero manual intervention.

This SPEC exists to prove the happy path works. It validates:
1. All 6 stages complete (Plan -> Tasks -> Implement -> Validate -> Audit -> Unlock)
2. Gate evaluation runs at each stage transition
3. Evidence artifacts are written correctly
4. Cost tracking and timing are recorded
5. No manual intervention required after initiation

## Problem Statement

Without a canonical gold run, the pipeline may be "correct in theory" but fail in real execution. This SPEC provides a reproducible test case for verifying the full pipeline behavior.

## Scope

**In Scope:**
- End-to-end pipeline execution validation
- Evidence chain completeness verification
- Cost and timing tracking
- Gate/review evidence file generation

**Out of Scope:**
- Actual code implementation
- Complex architectural decisions
- External API integrations

## Implementation

### Task: Validate Pipeline Infrastructure

Create a minimal validation task that exercises the full pipeline:

1. **Plan Stage**: Document the validation approach
2. **Tasks Stage**: Break down into verification steps
3. **Implement Stage**: Create validation artifacts
4. **Validate Stage**: Run validation checks
5. **Audit Stage**: Review evidence completeness
6. **Unlock Stage**: Mark as complete

### Expected Artifacts

```
docs/SPEC-KIT-900-gold-run/
├── spec.md           # This file
├── plan.md           # Stage 1 output
├── tasks.md          # Stage 2 output
├── implement.md      # Stage 3 output
├── validate.md       # Stage 4 output
├── audit.md          # Stage 5 output
└── unlock.md         # Stage 6 output

evidence/consensus/SPEC-KIT-900/
├── plan_synthesis.json
├── plan_verdict.json
├── tasks_synthesis.json
├── tasks_verdict.json
├── implement_synthesis.json
├── implement_verdict.json
├── validate_synthesis.json
├── validate_verdict.json
├── audit_synthesis.json
├── audit_verdict.json
├── unlock_synthesis.json
└── unlock_verdict.json
```

## Success Criteria

- [ ] `/speckit.auto SPEC-KIT-900` completes without manual intervention
- [ ] All 6 stage artifacts generated (plan.md through unlock.md)
- [ ] Gate evidence files created for each stage transition
- [ ] Cost summary file written with timing data
- [ ] `/speckit.verify SPEC-KIT-900` reports all checks passing

## Test Command

```bash
# Build and run
cd codex-rs
cargo build -p codex-tui --release

# Execute gold run
./target/release/codex-tui --initial-command "/speckit.auto SPEC-KIT-900"

# Verify evidence chain
/speckit.verify SPEC-KIT-900
```

## Related

- **NEXT_FOCUS_ROADMAP.md**: P0 priority that defines this gold run
- **HANDOFF.md**: PR7-PR9 context (gate/review vocabulary complete)
- **SPEC-KIT-922**: Auto-commit integration for stage artifacts

## Evidence Policy

- Evidence footprint: Target < 1 MB (this is a validation SPEC, not implementation)
- Retention: Standard 30-day unlock, 180-day purge

## Decision IDs

N/A — Validation SPEC; validates existing pipeline infrastructure with no new decisions required.
