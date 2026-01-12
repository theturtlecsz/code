# PROGRAM_2026Q1_ACTIVE.md - Active Program DAG

**Program:** Memvid-First Auditable Workbench
**Quarter:** Q1 2026
**Status:** Active

---

## Program Vision

Transform Codex-RS/Spec-Kit into a Memvid-first, auditable workbench with:
- Locked decisions (D1-D112+)
- V6 docs contract
- Hybrid retrieval (lexical + semantic)
- Full replay determinism
- Evidence-based evaluation

---

## Dependency DAG

```
                    ┌─────────────┐
                    │  SPEC-971   │ Capsule Foundation
                    │  (DONE)     │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌─────────┐  ┌─────────┐  ┌─────────┐
        │SPEC-972 │  │SPEC-977 │  │SPEC-978 │
        │(DONE)   │  │PolicySnp│  │Reflex   │
        │Hybrid   │  │         │  │Stack    │
        └────┬────┘  └────┬────┘  └────┬────┘
             │            │            │
             └────────────┼────────────┘
                          ▼
                    ┌─────────────┐
                    │  SPEC-975   │ Event Schema
                    │  (WAITING)  │
                    └──────┬──────┘
                           │
              ┌────────────┴────────────┐
              ▼                         ▼
        ┌─────────┐               ┌─────────┐
        │SPEC-973 │               │SPEC-976 │
        │Time-Trvl│               │LogicMesh│
        └─────────┘               └─────────┘
```

---

## Phase Schedule

### Phase 1: Foundation (COMPLETE)
- [x] SPEC-KIT-971: Capsule foundation, URI contract, crash recovery
- [x] Config switch (memory_backend = memvid | local-memory)

### Phase 2: Retrieval (COMPLETE)
- [x] SPEC-KIT-972: Hybrid retrieval, A/B harness, HybridBackend
- [x] Golden queries and evaluation metrics
- [x] P95 < 250ms verification

### Phase 3: Policy & Reflex (IN PROGRESS)
- [ ] SPEC-KIT-977: PolicySnapshot capture at boundaries
- [ ] SPEC-KIT-978: Reflex stack integration (SGLang/vLLM)
- [ ] SPEC-KIT-971 completion: resolve_uri, checkpoint listing

### Phase 4: Event & Export
- [ ] SPEC-KIT-975: Event schema v1
- [ ] Export verification and replay testing

### Phase 5: Advanced Features
- [ ] SPEC-KIT-973: Time-Travel UI
- [ ] SPEC-KIT-976: Logic Mesh

---

## Gate Criteria

| Gate | Criteria | Owner | Status |
|------|----------|-------|--------|
| G1 | 971 URI + checkpoint tests pass | - | PASSED |
| G2 | 972 eval harness operational | - | PASSED |
| G3 | 977 PolicySnapshot stored in capsule | - | PENDING |
| G4 | 978 Reflex bakeoff complete | - | PENDING |
| G5 | 975 Event schema validated | - | PENDING |

---

## Risk Register

| Risk | Mitigation | Status |
|------|------------|--------|
| Capsule corruption | Crash recovery + doctor command | MITIGATED |
| Retrieval latency | P95 benchmarking, caching | MITIGATED |
| Memory backend incompatibility | Config switch, fallback | MITIGATED |
| Reflex model quality | A/B harness, bakeoffs | PLANNED |

---

## Success Metrics

| Metric | Target | Current |
|--------|--------|---------|
| Memvid test coverage | 30+ tests | 37 tests |
| Stage0 test coverage | 250+ tests | 260 tests |
| Retrieval P95 latency | < 250ms | < 10ms (synthetic) |
| Doc lint pass rate | 100% | TBD |

---

*Updated: 2026-01-12*
