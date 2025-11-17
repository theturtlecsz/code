# SPEC-DOC-006: Configuration & Customization Guide

**Status**: Pending
**Priority**: P1 (Medium)
**Estimated Effort**: 8-12 hours
**Target Audience**: Power users, advanced users
**Created**: 2025-11-17

---

## Objectives

Comprehensive guide to configuring and customizing the codex CLI:
1. Configuration file structure (config.toml complete schema)
2. 5-tier precedence (CLI, shell, profile, TOML, defaults)
3. Model configuration (providers, reasoning, profiles)
4. Agent configuration (5 agents, subagent commands, quality gates)
5. Quality gate customization (per-checkpoint agent selection)
6. Hot-reload configuration (config_reload.rs, 300ms debounce)
7. MCP server configuration (server definitions, lifecycle)
8. Environment variables (CODEX_HOME, API keys, overrides)
9. Templates (installation, customization, versioning)
10. Theme system (TUI themes, accessibility)

---

## Scope

### In Scope

- Complete config.toml reference (all sections)
- 5-tier precedence system (with examples)
- Model provider configuration (OpenAI, Anthropic, Google, Ollama)
- Agent configuration (gemini, claude, code, gpt_pro, gpt_codex)
- Quality gate customization (per-checkpoint overrides)
- Hot-reload mechanism (300ms debounce, watch system)
- MCP server definitions (local-memory, git-status, hal, custom)
- Environment variables (CODEX_HOME, *_API_KEY, SPEC_OPS_*)
- Template customization (installing, modifying, versioning)
- TUI theme customization (colors, accessibility)

### Out of Scope

- Architecture of config system (see SPEC-DOC-002)
- Installation and setup (see SPEC-DOC-001)
- Security of secrets (see SPEC-DOC-007)

---

## Deliverables

1. **content/config-reference.md** - Complete config.toml schema
2. **content/precedence-system.md** - 5-tier precedence with examples
3. **content/model-configuration.md** - Provider setup, reasoning effort
4. **content/agent-configuration.md** - 5 agents, subagent commands
5. **content/quality-gate-customization.md** - Per-checkpoint overrides
6. **content/hot-reload.md** - Config reload mechanism, debouncing
7. **content/mcp-servers.md** - MCP server definitions, custom servers
8. **content/environment-variables.md** - All env vars, overrides
9. **content/template-customization.md** - Installing, modifying templates
10. **content/theme-system.md** - TUI themes, accessibility options

---

## Success Criteria

- [ ] Complete config.toml schema documented
- [ ] 5-tier precedence clearly explained with examples
- [ ] All agent configurations documented
- [ ] Quality gate customization guide complete
- [ ] Environment variables comprehensive list
- [ ] Template customization tutorial complete

---

## Related SPECs

- SPEC-DOC-000 (Master)
- SPEC-DOC-001 (User Onboarding - basic config)
- SPEC-DOC-002 (Core Architecture - config system internals)
- SPEC-DOC-003 (Spec-Kit - quality gate config)

---

**Status**: Structure defined, content pending
