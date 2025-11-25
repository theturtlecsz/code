# Cargo Cleanup & Disk Space Management

Rust build artifacts accumulate rapidly and can consume hundreds of gigabytes. This guide covers proactive cleanup strategies.

## Quick Reference

```bash
# Check current usage
du -sh ~/code/codex-rs/target

# Full cleanup (removes ALL build artifacts)
cd ~/code/codex-rs && cargo clean

# Force clean before build
CLEAN=1 ~/code/build-fast.sh
```

## Automatic Cleanup

The build script automatically cleans when switching profiles or on explicit request:

```bash
# Force clean before build
CLEAN=1 ~/code/build-fast.sh

# Clean specific profile
PROFILE=release CLEAN=1 ~/code/build-fast.sh
```

## Manual Cleanup Commands

```bash
# Full cleanup (removes ALL build artifacts - use when disk space critical)
cd ~/code/codex-rs && cargo clean

# Profile-specific cleanup (preserves other profiles)
cd ~/code/codex-rs && cargo clean --profile dev-fast
cd ~/code/codex-rs && cargo clean --release

# Check target directory size
du -sh ~/code/codex-rs/target
```

## Cleanup Triggers

Run cleanup in these situations:

1. **After completing major SPECs** - Run `cargo clean` after `/speckit.unlock` succeeds
2. **Before long sessions** - Clean at session start if target/ > 20GB
3. **After branch switches** - Clean when switching between feature branches with dependency changes
4. **On build errors** - Try `cargo clean` if encountering mysterious build failures
5. **End of day** - Run `cargo clean` before signing off

## Monitoring Commands

```bash
# Check current disk usage
du -sh ~/code/codex-rs/target
df -h ~/code/codex-rs

# List largest directories in target/
du -h ~/code/codex-rs/target | sort -rh | head -20

# Check for old build artifacts (older than 7 days)
find ~/code/codex-rs/target -type f -mtime +7 -ls
```

## Expected Sizes

| State | Size | Action |
|-------|------|--------|
| Clean workspace | ~500MB | Normal |
| After dev build | ~5-10GB | Normal |
| After multiple profiles | ~15-30GB | Monitor |
| **>50GB** | WARNING | Cleanup recommended |
| **>100GB** | CRITICAL | Full `cargo clean` mandatory |

## Session Start Check

```bash
# Run at session start
if [ $(du -s ~/code/codex-rs/target 2>/dev/null | cut -f1) -gt 20000000 ]; then
  echo "Target directory exceeds 20GB, running cleanup..."
  cd ~/code/codex-rs && cargo clean
fi
```

## What NOT to Clean

- DO NOT delete `~/.cargo/` (shared dependency cache)
- DO NOT clean during active development (only between major tasks)
- DO NOT clean if builds are in progress

## Emergency Cleanup (disk full)

```bash
# Nuclear option - removes everything including dependency cache
cd ~/code/codex-rs && cargo clean
rm -rf ~/.cargo/registry/cache/*
rm -rf ~/.cargo/git/checkouts/*
```

---

*Extracted from CLAUDE.md for maintainability. See main documentation for project context.*
