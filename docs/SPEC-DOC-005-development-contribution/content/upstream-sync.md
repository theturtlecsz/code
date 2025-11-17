# Upstream Sync Process

Quarterly merge process for upstream changes.

---

## Overview

**Upstream**: https://github.com/just-every/code
**Frequency**: Quarterly (or as needed)
**Strategy**: Merge with manual conflict resolution

---

## Process

### 1. Add Upstream Remote

```bash
git remote add upstream https://github.com/just-every/code.git
git remote -v
# upstream  https://github.com/just-every/code.git (fetch)
```

---

### 2. Fetch Upstream

```bash
git fetch upstream
git fetch upstream --tags
```

---

### 3. Merge Upstream

```bash
# Create merge commit (no fast-forward)
git merge --no-ff --no-commit upstream/main

# Review conflicts
git status
```

---

### 4. Resolve Conflicts

**Isolation Strategy**: Fork-specific code in `tui/src/chatwidget/spec_kit/`

**Conflict Resolution**:
- Accept upstream changes for non-spec_kit files
- Keep fork changes for spec_kit files
- Manually merge if both modified same file

**Example**:
```bash
# spec_kit conflict - keep ours
git checkout --ours codex-rs/tui/src/chatwidget/spec_kit/handler.rs

# Upstream change - keep theirs
git checkout --theirs codex-rs/tui/src/chatwidget/widget.rs
```

---

### 5. Test After Merge

```bash
# Build
./build-fast.sh

# Test
bash scripts/ci-tests.sh

# Full test suite
cd codex-rs && cargo test --workspace
```

---

### 6. Commit and Push

```bash
git add .
git commit -m "chore: merge upstream/main (2025-11-17)"
git push
```

---

## Conflict Minimization

**98.2% Isolation Achieved**: Spec-kit code isolated in separate modules

**Low-Conflict Areas**:
- `tui/src/chatwidget/spec_kit/*` (fork-specific)
- `docs/SPEC-*` (fork-specific)
- `.githooks/*` (fork-specific)

**High-Conflict Areas** (merge carefully):
- `Cargo.toml` (dependencies)
- `tui/src/chatwidget/widget.rs` (TUI core)
- `core/src/*` (conversation logic)

---

## Summary

**Frequency**: Quarterly
**Process**: Fetch → Merge → Resolve → Test → Commit
**Strategy**: Keep spec_kit changes, accept upstream otherwise

**References**:
- Upstream sync docs: `docs/UPSTREAM-SYNC.md`
- Conflict resolution: `.git/MERGE_HEAD`

**Next**: [Adding Commands](adding-commands.md)
