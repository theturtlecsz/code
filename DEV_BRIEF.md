# DEV\_BRIEF.md

> **Tier-1 Truth Anchor** — Required for every session. Update before starting work.

**Last Updated**: 2026-02-08

## Current Focus

Docs: digest external deep‑research outputs into a durable `SPEC‑PM‑003` research note (for planning/roadmap review), and keep the Spec‑Kit dev NotebookLM notebook sources current.

## Session Workflow

* `main` branch stays stable; `Current Focus: Idle` between sessions
* Per-PR context goes in `docs/briefs/<branch>.md` (branch name with `/` replaced by `__`)
* Branch briefs must be refreshed/snapshotted before commit (enforced by pre-commit)

## Scope / Constraints

* Local-memory: CLI-only (no MCP) — see [MEMORY-POLICY.md](MEMORY-POLICY.md)
* Historical docs under `docs/SPEC-KIT-*` are frozen
* NotebookLM dev notebook: keep `spec-kit-dev` sources current for Spec‑Kit development docs (roadmap item in PM‑003)

## Open Questions

<!-- Unresolved decisions or clarifications needed -->

## Verification

All must pass (local-only is sufficient):

```bash
python3 scripts/doc_lint.py                                            # warnings are errors
cargo clippy --workspace --all-targets --all-features -- -D warnings   # from codex-rs/
bash .githooks/pre-commit                                              # full validation
```
