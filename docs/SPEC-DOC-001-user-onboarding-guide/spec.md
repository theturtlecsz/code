# SPEC-DOC-001: User Onboarding & Getting Started Guide

**Status**: Pending
**Priority**: P0 (High)
**Estimated Effort**: 8-12 hours
**Target Audience**: New users
**Created**: 2025-11-17

---

## Objectives

Create comprehensive onboarding documentation that enables new users to:
1. Install the codex CLI on their system (npm, Homebrew, or from source)
2. Complete first-time setup (authentication, configuration)
3. Execute their first AI-assisted coding task within 15 minutes
4. Understand common workflows (spec-kit automation vs manual coding)
5. Troubleshoot common issues independently
6. Find answers to frequently asked questions

---

## Scope

### In Scope

**Installation Guide**:
- npm installation (recommended)
- Homebrew installation (macOS/Linux)
- Building from source (all platforms)
- System requirements and dependencies
- Verification steps

**First-Time Setup**:
- API key configuration (OpenAI, Anthropic, Google)
- auth.json setup
- Basic config.toml configuration
- MCP server setup (local-memory, git-status)
- Workspace initialization

**Quick Start Tutorial** (5-minute walkthrough):
- Interactive chat mode
- Running first spec-kit command (/speckit.new)
- Understanding agent responses
- Basic file operations

**Common Workflows**:
- Spec-kit automation (full pipeline /speckit.auto)
- Manual coding assistance (chat mode)
- Code review and refactoring
- Running tests and validation

**Troubleshooting Guide**:
- Installation errors
- Authentication issues
- MCP connection problems
- Agent execution failures
- Performance issues
- Common configuration mistakes

**FAQ**:
- Model selection and switching
- Cost management
- Offline usage
- Privacy and data handling
- Customization options
- Comparison with other tools (Cursor, Copilot)

### Out of Scope

- Advanced configuration (see SPEC-DOC-006)
- Internal architecture details (see SPEC-DOC-002)
- Contributing to the project (see SPEC-DOC-005)
- Security deep-dive (see SPEC-DOC-007)

---

## Deliverables

### Primary Documentation

1. **installation.md** - Comprehensive installation guide
2. **first-time-setup.md** - Setup walkthrough
3. **quick-start.md** - 5-minute tutorial
4. **workflows.md** - Common usage patterns
5. **troubleshooting.md** - Error resolution guide
6. **faq.md** - Frequently asked questions

### Supporting Materials

- **evidence/screenshots/** - Installation screenshots, UI examples
- **evidence/terminal-examples/** - Command outputs, typical workflows

---

## Success Criteria

- [ ] New user can install within 5 minutes
- [ ] New user can complete setup within 10 minutes
- [ ] New user can run first command within 15 minutes total
- [ ] Troubleshooting guide addresses 90%+ common errors
- [ ] FAQ answers 30+ common questions
- [ ] All installation paths tested (npm, Homebrew, source)
- [ ] All screenshots current and clear

---

## Related SPECs

- SPEC-DOC-000 (Master)
- SPEC-DOC-002 (Architecture - for advanced users)
- SPEC-DOC-003 (Spec-Kit Framework - detailed command reference)
- SPEC-DOC-006 (Configuration - advanced customization)

---

**Status**: Structure defined, content pending
