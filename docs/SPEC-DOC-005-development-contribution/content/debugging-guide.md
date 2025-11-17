# Debugging Guide

Comprehensive debugging techniques for development.

---

## Logging

### Enable Rust Logging

```bash
export RUST_LOG=debug
./codex-rs/target/dev-fast/code
```

**Levels**:
- `error`: Errors only
- `warn`: Warnings + errors
- `info`: Info + warn + errors
- `debug`: Debug + info + warn + errors
- `trace`: All messages

**Module-specific**:
```bash
export RUST_LOG=codex_tui::chatwidget::spec_kit=debug
```

---

### API Request Logging

```bash
./codex-rs/target/dev-fast/code --debug
```

**Output**: `~/.code/debug.log` (API requests/responses)

---

## Tmux Sessions

### View Active Sessions

```bash
tmux ls
```

**Example Output**:
```
speckit-SPEC-TEST-001-plan: 1 windows (created Fri Nov 17)
```

---

### Attach to Session

```bash
tmux attach -t speckit-SPEC-TEST-001-plan
```

**Detach**: `Ctrl-b d`

---

### Kill Session

```bash
tmux kill-session -t speckit-SPEC-TEST-001-plan
```

---

## MCP Debugging

### MCP Inspector

**Install**:
```bash
npm install -g @modelcontextprotocol/inspector
```

**Use**:
```bash
npx @modelcontextprotocol/inspector npx -y @modelcontextprotocol/server-memory
```

**Features**:
- Test tool calls
- Inspect responses
- Debug connection issues

---

### MCP Logs

**Enable verbose logging**:
```toml
# ~/.code/config.toml
[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory", "--verbose"]
```

---

## Agent Debugging

### Agent Spawn Failures

**Symptoms**: Agent doesn't start, timeout errors

**Debug**:
```bash
# Check agent availability
claude --version
gemini --version

# Check config
cat ~/.code/config.toml | grep -A 5 "\[agents\]"

# Manual test
claude "test message"
```

---

### Consensus Issues

**Symptoms**: Empty consensus, degraded mode

**Debug**:
```bash
# Check consensus artifacts
ls -la docs/SPEC-OPS-004*/evidence/consensus/SPEC-TEST/

# Inspect consensus file
cat docs/.../consensus/SPEC-TEST/spec-plan_*.json | jq
```

---

## Debugger (LLDB/GDB)

### VS Code

**.vscode/launch.json**:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug code",
      "cargo": {
        "args": ["build", "--bin=code", "--package=codex-cli"]
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

**Set breakpoint**: Click left margin in source file

**Run**: F5

---

### CLI (LLDB)

```bash
# Build with debug symbols
cargo build --bin code

# Run in debugger
lldb ./target/debug/code

# Set breakpoint
(lldb) breakpoint set --name main
(lldb) run
```

---

## Performance Debugging

### Profiling

```bash
cargo install flamegraph
cargo flamegraph --bin code
open flamegraph.svg
```

---

### Memory Leaks

```bash
# macOS
leaks --atExit -- ./target/debug/code

# Linux (valgrind)
valgrind --leak-check=full ./target/debug/code
```

---

## Common Issues

### Build Fails

**Check**:
```bash
cargo clean
cargo build
```

### Tests Fail

**Isolate**:
```bash
cargo test --package codex-tui specific_test -- --nocapture
```

### Slow Performance

**Profile**:
```bash
cargo flamegraph --bin code
```

---

## Summary

**Tools**:
- RUST_LOG (logging)
- --debug (API logs)
- tmux (session debugging)
- MCP inspector (MCP debugging)
- lldb/gdb (breakpoints)
- flamegraph (profiling)

**Next**: [Release Process](release-process.md)
