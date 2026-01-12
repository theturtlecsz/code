# SPEC.md - Codex-RS / Spec-Kit Task Tracking

**Version:** V6 Docs Contract
**Last Updated:** 2026-01-12

---

## Doc Precedence Order

When resolving conflicts or ambiguity, documents take precedence in this order:

1. **SPEC.md** (this file) - Task tracking, invariants, current state
2. **docs/PROGRAM_2026Q1_ACTIVE.md** - Active program DAG and phase gates
3. **docs/DECISION_REGISTER.md** - Locked decisions D1-D112+
4. **HANDOFF.md** - Session continuation context
5. **Individual SPEC-* directories** - Feature specifications

---

## Invariants

These invariants MUST NOT be violated:

### Architecture Boundary
- **Stage0 core has no Memvid dependency** - All Memvid concepts isolated in adapter
- **LocalMemoryClient trait is the interface** - MemvidMemoryAdapter is the implementation
- **Adapter boundary enforced** - Stage0 → LocalMemoryClient → MemvidMemoryAdapter → CapsuleHandle

### URI and Storage
- **Logical mv2:// URIs are immutable** - Once returned, never change
- **Physical IDs are never treated as stable keys** - Only logical URIs are stable
- **Single-writer capsule model** - Global lock + writer queue enforced

### Merge Policy
- **Merge modes are `curated` or `full` only** - Never squash, ff, or rebase
- **Curated**: Selective artifact inclusion with review
- **Full**: Complete artifact preservation

### Hybrid Retrieval
- **Hybrid = lex + vec** - Required for retrieval (not optional)
- **Score fusion via RRF or linear combination**

---

## Active Tasks

### In Progress

| Spec | Status | Owner | Next Action |
|------|--------|-------|-------------|
| SPEC-KIT-971 | 95% | - | resolve_uri API + checkpoint listing |
| SPEC-KIT-977 | 0% | - | PolicySnapshot capture |
| SPEC-KIT-978 | 0% | - | Reflex stack integration |

### Completed (Recent)

| Spec | Completion Date | Key Deliverables |
|------|-----------------|------------------|
| SPEC-KIT-972 | 2026-01-12 | Hybrid retrieval, A/B harness, HybridBackend |
| SPEC-KIT-971 (core) | 2026-01-11 | Capsule foundation, crash recovery, config switch |

### Blocked

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-975 (full) | Needs 972 eval harness | 976 Logic Mesh |
| SPEC-KIT-973 | Needs 977 PolicySnapshot | Time-Travel UI |

---

## Replay Truth Table

| Scenario | Expected Behavior | Determinism |
|----------|-------------------|-------------|
| Same capsule, same query, same branch | Identical results | Deterministic |
| Same capsule, same query, different branch | Branch-specific results | Deterministic within branch |
| Exported capsule, imported elsewhere | Identical to source | Deterministic |
| Offline replay (no network) | Uses cached embeddings | Deterministic if cached |
| Cross-version replay | Warn if version mismatch | Best-effort |

---

## Phase Gates

| Phase | Gate Criteria | Status |
|-------|---------------|--------|
| 1→2 | 971 URI contract + checkpoint tests | PASSED |
| 2→3 | 972 eval harness + 975 event schema v1 | PASSED |
| 3→4 | 972 parity gates + export verification | PASSED |
| 4→5 | 977 PolicySnapshot + 978 reflex stack | PENDING |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib memvid  # Memvid tests (37 passing)
cargo test -p codex-stage0 --lib      # Stage0 tests (260 passing)
python3 scripts/doc_lint.py           # Doc contract lint
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/  # Memvid implementation
codex-rs/stage0/src/              # Stage0 core (no Memvid dep)
codex-rs/stage0/src/hybrid.rs     # HybridBackend
docs/SPEC-KIT-*/                  # Feature specifications
```

---

*Maintained by automated tooling and session handoffs.*
