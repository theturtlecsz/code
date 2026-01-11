# PRD: Hermetic Agent Isolation (SPEC-KIT-964)

**Version**: v20251130-isolation-a
**Status**: Draft
**Author**: Claude (P63)
**Created**: 2025-11-30

## 1. Overview

### Problem Statement

Spawned agents (gemini, claude, code) can access global configuration files, leading to:

1. **Inconsistent behavior**: Different users' global configs produce different agent outputs
2. **Non-reproducible results**: Same spec-kit command yields different results across environments
3. **Config leakage**: User-specific paths (`/home/username/`, `~/.config/`) leak into prompts
4. **Hidden dependencies**: Agents silently depend on global templates instead of project defaults

**Current Config Layers (Problematic)**:
```
GLOBAL:    ~/.claude/CLAUDE.md, ~/.gemini/GEMINI.md, ~/.config/code/templates/
              ↓ (LEAKS INTO)
PROJECT:   ./CLAUDE.md, ./AGENTS.md, ./templates/
              ↓
SPAWNED:   Task agents (gemini, claude, code)
              ↓
OUTPUT:    Generated code (INCONSISTENT)
```

### Solution

Implement **hermetic agent isolation** - spawned agents receive ONLY:
1. Project-local instruction files (`./CLAUDE.md`, `./AGENTS.md`, `./GEMINI.md`)
2. Project-local templates (`./templates/`) OR embedded defaults
3. Explicit prompts from `prompts.json`
4. External tool calls scoped by project (for MCP tools, require project scoping; local-memory uses CLI/REST only)

**Target Architecture**:
```
┌─────────────────────────────────────────────────────────────┐
│                 HERMETIC AGENT SANDBOX                      │
├─────────────────────────────────────────────────────────────┤
│ ALLOWED:                                                    │
│   ./CLAUDE.md, ./AGENTS.md, ./GEMINI.md (project)          │
│   ./templates/* (project-local)                             │
│   [embedded templates] (binary)                             │
│   prompts.json (project-relative refs only)                 │
│   MCP tool calls with project:* scope (non-memory)          │
│   local-memory via CLI/REST scoped by domain/tags           │
├─────────────────────────────────────────────────────────────┤
│ BLOCKED:                                                    │
│   ~/.claude/*, ~/.gemini/*, ~/.config/code/* (global)      │
│   /home/*/* paths in prompts                                │
│   Unscoped MCP tool calls                                   │
│   local-memory over MCP                                     │
└─────────────────────────────────────────────────────────────┘
```

### Success Criteria

- [ ] Template resolution: `./templates/ > embedded` (global path removed)
- [ ] No user-specific paths in prompts.json
- [ ] Project instruction files required: CLAUDE.md, AGENTS.md, GEMINI.md
- [ ] local-memory operations scoped by domain/tags (CLI/REST only; no MCP)
- [ ] Pre-agent-spawn validation (runtime)
- [ ] Pre-commit hook validation
- [ ] CI workflow validation

## 2. Functional Requirements

### FR-1: Template Resolution Isolation

**Current** (lines 186-200 in `templates/mod.rs`):
```rust
// Resolution order:
// 1. Project-local: ./templates/{name}-template.md
// 2. User config: ~/.config/code/templates/{name}-template.md  <-- REMOVE
// 3. Embedded: Compiled-in default
```

**Target**:
```rust
// Resolution order:
// 1. Project-local: ./templates/{name}-template.md
// 2. Embedded: Compiled-in default (skip global entirely)
```

Changes required:
1. Remove `TemplateSource::UserConfig` variant from enum
2. Remove user config resolution from `resolve_template()`
3. Remove user config resolution from `resolve_template_source()`
4. Change `install_templates()` to install to `./templates/` (project-local)
5. Update documentation and comments

### FR-2: Project Instruction File Parity

All spec-kit projects MUST have three instruction files:

| File | Purpose | Content |
|------|---------|---------|
| `CLAUDE.md` | Claude Code instructions | Spec-kit commands, memory policy, tooling |
| `AGENTS.md` | Multi-agent documentation | Agent roster, tiers, consensus workflow |
| `GEMINI.md` | Gemini CLI instructions | Mirror of CLAUDE.md for Gemini users |

These files should be **nearly identical** to ensure any LLM working in the project has equivalent context.

### FR-3: prompts.json Sanitization

Scan and remove any:
- Hardcoded user paths (`/home/*/`, `/Users/*/`)
- Global config references (`~/.config/`, `~/.claude/`, `~/.gemini/`)
- User-specific environment variables

Replace with:
- Project-relative paths (`./templates/`, `./docs/`)
- `${TEMPLATE:name}` syntax (resolved at runtime)
- Environment-agnostic references

### FR-4: Local-memory Project Scoping (No MCP)

All local-memory operations should include project context:

**Storage**:
```bash
lm remember "..." \
  --importance 8 \
  --tags "project:theturtlecsz/code,spec:SPEC-KIT-964,type:decision"
```

**Queries**:
```bash
lm search "..." --tags "project:theturtlecsz/code" --limit 10
```

This enables:
- Global memory store (shared infrastructure)
- Project-scoped queries (isolation)
- Cross-project knowledge when explicitly requested

