# Deprecations

This document tracks deprecated features, renamed configuration, and migration paths.

---

## Renamed Environment Variables

| Deprecated | Canonical | Behavior | Since |
|------------|-----------|----------|-------|
| `SPEC_KIT_CRITIC` | `SPEC_KIT_SIDECAR_CRITIC` | Canonical wins if both set; warn-once | PR3 |

## Removed Environment Variables

| Env Var | Status | Behavior | Since |
|---------|--------|----------|-------|
| `SPEC_KIT_CONSENSUS` | **REMOVED** | Ignored with warning; legacy voting deleted | PR6 |

**Example:**
```bash
# Before (deprecated)
export SPEC_KIT_CRITIC=true

# After (canonical)
export SPEC_KIT_SIDECAR_CRITIC=true
```

---

## Renamed Config Keys

| Deprecated | Canonical | Behavior | Since |
|------------|-----------|----------|-------|
| `quality_gates.consensus_threshold` | `quality_gates.min_confidence_for_auto_apply` | Canonical wins if both set; warn-once | PR2 |

**Example (speckit.toml):**
```toml
# Before (deprecated)
[quality_gates]
consensus_threshold = 0.65

# After (canonical)
[quality_gates]
min_confidence_for_auto_apply = 0.65
```

**Environment variable equivalent:**
```bash
# Before (deprecated)
export SPECKIT_QUALITY_GATES__CONSENSUS_THRESHOLD=0.65

# After (canonical)
export SPECKIT_QUALITY_GATES__MIN_CONFIDENCE_FOR_AUTO_APPLY=0.65
```

---

## Legacy Storage Naming (Intentionally Preserved)

The following names are **not** deprecated — they remain for read compatibility with historical artifacts:

| Name | Location | Rationale |
|------|----------|-----------|
| `consensus_runs` | SQLite table | No schema migrations; read compatibility |
| `consensus_ok` | JSON field in evidence | Historical artifact parsing |
| `consensus/` | Evidence directory prefix | Backward-compatible evidence loading |

These will **not** be renamed. New code uses canonical vocabulary (gate, signal, verdict) but reads legacy storage transparently.

---

## Removal Timeline

| Item | Status | Target |
|------|--------|--------|
| Legacy multi-agent voting path | **REMOVED** | PR6 |
| `SPEC_KIT_CONSENSUS` env var | **REMOVED** | PR6 |
| `consensus.rs` shim module | **REMOVED** | PR6 |
| `consensus_threshold` alias | Deprecated | Keep indefinitely (low maintenance) |
| `SPEC_KIT_CRITIC` alias | Deprecated | Keep indefinitely (low maintenance) |

---

## Removed Commands

| Command | Status | Migration |
|---|---|---|
| `/plan` | Removed | Use `/speckit.new` (create a SPEC) then `/speckit.auto SPEC-ID` |
| `/solve` | Removed | Use `/speckit.auto SPEC-ID` |
| `/code` | Removed | Use `/speckit.auto SPEC-ID` |
| `/spec-auto` | Removed | Use `/speckit.auto SPEC-ID` |
| `/spec-ops-*` | Removed | Use `/guardrail.*` |
