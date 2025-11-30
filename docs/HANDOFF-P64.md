# P64 SYNC CONTINUATION PROMPT

_Generated: 2025-11-30_
_Previous Session: P63_
_Commit: Uncommitted (P63 changes pending)_

---

## CRITICAL DISCOVERY (End of P63)

**AGENTS.md is NOT just documentation** - it's the instruction file for the Codex TUI (`code` binary):

From `core/prompt.md`:
```
- Repos often contain AGENTS.md files
- The contents of the AGENTS.md file at the root of the repo are included with the developer message
- For every file you touch, you must obey instructions in any AGENTS.md file
```

**Current problem**: Our AGENTS.md is structured as multi-agent system documentation, NOT operational instructions like CLAUDE.md/GEMINI.md.

**Required fix**:
1. Move current AGENTS.md content → `docs/spec-kit/MULTI-AGENT-SYSTEM.md`
2. Restructure AGENTS.md to mirror CLAUDE.md/GEMINI.md format

**Instruction file parity**:
| File | Loaded By | Purpose |
|------|-----------|---------|
| `CLAUDE.md` | Claude Code CLI | Operational instructions |
| `GEMINI.md` | Gemini CLI | Operational instructions |
| `AGENTS.md` | Codex TUI (`code`) | Operational instructions |

---

## Session Context

**Completed This Session (P63):**

### SPEC-KIT-964: Hermetic Agent Isolation (Phases 1-5)
- **Phase 1**: PRD created at `docs/SPEC-KIT-964-config-isolation/PRD.md`
- **Phase 2**: Template resolution modified - removed global `~/.config/code/templates/`
  - `templates/mod.rs`: Removed `TemplateSource::UserConfig`, updated resolution functions
  - `spec_prompts.rs`: Removed UserConfig match arm
  - `chatwidget/spec_kit/commands/templates.rs`: Updated display
  - `prompts.json`: Updated description to reflect hermetic system
- **Phase 3**: GEMINI.md created (project-level, mirrors CLAUDE.md)
- **Phase 4**: CLAUDE.md updated with Section 12 (Config Isolation)
- **Phase 5**: `scripts/validate-config-isolation.sh` created (6 checks, all passing)

**Build Status**: `cargo check -p codex-tui` passes

---

## PRIORITY 0: AGENTS.md Restructuring (CRITICAL)

**This must be done first** - AGENTS.md is currently loaded by Codex TUI but has wrong structure.

### Step 1: Move Documentation (10 min)
```bash
# Move current AGENTS.md to documentation location
mv AGENTS.md docs/spec-kit/MULTI-AGENT-SYSTEM.md
```

### Step 2: Create Operational AGENTS.md (30 min)

Create new `AGENTS.md` that mirrors CLAUDE.md/GEMINI.md structure:

```markdown
# AGENTS.md — How Codex TUI Works In This Repo

## Repository Context
[Same as CLAUDE.md]

## 0. Prerequisites & Known Limitations
[Same as CLAUDE.md]

## 0.1 Model Guidance
[Adapted for GPT-5/Codex models]

## 1. Load These References Every Session
[Same as CLAUDE.md]

## 2. Operating Modes & Slash Commands
[Same as CLAUDE.md]

... [Sections 3-12 mirror CLAUDE.md] ...

## 12. Config Isolation (SPEC-KIT-964)
[Same as CLAUDE.md]
```

### Step 3: Update References (10 min)

Files that reference AGENTS.md as documentation need updating:
- `CLAUDE.md` Section 1: "AGENTS.md (this document's partner)"
- `docs/spec-kit/TEMPLATES.md`: May reference AGENTS.md
- Update to point to `docs/spec-kit/MULTI-AGENT-SYSTEM.md` for system docs

### Step 4: Update Validation Script (5 min)

`scripts/validate-config-isolation.sh` checks instruction file parity - verify it still works after restructure.

---

## PRIORITY 1: Complete SPEC-KIT-964 (Phases 6-8)

### Phase 6: Pre-Agent-Spawn Runtime Check (30 min)

**Decision**: Hard fail on isolation violations

Add to `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`:

```rust
/// SPEC-KIT-964: Validate hermetic isolation before spawning agents
fn validate_agent_isolation() -> Result<(), IsolationError> {
    // 1. Check project instruction files exist
    let required = ["CLAUDE.md", "AGENTS.md", "GEMINI.md"];
    for file in required {
        if !Path::new(file).exists() {
            return Err(IsolationError::MissingInstructionFile(file.to_string()));
        }
    }

    // 2. Verify no global template override is active
    // (Template resolution already handles this, but verify)

    // 3. Return Ok or hard fail
    Ok(())
}
```

