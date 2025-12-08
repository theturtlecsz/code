# Architect Sidecar Feature Parity Analysis

_Generated: 2025-12-08 | Reference: Advanced Repository Ingestion for LLM Context Augmentation_

## Executive Summary

This document compares our current Architect Sidecar implementation against the reference architecture for "god-level" codebase understanding in LLM contexts. The reference defines a 6-stage pipeline using specialized tools for structural mapping, visualization, forensics, and semantic linking.

**Current State**: Phase 2.5 complete - native Rust harvester + Mermaid visualization.

**Completed**:
- P103: Native Rust harvester (churn.rs, complexity.rs, skeleton.rs)
- P104: Mermaid call graph + graph_bridge documentation (CodeGraphContext is Python-only)

**Next**: P105 - NotebookLM HTTP Service integration (see `docs/PROMPT-P105-NLM-SERVICE.md`)

---

## Feature Parity Matrix

### Stage 1: Structural Mapping (The Skeleton)

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| XML-structured context | Repomix | `skeleton.rs` (XML output) | Partial | Missing Tree-sitter compression, secretlint |
| .gitignore respect | Repomix | `complexity.rs` (hardcoded excludes) | Partial | Not configurable, no .repomixignore |
| Graph-ranked importance | Aider repo-map | Not implemented | Missing | Need PageRank-style symbol ranking |
| Secret detection | secretlint | Not implemented | Missing | Security gap for cloud uploads |
| Token budget optimization | Aider | Not implemented | Missing | No dynamic context sizing |

### Stage 2: Architectural Visualization (The Nervous System)

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| Control flow graphs | Sirens Call | `mermaid.rs` (P104) | **Complete** | Call graph extraction via tree-sitter |
| Module dependencies | Pymermaider | `mermaid.rs` (P104) | **Complete** | Module import analysis |
| Class diagrams | Pymermaider | Not implemented | Missing | Need Rust struct/impl hierarchy |
| C4 model DSL | Structurizr | Not implemented | Missing | Need architectural boundary inference |
| Symbol bindings | Stack Graphs | Not implemented | Missing | Need precise name resolution |

### Stage 3: Behavioral Forensics (The History)

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| Churn hotspots | Hercules/GitNStats | `churn.rs` | Complete | Native git2, 12-month window |
| Logical coupling | Hercules | `churn.rs` | Complete | Co-change matrix with min threshold |
| Code burndown | Hercules | Not implemented | Missing | Need temporal survival analysis |
| Author/ownership | AskGit | Not implemented | Missing | Need bus factor calculation |
| SQL querying | AskGit | Not implemented | Missing | Consider rusqlite integration |

### Stage 4: Quantitative Metrics (The Health)

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| LOC/SLOC counting | SCC | `complexity.rs` | **Complete** | Native implementation |
| Cyclomatic complexity | rust-code-analysis | `graph_bridge.rs` (Python only) | Partial | CodeGraphContext only parses Python |
| Call graph complexity | Custom | `mermaid.rs` (P104) | **Complete** | Edge count per function |
| Cognitive complexity | Complexipy | Not implemented | Missing | Need human-readability scoring |
| Halstead metrics | rust-code-analysis | Not implemented | Missing | Low priority |
| COCOMO estimation | SCC | Not implemented | Missing | Low priority |

### Stage 5: Semantic Navigation (The Brain)

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| AST parsing | Tree-sitter | `skeleton.rs` | Complete | Rust parser, TS/Python pending |
| Symbol extraction | Dossier | `skeleton.rs` | Partial | Public API only, no usage tracking |
| Definition resolution | Stack Graphs | Not implemented | Missing | Critical for precise RAG |
| Cross-file references | Stack Graphs | Not implemented | Missing | Need scope-aware binding |

### Stage 6: Integration Pipeline

| Feature | Reference Tool | Our Implementation | Status | Gap Analysis |
|---------|---------------|-------------------|--------|--------------|
| NotebookLM upload | CLI | `architect_cmd.rs` | Complete | Native + legacy modes |
| Artifact synthesis | Custom | Not implemented | Missing | Need unified output format |
| Caching/incremental | Custom | `.repo_hash` only | Partial | Need per-module staleness |

---

## Implementation Priority Matrix

Based on gap analysis and value/effort ratio:

### P0 - Critical (COMPLETED in P103/P104)
1. ~~**CodeGraphContext MCP Integration**~~ - `graph_bridge.rs` documented as Python-only
   - Discovery: CodeGraphContext only parses Python files
   - For Rust: Use native `mermaid.rs` instead

