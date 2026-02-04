# Session Brief — fix/speckit-brief-init-check

## Goal

Add native CLI commands to initialize and validate per-branch session briefs:

- `code speckit brief init`
- `code speckit brief check`

## Scope / Constraints

- Brief path: `docs/briefs/<branch>.md` where `/` → `__` and other non `[A-Za-z0-9._-]` → `-` (must match `.githooks/pre-commit`).
- No MCP for local-memory.
- Keep `doc_lint` strict (warnings are errors).

## Plan

1. Implement `brief init` (template creation + --force)
2. Implement `brief check` (exists/non-empty + optional marker requirement)
3. Update docs and pre-commit UX

## Open Questions

- None

## Verification

```bash
cd codex-rs && cargo fmt --all -- --check
cd codex-rs && cargo clippy -p codex-cli --all-targets --all-features -- -D warnings
cd codex-rs && cargo test -p codex-cli
python3 scripts/doc_lint.py
bash .githooks/pre-commit
```
