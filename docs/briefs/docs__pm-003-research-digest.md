# Session Brief — docs/pm-003-research-digest

**Date**: 2026-02-08

## Goal

Convert Gemini/ChatGPT deep‑research outputs for `SPEC‑PM‑003` into a durable, repo‑local research digest that:

- clearly separates **locked constraints** vs **new proposals**,
- flags any **unverified / potentially hallucinated** claims,
- is easy to review by the planning architect.

## Scope

- Add `docs/SPEC-PM-003-bot-system/research-digest.md` and link it from `docs/SPEC-PM-003-bot-system/spec.md`.
- Add the digest as a source in the NotebookLM dev notebook (`spec-kit-dev`).

## Non-Goals

- Changing the locked decisions or interface contract (`SPEC‑PM‑002`).
- Implementing the PM bot runner/service.

## Validation

- `python3 scripts/doc_lint.py`

<!-- BEGIN: SPECKIT_BRIEF_REFRESH -->
## Product Knowledge (auto)

- Query: `SPEC-PM-003 research digest Gemini ChatGPT`
- Domain: `codex-product`
- Capsule URI: `mv2://default/WORKFLOW/brief-20260208T193250Z/artifact/briefs/docs__pm-003-research-digest/20260208T193250Z.md`
- Capsule checkpoint: `brief-docs__pm-003-research-digest-20260208T193250Z`

No high-signal product knowledge matched. Try a more specific `--query` and/or raise `--limit`.

<!-- END: SPECKIT_BRIEF_REFRESH -->
