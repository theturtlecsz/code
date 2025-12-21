# Continuation Prompt: SPEC-KIT-921 P2 Phase

**Date Created:** 2025-12-21
**Previous Commits:**
- `76e62675f` (P0 fixes)
- `f6daa0f0f` (P1 review enhancements)
- `0c61a9dad` (P1 lint/format fixes)
**Status:** Ready for P2 implementation

---

## Context Summary

P1 is complete. The review command now has:
- `--strict-schema` flag (parse failures → exit 3)
- `--strict-artifacts` flag (missing artifacts → exit 2)
- `--strict-warnings` flag (advisory signals → exit 1)
- `--evidence-root` CLI override for custom evidence paths
- SPEC-CI-001 smoke packet with clean/conflict/malformed test cases
- TUI/CLI parity verified (both route through `SpeckitExecutor`)
- 14 CLI integration tests + 193 spec-kit unit tests passing

## P2 Scope: CI Automation + CLI Polish

**Strategic rationale:** Lock the executor contract into GitHub Actions *before* Phase C/D pipeline migration. This ensures regressions are caught automatically, not manually debugged in TUI.

---

### P2-A: GitHub Actions CI Workflow (CRITICAL)

**Goal:** CI runs on PR + main and fails deterministically on contract violations.

**Create:** `.github/workflows/spec-kit-ci.yml`

```yaml
name: Spec-Kit Contract CI

on:
  push:
    branches: [main]
    paths:
      - 'codex-rs/spec-kit/**'
      - 'codex-rs/cli/src/speckit_cmd.rs'
      - 'codex-rs/cli/tests/speckit.rs'
  pull_request:
    paths:
      - 'codex-rs/spec-kit/**'
      - 'codex-rs/cli/src/speckit_cmd.rs'
      - 'codex-rs/cli/tests/speckit.rs'

jobs:
  contract-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-action@stable

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            target
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Build CLI
        run: cargo build -p codex-cli --release
        working-directory: codex-rs

      - name: Run spec-kit unit tests
        run: cargo test -p codex-spec-kit
        working-directory: codex-rs

      - name: Run CLI contract tests
        run: cargo test -p codex-cli --test speckit
        working-directory: codex-rs

      - name: Verify SPEC-CI-001 clean case (exit 0)
        run: |
          ./target/release/code speckit review \
            -C spec-kit/tests/fixtures/SPEC-CI-001 \
            --spec SPEC-CI-001-clean \
            --stage plan \
            --json > /tmp/clean.json
          cat /tmp/clean.json
        working-directory: codex-rs

      - name: Verify SPEC-CI-001 conflict case (exit 2)
        run: |
          ./target/release/code speckit review \
            -C spec-kit/tests/fixtures/SPEC-CI-001 \
            --spec SPEC-CI-001-conflict \
            --stage plan \
            --json > /tmp/conflict.json || exit_code=$?
          [ "$exit_code" = "2" ] || (echo "Expected exit 2, got $exit_code" && exit 1)
          cat /tmp/conflict.json
        working-directory: codex-rs

      - name: Verify --strict-schema triggers exit 3
        run: |
          ./target/release/code speckit review \
            -C spec-kit/tests/fixtures/SPEC-CI-001 \
            --spec SPEC-CI-001-malformed \
            --stage plan \
            --strict-schema \
            --json 2>&1 || exit_code=$?
          [ "$exit_code" = "3" ] || (echo "Expected exit 3, got $exit_code" && exit 1)
        working-directory: codex-rs

      - name: Upload test artifacts on failure
        if: failure()
        uses: actions/upload-artifact@v4
        with:
          name: spec-kit-debug
          path: |
            /tmp/*.json
            codex-rs/spec-kit/tests/fixtures/SPEC-CI-001/
```

**Acceptance Criteria:**
- [ ] CI runs on spec-kit paths (not full workspace)
- [ ] Exit code contract enforced: clean→0, conflict→2, malformed+strict→3
- [ ] Artifacts uploaded on failure for debugging

---

### P2-B: Status Command JSON Output

**Goal:** Machine-parsable status output for CI reporting.

**Implementation:**
1. Add `--json` flag to `StatusArgs` in `cli/src/speckit_cmd.rs` (already exists, verify working)
2. Ensure JSON schema matches review command stability:
   - All paths repo-relative
   - Timestamps in RFC3339
   - Stage snapshots serializable
3. Add CLI integration test for JSON output structure

