# Convergence Documentation Pointers

> **Do not fork the matrix here.**
> All convergence policy lives in `localmemory-policy`. This file provides pointers.

## Canonical Documentation

| Document | Location | Purpose |
|----------|----------|---------|
| CONVERGENCE_OVERVIEW.md | `localmemory-policy/docs/convergence/` | System-wide convergence architecture |
| CONVERGENCE_MATRIX.yaml | `localmemory-policy/docs/convergence/` | Service interface compatibility matrix |
| INTERFACES.md | `localmemory-policy/` | Canonical interface specifications |
| COMPATIBILITY.yaml | `localmemory-policy/` | Version compatibility requirements |

## codex-rs Convergence Role

This repo owns:

1. **Golden-path orchestration**: `code /speckit.auto` and Stage0 Tier1/Tier2
2. **Stage0 engine**: DCC (Tier1) + NotebookLM synthesis (Tier2)
3. **System pointer storage**: Best-effort traceability to local-memory

This repo does NOT own:

- Policy logic (owned by `localmemory-policy`)
- NotebookLM service internals (owned by `notebooklm-mcp`)
- Memory schema/domain definitions (owned by `localmemory-policy`)

## Tier2 Fail-Closed Semantics

Stage0 Tier2 (NotebookLM) is **enabled by default** but must **fail closed**:

- If NotebookLM service not ready OR notebook mapping missing:
  - Tier2 is SKIPPED (not errored)
  - Tier1 continues normally
  - Diagnostics are emitted (via tracing)
  - Fallback Divine Truth is generated

- There is NO "general notebook fallback"
- There is NO silent notebook creation

## System Pointer Memories

Stage0 stores execution artifacts as pointer memories in local-memory:

```yaml
domain: spec-tracker
tags:
  - system:true           # Required: marks as system artifact
  - spec:<SPEC-ID>        # e.g., spec:SPEC-KIT-102
  - stage:0               # Stage indicator
  - tier2:success|skipped|error  # Tier2 status
content: |
  Pointers (hashes, paths) + short summary
  NOT raw artifact content (no TASK_BRIEF/DIVINE_TRUTH blobs)
```

These are excluded from normal Tier1 retrieval by default.

## Service URLs

| Service | Default URL | Purpose |
|---------|-------------|---------|
| local-memory | `http://localhost:3002/api/v1` | Tier1 memory retrieval, pointer storage |
| NotebookLM | `http://127.0.0.1:3456` | Tier2 synthesis (optional) |

## Doctor Command

Verify convergence health with:

```bash
code stage0 doctor
```

Checks:
1. local-memory reachable
2. Domain resolution works (no silent fallback)
3. notebooklm-mcp reachable (if Tier2 enabled)
4. Notebook mapping exists for current domain/spec

Exit codes:
- 0: All checks pass
- 1: Warnings (Tier2 will skip but Tier1 works)
- 2: Errors (Stage0 may fail)

## Interface Versioning

If you change any assumption about:
- Endpoints (local-memory API, NotebookLM HTTP)
- Config paths
- Default behaviors

You MUST:
1. Bump the relevant version field
2. Update canonical docs via coordinated change with `localmemory-policy`

## Related Files in This Repo

| File | Purpose |
|------|---------|
| `codex-rs/stage0/src/lib.rs` | Stage0 engine entry |
| `codex-rs/stage0/src/tier2.rs` | NotebookLM client trait |
| `codex-rs/stage0/src/system_memory.rs` | Pointer memory storage |
| `codex-rs/stage0/tests/convergence_acceptance.rs` | Convergence integration tests |
| `scripts/convergence_check.sh` | CI convergence validation |
