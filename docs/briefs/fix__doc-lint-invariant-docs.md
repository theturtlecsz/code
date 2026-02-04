# Session Brief — fix/doc-lint-invariant-docs

## Goal

Document the Stage0↔Memvid boundary with rationale in canonical docs, fix doc_lint.py to have consistent warning behavior (no "only warns in --verbose"), and update DEV_BRIEF.md to reflect verification + session workflow expectations.

## Scope / Constraints

- Stage0 core has no Memvid dependency (locked invariant)
- Capsule (mv2://…) is SoR; filesystem is projection
- local-memory usage is CLI/REST only (no MCP)
- Doc lint warnings must be fatal regardless of --verbose flag

## Plan

1. Update DEV_BRIEF.md with session workflow note + expanded verification
2. Add Stage0↔Memvid boundary section to docs/STAGE0-REFERENCE.md
3. Add cross-reference row in docs/ARCHITECTURE.md Key Boundaries table
4. Fix doc_lint.py: remove verbose guard, expand search files to include codex-rs/
5. Run verification (doc_lint default + verbose, pre-commit)
6. Open PR

## Open Questions

None - requirements are specific.

## Verification

```bash
python3 scripts/doc_lint.py                                       # must exit 0
python3 scripts/doc_lint.py --verbose                             # must exit 0 (same behavior)
cargo clippy --workspace --all-targets --all-features -- -D warnings  # from codex-rs/
bash .githooks/pre-commit                                         # must pass
```

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)
- Query: `doc_lint invariants Stage0 memvid boundary`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260204T143731Z/artifact/briefs/fix__doc-lint-invariant-docs/20260204T143731Z.md`
- Capsule checkpoint: `brief-fix__doc-lint-invariant-docs-20260204T143731Z`

Relevant context from local-memory and session start hook:
- BUG-FIX GAP: SPEC-KIT-981/982 commit 175c68f33 adds SpecKitStageAgents config + prompt_vars builder
- Spec-Kit prompt system drift (Jan 2026): runtime uses TWO prompt sources
- PATTERN: Treat codex-rs/SPEC.md as canonical completion tracker

<!-- END: SPECKIT_BRIEF_REFRESH -->
