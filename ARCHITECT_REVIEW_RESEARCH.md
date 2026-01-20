# ARCHITECT_REVIEW_RESEARCH.md

**Generated:** 2026-01-19
**Session:** 7 of N
**Status:** Phase 0 Complete, Phase 1 (A1-G3) Complete

---

## 1. STATE

### Research Progress

| Question | Status | Session |
|----------|--------|---------|
| **Phase 0: Archive Scan** | COMPLETE | 1 |
| A1: Primary product promise | COMPLETE | 1 |
| A2: Must-have interaction surfaces | COMPLETE | 1 |
| B1: Capsule SOR content | COMPLETE | 2 |
| B2: Retention posture | COMPLETE | 2 |
| C1: Capture-mode enforcement | COMPLETE | 3 |
| C2: Default capture mode | COMPLETE | 3 |
| D1: Pipeline modularity | COMPLETE | 4 |
| D2: Quality gates posture | COMPLETE | 4 |
| E1: Enforcement strictness | COMPLETE | 5 |
| E2: Hard-fail policy set | COMPLETE | 5 |
| F1: Maintenance framework timing | COMPLETE | 6 |
| F2: First maintenance job family | COMPLETE | 6 |
| G1: Cross-cutting capability verification | COMPLETE | 7 |
| G2: Capability matrix definition | COMPLETE | 7 |
| G3: Regression prevention | COMPLETE | 7 |

### Next Session Targets
- H (Final Synthesis & Recommendations) or research complete

---

## 2. Repo Context Summary

### Canonical Sources Read

| Document | Key Takeaways |
|----------|---------------|
| `codex-rs/SPEC.md` | V6 Docs Contract; Memvid capsule SOR; single-writer model; Reflex = Implementer(mode=reflex); capture modes: none/prompts_only/full_io |
| `docs/PROGRAM_2026Q1_ACTIVE.md` | Q1 2026 program complete (SPEC-971 through SPEC-976); "Memvid-First Auditable Workbench" vision |
| `docs/DECISION_REGISTER.md` | 112 locked decisions (D1-D112); hybrid retrieval mandatory; mv2:// URIs immutable |
| `docs/MODEL-POLICY.md` | Cloud models default; Reflex for Implement stage only; capture mode per-run; $5/$10/$25 cost thresholds |
| `model_policy.toml` | schema_version 1.0; prompts_only default; reflex disabled; vector_weight 0.6 |

### Non-Negotiable Invariants (From SPEC.md)

1. **Stage0 core has no Memvid dependency** (adapter boundary)
2. **Memvid capsule is SOR** (local-memory fallback until SPEC-979 parity)
3. **Logical mv2:// URIs immutable** (never use physical IDs as keys)
4. **Single-writer capsule model** (cross-process lock)
5. **Stage boundary commits create checkpoints**
6. **Run branches `run/<RUN_ID>`** merge at Unlock
7. **Merge modes: curated or full only** (never squash/ff/rebase)
8. **Reflex is a routing mode** (not a new role)
9. **Hybrid = lex + vec** (required for retrieval)
10. **Replay offline-first**: exact for retrieval + events; LLM I/O depends on capture mode

---

## 3. Archive Index Summary

### Location
`/home/thetu/code/docs/archive/specs/`

### Counts
- **Total spec directories:** 114
- **SPEC-KIT-* series:** 78
- **SYNC-* series:** 31
- **SPEC-* (legacy):** 5

### Theme Distribution (Approximate)

| Theme | Count | Examples |
|-------|-------|----------|
| Architecture/Design | 12 | SPEC-KIT-931, SPEC-KIT-930, SPEC-948 |
| Pipeline/Automation | 15 | SPEC-KIT-920, SPEC-KIT-921, SPEC-948, SPEC-947 |
| Memory/Storage | 10 | SPEC-KIT-071, SPEC-KIT-072, SPEC-KIT-102, SPEC-KIT-103 |
| Performance | 6 | SPEC-KIT-936, SPEC-KIT-940 |
| OAuth/Identity | 5 | SPEC-KIT-947, SPEC-KIT-951-953 |
| Testing | 8 | SPEC-KIT-900, SPEC-KIT-955 |
| Telemetry/Observability | 4 | SYNC-011, SYNC-031, SPEC-OPS-004 |
| Templates/Scaffolding | 6 | SPEC-KIT-960, SPEC-KIT-961, SPEC-KIT-962 |
| Sync/Upstream | 31 | SYNC-002 through SYNC-031 |

---

## 4. Architectural Shift Carrier Specs (16 Identified)

These specs carry significant architectural weight and inform multiple review questions.

| ID | Title | Tags | Summary |
|----|-------|------|---------|
| **SPEC-KIT-931** | Architectural Deep Dive | architecture, analysis | Master index coordinating 10 child specs; key decisions: Event Sourcing NO-GO, Actor Model NO-GO, Storage Consolidation GO, Tmux Removal DONE |
| **SPEC-KIT-936** | Tmux Elimination | performance, architecture | 65x speedup (6.5s→<50ms); DirectProcessExecutor replaced tmux; 851 LOC deleted |
| **SPEC-KIT-102** | NotebookLM Integration | memory, retrieval, synthesis | Tier 2 synthesis layer; Stage 0 planning; citation-grounded reasoning; HTTP API (localhost:3456) |
| **SPEC-KIT-103** | Librarian & Repair Jobs | memory, maintenance | Offline LLM-powered graph repair; meta-memory synthesis; causal relationship labeling; local 8-14B models |
| **SPEC-KIT-947** | Multi-Provider OAuth | oauth-identity, architecture | Auto-switch authentication; seamless multi-provider OAuth; unified `/model` command |
| **SPEC-KIT-948** | Modular Pipeline Logic | pipeline, modularity | Stage dependency graph; 3-tier config precedence; skip conditions; 4 workflow patterns |
| **SPEC-KIT-920** | TUI Automation | automation, cli | Superseded by SPEC-921; documents why PTY-based TUI automation failed; led to CLI adapter |
| **SPEC-KIT-940** | Performance Instrumentation | performance, testing | Measurement infrastructure; statistical benchmarking; pre/post validation |
| **SPEC-KIT-941** | Automated Policy Compliance | policy, automation | Guardrails enforcement; violation detection; evidence storage |
| **SPEC-KIT-960** | /speckit.project Command | templates, cli | Scaffold projects in <1s; Tier 0 (zero agents); Rust/Python/TypeScript/generic |
| **SPEC-KIT-970** | Interactive PRD | ui-tui, templates | Interactive PRD builder; guided workflow; spec.md generation |
| **SYNC-011** | OpenTelemetry | telemetry, observability | Distributed tracing; OTLP export; production observability |
| **SYNC-026** | Retention/Compaction | storage, maintenance | (Spec not found but referenced in archive list) |
| **SPEC-OPS-004** | Integrated Coder Hooks | automation, evidence | Evidence collection; telemetry logging; cost tracking |
| **SPEC-947** | Pipeline UI Configurator | pipeline, ui-tui | Visual pipeline builder; stage configuration; model routing |
| **SPEC-949** | Extended Model Support | model-routing, policy | 13+ models across Gemini/Claude/GPT; per-model routing |

---

## 5. Archive-to-Question Mapping (Draft)

| Question | Most Relevant Archived Specs |
|----------|------------------------------|
| **A1**: Primary product promise | SPEC-KIT-931 (architectural decisions), SPEC-KIT-102 (tiered architecture), SPEC-KIT-940 (measurement) |
| **A2**: Interaction surfaces | SPEC-KIT-920/921 (TUI vs CLI), SPEC-KIT-960 (CLI scaffolding), SPEC-KIT-970 (TUI PRD) |
| **B1**: Capsule SOR content | SPEC-KIT-102 (tier1/tier2 split), SPEC-KIT-103 (meta-memories), SPEC-KIT-971 (capsule foundation) |
| **B2**: Retention posture | SPEC-KIT-103 (compaction), SYNC-026 (retention), SPEC-KIT-909 (evidence cleanup) |
| **C1/C2**: Capture modes | SPEC-KIT-977 (PolicySnapshot), docs/MODEL-POLICY.md |
| **D1**: Pipeline modularity | SPEC-KIT-948 (modular logic), SPEC-947 (UI configurator) |
| **D2**: Quality gates | SPEC-KIT-068, SPEC-KIT-941 (policy compliance), SPEC-KIT-904 |
| **E1/E2**: Enforcement | SPEC-KIT-941, docs/MODEL-POLICY.md, SPEC-KIT-902 (guardrails) |
| **F1/F2**: Maintenance | SPEC-KIT-103 (librarian), SPEC-KIT-909 (evidence cleanup), SYNC-026 |
| **G**: Capabilities | Cross-cutting: SPEC-KIT-947 (OAuth), SPEC-KIT-103 (librarian), SPEC-KIT-948 (modular), SPEC-KIT-102 (synthesis) |

---

## 6. Research Brief: A1 - Primary Product Promise

### Question
**A1) Primary product promise: Forensics vs Productivity vs Automation vs Balanced**

