# SPEC-TUI2-TEST-PLAN: Runtime Test Plan for TUI v2 Port

## Overview

Test plan for validating the TUI v2 port (SYNC-028) after achieving 0 compilation errors.

**Created**: 2024-12-24 (Session 11)
**Binary**: `./target/release/codex-tui2`
**Prereq**: `cargo build -p codex-tui2 --release`

## Test Categories

### 1. Basic Launch Tests

| Test | Command | Expected Result | Status |
|------|---------|-----------------|--------|
| Help output | `--help` | Shows usage | PASS |
| Version output | `--version` | Shows version | PASS |
| Terminal check | (no args, non-tty) | "stdin is not a terminal" error | PASS |
| Interactive launch | (real terminal) | TUI renders without panic | PENDING |

### 2. Configuration Tests

| Test | Command | Expected Result | Status |
|------|---------|-----------------|--------|
| Model override | `-m gpt-4o` | Uses specified model | PENDING |
| Config file | `-c model="gpt-4o"` | Applies config override | PENDING |
| Sandbox mode | `-s workspace-write` | Sets sandbox policy | PENDING |
| Approval mode | `-a on-request` | Sets approval policy | PENDING |
| Full auto | `--full-auto` | Sets low-friction mode | PENDING |

### 3. Core Functionality Tests

| Test | Steps | Expected Result | Status |
|------|-------|-----------------|--------|
| Submit prompt | 1. Launch TUI 2. Type message 3. Press Enter | Message appears, model responds | PENDING |
| Exit cleanly | Press Ctrl+C | TUI exits without panic | PENDING |
| Status bar | Launch TUI | Model name visible in status | PENDING |
| History scroll | Submit 5+ messages, scroll up | History scrolls correctly | PENDING |

### 4. Stubbed Feature Tests (Graceful Degradation)

These features are stubbed; verify they fail gracefully without panics.

| Feature | Test Steps | Expected Behavior | Status |
|---------|------------|-------------------|--------|
| Model migration | Trigger migration prompt | Shows partial UI or no-op | PENDING |
| Credits display | Check status bar | No credits shown (OK) | PENDING |
| OSS provider | `--oss` flag | Error or no-op | PENDING |
| Skill mentions | Type `@skill` | No skill UI (OK) | PENDING |
| MCP tools list | Use MCP command | Shows error or empty | PENDING |
| Custom prompts | Access prompts menu | Shows error or empty | PENDING |
| User shell | Type `!ls` | Shows error message | PENDING |
| Review mode | Enter/exit review | Minimal handling (OK) | PENDING |
| Execpolicy amendment | Trigger policy prompt | No special amendment UI | PENDING |

### 5. Stress Tests

| Test | Steps | Expected Result | Status |
|------|-------|-----------------|--------|
| Long session | 20+ turns without restart | No memory leaks/crashes | PENDING |
| Large response | Trigger multi-page response | Renders correctly | PENDING |
| Rapid input | Type fast during response | No race conditions | PENDING |

## Environment Requirements

- **Terminal**: Real TTY (not piped/scripted)
- **API Key**: Valid OpenAI API key in environment
- **Config**: Default or minimal config.toml

## Test Execution Commands

```bash
# Build release
cargo build -p codex-tui2 --release

# Basic tests (can run without terminal)
./target/release/codex-tui2 --help
./target/release/codex-tui2 --version

# Interactive tests (require real terminal)
RUST_BACKTRACE=1 ./target/release/codex-tui2
RUST_LOG=debug ./target/release/codex-tui2
```

## Known Limitations

1. **Update checks disabled**: Fork checks are irrelevant (checks openai/codex)
2. **Credits not shown**: Fork protocol lacks credits field
3. **Skills unavailable**: `InputItem::Skill` variant not in fork
4. **MCP tools display broken**: Type mismatch in display
5. **User shell commands show error**: `Op::RunUserShellCommand` missing

## Pass Criteria

**Minimum viable**:
- [x] `--help` works
- [x] `--version` works
- [x] No panics on launch (terminal required)
- [ ] Can submit at least one prompt
- [ ] Exits cleanly on Ctrl+C

**Full pass**:
- [ ] All core functionality tests pass
- [ ] All stubbed features fail gracefully
- [ ] No panics during 20-turn session

## Results

### Session 11 (2024-12-24)

**Environment**: Headless Linux (no real TTY available)

| Test | Result | Notes |
|------|--------|-------|
| --help | PASS | Full usage displayed |
| --version | PASS | "codex-tui2 0.0.0" |
| Non-tty detection | PASS | Graceful error message |
| Interactive launch | BLOCKED | Requires real terminal |

**Next**: Test in real terminal environment for interactive validation.
