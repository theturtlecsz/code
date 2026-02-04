# Session Brief — fix/precommit-rustfmt-gate

## Goal

Add a staged-only rustfmt gate to pre-commit hook, mirroring the existing clippy gate pattern.

## Scope / Constraints

- Rustfmt gate runs only when staged Rust/Cargo files exist (same trigger as clippy)
- Run from codex-rs/: `cargo fmt --all -- --check`
- Fail with actionable error showing fix command
- Doc-only commits remain fast

## Plan

1. Add rustfmt gate after clippy gate in `.githooks/pre-commit`
2. Reuse `$RUST_CHANGES` variable (already computed for clippy)
3. Verify doc-only and Rust change scenarios

## Open Questions

None

## Verification

```bash
# Doc-only should skip
echo "test" >> README.md && git add README.md
bash .githooks/pre-commit  # Should skip fmt check
git reset HEAD README.md && git checkout README.md

# Rust changes should trigger
bash .githooks/pre-commit  # Should run fmt on staged .rs files
```

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `pre-commit staged-only rustfmt gate`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260204T225800Z/artifact/briefs/fix__precommit-rustfmt-gate/20260204T225800Z.md`
- Capsule checkpoint: `brief-fix__precommit-rustfmt-gate-20260204T225800Z`

Pre-commit hook rustfmt gate implementation session. Gate pattern: detect staged .rs/Cargo files -> run cargo fmt --check -> block on failure with actionable fix command.

<!-- END: SPECKIT_BRIEF_REFRESH -->
