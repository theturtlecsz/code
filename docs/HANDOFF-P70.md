# P70 Session Handoff

**Previous**: P69 (deferred) | **Spec**: SPEC-KIT-099 | **Status**: Architecture Complete, Implementation Ready

---

## Session Context

### What We Built (P69→P70)
- **SPEC-KIT-099**: Research-to-Code Context Bridge
- Complete technical specification at `docs/SPEC-KIT-099-context-bridge/spec.md`
- 4-stage integrated pipeline architecture (Research → Plan → Tasks → Implement → Validate)
- Four Pillars implementation blueprint

### Design Decisions (Locked)
| Decision | Choice |
|----------|--------|
| MCP Strategy | `ask_question` + JSON prompt (use existing tool) |
| Injection Style | Aggressive (`!!! SYSTEM OVERRIDE !!!`) |
| Reference Rot | Pause with modal (user must acknowledge) |
| Schema Complexity | **TBD** - needs research on NotebookLM JSON capability |

### Deferred from P69
These items are queued but lower priority than SPEC-KIT-099:

| Task | Effort | Description |
|------|--------|-------------|
| Pre-commit cleanup | 10 min | Commit P68 (SPEC-KIT-971 clarify modal) |
| Minimal clarify tests | 15 min | Add marker regex test |
| Question customization | 45 min | Project-type-aware questions |
| Ferris benchmark | 30 min | Benchmark against reference |

---

## P70 Implementation Roadmap

### Phase 1: Schema Research & Decision (30 min)
**Goal**: Resolve the schema complexity question

1. Test NotebookLM's ability to return structured JSON:
   ```
   /speckit.research test query to evaluate JSON output capability
   ```
2. Evaluate if NotebookLM can reliably distinguish:
   - CRITICAL vs HIGH vs MEDIUM constraints
   - Or if we should infer from language ("MUST" vs "SHOULD")
3. **Decision point**: Choose `Vec<String>` or `Vec<Constraint>`

### Phase 2: Core Data Layer (2-3 hours)
**Files to create**:
```
codex-rs/core/src/research/
├── mod.rs           # pub mod schema; pub mod validator; pub mod persistence;
├── schema.rs        # ResearchBrief, Adr, CodeAnchor structs
├── validator.rs     # Semantic hashing with syn, Reference Rot detection
└── persistence.rs   # .code/context/ file operations
```

**Dependencies to add** (`core/Cargo.toml`):
```toml
syn = { version = "2", features = ["full", "parsing"] }
quote = "1"  # For signature serialization
```

**Verification**: Unit test that detects function signature change

### Phase 3: Validation Engine (2-3 hours)
**Key implementation**:
- `validate_anchors(brief, repo_root) -> Vec<ValidationWarning>`
- Recursive AST traversal (top-level + `impl` blocks + nested modules)
- SHA-256 semantic hashing of normalized signatures

**Critical fix from analysis**: Add `impl` block traversal:
```rust
Item::Impl(impl_block) => {
    for impl_item in &impl_block.items {
        if let ImplItem::Fn(method) = impl_item { ... }
    }
}
```

### Phase 4: MCP Integration (2-3 hours)
**Strategy**: Use `ask_question` with structured JSON prompt

```rust
let query = format!(r#"
Analyze this PRD and return ONLY a JSON object (no explanation):
{{
  "constraints": ["string array of rules"],
  "architectural_decisions": [{{ "id": "ADR-001", "title": "...", ... }}],
  "code_snippets": [{{ "file_path": "...", "target_symbol": "...", "signature_hash": "..." }}]
}}

PRD Content:
{prd_content}
"#);
```

**MCP call pattern** (via Session, not Widget):
```rust
session.call_tool("notebooklm", "ask_question", json!({
    "question": query,
    "notebook_id": config.default_notebook_id
}), timeout).await
```

### Phase 5: Research Stage Executor (3-4 hours)
**Integration with PipelineCoordinator**:
- Add `ResearchStageExecutor` for Stage 0
- Circuit breaker with configurable `on_mcp_unavailable` and `on_reference_rot`
- Reference Rot → show modal, pause pipeline

