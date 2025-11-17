# Pre-Commit Hooks Guide

Setup, debugging, and bypass procedures for git hooks.

---

## Setup

### Install Hooks

```bash
bash scripts/setup-hooks.sh
```

**Verifies**:
```bash
git config core.hooksPath
# Output: .githooks
```

---

## Hook: Pre-Commit

**Location**: `.githooks/pre-commit`

**Runs**: Policy compliance checks (< 5s)

**Checks**:
1. Storage policy (local-memory usage)
2. Tag schema (namespacing)

**Trigger**: Only runs if spec_kit files modified

---

## Hook: Pre-Push

**Runs**: Format, lint, build checks (~2-5min)

**Checks**:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo build --workspace --all-features`

---

## Bypass Hooks (Emergency Only)

### Skip Pre-Commit

```bash
git commit --no-verify -m "Emergency hotfix"
```

### Skip Pre-Push

```bash
PREPUSH_FAST=0 git push
```

**Use sparingly**: Only for emergencies

---

## Debugging Hooks

### Manual Run

```bash
# Pre-commit
bash .githooks/pre-commit

# Specific check
bash scripts/validate_storage_policy.sh
```

### Verbose Output

```bash
# Enable debug
set -x
bash .githooks/pre-commit
```

---

## Common Issues

**Issue**: Hook doesn't run

**Solution**:
```bash
git config core.hooksPath
# If not .githooks, re-run setup
bash scripts/setup-hooks.sh
```

**Issue**: Hook fails on unrelated files

**Solution**: Hooks only run for spec_kit changes. Check modified files:
```bash
git diff --cached --name-only | grep spec_kit
```

---

## Summary

**Setup**: `bash scripts/setup-hooks.sh`
**Bypass**: `git commit --no-verify` (emergencies only)
**Debug**: Run hooks manually

**Next**: [Upstream Sync](upstream-sync.md)