### FR-5: Validation Script

Create `scripts/validate-config-isolation.sh`:

```bash
#!/bin/bash
# SPEC-KIT-964: Config Isolation Validation
#
# Checks:
# 1. No user-specific paths in prompts.json
# 2. Project instruction files exist (CLAUDE.md, AGENTS.md, GEMINI.md)
# 3. Template resolution doesn't hit global paths
# 4. Agent prompts are hermetic
#
# Exit codes:
# 0 - All checks pass
# 1 - Validation failures found
```

### FR-6: Pre-Agent-Spawn Validation

Add runtime check in `handler.rs` before spawning agents:

```rust
fn validate_agent_isolation() -> Result<(), IsolationError> {
    // 1. Check project instruction files exist
    // 2. Verify no global template override active
    // 3. Validate prompt contains no user paths
    // 4. Return error if any check fails
}
```

### FR-7: Pre-Commit Hook Integration

Add to `.githooks/pre-commit`:
```bash
# Config isolation check
scripts/validate-config-isolation.sh || exit 1
```

### FR-8: CI Workflow

Add `.github/workflows/config-isolation.yml`:
```yaml
name: Config Isolation
on: [push, pull_request]
jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Validate config isolation
        run: scripts/validate-config-isolation.sh
```

## 3. Non-Functional Requirements

### NFR-1: Zero Global Dependencies

Spawned agents must function identically regardless of:
- User's home directory contents
- Global config file presence/absence
- User-specific environment variables

### NFR-2: Reproducibility

Same spec-kit command + same project state = identical agent behavior

### NFR-3: Fail-Fast

Isolation violations detected at:
- **Pre-spawn**: Runtime error before agent starts
- **Pre-commit**: Block commit with clear error
- **CI**: Fail PR merge

### NFR-4: Backward Compatibility

- Existing projects continue to work
- Warn if relying on global templates (migration path)
- Grace period before hard enforcement

## 4. Implementation Phases

### Phase 1: Template Resolution (30 min)
- Remove global path from `templates/mod.rs`
- Update `TemplateSource` enum
- Change `install_templates()` target
- Update tests

### Phase 2: Project Instruction Parity (30 min)
- Create `GEMINI.md` (mirrors CLAUDE.md)
- Verify CLAUDE.md, AGENTS.md, GEMINI.md consistency
- Add GEMINI-template.md to embedded templates

### Phase 3: Validation Script (30 min)
- Create `scripts/validate-config-isolation.sh`
- Add checks for paths, files, templates
- Test against current codebase

### Phase 4: Pre-Agent-Spawn Check (30 min)
- Add `validate_agent_isolation()` to handler.rs
- Integrate into agent spawn flow
- Add `--skip-isolation-check` escape hatch for debugging

### Phase 5: Local-memory Project Scoping (30 min)
- Ensure all memory stores include `project:*` tag (and `type:*`)
- Ensure search defaults to current project scope unless explicitly widened
- Document CLI/REST usage (no MCP) + scoping pattern

### Phase 6: Hook & CI Integration (15 min)
- Add to pre-commit hook
- Create CI workflow
- Document in CLAUDE.md

## 5. Testing Plan

### Unit Tests
- [ ] `test_template_resolution_skips_global()` - verify no UserConfig source
- [ ] `test_isolation_validation_catches_user_paths()` - detect `/home/*/`
- [ ] `test_local_memory_calls_include_project_scope()` - verify tag present

### Integration Tests
- [ ] Spawn agent with global config present - verify ignored
- [ ] Spawn agent with only project config - verify works
- [ ] Run validation script on clean project - passes
- [ ] Run validation script with user path in prompts - fails

### Manual Verification
- [ ] Fresh clone + spec-kit commands work without any global setup
- [ ] Two users with different global configs get identical results

## 6. Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking existing workflows | High | Warn before enforce, migration docs |
| Performance overhead from validation | Low | Cache validation results |
| Scope enforcement breaks cross-project queries | Medium | Explicit `--all-projects` flag |
| False positives in path detection | Medium | Allowlist for legitimate paths |

## 7. Future Considerations

- **Project fingerprinting**: Auto-detect project from git remote/hash
- **Isolation levels**: Strict (error) vs Permissive (warn)
- **Audit log**: Track isolation violations for debugging

## 8. Model & Runtime (Spec Overrides)

Policy: docs/MODEL-POLICY.md (version: 1.0.0)

This spec is **infrastructure-only** (config isolation/hermetic sandboxing) and does not invoke model routing directly.
Roles exercised: none (no Architect/Implementer/Librarian/Tutor/Judge).
Privacy: local_only = true (enforces isolation, no cloud calls from this component)
Guardrails still apply: sandboxing (this spec defines sandboxing), evidence/logging.

---

## 9. References

- SPEC-KIT-961: Template Ecosystem (parity requirements)
- SPEC-KIT-962: Template Installation Architecture
- `codex-rs/tui/src/templates/mod.rs`: Current implementation
- `docs/spec-kit/prompts.json`: Agent prompt definitions

---

Back to [Key Docs](../KEY_DOCS.md)
