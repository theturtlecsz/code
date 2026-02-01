# SPEC.md - Codex-RS / Spec-Kit Task Tracking

**Version:** V6 Docs Contract
**Last Updated:** 2026-01-31

***

## Doc Precedence Order

When resolving conflicts or ambiguity, documents take precedence in this order:

1. **SPEC.md** (this file) - Task tracking, invariants, current state
2. **docs/PROGRAM.md** - Active program DAG and phase gates
3. **docs/DECISIONS.md** - Locked decisions D1-D134
4. **HANDOFF.md** - Session continuation context
5. **Individual SPEC-* directories*\* - Feature specifications

***

## Invariants

These invariants MUST NOT be violated:

### Architecture Boundary

* **Stage0 core has no Memvid dependency** - All Memvid concepts isolated in adapter
* **LocalMemoryClient trait is the interface** - MemvidMemoryAdapter is the implementation
* **Adapter boundary enforced** - Stage0 -> LocalMemoryClient -> MemvidMemoryAdapter -> CapsuleHandle

### System of Record

* **Memvid capsule is the system-of-record** - local-memory is fallback only (until SPEC-KIT-979 parity gates pass)
* **Stage0 pipeline honors `memory_backend`** - Do not hard-require local-memory when memvid selected
* **Fallback is conditional** - Only activate if enabled AND memvid fails AND local-memory healthy

### URI and Storage

* **Logical mv2:// URIs are immutable** - Once returned, never change
* **Physical IDs are never treated as stable keys** - Only logical URIs are stable
* **URI stability** - Graph/event references use logical URIs only (never raw frame IDs)
* **Single-writer capsule model** - Cross-process lock + writer queue enforced

### Checkpoints and Branches

* **Stage boundary commits create checkpoints** - Automatic on stage transitions
* **Manual commits also create checkpoints** - User-triggered via CLI/TUI
* **Run isolation via branches** - Every run writes to `run/<RUN_ID>` branch
* **Merge at Unlock** - Merges run branch into main using defined merge semantics

### Merge Policy

* **Merge modes are `curated` or `full` only** - Never squash, ff, or rebase
* **Curated**: Selective artifact inclusion with review
* **Full**: Complete artifact preservation

### Hybrid Retrieval

* **Hybrid = lex + vec** - Required for retrieval (not optional)
* **Score fusion via RRF or linear combination**

### Reflex Mode

* **Reflex is a routing mode** - `Implementer(mode=reflex)` not a new Stage0 role
* **No new role name** - Routing chooses backend based on policy + health + bakeoff thresholds
* **Stage context** - Reflex only applies to Implement stage

### Replay Determinism

* **Offline replay is exact for retrieval + events** - Timeline deterministic
* **LLM I/O depends on capture mode** - Controlled by PolicySnapshot settings
* **Capture modes** - `none | prompts_only | full_io`

### Explainability Artifacts (D127-D134)

* **ACE Frames + Maieutic Specs are canonical** - Consensus artifacts deprecated (D127)
* **Maieutic step is mandatory pre-execution** - Fast path allowed, no skip (D130)
* **Capture mode controls persistence** - `capture=none` runs in-memory only (D131)
* **Ship requires persisted artifacts** - `capture=none` cannot ship (D132)

### Multi-Surface Parity (D113/D133)

* **Tier 1 commands have full parity across TUI/CLI/headless** - Artifacts, gating semantics, and exit codes must match
* **Visualization is tiered** - UI/visualization may be TUI-first, but CLI must provide automation-critical coverage

### Headless Behavior (D133)

* **Headless requires maieutic input** - `--maieutic <path>` or `--maieutic-answers <json>`
* **Headless never prompts** - Hard assertion; no interactive codepath in headless
* **Exit codes for blocking states** - NEEDS\_INPUT, NEEDS\_APPROVAL, BLOCKED\_SHIP

### Schema Versioning (D134)

* **ACE Frame schema is generated** - Via schemars from Rust structs
* **Schema version embedded** - Every ACE Frame includes `schema_version` field
* **Breaking changes = new version** - Never mutate released schema

***

## Active Tasks

### In Progress

| Spec | Status | Owner | Next Action |
| ---- | ------ | ----- | ----------- |
| -    | -      | -     | -           |

### Planned

| Spec | Description |
| ---- | ----------- |
| -    | -           |

### Completed (Recent)

