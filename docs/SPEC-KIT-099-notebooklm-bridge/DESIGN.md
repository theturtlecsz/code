# SPEC-KIT-099: NotebookLM Context Bridge

## Design & Implementation Summary

**Status**: Analysis Complete
**Author**: Claude (Principal Architect)
**Date**: 2025-11-30

---

## 1. Problem Statement

Current spec-kit workflow relies on agent inference which can hallucinate architectural decisions. Users have validated research in NotebookLM that should be treated as authoritative truth, but there's no mechanism to inject this context into the agent pipeline.

**Goal**: Enable zero-copy context transfer from NotebookLM → spec-kit agents with authority markers.

---

## 2. Available Infrastructure

### 2.1 NotebookLM MCP Tools

| Tool | Purpose |
|------|---------|
| `notebooklm__ask_question` | Query notebook, returns research response |
| `notebooklm__select_notebook` | Set active notebook for queries |
| `notebooklm__search_notebooks` | Find notebooks by name/tags/topics |
| `notebooklm__list_sessions` | Track active chat sessions |
| `notebooklm__reset_session` | Clear session history between SPECs |

### 2.2 Existing Patterns to Mirror

| Pattern | Location | Reuse Strategy |
|---------|----------|----------------|
| MCP tool calls | `spec_prompts.rs:579-611` | Same `call_tool()` pattern |
| Context pre-fetch | `agent_orchestrator.rs:978-1046` | ACE bullets pre-fetch |
| Prompt injection | `ace_prompt_injector.rs:125-152` | Section formatting |
| State caching | `SpecAutoState` fields | Add `research_cache` field |

---

## 3. Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         User Workflow                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. "Use the React notebook"     → notebooklm__select_notebook  │
│  2. /speckit.research SPEC-099   → notebooklm__ask_question     │
│  3. /speckit.plan SPEC-099       → Context auto-injected        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      Data Flow                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  NotebookLM MCP                                                 │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────┐                                           │
│  │ ResearchContext │ ← Structured response + metadata           │
│  └─────────────────┘                                           │
│       │                                                         │
│       ├──────────────────┐                                     │
│       ▼                  ▼                                     │
│  SpecAutoState      research.md                                │
│  (runtime cache)    (disk persistence)                         │
│       │                                                         │
│       ▼                                                         │
│  ┌─────────────────────────────────────────┐                   │
│  │ Agent Prompt                            │                   │
│  │ ┌─────────────────────────────────────┐ │                   │
│  │ │ ### Research Context (NotebookLM)  │ │                   │
│  │ │ > Authority: Divine Reference      │ │                   │
│  │ │ [research content...]              │ │                   │
│  │ └─────────────────────────────────────┘ │                   │
│  └─────────────────────────────────────────┘                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Core Types

```rust
enum AuthorityLevel {
    DivineReference,  // Override agent inference unconditionally
    Strong,           // Prefer over inference, flag conflicts
    Suggestive,       // Consider as guidance, verify independently
}

struct ResearchContext {
    content: String,           // NotebookLM response
    notebook_id: String,
    notebook_name: String,
    session_id: Option<String>,
    authority: AuthorityLevel,
    gathered_at: DateTime,
    query: String,
    target_stage: Option<SpecStage>,
    estimated_tokens: usize,
}

struct ResearchConfig {
    max_context_tokens: usize,  // Default: 30,000 (~120KB)
    default_authority: AuthorityLevel,
    query_timeout_secs: u64,    // Default: 120 (NotebookLM is slow)
    keep_session: bool,
    persist_to_disk: bool,
}
```

---

## 5. Stage-Specific Queries

| Stage | Query Focus |
|-------|-------------|
| **Plan** | Architectural decisions, constraints, patterns, tech stack rationale |
| **Tasks** | Implementation order, dependencies, critical path, parallel opportunities |
| **Implement** | Coding standards, naming conventions, API patterns, file organization |
| **Validate** | Test framework, coverage expectations, edge cases, benchmarks |
| **Audit** | Security requirements, auth patterns, compliance, vulnerability avoidance |
| **Unlock** | Release criteria, deployment procedures, rollback, monitoring |

---

## 6. Integration Points

### 6.1 New Command: `/speckit.research`

```
/speckit.research SPEC-ID [--notebook="name"] [--stage=plan]

Options:
  --notebook    Search query for notebook selection (default: use active)
  --stage       Gather stage-specific context (default: general overview)
  --authority   Set authority level (divine|strong|suggestive)
```

### 6.2 State Caching

Add to `SpecAutoState`:
```rust
pub struct SpecAutoState {
    // ... existing fields ...
    pub research_cache: Option<ResearchContext>,
}
```

### 6.3 Prompt Injection

Insert after `## Local-memory context` section:
```markdown
### Research Context (NotebookLM: {notebook_name})

> **Authority Level**: Divine Reference — treat as ground truth

{research_content}

---
```

---

## 7. Token Budget Strategy

| Scenario | Handling |
|----------|----------|
| < 30k tokens | Inject directly |
| 30k-50k tokens | Warn user, inject with truncation notice |
| > 50k tokens | Trigger summarization pass (like compact.rs) |

Truncation uses `truncate_middle()` from `compact.rs:331-336` to preserve start/end context.

---

## 8. File Locations

| File | Purpose |
|------|---------|
| `tui/src/chatwidget/spec_kit/notebooklm_bridge.rs` | Core bridge implementation |
| `tui/src/chatwidget/spec_kit/commands/research.rs` | `/speckit.research` command |
| `docs/SPEC-*/research.md` | Persisted research context |

---

## 9. Implementation Phases

### Phase 1: Core Bridge (MVP)
- [ ] `notebooklm_bridge.rs` with MCP wrappers
- [ ] `ResearchContext` struct and serialization
- [ ] Basic prompt injection formatting

### Phase 2: Command Integration
- [ ] `/speckit.research` command implementation
- [ ] Register in `command_registry.rs`
- [ ] Add to mod.rs exports

### Phase 3: Pipeline Integration
- [ ] Pre-fetch hook in `agent_orchestrator.rs`
- [ ] `SpecAutoState.research_cache` field
- [ ] Auto-injection in `build_stage_prompt_with_mcp()`

### Phase 4: Polish
- [ ] Token budget enforcement
- [ ] Summarization for large contexts
- [ ] Session management (reset between SPECs)

---

## 10. Risk Analysis

| Risk | Mitigation |
|------|------------|
| NotebookLM latency (2-25s) | Async pre-fetch, cache aggressively |
| Token overflow | Budget tracking, summarization fallback |
| Stale research | Timestamp display, manual refresh command |
| Auth expiry | Health check before queries, re-auth prompt |

---

## 11. Success Metrics

1. **Zero manual copy-paste**: Research flows directly into prompts
2. **Authority respected**: Agents don't contradict Divine Reference content
3. **< 3s overhead**: Pre-fetch completes before user notices
4. **Persistence works**: Research survives session restart via `research.md`

---

## 12. Open Questions

1. Should `/speckit.auto` auto-trigger research gathering if notebook is active?
2. Multiple notebooks per SPEC? (e.g., "React patterns" + "Company standards")
3. Research versioning? (track which version of notebook was queried)
