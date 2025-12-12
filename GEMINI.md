# GEMINI.md

This repository is a fork with fork-specific markers and an upstream sync workflow (see `UPSTREAM-SYNC.md`).
Not related to Anthropic's Claude Code.

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
/speckit.new <description>        # Create new SPEC (instant, free)
/speckit.project rust <name>      # Scaffold project with spec-kit
/speckit.auto SPEC-ID             # Full 6-stage pipeline (~$2.70)
/speckit.status SPEC-ID           # Check SPEC status
/spec-evidence-stats              # Monitor evidence footprint
```

## Code Style

- **Commits**: Conventional format - `feat(scope):`, `fix(scope):`, `docs(scope):`
- **Branches**: Feature branches only, never commit directly to main
- **Pre-commit**: Auto-runs fmt, clippy, doc validation via `.githooks/`
- **Setup hooks**: `bash scripts/setup-hooks.sh` (one-time)

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

## Testing Notes

- **Test location**: Run from `codex-rs/` directory
- **Current status**: 31 passing, 12 ignored in codex-core
- **Ignored tests**: Have documented blockers (fork divergences)
- **Don't use**: `cargo build -p codex-tui` directly (use build script)

## Known Quirks

- HAL secrets: Set `SPEC_OPS_HAL_SKIP=1` if `HAL_SECRET_KAVEDARR_API_KEY` unavailable
- Evidence limit: 25 MB soft limit per SPEC, monitor with `/spec-evidence-stats`
- Dirty tree: Guardrails require clean tree unless `SPEC_OPS_ALLOW_DIRTY=1`
- Config isolation: Templates resolve `./templates/` → embedded only (no global)

## Git

- Default branch: **main**
- Upstream sync: `git fetch upstream && git merge --no-ff --no-commit upstream/main`
- See `docs/UPSTREAM-SYNC.md` for details

## Extended Documentation

For detailed guidance beyond commands and structure:

| Topic | Document |
|-------|----------|
| Behavioral rules | `docs/OPERATIONAL-PLAYBOOK.md` |
| Model-specific reasoning | `docs/MODEL-GUIDANCE.md` |
| Multi-agent architecture | `docs/spec-kit/MULTI-AGENT-ARCHITECTURE.md` |
| Memory policy | `MEMORY-POLICY.md` |
| Project charter | `memory/constitution.md` |
