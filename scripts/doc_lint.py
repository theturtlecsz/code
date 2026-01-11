#!/usr/bin/env python3

"""
doc_lint.py — Minimal mechanical enforcement for Spec-Kit docs

This script intentionally checks only a few high-signal invariants that repeatedly caused
implementation drift.

Wire into CI as:
  python3 scripts/doc_lint.py

Exit code != 0 means "docs are out of contract".
"""
from __future__ import annotations

import re
import sys
from pathlib import Path


REQUIRED_FILES = [
    "SPEC.md",
    "docs/PROGRAM_2026Q1_ACTIVE.md",
    "docs/DECISION_REGISTER.md",
    "docs/GOLDEN_PATH.md",
]

ACTIVE_SPECS = [
    "docs/SPEC-KIT-971-memvid-capsule-foundation/spec.md",
    "docs/SPEC-KIT-972-hybrid-retrieval-eval/spec.md",
    "docs/SPEC-KIT-973-time-travel-ui/spec.md",
    "docs/SPEC-KIT-974-capsule-export-import-encryption/spec.md",
    "docs/SPEC-KIT-975-replayable-audits/spec.md",
    "docs/SPEC-KIT-976-logic-mesh-graph/spec.md",
    "docs/SPEC-KIT-977-model-policy-v2/spec.md",
    "docs/SPEC-KIT-978-local-reflex-sglang/spec.md",
    "docs/SPEC-KIT-979-local-memory-sunset/spec.md",
    "docs/SPEC-KIT-980-multimodal-ingestion/spec.md",
]

# "contract phrases" are intentionally strict so implementors cannot miss them.
SPEC_MD_MUST_CONTAIN = [
    "## Docs Contract (Non-Negotiable)",
    "Reflex is a routing mode",
    "Single-writer capsule model",
    "Memvid capsule is system-of-record",
    "Replay is offline-first",
    "URI stability",
]

def fail(msg: str) -> None:
    print(f"❌ {msg}", file=sys.stderr)

def ok(msg: str) -> None:
    print(f"✅ {msg}")

def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")

def main() -> int:
    repo_root = Path(__file__).resolve().parents[1]
    errors = 0

    # Required files
    for rel in REQUIRED_FILES:
        p = repo_root / rel
        if not p.exists():
            fail(f"Missing required doc: {rel}")
            errors += 1
        else:
            ok(f"Found: {rel}")

    # SPEC.md contract checks
    spec_md = repo_root / "SPEC.md"
    if spec_md.exists():
        t = read_text(spec_md)
        for phrase in SPEC_MD_MUST_CONTAIN:
            if phrase not in t:
                fail(f"SPEC.md missing contract phrase: {phrase!r}")
                errors += 1
        ok("SPEC.md contains Docs Contract + invariants (basic check)")

    # Active specs must exist
    for rel in ACTIVE_SPECS:
        p = repo_root / rel
        if not p.exists():
            fail(f"Missing active spec file: {rel}")
            errors += 1

    
    # Active specs must declare which Decision IDs they implement/refer to
    for rel in ACTIVE_SPECS:
        p = repo_root / rel
        if p.exists():
            t = read_text(p)
            if "## Decision IDs implemented" not in t:
                fail(f"Active spec missing Decision IDs section: {rel}")
                errors += 1
            else:
                ok(f"Decision IDs section present: {rel}")

# Merge mode terminology must be the new terms
    spec_973 = repo_root / "docs/SPEC-KIT-973-time-travel-ui/spec.md"
    if spec_973.exists():
        t = read_text(spec_973)
        if re.search(r"\b(squash|ff|fast-forward)\b", t):
            fail("SPEC-KIT-973 still mentions legacy merge terms (squash/ff/fast-forward). Must be curated|full only.")
            errors += 1
        if "curated" not in t or "full" not in t:
            fail("SPEC-KIT-973 must define merge modes curated|full.")
            errors += 1
        else:
            ok("SPEC-KIT-973 merge terminology looks correct")

    # Replay determinism truth statement
    spec_975 = repo_root / "docs/SPEC-KIT-975-replayable-audits/spec.md"
    if spec_975.exists():
        t = read_text(spec_975)
        if "Replay Truth Table" not in t:
            fail("SPEC-KIT-975 must include a Replay Truth Table to prevent 'exact replay' misunderstandings.")
            errors += 1
        if "model I/O" not in t and "capture_llm_io" not in t:
            fail("SPEC-KIT-975 must clarify model I/O replay depends on capture mode.")
            errors += 1
        else:
            ok("SPEC-KIT-975 replay determinism clarifications present (basic check)")

    if errors:
        fail(f"{errors} doc contract error(s). Fix before merging.")
        return 1

    ok("Doc contract checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
