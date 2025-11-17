# Troubleshooting Guide

Comprehensive error resolution guide for Code CLI.

---

## Table of Contents

1. [Installation Errors](#installation-errors)
2. [Authentication Issues](#authentication-issues)
3. [MCP Connection Problems](#mcp-connection-problems)
4. [Agent Execution Failures](#agent-execution-failures)
5. [Performance Issues](#performance-issues)
6. [Configuration Mistakes](#configuration-mistakes)
7. [File Operation Errors](#file-operation-errors)
8. [Network and Connectivity](#network-and-connectivity)
9. [Platform-Specific Issues](#platform-specific-issues)
10. [Getting Help](#getting-help)

---

## Installation Errors

### Error: `npm: command not found`

**Cause**: Node.js/npm not installed

**Solution**:

```bash
# Install Node.js (includes npm)
# Visit https://nodejs.org/ or use package manager:

# macOS (Homebrew)
brew install node

# Ubuntu/Debian
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt-get install -y nodejs

# Verify
node --version
npm --version
```

---

### Error: `EACCES: permission denied` (npm install)

**Cause**: Insufficient permissions to install global npm packages

**Solution 1**: Install to user directory

```bash
# Create npm global directory
mkdir -p ~/.npm-global

# Configure npm to use it
npm config set prefix '~/.npm-global'

# Add to PATH
echo 'export PATH=~/.npm-global/bin:$PATH' >> ~/.bashrc
source ~/.bashrc

# Install Code
npm install -g @just-every/code
```

**Solution 2**: Fix npm permissions

```bash
# Change npm directory ownership
sudo chown -R $(whoami) ~/.npm
sudo chown -R $(whoami) /usr/local/lib/node_modules

# Retry installation
npm install -g @just-every/code
```

**Solution 3**: Use `sudo` (not recommended)

```bash
sudo npm install -g @just-every/code
```

---

### Error: `cargo: command not found` (build from source)

**Cause**: Rust toolchain not installed

**Solution**:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# Reload environment
source "$HOME/.cargo/env"

# Install required components
rustup component add rustfmt clippy

# Verify
cargo --version
rustc --version
```

---

### Error: `linking with cc failed` (Rust build)

**Cause**: Missing C compiler or system libraries

**Solution (Ubuntu/Debian)**:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

**Solution (macOS)**:

```bash
xcode-select --install
```

**Solution (Alpine Linux)**:

```bash
apk add build-base openssl-dev
```

---

### Error: `code: command not found` after installation

**Cause**: npm global bin directory not in PATH

**Solution**:

```bash
# Find npm global bin path
npm config get prefix

# Expected output: /home/user/.npm-global (or similar)

# Add to PATH in ~/.bashrc or ~/.zshrc
echo 'export PATH="$(npm config get prefix)/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify
code --version
```

**Alternative**: Use full path

```bash
# Find code binary location
npm list -g @just-every/code | grep code

# Run with full path
/path/to/code --version
```

---

## Authentication Issues

### Error: `Failed to authenticate` (ChatGPT login)

**Cause**: Browser flow failed or credentials not saved

**Solution**:

```bash
# Delete existing auth and retry
rm ~/.code/auth.json
code login

# Follow browser prompts carefully
# Ensure you're logged into ChatGPT
# Authorize Code CLI when prompted
```

**For headless/remote servers**:

```bash
# Use SSH tunnel (from local machine)
ssh -L 1455:localhost:1455 user@remote-host

# In SSH session, run:
code

# Open localhost:1455 in LOCAL browser
```

---

### Error: `401 Unauthorized` (API key)

**Cause**: Invalid API key or insufficient permissions

**Solution**:

```bash
# Verify API key format (should start with sk-proj-)
echo $OPENAI_API_KEY

# Check key at https://platform.openai.com/api-keys
# Ensure key has access to Responses API
# Check account verification status

# Set correct key
export OPENAI_API_KEY="sk-proj-CORRECT_KEY_HERE"

# Or add to ~/.code/.env
mkdir -p ~/.code
echo 'OPENAI_API_KEY=sk-proj-YOUR_KEY' > ~/.code/.env
chmod 600 ~/.code/.env
```

---

### Error: `403 Forbidden` (ChatGPT)

**Cause**: Account not eligible or subscription expired

**Solution**:

```bash
# Verify ChatGPT subscription status at https://chat.openai.com/

# Check account type:
# - ChatGPT Plus: ✅ Supported
# - ChatGPT Pro: ✅ Supported
# - ChatGPT Team: ✅ Supported
# - Free tier: ❌ Not supported for CLI

# Switch to API key if needed
export OPENAI_API_KEY="sk-proj-YOUR_KEY"
```

---

### Error: `Rate limit exceeded`

**Cause**: Too many requests to API provider

**Solution**:

**Wait for reset**:
```bash
# Rate limits reset after time period
# Free tier: ~1 hour
# Paid tier: ~1 minute
```

**Upgrade plan**:
```bash
# Visit https://platform.openai.com/settings/organization/billing
# Upgrade to paid tier for higher limits
```

**Use retry logic** (automatic in Code):
```bash
# Code automatically retries with exponential backoff
# Wait for retry to complete
```

**Rate Limits by Provider**:

| Provider | Free Tier | Paid Tier |
|----------|-----------|-----------|
| **OpenAI** | 3 req/min, 200 req/day | 60-90 req/min |
| **Anthropic** | 5 req/min | 50 req/min |
| **Google Gemini** | 15 req/min | 60 req/min |

---

### Error: `OPENAI_API_KEY not set` (but it is set)

**Cause**: Environment variable not loaded or scoping issue

**Solution**:

```bash
# Check if variable is actually set
echo $OPENAI_API_KEY

# If empty, set it
export OPENAI_API_KEY="sk-proj-YOUR_KEY"

# Permanently set in shell profile
echo 'export OPENAI_API_KEY="sk-proj-YOUR_KEY"' >> ~/.bashrc
source ~/.bashrc

# Or use ~/.code/.env
mkdir -p ~/.code
echo 'OPENAI_API_KEY=sk-proj-YOUR_KEY' > ~/.code/.env
chmod 600 ~/.code/.env

# Verify Code sees it
code --print-config | grep OPENAI_API_KEY
```

---

## MCP Connection Problems

### Error: `MCP server 'filesystem' failed to start`

**Cause**: MCP server not installed or command incorrect

**Solution**:

```bash
# Check if server is installed globally
npm list -g @modelcontextprotocol/server-filesystem

# If not installed
npm install -g @modelcontextprotocol/server-filesystem

# Verify command works
npx @modelcontextprotocol/server-filesystem /tmp

# Check config.toml syntax
cat ~/.code/config.toml
# Look for [mcp_servers.filesystem] section
```

**Correct configuration**:

```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/project"]
startup_timeout_sec = 10
```

---

### Error: `MCP server timeout`

**Cause**: Server takes too long to start

**Solution**: Increase timeout

```toml
# ~/.code/config.toml

[mcp_servers.slow-server]
command = "npx"
args = ["-y", "slow-mcp-server"]
startup_timeout_sec = 30  # Increase from default 10
tool_timeout_sec = 120    # Increase tool timeout too
```

---

### Error: `Tool 'filesystem' not found`

**Cause**: MCP server not configured or failed to start silently

**Solution**:

```bash
# List configured MCP servers
code mcp list

# Test server health
code mcp test filesystem

# Check logs for startup errors
code --debug
# Then try invoking the tool
# Check debug output for MCP server errors
```

---

### Error: `MCP server crashed` (mid-session)

**Cause**: Server bug or resource exhaustion

**Solution**:

```bash
# Check server output/logs
code --debug

# Restart Code (MCP servers restart automatically)
code

# If persistent, try running server manually to see errors
npx @modelcontextprotocol/server-filesystem /tmp

# Report issue to MCP server maintainers
```

---

## Agent Execution Failures

### Error: `Command 'claude' not found` (multi-agent)

**Cause**: CLI tools not installed

**Solution**:

```bash
# Install Claude CLI
npm install -g @anthropic-ai/claude-code

# Install Gemini CLI
npm install -g @google/gemini-cli

# Verify installations
which claude
which gemini

# Test commands
claude "test"
gemini -i "test"
```

---

### Error: `ANTHROPIC_API_KEY not set`

**Cause**: API key not configured for multi-agent setup

**Solution**:

```bash
# Set API keys for all providers
export ANTHROPIC_API_KEY="sk-ant-api03-YOUR_KEY"
export GOOGLE_API_KEY="AIza_YOUR_KEY"

# Add to shell profile
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
echo 'export GOOGLE_API_KEY="AIza..."' >> ~/.bashrc
source ~/.bashrc

# Verify
echo $ANTHROPIC_API_KEY
echo $GOOGLE_API_KEY
```

**Get API keys**:
- Anthropic: https://console.anthropic.com/settings/keys
- Google: https://ai.google.dev/

---

### Error: `/speckit.auto` fails with "agent missing"

**Cause**: Quality gates configured with unavailable agents

**Solution**:

**Check configuration**:

```toml
# ~/.code/config.toml

[quality_gates]
# Ensure agents are installed and configured
plan = ["gemini", "claude", "code"]  # All must be available

# If you only have OpenAI configured:
plan = ["code"]  # Single agent
tasks = ["code"]
validate = ["code"]
audit = ["code"]
unlock = ["code"]
```

**Or install missing providers**:

```bash
npm install -g @anthropic-ai/claude-code @google/gemini-cli
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="AIza..."
```

---

### Error: `Consensus failed: 0/3 agents responded`

**Cause**: All agents failed (network, rate limits, or bugs)

**Solution**:

```bash
# Check network connectivity
ping api.openai.com
ping api.anthropic.com

# Check rate limits (may need to wait)
# Try with single agent temporarily:

# In Code:
/speckit.plan SPEC-ID --agents code

# Or update config to use single agent
[quality_gates]
plan = ["code"]  # Temporarily use only OpenAI
```

**Enable debug mode** to see detailed errors:

```bash
code --debug
```

---

## Performance Issues

### Issue: Code CLI is slow to start

**Cause**: Large history file or MCP servers slow to initialize

**Solution**:

**Reduce history size**:

```bash
# Check history size
ls -lh ~/.code/history.jsonl

# If large (>100MB), truncate
mv ~/.code/history.jsonl ~/.code/history.jsonl.backup
touch ~/.code/history.jsonl

# Or disable history
# In ~/.code/config.toml:
[history]
persistence = "none"
```

**Disable slow MCP servers** temporarily:

```toml
# Comment out slow servers in config.toml
# [mcp_servers.slow-server]
# command = "..."
```

---

### Issue: Model responses are very slow

**Cause**: Complex reasoning, large context, or network issues

**Solution**:

**Reduce reasoning effort**:

```toml
# ~/.code/config.toml
model_reasoning_effort = "low"  # or "minimal"
```

**Use faster model**:

```bash
code --model gpt-4o-mini "simple task"
```

**Check network**:

```bash
# Test API endpoint connectivity
curl -I https://api.openai.com

# Check if using proxy
echo $HTTP_PROXY
echo $HTTPS_PROXY
```

---

### Issue: High memory usage

**Cause**: Large conversation history or MCP server memory leaks

**Solution**:

**Start new conversation**:

```bash
# In Code:
/new
```

**Restart Code** to free memory:

```bash
# Exit and restart
code
```

**Monitor memory**:

```bash
# Check Code process memory
ps aux | grep code
```

---

## Configuration Mistakes

### Error: `config.toml: unknown field 'xyz'`

**Cause**: Typo or invalid configuration option

**Solution**:

**Common typos**:

```toml
# ❌ Wrong (JSON style)
mcpServers.filesystem.command = "npx"

# ✅ Correct (TOML style, snake_case)
[mcp_servers.filesystem]
command = "npx"

# ❌ Wrong
modelProvider = "openai"

# ✅ Correct
model_provider = "openai"
```

**Validate config**:

```bash
# Check config syntax
code --config-check

# Print loaded config to verify
code --print-config
```

**Refer to docs**:

- See [config.md](../../config.md) for all valid options
- See [examples/config.toml](../../examples/config.toml) for templates

---

### Error: Config changes not taking effect

**Cause**: Wrong config file location or profile override

**Solution**:

**Verify config file location**:

```bash
# Check which config is loaded
code --print-config | head -20

# Verify file exists
ls -la ~/.code/config.toml

# Not ~/.codex/ (legacy location, read but not written to)
```

**Check for profile override**:

```toml
# If you have a profile set:
profile = "premium"

# It overrides root config
[profiles.premium]
model = "o3"  # This takes precedence
```

**Test without profile**:

```bash
code --profile=none
```

---

### Error: `sandbox_mode 'xyz' invalid`

**Cause**: Invalid sandbox mode value

**Solution**:

**Valid values only**:

```toml
# Valid options:
sandbox_mode = "read-only"         # ✅ No writes
sandbox_mode = "workspace_write"   # ✅ Write to workspace
sandbox_mode = "danger-full-access" # ✅ Full access (risky)

# Invalid:
sandbox_mode = "read_only"   # ❌ Wrong (use dash not underscore)
sandbox_mode = "full-access" # ❌ Wrong (missing "danger-")
```

---

## File Operation Errors

### Error: `Permission denied` when writing files

**Cause**: Sandbox mode prevents writes

**Solution**:

**Check current sandbox mode**:

```bash
# In Code:
/status

# Shows current sandbox_mode
```

**Adjust sandbox mode**:

```bash
# Allow workspace writes
code --sandbox workspace-write

# Or update config.toml:
sandbox_mode = "workspace_write"
```

**For specific directories**, add to writable_roots:

```toml
sandbox_mode = "workspace_write"

[sandbox_workspace_write]
writable_roots = ["/path/to/additional/dir"]
```

---

### Error: `File not found` when Code tries to read

**Cause**: File doesn't exist or path incorrect

**Solution**:

**Use absolute paths**:

```bash
# Instead of relative paths:
code "read ~/project/main.py"

# Use absolute path
code "read /home/user/project/main.py"
```

**Verify file exists**:

```bash
ls -la /path/to/file
```

**Use `--cd` flag** to set working directory:

```bash
code --cd /home/user/project "read main.py"
```

---

### Error: `Git operation failed`

**Cause**: `.git` directory not writable in workspace-write mode

**Solution**:

**Enable git writes**:

```toml
# ~/.code/config.toml

sandbox_mode = "workspace_write"

[sandbox_workspace_write]
allow_git_writes = true  # Default: true
```

**Or use danger-full-access** (in isolated environment):

```toml
sandbox_mode = "danger-full-access"
```

---

## Network and Connectivity

### Error: `Connection timeout` or `Network error`

**Cause**: Network issues, proxy, or firewall

**Solution**:

**Check connectivity**:

```bash
# Test API endpoints
curl -I https://api.openai.com
curl -I https://api.anthropic.com
```

**Configure proxy** (if behind corporate proxy):

```bash
# Set proxy environment variables
export HTTP_PROXY="http://proxy.company.com:8080"
export HTTPS_PROXY="http://proxy.company.com:8080"

# Run Code
code
```

**Check firewall**:

```bash
# Ensure outbound HTTPS (port 443) is allowed
# Contact IT if corporate firewall blocks OpenAI/Anthropic domains
```

---

### Error: `SSL certificate verification failed`

**Cause**: Corporate SSL inspection or outdated certificates

**Solution** (NOT recommended for production):

```bash
# Disable SSL verification (use only in development)
export NODE_TLS_REJECT_UNAUTHORIZED=0

# Better: Install corporate CA certificate
# Contact IT for proper certificate installation
```

---

### Error: `502 Bad Gateway` or `503 Service Unavailable`

**Cause**: OpenAI/Anthropic/Google API outage

**Solution**:

**Check status pages**:
- OpenAI: https://status.openai.com/
- Anthropic: https://status.anthropic.com/
- Google: https://status.cloud.google.com/

**Wait and retry**: Services usually recover within minutes

**Switch providers** temporarily:

```bash
# Use Anthropic if OpenAI is down
code --model claude-sonnet-3-5 "task"

# Or configure fallback in quality gates
[quality_gates]
plan = ["claude", "gemini"]  # Exclude OpenAI temporarily
```

---

## Platform-Specific Issues

### Windows (WSL2)

**Issue**: `code: command not found` after installation

**Solution**:

```bash
# Ensure npm global bin is in PATH
export PATH="$(npm config get prefix)/bin:$PATH"

# Add to ~/.bashrc for persistence
echo 'export PATH="$(npm config get prefix)/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

---

**Issue**: Git operations fail with "permission denied"

**Solution**:

```bash
# Clone repos into WSL filesystem (not /mnt/c/)
cd ~
git clone https://github.com/just-every/code.git

# Avoid working in /mnt/c/Users/... (Windows filesystem)
# Use native WSL paths like /home/user/
```

---

### macOS

**Issue**: `xcode-select: command not found`

**Solution**:

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Follow prompts to complete installation

# Verify
xcode-select -p
# Should output: /Library/Developer/CommandLineTools
```

---

**Issue**: Homebrew installation fails

**Solution**:

```bash
# Install Homebrew first
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Add Homebrew to PATH (M1/M2 Macs)
echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
eval "$(/opt/homebrew/bin/brew shellenv)"

# Retry Code installation
brew tap just-every/code
brew install code-cli
```

---

### Linux (Specific Distributions)

**Alpine Linux**: Build issues with musl libc

**Solution**:

```bash
# Install build dependencies
apk add build-base openssl-dev pkgconfig

# Use musl target for Rust
rustup target add x86_64-unknown-linux-musl
cd codex-rs
cargo build --target x86_64-unknown-linux-musl --release
```

---

**Ubuntu/Debian**: Missing libraries

**Solution**:

```bash
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev
```

---

## Getting Help

### Before Asking for Help

1. **Check error message carefully**: Error messages often contain the solution
2. **Review this troubleshooting guide**: Search for your specific error
3. **Check GitHub Issues**: https://github.com/just-every/code/issues
4. **Enable debug mode**: `code --debug` for detailed logs

---

### Gathering Debug Information

When reporting issues, provide:

```bash
# 1. Version information
code --version

# 2. System information
uname -a                    # OS version
node --version              # Node.js version
npm --version               # npm version
rustc --version             # Rust version (if building from source)

# 3. Configuration (sanitize secrets!)
code --print-config

# 4. Debug logs
code --debug 2>&1 | tee debug.log
# Reproduce issue
# Share debug.log (after removing any API keys!)

# 5. Environment variables
env | grep -E 'OPENAI|ANTHROPIC|GOOGLE|CODE|CODEX'
```

---

### Where to Get Help

**Official Documentation**:
- Installation: [installation.md](installation.md)
- Setup: [first-time-setup.md](first-time-setup.md)
- Configuration: [../../config.md](../../config.md)
- FAQ: [faq.md](faq.md)

**Community Support**:
- **GitHub Issues**: https://github.com/just-every/code/issues
  - Search existing issues first
  - Provide debug information
  - Include steps to reproduce

- **GitHub Discussions**: https://github.com/just-every/code/discussions
  - Ask questions
  - Share workflows
  - Request features

**Fork-Specific** (theturtlecsz/code):
- Issues: https://github.com/theturtlecsz/code/issues
- Spec-Kit documentation: [../../spec-kit/README.md](../../spec-kit/README.md)

---

### Reporting Bugs

**Good bug report includes**:

1. **Clear title**: "MCP server fails to start on Ubuntu 22.04"
2. **Expected behavior**: What should happen
3. **Actual behavior**: What actually happens
4. **Steps to reproduce**:
   ```
   1. Install Code via npm
   2. Configure MCP server in config.toml
   3. Run `code`
   4. Server fails to start
   ```
5. **Environment**: OS, Code version, Node version
6. **Logs**: Debug logs, error messages
7. **Config** (sanitized): Relevant config.toml sections

---

## Common Error Reference

Quick lookup table for frequent errors:

| Error | Common Cause | Quick Fix |
|-------|--------------|-----------|
| `npm: command not found` | Node.js not installed | Install Node.js from nodejs.org |
| `EACCES: permission denied` | npm permissions | Use `npm install -g --prefix ~/.npm-global` |
| `code: command not found` | Not in PATH | Add npm global bin to PATH |
| `401 Unauthorized` | Invalid API key | Check API key at platform.openai.com |
| `403 Forbidden` | No ChatGPT subscription | Upgrade plan or use API key |
| `Rate limit exceeded` | Too many requests | Wait or upgrade plan |
| `MCP server failed to start` | Server not installed | `npm install -g @modelcontextprotocol/server-*` |
| `Permission denied` (files) | Wrong sandbox mode | Use `--sandbox workspace-write` |
| `config.toml: unknown field` | Typo in config | Check snake_case: `model_provider` not `modelProvider` |
| `Connection timeout` | Network/proxy issue | Check connectivity, configure proxy |
| `Command 'claude' not found` | CLI not installed | `npm install -g @anthropic-ai/claude-code` |

---

**Still stuck?** → Open an issue on [GitHub](https://github.com/just-every/code/issues)
