# Session Brief — fix/precommit-clippy-gate

## Goal

Land the staged-only clippy gate in pre-commit hook, then fix all clippy warnings so the gate is usable.

## Scope / Constraints

- Clippy gate runs only when staged Rust/Cargo files exist
- Fix clippy errors in codex-tui (8 issues: expect/unwrap, format args, eprintln)
- Remove tui2-legacy-tests feature (475 compilation errors, per ADR-002 decision)
- Do not change architecture; workflow + lint hygiene only

## Plan

1. Land pre-commit clippy gate (already staged)
2. Fix 8 clippy errors in codex-tui test files
3. Remove tui2-legacy-tests feature + ~20K lines dead test code
4. Verify full workspace clippy passes

## Open Questions

None

## Verification

```bash
cd codex-rs && cargo clippy --workspace --all-targets --all-features -- -D warnings
bash .githooks/pre-commit
```

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `pre-commit clippy staged-only gate`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260204T194200Z/artifact/briefs/fix__precommit-clippy-gate/20260204T194200Z.md`
- Capsule checkpoint: `brief-fix__precommit-clippy-gate-20260204T194200Z`

Pre-commit hook clippy gate implementation session. Gate pattern: detect staged .rs/Cargo files -> run clippy with -D warnings -> block on failure.

<!-- END: SPECKIT_BRIEF_REFRESH -->
