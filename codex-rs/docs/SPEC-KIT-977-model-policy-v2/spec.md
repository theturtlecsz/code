# SPEC-KIT-977 — Model Policy v2 (Lifecycle + Enforcement)
**Date:** 2026-01-16 (Updated)
**Status:** IN PROGRESS (85%)
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

| File Pattern | Purpose | Location |
|--------------|---------|----------|
| `.speckit/policies/snapshot-<policy_id>.json` | Filesystem snapshot | Per-policy JSON files |
| `mv2://<workspace>/policy/<policy_id>` | Capsule object | Stored in MV2 capsule |

**Rule**: Multiple snapshots may exist (one per run/capture event). The `.md`/`.toml` are provenance inputs; only compiled snapshots are stored in capsules.

---

## PolicySnapshot Schema v1.0

Defined in `stage0/src/policy.rs` as `PolicySnapshot` struct.

### Required Fields

```json
{
  "schema_version": "1.0",
  "policy_id": "uuid-v4",
  "hash": "sha256-of-canonical-json-without-hash",
  "created_at": "2026-01-16T10:30:00Z",
  "source_files": ["docs/MODEL-POLICY.md", "model_policy.toml"],
  "model_config": {
    "max_tokens": 8000,
    "top_k": 10,
    "pre_filter_limit": 100,
    "diversity_lambda": 0.5,
    "iqo_llm_enabled": true,
    "hybrid_enabled": true,
    "vector_weight": 0.7,
    "tier2_enabled": false,
    "tier2_cache_ttl_hours": 24
  },
  "weights": {
    "recency": 0.25,
    "importance": 0.35,
    "type_bonus": 0.2,
    "access_frequency": 0.2
  },
  "prompts": {},
  "governance": { ... }
}
```

### Governance Block (from model_policy.toml)

The `governance` field contains the full parsed `GovernancePolicy` from `model_policy.toml`:

```json
{
  "governance": {
    "meta": {
      "schema_version": "1.0.0",
      "last_updated": "2026-01-16"
    },
    "system_of_record": {
      "source": "memvid",
      "fallback": "local-memory"
    },
    "routing": { ... },
    "capture": {
      "mode": "prompts_only",
      "store_embeddings": true
    },
    "budgets": { ... },
    "scoring": { ... },
    "gates": { ... },
    "security": { ... }
  }
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

### When Snapshots Are Created (Implemented)

| Trigger | Location | Action |
|---------|----------|--------|
| **Run start** | `pipeline_coordinator.rs:203` | `capture_and_store_policy()` -> write FS -> store capsule -> emit `PolicySnapshotRef` |
| **Stage boundary** | `git_integration.rs:291` | `check_and_recapture_if_changed()` -> if policy hash differs, capture new snapshot before checkpoint |
| **Manual commit** | Same rule | If policy changed since last snapshot, recapture first |

### Implementation Details

**Run Start** (`pipeline_coordinator.rs`):
```rust
match policy_capture::capture_and_store_policy(
    &handle,
    &stage0_cfg,
    &spec_id,
    run_id,
) { ... }
```

**Stage Boundary** (`git_integration.rs`):
```rust
match policy_capture::check_and_recapture_if_changed(
    &handle, &stage0_config, spec_id, run_id
) {
    Ok(Some(new_policy)) => {
        // Policy drift detected, new snapshot captured
    }
    ...
}
```

### Event Binding (Implemented)

Every StageTransition / Checkpoint event MUST include:
- `policy_id` and/or `policy_uri`
- `policy_hash`

**Implementation**: `PolicySnapshotRef` event emitted via `handle.emit_policy_snapshot_ref()` after capture.

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

### Core Infrastructure (DONE)
- [x] PolicySnapshot struct (`stage0/src/policy.rs`)
- [x] GovernancePolicy struct for model_policy.toml parsing
- [x] Filesystem storage (`PolicyStore` - `.speckit/policies/`)
- [x] Hash computation and verification (SHA256)
- [x] `capture_policy_snapshot()` function
- [x] `capture_with_governance()` for full policy capture

### Capture Integration (DONE)
- [x] Wire capture at run start (`pipeline_coordinator.rs:203`)
- [x] Wire capture at stage boundaries with drift detection (`git_integration.rs:291`)
- [x] Capsule storage integration (`policy_capture::capture_and_store_policy()`)
- [x] Event binding - `PolicySnapshotRef` emitted after capture

### CI Validation (DONE)
- [x] `doc_lint.py` checks `docs/MODEL-POLICY.md` exists
- [x] `doc_lint.py` checks `model_policy.toml` exists
- [x] `doc_lint.py` validates required TOML sections (`[meta]`, `[system_of_record]`, etc.)

### CLI Commands (NOT IMPLEMENTED)
- [ ] `code speckit policy list [--json]` - List policy snapshots
- [ ] `code speckit policy show <id> [--json]` - Show policy details
- [ ] `code speckit policy current [--json]` - Show current active policy
- [ ] `code speckit policy validate` - Validate model_policy.toml schema
- [ ] `/speckit.policy` TUI slash commands

---

## Acceptance Criteria (Testable)

### 977-A1: Deterministic Hash Test ✅ PASSING
- Capturing twice with identical inputs produces identical `hash`
- Different source_files produce different hash
- **Test**: `stage0/src/policy.rs` - `test_policy_snapshot_hash_determinism`

### 977-A2: Dual Storage Parity ✅ PASSING
- Snapshot written to FS and stored in capsule bytes are identical
- Load from either location produces same PolicySnapshot
- **Test**: `tui/src/memvid_adapter/tests.rs` - policy capture tests

### 977-A3: Event Binding ✅ PASSING
- After run start, capsule event log contains `PolicySnapshotRef`
- Event's policy_id matches latest snapshot ID/hash
- **Test**: `git_integration.rs` - `test_capsule_events_include_policy_snapshot_ref`

### 977-A4: Run Traceability ✅ PASSING
- Given a run_id, can retrieve the PolicySnapshot that governed it
- StageTransition events include policy reference
- **Implementation**: `policy_capture::capture_and_store_policy()` at run start

### 977-A5: CI Validation ✅ PASSING
- `doc_lint.py` fails if `docs/MODEL-POLICY.md` or `model_policy.toml` missing
- `doc_lint.py` validates required TOML sections
- **Test**: Run `python3 scripts/doc_lint.py` - all checks pass

---

## CI / Doc Contract Enforcement (IMPLEMENTED)

### Required Checks (in `scripts/doc_lint.py`)

| Check | Status | Function |
|-------|--------|----------|
| `docs/MODEL-POLICY.md` exists | DONE | `check_required_files()` |
| `model_policy.toml` exists | DONE | `check_required_files()` |
| TOML required sections present | DONE | `check_policy_toml_schema()` |

**Required TOML sections** (validated by `check_policy_toml_schema()`):
- `[meta]`
- `[system_of_record]`
- `[routing]`
- `[capture]`
- `[budgets]`
- `[scoring]`
- `[gates]`
- `[security]`

### Pre-commit Hook (ACTIVE)

```bash
# In .githooks/pre-commit (line 27-34)
python3 scripts/doc_lint.py --warn-only || exit 1
```

### Future: Event Validation
- [ ] Lint ensures every `PolicySnapshotRef` event points to valid `mv2://.../policy/...` object (requires 975 event schema)

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
