# Architect Sidecar Feature Parity Analysis

_Generated: 2025-12-08 | Reference: Advanced Repository Ingestion for LLM Context Augmentation_

## Executive Summary

This document compares our current Architect Sidecar implementation against the reference architecture for "god-level" codebase understanding in LLM contexts. The reference defines a 6-stage pipeline using specialized tools for structural mapping, visualization, forensics, and semantic linking.

**Current State**: Phase 2 complete - native Rust harvester with churn, complexity, and skeleton modules.

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
| Control flow graphs | Sirens Call | Not implemented | Missing | Need Mermaid.js CFG generation |
| Class diagrams | Pymermaider | Not implemented | Missing | Need Rust/TS/Python class extraction |
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
| LOC/SLOC counting | SCC | `complexity.rs` | Complete | Native implementation |
| Cyclomatic complexity | rust-code-analysis | `graph_bridge.rs` (stub) | Placeholder | Need CodeGraphContext MCP |
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

### P0 - Critical (Next Session)
1. **CodeGraphContext MCP Integration** - `graph_bridge.rs` full implementation
   - Cyclomatic complexity queries
   - Call graph analysis
   - Dead code detection
   - Replaces need for external rust-code-analysis

2. **Mermaid.js CFG Generation** - Control flow visualization
   - Leverage existing tree-sitter infrastructure
   - Output: `call_graph.mmd` for NotebookLM

### P1 - High Value
3. **PageRank Symbol Ranking** - Aider-style importance scoring
   - Use CodeGraphContext call graph data
   - Output: Ranked symbol list with centrality scores

4. **Class Diagram Generation** - Pymermaider equivalent
   - Extract inheritance relationships via tree-sitter
   - Output: `class_hierarchy.mmd`

### P2 - Medium Value
5. **Author/Ownership Analysis** - Bus factor calculation
   - Extend `churn.rs` with author tracking
   - Output: Knowledge silo warnings

6. **Secret Detection** - Security hardening
   - Integrate with existing lint infrastructure
   - Block sensitive content from NotebookLM uploads

### P3 - Future
7. **Stack Graphs Integration** - Precise name binding
8. **Cognitive Complexity** - Human-readability scoring
9. **C4 Model Inference** - Architectural boundary detection

---

## Tool Mapping: Reference vs. Implementation

```
Reference Pipeline          Our Implementation
─────────────────────────────────────────────────────
[Repomix]          ──────►  skeleton.rs (partial)
[Aider repo-map]   ──────►  NOT IMPLEMENTED (P1)
[Sirens Call]      ──────►  graph_bridge.rs (planned)
[Pymermaider]      ──────►  NOT IMPLEMENTED (P1)
[Structurizr]      ──────►  NOT IMPLEMENTED (P3)
[Hercules]         ──────►  churn.rs (complete)
[AskGit]           ──────►  NOT IMPLEMENTED (P2)
[SCC]              ──────►  complexity.rs (complete)
[Stack Graphs]     ──────►  NOT IMPLEMENTED (P3)
[Complexipy]       ──────►  NOT IMPLEMENTED (P2)
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

## Next Session Objectives (P104)

1. Implement `graph_bridge.rs` with full CodeGraphContext MCP integration
2. Add Mermaid.js call graph output to harvester
3. Create unified artifact format for NotebookLM synthesis
4. Performance benchmark: native Rust vs. Python scripts

---

## References

- Reference Architecture: "Advanced Repository Ingestion and Semantic Mapping for LLM Context Augmentation"
- Repomix: https://github.com/yamadashy/repomix
- Aider: https://github.com/paul-gauthier/aider
- Hercules: https://github.com/src-d/hercules
- Stack Graphs: https://github.com/github/stack-graphs
