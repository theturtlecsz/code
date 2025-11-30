# SPEC-KIT-964: Hermetic Isolation Architecture

Hermetic isolation ensures spawned agents operate in controlled, reproducible environments independent of user-specific global configurations.

## Problem Statement

Multi-agent pipelines can fail or produce inconsistent results when:
- Agents rely on `~/.config/code/templates/` which varies between users
- Global configurations override project-specific settings
- Missing instruction files (CLAUDE.md, AGENTS.md, GEMINI.md) cause silent failures

## Design Principles

1. **Project-local first**: Templates resolve from `./templates/` before embedded
2. **No global fallback**: `~/.config/code/templates/` intentionally excluded
3. **Pre-spawn validation**: Check instruction files exist before agent spawn
4. **Graceful degradation**: Warn on missing files but don't block execution

## Template Resolution Order

```
1. Project-local:  ./templates/{name}-template.md
2. Embedded:       Compiled into binary (always available)

❌ NOT checked:    ~/.config/code/templates/ (breaks hermeticity)
```

**Implementation**: `codex-rs/tui/src/templates/mod.rs`

```rust
pub fn resolve_template(name: &str) -> String {
    // 1. Check project-local first
    let local_path = format!("./templates/{}-template.md", name);
    if let Ok(content) = fs::read_to_string(&local_path) {
        return content;
    }
    // 2. Fall back to embedded (always succeeds)
    get_embedded(name).unwrap_or_default().to_string()
}
```

## Pre-Spawn Validation

Before spawning agents, the orchestrator validates hermetic isolation:

**Required instruction files**:
- `CLAUDE.md` - Claude/Anthropic agent instructions
- `AGENTS.md` - Multi-agent coordination rules
- `GEMINI.md` - Google Gemini agent instructions

**Implementation**: `codex-rs/tui/src/chatwidget/spec_kit/isolation_validator.rs`

```rust
const REQUIRED_INSTRUCTION_FILES: &[&str] = &["CLAUDE.md", "AGENTS.md", "GEMINI.md"];

pub fn validate_agent_isolation(cwd: &Path) -> Result<(), IsolationError> {
    if !cwd.exists() {
        return Err(IsolationError::InvalidWorkingDirectory);
    }
    for file in REQUIRED_INSTRUCTION_FILES {
        if !cwd.join(file).exists() {
            return Err(IsolationError::MissingInstructionFile(file.to_string()));
        }
    }
    Ok(())
}
```

**Call site**: `agent_orchestrator.rs:767`

```rust
// SPEC-KIT-964 Phase 6: Validate hermetic isolation before spawning agents
if let Err(e) = super::isolation_validator::validate_agent_isolation_with_skip(cwd) {
    tracing::warn!("{} SPEC-KIT-964: Isolation validation failed: {}", run_tag, e);
    // Log warning but don't block execution
}
```

## Environment Variables

| Variable | Values | Effect |
|----------|--------|--------|
| `SPEC_KIT_SKIP_ISOLATION` | `1`, `true`, `yes` | Skip pre-spawn validation |

**Use cases**:
- Development/testing without full project scaffold
- CI environments with minimal setup
- Legacy projects during migration

## Project Scaffolding

Create required instruction files with `/speckit.project`:

```bash
/speckit.project rust my-project    # Creates with Rust-specific CLAUDE.md
/speckit.project python my-project  # Creates with Python-specific CLAUDE.md
/speckit.project go my-project      # Creates with Go-specific CLAUDE.md
```

**Generated files**:
```
my-project/
├── CLAUDE.md       # Agent instructions (from CLAUDE-template.md)
├── AGENTS.md       # Multi-agent rules (from AGENTS-template.md)
├── GEMINI.md       # Gemini instructions (from GEMINI-template.md)
├── SPEC.md         # Task tracker
├── docs/           # Documentation root
├── memory/
│   └── constitution.md
└── [type-specific files]
```

## Embedded Templates

14 templates compiled into binary:

| Category | Templates |
|----------|-----------|
| **Stages** | plan, tasks, implement, validate, audit, unlock |
| **Quality Gates** | clarify, analyze, checklist |
| **Documents** | prd, spec |
| **Instructions** | claude, agents, gemini |

## Testing

```bash
# Run isolation validator tests
cd codex-rs && cargo test -p codex-tui -- isolation_validator

# Run project scaffolding tests (validates template resolution)
cd codex-rs && cargo test -p codex-tui -- project_native::tests
```

**Test coverage**:
- `test_validate_missing_directory` - Invalid cwd handling
- `test_validate_missing_instruction_files` - Empty directory fails
- `test_validate_partial_instruction_files` - Partial files detected
- `test_validate_all_files_present` - Success case

## Migration Guide

For existing projects without instruction files:

1. **Quick scaffold**: `/speckit.project generic .` (adds missing files)
2. **Manual creation**: Copy templates from `./templates/` directory
3. **Skip validation**: Set `SPEC_KIT_SKIP_ISOLATION=1` (not recommended)

## Related Specs

- **SPEC-KIT-961**: Template Ecosystem & Multi-Agent Parity
- **SPEC-KIT-962**: Template Installation & Distribution Architecture
- **SPEC-KIT-960**: `/speckit.project` Command
