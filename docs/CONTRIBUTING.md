# Contributing Guide

> **Version**: 1.0.0 (2026-01-21)
>
> **Purpose**: Development guidelines, fork workflow, and contribution standards.
>
> **Supersedes**: `CONTRIBUTING.md` (root), `docs/TUI.md` (fork management sections)

---

## Table of Contents

**Part I: Development Standards**
- [1. Architecture Overview](#1-architecture-overview)
- [2. Code Standards](#2-code-standards)
- [3. High-Risk Modules](#3-high-risk-modules)

**Part II: Development Workflow**
- [4. Development Setup](#4-development-setup)
- [5. Branch and PR Strategy](#5-branch-and-pr-strategy)
- [6. Testing Requirements](#6-testing-requirements)

**Part III: Fork Management**
- [7. Fork Deviation Tracking](#7-fork-deviation-tracking)
- [8. Rebase Strategy](#8-rebase-strategy)
- [9. Validation After Rebase](#9-validation-after-rebase)

**Appendices**
- [A. Rebase Validation Script](#a-rebase-validation-script)
- [B. Change History](#b-change-history)

---

# Part I: Development Standards

## 1. Architecture Overview

The codex-rs system follows a clear separation of concerns, dividing the user interface from core operational logic.

### Mental Model

1. **TUI (Terminal User Interface)**: The TUI components (`codex-rs/tui/src/app.rs`) serve as the user interaction surface. The ChatWidget manages conversational state, rendering pipeline, event routing, and agent orchestration.

2. **Backend/Core (Codex Engine)**: Handles core conversational operations (`Op`) and business logic (DCC pipeline, sandbox policies).

### Communication Pattern

- **TUI to Core**: TUI uses channels (`codex_op_tx`) to send core operations (`Op`) to Codex engine
- **Core to TUI**: Core sends events via `AppEvent` enumeration (`codex-rs/tui/src/app_event.rs`)
- **UI Events**: ChatWidget sends internal events to app loop via `app_event_tx`

## 2. Code Standards

### Avoid Monolithic Control Flow

Do not introduce large `match` statements handling numerous pathways.

**Standard**: Use Strategy and Polymorphism (Rust traits) to abstract complexity.

### Limit Parameter Arity

Avoid functions with complex, numerous optional parameters.

**Standard**: Employ the Fluent Builder Pattern.

### No New JavaScript or Shell Control Logic

Due to high-risk scores (JS: **286.9**; Shell: **157.0**), do not introduce new complex logic via JavaScript or Shell scripts.

### Enforce Explicit Dependencies

If a change to Module A forces a change to Module B, there must be an explicit structural dependency.

## 3. High-Risk Modules

### TUI Core (The Chat Nexus)

| File | Metric | Assessment |
|------|--------|------------|
| `codex-rs/tui/src/chatwidget.rs` | 509 commits | **High Volatility** - entangled monolith |
| `codex-rs/tui/src/app.rs` | 22 co-changes | **Tight Coupling** - cascading modifications |

### Glue Components

| Language | Risk Score | Assessment |
|----------|------------|------------|
| JavaScript | **286.9** | Highest risk - `codex-cli/package.json` |
| BASH/Shell | **265.3** | Highly fragile - env configuration |

---

# Part II: Development Workflow

## 4. Development Setup

### Prerequisites

- Rust toolchain (stable)
- cargo, clippy, rustfmt
- Git (with hooks configured)

### Build Commands

```bash
# Fast build (dev-fast profile)
~/code/build-fast.sh

# Build and run TUI
~/code/build-fast.sh run

# Release build
PROFILE=release ~/code/build-fast.sh
```

### Setup Git Hooks

```bash
bash scripts/setup-hooks.sh
```

### Test Commands

```bash
cd codex-rs
cargo test -p codex-core                           # All core tests
cargo test -p codex-core -- suite::fork_conversation  # Specific module
cargo fmt --all -- --check                         # Format check
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Lint
```

## 5. Branch and PR Strategy

### Branch Naming

| Type | Pattern | Example |
|------|---------|---------|
| Feature | `feat/<SPEC-ID>-<slug>` | `feat/SPEC-KIT-900-pipeline` |
| Fix | `fix/<SPEC-ID>-<slug>` | `fix/SPEC-KIT-901-consensus` |
| Docs | `docs/<slug>` | `docs/architecture-cleanup` |

### Commit Messages

Conventional format required:
- `feat(scope):` - New feature
- `fix(scope):` - Bug fix
- `docs(scope):` - Documentation
- `refactor(scope):` - Code restructuring
- `test(scope):` - Test additions

### PR Requirements

1. Clean tree (no uncommitted changes)
2. All tests pass
3. doc_lint.py passes
4. SPEC.md updated (if task-related)

## 6. Testing Requirements

### Unit Tests

```rust
mod spec_auto_tests {
    #[test]
    fn state_machine_transitions() {
        // Guardrail -> ExecutingAgents -> CheckingConsensus -> NextStage
    }

    #[test]
    fn consensus_conflict_halts() {
        // synthesis.json status=conflict -> pipeline stops
    }
}
```

### Integration Tests

- Run full pipeline with mocked guardrails
- Verify 6 stages execute correctly
- Test halt conditions

### Manual Validation Checklist

- [ ] `/new-spec` creates valid SPEC package
- [ ] `/spec-auto` shows progress
- [ ] Agents spawn visibly with correct models
- [ ] Consensus results shown clearly
- [ ] Auto-advances on success
- [ ] Halts on failure with error
- [ ] Can cancel with Ctrl-C

---

# Part III: Fork Management

## 7. Fork Deviation Tracking

### Critical TUI Files

| File | Changes | Markers |
|------|---------|---------|
| `codex-rs/tui/src/chatwidget.rs` | +304 lines | FORK-SPECIFIC guards |
| `codex-rs/tui/src/chatwidget/exec_tools.rs` | +33 lines | FORK-SPECIFIC guard |
| `codex-rs/tui/src/slash_command.rs` | +36 lines | FORK-SPECIFIC guards |
| `codex-rs/tui/src/spec_prompts.rs` | ~200 lines | New file (no upstream) |
| `codex-rs/tui/src/app.rs` | +10 lines | Minor additions |

### FORK-SPECIFIC Guard Pattern

```rust
// === FORK-SPECIFIC: spec-kit automation START ===
// ... fork-only code ...
// === FORK-SPECIFIC: spec-kit automation END ===
```

**Purpose**: Clearly marks code that differs from upstream, making rebases easier.

### Non-TUI Changes (Safe)

| Path | Risk |
|------|------|
| `scripts/spec_ops_004/` | Zero (entirely fork-only) |
| `docs/spec-kit/` | Zero (entirely fork-only) |
| `.github/codex/home/config.toml` | Zero (user config) |

## 8. Rebase Strategy

### Before Rebase

1. **Tag current state**:
   ```bash
   git tag spec-kit-pre-rebase-$(date +%Y%m%d)
   git push origin spec-kit-pre-rebase-$(date +%Y%m%d)
   ```

2. **Update spec-kit-base branch**:
   ```bash
   git checkout spec-kit-base
   git merge feat/spec-auto-telemetry
   git push origin spec-kit-base
   ```

3. **Review upstream changes**:
   ```bash
   git fetch upstream
   git log main..upstream/main --oneline
   git diff main..upstream/main -- codex-rs/tui/src/chatwidget.rs
   ```

4. **Identify conflict zones**:
   - If upstream touched chatwidget.rs around lines 16000-18000: **HIGH RISK**
   - If upstream modified exec_tools.rs: **MEDIUM RISK**
   - If upstream added new slash commands: **LOW RISK**

### During Rebase

```bash
git rebase upstream/main
```

**Conflict Resolution**:

**Scenario 1: chatwidget.rs conflict**
```bash
# Accept upstream for non-FORK-SPECIFIC code
git show :3:codex-rs/tui/src/chatwidget.rs > theirs.rs

# Extract fork sections
grep -A 9999 "=== FORK-SPECIFIC" codex-rs/tui/src/chatwidget.rs > fork-sections.txt

# Merge: copy theirs, re-inject fork sections at correct locations
cp theirs.rs codex-rs/tui/src/chatwidget.rs
# Re-inject:
# 1. spec_auto_state field (around line 548)
# 2. State machine structures (around line 16612)
# 3. Pipeline methods (around line 17138)

git add codex-rs/tui/src/chatwidget.rs
```

**Scenario 2: New upstream features overlap**
- Read upstream change intent
- Adapt FORK-SPECIFIC code to work with new structure
- Test compilation after each resolution

### Rebase Frequency

| Cadence | Action |
|---------|--------|
| Monthly | Check upstream for major changes, test rebase on throwaway branch |
| Before major changes | Sync with upstream first to reduce merge complexity |

## 9. Validation After Rebase

### Validation Checklist

1. **Compile check**:
   ```bash
   ./scripts/test_fork_deviations.sh
   ```

2. **Manual test**:
   ```bash
   cd /home/thetu/code
   ./codex-rs/target/dev-fast/code

   # In TUI:
   /new-spec Test rebase functionality
   /spec-auto SPEC-KIT-030-test-rebase-functionality
   ```

3. **Verify telemetry**:
   ```bash
   ls docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-030*/
   # Should see: baseline*.md, spec-plan*.json, spec-plan*.log
   ```

4. **Verify consensus**:
   ```bash
   ls docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-030*/
   # Should see: synthesis.json, telemetry.jsonl, per-agent JSON
   ```

**If any fail**: Rollback, analyze, re-rebase with fixes.

---

# Appendices

## A. Rebase Validation Script

```bash
#!/usr/bin/env bash
# scripts/test_fork_deviations.sh
# Validate spec-kit TUI code after upstream rebases

set -euo pipefail

echo "=== Fork Deviation Validation ==="

# 1. Check FORK-SPECIFIC guards present
echo "Checking fork guards..."
if ! grep -q "FORK-SPECIFIC: spec-kit automation" codex-rs/tui/src/chatwidget.rs; then
    echo "ERROR: Fork guards missing in chatwidget.rs"
    exit 1
fi

# 2. Verify spec-auto state machine exists
echo "Checking state machine..."
if ! grep -q "struct SpecAutoState" codex-rs/tui/src/chatwidget.rs; then
    echo "ERROR: SpecAutoState missing"
    exit 1
fi

# 3. Verify guardrail completion handler
echo "Checking guardrail handler..."
if ! grep -q "auto_submit_spec_stage_prompt" codex-rs/tui/src/chatwidget/exec_tools.rs; then
    echo "ERROR: Guardrail completion handler missing"
    exit 1
fi

# 4. Compile check
echo "Compiling TUI..."
cd codex-rs
if ! cargo build --profile dev-fast -p codex-tui 2>&1 | tee /tmp/tui-build.log; then
    echo "ERROR: TUI compilation failed"
    exit 1
fi

# 5. Run spec-auto tests
echo "Running spec-auto tests..."
if ! cargo test -p codex-tui spec_auto 2>&1 | tee /tmp/tui-test.log; then
    echo "ERROR: Tests failed"
    exit 1
fi

echo "All fork deviation validations passed"
```

## B. Change History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-21 | Consolidated from root CONTRIBUTING.md + TUI.md fork management sections |

---

_Last Updated: 2026-01-21_
