# Convergence Documentation (Canonical Pointer)

Canonical convergence docs live in the `localmemory-policy` repo:

- `docs/convergence/CONVERGENCE_OVERVIEW.md`
- `docs/convergence/CONVERGENCE_MATRIX.yaml`

This repo (codex-rs workspace, `code` CLI) must implement the rows owned by `codex-rs` and conform to the policies in the canonical docs.

**Do not copy/fork the matrix here—link to it to avoid drift.**

## Repo-Local Documents

| File | Purpose |
|------|---------|
| `MEMO_codex-rs.md` | Update memo with required behaviors |
| `PROMPT_codex-rs.md` | Implementation prompt for Claude Code |

## Key Behaviors (Summary)

1. **Tier2 fail-closed**: Stage0 attempts NotebookLM by default; skips gracefully if unavailable
2. **`code doctor`**: Single diagnostic surface for local-memory, NotebookLM, and notebook mapping
3. **System pointer memories**: Stage0 artifacts stored with `domain:spec-tracker`, `system:true` tag
4. **Exclusion compliance**: Tier1 retrieval excludes `system:true` memories by default

## External References

- Convergence Matrix: `~/infra/localmemory-policy/docs/convergence/CONVERGENCE_MATRIX.yaml`
- local-memory API: `http://localhost:3002/api/v1`
- NotebookLM service: `http://127.0.0.1:3456`
