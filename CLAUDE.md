# CLAUDE.md

Project instructions for AI agents (Claude Code, code TUI, Gemini, etc.)

> **tui vs tui2**: tui is the primary UI with spec-kit support. tui2 is an upstream scaffold only.
> See [ADR-002](docs/adr/ADR-002-tui2-purpose-and-future.md).

## Commands

### Build

```bash
~/code/build-fast.sh              # Fast build (dev-fast profile)
~/code/build-fast.sh run          # Build and run TUI
PROFILE=release ~/code/build-fast.sh   # Release build
```

### Test

```bash
cd codex-rs
cargo test -p codex-core                           # All core tests
cargo test -p codex-core -- suite::fork_conversation  # Specific module
cargo test -p codex-core -- --ignored              # Include ignored
```

### Lint & Validate

```bash
cd codex-rs
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo build --workspace --all-features
```

### Spec-Kit Commands

```bash
/speckit.new <AREA> <description> # Create new SPEC (instant, free)
/speckit.project rust <name>      # Scaffold project with spec-kit
/speckit.auto SPEC-ID             # Full 6-stage pipeline (~$2.70)
/speckit.status SPEC-ID           # Check SPEC status
/spec-evidence-stats              # Monitor evidence footprint
```

## Code Style

* **Commits**: Conventional format - `feat(scope):`, `fix(scope):`, `docs(scope):`
* **Branches**: Feature branches only, never commit directly to main
* **Pre-commit**: Auto-runs fmt, clippy, doc validation via `.githooks/`
* **Setup hooks**: `bash scripts/setup-hooks.sh` (one-time)

## Project Structure

```
codex-rs/                    # Rust workspace (run cargo from here)
├── tui/src/                 # TUI implementation
│   └── chatwidget/spec_kit/ # Spec-kit commands
├── core/                    # Core library
templates/                   # Spec-kit templates (project-local)
docs/
├── SPEC-*/                  # Feature specs and PRDs
├── spec-kit/                # Spec-kit documentation
├── OPERATIONAL-PLAYBOOK.md  # Behavioral guidance
├── MODEL-GUIDANCE.md        # Model-specific instructions
SPEC.md                      # Task tracking (single source of truth)
```

## Required Files

* **DEV\_BRIEF.md**: Tier-1 truth anchor (must exist and be non-empty)
  * Update at session start with current focus/constraints
  * Enforced by pre-commit hook (hard block) and doc\_lint
* **docs/briefs/<branch>.md**: Per-PR session brief (must exist and be non-empty on feature branches)
  * `<branch>` = git branch name with `/` replaced by `__`
  * Enforced by pre-commit hook (hard block)

## Testing Notes

* **Test location**: Run from `codex-rs/` directory
* **Current status**: 31 passing, 12 ignored in codex-core
* **Ignored tests**: Have documented blockers (fork divergences)
* **Don't use**: `cargo build -p codex-tui` directly (use build script)

## Known Quirks

* HAL secrets: Set `SPEC_OPS_HAL_SKIP=1` if `HAL_SECRET_KAVEDARR_API_KEY` unavailable
* Evidence limit: 25 MB soft limit per SPEC, monitor with `/spec-evidence-stats`
* Dirty tree: Guardrails require clean tree unless `SPEC_OPS_ALLOW_DIRTY=1`
* Config isolation: Templates resolve `./templates/` → embedded only (no global)

## Local Memory Integration

**Policy**: CLI + REST only. No MCP.

### Golden Path vs Manual

| Mode                | When                   | Memory Handling                                        |
| ------------------- | ---------------------- | ------------------------------------------------------ |
| **`/speckit.auto`** | Primary workflow       | Stage0 orchestrates memory recall + Tier2 (NotebookLM) |
| **Ad-hoc work**     | Debugging, exploration | Use `lm` commands manually (below)                     |

### Manual Commands (Non-Golden-Path Only)

**Before proposing changes** (if NOT using `/speckit.auto`):

```bash
lm recall "<task keywords>" --limit 5
lm domain  # Verify domain resolution
```

**After significant work** (importance >= 8 only):

```bash
lm remember "<insight>" --type <TYPE> --importance 8 --tags "component:..."
```

**Canonical types**: `decision`, `pattern`, `bug-fix`, `milestone`, `discovery`, `limitation`, `architecture`

**Policy reference**: `~/.claude/skills/local-memory/SKILL.md`

## Git

* Default branch: **main**

## Extended Documentation

For detailed guidance beyond commands and structure:

| Topic                    | Document                                    |
| ------------------------ | ------------------------------------------- |
| Behavioral rules         | `docs/OPERATIONAL-PLAYBOOK.md`              |
| Model-specific reasoning | `docs/MODEL-GUIDANCE.md`                    |
| Multi-agent architecture | `docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md` |
| Memory policy            | `MEMORY-POLICY.md`                          |
| Project charter          | `memory/constitution.md`                    |

## Policy Pointers

* **Authoritative policy**: `docs/MODEL-POLICY.md` (v1.0.0)
* **Guardrails**: GR-001 through GR-013
* **No consensus by default** — single-owner pipeline (Architect → Implementer → Judge)
* **Routing thresholds**: Architect escalation at `confidence < 0.75`, Implementer escalation after 2 failed loops
* **Kimi/DeepSeek**: escalation-only (never default path)

***

Back to [Key Docs](docs/KEY_DOCS.md)
