# SPEC-KIT-977 — Model Policy v2 (Lifecycle + Enforcement)
**Date:** 2026-01-12 (Updated)
**Status:** IN PROGRESS (30%)
**Owner (role):** Platform+Security Eng

## Summary
Turn model policy into an executable system: authored in repo, validated in CI, snapshotted into capsules, enforced at routing and gates, monitored and auditable.

## Decision IDs implemented

**Implemented by this spec:** D12, D17, D36, D56, D57, D44, D100, D101, D102

**Referenced (must remain consistent):** D30, D59

**Explicitly out of scope:** D60

---

## Non-Negotiables

- **PolicySnapshot is the compiled artifact** - Stored in capsule and bound to events
- **Dual storage** - Filesystem (`.speckit/policies/`) AND capsule (`mv2://.../policy/<ID>`)
- **Events tagged with policy_id** - Every StageTransition/Checkpoint includes policy reference
- **Capture mode is policy-controlled** - `none | prompts_only | full_io`

---

## Goals
- Deliver the listed deliverables with tests and safe rollout.
- Make policy decisions auditable and reproducible.

## Non-Goals
- Hosted multi-tenant policy service.
- Dynamic policy changes mid-run (policy is captured at run start).

---

## Source-of-Truth Files

### Policy Inputs (Repo-Authored)

| File | Purpose | Format |
|------|---------|--------|
| `docs/MODEL-POLICY.md` | Human-readable rationale ("why") | Markdown |
| `model_policy.toml` | Machine-authoritative config ("what") | TOML |

### Compilation Output

| File | Purpose | Location |
|------|---------|----------|
| `PolicySnapshot.json` | Compiled artifact | Filesystem + Capsule |

**Rule**: The snapshot is the only thing stored in capsules; the `.md`/`.toml` are provenance inputs.

---

## PolicySnapshot Schema v1.0

### Required Fields

```json
{
  "schema_version": "1.0",
  "policy_id": "uuid-v4",
  "hash": "sha256-of-canonical-json-without-hash",
  "created_at": "2026-01-12T10:30:00Z",
  "source_files": ["docs/MODEL-POLICY.md", "model_policy.toml"],
  "model_config": { ... },
  "weights": { ... },
  "prompts": { ... }
}
```

### Policy Content Blocks (v2 Governance)

#### routing
```toml
[routing.cloud]
architect = ["claude-sonnet-4-20250514", "gpt-4o"]
implementer = ["claude-sonnet-4-20250514", "gpt-4o"]
judge = ["claude-sonnet-4-20250514"]

[routing.reflex]
enabled = false
endpoint = "http://127.0.0.1:3009/v1"
model = "qwen2.5-coder-7b-instruct"
```

#### capture
```toml
[capture]
mode = "prompts_only"  # none | prompts_only | full_io
store_embeddings = true
```

#### budgets
```toml
[budgets.tokens]
plan = 8000
tasks = 4000
implement = 6000

[budgets.cost]
warn_threshold = 5.0
hard_limit = 25.0
```

#### gates
```toml
[gates.reflex_promotion]
p95_latency_ms = 2000
success_parity_percent = 85

[gates.local_memory_sunset]
retrieval_p95_parity = true
stability_days = 30
```

---

## Capture Triggers

### When Snapshots Are Created

| Trigger | Action |
|---------|--------|
| **Run start** | Capture -> write FS -> store capsule -> emit `PolicySnapshotRef` |
| **Stage boundary** | If policy hash changed, capture new snapshot before checkpoint |
| **Manual commit** | Same rule (if policy changed, snapshot first) |

### Event Binding

Every StageTransition / Checkpoint event MUST include:
- `policy_id` and/or `policy_uri`
- `policy_hash`

---

## Storage Contract

### Filesystem
- Path: `.speckit/policies/snapshot-<policy_id>.json`
- Retention: Indefinite (prunable via cleanup command)

### Capsule
- URI: `mv2://<workspace>/policy/<policy_id>`
- Content: Identical JSON as filesystem

### Parity Requirement
- Snapshot written to FS and stored in capsule bytes MUST be identical
- Hash verification on load

---

## CLI Commands

### TUI
| Command | Description |
|---------|-------------|
| `/speckit.policy list` | List policy snapshots |
| `/speckit.policy show <id>` | Show policy details |
| `/speckit.policy current` | Show current active policy |

### Headless
| Command | Description |
|---------|-------------|
| `code speckit policy list [--json]` | List snapshots |
| `code speckit policy show <id> [--json]` | Show details |
| `code speckit policy validate` | Validate model_policy.toml |

---

## Deliverables

- [x] PolicySnapshot struct (`stage0/src/policy.rs`)
- [x] Filesystem storage (`PolicyStore`)
- [x] Hash computation and verification
- [x] `capture_policy_snapshot()` function
- [ ] Wire capture at run start
- [ ] Wire capture at stage boundaries
- [ ] Capsule storage integration
- [ ] Event binding (policy_id in StageTransition)
- [ ] CLI commands
- [ ] CI validation (`policy_lint.py` or extend `doc_lint.py`)

---

## Acceptance Criteria (Testable)

### 977-A1: Deterministic Hash Test
- Capturing twice with identical inputs produces identical `hash`
- Different source_files produce different hash

### 977-A2: Dual Storage Parity
- Snapshot written to FS and stored in capsule bytes are identical
- Load from either location produces same PolicySnapshot

### 977-A3: Event Binding
- After run start, capsule event log contains `PolicySnapshotRef`
- Event's policy_id matches latest snapshot ID/hash

### 977-A4: Run Traceability
- Given a run_id, can retrieve the PolicySnapshot that governed it
- StageTransition events include policy reference

### 977-A5: CI Validation
- `doc_lint.py` fails if `docs/MODEL-POLICY.md` or `model_policy.toml` missing
- `policy_lint.py` validates TOML schema

---

## CI / Doc Contract Enforcement

### Required Checks

1. `scripts/doc_lint.py` asserts `docs/MODEL-POLICY.md` and `model_policy.toml` exist
2. TOML schema validation (required sections present)
3. Lint ensures every `PolicySnapshotRef` event points to valid `mv2://.../policy/...` object

### Pre-commit Hook

```bash
# In .githooks/pre-commit
python3 scripts/doc_lint.py || exit 1
```

---

## Dependencies
- SPEC-KIT-971: Capsule foundation (for storage)
- Decision Register: `docs/DECISION_REGISTER.md`

## Rollout / Rollback
- Roll out incrementally: FS storage first, then capsule storage
- Roll back by disabling capsule storage (keep FS snapshots)

## Risks & Mitigations
- **Policy drift** -> CI validation + hash verification
- **Storage failure** -> Dual storage provides redundancy
- **Schema evolution** -> Schema version in snapshot enables migration
