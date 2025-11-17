# Build System

Comprehensive guide to the Cargo build system and profiles.

---

## Cargo Profiles

### dev-fast (Default Development)

**Purpose**: Fast incremental builds for local development

**Build Time**: ~30-60s (incremental: ~5-10s)

**Command**:
```bash
./build-fast.sh
# Or: cargo build --profile dev-fast
```

**Output**: `codex-rs/target/dev-fast/code`

**Optimizations**:
- opt-level = 1 (basic optimizations)
- debug = false (no debug symbols)
- incremental = true

---

### dev (Standard Debug)

**Purpose**: Full debug info for debugging

**Build Time**: ~2-5 minutes

**Command**:
```bash
cargo build
```

**Output**: `codex-rs/target/debug/code`

**Optimizations**:
- opt-level = 0
- debug = true
- incremental = true

---

### release (Production)

**Purpose**: Optimized for production

**Build Time**: ~5-10 minutes

**Command**:
```bash
cargo build --release
```

**Output**: `codex-rs/target/release/code`

**Optimizations**:
- opt-level = 3
- lto = true (link-time optimization)
- codegen-units = 1

---

### perf (Performance Testing)

**Purpose**: Profiling and benchmarking

**Command**:
```bash
./build-fast.sh perf
```

**Optimizations**:
- opt-level = 3
- debug = true (for profiling symbols)

---

## Build Flags

### TRACE_BUILD

**Purpose**: Print build metadata

```bash
TRACE_BUILD=1 ./build-fast.sh
```

**Output**: Toolchain version, artifact SHA

---

### DETERMINISTIC

**Purpose**: Reproducible builds

```bash
DETERMINISTIC=1 ./build-fast.sh
```

**Behavior**: Removes timestamps, UUIDs

---

## Cross-Compilation

### Linux → macOS

```bash
rustup target add x86_64-apple-darwin
cargo build --target x86_64-apple-darwin --release
```

### Linux → Windows

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --target x86_64-pc-windows-gnu --release
```

---

## Workspace Structure

**Root**: `codex-rs/Cargo.toml`

**Packages**:
- codex-tui (main TUI)
- codex-core (conversation logic)
- codex-cli (CLI entry point)
- mcp-client (MCP integration)
- 20+ other crates

**Build All**:
```bash
cd codex-rs
cargo build --workspace
```

---

## Summary

**Profiles**:
- dev-fast: Fast dev builds (~30-60s)
- dev: Full debug (~2-5min)
- release: Production (~5-10min)
- perf: Profiling (~5-10min)

**Next**: [Git Workflow](git-workflow.md)
