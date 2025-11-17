# SPEC-DOC-005: Development & Contribution Guide

**Status**: Pending
**Priority**: P1 (Medium)
**Estimated Effort**: 10-14 hours
**Target Audience**: Contributors, maintainers
**Created**: 2025-11-17

---

## Objectives

Provide complete guide for developers contributing to the project:
1. Development environment setup
2. Build system (profiles, fast builds, cross-compilation)
3. Git workflow (branching, commits, PR process)
4. Code style (rustfmt, clippy, lints)
5. Pre-commit hooks (setup, bypass, debugging)
6. Upstream sync process (quarterly merge)
7. Adding new commands (command registry, routing, handlers)
8. Debugging guide (logs, tmux, MCP, agent issues)
9. Release process (versioning, changelog, Homebrew)

---

## Scope

### In Scope

- Dev environment setup (Rust toolchain, Node.js, MCP servers)
- Build system (Cargo profiles: dev-fast, release, perf)
- Git workflow (conventional commits, branching strategy)
- Code style enforcement (rustfmt, clippy --all-targets --all-features)
- Pre-commit hooks (setup-hooks.sh, .githooks/)
- Upstream sync (quarterly merge, conflict resolution, UPSTREAM-SYNC.md)
- Adding slash commands (command registry pattern)
- Debugging techniques (logs, tmux sessions, MCP debugging)
- Release process (versioning, changelog generation, Homebrew formula)

### Out of Scope

- Architecture details (see SPEC-DOC-002)
- Testing guidelines (see SPEC-DOC-004)
- User-facing documentation (see SPEC-DOC-001)

---

## Deliverables

1. **content/development-setup.md** - Environment, dependencies, tools
2. **content/build-system.md** - Cargo profiles, fast builds, cross-compilation
3. **content/git-workflow.md** - Branching, commits, PRs, conventional commits
4. **content/code-style.md** - rustfmt, clippy, lints, guidelines
5. **content/pre-commit-hooks.md** - Setup, debugging, bypass
6. **content/upstream-sync.md** - Quarterly merge process
7. **content/adding-commands.md** - Command registry, routing, examples
8. **content/debugging-guide.md** - Logs, tmux, MCP, agents
9. **content/release-process.md** - Versioning, changelog, publishing

---

## Success Criteria

- [ ] New contributor can set up dev environment in 30 minutes
- [ ] Build system documented with all profiles
- [ ] Git workflow clearly explained
- [ ] Pre-commit hooks setup guide complete
- [ ] Adding commands tutorial with working example
- [ ] Debugging techniques comprehensive

---

## Related SPECs

- SPEC-DOC-000 (Master)
- SPEC-DOC-002 (Core Architecture - for deep understanding)
- SPEC-DOC-004 (Testing - for testing contributions)

---

**Status**: Structure defined, content pending
