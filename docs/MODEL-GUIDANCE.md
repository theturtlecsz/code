# Model-Specific Guidance

_Extended reasoning and validation guidance for different AI models._

**For commands and project structure**: See `CLAUDE.md`, `AGENTS.md`, or `GEMINI.md` in repo root.

---

## Claude Opus 4.5

**Model ID**: `claude-opus-4-5-20251101`

### Extended Thinking

Use deep reasoning (`ultrathink`) for:
- Architecture decisions affecting >3 files
- Multi-agent consensus synthesis and conflict resolution
- Complex debugging with multiple hypotheses
- Security audit and compliance review
- Refactors touching shared interfaces

Use standard reasoning for:
- Single-file changes and bug fixes
- Documentation updates
- Status queries and diagnostics
- Direct tool operations

### Judgment Trust

Opus 4.5 has improved instruction following and nuanced decision-making. Guidelines are **principles, not absolute rules**. When context clearly warrants deviation:
1. Document your reasoning briefly
2. Proceed with the appropriate action
3. The goal is quality outcomes, not mechanical compliance

### Context Efficiency

With 200K tokens available:
- Prefer loading full files over incremental reads when understanding is needed
- Agent spawning for "context preservation" is less critical than with previous models
- Focus on expertise isolation (specialized prompts) rather than context savings

---

## Claude Sonnet 4.5

**Model ID**: `claude-sonnet-4-5-20250929`

### When to Use

- Tier 3 premium stages in spec-kit (audit, unlock)
- Balance of capability and cost for multi-agent consensus
- Code review and security analysis

### Reasoning Mode

Standard reasoning is typically sufficient. Reserve extended thinking for:
- Security vulnerabilities with complex attack vectors
- Multi-file refactors with dependency chains

---

## Gemini 2.5 Pro

**Model ID**: `gemini-2.5-pro`

### Thinking Mode

Use thinking mode for:
- Architecture decisions affecting >3 files
- Multi-agent consensus synthesis and conflict resolution
- Complex debugging with multiple hypotheses
- Security audit and compliance review
- Refactors touching shared interfaces

### When to Use

- Tier 3 premium stages in spec-kit (audit, unlock)
- Long-context document analysis
- Research and information synthesis

---

## Gemini 2.5 Flash

**Model ID**: `gemini-2.5-flash`

### Fast Mode

Optimized for speed and cost. Use for:
- Single-file changes and bug fixes
- Documentation updates
- Status queries and diagnostics
- Tier 2 multi-agent stages (plan, validate)

---

## GPT-5 / GPT-5-Codex

### Effort Levels

| Level | Use Case | Stages |
|-------|----------|--------|
| **Low** | Simple analysis, task breakdown | specify, tasks |
| **Medium** | Planning, validation synthesis | plan, validate |
| **High** | Critical decisions, security | audit, unlock |
| **Codex (HIGH)** | Code generation only | implement |

### When to Use

- GPT-5-Codex is the specialist for code generation
- Use appropriate effort level based on task complexity
- High effort reserved for ship/no-ship decisions

---

## Validation Tiers (All Models)

Match validation effort to change scope:

| Change Size | Validation | Notes |
|-------------|------------|-------|
| **<50 lines** | Trust model self-check | Validate after completion |
| **50-200 lines** | fmt + clippy | Run after completion |
| **>200 lines or cross-module** | Full harness | fmt, clippy, build, tests |

### Full Validation Commands

```bash
cd codex-rs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-features
cargo test -p codex-core
```

---

## Multi-Provider CLI Setup (SPEC-KIT-952)

The TUI supports three model providers:

| Provider | TUI Display Names | Auth Method | Status |
|----------|-------------------|-------------|--------|
| **ChatGPT** | gpt-5, gpt-5-codex | Native OAuth | Working |
| **Claude** | claude-opus-4-5, claude-sonnet-4-5, claude-haiku-4-5 | CLI routing | Working |
| **Gemini** | gemini-2.5-pro, gemini-2.5-flash, gemini-2.0-flash | CLI routing | Working |

**Note**: Display names are TUI shortcuts. Actual API model IDs are resolved at runtime.

### Claude CLI Setup

```bash
# Install from https://claude.ai/download
claude
# Follow prompts to complete login
```

### Using Claude Models

```bash
/model claude-sonnet-4-5
/model claude-opus-4-5
/model claude-haiku-4-5
```

### Known Limitations

- When selecting a Claude model without the CLI installed, you'll see installation instructions in chat history
- CLI responses may take 2-25s (variability in CLI performance)

---

_Last Updated: 2025-11-30_
