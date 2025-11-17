# SPEC-DOC-008: API & Extension Development Guide

**Status**: Pending (Deferred until MAINT-10)
**Priority**: P3 (Future Consideration)
**Estimated Effort**: 12-16 hours
**Target Audience**: Plugin developers, integrators
**Created**: 2025-11-17

---

## Objectives

**Note**: This SPEC is deferred until MAINT-10 (spec-kit extraction as standalone crate) has strategic justification.

Future objectives when activated:
1. Spec-kit public API documentation (when extracted as separate crate)
2. MCP server development guide (creating custom MCP servers)
3. Custom slash command development (command registry, handlers)
4. Plugin architecture (if implemented in future)
5. Rust API documentation (rustdoc organization, public APIs)
6. TypeScript CLI wrapper API (npm package integration)
7. Integration examples (CI/CD, editors, automation workflows)

---

## Scope

### In Scope (When Activated)

- Spec-kit public API (post-MAINT-10 extraction)
- MCP server development (custom server creation, tool definitions)
- Custom slash command development (command registry pattern, examples)
- Plugin architecture (if/when designed and implemented)
- Rust API documentation (public crate APIs, rustdoc conventions)
- TypeScript wrapper API (npm package programmatic usage)
- Integration examples (GitHub Actions, VS Code, CI/CD pipelines)

### Out of Scope

- Internal implementation details (see SPEC-DOC-002)
- Spec-kit usage guide (see SPEC-DOC-003)
- Contributing to core (see SPEC-DOC-005)

---

## Deliverables (Future)

1. **content/spec-kit-api.md** - Public API reference (post-MAINT-10)
2. **content/mcp-server-development.md** - Custom MCP server guide
3. **content/custom-commands.md** - Slash command development
4. **content/plugin-architecture.md** - Plugin system (if implemented)
5. **content/rust-api-reference.md** - Rustdoc organization, public APIs
6. **content/typescript-api.md** - npm package programmatic usage
7. **content/integration-examples.md** - CI/CD, editors, automation

---

## Success Criteria (When Activated)

- [ ] Spec-kit API fully documented (post-MAINT-10)
- [ ] MCP server development tutorial complete
- [ ] Custom command example working and tested
- [ ] Integration examples for GitHub Actions, VS Code
- [ ] Rustdoc properly organized for public APIs

---

## Deferral Rationale

**Why Deferred**:
- MAINT-10 (spec-kit extraction) currently lacks strategic justification
- No public API exists yet (spec-kit is integrated into TUI)
- Plugin architecture not designed
- Limited demand for programmatic API usage

**Activation Triggers**:
- MAINT-10 approved and in progress
- Multiple requests for programmatic integration
- Plugin ecosystem demand emerges
- Spec-kit as standalone library becomes strategic

---

## Related SPECs

- SPEC-DOC-000 (Master)
- SPEC-DOC-002 (Core Architecture - internal APIs)
- SPEC-DOC-003 (Spec-Kit Framework - user-facing documentation)
- SPEC-DOC-005 (Development - internal contribution)

---

**Status**: Structure defined, deferred until MAINT-10
