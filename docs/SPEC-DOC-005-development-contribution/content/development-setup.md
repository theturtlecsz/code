# Development Environment Setup

Complete guide to setting up your development environment.

---

## Prerequisites

### System Requirements

**Minimum**:
- CPU: 2 cores
- RAM: 4 GB
- Disk: 2 GB free space
- OS: Linux, macOS, or Windows (WSL2)

**Recommended**:
- CPU: 4+ cores
- RAM: 8+ GB
- Disk: 5 GB free space
- OS: Linux or macOS (for best performance)

---

## Required Tools

### 1. Rust Toolchain

**Version**: 1.90.0 (Rust Edition 2024)

**Install via rustup**:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Set version**:
```bash
rustup toolchain install 1.90.0
rustup default 1.90.0
```

**Verify**:
```bash
rustc --version
# Should output: rustc 1.90.0 (...)
```

---

### 2. Node.js & npm

**Version**: Node.js 20+ (for npm packaging and CLI tooling)

**Install**:
```bash
# Using nvm (recommended)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 20
nvm use 20

# Or via package manager
# macOS: brew install node@20
# Ubuntu: sudo apt install nodejs npm
```

**Verify**:
```bash
node --version  # v20.x.x
npm --version   # 10.x.x
```

---

### 3. Git

**Version**: 2.30+

**Install**:
```bash
# macOS
brew install git

# Ubuntu
sudo apt install git

# Verify
git --version  # git version 2.x.x
```

---

## Optional Tools

### 4. Development Tools

**cargo-watch** (auto-rebuild on changes):
```bash
cargo install cargo-watch
```

**cargo-tarpaulin** (coverage):
```bash
cargo install cargo-tarpaulin
```

**cargo-flamegraph** (profiling):
```bash
cargo install flamegraph
```

**hyperfine** (CLI benchmarking):
```bash
cargo install hyperfine
```

---

## Clone Repository

```bash
# Clone
git clone https://github.com/theturtlecsz/code.git
cd code

# Set up git hooks
bash scripts/setup-hooks.sh

# Verify hooks
git config core.hooksPath
# Should output: .githooks
```

---

## Build Project

### Quick Build (Fast Profile)

```bash
./build-fast.sh
```

**Output**: `codex-rs/target/dev-fast/code`

**Profile**: Optimized for fast builds (~30-60s)

---

### Full Build (Release Profile)

```bash
cd codex-rs
cargo build --release
```

**Output**: `codex-rs/target/release/code`

**Profile**: Optimized for performance (~5-10min first build)

---

### Verify Build

```bash
./codex-rs/target/dev-fast/code --version
# Output: code x.x.x

./codex-rs/target/dev-fast/code --help
# Shows help
```

---

## Run Tests

### All Tests

```bash
cd codex-rs
cargo test --workspace --all-features
```

**Time**: ~10-15 minutes (604 tests)

---

### Fast Tests (Curated)

```bash
bash scripts/ci-tests.sh
```

**Time**: ~3-5 minutes

---

### Specific Module

```bash
cd codex-rs
cargo test -p codex-tui
```

---

## MCP Server Setup (Optional)

### Local-Memory MCP

**Purpose**: Spec-kit consensus storage

**Install**:
```bash
npm install -g @modelcontextprotocol/server-memory
```

**Configure** (`~/.code/config.toml`):
```toml
[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]
startup_timeout_sec = 10
```

**Verify**:
```bash
npx -y @modelcontextprotocol/server-memory --version
```

---

### Filesystem MCP

**Purpose**: File operations via MCP

**Configure**:
```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
```

---

## IDE Setup

### VS Code

**Extensions**:
- rust-analyzer (rust-lang.rust-analyzer)
- CodeLLDB (vadimcn.vscode-lldb) - Debugging
- Even Better TOML (tamasfe.even-better-toml)
- Error Lens (usernamehw.errorlens)

**Settings** (.vscode/settings.json):
```json
{
  "rust-analyzer.cargo.features": "all",
  "rust-analyzer.check.command": "clippy",
  "rust-analyzer.check.extraArgs": ["--all-targets", "--all-features"],
  "editor.formatOnSave": true,
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  }
}
```

---

### IntelliJ IDEA / CLion

**Plugins**:
- Rust (JetBrains)
- TOML (JetBrains)

**Settings**:
- Enable "Run clippy on save"
- Enable "Format on save"

---

## Environment Variables

### Required for Development

```bash
# .env or ~/.bashrc

# OpenAI API key (for testing)
export OPENAI_API_KEY=sk-...

# Optional: Logging
export RUST_LOG=info

# Optional: Faster linking (macOS)
export CARGO_PROFILE_DEV_BUILD_OVERRIDE_DEBUG=true
```

---

### Optional for Testing

```bash
# HAL testing (optional)
export HAL_SECRET_KAVEDARR_API_KEY=...

# Skip HAL tests
export SPEC_OPS_HAL_SKIP=1

# Enable telemetry capture
export SPEC_OPS_TELEMETRY_HAL=1

# Fast test mode (skip some pre-commit checks)
export PRECOMMIT_FAST_TEST=0
```

---

## Verify Setup

### Checklist

```bash
# ✅ Rust toolchain
rustc --version | grep "1.90"

# ✅ Cargo works
cargo --version

# ✅ Node.js/npm
node --version | grep "v20"

# ✅ Git configured
git config user.name
git config user.email

# ✅ Hooks installed
git config core.hooksPath | grep ".githooks"

# ✅ Build succeeds
./build-fast.sh && ./codex-rs/target/dev-fast/code --version

# ✅ Tests pass
cd codex-rs && cargo test -p codex-login --test all

# ✅ Clippy passes
cargo clippy --workspace --all-targets --all-features -- -D warnings

# ✅ Format check
cargo fmt --all -- --check
```

**All checks should pass** ✅

---

## Troubleshooting

### Build Errors

**Error**: `rustc version 1.x.x is too old`
```bash
# Solution: Update Rust
rustup update
rustup default 1.90.0
```

**Error**: `linker 'cc' not found`
```bash
# Solution: Install build tools
# macOS: xcode-select --install
# Ubuntu: sudo apt install build-essential
```

---

### Test Failures

**Error**: Tests fail with "Connection refused"
```bash
# Solution: MCP server not running (expected if not configured)
# Tests should pass with SPEC_OPS_HAL_SKIP=1
export SPEC_OPS_HAL_SKIP=1
cargo test
```

---

### Slow Builds

**Solution 1**: Use dev-fast profile
```bash
./build-fast.sh  # ~30-60s
```

**Solution 2**: Enable incremental compilation
```bash
export CARGO_INCREMENTAL=1
```

**Solution 3**: Use sccache (build cache)
```bash
cargo install sccache
export RUSTC_WRAPPER=sccache
```

---

## Summary

**Setup Time**: ~30 minutes

**Steps**:
1. ✅ Install Rust 1.90.0
2. ✅ Install Node.js 20+
3. ✅ Clone repository
4. ✅ Set up git hooks (`bash scripts/setup-hooks.sh`)
5. ✅ Build project (`./build-fast.sh`)
6. ✅ Run tests (`bash scripts/ci-tests.sh`)
7. ✅ Configure IDE (VS Code recommended)

**Next Steps**:
- [Build System](build-system.md) - Cargo profiles, cross-compilation
- [Git Workflow](git-workflow.md) - Branching, commits, PRs
- [Code Style](code-style.md) - rustfmt, clippy, lints

---

**References**:
- Rust installation: https://rustup.rs/
- Project README: `/README.md`
- Setup hooks: `scripts/setup-hooks.sh`
