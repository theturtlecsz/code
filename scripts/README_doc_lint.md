# Doc Lint (docs contract gate)

This directory exists to prevent repeated “intent drift” during implementation.

## Run locally

```bash
python3 scripts/doc_lint.py
```

## Wire into CI (example)

- Add a job that runs `python3 scripts/doc_lint.py`
- Fail PRs if lint fails

## What it checks (v1)

- `SPEC.md` has the **Docs Contract** and key invariants
- The **Active Program** doc exists
- Active specs 971–980 exist
- SPEC-KIT-973 uses merge terms `curated|full` (not `squash|ff`)
- SPEC-KIT-975 includes a **Replay Truth Table** clarifying determinism

## Why this exists

If a doc contract is not enforced mechanically, implementors will infer behavior from
older docs or prior intuition and we’ll keep iterating on “what we meant”.

This is the cheapest guardrail we can ship immediately.
