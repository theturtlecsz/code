# MODEL-POLICY.md - Model Governance Rationale

**Version:** 2.0
**Effective Date:** 2026-01-12
**Status:** ACTIVE

---

## Purpose

This document provides the human-readable rationale ("why") for model governance decisions.
The machine-authoritative configuration ("what") lives in `model_policy.toml`.
At runtime, both are compiled into `PolicySnapshot.json` and stored in the Memvid capsule.

---

## System of Record

### Memvid-First Architecture

**Decision:** Memvid capsule is the system-of-record for all spec-kit artifacts, events, and policy snapshots.

**Rationale:**
1. **Audit trail**: Every artifact has a stable `mv2://` URI and is immutable once committed
2. **Replay determinism**: Capsule contains everything needed for offline replay
3. **Branch isolation**: Runs are isolated until explicit merge at Unlock stage
4. **Crash recovery**: Checkpoints enable recovery from mid-run failures

**Fallback:** local-memory is fallback only, activated when:
- Memvid capsule fails to open AND
- Fallback is explicitly enabled in config AND
- local-memory daemon is healthy

This fallback will be sunset once SPEC-KIT-979 parity gates pass.

---

## Model Routing

### Cloud Models (Default)

**Policy:** Use cloud models for all stages by default.

**Allowed models by role:**
- **Architect**: claude-sonnet-4-20250514, gpt-4o, gemini-2.0-flash
- **Implementer**: claude-sonnet-4-20250514, gpt-4o (default: claude)
- **Judge**: claude-sonnet-4-20250514 (requires high reasoning)

**Rationale:**
- Quality is paramount for production workflows
- Cost is acceptable for spec-kit automation (~$2.70/run)
- Latency is acceptable for non-interactive stages

### Reflex Mode (Local Inference)

**Decision:** Reflex is a routing mode, not a new role. Expressed as `Implementer(mode=reflex)`.

**When to use Reflex:**
- Implement stage only (code generation tasks)
- Local server is healthy and passes bakeoff thresholds
- Task is suitable for local model (structured output, bounded complexity)

**Bakeoff thresholds (promotion gate):**
- P95 latency < 2000ms
- Success parity >= 85% vs cloud implementer
- JSON schema compliance: 100%

**Fallback order:**
1. Reflex (if enabled + healthy + thresholds met)
2. Cloud implementer (default)

**Rationale:**
- Cost reduction for high-volume implementation tasks
- Latency reduction for interactive workflows
- Privacy for sensitive codebases
- NOT a quality compromise - bakeoff gates ensure parity

---

## Capture and Replay

### Capture Modes

**Policy:** Capture mode is set per-run via PolicySnapshot.

| Mode | What's Captured | Use Case |
|------|-----------------|----------|
| `none` | Events only, no LLM I/O | Production, privacy-sensitive |
| `prompts_only` | Prompts + metadata, no responses | Debugging, audit |
| `full_io` | Full request/response pairs | Replay, regression testing |

**Default:** `prompts_only`

**Rationale:**
- `full_io` enables exact replay but has storage cost
- `prompts_only` balances auditability with storage efficiency
- `none` is for production where storage/privacy are concerns

### Replay Determinism

**Invariant:** Retrieval + event timeline is exact for offline replay.

**LLM I/O determinism depends on capture mode:**
- `full_io`: Exact replay of all LLM responses
- `prompts_only`: Re-execute prompts (may differ due to model updates)
- `none`: No LLM replay possible

---

## Budget and Cost Control

### Token Budgets

**Policy:** Stage0 context compilation respects token budgets.

| Stage | Max Tokens | Rationale |
|-------|------------|-----------|
| Plan | 8000 | Needs full context for architecture decisions |
| Tasks | 4000 | Focused on task breakdown |
| Implement | 6000 | Needs relevant context for code generation |
| Validate | 4000 | Focused on test execution |
| Audit | 4000 | Focused on review criteria |
| Unlock | 2000 | Summary only |

### Cost Ceilings

**Policy:** Warn if run cost exceeds thresholds.

| Threshold | Action |
|-----------|--------|
| $5.00 | Warning in logs |
| $10.00 | Require explicit confirmation |
| $25.00 | Hard block (configurable) |

**Rationale:** Prevent runaway costs from loops or large contexts.

---

## Gate Criteria

### Reflex Promotion Gate

A local model is promoted to reflex-eligible when:
1. Bakeoff harness passes (success parity >= 85%)
2. P95 latency < 2000ms on representative tasks
3. JSON schema compliance = 100%
4. No regressions on golden query suite

### Parity Gate (SPEC-KIT-979)

local-memory can be sunset when:
1. Memvid retrieval P95 < local-memory P95
2. Memvid search quality >= local-memory (measured by eval harness)
3. 30-day stability without fallback activations
4. All existing workflows migrated

---

## Security and Privacy

### Secrets Handling

**Policy:** Never capture secrets in PolicySnapshot or capsule events.

**Redaction rules:**
- Environment variables matching `*_KEY`, `*_SECRET`, `*_TOKEN` are redacted
- API responses containing credentials are redacted before capture
- File paths containing `.env`, `credentials`, `secrets` are excluded

### Data Residency

**Policy:** Capsule data stays local by default.

**Export controls:**
- Export requires explicit user action
- Sensitive runs can be marked "no-export"
- Encryption at rest is optional but recommended for shared capsules

---

## Versioning

### Policy Version Compatibility

**Schema version:** 1.0

**Compatibility rules:**
- Minor version bumps (1.x) are backward compatible
- Major version bumps require migration
- PolicySnapshot includes schema_version for validation

### Migration Path

When policy schema changes:
1. Old snapshots remain valid (read with compat shim)
2. New runs use new schema
3. Migration tool converts old snapshots if needed

---

## References

- `model_policy.toml` - Machine-authoritative configuration
- `SPEC.md` - Invariants and task tracking
- `docs/SPEC-KIT-977-model-policy-v2/spec.md` - PolicySnapshot specification
- `docs/SPEC-KIT-978-local-reflex-sglang/spec.md` - Reflex specification

---

*This document is the source of truth for policy rationale. Changes require review.*
