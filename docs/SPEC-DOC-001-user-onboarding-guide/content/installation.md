# Installation Guide

This guide covers all methods for installing the Code CLI tool on your system.

---

## Table of Contents

1. [System Requirements](#system-requirements)
2. [Quick Install (Recommended)](#quick-install-recommended)
3. [Installation Methods](#installation-methods)
   - [Method 1: NPM (Recommended)](#method-1-npm-recommended)
   - [Method 2: One-Time Execution (No Install)](#method-2-one-time-execution-no-install)
   - [Method 3: Homebrew (macOS/Linux)](#method-3-homebrew-macoslinux)
   - [Method 4: Build from Source](#method-4-build-from-source)
4. [Verification](#verification)
5. [Next Steps](#next-steps)
6. [Troubleshooting Installation](#troubleshooting-installation)

---

## System Requirements

Before installing Code, ensure your system meets these minimum requirements:

| Requirement | Details |
|------------|---------|
| **Operating System** | macOS 12+, Ubuntu 20.04+/Debian 10+, or Windows 11 via WSL2 |
| **Node.js** | Version 22+ (for npm installation) |
| **RAM** | 4 GB minimum (8 GB recommended) |
| **Disk Space** | 500 MB minimum |
| **Git** | 2.23+ (optional, recommended for built-in PR helpers) |

**Windows Users**: Direct Windows installation is not officially supported. Use [Windows Subsystem for Linux (WSL2)](https://learn.microsoft.com/en-us/windows/wsl/install) for the best experience.

---

## Quick Install (Recommended)

The fastest way to get started is using npm:

```bash
# Install globally
npm install -g @just-every/code

# Run Code
code
```

**Note**: If another tool provides a `code` command (e.g., VS Code), use `coder` instead to avoid conflicts.

---

## Installation Methods

### Method 1: NPM (Recommended)

**Best for**: Most users, especially those familiar with Node.js ecosystems.

**Prerequisites**:
- Node.js 22+ installed
- npm or pnpm package manager

**Installation Steps**:

```bash
# Install using npm
npm install -g @just-every/code

# Or using pnpm (faster)
pnpm add -g @just-every/code
```

**Verify Installation**:

```bash
# Check version
code --version

# If 'code' conflicts with VS Code, use:
coder --version
```

**Command Aliases**:
- `code` - Primary command
- `coder` - Alternative command (avoids conflicts with VS Code)

Both commands are functionally identical.

---

### Method 2: One-Time Execution (No Install)

**Best for**: Quick testing, CI/CD pipelines, temporary use.

**No installation required** - uses `npx` to download and run:

```bash
# Run directly without installing
npx -y @just-every/code

# With a prompt
npx -y @just-every/code "explain this codebase"
```

**Advantages**:
- ✅ No global installation
- ✅ Always uses latest version
- ✅ Perfect for CI/CD
- ✅ No conflicts with existing tools

**Disadvantages**:
- ❌ Slower startup (downloads each time)
- ❌ Requires internet connection

---

### Method 3: Homebrew (macOS/Linux)

**Best for**: macOS and Linux users who prefer Homebrew package management.

**Prerequisites**:
- [Homebrew](https://brew.sh/) installed

**Installation Steps**:

```bash
# Add the tap (if not already added)
brew tap just-every/code

# Install Code
brew install code-cli

# Run Code
code
```

**Update via Homebrew**:

```bash
brew upgrade code-cli
```

**Uninstall**:

```bash
brew uninstall code-cli
```

---

### Method 4: Build from Source

**Best for**: Contributors, advanced users, custom builds, or testing unreleased features.

**Prerequisites**:
- Git 2.23+
- Rust toolchain (will be installed automatically)
- Node.js 22+ (for TypeScript CLI wrapper)

**Installation Steps**:

#### Step 1: Clone Repository

```bash
# Clone from GitHub (upstream community fork)
git clone https://github.com/just-every/code.git
cd code
```

**For fork contributors** (theturtlecsz/code):
```bash
# Clone the fork
git clone https://github.com/theturtlecsz/code.git
cd code

# Add upstream remote
git remote add upstream https://github.com/just-every/code.git
```

#### Step 2: Install Rust Toolchain

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Load Rust environment
source "$HOME/.cargo/env"

# Install required components
rustup component add rustfmt clippy
```

#### Step 3: Setup Git Hooks (Contributors Only)

```bash
# Required for contributors to theturtlecsz/code fork
bash scripts/setup-hooks.sh
```

This installs pre-commit hooks that ensure:
- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Tests compile successfully
- Documentation structure is valid

#### Step 4: Build Code

**Fast Build** (recommended for development):

```bash
# Build with fast profile (optimized for iteration speed)
./build-fast.sh

# Binary location
./codex-rs/target/dev-fast/code
```

**Release Build** (optimized for performance):

```bash
# Navigate to Rust workspace
cd codex-rs

# Build release binaries
cargo build --release --bin code --bin code-tui --bin code-exec

# Binary location
./target/release/code
```

**Quick Build** (Code CLI only):

```bash
cd codex-rs
cargo build --release --bin code
```

#### Step 5: Run Locally

```bash
# From fast build
./codex-rs/target/dev-fast/code

# From release build
./codex-rs/target/release/code

# With a prompt
./codex-rs/target/release/code "explain this codebase to me"
```

#### Step 6: Install Globally (Optional)

```bash
# Install the binary to ~/.cargo/bin (in your PATH)
cd codex-rs
cargo install --path cli --bin code

# Now you can run 'code' from anywhere
code --version
```

#### Verify Build Quality

```bash
cd codex-rs

# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build all binaries
cargo build --workspace --all-features

# Run test suite
cargo test
```

---

## Verification

After installation, verify Code is working correctly:

### Basic Verification

```bash
# Check version
code --version
# Expected output: code 0.0.0 (or current version)

# Display help
code --help

# Generate shell completions (optional)
code completion bash   # for Bash
code completion zsh    # for Zsh
code completion fish   # for Fish
```

### Test Interactive Mode

```bash
# Start interactive TUI
code
```

You should see:
- Authentication prompt (if first run)
- Chat interface with composer
- Status bar showing model and configuration

**Exit**: Press `Ctrl+C` or type `/exit`

### Test Non-Interactive Mode

```bash
# Run a simple prompt (requires authentication)
code "What are the files in this directory?"
```

---

## Next Steps

After successful installation:

1. **First-Time Setup** → See [first-time-setup.md](first-time-setup.md)
   - Configure authentication (ChatGPT or API key)
   - Set up config.toml
   - Configure MCP servers (optional)

2. **Quick Start Tutorial** → See [quick-start.md](quick-start.md)
   - Run your first command
   - Understand the TUI interface
   - Try example workflows

3. **Learn Common Workflows** → See [workflows.md](workflows.md)
   - Spec-kit automation
   - Code refactoring
   - Testing and validation

---

## Troubleshooting Installation

### NPM Installation Issues

**Error**: `EACCES: permission denied`

**Solution 1**: Use `--prefix` to install to user directory:
```bash
npm install -g --prefix ~/.npm-global @just-every/code
export PATH=~/.npm-global/bin:$PATH
```

**Solution 2**: Fix npm permissions:
```bash
mkdir -p ~/.npm-global
npm config set prefix '~/.npm-global'
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc
source ~/.bashrc
npm install -g @just-every/code
```

---

**Error**: `npm ERR! code E404` or `npm ERR! 404 Not Found`

**Cause**: Package name incorrect or npm registry issue.

**Solution**:
```bash
# Verify package name
npm info @just-every/code

# Clear npm cache
npm cache clean --force

# Try again
npm install -g @just-every/code
```

---

### Build from Source Issues

**Error**: `rustc: command not found`

**Cause**: Rust toolchain not installed or not in PATH.

**Solution**:
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Reload shell or source manually
source "$HOME/.cargo/env"

# Verify
rustc --version
cargo --version
```

---

**Error**: `cargo build` fails with "linking with `cc` failed"

**Cause**: Missing system dependencies (especially on Linux).

**Solution (Ubuntu/Debian)**:
```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

**Solution (macOS)**:
```bash
# Install Xcode Command Line Tools
xcode-select --install
```

---

**Error**: `error: linker 'cc' not found` on Alpine Linux

**Cause**: Alpine uses musl libc, requires additional setup.

**Solution**:
```bash
# Install musl-dev
apk add musl-dev

# Or use rustup to add musl target
rustup target add x86_64-unknown-linux-musl
cargo build --target x86_64-unknown-linux-musl
```

---

**Error**: Pre-commit hook blocks commit with `cargo clippy` warnings

**Cause**: Code quality checks failed.

**Solution**:
```bash
# Fix formatting
cargo fmt --all

# Fix clippy warnings
cargo clippy --workspace --all-targets --all-features --fix --allow-dirty

# Or temporarily skip hooks (NOT recommended for regular commits)
PRECOMMIT_FAST_TEST=0 git commit -m "your message"
```

---

### Windows (WSL2) Issues

**Error**: `code: command not found` after installation in WSL2

**Cause**: PATH not updated or npm global bin not in PATH.

**Solution**:
```bash
# Find npm global bin path
npm config get prefix

# Add to PATH (add to ~/.bashrc or ~/.zshrc)
export PATH="$(npm config get prefix)/bin:$PATH"

# Reload shell
source ~/.bashrc
```

---

**Error**: Git operations fail with "permission denied" in WSL2

**Cause**: Windows file permissions issue.

**Solution**:
```bash
# Clone repos into WSL filesystem (not /mnt/c/)
cd ~
git clone https://github.com/just-every/code.git
```

---

### Command Conflicts

**Error**: `code` opens VS Code instead of Code CLI

**Cause**: VS Code's `code` command takes precedence in PATH.

**Solution 1**: Use `coder` alias
```bash
coder --version
coder "your prompt"
```

**Solution 2**: Create shell alias
```bash
# Add to ~/.bashrc or ~/.zshrc
alias codex-cli='code'

# Or point directly to binary
alias codex-cli='/path/to/code'
```

**Solution 3**: Adjust PATH order (advanced)
```bash
# Find Code CLI location
which -a code

# Add to beginning of PATH in ~/.bashrc
export PATH="/path/to/code/bin:$PATH"
```

---

### Verification Failures

**Error**: `code --version` shows old version after update

**Cause**: Multiple installations or cached binary.

**Solution**:
```bash
# Find all 'code' binaries
which -a code

# Clear npm cache
npm cache clean --force

# Reinstall
npm uninstall -g @just-every/code
npm install -g @just-every/code

# Verify
code --version
```

---

## Additional Resources

- **Official Documentation**: [docs/README.md](../../README.md)
- **Configuration Guide**: [config.md](config.md)
- **Troubleshooting**: [troubleshooting.md](troubleshooting.md)
- **GitHub Repository**: https://github.com/just-every/code
- **Fork Repository** (enhanced features): https://github.com/theturtlecsz/code

---

**Installation Complete!** → Continue to [First-Time Setup](first-time-setup.md)
