# First-Time Setup Guide

Complete setup guide for configuring Code CLI after installation.

---

## Table of Contents

1. [Overview](#overview)
2. [Step 1: Authentication](#step-1-authentication)
   - [Option A: Sign in with ChatGPT](#option-a-sign-in-with-chatgpt)
   - [Option B: API Key](#option-b-api-key)
3. [Step 2: Basic Configuration](#step-2-basic-configuration)
4. [Step 3: MCP Server Setup (Optional)](#step-3-mcp-server-setup-optional)
5. [Step 4: Multi-Provider Setup (Optional)](#step-4-multi-provider-setup-optional)
6. [Step 5: Verify Setup](#step-5-verify-setup)
7. [Configuration File Reference](#configuration-file-reference)
8. [Troubleshooting Setup](#troubleshooting-setup)

---

## Overview

**Time Required**: 5-15 minutes

**What You'll Configure**:
- ✅ Authentication (ChatGPT or API key)
- ✅ Basic config.toml settings
- ✅ MCP servers (optional)
- ✅ Multi-provider agents (optional)

**Prerequisites**:
- Code CLI installed ([installation guide](installation.md))
- OpenAI account (ChatGPT Plus/Pro/Team OR API key)

---

## Step 1: Authentication

Code supports two authentication methods. Choose one:

### Option A: Sign in with ChatGPT

**Best for**: ChatGPT Plus, Pro, or Team subscribers

**Advantages**:
- ✅ No per-token billing
- ✅ Access to models included in your plan
- ✅ Easy setup
- ✅ Credentials stored locally (not proxied)

**Setup Steps**:

1. **Launch Code**:
   ```bash
   code
   ```

2. **Select authentication method**:
   - You'll see a prompt: `Sign in with ChatGPT` or `Use API key`
   - Select `Sign in with ChatGPT`

3. **Complete browser flow**:
   - Code will start a local server on `localhost:1455`
   - Your browser will open automatically
   - Follow the ChatGPT sign-in flow
   - Authorize Code CLI

4. **Return to terminal**:
   - Once authorized, credentials are saved to `~/.code/auth.json`
   - Code will start automatically

**Headless/Remote Setup** (SSH, Docker, VPS):

If you're on a remote machine without a browser:

```bash
# From your LOCAL machine, create SSH tunnel:
ssh -L 1455:localhost:1455 user@remote-host

# Then, in that SSH session, run:
code

# Select "Sign in with ChatGPT"
# Open the printed URL in your LOCAL browser
# The tunnel will forward traffic to the remote server
```

**Or** authenticate locally and copy credentials:

```bash
# On LOCAL machine:
code login
# Complete authentication
# This creates ~/.code/auth.json

# Copy to REMOTE via scp:
ssh user@remote 'mkdir -p ~/.code'
scp ~/.code/auth.json user@remote:~/.code/auth.json

# Or one-liner:
ssh user@remote 'mkdir -p ~/.code && cat > ~/.code/auth.json' < ~/.code/auth.json
```

---

### Option B: API Key

**Best for**: Usage-based billing, automation, CI/CD

**Advantages**:
- ✅ Pay-as-you-go pricing
- ✅ No subscription required
- ✅ Works in CI/CD environments
- ✅ Programmatic access

**Setup Steps**:

1. **Get API key**:
   - Go to https://platform.openai.com/api-keys
   - Create new secret key
   - Copy the key (starts with `sk-proj-...`)

2. **Set environment variable**:

   **Temporary** (current session only):
   ```bash
   export OPENAI_API_KEY="sk-proj-YOUR_KEY_HERE"
   code
   ```

   **Permanent** (add to shell profile):
   ```bash
   # For Bash (~/.bashrc)
   echo 'export OPENAI_API_KEY="sk-proj-YOUR_KEY_HERE"' >> ~/.bashrc
   source ~/.bashrc

   # For Zsh (~/.zshrc)
   echo 'export OPENAI_API_KEY="sk-proj-YOUR_KEY_HERE"' >> ~/.zshrc
   source ~/.zshrc
   ```

   **Using ~/.code/.env** (persistent, secure):
   ```bash
   mkdir -p ~/.code
   echo 'OPENAI_API_KEY=sk-proj-YOUR_KEY_HERE' > ~/.code/.env
   chmod 600 ~/.code/.env
   ```

3. **Launch Code**:
   ```bash
   code
   ```

   Code will automatically detect the API key and skip the login screen.

**API Key Requirements**:
- Must have write access to the Responses API
- Free tier has rate limits (3 req/min, 200 req/day)
- Upgrade to paid tier for higher limits

---

### Switching Authentication Methods

**From API Key to ChatGPT**:

```bash
# Unset API key
unset OPENAI_API_KEY

# Remove from .env if present
rm ~/.code/.env

# Run Code and select ChatGPT login
code login
```

**From ChatGPT to API Key**:

```bash
# Set API key
export OPENAI_API_KEY="sk-proj-YOUR_KEY"

# Configure preference (optional)
echo 'preferred_auth_method = "apikey"' >> ~/.code/config.toml
```

**Force specific method** in config.toml:

```toml
# Always use API key (even if ChatGPT auth exists)
preferred_auth_method = "apikey"

# Or always use ChatGPT (default)
preferred_auth_method = "chatgpt"
```

---

## Step 2: Basic Configuration

Create and customize `~/.code/config.toml`:

### Create Configuration File

```bash
# Create config directory
mkdir -p ~/.code

# Create config file
touch ~/.code/config.toml
```

### Minimal Configuration

**Recommended starter config**:

```toml
# ~/.code/config.toml

# Model Settings
model = "gpt-5"
model_provider = "openai"

# Behavior
approval_policy = "on_request"  # Model decides when to ask
model_reasoning_effort = "medium"  # low | medium | high
sandbox_mode = "workspace_write"  # read-only | workspace-write | danger-full-access

# UI Preferences
[tui]
notifications = true  # Desktop notifications for approvals
```

### Configuration Options

**Model Settings**:
```toml
model = "gpt-5"              # Default model
model_reasoning_effort = "medium"  # Reasoning depth
model_reasoning_summary = "auto"   # auto | concise | detailed | none
model_verbosity = "medium"         # low | medium | high
```

**Approval Policy**:
```toml
# When should Code ask for permission to run commands?
approval_policy = "untrusted"   # Ask for untrusted commands only
# approval_policy = "on-failure" # Ask when commands fail
# approval_policy = "on-request" # Model decides (recommended)
# approval_policy = "never"      # Never ask (full auto, risky)
```

**Sandbox Mode**:
```toml
# What can Code modify?
sandbox_mode = "read-only"        # No writes, no network
# sandbox_mode = "workspace_write" # Write to workspace, no network (recommended)
# sandbox_mode = "danger-full-access" # Full access (use in Docker/isolated env)

# Fine-tune workspace-write behavior
[sandbox_workspace_write]
allow_git_writes = true           # Allow writing to .git/ (default: true)
network_access = false            # Enable network (default: false)
writable_roots = ["/tmp"]         # Additional writable paths
```

**History and Privacy**:
```toml
# Message history
[history]
persistence = "save-all"  # save-all | none

# Zero Data Retention (for ZDR orgs)
disable_response_storage = false  # Set to true for ZDR
```

---

## Step 3: MCP Server Setup (Optional)

**What are MCP Servers?**
Model Context Protocol (MCP) servers extend Code's capabilities with custom tools:
- File operations
- Database connections
- API integrations
- Custom tools

### Install MCP Servers

**Filesystem Server** (file operations):
```bash
npm install -g @modelcontextprotocol/server-filesystem
```

**Git Status Server** (git integration):
```bash
npm install -g @modelcontextprotocol/server-git-status
```

**Local Memory Server** (persistent memory, recommended for this fork):
```bash
npm install -g @modelcontextprotocol/server-local-memory
```

### Configure MCP Servers in config.toml

Add MCP server configurations to `~/.code/config.toml`:

```toml
# MCP Servers Configuration

[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/your/project"]
startup_timeout_sec = 10
tool_timeout_sec = 60

[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-local-memory"]
startup_timeout_sec = 10
tool_timeout_sec = 30

[mcp_servers.git-status]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-git-status"]
startup_timeout_sec = 10
```

**Notes**:
- Replace `/path/to/your/project` with your actual project path
- `startup_timeout_sec`: How long to wait for server to start (default: 10)
- `tool_timeout_sec`: Max time for each tool call (default: 60)

### Verify MCP Servers

```bash
# List configured MCP servers
code mcp list

# Get details for specific server
code mcp get filesystem

# Test server health
code mcp test filesystem
```

---

## Step 4: Multi-Provider Setup (Optional)

**Why Multi-Provider?**
- Use multiple AI models for consensus and quality gates
- Cost optimization (cheap models for simple tasks, premium for critical)
- Enhanced reliability (fallback if one provider fails)

### Install Provider CLI Tools

```bash
# Anthropic Claude
npm install -g @anthropic-ai/claude-code

# Google Gemini
npm install -g @google/gemini-cli

# Verify installations
claude "test"
gemini -i "test"
```

### Configure API Keys

```bash
# Anthropic Claude
export ANTHROPIC_API_KEY="sk-ant-api03-YOUR_KEY"
# Get key: https://console.anthropic.com/settings/keys

# Google Gemini
export GOOGLE_API_KEY="AIza_YOUR_KEY"
# Get key: https://ai.google.dev/

# Add to shell profile for persistence (~/.bashrc or ~/.zshrc)
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
echo 'export GOOGLE_API_KEY="AIza..."' >> ~/.bashrc
source ~/.bashrc
```

### Configure Agents in config.toml

```toml
# Multi-Agent Configuration

[[agents]]
name = "gemini-flash"
canonical_name = "gemini"
command = "gemini"
enabled = true

[[agents]]
name = "claude-sonnet"
canonical_name = "claude"
command = "claude"
enabled = true

[[agents]]
name = "gpt-5"
canonical_name = "code"
command = "code"
enabled = true
```

### Configure Quality Gates (Spec-Kit Framework)

```toml
[quality_gates]
# Simple stages: cheap agents
tasks = ["gemini"]  # Gemini Flash: $0.075/1M tokens (12x cheaper)

# Complex stages: multi-agent consensus
plan = ["gemini", "claude", "code"]
validate = ["gemini", "claude", "code"]

# Critical stages: premium agents
audit = ["gemini-pro", "claude-opus", "gpt-5"]
unlock = ["gemini-pro", "claude-opus", "gpt-5"]
```

**Cost Comparison**:
- **Cheap strategy** (Gemini only): ~$0.10/pipeline
- **Balanced strategy** (above config): ~$2.70/pipeline
- **Premium strategy** (all top models): ~$11/pipeline

---

## Step 5: Verify Setup

### Test Authentication

```bash
# Check which auth method is active
code /status

# Should show:
# - Model: gpt-5
# - Auth: ChatGPT (or API key)
# - Provider: openai
```

### Test Basic Functionality

```bash
# Interactive mode
code

# Type in chat: "What files are in this directory?"
# Code should list files successfully
```

### Test MCP Servers (if configured)

```bash
# In Code chat, ask:
"Use the filesystem MCP server to list files in /home/user"

# Code should invoke the MCP tool and list files
```

### Test Multi-Provider (if configured)

```bash
# Run multi-agent command (requires all providers configured)
code "/plan 'Add user authentication'"

# Should see consensus from multiple models
```

---

## Configuration File Reference

### File Locations

| File | Purpose | Notes |
|------|---------|-------|
| `~/.code/config.toml` | Main configuration | **Primary** (reads legacy `~/.codex/config.toml`) |
| `~/.code/auth.json` | Authentication credentials | Auto-generated, read-only (0600 permissions) |
| `~/.code/.env` | Environment variables | Optional, for API keys |
| `~/.code/history.jsonl` | Message history | Auto-generated, can disable via config |

**Backwards Compatibility**:
- Code reads from both `~/.code/` (primary) and `~/.codex/` (legacy)
- Code only writes to `~/.code/`
- If migrating from Codex, copy `~/.codex/config.toml` to `~/.code/config.toml`

### Configuration Precedence

**Order of precedence** (highest to lowest):

1. **Command-line flags**: `--model gpt-5`, `--config key=value`
2. **Profile** (via `--profile` or `profile = "name"` in config)
3. **config.toml entries**: Direct settings
4. **Environment variables**: `OPENAI_API_KEY`, `CODEX_HOME`, etc.
5. **Default values**: Built-in Code CLI defaults

**Example**:
```bash
# All of these work, in order of precedence:
code --model o3                          # 1. CLI flag (highest)
code --profile premium                   # 2. Profile
export OPENAI_API_KEY="..."              # 3. Environment variable
# config.toml: model = "gpt-5"          # 4. Config file
# (defaults to gpt-5-codex if none set)  # 5. Default (lowest)
```

### Profiles

Create named profiles for different workflows:

```toml
# Default settings
model = "gpt-5"
approval_policy = "on-request"

# Profile for premium reasoning
[profiles.premium]
model = "o3"
model_reasoning_effort = "high"
model_reasoning_summary = "detailed"
approval_policy = "never"

# Profile for fast iteration
[profiles.fast]
model = "gpt-4o-mini"
model_reasoning_effort = "low"
approval_policy = "never"

# Profile for automation/CI
[profiles.ci]
model = "gpt-4o"
approval_policy = "never"
sandbox_mode = "read-only"
disable_response_storage = true
```

**Use profiles**:
```bash
code --profile premium "complex refactoring task"
code --profile fast "simple code formatting"
code --profile ci "run tests and generate report"
```

---

## Troubleshooting Setup

### Authentication Issues

**Error**: `Failed to authenticate`

**Solution**: Check credentials
```bash
# For ChatGPT: Delete and re-authenticate
rm ~/.code/auth.json
code login

# For API Key: Verify key is correct
echo $OPENAI_API_KEY
# Should output: sk-proj-...
```

---

**Error**: `401 Unauthorized` with API key

**Cause**: Invalid API key or insufficient permissions.

**Solution**:
```bash
# Verify API key at https://platform.openai.com/api-keys
# Ensure key has access to Responses API
# Check account verification status
```

---

**Error**: ChatGPT login fails on remote/headless server

**Solution**: Use SSH tunnel or copy credentials (see [Option A: Sign in with ChatGPT](#option-a-sign-in-with-chatgpt))

---

### Configuration Issues

**Error**: `config.toml: unknown field 'xyz'`

**Cause**: Typo or unsupported config option.

**Solution**:
```bash
# Check config syntax
code --config-check

# Refer to docs/config.md for valid options
# Common typos:
# - mcpServers → mcp_servers
# - modelProvider → model_provider
```

---

**Error**: Config changes not taking effect

**Cause**: Config file not in correct location or profile override.

**Solution**:
```bash
# Verify config file location
ls -la ~/.code/config.toml

# Check which config is loaded
code --print-config

# Disable profile override temporarily
code --profile=none
```

---

### MCP Server Issues

**Error**: `MCP server 'filesystem' failed to start`

**Cause**: Server not installed or command incorrect.

**Solution**:
```bash
# Verify server is installed
npm list -g @modelcontextprotocol/server-filesystem

# If not installed:
npm install -g @modelcontextprotocol/server-filesystem

# Test server manually
npx @modelcontextprotocol/server-filesystem /path/to/project
```

---

**Error**: MCP server times out on startup

**Cause**: Server takes longer than `startup_timeout_sec` to start.

**Solution**: Increase timeout in config.toml
```toml
[mcp_servers.slow-server]
command = "npx"
args = ["-y", "slow-mcp-server"]
startup_timeout_sec = 30  # Increase from default 10
```

---

### Multi-Provider Issues

**Error**: `Command 'claude' not found`

**Solution**: Install CLI tools
```bash
npm install -g @anthropic-ai/claude-code @google/gemini-cli

# Verify
which claude
which gemini
```

---

**Error**: `API key 'ANTHROPIC_API_KEY' not set`

**Solution**: Set environment variable
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-YOUR_KEY"

# Add to shell profile for persistence
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
source ~/.bashrc
```

---

**Error**: Rate limit exceeded

**Cause**: Too many requests to API provider.

**Solutions**:
1. **Wait**: Rate limits reset after time period
2. **Upgrade plan**: Higher tier = higher limits
3. **Use cheaper models**: Gemini Flash has higher limits
4. **Reduce agent count**: Use single agent for simple tasks

**Rate Limits** (typical free tiers):
- OpenAI: 3 requests/min, 200 requests/day
- Anthropic: 5 requests/min
- Google: 15 requests/min

---

## Next Steps

Your setup is complete! Now:

1. **Quick Start Tutorial** → [quick-start.md](quick-start.md)
   - Run your first command
   - Learn the TUI interface
   - Try example prompts

2. **Learn Common Workflows** → [workflows.md](workflows.md)
   - Spec-kit automation
   - Code refactoring
   - Multi-agent collaboration

3. **Advanced Configuration** → [../config.md](../config.md)
   - Custom model providers
   - Project-specific hooks
   - Validation harnesses

---

**Setup Complete!** 🎉 → Continue to [Quick Start](quick-start.md)
