# SPEC-KIT-921 Phase B: Review Contract

**Status:** Active
**Scope:** Executor review command topology, parsing, and semantics

This document defines the **single source of truth** for the review command's behavior.
All adapters (TUI, CLI, CI) must produce identical outcomes for equivalent inputs.

---

## 1. Canonical Evidence Topology

Review reads from these paths (no stub files, no invented locations):

| Artifact Type | Path Pattern | Purpose |
|---------------|--------------|---------|
| **Spec packet** | `docs/<SPEC-ID>/{spec.md, plan.md, tasks.md, ...}` | Stage completion artifacts |
| **Review evidence** | `docs/SPEC-OPS-004-.../evidence/consensus/<SPEC-ID>/spec-<stage>_*.json` | Gate signals (ConsensusJson) |
| **Telemetry** | `docs/SPEC-OPS-004-.../evidence/commands/<SPEC-ID>/*_telemetry_*.json` | Optional enrichment (never changes resolution) |

**Constants:**
```rust
const SPEC_PACKET_ROOT: &str = "docs";
const EVIDENCE_ROOT: &str = "docs/SPEC-OPS-004-integrated-coder-hooks/evidence";
const CONSENSUS_DIR: &str = "consensus";  // legacy name retained
const COMMANDS_DIR: &str = "commands";
```

---

## 2. Stage to Checkpoint Mapping

| Requested Stage | Evaluated Checkpoint | Notes |
|-----------------|---------------------|-------|
| `Plan` | `AfterPlan` | Canonical review point |
| `Tasks` | `AfterTasks` | Canonical review point |
| `Audit` | `BeforeUnlock` | Canonical review point |
| `Implement` | `AfterImplement` | Diagnostic (opt-in) |
| `Validate` | `AfterValidate` | Diagnostic (opt-in) |
| `Unlock` | `BeforeUnlock` | Alias with message: "Reviewing Audit output" |
| `Specify` | N/A | NotApplicable: "Run /speckit.status or review Plan" |

**Result includes both:** `requested_stage` and `evaluated_checkpoint` for debuggability.

---

## 3. ConsensusJson Parsing Rules

### File Selection
- Pattern: `spec-<stage_slug>_*.json` (e.g., `spec-plan_claude_20251120.json`)
- Selection: **Lexicographic max** of matching filenames (latest by naming convention)
- Stage slug mapping: `plan`, `tasks`, `implement`, `validate`, `audit`

### Parse Failure Handling
- Emit `ReviewSignal { kind: Other, origin: SignalOrigin::System, severity: Advisory }`
- Message: "Failed to parse consensus file: {path}: {error}"
- **Does not block** (advisory only) — infra issues shouldn't halt review

### Stage Mismatch
- Files exist but none match checkpoint's stage slug → treat as "no artifacts for this stage"

### ConsensusJson Schema (subset we read)
```rust
struct ConsensusJson {
    agent: Option<String>,
    model: Option<String>,
    error: Option<String>,
    consensus: Option<ConsensusDetail>,
}

struct ConsensusDetail {
    conflicts: Option<Vec<String>>,
    synthesis_status: Option<String>,
}
```

---

## 4. Signal Derivation Rules

### Blocking Signals (force Escalate)
- `consensus.conflicts` non-empty → one `ReviewSignal` per conflict
  - `kind: Contradiction`
  - `origin: SignalOrigin::Role(inferred from agent field)`
  - `severity: Block`

### Advisory Signals (warnings, don't block)
- `error` field present → `ReviewSignal`
  - `kind: Other`
  - `origin: SignalOrigin::System`
  - `severity: Advisory`
- Parse failures (as above)

### No Artifacts Case
- No matching `spec-<stage>_*.json` files → `artifacts_collected = 0`
- Results in `DisplayVerdict::Skipped { reason: NoArtifactsFound }`

### Invariant (enforced)
```
resolution == AutoApply ⟹ blocking_signals.is_empty()
```
If this invariant would be violated, resolution MUST be Escalate.

---

## 5. Resolution and Exit-Code Mapping

| Resolution | Display Verdict | Exit Code | Notes |
|------------|-----------------|-----------|-------|
| `AutoApply` + no signals | `Passed` | 0 | Clean pass |
| `AutoApply` + advisory signals | `PassedWithWarnings` | 0 (or 1 if `strict_warnings`) | Warnings present |
| `Escalate` | `Failed` | 2 | Human review required |
| Skipped (no artifacts) | `Skipped` | 0 (or 2 if `strict_artifacts`) | Warning on stderr |
| Tool/infra failure | N/A | 3 | Via `Result::Err`, not `StageReviewResult` |

### Strict Mode Flags
- `--strict-artifacts`: Missing artifacts → exit 2 (fail CI)
- `--strict-warnings`: PassedWithWarnings → exit 1 (soft fail)

---

## 6. Output Stability Rules

### P0-B: No Env Reads in Core
- `PolicySnapshot` is passed via `ReviewRequest.policy` or `ExecutionContext`
- Env resolution happens once in adapter, before calling executor

### P0-C: Repo-Relative Evidence Refs
- All paths in `EvidenceRefs` are **repo-relative** (no leading `/`, no absolute paths)
- Implementation: strip `repo_root` prefix before storing
- Enables: stable serialization, golden tests, cross-machine CI

---

## Implementation Checklist

- [ ] P0-A: Read from `evidence/consensus/<SPEC-ID>/spec-<stage>_*.json`
- [ ] P0-D: Parse ConsensusJson, extract conflicts → blocking, errors → advisory
- [ ] P0-B: Remove `std::env::var()` calls; policy via request
- [ ] P0-C: Evidence refs are repo-relative strings
- [ ] Tests: Fixture-based with conflict/clean/parse-error cases
- [ ] Tests: Strict mode (missing artifacts + strict → exit 2)
