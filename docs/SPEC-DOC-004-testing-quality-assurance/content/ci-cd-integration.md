# CI/CD Integration

Comprehensive guide to CI/CD integration and automated testing.

---

## Overview

**CI/CD Testing Strategy**: Automated testing at every stage (pre-commit, pre-push, CI, release)

**Goals**:
- Fast feedback (<5s pre-commit, <2min pre-push)
- Comprehensive coverage (all tests in CI)
- Prevent regressions
- Maintain code quality

**Current Status**:
- Pre-commit hooks: 100% adoption
- CI pipeline: GitHub Actions
- Test execution: ~10-15 minutes
- Pass rate: 100%

---

## Testing Stages

###  1. Pre-Commit (Local) - <5s

**Purpose**: Fast policy checks before commit

**Location**: `.githooks/pre-commit`

**What it runs**:
```bash
# Only runs if spec_kit modules modified
# Check 1: Storage policy
bash scripts/validate_storage_policy.sh

# Check 2: Tag schema
bash scripts/validate_tag_schema.sh
```

**Execution Time**: <5s

**Bypass** (emergencies only):
```bash
git commit --no-verify
```

---

### 2. Pre-Push (Local) - ~2-5 min

**Purpose**: Compile and lint checks before push

**Triggered**: Before `git push`

**What it runs**:
```bash
# Format check
cargo fmt --all -- --check

# Linting
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build (all features)
cargo build --workspace --all-features

# Optional: Targeted test compilation
cargo test --workspace --no-run
```

**Execution Time**: ~2-5 minutes

**Bypass** (emergencies only):
```bash
PREPUSH_FAST=0 git push
```

---

### 3. CI/CD (GitHub Actions) - ~10-15 min

**Purpose**: Complete testing and release

**Triggered**: Push to `main`, pull requests

**Location**: `.github/workflows/release.yml`

**Jobs**:
1. **Preflight Tests** (Linux fast E2E)
2. **Determine Version** (semantic versioning)
3. **Build** (Linux, macOS, Windows)
4. **Test** (all tests, all platforms)
5. **Release** (npm publish)

---

## GitHub Actions Workflows

### Preflight Tests Job

**Purpose**: Fast integration tests before full build matrix

**Platform**: Ubuntu 24.04

**Steps**:

```yaml
jobs:
  preflight-tests:
    name: Preflight Tests (Linux fast E2E)
    runs-on: ubuntu-24.04
    steps:
      - name: Checkout code
        uses: actions/checkout@v4

      - name: Install Rust (1.90)
        run: |
          rustup set profile minimal
          rustup toolchain install 1.90.0 --profile minimal
          rustup default 1.90.0

      - name: Setup Rust Cache
        uses: Swatinem/rust-cache@v2
        with:
          prefix-key: v5-rust
          shared-key: codex-preflight-1.90
          workspaces: codex-rs -> target
          cache-targets: true
          cache-on-failure: true

      - name: Build (fast profile)
        run: ./build-fast.sh

      - name: Curated tests + CLI smokes
        run: bash scripts/ci-tests.sh
```

**What it tests**:
```bash
# scripts/ci-tests.sh

# Curated integration tests
cargo test -p codex-login --test all -q
cargo test -p codex-chatgpt --test all -q
cargo test -p codex-apply-patch --test all -q
cargo test -p codex-execpolicy --tests -q
cargo test -p mcp-types --tests -q

# CLI smoke tests
./codex-rs/target/dev-fast/code --version
./codex-rs/target/dev-fast/code completion bash
./codex-rs/target/dev-fast/code doctor
```

**Execution Time**: ~3-5 minutes

**Benefits**:
- ✅ Fast feedback (before full matrix)
- ✅ Catches common errors early
- ✅ Tests critical integration points
- ✅ Validates CLI functionality

---

### Build Matrix Job

**Purpose**: Build and test on all platforms

**Platforms**:
- Linux (Ubuntu 24.04, x64 + arm64)
- macOS (latest, x64 + arm64)
- Windows (latest, x64)

**Rust Versions**:
- Stable (1.90)
- Beta (optional)

**Steps**:
```yaml
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-24.04, macos-latest, windows-latest]
        rust: [1.90.0]
    runs-on: ${{ matrix.os }}
    steps:
      - name: Checkout
        uses: actions/checkout@v4

      - name: Install Rust
        run: rustup toolchain install ${{ matrix.rust }}

      - name: Build
        run: cargo build --workspace --all-features

      - name: Test
        run: cargo test --workspace --all-features
```

**Execution Time**: ~8-12 minutes per platform

---

### Release Job

**Purpose**: Publish to npm after successful tests

**Triggers**:
- Push to `main`
- All tests pass

**Steps**:
1. Determine version (semantic versioning)
2. Build binaries (all platforms)
3. Publish to npm (`@just-every/code`)

**Packages Published**:
- `@just-every/code` (main package)
- `@just-every/code-darwin-arm64`
- `@just-every/code-darwin-x64`
- `@just-every/code-linux-x64-musl`
- `@just-every/code-linux-arm64-musl`
- `@just-every/code-win32-x64`

