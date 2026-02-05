# Gold Run Playbook

> **Status**: Needs refresh. Use `codex-rs/SPEC.md` as the canonical tracker for the current gold-run SPEC and acceptance criteria.

**Purpose**: Reproducible steps to validate the `/speckit.auto` pipeline produces a complete evidence chain.

**Target SPEC**: SPEC-DOGFOOD-002

---

## Prerequisites

### 1. Build

```bash
cd codex-rs
cargo build -p codex-tui --release
```

### 2. Configuration

Ensure the following are configured:

```bash
# Stage0 context injection (optional but recommended)
export SPEC_KIT_STAGE0_ENABLED=true

# Auto-commit (keeps tree clean between stages)
export SPEC_KIT_AUTO_COMMIT=true

# Quality gates (default: enabled)
export SPEC_KIT_QUALITY_GATES=true
```

### 3. Clean State

```bash
# Verify no uncommitted changes
git status

# If dirty, either commit or stash
git stash push -m "pre-gold-run"
```

---

## Execution

### Option A: Interactive Mode

```bash
# Start TUI
./target/release/codex-tui

# Run gold run SPEC
/speckit.auto SPEC-DOGFOOD-002
```

### Option B: Automated Mode

```bash
# Single command (useful for CI)
./target/release/codex-tui --initial-command "/speckit.auto SPEC-DOGFOOD-002"
```

---

## Expected Pipeline Flow

```
Stage 0: Context Injection
├── Load spec.md from docs/SPEC-DOGFOOD-002/
├── Generate TASK_BRIEF.md (if tier2 enabled)
└── Inject context into agent prompts

Stage 1: Plan
├── Guardrail: Validate SPEC structure
├── Execute: Generate plan.md
├── Gate: Evaluate plan quality
└── Auto-commit (if enabled)

Stage 2: Tasks
├── Guardrail: Validate plan exists
├── Execute: Generate tasks.md
├── Gate: Evaluate task breakdown
└── Auto-commit

Stage 3: Implement
├── Guardrail: Validate tasks exist
├── Execute: Generate implement.md
├── Gate: Evaluate implementation
└── Auto-commit

Stage 4: Validate
├── Guardrail: Validate implementation exists
├── Execute: Generate validate.md
├── Gate: Evaluate validation results
└── Auto-commit

Stage 5: Audit
├── Guardrail: Validate all prior stages
├── Execute: Generate audit.md
├── Gate: Evaluate audit findings
└── Auto-commit

Stage 6: Unlock
├── Guardrail: Validate audit complete
├── Execute: Generate unlock.md
├── Gate: Final sign-off
├── Auto-commit
└── Pipeline Complete
```

---

## Verification

After pipeline completes, run:

```bash
/speckit.verify SPEC-DOGFOOD-002
```

### Expected Output

```
SPEC-DOGFOOD-002 Verification Report
================================

Stage Artifacts:
  [x] plan.md exists (N bytes)
  [x] tasks.md exists (N bytes)
  [x] implement.md exists (N bytes)
  [x] validate.md exists (N bytes)
  [x] audit.md exists (N bytes)
  [x] unlock.md exists (N bytes)

Evidence Chain:
  [x] evidence/consensus/SPEC-DOGFOOD-002/ directory exists
  [x] 12 verdict files present
  [x] Cost summary written

Metrics:
  Total duration: XX:XX
  Total cost: $X.XX
  Evidence footprint: X.X MB

Result: PASS
```

---

## Troubleshooting

### Pipeline Stalls

**Symptom**: No progress for >5 minutes

**Actions**:
1. Check `/speckit.status SPEC-DOGFOOD-002`
2. Look for "waiting on model output" in status
3. Check API rate limits if using external providers

### Gate Evaluation Fails

**Symptom**: "Stage review failed" or "REVIEW CONFLICT"

**Actions**:
1. Check evidence files for specific failure reason
2. Review `evidence/consensus/SPEC-DOGFOOD-002/<stage>_verdict.json`
3. If sidecar critic enabled, check for advisory signals

### Missing Evidence Files

**Symptom**: Verification reports missing files

**Actions**:
1. Check `evidence/consensus/` directory structure
2. Verify SQLite has entries: `/speckit.db-stats`
3. Check for "export evidence" log messages

### Dirty Tree Errors

**Symptom**: "Working tree has uncommitted changes"

**Actions**:
1. Enable auto-commit: `export SPEC_KIT_AUTO_COMMIT=true`
2. Or bypass: `export SPEC_OPS_ALLOW_DIRTY=1`
3. Commit manually between stages if needed

---

## CI Integration

For continuous integration, use this pattern:

```yaml
# .github/workflows/gold-run.yml
name: Gold Run Validation
on:
  push:
    branches: [main]
  schedule:
    - cron: '0 6 * * 1'  # Weekly Monday 6am UTC

jobs:
  gold-run:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Build
        run: cargo build -p codex-tui --release
        working-directory: codex-rs

      - name: Run Gold SPEC
        run: |
          timeout 30m ./target/release/codex-tui \
            --initial-command "/speckit.auto SPEC-DOGFOOD-002"
        working-directory: codex-rs
        env:
          SPEC_KIT_AUTO_COMMIT: true

      - name: Verify Evidence
        run: ./target/release/codex-tui \
          --initial-command "/speckit.verify SPEC-DOGFOOD-002"
        working-directory: codex-rs

      - name: Upload Evidence
        uses: actions/upload-artifact@v4
        with:
          name: gold-run-evidence
          path: codex-rs/evidence/consensus/SPEC-DOGFOOD-002/
```

---

## Success Criteria

A successful gold run produces:

1. **6 stage artifacts** in `docs/SPEC-DOGFOOD-002/`
2. **12 evidence files** in `evidence/consensus/SPEC-DOGFOOD-002/`
3. **Cost summary** with timing data
4. **Zero manual intervention** after initiation
5. **Clean git history** (if auto-commit enabled)

---

## Related Documentation

- [NEXT_FOCUS_ROADMAP.md](NEXT_FOCUS_ROADMAP.md) - P0 priority definition
- [HANDOFF.md](../HANDOFF.md) - Migration context (PR7-PR9)
- [SPEC-KIT-922](SPEC-KIT-922-AUTO-COMMIT.md) - Auto-commit feature

---

**Last Updated**: 2025-12-19
**Status**: Ready for validation