**Integration point**: Call before `spawn_agent()` in multi-agent stages

### Phase 7: MCP Project Scoping (30 min)

**Architecture Decision (from P63)**:
- Global memory store (shared infrastructure)
- Project-scoped queries via `project:*` tag
- Cross-project access when explicitly requested

**Implementation**:
1. Update memory storage calls to include `project:theturtlecsz/code` tag
2. Update search queries to filter by project tag
3. Add `--all-projects` flag for explicit cross-project queries

**Key files**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` (memory operations)
- `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` (orchestration)

### Phase 8: Pre-Commit Hook Integration (15 min)

Add to `.githooks/pre-commit`:
```bash
# SPEC-KIT-964: Config isolation validation
if [[ -f "scripts/validate-config-isolation.sh" ]]; then
    scripts/validate-config-isolation.sh || exit 1
fi
```

---

## PRIORITY 2: Complete SPEC-KIT-961 (Remaining Phases)

### Phase 4: Template Validation Script (30 min)

**Note**: May overlap with 964's validation script. Consider merging or keeping separate.

Create `scripts/validate-templates.sh`:
```bash
#!/bin/bash
# Validates:
# 1. All 11 embedded templates compile (syntax check)
# 2. ${TEMPLATE:name} references resolve
# 3. Project-local templates match expected structure
# 4. GEMINI-template.md exists (new requirement)
```

### Phase 5: GEMINI-template.md for /speckit.project (20 min)

**Decision**: Add GEMINI-template.md to embedded templates

1. Create `templates/GEMINI-template.md` (mirror of CLAUDE-template.md)
2. Add to `templates/mod.rs`:
   ```rust
   pub const GEMINI: &str = include_str!("../../../../templates/GEMINI-template.md");
   ```
3. Update `get_embedded()` to include "gemini" mapping
4. Update `template_names()` to include "gemini"
5. Update `project_native.rs` to scaffold GEMINI.md

### Phase 6: Go Template Support (30 min)

Add Go project type to `/speckit.project`:

```rust
// In project_native.rs
ProjectType::Go => {
    // go.mod
    // main.go or cmd/{name}/main.go
    // internal/ directory structure
    // Makefile
}
```

### Phase 7: ACE Playbook Integration Docs (20 min)

Document in `docs/spec-kit/ACE-PLAYBOOK.md`:
- `playbook_slice` patterns for spec-kit stages
- `learn` feedback patterns for consensus outcomes
- Integration with local-memory for playbook persistence

---

## Key Files Reference

| File | Purpose | Status |
|------|---------|--------|
| `codex-rs/tui/src/templates/mod.rs` | Template resolution | Modified (964) |
| `codex-rs/tui/src/spec_prompts.rs` | Prompt rendering | Modified (964) |
| `codex-rs/tui/src/chatwidget/spec_kit/handler.rs` | Orchestration | Needs Phase 6 |
| `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` | MCP integration | Needs Phase 7 |
| `.githooks/pre-commit` | Git hooks | Needs Phase 8 |
| `templates/GEMINI-template.md` | New template | Needs 961 Phase 5 |
| `codex-rs/tui/src/chatwidget/spec_kit/commands/project_native.rs` | Project scaffolding | Needs Go support |

---

## Uncommitted Changes (P63)

```
M  AGENTS.md                              (template isolation docs)
M  CLAUDE.md                              (Section 12 added)
M  codex-rs/tui/src/templates/mod.rs      (hermetic resolution)
M  codex-rs/tui/src/spec_prompts.rs       (UserConfig removed)
M  codex-rs/tui/src/chatwidget/spec_kit/commands/templates.rs
M  docs/spec-kit/prompts.json             (description updated)
?? GEMINI.md                              (new - project-level)
?? docs/SPEC-KIT-964-config-isolation/    (new PRD)
?? scripts/validate-config-isolation.sh   (new validation script)
```

**Recommendation**: Commit P63 changes before starting P64 work.

---

## SPEC Status Summary

| SPEC | Status | Notes |
|------|--------|-------|
| SPEC-KIT-960 | Done | /speckit.project command |
| SPEC-KIT-961 | In Progress | 3 phases remaining (4, 5-7) |
| SPEC-KIT-962 | Done | Template installation architecture |
| SPEC-KIT-963 | Done | Upstream command deprecation |
| SPEC-KIT-964 | In Progress | 3 phases remaining (6-8) |

---

## Quick Start Commands

```bash
# Load this handoff
load docs/HANDOFF-P64.md