---

## Pre-Commit Hook

### Installation

**One-time setup**:
```bash
bash scripts/setup-hooks.sh
```

**Verifies**:
```bash
git config core.hooksPath
# Should output: .githooks
```

---

### What It Checks

**File**: `.githooks/pre-commit`

```bash
#!/bin/bash
# Pre-commit hook for policy compliance

# Only run if spec_kit modules modified
SPEC_KIT_CHANGES=$(git diff --cached --name-only | grep "spec_kit" || true)

if [ -z "$SPEC_KIT_CHANGES" ]; then
    # No spec_kit changes, skip policy checks
    exit 0
fi

echo "🔍 Running policy compliance checks (spec_kit modified)..."

# Check 1: Storage policy
if ! bash scripts/validate_storage_policy.sh; then
    echo "❌ Storage policy violation detected"
    exit 1
fi

# Check 2: Tag schema
if ! bash scripts/validate_tag_schema.sh; then
    echo "❌ Tag schema violation detected"
    exit 1
fi

echo "✅ Policy compliance checks passed"
exit 0
```

**Checks**:
1. **Storage policy**: Ensures local-memory usage compliant (MEMORY-POLICY.md)
2. **Tag schema**: Validates tag namespacing and naming

**Performance**: <5s (only runs for spec_kit changes)

---

### Bypass Pre-Commit (Emergencies Only)

```bash
# Skip hook (use sparingly)
git commit --no-verify -m "Emergency fix"
```

**When to bypass**:
- Critical production hotfix
- Hook infrastructure broken
- Reviewing/reverting broken commits

**When NOT to bypass**:
- Avoiding policy violations (fix the code instead)
- Convenience (hooks are fast)
- Regular workflow

---

## CI Test Script

### Location

**File**: `scripts/ci-tests.sh`

**Purpose**: Fast integration tests for CI

---

### What It Tests

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "[ci-tests] Running curated integration tests..."
pushd codex-rs >/dev/null

# Login integration tests
cargo test -p codex-login --test all -q

# ChatGPT integration tests
cargo test -p codex-chatgpt --test all -q

# Apply patch integration tests
cargo test -p codex-apply-patch --test all -q

# Execution policy tests
cargo test -p codex-execpolicy --tests -q

# MCP types tests
cargo test -p mcp-types --tests -q

popd >/dev/null

echo "[ci-tests] CLI smokes with host binary..."
BIN=./codex-rs/target/dev-fast/code

# Smoke tests
"${BIN}" --version >/dev/null
"${BIN}" completion bash >/dev/null
"${BIN}" doctor >/dev/null || true

echo "[ci-tests] Done."
```

---

### Why Curated Tests?

**Full test suite**: 604 tests, ~15 minutes

**Curated subset**: ~150 tests, ~3-5 minutes

**Selection Criteria**:
- ✅ Integration tests (cross-module)
- ✅ E2E tests (complete workflows)
- ✅ Critical paths (login, apply, MCP)
- ❌ Unit tests (fast, covered by local dev)
- ❌ Property tests (slow, covered by weekly runs)

**Benefits**:
- ✅ Fast feedback (3-5 min vs 15 min)
- ✅ High signal (integration tests find real bugs)
- ✅ CI efficiency (parallel preflight + full tests)

---

## Local Testing Before Push

### Recommended Workflow

**Step 1: Run affected tests** (iterative development):
```bash
cd codex-rs

# Test specific module you changed
cargo test -p codex-tui --lib

# Test specific file
cargo test -p codex-tui spec_kit::clarify_native
```

**Step 2: Run full test suite** (before committing):
```bash
cd codex-rs
cargo test --workspace --all-features
```

**Step 3: Check format and lint** (before committing):
```bash
cd codex-rs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Step 4: Commit** (pre-commit hook runs automatically):
```bash
git add .
git commit -m "feat(tui): add clarify native checks"
# Hook runs: storage policy, tag schema (<5s)
```

**Step 5: Push** (pre-push hook runs automatically, optional):
```bash
git push
# Hook runs: fmt, clippy, build (~2-5min)
```

---

### Fast Iteration Loop

**For rapid development**:
```bash
# 1. Make changes
vim codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs

# 2. Test just this module (fast)
cd codex-rs
cargo test -p codex-tui clarify_native -- --nocapture

# 3. If tests pass, run clippy on this crate
cargo clippy -p codex-tui -- -D warnings

# 4. Commit (hook runs policy checks)
git add codex-rs/tui
git commit -m "fix(clarify): improve ambiguity detection"

# 5. Push later after multiple commits
git push
```

**Execution Time**:
- Module tests: ~5-10s
- Clippy: ~15-30s
- Commit: <5s (hook)
- **Total**: ~30-50s per iteration

---

## Code Coverage Integration

### Local Coverage Measurement

**Tool**: cargo-tarpaulin or cargo-llvm-cov

**Install**:
```bash
cargo install cargo-tarpaulin
# or
cargo install cargo-llvm-cov
```

**Usage**:
```bash
cd codex-rs

# Generate coverage report
cargo tarpaulin --workspace --all-features --out Html

# Open report
open target/tarpaulin/index.html
```

