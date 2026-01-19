# DECISION_REGISTER.md - Locked Decisions

**Version:** 1.1
**Last Updated:** 2026-01-19
**Total Decisions:** D1-D112 (all LOCKED)

---

## Decision Status Legend

- **LOCKED**: Decision is final, implementation must follow
- **ACTIVE**: Decision is being implemented
- **SUPERSEDED**: Replaced by a newer decision

---

## Foundation Decisions (D1-D20)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D1 | Workspace capsule + per-run exports | LOCKED | 971 |
| D2 | Canonical capsule path conventions (.speckit/memvid/) | LOCKED | 971 |
| D3 | Stage0 core has no Memvid dependency | LOCKED | 971 |
| D4 | LocalMemoryClient trait is the interface | LOCKED | 971 |
| D5 | Hybrid = lex + vec (required, not optional) | LOCKED | 972 |
| D6 | mv2:// URI scheme for logical references | LOCKED | 971 |
| D7 | Single-writer capsule model with global lock | LOCKED | 971 |
| D8 | Memory_backend config switch | LOCKED | 971 |
| D9 | Fallback to local-memory if capsule unavailable | LOCKED | 971 |
| D10 | Crash recovery via write-ahead log | LOCKED | 971 |
| D11 | Checkpoint at stage boundaries | LOCKED | 971 |
| D12 | Branch isolation for runs | LOCKED | 971 |
| D13 | Curated or full merge modes only | LOCKED | 971 |
| D14 | Logical URIs are immutable once returned | LOCKED | 971 |
| D15 | Physical IDs never treated as stable keys | LOCKED | 971 |
| D16 | Adapter boundary enforced at compile time | LOCKED | 971 |
| D17 | Doctor command for capsule health | LOCKED | 971 |
| D18 | Stage boundary checkpoints | LOCKED | 971 |
| D19 | Evidence stored in capsule | LOCKED | 972 |
| D20 | TF-IDF/BM25 for lexical search | LOCKED | 972 |

---

## Retrieval Decisions (D21-D30)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D21 | RRF or linear fusion for hybrid scoring | LOCKED | 972 |
| D22 | IQO parameters for search filtering | LOCKED | 972 |
| D23 | Domain filtering with spec:* prefix support | LOCKED | 972 |
| D24 | Required/optional/exclude tag semantics | LOCKED | 972 |
| D25 | Importance threshold filtering | LOCKED | 972 |
| D26 | Explainable scoring with signal breakdown | LOCKED | 972 |
| D27 | A/B evaluation harness for comparison | LOCKED | 972 |
| D28 | Golden queries for regression testing | LOCKED | 972 |
| D29 | P95 latency < 250ms acceptance criteria | LOCKED | 972 |
| D30 | Report generation (JSON + Markdown) | LOCKED | 972 |

---

## Graph & Time-Travel Decisions (D31-D40)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D31 | Memory Card schema v1 (card_id, card_type, facts, provenance) | LOCKED | 976 |
| D33 | Run event capture in events track | LOCKED | 975 |
| D35 | Passphrase/key UX for exports | LOCKED | 974 |
| D37 | Multi-modal ingestion behind feature gates | LOCKED | 980 |
| D39 | Dual-backend runtime flag for A/B comparison | LOCKED | 979 |
| D40 | Parity gates for migration | LOCKED | 979 |

---

## Export & Security Decisions (D41-D60)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D43 | Reflex fallback order configurable | LOCKED | 978 |
| D46 | Export triggers (manual/risk/always modes) | LOCKED | 974 |
| D49 | Reflex loop compile/parse/patch/apply cycle | LOCKED | 978 |
| D50 | Bakeoff harness for reflex evaluation | LOCKED | 978 |
| D51 | Retrieval events captured with explainability | LOCKED | 975 |
| D52 | Migration tool for local-memory corpus | LOCKED | 979 |
| D53 | Deprecation plan for local-memory | LOCKED | 979 |
| D54 | Password/env var for capsule encryption | LOCKED | 974 |
| D55 | JSON schema enforcement for reflex | LOCKED | 978 |

---

## Advanced Decisions (D61-D80)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D58 | Graph references use logical URIs only | LOCKED | 976 |
| D59 | Offline replay never reissues network calls | LOCKED | 975 |
| D62 | Logic Edge schema v1 (edge_id, edge_type, from/to URIs) | LOCKED | 976 |
| D63 | EdgeType enum (DependsOn, Blocks, Implements, etc.) | LOCKED | 976 |
| D64 | CardType enum (Spec, Decision, Task, Risk, etc.) | LOCKED | 976 |
| D65 | StageTransition event type | LOCKED | 975 |
| D66 | PolicySnapshotRef event type | LOCKED | 975 |
| D70 | Safe export scope includes run artifacts | LOCKED | 974 |
| D71 | Safe export excludes raw LLM I/O by default | LOCKED | 974 |
| D72 | ToolCall/ToolResult event types | LOCKED | 975 |
| D75 | Export triggers on high-risk specs | LOCKED | 974 |
| D76 | Retention/GC for exports | LOCKED | 974 |
| D77 | Safe export redaction/masking | LOCKED | 974/976 |
| D78 | Audit logging for exports (CapsuleExported event) | LOCKED | 974/978 |
| D79 | Import verification via capsule doctor | LOCKED | 974 |

---

## Evaluation Decisions (D80-D100)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D89 | A/B harness saves to .speckit/eval/ | LOCKED | 972 |
| D90 | b_latency_acceptable(250) for P95 check | LOCKED | 972 |
| D91 | Precision@k, Recall@k, MRR metrics | LOCKED | 972 |
| D92 | Golden test memories for evaluation | LOCKED | 972 |
| D93 | Bakeoff CLI for reflex evaluation | LOCKED | 978 |
| D94 | Feature flag for local-memory removal | LOCKED | 979 |
| D95 | Replay report generator (markdown + JSON) | LOCKED | 975 |
| D97 | Graph-aware retrieval expansion | LOCKED | 976 |

---

## Policy Decisions (D100-D110)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D100 | PolicySnapshot captured at run boundaries | LOCKED | 977 |
| D101 | Events linked to policy snapshot version | LOCKED | 977 |
| D102 | Policy snapshot stored in capsule | LOCKED | 977 |

---

## Reflex Decisions (D110-D112)

| ID | Decision | Status | Spec |
|----|----------|--------|------|
| D110 | SGLang primary, vLLM fallback | LOCKED | 978 |
| D111 | Reflex operations captured in A/B harness | LOCKED | 978 |
| D112 | Fallback order configurable | LOCKED | 978 |

---

## Adding New Decisions

New decisions should:
1. Get the next available ID (D113+)
2. Reference the implementing SPEC
3. Be marked ACTIVE until implementation complete
4. Be marked LOCKED once implementation merged

---

## Decision Conflicts

If two decisions conflict:
1. Higher ID takes precedence (newer decision)
2. Document the supersession in this register
3. Mark old decision as SUPERSEDED

---

*This register is the authoritative source for architectural decisions.*