**Example Output:**
```json
{
  "spec_id": "SPEC-KIT-921",
  "generated_at": "2025-12-21T03:00:00Z",
  "evidence": {
    "commands_bytes": 12345,
    "consensus_bytes": 6789,
    "combined_bytes": 19134,
    "threshold": "Moderate"
  },
  "stage_count": 6,
  "warnings": []
}
```

**Files to Update:**
- `cli/src/speckit_cmd.rs` - Verify/enhance JSON output
- `cli/tests/speckit.rs` - Add `status_json_output_structure` test

---

### P2-C: Exit Code --explain Mode

**Goal:** Help debug CI failures with human-readable exit code explanations.

**Implementation:**
1. Add `--explain` flag to both `StatusArgs` and `ReviewArgs`
2. When used with `--json`, add `"explanation"` field to output
3. Without `--json`, print explanation to stderr

**Output Fields:**
```json
{
  "exit_code": 2,
  "explanation": {
    "code_meaning": "Escalation required - human review needed",
    "trigger_rule": "blocking_signals present",
    "checkpoint": "AfterPlan",
    "verdict": "Escalate",
    "blocking_count": 1,
    "advisory_count": 0,
    "suggestion": "Review conflicts in evidence/consensus/SPEC-ID/"
  }
}
```

**Files to Create/Update:**
- `spec-kit/src/executor/review.rs` - Add `ExitCodeExplanation` struct
- `spec-kit/src/executor/mod.rs` - Export explanation type
- `cli/src/speckit_cmd.rs` - Add `--explain` flag, render explanation

---

## Implementation Order

1. **P2-A: CI Workflow** — Lock the contract first
2. **P2-B: Status JSON** — Needed for CI artifact parsing
3. **P2-C: Exit --explain** — Debugging aid once CI is running

---

## Key Files Reference

| File | Purpose |
|------|---------|
| `.github/workflows/spec-kit-ci.yml` | New CI workflow (create) |
| `cli/src/speckit_cmd.rs` | Add --explain, verify status JSON |
| `cli/tests/speckit.rs` | Add status JSON + explain tests |
| `spec-kit/src/executor/review.rs` | ExitCodeExplanation struct |
| `spec-kit/tests/fixtures/SPEC-CI-001/` | Smoke packet (exists) |

---

## Don't Regress Checklist

1. **193 spec-kit tests pass** — `cargo test -p codex-spec-kit`
2. **14 CLI tests pass** — `cargo test -p codex-cli --test speckit`
3. **Clippy clean** — `cargo clippy -p codex-spec-kit -p codex-cli --all-targets -- -D warnings`
4. **Format clean** — `cargo fmt --all -- --check`
5. **All paths repo-relative** — in JSON output
6. **Exit codes per contract** — 0/1/2/3 meanings unchanged

---

## Commands for Next Session

```bash
# Build
cd /home/thetu/code/codex-rs
cargo build -p codex-spec-kit -p codex-cli

# Test
cargo test -p codex-spec-kit
cargo test -p codex-cli --test speckit

# Lint
cargo clippy -p codex-spec-kit -p codex-cli --all-targets -- -D warnings
cargo fmt --all -- --check

# Manual CLI verification
./target/debug/code speckit review \
  -C spec-kit/tests/fixtures/SPEC-CI-001 \
  --spec SPEC-CI-001-clean --stage plan --json

./target/debug/code speckit status --spec SPEC-KIT-921 --json
```

---

## Prompt for Next Session

```
Continue SPEC-KIT-921 P2 implementation from docs/spec-kit/CONTINUATION-PROMPT-P2.md

Context:
- P0 and P1 complete (commits through 0c61a9dad)
- 14 CLI tests + 193 spec-kit tests passing
- TUI/CLI parity verified, review command has strict modes + evidence-root override
- SPEC-CI-001 smoke packet exists with clean/conflict/malformed cases

P2 Tasks (in order):
1. P2-A: Create GitHub Actions CI workflow (spec-kit-ci.yml)
   - Exit code contract enforcement
   - SPEC-CI-001 smoke tests
   - Artifact upload on failure
2. P2-B: Verify/enhance status command JSON output
   - Add test for JSON structure
   - Ensure repo-relative paths
3. P2-C: Add --explain flag for exit code debugging
   - Human-readable explanation of why review passed/failed
   - Works with and without --json

Start with P2-A (CI workflow creation).
```