---

### CI Coverage (Future)

**GitHub Actions** (not yet implemented):
```yaml
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Run coverage
        run: cargo tarpaulin --workspace --all-features --out Xml

      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: ./cobertura.xml

      - name: Comment PR with coverage
        uses: codecov/codecov-action@v3
```

**Benefits** (when implemented):
- ✅ Track coverage trends
- ✅ Fail PR if coverage drops >5%
- ✅ Visualize coverage in PRs

---

## Best Practices

### DO

**✅ Run tests locally before pushing**:
```bash
# Always test before pushing
cargo test --workspace --all-features

# Push after tests pass
git push
```

---

**✅ Fix CI failures immediately**:
```
# CI failed? Fix it now, not later
git pull
cargo test --workspace
# Fix failures
git commit -m "fix(ci): resolve test failures"
git push
```

---

**✅ Keep CI green**:
- Main branch should always pass tests
- Revert breaking commits if fix takes >1 hour
- Document known flaky tests

---

**✅ Use caching effectively**:
```yaml
# GitHub Actions caching
- uses: Swatinem/rust-cache@v2
  with:
    prefix-key: v5-rust
    shared-key: codex-preflight-1.90
```

---

**✅ Run curated tests in CI** (fast feedback):
```bash
# Preflight: curated subset (3-5 min)
bash scripts/ci-tests.sh

# Full matrix: all tests (10-15 min)
cargo test --workspace --all-features
```

---

### DON'T

**❌ Skip pre-commit hooks routinely**:
```bash
# Bad: Habitual bypassing
git commit --no-verify  # ❌ Don't make this a habit
```

---

**❌ Push without testing**:
```bash
# Bad: Push untested code
git commit -m "quick fix"
git push  # ❌ No local testing
```

---

**❌ Ignore CI failures**:
```
# Bad: "CI is always red anyway"
# ❌ Fix CI or revert
```

---

**❌ Commit broken tests**:
```bash
# Bad: Disable failing tests instead of fixing
#[test]
#[ignore]  // ❌ Don't ignore, fix!
fn test_that_fails() { }
```

---

**❌ Let coverage drop**:
```
# Bad: Coverage drops from 45% to 30%
# ❌ Add tests, don't delete them
```

---

## Troubleshooting

### Pre-Commit Hook Not Running

**Symptom**: Commits succeed without running hook

**Fix**:
```bash
# Check git config
git config core.hooksPath
# Should output: .githooks

# If not set, run setup
bash scripts/setup-hooks.sh
```

---

### CI Timeout

**Symptom**: CI job times out after 60 minutes

**Causes**:
- Infinite loop in test
- Deadlock in concurrent test
- Slow property test (PROPTEST_CASES too high)

**Fix**:
```bash
# Find slow tests locally
cargo test --workspace -- --nocapture --test-threads=1

# Reduce property test cases
PROPTEST_CASES=100 cargo test --test property_based_tests
```

---

### Flaky Tests

**Symptom**: Test passes locally, fails in CI (or vice versa)

**Common Causes**:
- Race conditions (concurrent tests)
- Hardcoded paths (use TempDir)
- Network dependencies (use mocks)
- Time-dependent tests (use fixed timestamps)

**Fix**:
```bash
# Run test multiple times locally
for i in {1..100}; do
    cargo test test_flaky_name || break
done

# If it fails, debug with single thread
cargo test test_flaky_name -- --test-threads=1 --nocapture
```

---

### Build Cache Corruption

**Symptom**: Build fails in CI with cryptic errors, passes locally

**Fix** (GitHub Actions):
```yaml
# Clear cache by changing cache key
- uses: Swatinem/rust-cache@v2
  with:
    prefix-key: v6-rust  # Increment version
```

---

## Summary

**CI/CD Testing Stages**:

1. **Pre-Commit** (<5s): Policy checks (storage, tags)
2. **Pre-Push** (2-5min): Format, clippy, build
3. **Preflight Tests** (3-5min): Curated integration tests
4. **Full CI** (10-15min): All tests, all platforms
5. **Release** (auto): Publish on main after tests pass

**Tools**:
- ✅ GitHub Actions (CI/CD)
- ✅ Rust Cache (faster builds)
- ✅ Git Hooks (pre-commit, pre-push)
- ✅ cargo-tarpaulin (coverage)

**Best Practices**:
- ✅ Test locally before pushing
- ✅ Keep CI green (100% pass rate)
- ✅ Fast feedback (curated tests in preflight)
- ✅ Fix failures immediately
- ✅ Use caching (Rust Cache)

**Next Steps**:
- [Performance Testing](performance-testing.md) - Benchmarks and profiling
- [Testing Strategy](testing-strategy.md) - Overall testing approach
- [Test Infrastructure](test-infrastructure.md) - MockMcpManager, fixtures

---

**References**:
- GitHub Actions: `.github/workflows/release.yml`
- Pre-commit hook: `.githooks/pre-commit`
- CI test script: `scripts/ci-tests.sh`
- Setup hooks: `scripts/setup-hooks.sh`