| Spec                | Completion Date | Key Deliverables                                                                              |
| ------------------- | --------------- | --------------------------------------------------------------------------------------------- |
| SPEC-KIT-983        | 2026-02-01      | Stage→agent defaults modal + root-only persistence with user-visible errors                   |
| MAINT-17            | 2026-02-01      | Fix codex-cli hermetic speckit tests to set \[speckit.stage\_agents] under GPT defaults       |
| MAINT-16            | 2026-01-31      | Headless ACE init + runtime-safe fetch + git repo-root parity (D113/D133)                     |
| SPEC-KIT-982        | 2026-01-31      | ACE + maieutic injection into per-agent prompts via unified builder (D113/D133 parity)        |
| SPEC-KIT-981        | 2026-01-31      | Config-driven stage→agent mapping with GPT-5.2 defaults, TUI/headless parity                  |
| MAINT-14            | 2026-01-31      | Fix ${ARTIFACTS}/${PREVIOUS\_OUTPUTS} placeholder leakage, NEXT\_FOCUS\_ROADMAP refresh       |
| SPEC-KIT-905        | 2026-01-30      | CLI stage parity: ID rename, docstring fixes, table-driven test, D113/D133 alignment          |
| SPEC-KIT-900        | 2026-01-29      | Headless CLI execution parity, real agent spawning via AGENT\_MANAGER, exit codes (D113/D133) |
| SPEC-KIT-980        | 2026-01-28      | PDF/DOCX ingest with feature gates, text extraction, searchable capsule persistence           |
| SPEC-KIT-974        | 2026-01-27      | Export/import, encryption, safe export, risk auto-export, GC enhancements                     |
| SPEC-KIT-979        | 2026-01-21      | Local-memory sunset phases, CLI flags, nightly parity workflow, import CLI, diagnostics       |
| SPEC-KIT-976        | 2026-01-19      | Logic Mesh graph foundation: Card/Edge schemas, CLI commands, 6 tests passing                 |
| SPEC-KIT-973        | 2026-01-19      | Time-travel TUI commands (timeline, asof, diff), label lookup, 3 tests passing                |
| SPEC-KIT-978        | 2026-01-18      | Circuit breaker types, BreakerState/BreakerStateChangedPayload, EventType integration         |
| SPEC-KIT-975        | 2026-01-18      | Replay timeline determinism, offline retrieval exactness, 5 replay tests passing              |
| SPEC-KIT-971        | 2026-01-17      | Branch isolation, time-travel URI, checkpoints, merge at unlock, CLI complete                 |
| SPEC-KIT-977        | 2026-01-17      | PolicySnapshot capture, dual storage, CLI/TUI commands, drift detection                       |
| SPEC-KIT-978 (core) | 2026-01-16      | JSON schema enforcement, bakeoff CLI, reflex routing decisions                                |
| SPEC-KIT-972        | 2026-01-12      | Hybrid retrieval, A/B harness, HybridBackend                                                  |

### Blocked

| Spec   | Blocker | Unblocks |
| ------ | ------- | -------- |
| (none) | -       | -        |

### Unblocked (Recent)

| Spec         | Resolution                                                     | Date       |
| ------------ | -------------------------------------------------------------- | ---------- |
| SPEC-KIT-900 | Implemented via AGENT\_MANAGER + tokio block\_in\_place (D133) | 2026-01-29 |

***

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

***

## Replay Truth Table

| Scenario                                   | Expected Behavior        | Determinism                 |
| ------------------------------------------ | ------------------------ | --------------------------- |
| Same capsule, same query, same branch      | Identical results        | Deterministic               |
| Same capsule, same query, different branch | Branch-specific results  | Deterministic within branch |
| Exported capsule, imported elsewhere       | Identical to source      | Deterministic               |
| Offline replay (no network)                | Uses cached embeddings   | Deterministic if captured   |
| Cross-version replay                       | Warn if version mismatch | Best-effort                 |

***

## Phase Gates

| Phase | Gate Criteria                                    | Status |
| ----- | ------------------------------------------------ | ------ |
| 1->2  | 971 URI contract + checkpoint tests              | PASSED |
| 2->3  | 972 eval harness operational                     | PASSED |
| 3->4  | 977 PolicySnapshot stored in capsule             | PASSED |
| 4->5  | 978 Reflex bakeoff complete + 975 event baseline | PASSED |
| 5->6  | 973/976 advanced features                        | PASSED |

***

## Policy Source Files

These files are REQUIRED and enforced by doc\_lint:

| File                   | Purpose                                      | Status    |
| ---------------------- | -------------------------------------------- | --------- |
| `docs/MODEL-POLICY.md` | Human-readable policy rationale ("why")      | Required  |
| `model_policy.toml`    | Machine-authoritative policy config ("what") | Required  |
| `PolicySnapshot.json`  | Compiled artifact stored in capsule          | Generated |

***

## Quick Reference

### Build & Test

```bash
~/code/build-fast.sh              # Fast build
cargo test -p codex-tui --lib         # TUI tests (667 passing)
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

***

*Maintained by automated tooling and session handoffs.*
