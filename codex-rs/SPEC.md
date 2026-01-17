# SPEC.md - Codex-RS / Spec-Kit Task Tracking

**Version:** V6 Docs Contract
**Last Updated:** 2026-01-16

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
- **Adapter boundary enforced** - Stage0 -> LocalMemoryClient -> MemvidMemoryAdapter -> CapsuleHandle

### System of Record
- **Memvid capsule is the system-of-record** - local-memory is fallback only (until SPEC-KIT-979 parity gates pass)
- **Stage0 pipeline honors `memory_backend`** - Do not hard-require local-memory when memvid selected
- **Fallback is conditional** - Only activate if enabled AND memvid fails AND local-memory healthy

### URI and Storage
- **Logical mv2:// URIs are immutable** - Once returned, never change
- **Physical IDs are never treated as stable keys** - Only logical URIs are stable
- **URI stability** - Graph/event references use logical URIs only (never raw frame IDs)
- **Single-writer capsule model** - Cross-process lock + writer queue enforced

### Checkpoints and Branches
- **Stage boundary commits create checkpoints** - Automatic on stage transitions
- **Manual commits also create checkpoints** - User-triggered via CLI/TUI
- **Run isolation via branches** - Every run writes to `run/<RUN_ID>` branch
- **Merge at Unlock** - Merges run branch into main using defined merge semantics

### Merge Policy
- **Merge modes are `curated` or `full` only** - Never squash, ff, or rebase
- **Curated**: Selective artifact inclusion with review
- **Full**: Complete artifact preservation

### Hybrid Retrieval
- **Hybrid = lex + vec** - Required for retrieval (not optional)
- **Score fusion via RRF or linear combination**

### Reflex Mode
- **Reflex is a routing mode** - `Implementer(mode=reflex)` not a new Stage0 role
- **No new role name** - Routing chooses backend based on policy + health + bakeoff thresholds
- **Stage context** - Reflex only applies to Implement stage

### Replay Determinism
- **Offline replay is exact for retrieval + events** - Timeline deterministic
- **LLM I/O depends on capture mode** - Controlled by PolicySnapshot settings
- **Capture modes** - `none | prompts_only | full_io`

---

## Active Tasks

### In Progress

| Spec | Status | Owner | Next Action |
|------|--------|-------|-------------|
| SPEC-KIT-971 | 95% | - | Final polish: checkpoint listing CLI, branch merge UI |
| SPEC-KIT-977 | 85% | - | Policy CLI/TUI commands (show, compare, diff) |
| SPEC-KIT-978 | 65% | - | Eval artifact writer + CI gate + LLMCall capture alignment with 975 |

### Completed (Recent)

| Spec | Completion Date | Key Deliverables |
|------|-----------------|------------------|
| SPEC-KIT-971 (core) | 2026-01-16 | Branch isolation, time-travel URI resolution, checkpoint labels, cross-process lock |
| SPEC-KIT-977 (core) | 2026-01-16 | GovernancePolicy capture, capsule storage, PolicySnapshotRef binding, drift detection |
| SPEC-KIT-978 (core) | 2026-01-16 | JSON schema enforcement, bakeoff_runner module, reflex routing decisions |
| SPEC-KIT-972 | 2026-01-12 | Hybrid retrieval, A/B harness, HybridBackend |

### Blocked

| Spec | Blocker | Unblocks |
|------|---------|----------|
| SPEC-KIT-975 | Needs 977 PolicySnapshot + events query API | 973 Time-Travel, 976 Logic Mesh |
| SPEC-KIT-973 | Needs 975 event schema | Time-Travel UI |
| SPEC-KIT-976 | Needs 975 event schema | Logic Mesh graph |

---

## Gating Chain

```
971 (Capsule) + 977 (PolicySnapshot) + 978 (Reflex)
                    |
                    v
              975 (Event Schema)
                    |
        +-----------+-----------+
        v                       v
  973 (Time-Travel)      976 (Logic Mesh)
                    |
                    v
              979 (local-memory sunset)
```

---

## Replay Truth Table

| Scenario | Expected Behavior | Determinism |
|----------|-------------------|-------------|
| Same capsule, same query, same branch | Identical results | Deterministic |
| Same capsule, same query, different branch | Branch-specific results | Deterministic within branch |
| Exported capsule, imported elsewhere | Identical to source | Deterministic |
| Offline replay (no network) | Uses cached embeddings | Deterministic if captured |
| Cross-version replay | Warn if version mismatch | Best-effort |

---

## Phase Gates

| Phase | Gate Criteria | Status |
|-------|---------------|--------|
| 1->2 | 971 URI contract + checkpoint tests | PASSED |
| 2->3 | 972 eval harness operational | PASSED |
| 3->4 | 977 PolicySnapshot stored in capsule | PASSED |
| 4->5 | 978 Reflex bakeoff complete + 975 event baseline | PENDING |
| 5->6 | 973/976 advanced features | PENDING |

---

## Policy Source Files

These files are REQUIRED and enforced by doc_lint:

| File | Purpose | Status |
|------|---------|--------|
| `docs/MODEL-POLICY.md` | Human-readable policy rationale ("why") | Required |
| `model_policy.toml` | Machine-authoritative policy config ("what") | Required |
| `PolicySnapshot.json` | Compiled artifact stored in capsule | Generated |

---

## Quick Reference

### Build & Test
```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib memvid  # Memvid tests (47 passing)
cargo test -p codex-stage0 --lib      # Stage0 tests (269 passing)
python3 scripts/doc_lint.py           # Doc contract lint
python3 scripts/golden_path_test.py   # E2E validation (10/10)
```

### Key Paths
```
codex-rs/tui/src/memvid_adapter/  # Memvid implementation
codex-rs/stage0/src/              # Stage0 core (no Memvid dep)
codex-rs/stage0/src/policy.rs     # PolicySnapshot
codex-rs/stage0/src/hybrid.rs     # HybridBackend
docs/SPEC-KIT-*/                  # Feature specifications
```

---

*Maintained by automated tooling and session handoffs.*
