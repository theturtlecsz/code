# Frequently Asked Questions (FAQ)

Comprehensive answers to common questions about Code CLI.

---

## Table of Contents

1. [General Questions](#general-questions)
2. [Model and Authentication](#model-and-authentication)
3. [Cost and Pricing](#cost-and-pricing)
4. [Privacy and Security](#privacy-and-security)
5. [Features and Capabilities](#features-and-capabilities)
6. [Comparison with Other Tools](#comparison-with-other-tools)
7. [Customization and Configuration](#customization-and-configuration)
8. [Troubleshooting](#troubleshooting)

---

## General Questions

### What is Code CLI?

**Code** (also `@just-every/code`) is a fast, local coding agent for your terminal. It's a community-driven fork of `openai/codex` focused on real developer ergonomics:

- 🌐 Browser integration (CDP support, headless browsing)
- 📝 Diff viewer (side-by-side diffs with syntax highlighting)
- 🤖 Multi-agent commands (/plan, /solve, /code)
- 🎨 Theme system with live preview
- 🧠 Reasoning control (dynamic effort adjustment)
- 🔌 MCP support (Model Context Protocol)
- 🔒 Safety modes (read-only, approvals, sandboxing)

**Fork Enhancements** (theturtlecsz/code):
- **Spec-Kit Framework**: Multi-agent PRD automation pipeline
- **Native MCP integration**: 5.3× faster than subprocess
- **Quality gates**: Configurable consensus checkpoints
- **Evidence repository**: Automated telemetry collection

---

### How is this different from the original OpenAI Codex?

**OpenAI Codex** (2021):
- AI model for code generation (deprecated March 2023)
- Not related to this CLI tool

**Code CLI** (this project):
- Community fork of `openai/codex` terminal interface
- Adds browser integration, multi-agent workflows, themes
- Maintains full compatibility with upstream
- Uses modern models (GPT-5, Claude, Gemini)

---

### Is this affiliated with OpenAI or Anthropic?

**No.** Code is a community-driven open source project:
- **Not** affiliated with, sponsored by, or endorsed by OpenAI
- **Not** affiliated with Anthropic (though supports Claude via CLI)
- **Not** related to "Anthropic's Claude Code" (different product)
- Apache 2.0 license, community maintained

---

### Can I use my existing Codex configuration?

**Yes.** Code maintains full backwards compatibility:

- Reads from both `~/.code/` (primary) and `~/.codex/` (legacy)
- Writes only to `~/.code/`
- Automatically migrates settings on first run
- Codex will keep running if you switch back

**To migrate manually**:

```bash
# Copy config
cp ~/.codex/config.toml ~/.code/config.toml

# Copy auth (if using ChatGPT login)
cp ~/.codex/auth.json ~/.code/auth.json
```

---

### What operating systems are supported?

**Officially Supported**:
- ✅ macOS 12+ (Monterey and later)
- ✅ Ubuntu 20.04+ / Debian 10+
- ✅ Windows 11 **via WSL2** (Windows Subsystem for Linux)

**Experimental**:
- ⚠️ Other Linux distributions (Alpine, Fedora, Arch) - usually work
- ⚠️ Direct Windows install - may work but unsupported

**Not Supported**:
- ❌ macOS 11 and earlier
- ❌ Ubuntu 18.04 and earlier
- ❌ Windows without WSL2

---

## Model and Authentication

### Which models are supported?

**OpenAI Models** (primary):
- **GPT-5** (recommended, default: `gpt-5-codex`)
- **GPT-4o** (faster, cheaper)
- **GPT-4o-mini** (cheapest, good for simple tasks)
- **o3**, **o4-mini** (reasoning models)

**Anthropic Claude** (via multi-agent setup):
- **Claude Sonnet 4.5** (balanced)
- **Claude Haiku 3.5** (cheap and fast)
- **Claude Opus 3.5** (premium reasoning)

**Google Gemini** (via multi-agent setup):
- **Gemini Pro 1.5** (balanced)
- **Gemini Flash 1.5** (cheapest: $0.075/1M tokens)

**Local Models** (experimental):
- Ollama support via custom provider configuration
- Any OpenAI API-compatible endpoint

---

### Do I need ChatGPT Plus or an API key?

**You need ONE of the following**:

**Option 1: ChatGPT Subscription** (no per-token billing)
- ChatGPT Plus ($20/month)
- ChatGPT Pro ($200/month)
- ChatGPT Team (varies)
- Uses models included in your plan
- ✅ Best for: Regular interactive use

**Option 2: OpenAI API Key** (pay-as-you-go)
- Usage-based billing (see [pricing](#how-much-does-it-cost))
- ✅ Best for: Automation, CI/CD, precise cost control

**ChatGPT Free Tier does NOT work** with Code CLI.

---

### Can I switch between ChatGPT auth and API key?

**Yes**, anytime:

**Switch to API key** (from ChatGPT):

```bash
# Set API key environment variable
export OPENAI_API_KEY="sk-proj-YOUR_KEY"

# Or add to config.toml
echo 'preferred_auth_method = "apikey"' >> ~/.code/config.toml
```

**Switch to ChatGPT** (from API key):

```bash
# Remove API key
unset OPENAI_API_KEY

# Remove from .env if present
rm ~/.code/.env

# Re-authenticate
code login
```

**Force specific method** in config:

```toml
# ~/.code/config.toml
preferred_auth_method = "chatgpt"  # or "apikey"
```

---

### Why does `o3` or `o4-mini` not work for me?

**Possible causes**:

1. **Account not verified**: Free tier API accounts need [verification](https://help.openai.com/en/articles/10910291-api-organization-verification) to access reasoning models
2. **Model not available to your account**: Some models require paid tier
3. **Wrong model name**: Use exact name (e.g., `o3` not `o-3`)

**Solution**:

```bash
# Check account verification at platform.openai.com
# Upgrade to paid tier if needed
# Or use GPT-5 instead:

code --model gpt-5 "your task"
```

---

## Cost and Pricing

### How much does it cost?

**With ChatGPT Plus/Pro/Team**:
- **$0 per use** (covered by subscription)
- Already paying for ChatGPT subscription

**With API Key** (pay-as-you-go):

**Model Pricing** (January 2025):

| Provider | Model | Input (1M tokens) | Output (1M tokens) |
|----------|-------|-------------------|--------------------|
| **OpenAI** | GPT-5 | $5.00 | $15.00 |
| | GPT-4o | $2.50 | $10.00 |
| | GPT-4o-mini | $0.15 | $0.60 |
| **Anthropic** | Claude Sonnet 4.5 | $3.00 | $15.00 |
| | Claude Haiku 3.5 | $0.80 | $4.00 |
| | Claude Opus 3.5 | $15.00 | $75.00 |
| **Google** | Gemini Pro 1.5 | $1.25 | $5.00 |
| | **Gemini Flash 1.5** | **$0.075** | **$0.30** |

**Typical Usage Costs**:

| Task | Model | Estimated Cost |
|------|-------|----------------|
| Simple code explanation | GPT-4o-mini | ~$0.01 |
| Refactor function | GPT-4o | ~$0.05 |
| Generate module with tests | GPT-5 | ~$0.20 |
| Full Spec-Kit pipeline | Multi-agent balanced | ~$2.70 |
| Complex architectural decision | o3 (high reasoning) | ~$1.50 |

**Cost-Saving Strategies**:

1. **Use cheaper models for simple tasks**:
   ```bash
   code --model gpt-4o-mini "format this file"
   ```

2. **Use Gemini Flash** (12× cheaper than GPT-4o):
   ```toml
   [quality_gates]
   tasks = ["gemini"]  # $0.075/1M vs $2.50/1M for GPT-4o
   ```

3. **Optimize Spec-Kit quality gates**:
   ```toml
   # Cheap for simple stages, premium for critical
   tasks = ["gemini"]           # ~$0.10
   plan = ["gemini", "claude"]  # ~$0.35 (multi-agent)
   audit = ["gpt-5"]            # ~$0.80 (premium for critical)
   ```

4. **Use native tools** (FREE):
   ```bash
   /speckit.new        # $0 (native)
   /speckit.clarify    # $0 (native heuristics)
   /speckit.analyze    # $0 (native structural diff)
   /speckit.checklist  # $0 (native rubric scoring)
   ```

---

### Is there a free tier?

**For API usage**:
- OpenAI free tier: 3 requests/min, 200 requests/day (very limited)
- Anthropic free tier: 5 requests/min
- Google Gemini free tier: 15 requests/min, 1,500 requests/day

**Best free option**: Use **Gemini Flash** (12.5× cheaper) or get ChatGPT Plus subscription (unlimited use within plan limits).

---

### How can I monitor my costs?

**With API Key**:

1. **OpenAI Dashboard**: https://platform.openai.com/usage
   - Shows daily/monthly usage
   - Breakdown by model
   - Set spending limits

2. **Enable debug mode** to see token counts:
   ```bash
   code --debug
   ```

3. **Use `--read-only` mode** for cost-free exploration:
   ```bash
   code --read-only "analyze this codebase"
   ```

**With ChatGPT subscription**:
- No per-token billing
- Covered by flat monthly fee

---

## Privacy and Security

### Is my data secure?

**Yes.** Code CLI follows these security practices:

1. **Authentication stays local**:
   - Credentials stored at `~/.code/auth.json` (0600 permissions)
   - No proxying through third-party servers
   - Direct communication with OpenAI/Anthropic/Google

2. **No telemetry by default**:
   - Code doesn't send usage data to project maintainers
   - Only communication is with AI providers you configure

3. **Conversation history**:
   - Stored locally at `~/.code/history.jsonl`
   - File permissions: 0600 (owner read/write only)
   - Can disable: `[history] persistence = "none"`

4. **Sandbox modes**:
   - `read-only`: No file writes, no network
   - `workspace-write`: Limited writes to workspace only
   - `danger-full-access`: Full access (use in Docker/isolated env)

---

### Where does my data go?

**Inputs/outputs** you send through Code are handled under AI provider terms:

- **OpenAI**: See [OpenAI Privacy Policy](https://openai.com/policies/privacy-policy)
- **Anthropic**: See [Anthropic Privacy Policy](https://www.anthropic.com/privacy)
- **Google**: See [Google AI Privacy Policy](https://ai.google.dev/terms)

**Key points**:
- Code doesn't store or proxy your conversations
- AI providers may use data per their terms (check policy)
- For zero data retention: Use OpenAI ZDR (Zero Data Retention) orgs
  ```toml
  disable_response_storage = true
  ```

---

### Can I use Code in an enterprise environment?

**Yes**, with considerations:

1. **API Key method** (recommended for enterprise):
   ```bash
   export OPENAI_API_KEY="sk-proj-ENTERPRISE_KEY"
   ```

2. **Zero Data Retention** (for sensitive code):
   ```toml
   # ~/.code/config.toml
   disable_response_storage = true
   ```

3. **Network restrictions**:
   - Requires outbound HTTPS to `api.openai.com`, `api.anthropic.com`, etc.
   - Configure proxy if needed:
     ```bash
     export HTTPS_PROXY="http://proxy.company.com:8080"
     ```

4. **Disable history** (for compliance):
   ```toml
   [history]
   persistence = "none"
   ```

5. **Read-only mode** for analysis:
   ```bash
   code --read-only "analyze codebase for vulnerabilities"
   ```

---

### How do I prevent Code from editing my files?

**Use read-only mode**:

```bash
# CLI flag
code --read-only

# Or in config.toml
sandbox_mode = "read-only"

# Or mid-conversation
/approvals
# Select "Read Only" preset
```

**Read-only mode**:
- ✅ Can read files
- ✅ Can run commands (sandboxed)
- ✅ Can answer questions
- ❌ Cannot write files
- ❌ Cannot modify code

---

## Features and Capabilities

### What can Code CLI do?

**Core Capabilities**:
- ✅ Code generation (functions, modules, full features)
- ✅ Code refactoring and optimization
- ✅ Bug fixing and debugging
- ✅ Test generation (unit, integration, E2E)
- ✅ Documentation generation (README, API docs, comments)
- ✅ Code review and analysis
- ✅ Codebase Q&A and exploration
- ✅ File operations (read, write, modify with approval)
- ✅ Command execution (sandboxed)

**Advanced Features**:
- ✅ Browser control (Chrome DevTools Protocol)
- ✅ Multi-agent workflows (consensus, racing, collaboration)
- ✅ MCP server integration (filesystem, databases, APIs)
- ✅ Spec-Kit automation (fork feature: PRD → implementation pipeline)
- ✅ Quality gates (multi-agent validation checkpoints)
- ✅ Theming and customization
- ✅ Reasoning control (adjust effort dynamically)

---

### Does it work offline?

**No**, Code requires internet for AI models:
- OpenAI API requires network
- Claude and Gemini also require network
- MCP servers may require network (depends on server)

**Partial offline** (experimental):
- Use local models via Ollama (requires setup)
- Configure local OpenAI-compatible endpoint
- Quality depends on local model capabilities

---

### Can Code CLI commit and push to Git?

**Yes**, with proper sandbox configuration:

```toml
# Allow git operations
sandbox_mode = "workspace_write"

[sandbox_workspace_write]
allow_git_writes = true  # Default: true
```

**Example**:

```bash
code "Create a commit for the changes we made with message 'Add user authentication'"

# Code will:
# 1. Stage changes (git add)
# 2. Create commit (git commit)
# 3. Show commit hash and message
```

**Safety**:
- Code doesn't push automatically (unless explicitly requested)
- Always review commits before pushing
- Use `/status` to check git state

---

### Can Code generate entire applications from scratch?

**Yes**, but with considerations:

**Best for**:
- ✅ Small to medium applications (todo apps, APIs, dashboards)
- ✅ Well-defined requirements (clear specifications)
- ✅ Standard tech stacks (React, Express, Flask, etc.)

**Challenges**:
- ⚠️ Large applications may hit context limits
- ⚠️ Requires iterative refinement
- ⚠️ Generated code needs review and testing

**Recommended approach**:

1. **Use Spec-Kit automation** for structured development:
   ```bash
   /speckit.new Build a REST API for a blog with user auth, posts, and comments
   /speckit.auto SPEC-ID
   ```

2. **Break into modules**:
   ```bash
   # Generate one module at a time
   code "Create the user authentication module with JWT"
   code "Create the blog post CRUD operations"
   code "Create the comments module with nested replies"
   ```

3. **Iterate and refine**:
   ```bash
   code "Add input validation to the auth module"
   code "Add rate limiting to the API endpoints"
   code "Generate comprehensive tests for all modules"
   ```

---

## Comparison with Other Tools

### How is Code different from GitHub Copilot?

| Feature | Code CLI | GitHub Copilot |
|---------|----------|----------------|
| **Interface** | Terminal (TUI) | IDE extension |
| **Scope** | Full file context, multi-file | Line/function suggestions |
| **Autonomy** | Can execute commands, make changes | Suggestions only |
| **Conversation** | Interactive chat | No conversation |
| **Testing** | Can run tests, fix failures | No test execution |
| **Reasoning** | Adjustable reasoning levels | Fixed |
| **Multi-agent** | Yes (consensus/racing) | No |
| **Browser control** | Yes (CDP) | No |
| **Cost** | Pay-per-use or ChatGPT sub | $10/month subscription |

**Use Code CLI for**:
- Large refactorings
- Debugging complex issues
- Test generation and execution
- Documentation generation
- Multi-file code generation

**Use Copilot for**:
- Quick inline suggestions while coding
- Auto-completion
- IDE-integrated workflow

---

### How is Code different from Cursor?

| Feature | Code CLI | Cursor |
|---------|----------|--------|
| **Interface** | Terminal | Full IDE (VS Code fork) |
| **Autonomy** | Full automation pipelines | Assisted coding |
| **Spec-Kit** | Yes (fork feature) | No |
| **Multi-agent** | Yes | Limited |
| **Browser control** | Yes | No |
| **Setup** | Lightweight (CLI only) | Full IDE installation |
| **Terminal workflow** | Native | Via IDE terminal |
| **Cost** | Flexible (API or subscription) | $20/month |

**Use Code CLI for**:
- Terminal-first workflows
- CI/CD automation
- Spec-Kit PRD automation
- Multi-agent consensus
- Server/remote environments

**Use Cursor for**:
- IDE-integrated development
- GUI-first workflows
- Inline editing with AI assistance

---

### How does Spec-Kit compare to other AI dev tools?

**Spec-Kit** (theturtlecsz/code fork feature):
- ✅ Full PRD → Plan → Tasks → Implementation → Validation → Audit pipeline
- ✅ Multi-agent consensus at each stage
- ✅ Configurable quality gates
- ✅ Native cost optimization ($2.70 vs $11 for full pipeline)
- ✅ Evidence collection and telemetry
- ✅ Automated or manual step-through

**Other tools**:
- Devin, Replit Agent: Cloud-based, less customizable
- Aider: Terminal-based but single-agent, no pipeline
- Smol Developer: Script-based, no consensus

**Spec-Kit advantages**:
- Multi-agent validation (3-5 agents per critical stage)
- Quality gates prevent bad implementations early
- Evidence trail for debugging and auditing
- Cost-optimized (cheap agents for simple stages, premium for critical)

---

## Customization and Configuration

### Can I customize the model for different tasks?

**Yes**, multiple ways:

**Per-command** (CLI flag):

```bash
code --model gpt-4o-mini "simple formatting task"
code --model o3 --config model_reasoning_effort=high "complex refactoring"
```

**Profiles** (named configurations):

```toml
# ~/.code/config.toml

[profiles.fast]
model = "gpt-4o-mini"
model_reasoning_effort = "low"
approval_policy = "never"

[profiles.premium]
model = "o3"
model_reasoning_effort = "high"
approval_policy = "on-request"

[profiles.automation]
model = "gpt-4o"
approval_policy = "never"
sandbox_mode = "read-only"
```

**Use profiles**:

```bash
code --profile fast "quick task"
code --profile premium "complex architecture decision"
code --profile automation "generate report"
```

**Dynamic switching** in TUI:

```bash
# In Code:
/model          # Interactive model selector
/reasoning high # Adjust reasoning mid-conversation
```

---

### Can I extend Code with custom tools?

**Yes**, via MCP (Model Context Protocol) servers:

**Example custom MCP server** (Node.js):

```javascript
// custom-mcp-server.js
import { Server } from "@modelcontextprotocol/sdk/server";

const server = new Server({
  name: "custom-tools",
  version: "1.0.0"
});

// Define custom tool
server.tool("deploy_to_production", async (params) => {
  // Your custom deployment logic
  return { success: true, message: "Deployed successfully" };
});

server.listen();
```

**Configure in config.toml**:

```toml
[mcp_servers.custom-tools]
command = "node"
args = ["/path/to/custom-mcp-server.js"]
```

**Use in Code**:

```bash
code "Deploy the application to production"
# Code will invoke your custom MCP tool
```

---

### Can I use Code with custom OpenAI-compatible endpoints?

**Yes**, configure custom provider:

```toml
# ~/.code/config.toml

[model_providers.custom]
name = "Custom Provider"
base_url = "https://custom-api.example.com/v1"
env_key = "CUSTOM_API_KEY"
wire_api = "chat"  # or "responses"

# Use custom provider
model = "custom-model"
model_provider = "custom"
```

**Examples**:

**Ollama** (local models):

```toml
[model_providers.ollama]
name = "Ollama"
base_url = "http://localhost:11434/v1"

model = "mistral"
model_provider = "ollama"
```

**Azure OpenAI**:

```toml
[model_providers.azure]
name = "Azure OpenAI"
base_url = "https://YOUR_PROJECT.openai.azure.com/openai"
env_key = "AZURE_OPENAI_API_KEY"
query_params = { api-version = "2025-04-01-preview" }
wire_api = "responses"
```

---

## Troubleshooting

### Why is Code slow to start?

**Common causes**:

1. **Large history file**: `~/.code/history.jsonl` >100MB
   ```bash
   ls -lh ~/.code/history.jsonl
   # Truncate if large
   mv ~/.code/history.jsonl ~/.code/history.jsonl.backup
   ```

2. **Slow MCP servers**: Servers take time to initialize
   ```bash
   # Temporarily disable to test
   # Comment out in config.toml
   ```

3. **Network issues**: Slow connection to API providers
   ```bash
   # Test connectivity
   ping api.openai.com
   ```

**Solutions** → See [troubleshooting.md](troubleshooting.md#issue-code-cli-is-slow-to-start)

---

### Why do I keep getting rate limit errors?

**Causes**:
- Free tier API limits (3 req/min, 200 req/day for OpenAI)
- Too many Spec-Kit multi-agent requests
- Shared IP address (VPN, corporate network)

**Solutions**:

1. **Upgrade to paid tier**: Higher rate limits
2. **Use cheaper providers**: Gemini has higher free tier limits
3. **Wait and retry**: Rate limits reset after time period
4. **Reduce agent count**: Use single agent for simple tasks
   ```toml
   [quality_gates]
   tasks = ["code"]  # Single agent instead of multi-agent
   ```

---

### Code generated incorrect/broken code. What do I do?

**Immediate steps**:

1. **Review the diff before approving**:
   - Always check changes carefully
   - Understand why changes were made
   - Look for unintended side effects

2. **Reject and provide feedback**:
   ```bash
   # In approval prompt, reject and respond:
   "The function should use async/await, not callbacks. Please refactor."
   ```

3. **Use higher reasoning for complex tasks**:
   ```bash
   code --model o3 --config model_reasoning_effort=high "complex task"
   ```

4. **Use multi-agent consensus** for critical changes:
   ```bash
   /code "refactor authentication system"
   # Multiple agents review and validate
   ```

**Prevention**:

- ✅ Write specific, clear prompts
- ✅ Provide examples of desired output
- ✅ Review diffs before approving
- ✅ Run tests after changes
- ✅ Use read-only mode first to see proposed changes

---

### Where can I get more help?

**Documentation**:
- [Troubleshooting Guide](troubleshooting.md) - Comprehensive error solutions
- [Installation Guide](installation.md) - Setup issues
- [Configuration Docs](../../config.md) - Advanced configuration

**Community**:
- GitHub Issues: https://github.com/just-every/code/issues
- GitHub Discussions: https://github.com/just-every/code/discussions

**Fork-Specific** (theturtlecsz/code):
- Fork Issues: https://github.com/theturtlecsz/code/issues
- Spec-Kit Docs: [../../spec-kit/README.md](../../spec-kit/README.md)

---

## Additional Questions?

**Didn't find your question?**

1. Search GitHub Issues: https://github.com/just-every/code/issues
2. Check Discussions: https://github.com/just-every/code/discussions
3. Open a new issue with "Question:" prefix

**Before asking**:
- ✅ Search existing issues/discussions
- ✅ Review relevant documentation sections
- ✅ Provide version info and error messages
- ✅ Include steps to reproduce (if applicable)

---

**Got your answer?** → Continue exploring [workflows](workflows.md) or dive into [advanced configuration](../../config.md)!
