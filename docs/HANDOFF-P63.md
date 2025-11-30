# P63 SYNC CONTINUATION PROMPT

_Generated: 2025-11-30_
_Previous Session: P62_
_Commit: 5e3f6fa82_

---

## Session Context

**Completed This Session (P62):**
- SPEC-KIT-963: Upstream Command Deprecation - **DONE**
  - Removed `/plan`, `/solve`, `/code` commands
  - Fork now uses `/speckit.*` namespace exclusively
  - -153 net lines, 27 commands (was 30)
  - Commit: `5e3f6fa82`

---

## PRIORITY 1: Complete SPEC-KIT-961 (All Remaining Phases)

**Status:** Phases 1-2, 5 complete. 5 phases remaining.

### Phase Breakdown

| Phase | Task | Effort | Details |
|-------|------|--------|---------|
| **3** | AGENTS.md + .gemini/ parity | 1h | Update multi-agent documentation, ensure Gemini IDE config sync |
| **4** | Template validation script | 1h | CI-ready script to validate template syntax and completeness |
| **6** | Go template support | 0.5h | Add Go project type to /speckit.project (if needed) |
| **7** | ACE playbook integration | 1h | Document playbook_slice/learn patterns for templates |
| **8** | local-memory patterns | 0.5h | Document template storage/retrieval in local-memory |

### Phase 3 Details (AGENTS.md + .gemini/)

**AGENTS.md Updates Required:**
```markdown
- Document all 13 /speckit.* commands with agent routing
- Document tiered model strategy (Tier 0-4)
- Add template system section
- Remove any /plan /solve /code references
```

**IMPORTANT - .gemini/settings.json Parity Check:**
The `.gemini/` directory contains Gemini IDE integration:
```json
{
  "model": {"name": "gemini-2.5-flash"},
  "hooks": {
    "pre_command": "/home/thetu/.claude/hooks/session_start.sh",
    "post_tool_use": {"command": "/home/thetu/.claude/hooks/lm-precommit.sh"}
  },
  "mcpServers": {
    "local-memory": {...},
    "notebooklm": {...}
  }
}
```

Ensure AGENTS.md documents:
1. How spec-kit works with Gemini CLI (`gemini-2.5-flash`, `gemini-2.5-pro`)
2. Shared hooks between Claude Code and Gemini IDE
3. MCP server configuration parity

### Phase 4 Details (Validation Script)

Create `scripts/validate-templates.sh`:
```bash
#!/bin/bash
# Validates:
# 1. All 11 embedded templates compile (syntax check)
# 2. ${TEMPLATE:name} references resolve
# 3. Project-local templates match expected structure
# 4. Exit code suitable for CI
```

### Key Files

- `docs/SPEC-KIT-961-template-ecosystem/PRD.md` - Full requirements
- `docs/spec-kit/TEMPLATES.md` - Template reference (created P60)
- `AGENTS.md` - Needs multi-agent parity updates
- `.gemini/settings.json` - Gemini IDE config to sync
- `codex-rs/tui/src/templates/mod.rs` - Template implementation

---

## PRIORITY 2: Cleanup Old Handoffs

**Action:** Delete superseded handoff files:
```bash
rm docs/HANDOFF-P58.md docs/HANDOFF-P60.md docs/HANDOFF-P61.md docs/HANDOFF-P62.md
```

These are superseded by P63. Don't commit to git - just delete.

---

## Current Repository State

### Recent Commits
```
5e3f6fa82 feat(spec-kit): Remove upstream /plan /solve /code (SPEC-KIT-963)
c75ff86ba feat(spec-kit): Complete SPEC-KIT-962 template installation
e76582339 docs: Add P59 handoff with template ecosystem analysis
55ca08937 fix(spec-kit): Complete /speckit.project templates (SPEC-KIT-960)
```

### Uncommitted Files
```
M  CLAUDE.md                              (minor, from P62)
M  docs/SPEC-KIT-960-speckit-project/PRD.md
?? .gemini/                               (Gemini IDE config)
?? docs/HANDOFF-P58.md through P62.md     (TO DELETE)
?? docs/SPEC-KIT-961-template-ecosystem/
?? docs/spec-kit/TEMPLATES.md
```

### SPEC Status Summary

| SPEC | Status | Notes |
|------|--------|-------|
| SPEC-KIT-960 | Done | /speckit.project command |
| SPEC-KIT-961 | In Progress | Template ecosystem (5 phases remaining) |
| SPEC-KIT-962 | Done | Template installation architecture |
| SPEC-KIT-963 | Done | Upstream command deprecation |

---

## Quick Start Commands

```bash
# Load this handoff
load docs/HANDOFF-P63.md

# Delete old handoffs first
rm docs/HANDOFF-P58.md docs/HANDOFF-P60.md docs/HANDOFF-P61.md docs/HANDOFF-P62.md

# Then start Phase 3
# Read current AGENTS.md
cat AGENTS.md

# Check .gemini/ config
cat .gemini/settings.json
```

---

## Architecture Reference

### Template Resolution (SPEC-KIT-962)
```
Priority Order:
1. ./templates/{name}-template.md        (project-local)
2. ~/.config/code/templates/{name}.md    (user config, XDG)
3. [embedded]                            (compiled in binary)
```

### Tiered Model Strategy (SPEC-KIT-957)
```
Tier 0: Native Rust       (0 agents, $0, <1s)
Tier 1: Single Agent      (1 agent: gpt5-low, ~$0.10, 3-5 min)
Tier 2: Multi-Agent       (2-3 agents, ~$0.35, 8-12 min)
Tier 3: Premium           (3 premium agents, ~$0.80, 10-12 min)
Tier 4: Full Pipeline     (~$2.70, 45-50 min)
```

### Command Count After SPEC-KIT-963
- Total: 27 commands
- SpecKit namespace: 16 (/speckit.*)
- Guardrail namespace: 7 (/guardrail.*)
- Utility: 4 (/spec-*)

---

## Decision Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-11-30 | SPEC-KIT-963 committed | Clean break before 961 completion |
| 2025-11-30 | Delete old handoffs | Superseded, reduce noise |
| 2025-11-30 | .gemini/ parity in Phase 3 | User flagged Gemini IDE config needs sync |
| 2025-11-30 | All 961 phases in order | Complete template ecosystem systematically |

---

## Acceptance Criteria for P63 Session

1. [ ] Old handoffs deleted (P58-P62)
2. [ ] Phase 3: AGENTS.md updated with multi-agent parity
3. [ ] Phase 3: .gemini/ config documented/synced
4. [ ] Phase 4: validate-templates.sh created and tested
5. [ ] Phase 6: Go template support added (if applicable)
6. [ ] Phase 7: ACE playbook patterns documented
7. [ ] Phase 8: local-memory template patterns documented
8. [ ] SPEC-KIT-961 marked Done in SPEC.md
9. [ ] P64 handoff created (or session complete)