**Files to modify**:
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs`
- `tui/src/chatwidget/spec_kit/state.rs` (add `active_research_brief`)

### Phase 6: Prompt Injection (2-3 hours)
**Modify**: `core/src/slash_commands.rs`

```rust
pub fn format_subagent_command(
    name: &str,
    task: &str,
    agents: Option<&[AgentConfig]>,
    commands: Option<&[SubagentCommandConfig]>,
    research_brief: Option<&ResearchBrief>,  // NEW
) -> SubagentResolution
```

**Injection format**:
```
!!! SYSTEM OVERRIDE: ACTIVE !!!
You are operating in STRICT ARCHITECTURAL MODE.
The following JSON is the AUTHORITATIVE SPECIFICATION.
...
```

### Phase 7: Command & UI (2-3 hours)
**Create**: `tui/src/chatwidget/spec_kit/commands/research.rs`
- `/speckit.research` command (alias `/research`)
- Manual sync for outside-pipeline use

**Create**: Reference Rot modal
- Display warnings with file paths and hash mismatches
- "Continue Anyway" / "Abort" buttons

### Phase 8: Testing & Polish (2-3 hours)
- Unit tests for schema serialization
- Unit tests for validator (signature change detection)
- Integration test with mock MCP response
- Circuit breaker behavior tests

---

## Key Files Reference

### To Create
| File | Purpose |
|------|---------|
| `core/src/research/mod.rs` | Module exports |
| `core/src/research/schema.rs` | ResearchBrief, Adr, CodeAnchor |
| `core/src/research/validator.rs` | Semantic hashing, Reference Rot |
| `core/src/research/persistence.rs` | .code/context/ operations |
| `tui/src/chatwidget/spec_kit/commands/research.rs` | /speckit.research command |
| `tui/src/chatwidget/spec_kit/research_stage.rs` | Stage 0 executor |
| `tui/src/bottom_pane/reference_rot_modal.rs` | Rot warning modal |

### To Modify
| File | Change |
|------|--------|
| `core/Cargo.toml` | Add syn, quote dependencies |
| `core/src/lib.rs` | Add `pub mod research;` |
| `core/src/slash_commands.rs` | Add ResearchBrief parameter to format_subagent_command |
| `core/src/config_types.rs` | Add ContextBridgeConfig, ResearchStageConfig |
| `tui/src/chatwidget/spec_kit/mod.rs` | Export new modules |
| `tui/src/chatwidget/spec_kit/command_registry.rs` | Register ResearchCommand |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Add Stage 0 |
| `tui/src/chatwidget/spec_kit/state.rs` | Add active_research_brief field |

---

## Continuation Prompt

```
load docs/HANDOFF-P70.md

## P70 Session: SPEC-KIT-099 Implementation

I'm implementing the Research-to-Code Context Bridge. The spec is complete at:
docs/SPEC-KIT-099-context-bridge/spec.md

### Locked Decisions
- MCP: Use `ask_question` with JSON prompt (no new tool)
- Injection: Aggressive `!!! SYSTEM OVERRIDE !!!` framing
- Reference Rot: Pause with modal

### Open Decision
- Schema: Need to test if NotebookLM can output structured severity levels
  - If yes: Use `Vec<Constraint>` with severity
  - If no: Use `Vec<String>` (simple)

### Current Phase
Start with **Phase 1: Schema Research** - test NotebookLM JSON capability

Then proceed through Phases 2-8 in order:
2. Core Data Layer (schema.rs, validator.rs, persistence.rs)
3. Validation Engine (syn AST parsing, impl block support)
4. MCP Integration (ask_question with JSON prompt)
5. Research Stage Executor (Stage 0 of pipeline)
6. Prompt Injection (modify format_subagent_command)
7. Command & UI (/speckit.research, Reference Rot modal)
8. Testing & Polish

Build incrementally. Run `cargo check -p codex-core` after each phase.
```

---

## Quick Commands

```bash
# Build & verify
~/code/build-fast.sh              # Fast build
cargo check -p codex-core         # Check core changes
cargo test -p codex-core research # Run research module tests

# Run TUI
~/code/build-fast.sh run

# Test NotebookLM JSON capability (in TUI)
# Use MCP tool directly to test structured output
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                    /speckit.auto SPEC-KIT-XXX                       │
├─────────────────────────────────────────────────────────────────────┤
│  STAGE 0: RESEARCH                                                  │
│  ┌───────────────┐    ┌──────────────┐    ┌──────────────────────┐ │
│  │ ask_question  │───▶│ Parse JSON   │───▶│ Validate Anchors     │ │
│  │ (NotebookLM)  │    │ Response     │    │ (syn + SHA-256)      │ │
│  └───────────────┘    └──────────────┘    └──────────────────────┘ │
│         │                    │                      │               │
│         │                    │           ┌──────────┴────────────┐  │
│         │                    │           │ Reference Rot?        │  │
│         │                    │           │ YES → Pause + Modal   │  │
│         │                    │           │ NO  → Continue        │  │
│         │                    │           └───────────────────────┘  │
│         │                    ▼                      │               │
│         │           .code/context/active_brief.json │               │
│         │                    │                      │               │
│         └────────────────────┴──────────────────────┘               │
│                              │                                      │
│  STAGES 1-4: Plan/Tasks/Implement/Validate                         │
│  ┌──────────────────────────────────────────────────────────────┐  │
│  │  !!! SYSTEM OVERRIDE: ACTIVE !!!                             │  │
│  │  [ResearchBrief injected at TOP of every agent prompt]       │  │
│  └──────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```