2. ~~**Mermaid.js Call Graph Generation**~~ - `mermaid.rs` **COMPLETE**
   - Call graph extraction via tree-sitter
   - Module dependency analysis
   - Output: `call_graph.mmd`, `module_deps.mmd`

### P0.5 - Critical (Next Session: P105)
3. **NotebookLM HTTP Service Integration** - `nlm_service.rs`
   - HTTP client replacing CLI spawning
   - Service lifecycle management (start/stop/status)
   - Auto-upload artifacts on refresh
   - See: `docs/PROMPT-P105-NLM-SERVICE.md`

### P1 - High Value
4. **PageRank Symbol Ranking** - Aider-style importance scoring
   - Use `mermaid.rs` call graph data
   - Output: Ranked symbol list with centrality scores

5. **Class Diagram Generation** - Pymermaider equivalent
   - Extract struct/impl hierarchy via tree-sitter
   - Output: `class_hierarchy.mmd`

### P2 - Medium Value
6. **Author/Ownership Analysis** - Bus factor calculation
   - Extend `churn.rs` with author tracking
   - Output: Knowledge silo warnings

7. **Secret Detection** - Security hardening
   - Integrate with existing lint infrastructure
   - Block sensitive content from NotebookLM uploads

### P3 - Future
8. **Stack Graphs Integration** - Precise name binding
9. **Cognitive Complexity** - Human-readability scoring
10. **C4 Model Inference** - Architectural boundary detection

---

## Tool Mapping: Reference vs. Implementation

```
Reference Pipeline          Our Implementation
─────────────────────────────────────────────────────
[Repomix]          ──────►  skeleton.rs (complete)
[Aider repo-map]   ──────►  mermaid.rs (complete) - call graph ranking
[Sirens Call]      ──────►  mermaid.rs (complete) - call graph viz
[Pymermaider]      ──────►  mermaid.rs (partial) - module deps done
[Structurizr]      ──────►  NOT IMPLEMENTED (P3)
[Hercules]         ──────►  churn.rs (complete)
[AskGit]           ──────►  NOT IMPLEMENTED (P2)
[SCC]              ──────►  complexity.rs (complete)
[Stack Graphs]     ──────►  NOT IMPLEMENTED (P3)
[Complexipy]       ──────►  NOT IMPLEMENTED (P2)
[NotebookLM]       ──────►  nlm_service.rs (P105 - next)
```

---

## Architecture Decisions

### Why Native Rust vs. External Tools?

| Decision | Rationale |
|----------|-----------|
| git2 over Hercules | Zero runtime deps, single binary, better error handling |
| tree-sitter over ast-grep | Already in codex-core, consistent parsing |
| CodeGraphContext MCP | Existing Neo4j infrastructure, avoids tool sprawl |
| XML skeleton output | NotebookLM parses XML better than JSON for structure |

### Trade-offs Accepted

1. **No SQL querying** - git2 provides sufficient history access
2. **No secretlint** - Defer to user's existing CI/CD security
3. **No COCOMO** - Low value for architectural understanding

---

## Session History

### P104 (Completed - 2025-12-08)
**Objective**: Mermaid visualization + CodeGraphContext integration

**Discoveries**:
- CodeGraphContext MCP only parses Python (not Rust)
- Need native tree-sitter approach for Rust call graphs

**Deliverables**:
- `mermaid.rs` - Call graph extraction (5885 functions, 36537 edges from codex-rs)
- `mermaid.rs` - Module dependency analysis (366 modules, 1042 imports)
- CLI flags: `--graph`, `--mermaid`, `--focus`, `--depth`
- `graph_bridge.rs` documented as Python-only

### P105 (Next Session)
**Objective**: NotebookLM HTTP Service Integration

See `docs/PROMPT-P105-NLM-SERVICE.md` for full continuation prompt.

**Key Tasks**:
1. `nlm_service.rs` - HTTP client with lazy service spawning
2. `code architect service start/stop/status` - Lifecycle management
3. Auto-upload artifacts on refresh
4. Full research API (fast + deep)

---

## References

- Reference Architecture: "Advanced Repository Ingestion and Semantic Mapping for LLM Context Augmentation"
- Repomix: https://github.com/yamadashy/repomix
- Aider: https://github.com/paul-gauthier/aider
- Hercules: https://github.com/src-d/hercules
- Stack Graphs: https://github.com/github/stack-graphs