# Commit P63 changes first
git add -A && git commit -m "feat(spec-kit): Hermetic agent isolation (SPEC-KIT-964 phases 1-5)

- Remove global template resolution (~/.config/code/templates/)
- Create project-level GEMINI.md for agent parity
- Add validation script (6 checks)
- Update CLAUDE.md, AGENTS.md with isolation docs

SPEC-KIT-964, SPEC-KIT-961"

# Then start Phase 6 (pre-spawn check)
# Read handler.rs to find spawn_agent integration point
```

---

## Decision Log (P63)

| Decision | Rationale |
|----------|-----------|
| Hard fail on isolation violations | User preference - strict enforcement |
| GEMINI-template.md to be added | User confirmed - include in scaffolding |
| ~~No CODE.md needed~~ | ~~AGENTS.md serves as universal reference~~ |
| **AGENTS.md = instruction file** | Discovered: Codex TUI loads AGENTS.md like Claude loads CLAUDE.md |
| **Restructure AGENTS.md** | Must mirror CLAUDE.md/GEMINI.md format for tri-agent parity |
| Go template support | User selected as priority |
| ACE playbook docs | User selected as priority |
| Skip CI workflow for now | User did not select |

---

## Acceptance Criteria for P64 Session

**Priority 0 (CRITICAL):**
1. [ ] AGENTS.md restructured to operational format (mirrors CLAUDE.md)
2. [ ] Current AGENTS.md content moved to `docs/spec-kit/MULTI-AGENT-SYSTEM.md`
3. [ ] References updated (CLAUDE.md, etc.)

**Priority 1 (SPEC-KIT-964):**
4. [ ] P63 changes committed
5. [ ] Phase 6: Pre-agent-spawn isolation check in handler.rs
6. [ ] Phase 7: MCP project scoping implemented
7. [ ] Phase 8: Pre-commit hook integration
8. [ ] SPEC-KIT-964 marked Done

**Priority 2 (SPEC-KIT-961):**
9. [ ] GEMINI-template.md added to embedded templates
10. [ ] AGENTS-template.md added to embedded templates (for /speckit.project)
11. [ ] Go template support added to /speckit.project
12. [ ] ACE playbook integration documented
13. [ ] SPEC-KIT-961 phases updated/completed

**Wrap-up:**
14. [ ] P65 handoff created (or session complete)

---

## Architecture Reference

### Hermetic Isolation Model (SPEC-KIT-964)

```
┌─────────────────────────────────────────────────────────────┐
│                 HERMETIC AGENT SANDBOX                      │
├─────────────────────────────────────────────────────────────┤
│ ALLOWED:                                                    │
│   ./CLAUDE.md, ./AGENTS.md, ./GEMINI.md (project)          │
│   ./templates/* (project-local)                             │
│   [embedded templates] (binary)                             │
│   prompts.json (project-relative refs only)                 │
│   MCP queries with project:theturtlecsz/code scope          │
├─────────────────────────────────────────────────────────────┤
│ BLOCKED:                                                    │
│   ~/.claude/*, ~/.gemini/*, ~/.config/code/* (global)      │
│   /home/*/* paths in prompts                                │
│   Unscoped MCP queries                                      │
└─────────────────────────────────────────────────────────────┘
```

### Validation Layers

```
Layer 1: Pre-commit hook (scripts/validate-config-isolation.sh)
    ↓
Layer 2: Pre-agent-spawn runtime check (handler.rs)
    ↓
Layer 3: Template resolution (mod.rs - already hermetic)
```

---

## Estimated Effort

| Priority | Phase | Task | Time |
|----------|-------|------|------|
| **P0** | - | Move AGENTS.md to docs/spec-kit/ | 10 min |
| **P0** | - | Create operational AGENTS.md | 30 min |
| **P0** | - | Update references | 10 min |
| P1 | 964-6 | Pre-spawn check | 30 min |
| P1 | 964-7 | MCP project scoping | 30 min |
| P1 | 964-8 | Pre-commit hook | 15 min |
| P2 | 961-5 | GEMINI-template.md | 20 min |
| P2 | 961-5 | AGENTS-template.md | 10 min |
| P2 | 961-6 | Go template | 30 min |
| P2 | 961-7 | ACE playbook docs | 20 min |
| **Total** | | | **~3.5 hours** |