Should Codex-RS/Spec-Kit position itself as:
- A **forensic/audit workbench** (complete event trail, replay capability, compliance)
- A **productivity tool** (fast iteration, developer experience, time savings)
- An **automation platform** (CI/CD integration, headless pipelines, autonomous agents)
- A **balanced hybrid** combining elements

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Event Sourcing** | Pattern where state changes are stored as immutable events; the event log is the system of record |
| **Audit Trail** | Chronological record of system activities enabling reconstruction of past states |
| **Productivity Tool** | Software that reduces time/effort for specific tasks; measured in velocity gains |
| **Agentic Automation** | AI systems that execute multi-step tasks autonomously with minimal supervision |
| **Developer Workbench** | Integrated environment combining multiple tools for a workflow domain |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Martin Fowler: Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) - Canonical definition; trade-offs analysis
2. [Microsoft Azure: Event Sourcing Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing) - Enterprise implementation guidance
3. [AWS DevOps Blog: AI-DLC Adaptive Workflows](https://aws.amazon.com/blogs/devops/open-sourcing-adaptive-workflows-for-ai-driven-development-life-cycle-ai-dlc/) - End-to-end traceability in AI-driven development

#### Security/Privacy-Focused Critiques
4. [Lasso Security: LLM Data Privacy](https://www.lasso.security/blog/llm-data-privacy) - Audit trail challenges; shadow AI risks
5. [Protecto AI: LLM Privacy Protection 2025](https://www.protecto.ai/blog/llm-privacy-protection-strategies-2025) - Privacy-aware logging best practices
6. [Medium: The AI Audit Trail](https://medium.com/@kuldeep.paul08/the-ai-audit-trail-how-to-ensure-compliance-and-transparency-with-llm-observability-74fd5f1968ef) - LLM observability patterns

#### Real-World Implementation References
7. [BayTech Consulting: Event Sourcing Explained 2025](https://www.baytechconsulting.com/blog/event-sourcing-explained-2025) - Strategic use cases; hybrid patterns
8. [Eventuate.io: Why Event Sourcing](https://eventuate.io/whyeventsourcing.html) - Practical implementation rationale

#### Practitioner/Engineering Blog Posts
9. [Rico Fritzsche: Beyond the Hype - Event Modeling](https://ricofritzsche.me/beyond-the-hype-event-modeling-event-sourcing-and-real-choices/) - When to use vs avoid
10. [METR: AI Productivity Study](https://metr.org/blog/2025-07-10-early-2025-ai-experienced-os-dev-study/) - RCT finding AI tools made experienced devs 19% slower
11. [Index.dev: AI Coding Assistant ROI](https://www.index.dev/blog/ai-coding-assistants-roi-productivity) - Real productivity data 2025
12. [Bain: GenAI in Software Development](https://www.bain.com/insights/from-pilots-to-payoff-generative-ai-in-software-development-technology-report-2025/) - Enterprise deployment patterns

### C) Neutral Synthesis

#### What Tends to Work

**For Forensic/Audit Focus:**
- Event sourcing provides "perfect, intrinsic audit log" - every state change captured as immutable event
- Finance/healthcare/regulated industries derive clear value from compliance-ready trails
- Incident forensics and root cause analysis benefit from complete history
- Codex-RS already has: mv2:// URI stability, checkpoint commits, PolicySnapshot capture

**For Productivity Focus:**
- Studies show 10-30% individual productivity gains from AI tools
- Most value comes from routine task acceleration (boilerplate, testing, docs)
- Developer time breakdown: only 20-40% is actual coding; holistic gains require full-workflow optimization
- Codex-RS has: Stage0 context compilation, NotebookLM synthesis, sub-100ms cached responses

**For Automation Focus:**
- CI/CD integration and headless mode are now table stakes for AI coding tools
- Autonomous agents (Level 4) can execute multi-step plans with minimal supervision
- However: "fully hands-off coding is not reliable in 2025" - human-in-the-loop remains necessary
- Codex-RS has: /speckit.auto pipeline, CLI adapter, exit code contracts

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Over-instrumentation** | Event sourcing everywhere adds complexity without proportional value | Apply selectively (high-value entities only) |
| **Productivity theater** | Measuring velocity without impact; time saved not redirected to value | Measure outcomes, not just speed |
| **Automation brittleness** | Autonomous systems fail silently; human review becomes bottleneck | Explicit approval gates; PR review time already +91% in heavy AI teams |
| **Audit log gaps** | Logs deleted for storage; missing context for forensics | Immutable storage; retention policy from day 1 |

#### Engineering Costs/Risks

| Approach | Engineering Cost | Operational Cost | Reversibility |
|----------|------------------|------------------|---------------|
| Forensics-first | High (150-180h migration per SPEC-931F) | Medium (storage, replay) | Low (architecture-pervasive) |
| Productivity-first | Low-Medium | Low | High |
| Automation-first | Medium | Medium (CI/CD, monitoring) | Medium |
| Balanced | Medium-High | Medium | Medium |

#### What's Different for Offline-First Event-Sourced Workbench

Codex-RS's context is unusual:
1. **Offline replay is a first-class requirement** - not just audit, but deterministic reconstruction
2. **Single-writer capsule model** - simplifies event ordering vs distributed systems
3. **Capture mode toggle** - can run hot (prompts_only) or cold (full_io) depending on need
4. **Branch isolation** - run branches enable parallel exploration without SOR corruption
5. **Already invested in event infrastructure** - SPEC-975 event schema, timeline commands

### D) Decision Considerations Checklist

1. [ ] **Regulatory environment**: Are primary users in regulated industries (finance, healthcare, defense)?
2. [ ] **Incident frequency**: How often do users need to reconstruct "what happened"?
3. [ ] **Developer experience priority**: Is perceived speed/friction the dominant user complaint?
4. [ ] **CI/CD maturity**: Do users have existing automation expecting headless integration?
5. [ ] **Storage constraints**: What's the acceptable evidence footprint per SPEC? (current: 25MB soft limit)
6. [ ] **Replay frequency**: How often will users actually use time-travel/replay features?
7. [ ] **Competitor positioning**: Where do Cursor/Windsurf/Claude Code position themselves?
8. [ ] **Monetization model**: Enterprise (compliance value) vs individual (productivity value)?
9. [ ] **SPEC-931F finding**: Event sourcing was rejected as "too complex for current SLA" - has this changed?
10. [ ] **Hybrid viability**: Can forensic depth be gated (full_io for audit runs, prompts_only for dev)?
11. [ ] **Measurement gap**: Per SPEC-940, are claims about productivity/forensics actually measured?
12. [ ] **User segmentation**: Different promises for different user types (enterprise vs indie dev)?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-931 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/` | Event Sourcing NO-GO decision; 150-180h complexity estimate |
| SPEC-KIT-102 | `docs/archive/specs/SPEC-KIT-102-notebooklm-integration/` | Tiered architecture; "evolve from fast coding tool to context-aware partner" |
| SPEC-KIT-940 | `docs/archive/specs/SPEC-KIT-940-performance-instrumentation/` | Measurement infrastructure for validating claims |
| SPEC-KIT-977 | `docs/SPEC-KIT-977-*` (active) | PolicySnapshot capture modes |

---

## 7. Research Brief: A2 - Must-Have Interaction Surfaces

### Question
**A2) Must-have interaction surfaces: TUI vs CLI vs Headless/CI vs Parity**

Which interaction surfaces are required:
- **TUI-primary** with CLI as secondary
- **CLI-primary** with TUI as optional GUI
- **Headless/CI-first** with interactive modes as convenience
- **Full parity** across all surfaces

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **TUI** | Text-based User Interface; stateful, takes over terminal, keyboard-driven |
| **CLI** | Command-Line Interface; stateless command-response loop, scriptable |
| **Headless Mode** | Execution without terminal display; for CI/CD and automation |
| **Parity** | Feature equivalence across multiple interfaces |
| **Agentic CLI** | Terminal-based AI agent with autonomous multi-step execution |

### B) Sources (10 total)

#### Authoritative/Primary Sources
1. [Wikipedia: Text-based User Interface](https://en.wikipedia.org/wiki/Text-based_user_interface) - Historical context and definitions
2. [FreeCodeCamp: Essential CLI/TUI Tools](https://www.freecodecamp.org/news/essential-cli-tui-tools-for-developers/) - Developer tool patterns
3. [It's FOSS: GUI, CLI and TUI](https://itsfoss.com/gui-cli-tui/) - Clear taxonomy

#### Security/Privacy-Focused Critiques
4. [Angelo Lima: CI/CD and Headless Mode with Claude Code](https://angelo-lima.fr/en/claude-code-cicd-headless-en/) - Headless mode patterns and security considerations

#### Real-World Implementation References
5. [Qodo: Claude Code vs Cursor](https://www.qodo.ai/blog/claude-code-vs-cursor/) - Architecture comparison; terminal-first vs editor-first
6. [GitHub: awesome-tuis](https://github.com/rothgar/awesome-tuis) - TUI ecosystem survey
7. [Builder.io: Cursor vs Claude Code](https://www.builder.io/blog/cursor-vs-claude-code) - Feature matrix comparison

#### Practitioner/Engineering Blog Posts
8. [Trickster Dev: Back to the Terminal](https://www.trickster.dev/post/back-to-the-terminal-the-new-era-of-cli-and-tui-software/) - TUI renaissance analysis
9. [Medium: From CLI to GUI to TUI](https://medium.com/@chrysophilist/from-cli-to-gui-to-tui-why-developers-are-going-back-to-terminal-c6a27aab1375) - Why developers prefer terminal
10. [Golodiuk: Architecting for Control with CLIs and TUIs](https://www.golodiuk.com/news/ui-in-architecture-01-cli-tui/) - Architectural patterns

### C) Neutral Synthesis

#### What Tends to Work

**TUI Strengths:**
- "Blazing fast, keyboard-driven, hands never leave keys"
- High-density data visualization within resource-efficient terminal
- Stateful: can maintain context across operations
- Examples: vim, htop, Midnight Commander
- Codex-RS TUI: ChatWidget, spec-kit commands, real-time agent status

**CLI Strengths:**
- "Works over SSH, in Docker containers, on headless CI runners, across any OS"
- Scriptable, composable with other tools
- Stateless: each invocation independent
- "Infrastructure, not applications"
- Codex-RS CLI: `code speckit --help`, exit code contracts

**Headless Strengths:**
- CI/CD integration (GitHub Actions, etc.)
- No PTY dependency (SPEC-920 learned this the hard way)
- Parallel agent execution without IDE instances
- "If your editor crashes, the context vanishes" - CLI avoids this

**Parity Benefits:**
- Users can choose preferred interface without losing features
- Teams can mix interactive and automated workflows
- Reduces cognitive overhead for switching contexts

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **TUI-only trap** | Features only work interactively; CI/CD blocked | SPEC-920→921 showed this; CLI adapter required |
| **CLI feature lag** | TUI gets features first; CLI becomes second-class | Shared executor core (SpeckitExecutor pattern) |
| **Parity overhead** | Maintaining 3 surfaces multiplies development cost | Extract shared logic; test at core level |
| **PTY assumptions** | TUI automation via tmux/expect is fragile | Direct API calls without PTY dependency |

#### Engineering Costs/Risks

| Approach | Dev Cost | Maintenance | User Friction |
|----------|----------|-------------|---------------|
| TUI-primary | Low | Low | High for CI/CD users |
| CLI-primary | Low | Low | Higher learning curve |
| Headless-first | Medium | Low | TUI users feel neglected |
| Full parity | High | High | Lowest friction |

#### What Competitor Landscape Shows

From research:
- **Claude Code**: "Terminal-first AI agent" - CLI primary, autonomous operation
- **Cursor/Windsurf**: IDE-first, TUI-like experience in editor
- Both camps have adherents; no single winner

**Level Taxonomy (from research):**
- Level 3 (Supervised): Cursor, Windsurf, Cline - request permission each step
- Level 4 (Autonomous): Claude Code, Aider - multi-step with minimal supervision

Codex-RS's `/speckit.auto` fits Level 4 (autonomous pipeline with human review at stage gates).

#### What's Different for Codex-RS

1. **Already has both TUI and CLI** - SPEC-921 created shared SpeckitExecutor
2. **TUI is richest surface** - ChatWidget, spec-kit commands, real-time status
3. **CLI for automation** - `code speckit` provides headless mode
4. **Documented failure**: SPEC-920 proved PTY-based TUI automation doesn't work
5. **ADR-002**: tui is primary; tui2 is upstream scaffold only

### D) Decision Considerations Checklist

1. [ ] **Primary user workflow**: Are users mostly interactive (TUI) or automated (CI/CD)?
2. [ ] **CI/CD adoption**: What percentage of runs are headless vs interactive?
3. [ ] **SSH/remote use**: How often do users work on headless servers?
4. [ ] **Feature velocity**: Can parity be maintained without slowing releases?
5. [ ] **Testing burden**: How much does parity increase test matrix?
6. [ ] **CLI discoverability**: Can CLI self-document to match TUI's exploration?
7. [ ] **Error handling**: Does CLI provide equivalent error context to TUI?
8. [ ] **State persistence**: How to share state between TUI and CLI sessions?
9. [ ] **Competitor positioning**: Claude Code is CLI-first; differentiate or align?
10. [ ] **Enterprise vs indie**: Enterprises may prefer CI/CD; indies prefer TUI?
11. [ ] **SpeckitExecutor coverage**: Does shared executor cover all spec-kit commands?
12. [ ] **Exit code contract**: Is the CLI exit code semantics fully documented?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-920 | `docs/archive/specs/SPEC-KIT-920-tui-automation/` | Documents why PTY-based TUI automation failed; led to CLI adapter |
| SPEC-KIT-921 | referenced | CLI Adapter + Shared SpeckitExecutor; supersedes SPEC-920 |
| SPEC-KIT-960 | `docs/archive/specs/SPEC-KIT-960-speckit-project/` | CLI-first scaffolding (<1s, Tier 0) |
| SPEC-KIT-970 | `docs/archive/specs/SPEC-KIT-970-interactive-prd/` | TUI-specific interactive PRD builder |
| ADR-002 | `docs/adr/ADR-002-tui2-purpose-and-future.md` | tui vs tui2 distinction |

---

## 8. Research Brief: B1 - Capsule SOR Content

### Question
**B1) Capsule SOR content: Raw-only vs Raw+Derived vs Curated Snapshots vs Everything**

What should the Memvid capsule (system of record) contain:
- **Raw-only**: Only immutable events, derive everything on replay
- **Raw+Derived**: Events plus projections/materialized views
- **Curated snapshots**: Events plus periodic state snapshots for performance
- **Everything**: Full state at every point (maximum fidelity, maximum cost)

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **System of Record (SOR)** | The authoritative source where data value is definitively established; the "golden record" |
| **Event Store** | Append-only log of immutable events representing state changes over time |
| **Projections** | Read models/materialized views derived from events, optimized for queries |
| **Snapshots** | Point-in-time state captures to avoid full event replay; performance optimization |
| **Derived State** | State computed from events; can be rebuilt by replaying the event log |

### B) Sources (11 total)

#### Authoritative/Primary Sources
1. [Microsoft Azure: Event Sourcing Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing) - Enterprise implementation; events as SOR; projections as cache
2. [AWS: Event Sourcing Pattern](https://docs.aws.amazon.com/prescriptive-guidance/latest/cloud-design-patterns/event-sourcing.html) - Log serves as authoritative source of truth
3. [Microservices.io: Event Sourcing](https://microservices.io/patterns/data/event-sourcing.html) - Pattern definition and trade-offs
4. [IBM: System of Record](https://www.ibm.com/think/topics/system-of-record) - SOR definition and governance
5. [Dataconomy: What Is SOR?](https://dataconomy.com/2025/07/02/what-is-a-system-of-record-sor/) - SOR patterns in 2025

#### Security/Privacy-Focused Critiques
6. [Lasso Security: LLM Data Privacy](https://www.lasso.security/blog/llm-data-privacy) - Audit trail challenges; shadow AI risks
7. [Relyance AI: LLM GDPR Compliance](https://www.relyance.ai/blog/llm-gdpr-compliance) - Right of erasure in event stores; retention challenges

#### Real-World Implementation References
8. [Kurrent: Snapshots in Event Sourcing](https://www.kurrent.io/blog/snapshots-in-event-sourcing) - When and how to use snapshots
9. [Marten: Event Store](https://martendb.io/events/) - Practical event store implementation
10. [Kurrent: State vs Event-Based Data Model](https://docs.kurrent.io/getting-started/evaluate/state-vs-event-based-data-model) - Trade-off analysis

#### Practitioner/Engineering Blog Posts
11. [Martin Fowler: Event Sourcing](https://martinfowler.com/eaaDev/EventSourcing.html) - Canonical definition; "SOR can be event logs OR current state"

### C) Neutral Synthesis

#### What Tends to Work

**Raw-Only (Events as SOR):**
- "The events are persisted in an event store that acts as the system of record"
- Complete history preserved; can derive any past or current state
- "While historical events can be replayed to reconstruct current state, the inverse is impossible"
- Minimal storage of redundant data
- Codex-RS already implements: mv2:// URIs are immutable, single-writer model

**Raw+Derived (Projections):**
- "Projections are read models built from event data, optimized for queries"
- Balance between query performance and storage efficiency
- "This opens up interesting possibilities for technology—could be a cache or in-memory"
- SPEC-KIT-102 uses this: Tier 1 (SQLite + Qdrant) provides derived views

**Curated Snapshots:**
- "If streams are large, consider creating snapshots at specific intervals"
- "Snapshots are primarily a performance optimization tool—KISS principle: don't introduce until you encounter performance issues"
- Alternative: "Make streams short-living (e.g., 'complete the books' pattern)"
- Codex-RS has stage-boundary checkpoints already

**Everything (Maximum Fidelity):**
- "We put a bunch of our domain models in the events. That ended up burning us big time down the road"
- Upcasting complexity increases with event richness
- Storage costs scale with fidelity

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Bloated events** | Too much data in each event; painful upcasting | Keep events minimal; essential data only |
| **Snapshot addiction** | Over-reliance on snapshots; lose event benefits | Redesign streams to be short-living |
| **Projection drift** | Derived state becomes source of truth | Clear ownership; projections are disposable |
| **Erasure impossibility** | GDPR right-to-erasure conflicts with immutable events | Crypto-shredding; tombstone events |

#### What's Different for Codex-RS Capsule

1. **Memvid capsule is already declared SOR** (SPEC.md invariant #2)
2. **Capture modes vary content richness**: `none` / `prompts_only` / `full_io`
3. **Single-writer model** simplifies event ordering
4. **Tier 1/Tier 2 split** (SPEC-KIT-102): Tier 1 is derived/cached, Tier 2 (NotebookLM) is synthesis
5. **Librarian jobs** (SPEC-KIT-103) create meta-memories (derived content)
6. **Run branches** isolate exploration; merge at Unlock

#### Key Trade-off Matrix

| Approach | Storage Cost | Replay Latency | Query Latency | Complexity | Erasure Difficulty |
|----------|--------------|----------------|---------------|------------|-------------------|
| Raw-only | Low | High | High | Low | Hard |
| Raw+Derived | Medium | Low | Low | Medium | Medium |
| Curated Snapshots | Medium-High | Very Low | Low | Medium | Hard |
| Everything | Very High | None | None | High | Very Hard |

### D) Decision Considerations Checklist

1. [ ] **Replay frequency**: How often will users time-travel/replay? (affects snapshot need)
2. [ ] **Event stream length**: Expected events per run/SPEC? (short = no snapshots needed)
3. [ ] **Query patterns**: Are derived views needed for hot-path queries?
4. [ ] **Capture mode distribution**: What % of runs use `full_io` vs `prompts_only`?
5. [ ] **GDPR exposure**: Is personal data in the event stream? (erasure concerns)
6. [ ] **Storage budget**: Per-SPEC limit (currently 25-50MB soft/hard)?
7. [ ] **Librarian integration**: Are meta-memories stored in capsule or separate?
8. [ ] **Tier 1 disposability**: Can Tier 1 (local-memory) be fully rebuilt from capsule?
9. [ ] **Cross-domain queries**: Do projections need to span multiple SPECs?
10. [ ] **Snapshot invalidation**: How to handle snapshots when events are corrected?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-102 | `docs/archive/specs/SPEC-KIT-102-notebooklm-integration/` | Tier1/Tier2 split; dynamic context compilation; synthesis cache |
| SPEC-KIT-103 | `docs/archive/specs/SPEC-KIT-103-librarian/` | Meta-memory synthesis; StructuredMemory format; "meta-memories are always importance 9-10" |
| SPEC.md | `codex-rs/SPEC.md` | Invariant #2: "Memvid capsule is SOR"; Invariant #3: "mv2:// URIs immutable" |

---

## 9. Research Brief: B2 - Retention Posture

### Question
**B2) Retention posture: Keep-all vs Tiered Retention vs Strict Budgets vs Operator-Managed**

What retention policy should govern capsule/evidence data:
- **Keep-all**: Never delete; archive everything forever
- **Tiered retention**: Hot/warm/cold/frozen tiers with automated movement
- **Strict budgets**: Hard caps (e.g., 50MB/SPEC) with automatic pruning
- **Operator-managed**: Manual retention; tools provided but no automation

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Data Retention Policy** | Rules defining how long data is kept, when it's archived, and when deleted |
| **Tiered Storage** | Hot (fast/expensive) → Warm → Cold → Archive (slow/cheap) hierarchy |
| **Legal Hold** | Preservation requirement that suspends normal retention/deletion |
| **Storage Limitation** | GDPR principle requiring data kept only as long as necessary |
| **Compaction** | Process of consolidating/pruning storage to reduce footprint |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Microsoft Azure: Blob Access Tiers](https://learn.microsoft.com/en-us/azure/storage/blobs/access-tiers-overview) - Hot/Cool/Cold/Archive tier definitions
2. [Elastic: Data Tiers](https://www.elastic.co/docs/manage-data/lifecycle/data-tiers) - Hot/warm/cold/frozen patterns
3. [Microsoft Purview: Data Lifecycle Management](https://learn.microsoft.com/en-us/purview/data-lifecycle-management) - Enterprise DLM framework
4. [Splunk: Data Lifecycle Management Guide](https://www.splunk.com/en_us/blog/learn/dlm-data-lifecycle-management.html) - DLM best practices

#### Security/Privacy-Focused Critiques
5. [Relyance AI: LLM GDPR Compliance](https://www.relyance.ai/blog/llm-gdpr-compliance) - "Indefinite storage without defined retention policy" is 39% of audit violations
6. [Proofpoint: AI and Data Protection](https://www.proofpoint.com/us/blog/dspm/ai-and-data-protection-strategies-for-llm-compliance-and-risk-mitigation) - Retention schedules critical for compliance
7. [DPC Ireland: AI and Data Protection](https://www.dataprotection.ie/en/dpc-guidance/blogs/AI-LLMs-and-Data-Protection) - Storage limitation principle

#### Real-World Implementation References
8. [JFrog: Software Data Retention Strategy](https://jfrog.com/blog/building-a-software-data-retention-strategy-and-why-you-need-one/) - Developer artifact retention
9. [Nakivo: Storage Tiering Guide](https://www.nakivo.com/blog/storage-tiering/) - Hot/warm/cold implementation
10. [Wasabi: Rethinking Cold Storage](https://wasabi.com/blog/data-management/storage-tiers) - "Only 19% of cloud data is actually cold"

#### Practitioner/Engineering Blog Posts
11. [Acceldata: Data Retention Policy Basics](https://www.acceldata.io/blog/data-retention-policy) - Balancing compliance with operational needs
12. [Secureframe: Data Retention Policy Template](https://secureframe.com/blog/data-retention-policy) - Creating policies that work

### C) Neutral Synthesis

#### What Tends to Work

**Keep-All:**
- Simplest to implement; no deletion logic
- Complete audit trail; maximum forensic capability
- "For compliance mandates (finance, healthcare, public sector), archiving ensures you meet retention periods (7–10 years)"
- Risk: Unbounded growth; storage costs; "indefinite storage" is GDPR violation

**Tiered Retention:**
- "Hot tier: 1-7 days, most powerful nodes with SSD"
- "Warm tier: 30-90 days, high-capacity HDDs"
- "Cold tier: rarely accessed, lower-cost storage"
- "Frozen/Archive: minimum 180 days, order-of-hours retrieval"
- Automated lifecycle rules reduce manual overhead
- "Quarterly reviews ideal for fast-growing or regulated environments"

**Strict Budgets:**
- SPEC-KIT-909 already defines: "50MB hard limit per SPEC"
- "Auto-archive consensus artifacts >30 days old"
- Predictable storage footprint
- Forces pruning decisions; may lose valuable data
- "/speckit.auto aborts with error if SPEC >50MB"

**Operator-Managed:**
- "Test on limited user group before rolling out company-wide"
- "Admins should balance automation with thoughtful policy durations"
- Maximum flexibility; minimum automation risk
- Risk: "Unintended content loss" if operators forget

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Keep-all bloat** | Repository grows unbounded; operations slow | Tiered archival with cleanup thresholds |
| **Premature deletion** | Aggressive pruning loses needed data | Soft limits with warnings before hard enforcement |
| **Cold storage surprise** | "Cold tier looks cheap but retrieval fees add up" | Budget for access patterns, not just storage |
| **Legal hold bypass** | Automated deletion violates preservation order | Hold detection before any deletion |
| **Audit gap** | Deleted data needed for compliance investigation | Manifest/tombstone trail of what was deleted |

#### Engineering Costs/Risks

| Approach | Implementation | Maintenance | Storage Cost | Compliance Risk |
|----------|----------------|-------------|--------------|-----------------|
| Keep-all | Very Low | Very Low | High | Medium (storage limitation) |
| Tiered Retention | High | Medium | Medium | Low |
| Strict Budgets | Medium | Low | Low | Medium (may lose data) |
| Operator-Managed | Low | High | Variable | High (human error) |

#### What's Different for Codex-RS

1. **SPEC-KIT-909 already defines 50MB hard limit** (current policy)
2. **Evidence hierarchy**: consensus artifacts → command telemetry → evidence
3. **Archive structure exists**: `evidence/archive/YYYY-MM/`
4. **Checksums required**: SHA256 for archived artifacts
5. **Compaction spec exists**: SYNC-026 (backlog, not implemented)
6. **Run branches create isolation**: Per-run data lifecycle possible
7. **Capture mode affects volume**: `full_io` >> `prompts_only` >> `none`

#### Recommended Tier Mapping (Codex-RS Context)

| Tier | Codex-RS Equivalent | Retention | Storage |
|------|---------------------|-----------|---------|
| Hot | Active SPEC evidence | Unlimited during active work | Local SSD |
| Warm | Completed SPEC (30-90 days) | 90 days uncompressed | Local |
| Cold | Archived SPEC (>90 days) | Compressed, 180 days | Local archive/ |
| Frozen | Purged | Manifest only; data deleted | Manifest file |

### D) Decision Considerations Checklist

1. [ ] **Regulatory requirements**: Any industry-specific retention mandates?
2. [ ] **Legal hold capability**: Can automation be suspended for litigation?
3. [ ] **Evidence recovery**: Can purged data be reconstructed from capsule?
4. [ ] **Cost per GB**: What's the real storage cost sensitivity?
5. [ ] **Access patterns**: How often is old evidence actually accessed?
6. [ ] **Operator burden**: Can maintainers handle manual retention decisions?
7. [ ] **Capture mode correlation**: Does `full_io` warrant different retention?
8. [ ] **Archive verification**: Are checksums validated on restore?
9. [ ] **Deletion audit trail**: Is there a manifest of what was deleted and when?
10. [ ] **Multi-tier complexity**: Is tiered storage worth the implementation cost?
11. [ ] **SYNC-026 status**: Is retention/compaction implemented or still backlog?
12. [ ] **Per-SPEC vs global**: Same policy for all SPECs or customizable?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-909 | `docs/archive/specs/SPEC-KIT-909-evidence-cleanup-automation/` | 50MB hard limit; auto-archive >30 days; evidence lifecycle |
| SYNC-026 | `docs/archive/specs/SYNC-026-retention-compaction/` | Retention + compaction hardening; backlog status |
| SPEC-KIT-103 | `docs/archive/specs/SPEC-KIT-103-librarian/` | Meta-memory immutability decision; "immutable with periodic regeneration" |

---

## 10. Research Brief: C1 - Capture-Mode Enforcement

### Question
**C1) Capture-mode enforcement: Operator-per-run vs Project-default vs Hard-wired vs Progressive**

How should capture mode be configured and enforced:
- **Operator-per-run**: Each run specifies its capture mode explicitly
- **Project-default**: Repository-level default, overridable per-run
- **Hard-wired**: Compile-time or deployment-time fixed; no runtime override
- **Progressive**: Automatic escalation based on events (e.g., ring buffer on error)

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Configuration Enforcement** | Mechanisms that ensure configuration values are applied and cannot be bypassed |
| **Dynamic vs Static Config** | Static = set at deploy; Dynamic = adjustable at runtime without redeploy |
| **Convention Over Configuration** | Pattern where sensible defaults reduce required explicit configuration |
| **Ring Buffer Pattern** | In-memory circular buffer of verbose logs; flushed on error, discarded otherwise |
| **Progressive Logging** | Automatic increase in log verbosity triggered by anomalies or errors |
| **Parent-Based Sampling** | Child spans inherit sampling decision from parent; ensures trace integrity |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Google SRE Workbook: Configuration Design](https://sre.google/workbook/configuration-design/) - Dynamic defaults; system property-based configuration
2. [Microsoft: Convention Over Configuration](https://learn.microsoft.com/en-us/archive/msdn-magazine/2009/february/patterns-in-practice-convention-over-configuration) - Project defaults with per-class override
3. [OpenTelemetry: Sampling](https://opentelemetry.io/docs/concepts/sampling/) - Default `ParentBased(root=AlwaysOn)`; production sampling patterns

#### Security/Privacy-Focused Critiques
4. [TermsFeed: GDPR and Log Data](https://www.termsfeed.com/blog/gdpr-log-data/) - Data minimization in logging
5. [NXLog: GDPR Compliance](https://nxlog.co/news-and-blog/posts/gdpr-compliance) - Log retention and privacy
6. [Mindgard: AI Security Tools 2026](https://mindgard.ai/blog/best-ai-security-tools-for-llm-and-genai) - Prompt/response data exposure risks

#### Real-World Implementation References
7. [Dash0: Log Levels with OpenTelemetry](https://www.dash0.com/knowledge/log-levels) - Ring buffer pattern; context-aware logging
8. [Last9: OpenTelemetry Configurations](https://last9.io/blog/opentelemetry-configurations-filtering-sampling-enrichment/) - Sampling strategies at scale
9. [liteLLM: Logging](https://docs.litellm.ai/docs/proxy/logging) - `turn_off_message_logging` flag; per-call no-log

#### Practitioner/Engineering Blog Posts
10. [Datalust: Choosing Log Levels](https://blog.datalust.co/choosing-the-right-log-levels) - "Log level should always be an operational task"
11. [Better Stack: Log Levels Explained](https://betterstack.com/community/guides/logging/log-levels-explained/) - Dynamic verbosity adjustment
12. [Is It Observable: Sampling Best Practices](https://isitobservable.io/open-telemetry/traces/trace-sampling-best-practices) - Head vs tail sampling trade-offs

### C) Neutral Synthesis

#### What Tends to Work

**Operator-per-run:**
- Maximum flexibility; each run can specify exact capture needs
- "External configurations allow operators to toggle verbose logging on demand"
- Aligns with GDPR purpose limitation (capture only what's needed for this run)
- Risk: Inconsistency across runs; harder to compare/analyze
- Codex-RS context: CLI flag or env var per invocation

**Project-default:**
- Convention-over-configuration pattern: "set project defaults with ability to override on per-class basis"
- Balances consistency with flexibility
- "Avoid embedding app configs directly in code and use feature toggles"
- Codex-RS already uses this: `model_policy.toml` sets `capture.mode = "prompts_only"`
- Current state: Project default is source of truth; no per-run override mechanism documented

**Hard-wired:**
- "Hardcoded controls don't cut it" for production systems
- Prevents accidental override; simplest mental model
- Risk: Cannot adapt to incident investigation needs
- May be appropriate for compliance-mandated minimum capture
- Use case: Regulatory floor (e.g., "must capture at least prompts for audit")

**Progressive (Ring Buffer Pattern):**
- "Under normal conditions, buffer overwrites itself; if ERROR occurs, buffer is flushed"
- Captures "context leading up to failure without incurring ongoing volume or cost"
- "By limiting both scope and lifetime of verbose logging, teams can capture rich diagnostic detail when it matters most"
- Requires in-memory buffer infrastructure
- Codex-RS would need: event stream monitoring + flush trigger logic

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Config drift** | Different runs use different modes silently | Enforce via PolicySnapshot; warn on deviation |
| **Insufficient context on error** | Low capture mode misses diagnostics | Ring buffer pattern OR escalation capability |
| **Over-capture in production** | `full_io` everywhere causes storage/privacy issues | Default to minimal; explicit escalation |
| **Operator fatigue** | Too many per-run decisions | Strong defaults with rare override |
| **Audit gap** | Capture mode not itself captured | PolicySnapshot includes capture mode |

#### Engineering Costs/Risks

| Approach | Implementation | Override Friction | Consistency | Incident Response |
|----------|----------------|-------------------|-------------|-------------------|
| Operator-per-run | Low | Very Low | Low | High (if remembered) |
| Project-default | Low (already done) | Low-Medium | High | Medium |
| Hard-wired | Very Low | Very High | Very High | Low (inflexible) |
| Progressive | High | N/A (automatic) | Medium | Very High |

#### What's Different for Codex-RS

1. **Already has project-default**: `model_policy.toml` → `capture.mode = "prompts_only"`
2. **PolicySnapshot captures mode**: SPEC-KIT-977 stores policy in capsule per run
3. **Three modes already defined**: `none`, `prompts_only`, `full_io`
4. **Single-writer model**: Simplifies enforcement (one place to check)
5. **Invariant #10 dependency**: "Replay offline-first; LLM I/O depends on capture mode"
6. **No per-run override documented**: Current CLI/TUI doesn't expose capture mode flag

### D) Decision Considerations Checklist

1. [ ] **Override need frequency**: How often do users actually need per-run capture change?
2. [ ] **Compliance floor**: Is there a minimum capture mode required by regulation/policy?
3. [ ] **Incident investigation**: Can current mode support post-mortem debugging?
4. [ ] **Ring buffer feasibility**: Would progressive capture require significant infrastructure?
5. [ ] **PolicySnapshot coverage**: Is capture mode enforcement verified in tests?
6. [ ] **CLI parity**: Should `code speckit --capture-mode=full_io` exist?
7. [ ] **Escalation path**: Can operator temporarily elevate capture without code change?
8. [ ] **Storage impact**: What's the footprint difference between modes per SPEC?
9. [ ] **Redaction interaction**: Does capture mode interact with `security.redaction` rules?
10. [ ] **Audit trail**: Is the capture mode decision itself auditable in capsule timeline?
11. [ ] **Default rationale**: Why `prompts_only` vs alternatives? Is this documented?
12. [ ] **Progressive trigger**: What events would justify automatic capture escalation?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-977 | `docs/SPEC-KIT-977-model-policy-v2/spec.md` | PolicySnapshot stores capture mode; enforcement mechanism |
| SPEC.md | `codex-rs/SPEC.md` | Invariant #10: "LLM I/O depends on capture mode" |
| model_policy.toml | `codex-rs/model_policy.toml` | Current default: `capture.mode = "prompts_only"` |

---

## 11. Research Brief: C2 - Default Capture Mode

### Question
**C2) Default capture mode: none vs prompts_only vs full_io**

What should be the out-of-box default capture mode:
- **none**: Minimal; events only, no LLM I/O captured
- **prompts_only**: Inputs captured, outputs not (current default)
- **full_io**: Complete request/response pairs captured

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Privacy by Default** | GDPR Article 25(2): only necessary data processed by default |
| **Data Minimization** | Collect least amount of data needed for purpose |
| **Audit Trail** | Chronological record enabling reconstruction of past states |
| **Observability** | Ability to understand system behavior from external outputs |
| **PII Redaction** | Automatic removal of personally identifiable information |
| **Prompt Leakage** | Sensitive information in prompts exposed through logging |

### B) Sources (11 total)

#### Authoritative/Primary Sources
1. [SecurePrivacy: Privacy by Design GDPR 2025](https://secureprivacy.ai/blog/privacy-by-design-gdpr-2025) - Article 25 implementation
2. [Usercentrics: Data Minimization](https://usercentrics.com/knowledge-hub/data-minimization/) - GDPR Article 5(1)(c) requirements
3. [Dynatrace: Data Privacy by Design](https://www.dynatrace.com/news/blog/data-privacy-by-design/) - "Strong defaults" for exclusions

#### Security/Privacy-Focused Critiques
4. [Lakera: Prompt Injection Attacks](https://www.lakera.ai/risk/prompt-injection-attacks) - Prompt data leakage risks
5. [Strac: AI Data Security Guide](https://www.strac.io/blog/ai-data-security-5f2f8) - LLM data leakage patterns
6. [Obsidian Security: Prompt Injection 2025](https://www.obsidiansecurity.com/blog/prompt-injection) - OWASP Top 10 for LLM

#### Real-World Implementation References
7. [liteLLM: Logging](https://docs.litellm.ai/docs/proxy/logging) - `turn_off_message_logging`; frontend rarely logs full payloads
8. [Last9: GDPR Log Management](https://last9.io/blog/gdpr-log-management/) - Default minimization strategy
9. [Observo.ai: Data Privacy in Observability](https://www.observo.ai/post/data-privacy-confidentiality-observability) - Strong defaults for exclusions

#### Practitioner/Engineering Blog Posts
10. [CookieYes: GDPR Logging Best Practices](https://www.cookieyes.com/blog/gdpr-logging-and-monitoring/) - Purpose specification; lifecycle planning
11. [Medium: LLM Monitoring Guide](https://medium.com/@amit25173/what-is-llm-monitoring-complete-guide-685baf336423) - Compliance vs operational trade-offs

### C) Neutral Synthesis

#### What Tends to Work

**none (Minimal):**
- Maximum privacy compliance; minimum storage
- "Employ no-data architecture principles"
- Sufficient for: routine runs, development, low-stakes work
- Risk: No forensic capability on failure; replay loses LLM context
- Trade-off: "If an ERROR occurs without capture, you lose context"

**prompts_only (Current Default):**
- Captures intent without response volume
- "At the frontend/consumer-facing layer, practitioners rarely log full payloads due to privacy risk"
- Enables: partial replay, intent reconstruction, debugging context
- Risk: Prompt leakage - "user prompts often contain proprietary context, customer data"
- GDPR compatible if prompts don't contain PII (or are redacted)
- Current Codex-RS rationale: Balance between audit and privacy

**full_io (Complete):**
- Maximum forensic capability; complete audit trail
- Required for: regulated industries, compliance mandates, incident investigation
- "Prompts and responses are captured... for audit trail"
- Risk: "AI data leakage is often irreversible"
- Storage impact: 10-100x larger than `prompts_only`
- May require PII redaction pipeline

#### GDPR Alignment Analysis

| Mode | Data Minimization | Purpose Limitation | Storage Limitation | Privacy by Default |
|------|-------------------|-------------------|-------------------|-------------------|
| none | ✅ Excellent | ✅ N/A | ✅ Minimal | ✅ Best |
| prompts_only | ⚠️ Good | ✅ Intent audit | ⚠️ Medium | ✅ Good |
| full_io | ❌ Poor | ⚠️ Full audit | ❌ High | ❌ Poor |

#### Security Risk Analysis

| Mode | Prompt Leakage | Response Leakage | Exfiltration Target | Redaction Need |
|------|----------------|------------------|---------------------|----------------|
| none | None | None | Low | None |
| prompts_only | Medium | None | Medium | Medium |
| full_io | High | High | High | High |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Insufficient forensics** | `none` mode loses debug context | Ring buffer or escalation path |
| **Prompt data exposure** | `prompts_only` captures secrets | Redaction pipeline (already in model_policy.toml) |
| **Storage explosion** | `full_io` exceeds SPEC budget | Tiered retention; compression |
| **GDPR violation** | PII captured without consent | Default redaction; consent tracking |
| **Replay inconsistency** | Different modes produce different replay | Document mode-specific replay behavior |

#### Engineering Costs/Risks

| Mode | Storage Cost | Privacy Risk | Forensic Value | Implementation |
|------|--------------|--------------|----------------|----------------|
| none | Very Low | Very Low | Low | Done |
| prompts_only | Low | Medium | Medium | Done (current) |
| full_io | High | High | High | Done |

#### What's Different for Codex-RS

1. **Current default is `prompts_only`**: Already implemented in `model_policy.toml`
2. **Redaction rules exist**: `security.redaction` patterns for secrets
3. **Invariant #10 specifies**: "LLM I/O depends on capture mode"
4. **SPEC budget constraint**: 25MB soft/50MB hard per SPEC
5. **Offline replay use case**: `prompts_only` enables partial replay; `full_io` enables exact
6. **Single-writer simplifies**: One place for capture mode enforcement

#### Default Selection Framework

| If Primary Use Case Is... | Recommended Default | Rationale |
|---------------------------|---------------------|-----------|
| Privacy-first product | none | GDPR compliance; trust boundary |
| Developer productivity | prompts_only | Debug context without response volume |
| Regulated/enterprise | full_io | Audit trail mandates |
| Offline-first replay | full_io | Exact reconstruction |
| Cost-conscious | none or prompts_only | Storage efficiency |

### D) Decision Considerations Checklist

1. [ ] **Regulatory environment**: Are users in regulated industries requiring full audit?
2. [ ] **Privacy stance**: Is Codex-RS positioned as privacy-first or audit-first?
3. [ ] **Replay importance**: How critical is exact LLM response reproduction?
4. [ ] **Storage budget**: Does 25MB/SPEC accommodate full_io for typical runs?
5. [ ] **Redaction coverage**: Are current redaction patterns sufficient for prompts?
6. [ ] **Response sensitivity**: Do LLM responses contain more sensitive data than prompts?
7. [ ] **User override**: Should users be able to elevate beyond default?
8. [ ] **Operator override**: Should operators be able to reduce below default?
9. [ ] **Per-SPEC variation**: Should different SPECs have different defaults?
10. [ ] **Documentation**: Is the default rationale documented in MODEL-POLICY.md?
11. [ ] **Consent tracking**: Is explicit consent needed for `full_io`?
12. [ ] **A/B evaluation**: Can capture mode impact on behavior be measured?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| model_policy.toml | `codex-rs/model_policy.toml` | Lines 66-76: `mode = "prompts_only"`; mode definitions |
| docs/MODEL-POLICY.md | `docs/MODEL-POLICY.md` | Human-readable rationale (referenced) |
| SPEC-KIT-977 | `docs/SPEC-KIT-977-model-policy-v2/spec.md` | PolicySnapshot captures mode |

---

## 12. Research Brief: D1 - Pipeline Modularity

### Question
**D1) Pipeline modularity: Monolith vs Plugin-per-stage vs Trait-based Injection vs Full Actor Model**

What architectural pattern should govern the spec-kit pipeline:
- **Monolith**: Single codebase, all stages in one binary
- **Plugin-per-stage**: Dynamic loading, stages as separate libraries
- **Trait-based Injection**: Compile-time polymorphism via Rust traits
- **Full Actor Model**: Message-passing concurrency (SEDA pattern)

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Pipes and Filters** | Architecture where processing is decomposed into independent filters connected by pipes (data channels) |
| **Plugin Architecture** | System designed to load and execute external modules at runtime via dynamic linking |
| **Trait Object** | Rust's runtime polymorphism mechanism using vtable dispatch (`dyn Trait`) |
| **SEDA** | Staged Event-Driven Architecture; decomposes applications into stages connected by queues |
| **Modular Monolith** | Monolithic deployment with internal module boundaries enforced at compile time |
| **Dependency Injection** | Pattern where dependencies are provided externally rather than constructed internally |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Microsoft Azure: Pipes and Filters Pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/pipes-and-filters) - Enterprise pipeline pattern; independent filters, stateless design
2. [Index.dev: Software Architecture Patterns 2025](https://www.index.dev/blog/software-architecture-patterns-guide) - Modern pattern taxonomy and selection criteria
3. [Wikipedia: Staged Event-Driven Architecture](https://en.wikipedia.org/wiki/Staged_event-driven_architecture) - SEDA definition; admission control per queue

#### Security/Privacy-Focused Critiques
4. [Michael F. Bryan: Plugins in Rust](https://adventures.michaelfbryan.com/posts/plugins-in-rust/) - "FFI approaches prioritize performance over safety"; vtable lifetime challenges
5. [Medium: The Sealed Trait Pattern](https://medium.com/@bugsybits/the-sealed-trait-pattern-controlling-extensibility-in-rust-9b9b206f8c22) - Controlling extensibility; preventing external implementations
6. [Rust RFC 0445: Extension Trait Conventions](https://rust-lang.github.io/rfcs/0445-extension-trait-conventions.html) - Object safety rules; trait splitting patterns

#### Real-World Implementation References
7. [Shopify Case Study via Rubyroid Labs](https://rubyroidlabs.com/blog/2025/04/microservices-vs-monolith/) - "Restructured to modular monolith for better maintenance"
8. [thin_trait_object Crate](https://docs.rs/thin_trait_object/latest/thin_trait_object/) - Plugin systems via trait objects without vtable overhead
9. [Spring Modulith via Java Code Geeks](https://www.javacodegeeks.com/2025/12/microservices-vs-modular-monoliths-in-2025-when-each-approach-wins.html) - "Enforces module boundaries at compile time"

#### Practitioner/Engineering Blog Posts
10. [Airspeed Consulting: When to Use Pipeline Architecture](https://airspeed.ca/when-to-use-the-pipeline-architecture-style/) - "Normally implemented as monolith; easy to understand"
11. [Foojay: Monolith vs Microservices 2025](https://foojay.io/today/monolith-vs-microservices-2025/) - "Many enterprises now return to modular monoliths"
12. [Will Crichton: Types Over Strings](https://willcrichton.net/notes/types-over-strings/) - TypeId for runtime type safety; extensible architectures
13. [Medium: Pawel Piwosz - Real Cloud Migration Costs](https://medium.com/@pawel.piwosz/monolith-vs-microservices-2025-real-cloud-migration-costs-and-hidden-challenges-8b453a3c71ec) - "Beware of the distributed monolith"

### C) Neutral Synthesis

#### What Tends to Work

**Monolith:**
- "Relatively easy to understand and build" - low operational overhead
- "Don't let trend articles pressure you into complexity you don't need"
- Single deployment, simple debugging, no version drift
- Codex-RS context: Already implements this pattern via `StageType` enum

**Plugin-per-stage:**
- Maximum extensibility; third-party stage contributions possible
- Rust challenges: "Binary compatibility across different compiler versions"
- "A trait object's vtable is embedded in the library's code" - plugin lifetime issues
- Requires careful FFI design or WASI isolation

**Trait-based Injection:**
- Compile-time safety, zero runtime cost
- "Types are better identifiers than strings" - TypeId pattern
- Extension traits for object safety when needed
- Cannot add new stages at runtime without recompilation

**Full Actor Model (SEDA):**
- "Decomposes complex application into stages connected by queues"
- "Avoids high overhead associated with thread-based concurrency"
- "Admission control on each event queue" for back-pressure
- "Enables modularity and code reuse" + debugging tools

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Monolith scaling** | "The monolithic nature limits horizontal scaling of individual components" | Accept for Codex-RS's use case (single-user workbench) |
| **Plugin version drift** | Different plugins compiled with different Rust versions | WASI isolation or strict version pinning |
| **Trait object safety** | Some trait patterns not object-safe | Sealed trait pattern; extension trait splitting |
| **Actor coordination** | "Microservices bring high coordination, deployment, and security costs" | Accept simpler model for fixed stage count |
| **Distributed monolith** | "All the microservice headaches, none of the benefits" | Avoid halfway solutions |

#### Engineering Costs/Risks

| Approach | Dev Cost | Maintenance | Extensibility | Runtime Overhead |
|----------|----------|-------------|---------------|------------------|
| Monolith | Very Low | Very Low | Low | None |
| Plugin-per-stage | High | High | High | Medium (dynamic loading) |
| Trait-based | Low | Low | Medium | None |
| Actor Model | High | Medium | High | Medium (queues, async) |

#### What's Different for Codex-RS

1. **Already has enum-based design**: `StageType` enum with impl methods (8 stages)
2. **Fixed stage count**: New/Specify/Plan/Tasks/Implement/Validate/Audit/Unlock - extensibility less critical
3. **Single-writer model**: Eliminates actor coordination concerns
4. **3-tier config precedence**: CLI > per-SPEC > global provides flexibility without plugin complexity
5. **Quality gates at boundaries**: PrePlanning, PostPlan, PostTasks already integrated
6. **Pattern matching dispatch**: Stage behavior via match statements keeps logic centralized
7. **SPEC-948 implemented**: `pipeline_config.rs` has validation, dependencies, skip conditions

### D) Decision Considerations Checklist

1. [ ] **Stage extensibility need**: Will third parties need to add new stages?
2. [ ] **Stage count stability**: Is 8 stages the final count or will more be added?
3. [ ] **Runtime configurability**: Is config-time flexibility sufficient vs runtime flexibility?
4. [ ] **Testing isolation**: Can stages be unit tested independently with current design?
5. [ ] **Debugging complexity**: Is current pattern-matching dispatch debuggable?
6. [ ] **Performance profile**: Does stage execution need horizontal scaling?
7. [ ] **Binary size**: Would plugin loading reduce binary size significantly?
8. [ ] **Security posture**: Would WASI isolation improve security for untrusted stages?
9. [ ] **Upgrade path**: Can current design evolve to plugins if needed later?
10. [ ] **Team familiarity**: Is the team comfortable with async/actor patterns?
11. [ ] **Dependency graph**: Is current hard/soft dependency validation sufficient?
12. [ ] **SPEC-948 status**: Is the current implementation meeting requirements?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-948 | `docs/SPEC-948-modular-pipeline-logic/spec.md` | Full modularity research; 3-tier config precedence; dependency graph |
| SPEC-947 | Referenced | Pipeline UI Configurator; visual stage selection |
| SPEC-KIT-931 | `docs/archive/specs/SPEC-KIT-931-architectural-deep-dive/` | Actor Model NO-GO decision; architectural trade-offs |

---

## 13. Research Brief: D2 - Quality Gates Posture

### Question
**D2) Quality gates posture: Warn-only vs Blocking-with-override vs Hard-fail vs Progressive Escalation**

What enforcement posture should quality gates take:
- **Warn-only**: Non-blocking warnings, developer discretion
- **Blocking-with-override**: Block by default, explicit bypass available
- **Hard-fail**: No exceptions, gate failure stops pipeline
- **Progressive Escalation**: Start permissive, tighten over time

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Quality Gate** | Checkpoint that code must pass before advancing to the next phase |
| **Blocking Gate** | Gate that halts pipeline progression until criteria are met |
| **Advisory Signal** | Non-blocking feedback that suggests improvements without stopping work |
| **Counter-Signal** | Evidence that contradicts or challenges a proposed action |
| **False Positive** | Gate failure that incorrectly flags acceptable code |
| **Shift-Left** | Practice of catching defects earlier in the development lifecycle |
| **Escalation Target** | Entity (Human, JudgeRole, etc.) that receives elevated decisions |

### B) Sources (11 total)

#### Authoritative/Primary Sources
1. [SonarSource: Integrating Quality Gates into CI/CD](https://www.sonarsource.com/resources/library/integrating-quality-gates-ci-cd-pipeline/) - "Blocking releases forces developers to fix newly introduced issues"
2. [Perforce: What Are Quality Gates?](https://www.perforce.com/blog/sca/what-quality-gates) - Gate types: Warn/Fail outcomes; criteria configuration
3. [InfoQ: Pipeline Quality Gates](https://www.infoq.com/articles/pipeline-quality-gates/) - Shift-left benefits; 10X cost increase per phase

#### Security/Privacy-Focused Critiques
4. [ARMO: Securing CI/CD Pipelines Through Security Gates](https://www.armosec.io/blog/securing-ci-cd-pipelines-security-gates/) - Security gate integration; vulnerability scanning
5. [testRigor: Software Quality Gates](https://testrigor.com/blog/software-quality-gates/) - "Rule-Based Static Analysis generated overwhelming false positive rates"
6. [Mindgard: AI Security Tools 2026](https://mindgard.ai/blog/best-ai-security-tools-for-llm-and-genai) - "Overreliance on AI cybersecurity can introduce vulnerabilities"

#### Real-World Implementation References
7. [Datadog: Quality Gates](https://docs.datadoghq.com/pr_gates/) - PR gates; green/red status checks; non-blocking warnings as comments
8. [OverOps: CI/CD Pipeline Quality Gates](https://doc.overops.com/docs/cicd-pipeline-quality-gates) - New Errors, Increasing Error Rate, Slowdowns metrics
9. [Augment Code: Autonomous Quality Gates](https://www.augmentcode.com/guides/autonomous-quality-gates-ai-powered-code-review) - "AI-powered detection improves accuracy through feedback loops"

#### Practitioner/Engineering Blog Posts
10. [DZone: DevOps Pipeline Quality Gates - A Double-Edged Sword](https://dzone.com/articles/devops-pipeline-quality-gates-a-double-edged-sword) - Trade-offs; false positive management
11. [Medium: Quality Gates - The Watchers of Software Quality](https://medium.com/@dneprokos/quality-gates-the-watchers-of-software-quality-af19b177e5d1) - Gate configuration strategies; gradual adoption

### C) Neutral Synthesis

#### What Tends to Work

**Warn-only:**
- "For gradual adoption, keep gates non-blocking until false-positive rates stabilize"
- Enables adoption without workflow disruption
- "Exemption lists for untouched legacy modules" as transitional strategy
- Risk: "Warning fatigue" - developers "trained to ignore warnings"

**Blocking-with-override:**
- "Quality gate blocks substandard code from deployment"
- "Green for merge-safe code, red for required fixes" - clear feedback
- Emergency bypass preserves flexibility for urgent hotfixes
- Industry standard: SonarQube, Datadog PR Gates

**Hard-fail:**
- Maximum enforcement; no exceptions possible
- "Blocking releases forces developers to fix newly introduced issues"
- Risk: "False positives block legitimate code"
- Appropriate for: Critical security vulnerabilities, compliance requirements

**Progressive Escalation:**
- Start non-blocking, analyze patterns, tighten thresholds
- "AI-powered systems continuously improve detection accuracy"
- "Warning-only initial settings, then tightened thresholds after first sprints"
- Complexity: Threshold management, baseline drift

#### Current Codex-RS Implementation

From codebase exploration:

| Component | Current Behavior |
|-----------|------------------|
| `SignalSeverity` | `Advisory` (auto-apply eligible) vs `Block` (always escalate) |
| Auto-apply threshold | Confidence >= 0.65 for advisory signals |
| Tool-truth failures | Always escalate (compiler, tests, lint) |
| GR-001 | Single-owner pipeline; multi-agent consensus disabled |
| Escalation targets | Human, JudgeRole, ImplementerFallback, LibrarianSweep |
| Gate criteria | `model_policy.toml` defines reflex_promotion, local_memory_sunset |

**Effective Posture**: Hybrid blocking + warning with escalation.

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Warning fatigue** | Too many warnings; developers ignore all | Prioritize; show only actionable issues |
| **False positive block** | Legitimate code blocked by incorrect detection | Override mechanism + feedback loop |
| **Threshold rigidity** | Hard-coded thresholds don't fit all projects | Configurable per-SPEC thresholds |
| **Escalation bottleneck** | All failures route to single human | Multiple escalation targets; auto-triage |
| **Detection gaps** | "Static analysis detects only 60-70% of critical vulnerabilities" | Layered detection; multiple tools |

#### Engineering Costs/Risks

| Approach | Implementation | Workflow Impact | False Positive Risk | Adoption Friction |
|----------|----------------|-----------------|---------------------|-------------------|
| Warn-only | Very Low | Very Low | Low (ignored) | Very Low |
| Blocking-with-override | Medium | Medium | Medium | Medium |
| Hard-fail | Low | High | High | High |
| Progressive | High | Variable | Managed | Low-Medium |

#### What's Different for Codex-RS

1. **Already implements hybrid**: `SignalSeverity::Advisory` vs `SignalSeverity::Block`
2. **Confidence-based auto-apply**: >= 0.65 threshold for advisory signals
3. **Tool-truth primacy**: Compiler/tests/lint failures always escalate (never auto-apply)
4. **GR-001 enforced**: Single-owner pipeline eliminates voting complexity
5. **Defined escalation graph**: Human → JudgeRole → ImplementerFallback → LibrarianSweep
6. **Gate criteria in TOML**: `model_policy.toml` gates section is machine-authoritative
7. **PolicySnapshot binding**: Every stage transition captures policy state
8. **SPEC-KIT-977 complete**: PolicySnapshot storage, dual filesystem + capsule

### D) Decision Considerations Checklist

1. [ ] **False positive rate**: What is the current rate? Is it acceptable?
2. [ ] **Override audit trail**: Are bypasses logged and reviewable?
3. [ ] **Escalation latency**: How long does human escalation take?
4. [ ] **Threshold configuration**: Are thresholds adjustable per-SPEC?
5. [ ] **Progressive roadmap**: Is there a plan to tighten thresholds over time?
6. [ ] **Tool-truth coverage**: Which tools have hard-fail authority?
7. [ ] **Advisory value**: Are advisory signals actually acted upon?
8. [ ] **Emergency bypass**: Is there a documented emergency procedure?
9. [ ] **Compliance requirements**: Any regulations requiring hard-fail gates?
10. [ ] **Feedback loop**: Does the gate system learn from false positives?
11. [ ] **Gate cost**: What is the time overhead of gate execution?
12. [ ] **SPEC-KIT-941 status**: Is automated policy compliance ready?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-977 | `docs/SPEC-KIT-977-model-policy-v2/spec.md` | PolicySnapshot storage; gate criteria schema |
| SPEC-KIT-941 | `docs/SPEC-KIT-941-automated-policy-compliance/PRD.md` | Automated compliance; violation detection |
| model_policy.toml | `codex-rs/model_policy.toml` | Gate criteria: reflex_promotion, local_memory_sunset |
| gate_policy.rs | `codex-rs/spec-kit/src/gate_policy.rs` | SignalSeverity, CounterSignalKind, Verdict types |

---

## 14. Research Brief: E1 - Enforcement Strictness

### Question
**E1) Enforcement strictness: Soft-fail-everywhere vs Hard-fail-core-only vs Hard-fail-all vs Graduated-by-stage**

What enforcement posture should quality gates adopt across the pipeline:
- **Soft-fail-everywhere**: All gates warn, developer discretion to proceed
- **Hard-fail-core-only**: Compile/tests/critical-security hard-fail, all others soft
- **Hard-fail-all**: Every gate blocks, no bypass mechanism
- **Graduated-by-stage**: Early stages permissive, late stages strict

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Soft Gate** | Quality check that flags issues as warnings without blocking progression |
| **Hard Gate** | Quality check that halts pipeline until criteria are met |
| **Graduated Enforcement** | Increasing strictness as code matures through pipeline stages |
| **Zero Tolerance** | Policy mandating no critical/high-severity issues may proceed |
| **Control Gate** | NIST term for verification checkpoint in secure development |
| **Phase-Gate Process** | Stage-by-stage progression with gates between phases |

### B) Sources (14 total)

#### Authoritative/Primary Sources
1. [InfoQ: The Importance of Pipeline Quality Gates](https://www.infoq.com/articles/pipeline-quality-gates/) - Shift-left benefits; 10X cost increase per phase; gate implementation
2. [SonarSource: Integrating Quality Gates into CI/CD](https://www.sonarsource.com/learn/integrating-quality-gates-ci-cd-pipeline/) - Blocking releases forces fix; CaYC methodology
3. [OWASP CI/CD Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/CI_CD_Security_Cheat_Sheet.html) - Security gates as mandatory checkpoints
4. [NIST SP 800-218 (SSDF)](https://csrc.nist.gov/projects/ssdf) - Control gates for secure development; federal requirements
5. [OWASP Top 10 CI/CD Security Risks](https://owasp.org/www-project-top-10-ci-cd-security-risks/) - Flow control mechanisms; CICD-SEC-01

#### Security/Compliance-Focused Critiques
6. [ARMO: Securing CI/CD Pipelines Through Security Gates](https://www.armosec.io/blog/securing-ci-cd-pipelines-security-gates/) - Zero tolerance for critical/high; status checks mandate completion
7. [Wiz: DevSecOps Pipeline Best Practices](https://www.wiz.io/academy/devsecops-pipeline-best-practices) - Soft gates for dev, hard gates for release candidates
8. [DZone: DevOps Pipeline Quality Gates - A Double-Edged Sword](https://dzone.com/articles/devops-pipeline-quality-gates-a-double-edged-sword) - False positive management; over-strict thresholds

#### Real-World Implementation References
9. [Wipfli: SOC 2 Audit Checklist for CI/CD](https://www.wipfli.com/insights/articles/ra-audit-ci-cd-as-part-of-your-soc-exam) - Gated builds required; change management controls
10. [Copado: How DevOps Quality Gates Improve Deployments](https://www.copado.com/resources/blog/how-devops-quality-gates-improve-deployments-cddd) - Manual override for urgent issues; balance rigor with pragmatism

#### Practitioner/Engineering Blog Posts
11. [ITONICS: Modern Phase Gate Process](https://www.itonics-innovation.com/blog/phase-gate-process-for-ideation) - "Early phases permissive, late phases demanding"
12. [testRigor: Software Quality Gates](https://testrigor.com/blog/software-quality-gates/) - False positive rates; gradual threshold tightening
13. [Medium: DevSecOps Beyond Tools](https://medium.com/devsecops-ai/devsecops-beyond-tools-how-compliance-can-break-your-pipeline-0fc282cb05cb) - Contextualize security findings; severity-based enforcement
14. [ZetCode: Quality Gate Tutorial](https://zetcode.com/terms-testing/quality-gate/) - Phase-specific requirements at appropriate stages

### C) Neutral Synthesis

#### What Tends to Work

**Soft-fail-everywhere:**
- "For gradual adoption, keep gates non-blocking until false-positive rates stabilize"
- Enables team onboarding without workflow disruption
- Risk: "Warning fatigue" - developers "trained to ignore warnings"
- Appropriate for: Legacy codebases, immature tooling, gradual rollout

**Hard-fail-core-only:**
- "Enforce fail-the-build policies for high-severity vulnerabilities"
- "Configure pipeline to fail only on new critical findings; medium and low pass as advisories"
- Industry consensus: Compilation, tests, and critical security should always block
- Provides "reliable signals teams actually trust" while avoiding noise

**Hard-fail-all:**
- "Zero tolerance policy for all critical and high-severity vulnerabilities"
- "Blocking releases forces developers to fix newly introduced issues"
- Risk: "Over-strict thresholds frustrate developers and slow delivery"
- Requires: Mature tooling with low false-positive rates

**Graduated-by-stage:**
- "Early phases should be permissive. Late phases should be demanding"
- "Applying strict predefined criteria too early kills weak signals"
- "Each phase must demand stronger evidence than the last"
- Matches natural maturity progression: dev → test → staging → production

#### Progressive Discipline Pattern

| Stage | Enforcement | Rationale |
|-------|-------------|-----------|
| Development | Soft (warnings) | Allow exploration; catch issues early |
| PR/Merge | Medium (critical blocks) | Prevent obviously broken code from merging |
| CI Build | Hard (compile/test) | Ensure baseline functionality |
| Pre-Release | Hard (security + quality) | Gate production access |
| Production | Hard (all gates) | Zero tolerance for production |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Warning fatigue** | Too many soft warnings; developers ignore all | Prioritize; show only actionable issues |
| **Over-strict early gates** | "Kills weak signals before they can be understood" | Progressive thresholds by stage |
| **False positive blocks** | Legitimate code blocked by incorrect detection | Feedback loop; exception lists for legacy |
| **Threshold drift** | Gates become irrelevant as codebase evolves | "Quarterly re-baselines catching drift" |
| **Bypass normalization** | Emergency overrides become routine | Audit trail; require justification |

#### Engineering Costs/Risks

| Approach | Implementation | Developer Friction | False Positive Risk | Adoption Path |
|----------|----------------|-------------------|---------------------|---------------|
| Soft-fail-everywhere | Very Low | Very Low | Low (ignored) | Easy |
| Hard-fail-core-only | Low | Low-Medium | Medium (in core) | Gradual |
| Hard-fail-all | Low | High | High | Difficult |
| Graduated-by-stage | Medium | Low | Managed | Recommended |

#### What's Different for Codex-RS

1. **Already implements hybrid**: `SignalSeverity::Advisory` vs `SignalSeverity::Block`
2. **Tool-truth primacy**: Compiler/tests/lint always escalate (never auto-apply)
3. **Confidence-based auto-apply**: >= 0.65 threshold for advisory signals
4. **Single-owner pipeline (GR-001)**: Eliminates voting; deterministic escalation
5. **Stage-based architecture**: 8 stages with defined boundaries
6. **Defined escalation graph**: Human → JudgeRole → ImplementerFallback → LibrarianSweep
7. **PolicySnapshot binding**: Enforcement state captured per stage transition

#### Recommendation Alignment

The codebase already implements **Hard-fail-core-only** with graduated characteristics:
- Tool-truth (compile, test, lint): Always block
- Block-severity signals: Always escalate
- Advisory signals: Auto-apply if confidence >= 0.65
- Stage progression: Each stage has harder gates than previous

### D) Decision Considerations Checklist

1. [ ] **Current false positive rate**: What percentage of blocks are incorrect?
2. [ ] **Stage maturity**: Are all 8 stages at same enforcement maturity?
3. [ ] **Legacy code handling**: Are there exemption lists for untouched modules?
4. [ ] **Threshold configurability**: Can per-SPEC thresholds override defaults?
5. [ ] **Graduation roadmap**: Is there a plan to tighten thresholds over time?
6. [ ] **Override audit trail**: Are bypasses logged and reviewable?
7. [ ] **Emergency procedures**: Is there a documented hotfix bypass path?
8. [ ] **Re-baseline cadence**: How often are thresholds reviewed and adjusted?
9. [ ] **Developer feedback loop**: Can developers report false positives?
10. [ ] **Compliance alignment**: Does current posture satisfy SOC2/NIST requirements?
11. [ ] **Stage-specific rules**: Should Specify stage be softer than Unlock stage?
12. [ ] **CI vs TUI parity**: Same enforcement in headless and interactive modes?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| gate_policy.rs | `codex-rs/spec-kit/src/gate_policy.rs` | SignalSeverity::Advisory vs Block; ToolTruthKind enum |
| model_policy.toml | `codex-rs/model_policy.toml` | Current gate criteria configuration |
| SPEC-KIT-941 | `docs/SPEC-KIT-941-automated-policy-compliance/PRD.md` | Automated blocking violations in CI |
| SPEC-KIT-977 | `docs/SPEC-KIT-977-model-policy-v2/spec.md` | PolicySnapshot storage; enforcement binding |

---

## 15. Research Brief: E2 - Hard-fail Policy Set

### Question
**E2) Hard-fail policy set: Which specific gates should have no bypass?**

Which gates should enforce zero-tolerance blocking with no override mechanism:
- Compilation failures
- Test failures (unit, integration)
- Critical security vulnerabilities
- Lint/format violations
- Policy violations (GR-* guardrails)
- Type check failures
- Schema validation failures

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Zero Tolerance Gate** | Gate that blocks pipeline with no bypass or override possible |
| **Tool-Truth** | Deterministic output from non-LLM tools (compiler, tests, linters) |
| **Critical Severity** | Highest risk classification; immediate action required |
| **SAST** | Static Application Security Testing; analyzes source code without execution |
| **DAST** | Dynamic Application Security Testing; tests running application |
| **SCA** | Software Composition Analysis; scans dependencies for vulnerabilities |
| **CaYC** | Clean as You Code; focus on new code quality |

### B) Sources (13 total)

#### Authoritative/Primary Sources
1. [OWASP: CICD-SEC-01 Insufficient Flow Control](https://owasp.org/www-project-top-10-ci-cd-security-risks/CICD-SEC-01-Insufficient-Flow-Control-Mechanisms) - Proper flow control prevents supply chain attacks
2. [NIST SP 800-53](https://www.nist.gov/news-events/news/2025/08/nist-revises-security-and-privacy-control-catalog-improve-software-update) - SA-11 Developer Testing; CM-3 Change Control; SI-7 Integrity
3. [CISA: NIST SSDF Recommendations](https://www.cisa.gov/resources-tools/resources/nist-sp-800-218-secure-software-development-framework-v11-recommendations-mitigating-risk-software) - Mandatory for federal software producers

#### Security/Compliance-Focused Critiques
4. [42Crunch: Security Quality Gates](https://docs.42crunch.com/latest/content/concepts/security_quality_gates.htm) - API security SQGs; automatic build failure on SQG failure
5. [Conviso: Security Gate](https://docs.convisoappsec.com/cli/security-gate/) - Vulnerability policies by severity; automatic CI/CD blocking
6. [OX Security: SAST Tools Integration](https://www.ox.security/blog/static-application-security-sast-tools/) - Policy gates auto-block PRs based on severity

#### Real-World Implementation References
7. [SonarQube: CaYC Quality Gate](https://www.sonarsource.com/resources/library/quality-gate/) - CaYC fails if: issues > 0, rating < A, or duplicates > threshold
8. [Testkube: Quality Gates Glossary](https://testkube.io/glossary/quality-gates) - Pipeline halts automatically on failure; immediate feedback loop
9. [GitLab CI: allow_failure](https://www.tutorialpedia.org/blog/gitlab-ci-conditional-allow-failure/) - Default false = job failure stops pipeline; conditional per-branch

#### Practitioner/Engineering Blog Posts
10. [Jit.io: Integrating SAST into CI/CD](https://www.jit.io/resources/app-security/integrating-sast-into-your-cicd-pipeline-a-step-by-step-guide) - Block critical, warn medium, log low
11. [Rapid7: Quality Gates for Security Checks](https://www.rapid7.com/blog/post/2018/04/12/how-devops-can-use-quality-gates-for-security-checks/) - Fail build if >5 high severity issues
12. [Undo: What To Do About Failing Tests](https://undo.io/resources/what-to-do-about-failing-tests/) - "Zero-tolerance of failing tests" required for continuous delivery
13. [Medium: CI/CD Security Implementation](https://medium.com/@chandan.chanddu/security-in-ci-cd-how-to-implement-devsecops-without-slowing-down-delivery-8e260990cbbb) - Quality gate prevents merging until conditions met

### C) Neutral Synthesis

#### Recommended Hard-Fail Gate Classification

| Gate Category | Hard-Fail? | Rationale | Industry Consensus |
|---------------|------------|-----------|-------------------|
| **Compilation Errors** | YES | Cannot execute broken code; no false positives | Universal |
| **Unit Test Failures** | YES | "Zero-tolerance of failing tests" for CD | Strong |
| **Integration Test Failures** | YES | Prevents broken integrations reaching production | Strong |
| **Critical Security (CVSS 9.0+)** | YES | "Zero tolerance for critical vulnerabilities" | Strong |
| **High Security (CVSS 7.0-8.9)** | Configurable | Block by default; override with justification | Moderate |
| **Type Check Failures** | YES | TypeScript/Rust: type errors are compile errors | Language-dependent |
| **Schema Validation** | YES | Invalid data structures break contracts | Strong |
| **Lint Critical** | Configurable | Some rules are style, some are bugs | Varies by rule |
| **Format Violations** | NO | Style preference; auto-fixable | Weak |
| **Medium Security (CVSS 4.0-6.9)** | NO | "Warn on medium issues" - context matters | Moderate |
| **Low Security (CVSS 0.1-3.9)** | NO | "Log low issues for later review" | Weak |

#### Tool-Truth Classification (Codex-RS Context)

From `gate_policy.rs`, `ToolTruthKind` enum defines these categories:

| ToolTruthKind | Hard-Fail Recommendation | Codex-RS Current |
|---------------|--------------------------|------------------|
| `Compile` | YES - No exceptions | `escalate_on_tool_failure=true` |
| `UnitTests` | YES - No exceptions | `escalate_on_tool_failure=true` |
| `IntegrationTests` | YES - No exceptions | `escalate_on_tool_failure=true` |
| `TypeCheck` | YES - Compile-equivalent | `escalate_on_tool_failure=true` |
| `Lint` | Configurable - Rule-dependent | `escalate_on_tool_failure=true` |
| `Format` | NO - Auto-fixable | Consider separate |
| `SchemaValidation` | YES - Contract enforcement | `escalate_on_tool_failure=true` |

#### Security Severity Mapping

| CVSS Range | Severity | Recommended Policy | Evidence |
|------------|----------|-------------------|----------|
| 9.0-10.0 | Critical | Hard-fail, no bypass | "Zero tolerance" universal |
| 7.0-8.9 | High | Hard-fail, justified bypass | "Enforce fail-the-build for high" |
| 4.0-6.9 | Medium | Soft-fail (warning) | "Warn on medium issues" |
| 0.1-3.9 | Low | Log only | "Log low for later review" |
| 0.0 | Info | Silent | Not security-relevant |

#### CounterSignalKind Classification (Codex-RS Context)

From `gate_policy.rs`, mapping counter-signals to hard-fail recommendation:

| CounterSignalKind | Hard-Fail? | Rationale |
|-------------------|------------|-----------|
| `PolicyViolation` | YES | GR-* guardrails are non-negotiable |
| `Contradiction` | Configurable | Blocking if `contradictions.blocking=true` |
| `SecurityRisk` | Critical YES, Medium NO | Severity-based |
| `SafetyRisk` | YES | Safety is non-negotiable |
| `HighRiskChange` | Configurable | Escalate to human/judge |
| `MissingAcceptanceCriteria` | NO | Can be resolved during stage |
| `Ambiguity` | NO | Can be clarified |
| `PerformanceRisk` | NO | Usually non-blocking |
| `Other` | NO | Catch-all, context-dependent |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Emergency hotfix blocked** | Critical fix blocked by unrelated test failure | Documented emergency bypass procedure |
| **False positive security** | Legitimate code flagged as vulnerable | Exception lists; feedback to tool vendor |
| **Flaky test blocks** | Non-deterministic test causes spurious failures | Test reliability gate; retry logic |
| **Dependency vulnerability** | Can't update due to breaking changes | Compensating control documentation |
| **Legacy module penalty** | Old code fails new standards | Exemption lists with sunset dates |

#### SOC2/Compliance Requirements

From research, SOC2 audits require:
- "System-enforced review process (pull request) that requires peer review"
- "Gated builds, artifact integrity, and controlled deploys"
- "Separation of duties for code, build, and production access"
- "Audit trail of who logged in, what they changed, when"

**Mandatory for compliance:** Code review gate, test gate, change tracking.

#### NIST SSDF Requirements

From NIST SP 800-218:
- **SA-11**: Developer testing and evaluation required
- **CM-3**: Configuration change control with approvals
- **SI-7**: Software integrity checks (code signing)

**Mandatory for federal contracts:** Test gates, integrity checks, audit trail.

### D) Decision Considerations Checklist

1. [ ] **Compile failures**: Currently always escalate - confirm no bypass needed
2. [ ] **Test failures**: All test types (unit, integration) same policy?
3. [ ] **Critical security**: CVSS 9.0+ always blocks - confirmed?
4. [ ] **High security**: CVSS 7.0-8.9 - hard-fail or require justification?
5. [ ] **Lint separation**: Split lint into bug-detection vs style rules?
6. [ ] **Format treatment**: Should format violations ever block?
7. [ ] **Schema validation**: Same as compile for contract enforcement?
8. [ ] **Policy violations**: GR-* guardrails - all hard-fail?
9. [ ] **Safety risks**: Always escalate regardless of confidence?
10. [ ] **Emergency bypass**: Documented procedure for hotfixes?
11. [ ] **Flaky test handling**: Retry logic before hard-fail?
12. [ ] **Legacy exemptions**: Sunset dates for exemption lists?

### E) Proposed Hard-Fail Policy Set for Codex-RS

Based on research synthesis and current implementation:

#### Tier 1: Absolute Hard-Fail (No Bypass)
- `ToolTruthKind::Compile` - Compilation errors
- `ToolTruthKind::TypeCheck` - Type check failures
- `ToolTruthKind::SchemaValidation` - Schema validation failures
- `CounterSignalKind::SafetyRisk` - Any safety concern
- Critical security vulnerabilities (CVSS 9.0+)

#### Tier 2: Hard-Fail with Documented Override
- `ToolTruthKind::UnitTests` - Unit test failures
- `ToolTruthKind::IntegrationTests` - Integration test failures
- `CounterSignalKind::PolicyViolation` - GR-* guardrails
- High security vulnerabilities (CVSS 7.0-8.9)

#### Tier 3: Configurable (Soft by Default)
- `ToolTruthKind::Lint` - Lint warnings (not errors)
- `CounterSignalKind::HighRiskChange` - Context-dependent
- `CounterSignalKind::Contradiction` - Unless explicitly blocking
- Medium security vulnerabilities (CVSS 4.0-6.9)

#### Tier 4: Advisory Only
- `ToolTruthKind::Format` - Style preferences
- `CounterSignalKind::Ambiguity` - Can be clarified
- `CounterSignalKind::MissingAcceptanceCriteria` - Resolvable
- `CounterSignalKind::PerformanceRisk` - Non-blocking
- Low/Info security findings

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| gate_policy.rs | `codex-rs/spec-kit/src/gate_policy.rs` | ToolTruthKind, CounterSignalKind enums; escalate_on_tool_failure flag |
| model_policy.toml | `codex-rs/model_policy.toml` | Gate criteria configuration |
| SPEC-KIT-941 | `docs/SPEC-KIT-941-automated-policy-compliance/PRD.md` | Blocking violations in CI |
| SPEC.md | `codex-rs/SPEC.md` | GR-* guardrails; non-negotiable invariants |

---

## 16. Research Brief: F1 - Maintenance Framework Timing

### Question
**F1) Maintenance framework timing: Continuous vs Scheduled vs Event-triggered vs Tiered**

When should maintenance jobs run:
- **Continuous**: Always running, polling for work
- **Scheduled**: Fixed cron schedules (daily, weekly)
- **Event-triggered**: Fire on specific events (pipeline complete, threshold crossed)
- **Tiered**: Hybrid combining event + scheduled + on-demand triggers

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Cron-based Scheduling** | Time-based job execution via cron expressions; traditional batch processing |
| **Event-Driven Architecture (EDA)** | Jobs triggered by events (file uploads, API calls, threshold crossings) |
| **Workload Automation (WLA)** | Evolution of job scheduling combining time-based and event-based triggers |
| **Toil** | Repetitive, manual, automatable work that doesn't add value (SRE concept) |
| **Condition-based Trigger** | Jobs triggered by real-time conditions (sensor data, metrics thresholds) |
| **Batch Window** | Traditional time period when resources are available for batch processing |
| **SODA/SOAPs** | Service Orchestration and Automation Platforms; modern WLA evolution |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Google SRE Book: Introduction](https://sre.google/sre-book/introduction/) - "50% cap on ops work; systems should self-repair"
2. [CircleCI: Scheduled Pipelines in CI](https://blog.railway.com/p/run-scheduled-and-recurring-tasks-with-cron) - Benefits of scheduled vs event-triggered
3. [Wikipedia: Job Scheduler](https://en.wikipedia.org/wiki/Job_scheduler) - Batch processing fundamentals
4. [eMaint: Maintenance Scheduling](https://www.emaint.com/blog-maintenance-scheduling/) - Time-based vs condition-based

#### SRE/Operational Perspectives
5. [APMdigest: 2025 SRE Report](https://www.apmdigest.com/maximizing-resilience-insights-2025-sre-report) - Toil rose to 30%; agility vs stability tension
6. [Google SRE: Service Best Practices](https://sre.google/sre-book/service-best-practices/) - N+2 configuration; planned outage windows
7. [Netguru: SRE Best Practices](https://www.netguru.com/blog/site-reliability-engineering) - Error budgets for balancing reliability vs velocity

#### Real-World Implementation References
8. [Medium: Why We Replaced 80% of Cron Jobs](https://medium.com/@TheOutageSpecialist/why-we-replaced-80-of-our-cron-jobs-with-event-driven-systems-d05e20317f0c) - Event-driven for unpredictable workloads
9. [Schematical: Cron vs EDA](https://schematical.com/posts/cron-v-eda_2024-03-28) - Cron for bounded tasks; EDA for coordination
10. [Stonebranch: Workload Automation](https://www.stonebranch.com/blog/what-is-workload-automation) - WLA combines scheduled + event-driven

#### Practitioner/Engineering Blog Posts
11. [DevOps Training: When to Replace Cron](https://www.devopstraininginstitute.com/blog/when-should-you-replace-cron-jobs-with-event-driven-serverless-workflows) - Event-driven for dynamic execution
12. [DataCamp: Cron Jobs in Data Engineering](https://www.datacamp.com/tutorial/cron-job-in-data-engineering) - Pipeline scheduling patterns

### C) Neutral Synthesis

#### What Tends to Work

**Continuous (Polling):**
- Always listening for work; minimal latency
- "Maintenance is a continuous process" but resource-intensive
- Risk: Resource waste during idle periods; polling overhead
- Best for: Real-time monitoring, health checks

**Scheduled (Cron):**
- "Cron is suitable for tasks that are short, bounded, and self-contained"
- "As long as a batch task can complete within its scheduled window... cron provides a stable trigger"
- Traditional 2-4 AM "batch window" for minimal impact
- Risk: "What happens if a job doesn't complete before the next scheduled run?"
- Best for: Cleanup jobs, reports, non-urgent maintenance

**Event-Triggered:**
- "Consider replacing cron with event-driven when tasks require dynamic, scalable, resilient execution"
- "Failed cron job often requires manual intervention; serverless has automatic retries, dead-letter queues"
- "Safer to run multiple consumers in parallel since an event would be for a specific entity"
- Risk: Event storms; infinite loops; complexity
- Best for: Threshold violations, pipeline completions, file arrivals

**Tiered (Hybrid):**
- "Traditional time-based schedulers eclipsed by event-based workload automation... plus near real-time"
- Gartner 2025: "90% of orgs will use SOAPs for hybrid environments by 2029"
- Combine predictability of scheduling with responsiveness of events
- Best for: Complex systems with multiple maintenance types

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Cron overlap** | Job doesn't complete before next scheduled run | Lock file; single-instance enforcement |
| **Event storm** | Cascade of events overwhelms system | Debouncing; rate limiting; admission control |
| **Silent failure** | Scheduled job fails without notification | Health checks; alerting; dead-letter handling |
| **Resource contention** | Multiple jobs compete for same resource | Queue-based coordination; priority lanes |
| **Toil accumulation** | Manual intervention required too often | "Systems should self-repair" (Google SRE) |

#### Engineering Costs/Risks

| Approach | Implementation | Operational | Responsiveness | Resource Efficiency |
|----------|----------------|-------------|----------------|---------------------|
| Continuous | Medium | High (monitoring) | Immediate | Low (always running) |
| Scheduled | Low | Low | Delayed (up to schedule interval) | High |
| Event-triggered | Medium-High | Medium | Immediate | High (on-demand) |
| Tiered | High | Medium | Variable | Optimal |

#### What's Different for Codex-RS

1. **No scheduler implemented**: Config defines cron schedules but no executor exists
2. **Single-user workbench**: Not a multi-tenant server; resource contention is minimal
3. **Pipeline-centric**: Natural event boundaries at stage transitions
4. **Evidence budget enforcement**: 50MB hard limit needs enforcement check
5. **Librarian uses local LLM**: Compute-intensive; best scheduled for idle time
6. **Offline-first**: Can't rely on cloud schedulers; must be local

#### Recommended Tiered Pattern for Codex-RS

| Tier | Trigger | Timing | Job Families |
|------|---------|--------|--------------|
| **Event** | Pipeline complete, evidence write | Immediate | Health checks, evidence limit warnings |
| **Scheduled (daily)** | Cron 2 AM local | Low-impact window | Evidence cleanup >30 days, stats |
| **Scheduled (weekly)** | Cron Sunday 3 AM | Extended window | Librarian repair, meta-memory synthesis |
| **On-demand** | User/operator request | Interactive | Compaction, graph repair, emergency |

### D) Decision Considerations Checklist

1. [ ] **Event infrastructure**: What events are already emitted by the pipeline?
2. [ ] **Scheduler choice**: In-process (tokio-cron) vs external (system cron)?
3. [ ] **Resource isolation**: Should jobs run in separate process or in-TUI?
4. [ ] **Failure handling**: How to handle jobs that fail silently?
5. [ ] **Overlap prevention**: Single-instance enforcement mechanism?
6. [ ] **User notification**: How to inform user of maintenance status?
7. [ ] **Idle detection**: Should jobs wait for user inactivity?
8. [ ] **Priority queuing**: If multiple jobs pending, which runs first?
9. [ ] **Dry-run support**: Can all jobs preview without modifying?
10. [ ] **Cancellation**: Can long-running jobs be interrupted?
11. [ ] **Observability**: How to track job history and performance?
12. [ ] **SPEC-KIT-103 cron config**: Activate existing config or redesign?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-103 | `docs/SPEC-KIT-103-librarian/spec.md` | Librarian schedule config: repair_cron, synthesis_cron, enrichment_cron |
| SPEC-KIT-909 | `docs/SPEC-KIT-909-evidence-cleanup-automation/` | Evidence cleanup timing; 30-day archive threshold |
| librarian/mod.rs | `codex-rs/stage0/src/librarian/mod.rs` | Sweep orchestration; dry-run support |
| evidence.rs | `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs` | Evidence limit checking; auto_export triggers |

---

## 17. Research Brief: F2 - First Maintenance Job Family

### Question
**F2) First maintenance job family: Which should be implemented first?**

Which job family should be implemented first:
- **Librarian**: Meta-memory synthesis, graph repair, causal relationship labeling
- **Evidence Cleanup**: Enforce 50MB limit, archive >30 days, purge >180 days
- **Compaction**: Retention hardening, orphan pruning, history truncation
- **Health Check**: System observability, resource monitoring, integrity verification

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Job Family** | Group of related maintenance jobs sharing common purpose and dependencies |
| **Librarian** | Offline LLM-powered memory maintenance (classification, templating, causal inference) |
| **Evidence Cleanup** | File system maintenance (archival, compression, purging old artifacts) |
| **Compaction** | Memory/storage optimization (deduplication, consolidation, pruning) |
| **Health Check** | Observability job verifying system integrity and resource availability |
| **Dependency Order** | Sequence in which jobs must run based on prerequisites |
| **Foundation Layer** | Infrastructure jobs that other jobs depend on |

### B) Sources (11 total)

#### Authoritative/Primary Sources
1. [ArXiv: Agentic Memory (AgeMem)](https://arxiv.org/html/2601.01885v1) - Unified long/short-term memory management
2. [Serokell: Design Patterns for LLM Memory](https://serokell.io/blog/design-patterns-for-long-term-memory-in-llm-powered-architectures) - Memory compaction reduces redundancy; tiered storage
3. [ACM: PagedAttention](https://dl.acm.org/doi/10.1145/3600006.3613165) - Efficient memory management for LLM serving

#### SRE/Operational Perspectives
4. [JobRunr: Prioritizing Background Jobs](https://www.jobrunr.io/en/blog/prioritizing-background-jobs/) - Multiple queues by priority level
5. [MaintainX: Work Order Prioritization](https://www.getmaintainx.com/blog/work-order-prioritization) - Critical > High > Medium > Low
6. [IDCON: Setting Disciplined Priorities](https://idcon.com/resource-library/work-management-planning-scheduling/setting-disciplined-priorities/) - Priorities based on consequences of not doing

#### Real-World Implementation References
7. [DEV: Why LLM Memory Still Fails](https://dev.to/isaachagoel/why-llm-memory-still-fails-a-field-guide-for-builders-3d78) - Rigorous selection for storage and removal
8. [Tribe AI: Context-Aware Memory 2025](https://www.tribe.ai/applied-ai/beyond-the-bubble-how-context-aware-memory-systems-are-changing-the-game-in-2025) - MemGPT pattern; memory as resource management

#### Practitioner/Engineering Blog Posts
9. [API7: Health Check Best Practices](https://api7.ai/blog/tips-for-health-check-best-practices) - Proactive maintenance uncovers latent issues
10. [Artkai: Software Health Check](https://artkai.io/blog/app-healthcheck) - Prioritize critical issues first
11. [Cloudvara: Database Management Best Practices 2025](https://cloudvara.com/database-management-best-practices/) - Start with low-hanging fruit (backups, indexing)

### C) Neutral Synthesis

#### What Tends to Work

**Health Check First:**
- "Proactive monitoring allows teams to detect anomalies before they escalate"
- "Schedule health checks based on system criticality"
- Foundation: Can't fix what you can't see
- Informs decisions about which other jobs are needed
- Lowest risk; read-only operations

**Evidence Cleanup Second:**
- Enforces hard limits preventing disk exhaustion
- "Start with low-hanging fruit like implementing a more robust backup schedule"
- Immediate, measurable impact (disk space)
- Prerequisites: Health check to know current state

**Librarian Third:**
- Enhancement layer; assumes healthy underlying state
- "Utility-based and retrieval-history-based deletion prevent memory bloat"
- Compute-intensive; best scheduled for idle time
- Prerequisites: Evidence within limits; stable system

**Compaction Last:**
- Optimization; not critical for operation
- "Snapshots are primarily a performance optimization—KISS: don't introduce until you encounter performance issues"
- Prerequisites: All other jobs stable

#### Codex-RS Implementation Status

| Job Family | Current Status | Key Gap |
|------------|----------------|---------|
| **Health Check** | NOT IMPLEMENTED | No observability jobs exist |
| **Evidence Cleanup** | IMPLEMENTED (partial) | Bash scripts + native Rust; no scheduler |
| **Librarian** | IMPLEMENTED (partial) | Classifier, templater, causal, audit modules exist |
| **Compaction** | BACKLOG (SYNC-026) | Spec only; no code |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Enhancement before foundation** | Running Librarian on corrupt state | Health check first |
| **Optimization before stability** | Compacting unstable data | Defer until system stable |
| **Parallel execution conflicts** | Multiple jobs modifying same data | Dependency ordering; locks |
| **Incomplete cleanup** | Cleanup job leaves orphans | Verify completeness post-run |
| **LLM resource exhaustion** | Librarian uses all GPU/CPU | Resource limits; queue priority |

#### Priority Framework

| Priority | Criterion | Example |
|----------|-----------|---------|
| P0 (Critical) | Safety, data integrity | Evidence limit enforcement |
| P1 (High) | Observability, diagnostics | Health checks |
| P2 (Medium) | Enhancement, quality | Librarian synthesis |
| P3 (Low) | Optimization, performance | Compaction |

#### Recommended Implementation Order

| Order | Job Family | Rationale |
|-------|------------|-----------|
| **1** | Health Check | Foundation for observability; informs all other decisions; read-only |
| **2** | Evidence Cleanup | Enforcement of 50MB hard limit; disk hygiene; already partially implemented |
| **3** | Librarian | Enhancement layer; depends on healthy evidence; partially implemented |
| **4** | Compaction | Optimization; last priority; requires stable foundation; backlog status |

#### Dependencies Graph

```
Health Check (P1, read-only)
    ↓
Evidence Cleanup (P0, write: archive, purge)
    ↓
Librarian (P2, write: classify, template, relate)
    ↓
Compaction (P3, write: consolidate, prune)
```

#### What's Different for Codex-RS

1. **Evidence Cleanup most advanced**: Native Rust + bash scripts exist
2. **Librarian partially implemented**: Modules exist but no scheduler
3. **Health Check missing**: No observability infrastructure
4. **Compaction spec-only**: SYNC-026 is backlog
5. **50MB hard limit enforcement**: Evidence cleanup critical for pipeline health
6. **Local LLM for Librarian**: Qwen 2.5 3B default; resource-intensive

### D) Decision Considerations Checklist

1. [ ] **Health check scope**: What metrics should be checked?
2. [ ] **Evidence cleanup automation**: Activate existing scripts or rebuild?
3. [ ] **Librarian scheduler**: Tokio-cron or system cron?
4. [ ] **Compaction deferral**: Is SYNC-026 blocked on other work?
5. [ ] **Dependency enforcement**: How to prevent out-of-order execution?
6. [ ] **Resource isolation**: Does Librarian need separate process?
7. [ ] **Partial failure handling**: What if cleanup succeeds but Librarian fails?
8. [ ] **Progress reporting**: How to show job progress in TUI?
9. [ ] **Dry-run parity**: All jobs support --dry-run?
10. [ ] **Audit trail**: All job executions logged to SQLite?
11. [ ] **User notification**: Alert when maintenance completes?
12. [ ] **CI/CD integration**: Can jobs run in headless mode?

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC-KIT-103 | `docs/SPEC-KIT-103-librarian/spec.md` | Librarian job definitions; 3 job types |
| SPEC-KIT-909 | `docs/SPEC-KIT-909-evidence-cleanup-automation/` | Evidence cleanup requirements; 50MB limit |
| SYNC-026 | `docs/SYNC-026-retention-compaction/` | Compaction spec (backlog) |
| librarian/* | `codex-rs/stage0/src/librarian/` | Implemented modules: classifier, templater, causal, audit |
| evidence.rs | `codex-rs/tui/src/chatwidget/spec_kit/evidence.rs` | Evidence limit checking; EvidenceRepository trait |

---

## 18. Research Brief: G1 - Cross-cutting Capability Verification

### Question
**G1) Cross-cutting capability verification: How to validate that all components work together?**

How should Codex-RS validate that all components (pipeline stages, quality gates, evidence lifecycle, state persistence) work together correctly:
- **End-to-end tests only**: Full pipeline runs testing complete workflows
- **Contract testing**: Define and verify inter-component contracts
- **Component tests with mocks**: Test groups of services as isolated units
- **Hybrid approach**: Combine contracts, component tests, and E2E

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Contract Testing** | Verifying compatibility between services by defining expected interactions; catches integration issues early |
| **Consumer-Driven Contract** | Contract defined by the client (consumer) and validated by the provider |
| **Component Testing** | Testing a group of related modules as a single unit focusing on interactions |
| **Integration Testing** | Verifying that different modules work together; focuses on interfaces |
| **Cross-cutting Concern** | Aspect that affects multiple components (logging, auth, caching) but doesn't align with primary functionality |
| **Traceability Matrix** | Document correlating requirements to test cases using many-to-many mapping |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [DEV: 2025 Integration Testing Handbook](https://dev.to/testwithtorin/2025-integration-testing-handbook-techniques-tools-and-trends-3ebc) - Techniques, tools, and trends for integration testing
2. [BrowserStack: Integration Testing Guide](https://www.browserstack.com/guide/integration-testing) - Best practices and testing pyramid
3. [Microservices.io: Service Integration Contract Test](https://microservices.io/patterns/testing/service-integration-contract-test.html) - Pattern definition
4. [Aqua-cloud: System Integration Testing](https://aqua-cloud.io/system-integration-testing/) - SIT best practices

#### Testing/QA Perspectives
5. [Tweag: Contract Testing Shift-Left](https://www.tweag.io/blog/2025-01-23-contract-testing/) - Confidence for enhanced integration
6. [Index.dev: Component Contract Testing](https://www.index.dev/blog/component-contract-testing-microservices) - Best practices for microservices
7. [Harness: Unit vs Integration Testing](https://www.harness.io/harness-devops-academy/unit-testing-vs-integration-testing) - Key differences and practices

#### Real-World Implementation References
8. [Bunnyshell: E2E Testing for Microservices 2025](https://www.bunnyshell.com/blog/end-to-end-testing-for-microservices-a-2025-guide/) - Engineering leader guide
9. [HyperTest: Contract Testing for Microservices](https://www.hypertest.co/contract-testing/contract-testing-for-microservices) - Complete guide
10. [Gravitee: Contract Testing Strategy](https://www.gravitee.io/blog/contract-testing-microservices-strategy) - Missing link in strategy

#### Practitioner/Engineering Blog Posts
11. [GitHub: Awesome Software Architecture - Cross-cutting](https://github.com/mehdihadeli/awesome-software-architecture/blob/main/docs/architectural-design-principles/cross-cutting-concerns.md) - Design patterns for cross-cutting
12. [t2informatik: Solving Cross-Cutting Concerns](https://t2informatik.de/en/blog/solving-cross-cutting-concerns-through-patterns/) - Pattern-based solutions

### C) Neutral Synthesis

#### What Tends to Work

**End-to-End Tests Only:**
- "Simulates full workflows but prone to flakiness, slow execution, and difficulty isolating failures"
- Maximum confidence in complete system behavior
- Risk: Slow feedback loops; expensive to maintain
- Best for: Critical user journeys; release validation

**Contract Testing:**
- "Instead of testing everything live, verify each service lives up to its agreed contract"
- "Fast, isolated, and plays well in CI/CD pipelines"
- "Fills the gap by validating interactions between components in isolation"
- "Yields faster feedback loops, more reliable signals, and drastically simplified debugging"
- Consumer-driven: Contract defined by client, validated by provider
- Risk: Contracts can drift from reality without enforcement

**Component Tests with Mocks:**
- "Bridges the gap between integration testing and E2E testing"
- "Can uncover issues not apparent when testing in isolation"
- "Treats a collection of microservices as a cohesive component"
- Risk: Mocks may not reflect real service behavior

**Hybrid Approach:**
- "A balanced testing pyramid: strong unit/contract tests under lean E2E tests"
- "Enjoy agility of independent services and assurance of integrated validation"
- "Shift-left strategy: test early rather than late in pipeline"
- Industry consensus for 2025

#### Current Codex-RS Implementation

| Component | Testing Approach | Coverage |
|-----------|------------------|----------|
| **Pipeline Stages** | Workflow integration tests (W01-W15) | Handler → consensus → evidence → guardrail → state |
| **Quality Gates** | Quality flow tests (Q01-Q10) | GPT-5 validation, auto-resolution, escalation |
| **Evidence Lifecycle** | State persistence tests (S01-S10) | Evidence coordination, pipeline interrupt/resume |
| **State Persistence** | Error recovery tests (E01-E15) | Consensus failures, MCP fallback, retry logic |
| **Concurrency** | Concurrent tests (C01-C10) | Parallel execution, locking, race conditions |

**Test Infrastructure:**
- `MockMcpManager`: Fixture-based MCP response replay
- `IntegrationTestContext`: Filesystem isolation with temp dirs
- `StateBuilder`: Fluent test state construction
- `EvidenceVerifier`: Evidence artifact validation

#### Cross-cutting Verification in Codex-RS

| Cross-cutting Concern | Current Verification | Gap |
|----------------------|---------------------|-----|
| **Logging/Telemetry** | Schema validation tests | No runtime verification |
| **Error Handling** | E01-E15 error recovery tests | Comprehensive |
| **State Persistence** | S01-S10 persistence tests | Comprehensive |
| **Security (GR-*)** | Guardrail tests | Policy enforcement verified |
| **Evidence Management** | EvidenceVerifier assertions | Comprehensive |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Mock drift** | Mocks diverge from real behavior | Contract tests enforce reality |
| **E2E flakiness** | Non-deterministic failures | Retry logic; deterministic fixtures |
| **Integration gap** | Components work alone but not together | Workflow tests cover full path |
| **Contract staleness** | Contracts not updated with code | CI/CD enforcement of contract verification |
| **Cross-cutting blind spots** | Concerns tested in isolation only | Decorator pattern; AOP verification |

#### Engineering Costs/Risks

| Approach | Implementation | Maintenance | Confidence | Speed |
|----------|----------------|-------------|------------|-------|
| E2E Only | Low | High | High | Slow |
| Contract Testing | Medium | Low | Medium-High | Fast |
| Component + Mocks | Medium | Medium | Medium | Medium |
| Hybrid | High | Medium | Highest | Variable |

#### What's Different for Codex-RS

1. **Already has comprehensive workflow tests**: W01-W15 cover handler → consensus → evidence → guardrail → state
2. **Fixture-based determinism**: MockMcpManager enables reproducible integration tests
3. **Single-user workbench**: Not distributed system; simpler coordination
4. **Quality gate broker**: Multi-source collection (native, filesystem, SQLite) already verified
5. **GR-001 enforcement**: Single-owner pipeline simplifies verification
6. **8 pipeline stages**: Fixed stage count; known interaction patterns

### D) Decision Considerations Checklist

1. [ ] **Contract coverage**: Are inter-component contracts explicitly defined?
2. [ ] **Fixture freshness**: Are MockMcpManager fixtures regularly updated from real outputs?
3. [ ] **E2E critical paths**: Which user journeys require full E2E validation?
4. [ ] **Cross-cutting isolation**: Can logging/telemetry be verified independently?
5. [ ] **CI/CD integration**: Are contract tests in the build pipeline?
6. [ ] **Flakiness rate**: What percentage of integration tests are non-deterministic?
7. [ ] **Mock vs real**: Where are real services used vs mocks in tests?
8. [ ] **Regression detection**: Do tests catch component interaction regressions?
9. [ ] **Traceability**: Is there a requirements-to-test mapping?
10. [ ] **Decorator verification**: Are cross-cutting concerns verified via decoration?
11. [ ] **Pipeline stages**: Are all 8 stages tested in combination?
12. [ ] **Quality gate flow**: Are BeforeSpecify/AfterSpecify/AfterTasks verified together?

### E) Recommended Approach for Codex-RS

**Hybrid Contract + Workflow Testing:**

1. **Keep existing workflow tests (W01-W15)**: These provide comprehensive cross-component verification
2. **Add explicit contracts for key interfaces**:
   - Stage transition contract (StageType enum → expected behaviors)
   - Quality gate contract (checkpoint → agent response schema)
   - Evidence contract (artifact type → storage format)
3. **Formalize traceability matrix**: Map invariants (SPEC.md #1-10) to test coverage
4. **Cross-cutting verification via decorators**: Apply decorator pattern tests for logging/telemetry

**Verification Pyramid:**
```
                    E2E (W01-W15 workflows)
                   ╱                      ╲
          Component Tests           Contract Tests
         (Q01-Q10, S01-S10)     (Stage/Gate/Evidence)
        ╱                                        ╲
                    Unit Tests (135+)
```

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| testing-policy.md | `codex-rs/tui/testing-policy.md` | 604 tests; 42-48% coverage; phase-based testing |
| workflow_integration_tests.rs | `codex-rs/tui/tests/workflow_integration_tests.rs` | W01-W15 multi-stage workflow tests |
| quality_gates_integration.rs | `codex-rs/tui/tests/quality_gates_integration.rs` | Q01-Q10 quality flow tests |
| integration_harness.rs | `codex-rs/tui/tests/common/integration_harness.rs` | IntegrationTestContext, StateBuilder |

---

## 19. Research Brief: G2 - Capability Matrix Definition

### Question
**G2) Capability matrix definition: What's the canonical list of capabilities that must be tested?**

What capabilities should be formally tracked in a testing matrix:
- **Pipeline capabilities**: Stage transitions, dependencies, skip conditions
- **Quality capabilities**: Gates, checkpoints, escalation paths
- **Evidence capabilities**: Lifecycle, limits, archival
- **State capabilities**: Persistence, recovery, concurrency
- **Cross-cutting capabilities**: Logging, error handling, security

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Traceability Matrix (RTM)** | Document mapping requirements to test cases; ensures completeness |
| **Coverage Matrix** | Table showing which requirements/features are tested by which test cases |
| **Forward Traceability** | Mapping requirements to test cases (requirement → test) |
| **Backward Traceability** | Mapping test cases to requirements (test → requirement) |
| **Bi-directional Traceability** | Both forward and backward; ensures no orphaned tests or untested requirements |
| **Capability** | Discrete functional or non-functional feature that system must provide |

### B) Sources (11 total)

#### Authoritative/Primary Sources
1. [Guru99: Requirements Traceability Matrix](https://www.guru99.com/traceability-matrix.html) - RTM definition and templates
2. [Wikipedia: Traceability Matrix](https://en.wikipedia.org/wiki/Traceability_matrix) - Formal definition
3. [TestRail: RTM How-To Guide](https://www.testrail.com/blog/requirements-traceability-matrix/) - Practical implementation

#### Testing/QA Perspectives
4. [Aqua-cloud: Traceability Matrix Guide](https://aqua-cloud.io/traceability-matrix/) - Types, benefits, creation
5. [LambdaTest: RTM Comprehensive Guide](https://www.lambdatest.com/learning-hub/requirements-traceability-matrix) - Best practices
6. [Inspired Testing: Coverage Matrix](https://www.inspiredtesting.com/news-insights/insights/344-what-is-a-coverage-matrix) - Coverage vs traceability

#### Real-World Implementation References
7. [TestBytes: Traceability Matrix Types](https://www.testbytes.net/blog/software-testing-traceability-matrix/) - Significance and types
8. [Software Testing Help: RTM Template](https://www.softwaretestinghelp.com/requirements-traceability-matrix/) - Example templates
9. [Testsigma: RTM and Regression Testing](https://testsigma.com/blog/requirement-traceability-matrix-regression-testing/) - RTM for regression

#### Practitioner/Engineering Blog Posts
10. [DesignRush: SIT Best Practices 2025](https://www.designrush.com/agency/it-services/system-integrators/trends/system-integration-testing-best-practices) - Clear objectives and documentation
11. [GeeksforGeeks: Regression Testing](https://www.geeksforgeeks.org/software-engineering/software-engineering-regression-testing/) - Regression test selection

### C) Neutral Synthesis

#### What Tends to Work

**Coverage Matrix Components:**
- "Includes new feature testing, application coverage, and code coverage"
- "Test Case Description, Status, Defect ID, Priority, Coverage Status, Test Type"
- "Makes sure that a piece of software has been thoroughly tested"

**Traceability Matrix Components:**
- "Used to check if current project requirements are being met"
- "Correlating any two baselined documents using many-to-many relationship"
- "Often used with high-level requirements to matching parts of design, test plan, and test cases"

**Forward vs Backward:**
- Forward: "Checks whether project progresses in right direction"
- Backward: "Ensures no test cases are unlinked from requirements"
- Bi-directional: "Complete traceability in both directions"

#### Codex-RS Capability Categories

Based on codebase exploration, the canonical capabilities are:

**1. Pipeline Capabilities (8 stages)**

| Capability | Stage(s) | Test Coverage | Current Status |
|------------|----------|---------------|----------------|
| Stage initialization | New | W01 | Covered |
| Specification generation | Specify | W02 | Covered |
| Plan generation | Plan | W03-W05 | Covered |
| Task decomposition | Tasks | W06 | Covered |
| Implementation execution | Implement | W07-W09 | Covered |
| Validation checks | Validate | W10-W11 | Covered |
| Audit completion | Audit | W12-W13 | Covered |
| Unlock/completion | Unlock | W14-W15 | Covered |

**2. Quality Gate Capabilities (3 checkpoints)**

| Capability | Checkpoint | Test Coverage | Current Status |
|------------|------------|---------------|----------------|
| Pre-plan clarification | BeforeSpecify | Q01-Q02 | Covered |
| Post-plan validation | AfterSpecify | Q03-Q05 | Covered |
| Post-tasks review | AfterTasks | Q06-Q10 | Covered |

**3. Evidence Capabilities**

| Capability | Function | Test Coverage | Current Status |
|------------|----------|---------------|----------------|
| Artifact creation | Write consensus, telemetry | S01-S03 | Covered |
| Size limit enforcement | 50MB hard limit | Evidence limit tests | Partial |
| Archival (>30 days) | Auto-archive old artifacts | Not tested | Gap |
| Cleanup/purge | Remove >180 days | Not tested | Gap |
| Checksum verification | SHA256 integrity | Not tested | Gap |

**4. State Capabilities**

| Capability | Function | Test Coverage | Current Status |
|------------|----------|---------------|----------------|
| State initialization | SpecAutoState creation | E01 | Covered |
| Stage persistence | SQLite + filesystem | S04-S06 | Covered |
| Interrupt recovery | Resume from checkpoint | E02-E05 | Covered |
| Error fallback | MCP fallback, retry | E06-E10 | Covered |
| Concurrent access | Locking, race prevention | C01-C10 | Covered |

**5. Cross-cutting Capabilities**

| Capability | Function | Test Coverage | Current Status |
|------------|----------|---------------|----------------|
| Telemetry logging | Schema-valid telemetry | Schema tests | Covered |
| Error classification | Retryable vs permanent | agent_retry tests | Covered |
| Security (GR-*) | Policy enforcement | Guardrail tests | Covered |
| Audit trail | PolicySnapshot binding | SPEC-KIT-977 tests | Covered |
| Configuration validation | Pipeline config | config_validator tests | Covered |

#### Recommended Capability Matrix Structure

**Level 1: High-Level Capability Categories**
```
├── Pipeline Capabilities (P)
│   ├── P.1 Stage Transitions
│   ├── P.2 Stage Dependencies
│   ├── P.3 Skip Conditions
│   └── P.4 Workflow Patterns
├── Quality Gate Capabilities (Q)
│   ├── Q.1 Gate Execution
│   ├── Q.2 Issue Resolution
│   ├── Q.3 Escalation Paths
│   └── Q.4 Confidence Thresholds
├── Evidence Capabilities (E)
│   ├── E.1 Artifact Lifecycle
│   ├── E.2 Size Enforcement
│   ├── E.3 Archival/Cleanup
│   └── E.4 Integrity Verification
├── State Capabilities (S)
│   ├── S.1 Persistence
│   ├── S.2 Recovery
│   ├── S.3 Concurrency
│   └── S.4 Dual-track Storage
└── Cross-cutting Capabilities (X)
    ├── X.1 Telemetry
    ├── X.2 Error Handling
    ├── X.3 Security/Policy
    └── X.4 Configuration
```

**Level 2: Traceability Matrix Format**

| Capability ID | Requirement | Test Case(s) | Status | Coverage % |
|---------------|-------------|--------------|--------|------------|
| P.1.1 | Stage New initializes correctly | W01 | Pass | 100% |
| P.1.2 | Stage transitions follow dependency graph | W01-W15 | Pass | 100% |
| Q.1.1 | BeforeSpecify gate executes | Q01-Q02 | Pass | 100% |
| E.2.1 | 50MB hard limit enforced | evidence_limit_test | Partial | 70% |
| S.3.1 | Concurrent access prevented | C01-C10 | Pass | 100% |
| X.1.1 | Telemetry schema validated | telemetry_schema_test | Pass | 100% |

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Orphaned tests** | Tests not linked to requirements | Backward traceability audit |
| **Untested requirements** | Requirements without test coverage | Forward traceability audit |
| **Stale matrix** | Matrix not updated with code changes | Automate from test results |
| **Over-coverage** | Same requirement tested multiple ways | Consolidate redundant tests |
| **Missing cross-cutting** | Cross-cutting not in matrix | Explicit X.* category |

### D) Decision Considerations Checklist

1. [ ] **Matrix format**: Spreadsheet vs code-as-documentation?
2. [ ] **Granularity**: How detailed should capability breakdown be?
3. [ ] **Automation**: Can coverage be extracted from test results?
4. [ ] **Invariant mapping**: Are SPEC.md invariants (#1-10) in matrix?
5. [ ] **Evidence gap**: E.3 (archival/cleanup) and E.4 (integrity) need tests?
6. [ ] **Cross-cutting coverage**: Is X.* category comprehensive?
7. [ ] **Update cadence**: How often is matrix reviewed?
8. [ ] **Ownership**: Who maintains the matrix?
9. [ ] **CI/CD integration**: Can matrix be validated in pipeline?
10. [ ] **Regression scope**: Does matrix inform regression test selection?
11. [ ] **Gap identification**: Does matrix highlight untested areas?
12. [ ] **Historical tracking**: Is coverage trend tracked over time?

### E) Proposed Capability Matrix for Codex-RS

**Format**: Markdown table in `docs/CAPABILITY_MATRIX.md` (to be created)

| Category | ID | Capability | Invariant | Test(s) | Status |
|----------|-----|------------|-----------|---------|--------|
| **Pipeline** | P.1 | Stage transitions | #5, #6 | W01-W15 | ✅ |
| **Pipeline** | P.2 | Stage dependencies | SPEC-948 | config_validator | ✅ |
| **Pipeline** | P.3 | Skip conditions | SPEC-948 | skip_condition_tests | ✅ |
| **Quality** | Q.1 | Gate execution | GR-001 | Q01-Q10 | ✅ |
| **Quality** | Q.2 | Issue resolution | - | resolution_tests | ✅ |
| **Quality** | Q.3 | Escalation paths | - | escalation_tests | ✅ |
| **Evidence** | E.1 | Artifact lifecycle | #2 | S01-S03 | ✅ |
| **Evidence** | E.2 | 50MB enforcement | SPEC-909 | limit_tests | ⚠️ Partial |
| **Evidence** | E.3 | Archival (>30 days) | SPEC-909 | - | ❌ Gap |
| **Evidence** | E.4 | Integrity (SHA256) | - | - | ❌ Gap |
| **State** | S.1 | Persistence | #2 | S04-S06 | ✅ |
| **State** | S.2 | Recovery | - | E02-E05 | ✅ |
| **State** | S.3 | Concurrency | - | C01-C10 | ✅ |
| **Cross-cutting** | X.1 | Telemetry | - | schema_tests | ✅ |
| **Cross-cutting** | X.2 | Error handling | - | E06-E10 | ✅ |
| **Cross-cutting** | X.3 | Security (GR-*) | GR-001-013 | guardrail_tests | ✅ |
| **Cross-cutting** | X.4 | Configuration | - | config_tests | ✅ |

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| SPEC.md | `codex-rs/SPEC.md` | 10 invariants to map |
| testing-policy.md | `codex-rs/tui/testing-policy.md` | Current test distribution |
| SPEC-KIT-909 | `docs/archive/specs/SPEC-KIT-909-evidence-cleanup-automation/` | Evidence lifecycle requirements |
| SPEC-KIT-948 | `docs/SPEC-948-modular-pipeline-logic/` | Pipeline dependency requirements |

---

## 20. Research Brief: G3 - Regression Prevention

### Question
**G3) Regression prevention: How to prevent capability regressions during refactoring?**

What strategies should Codex-RS use to prevent capability regressions:
- **Characterization tests**: Capture current behavior before refactoring
- **Property-based testing**: Verify invariants across random inputs
- **Snapshot/golden file testing**: Compare outputs to known-good references
- **Contract versioning**: Version interfaces to detect breaking changes
- **Mutation testing**: Verify tests catch injected bugs

### A) Key Concepts/Terms

| Term | Definition |
|------|------------|
| **Characterization Test** | Test that describes actual behavior of existing code; protects legacy during refactoring |
| **Golden Master Testing** | Capturing current output as reference; comparing future outputs against it |
| **Property-Based Testing** | Generating random inputs to verify properties/invariants hold across all cases |
| **Approval Testing** | Capturing output for human approval; fastest way to put existing code under test |
| **Snapshot Testing** | Freezing behavior at execution time; future runs compared to snapshot |
| **Mutation Testing** | Injecting bugs to verify test suite detects them |

### B) Sources (12 total)

#### Authoritative/Primary Sources
1. [Wikipedia: Characterization Test](https://en.wikipedia.org/wiki/Characterization_test) - Coined by Michael Feathers; Golden Master definition
2. [Martin Fowler: Characterization Testing](https://martinfowler.com/bliki/CharacterizationTest.html) - Legacy code protection
3. [Working Effectively with Legacy Code](https://www.amazon.com/Working-Effectively-Legacy-Michael-Feathers/dp/0131177052) - Feathers' canonical reference

#### Testing/QA Perspectives
4. [Understand Legacy Code: Characterization vs Approval Tests](https://understandlegacycode.com/blog/characterization-tests-or-approval-tests/) - Distinction and use cases
5. [NimblePros: Characterization Tests with Snapshot Testing](https://blog.nimblepros.com/blogs/characterization-tests-with-snapshot-testing/) - Practical implementation
6. [Production Ready: Snapshot Testing in C#](https://www.production-ready.de/2025/12/01/snapshot-testing-in-csharp-en.html) - Snapshot patterns

#### Real-World Implementation References
7. [Shaped AI: Golden Tests in AI](https://www.shaped.ai/blog/golden-tests-in-ai) - ML-specific golden testing
8. [Johal.in: Pytest Regressions 2025](https://johal.in/pytest-regressions-data-golden-file-updates-2025/) - ML pipeline golden files
9. [anp.lol: Golden Master in Rust](https://blog.anp.lol/rust/2017/08/18/golden-master-regression-in-rust/) - Rust-specific implementation

#### Practitioner/Engineering Blog Posts
10. [Widgetbook: Golden Tests / UI Regression](https://docs.widgetbook.io/glossary/golden-tests) - UI golden testing
11. [ScienceDirect: Regression Test Efficiency with Refactoring](https://www.sciencedirect.com/science/article/abs/pii/S095058491830137X) - Academic analysis
12. [GitHub: franiglesias/golden](https://github.com/franiglesias/golden) - Golang golden library

### C) Neutral Synthesis

#### What Tends to Work

**Characterization/Golden Master Testing:**
- "Means to describe the actual behavior of an existing piece of software"
- "Protect existing behavior of legacy code against unintended changes"
- "You don't need to understand the code to test it"
- "Achieve high coverage really fast, so refactoring will be safe"
- "Once you start refactoring and abstracting code, easier to introduce classic assertion testing"
- Risk: "Depends on repeatability; volatile values need to be masked"

**Property-Based Testing:**
- "Instead of looking for exact values, look for desired properties of the output"
- Codex-RS uses proptest: 2,560+ generated test cases
- Verifies invariants hold across random inputs
- Risk: Can be slower; requires careful property definition

**Snapshot Testing:**
- "Freeze the behavior of a system at execution time"
- "Result written to file and serves as reference for future runs"
- "Possible to freeze behavior and protect against unwanted changes"
- Codex-RS uses insta: VT100 terminal output snapshots
- Risk: Snapshots can become stale; need review process

**Contract Versioning:**
- "Implement versioning for contracts to manage changes over time"
- "Allows consumers to adapt to new versions without immediate disruption"
- CI/CD enforcement: "If verification fails, build is blocked"
- Risk: Version proliferation; maintenance burden

**For AI/ML Systems (2025 specific):**
- "Fuzzy matching for near-duplicates using cosine similarity on embeddings"
- "For LLMs, validate JSON schemas + semantic drift via BERTScore"
- "Golden tests detect regressions by comparing current outputs to saved golden set"
- Risk: AI outputs are non-deterministic; need semantic comparison

#### Current Codex-RS Implementation

| Technique | Framework | Location | Coverage |
|-----------|-----------|----------|----------|
| **Snapshot Testing** | insta 1.43.x | tui2/src/chatwidget/snapshots/ | UI output |
| **Property-Based** | proptest 1.4-1.5 | property_based_tests.rs | State invariants |
| **Golden Files** | Custom fixtures | tests/fixtures/ | Agent responses |
| **Schema Validation** | serde_json | Various | Telemetry, consensus |
| **Fixture Replay** | MockMcpManager | integration tests | MCP responses |

**Property-Based Tests (Current):**
- PB01: State index always in valid range
- PB02-PB10: Various state/consensus invariants
- 2,560+ generated test cases

**Snapshot Tests (Current):**
- VT100 terminal output for ChatWidget
- Slash command rendering
- Task status layouts

**Golden File Tests (Current):**
- `fixtures/consensus/demo-plan-claude.json`
- `fixtures/consensus/demo-plan-gemini-v3.json`
- `fixtures/spec_status/healthy/`, `conflict/`, `stale/`

#### Common Failure Modes

| Failure Mode | Description | Mitigation |
|--------------|-------------|------------|
| **Golden staleness** | Reference files outdated | Regular refresh; version tracking |
| **Snapshot noise** | Non-deterministic elements in snapshots | Mask timestamps, IDs |
| **Property blindness** | Properties too weak to catch bugs | Review properties quarterly |
| **AI output drift** | LLM responses change between versions | Semantic comparison; BERTScore |
| **Over-reliance on fixtures** | Real service diverges from fixtures | Periodic fixture refresh from production |
| **Mutation survival** | Injected bugs not caught | Mutation testing score tracking |

#### Recommended Regression Prevention Strategy

**Three-Layer Defense:**

1. **Property-Based Invariant Layer (Expand)**
   - Current: 10 properties, 2,560 cases
   - Target: 40+ properties covering all invariants
   - Focus: State transitions, evidence lifecycle, configuration validation

2. **Golden Master Layer (Maintain)**
   - Current: Fixture-based agent response replay
   - Enhancement: Add semantic drift detection for AI outputs
   - Process: Quarterly fixture refresh from real interactions

3. **Snapshot Layer (Refine)**
   - Current: insta for UI snapshots
   - Enhancement: Mask volatile fields (timestamps, UUIDs)
   - Process: Review snapshots on every PR

**AI-Specific Enhancements (2025):**
- For consensus outputs: JSON schema validation + semantic similarity
- Threshold: BERTScore > 0.85 for acceptable drift
- Log semantic diff on failure for debugging

#### Engineering Costs/Risks

| Technique | Implementation | Maintenance | Confidence | Speed |
|-----------|----------------|-------------|------------|-------|
| Characterization | Low | Medium | High | Fast |
| Property-Based | Medium | Low | High | Medium |
| Snapshot | Low | Medium | Medium | Fast |
| Contract Versioning | High | Medium | High | Medium |
| Mutation Testing | High | Low | Very High | Slow |

### D) Decision Considerations Checklist

1. [ ] **Property coverage**: Are all 10 SPEC.md invariants covered by properties?
2. [ ] **Fixture freshness**: When were agent fixtures last updated from real outputs?
3. [ ] **Snapshot masking**: Are volatile fields (timestamps, IDs) masked?
4. [ ] **AI drift handling**: Is there semantic comparison for LLM outputs?
5. [ ] **Mutation testing**: Should mutation testing be added to CI/CD?
6. [ ] **Contract evolution**: Are stage/gate contracts versioned?
7. [ ] **Property expansion**: What new properties should be added?
8. [ ] **Golden refresh cadence**: How often to update golden files?
9. [ ] **Refactoring safety**: Do current tests enable confident refactoring?
10. [ ] **Coverage metrics**: Is characterization coverage tracked?
11. [ ] **Semantic thresholds**: What BERTScore threshold for AI outputs?
12. [ ] **Approval workflow**: Is there snapshot review in PR process?

### E) Recommended Regression Prevention Framework

**Immediate (Session 7 findings):**
- Existing proptest, insta, and fixture patterns are sound
- Gap: No semantic comparison for AI outputs

**Near-term:**
1. Expand proptest to cover all 10 SPEC.md invariants
2. Add timestamp/UUID masking to snapshot tests
3. Establish quarterly fixture refresh process

**Future:**
1. Add BERTScore semantic comparison for consensus outputs
2. Consider mutation testing (cargo-mutants) for critical paths
3. Version contracts for stage transitions and gate interfaces

### Archive Evidence

| Spec ID | Path | Why Relevant |
|---------|------|--------------|
| property_based_tests.rs | `codex-rs/tui/tests/property_based_tests.rs` | Current proptest implementation |
| snapshots/ | `codex-rs/tui2/src/chatwidget/snapshots/` | Current insta snapshots |
| fixtures/ | `codex-rs/tui/tests/fixtures/` | Golden file examples |
| SPEC.md | `codex-rs/SPEC.md` | 10 invariants to cover with properties |

---

## 21. Open Unknowns / Spikes to Validate (17)

### Architectural Questions (Sessions 1-5)

1. **SPEC-931F Status**: Event sourcing was rejected in Nov 2025. Has anything changed (Memvid architecture, capsule model) that would revisit this decision?

2. **Productivity Claims**: Per METR study, AI tools made experienced devs 19% slower. Does Codex-RS have evidence for its productivity claims? SPEC-940 instrumentation may provide data.

3. **Tier 1 Disposability**: Can local-memory (Tier 1) be fully rebuilt from Memvid capsule? If not, what's lost? This affects whether projections should be in SOR.

4. **Capture Mode Adoption**: What percentage of users actually use `full_io` vs `prompts_only` vs `none`? This informs whether forensic depth is valued.

5. **CLI Feature Coverage**: Does `code speckit` have 100% parity with TUI spec-kit commands? Inventory needed.

### Maintenance Questions (Session 6)

6. **Scheduler Infrastructure Gap**: SPEC-KIT-103 defines cron schedules in config but no executor exists. What's the implementation path? Options: tokio-cron, system cron, or custom scheduler.

7. **Health Check Scope**: No observability jobs exist. What metrics should be checked? Options: Evidence size, memory count, last sync time, capsule integrity.

8. **SYNC-026 Dependency**: Compaction is backlog status. Is it blocked on other work? Can Evidence Cleanup + Librarian proceed independently?

9. **Librarian Resource Isolation**: Local LLM (Qwen 2.5 3B) is compute-intensive. Should Librarian run in-process or separate process to avoid TUI blocking?

10. **Event Infrastructure**: What events are already emitted by the pipeline that could trigger maintenance? Stage transitions? Evidence writes?

11. **Dry-run Parity**: Do all maintenance jobs support `--dry-run`? Librarian has it; Evidence Cleanup scripts have it. Need to verify consistency.

### Capability Testing Questions (Session 7)

12. **Evidence Archival Test Gap**: E.3 (archival >30 days) and E.4 (SHA256 integrity) capabilities have no test coverage. Are these features implemented but untested, or not implemented?

13. **Fixture Freshness**: When were MockMcpManager fixtures last updated from real agent outputs? Stale fixtures may mask integration issues.

14. **Semantic Comparison for AI**: Current golden file tests use exact match. Should BERTScore or cosine similarity be added for LLM output comparison to handle non-determinism?

15. **Property Coverage of Invariants**: Current proptest covers ~10 properties. Are all 10 SPEC.md invariants represented? Which invariants lack property-based coverage?

16. **Contract Definition**: Are inter-component contracts (Stage → Gate → Evidence) explicitly defined anywhere, or only implicit in integration tests?

17. **Capability Matrix Location**: Should a formal CAPABILITY_MATRIX.md be created? If so, who maintains it and how is it kept in sync with tests?

---

## 22. RESUME PROMPT

```markdown
Continue ARCHITECT_REVIEW_RESEARCH for Codex-RS/Spec-Kit.

## Session Context
- Session 8 of N
- Phase 0 (Archive Scan): COMPLETE
- A1-F2 (Product/Pipeline/Policy Questions): COMPLETE
- G1 (Cross-cutting Capability Verification): COMPLETE
- G2 (Capability Matrix Definition): COMPLETE
- G3 (Regression Prevention): COMPLETE

## Research Status
All primary research questions (A1-G3) are now COMPLETE.

## Next Steps (Choose one)
1. **Final Synthesis**: Create summary recommendations section consolidating all findings
2. **Research H (Additional Topics)**: If new questions emerged during G research
3. **Implementation Planning**: Convert findings into actionable SPEC proposals

## Key Findings from Session 7 (G1/G2/G3)

### G1 Recommendation: Hybrid Contract + Workflow Testing
- Keep existing W01-W15 workflow tests for comprehensive cross-component verification
- Add explicit contracts for Stage/Gate/Evidence interfaces
- Formalize traceability matrix mapping SPEC.md invariants to tests

### G2 Recommendation: Capability Matrix Structure
| Category | Capabilities | Test Coverage |
|----------|-------------|---------------|
| Pipeline (P) | Stage transitions, dependencies, skip | W01-W15 ✅ |
| Quality (Q) | Gates, checkpoints, escalation | Q01-Q10 ✅ |
| Evidence (E) | Lifecycle, limits, archival | Partial ⚠️ |
| State (S) | Persistence, recovery, concurrency | E01-E15, C01-C10 ✅ |
| Cross-cutting (X) | Telemetry, errors, security | Various ✅ |

**Gaps identified**: E.3 (archival), E.4 (integrity verification)

### G3 Recommendation: Three-Layer Regression Defense
1. **Property-Based Invariant Layer**: Expand proptest to 40+ properties covering all SPEC.md invariants
2. **Golden Master Layer**: Maintain fixture-based approach; add semantic drift detection
3. **Snapshot Layer**: Refine insta usage; mask volatile fields

### Testing Infrastructure Strengths
- 604 tests (unit: 135, integration: 256+, workflow: 60, property: 35)
- MockMcpManager, IntegrationTestContext, EvidenceVerifier patterns
- proptest + insta frameworks already in use

## Key Files Already Read
- All files from Sessions 1-6
- codex-rs/tui/tests/ (workflow, quality, property, edge case tests)
- codex-rs/tui/testing-policy.md (604 test summary)
- codex-rs/tui/tests/common/ (integration harness, mock MCP)
- codex-rs/tui/tests/fixtures/ (consensus, spec_status golden files)

## Research Output Options
1. Add section 23: Final Synthesis & Recommendations
2. Add sections for any new research questions
3. Mark research COMPLETE and archive document
```

---

*Generated by Architecture Review Board Researcher - Session 7*
*All primary questions (A1-G3) COMPLETE. Next: Final synthesis or implementation planning.*
