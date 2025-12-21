# SPEC-CI-001: Smoke Test Packet

Deterministic test fixture for spec-kit review validation.
Used by CLI integration tests and contract verification.

## Cases

| Case | SPEC ID | Exit Code | Description |
|------|---------|-----------|-------------|
| Clean | SPEC-CI-001-clean | 0 | Valid consensus, no conflicts |
| Conflict | SPEC-CI-001-conflict | 2 | Blocking conflict detected |
| Malformed | SPEC-CI-001-malformed | 0 (default) / 3 (--strict-schema) | Invalid JSON |

## Exit Code Contract

- 0: Passed (or Skipped in default mode)
- 1: PassedWithWarnings (when --strict-warnings)
- 2: Failed (Escalate) or missing artifacts (when --strict-artifacts)
- 3: Infrastructure error (when --strict-schema and parse fails)

## Directory Structure

```
SPEC-CI-001/
├── docs/
│   ├── SPEC-CI-001-clean/plan.md
│   ├── SPEC-CI-001-conflict/plan.md
│   └── SPEC-CI-001-malformed/plan.md
└── docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/
    ├── SPEC-CI-001-clean/spec-plan_claude_20251220T000000.json
    ├── SPEC-CI-001-conflict/spec-plan_architect_20251220T000000.json
    └── SPEC-CI-001-malformed/spec-plan_broken_20251220T000000.json
```

## Contract Lock

This fixture is a contract lock. Changes require:
1. Update test expectations in cli/tests/speckit.rs
2. Update review.rs fixture tests
3. Bump fixture version in this README

**Fixture Version**: 1.0.0
**Created**: 2025-12-21
**P1-B Reference**: SPEC-KIT-921
