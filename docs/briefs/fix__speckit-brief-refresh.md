# Session Brief — fix/speckit-brief-refresh

## Goal

Add native CLI command `code speckit brief refresh` to generate/update this branch brief using codex-product (local-memory) + Ollama synthesis.

## Scope / Constraints

* local-memory access via `lm` CLI only (no MCP)
* Use local LLM (Ollama) for synthesis
* Do not touch frozen historical docs under `docs/SPEC-KIT-*`

## Plan

1. Add `code speckit brief refresh`
2. Query `lm search` in `codex-product` and filter (importance>=8, type tag present, no system:true)
3. Synthesize constraints with Ollama and write to `docs/briefs/<branch>.md`

## Open Questions

* None

## Verification

```bash
cargo check -p codex-cli
python3 scripts/doc_lint.py
bash .githooks/pre-commit
```
