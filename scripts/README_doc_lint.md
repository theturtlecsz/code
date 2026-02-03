# Doc Lint (docs contract gate)

> **⚠️ Canonical Location: `codex-rs/scripts/doc_lint.py`**

This directory contains a **forwarding stub** that redirects to the canonical
doc_lint.py in `codex-rs/scripts/`. This ensures:

1. One source of truth for documentation contract enforcement
2. model_policy.toml schema validation is always enforced
3. No drift between duplicate implementations

## Run locally

```bash
# From repo root - uses forwarding stub
python3 scripts/doc_lint.py

# Directly from codex-rs (canonical)
cd codex-rs && python3 scripts/doc_lint.py
```

## Wire into CI (example)

- Add a job that runs `python3 scripts/doc_lint.py`
- Fail PRs if lint fails
- The forwarding stub handles routing to canonical implementation

## What it checks (V6 Docs Contract)

- `SPEC.md` has the **Doc Precedence Order** and **Invariants** sections
- Required files exist (`DEV_BRIEF.md`, `docs/PROGRAM.md`, `docs/DECISIONS.md`, `docs/POLICY.md`, `docs/SPEC-KIT.md`, `model_policy.toml`, etc.)
- `model_policy.toml` has required sections (meta, system_of_record, routing, etc.)
- Active specs listed in `docs/PROGRAM.md` have Decision IDs sections/references
- Merge terminology uses `curated|full` (not `squash|ff/rebase`)
- **Replay Truth Table** exists somewhere in docs
- Key invariants are documented (Stage0 no Memvid dep, URI immutability, etc.)
- Warnings are treated as failures (exit code 1)

## Why this exists

If a doc contract is not enforced mechanically, implementors will infer behavior from
older docs or prior intuition and we'll keep iterating on "what we meant".

This is the cheapest guardrail we can ship immediately.

---

*See also: `codex-rs/SPEC.md` section "Policy Source Files"*
