# Authentication

## Usage-based billing alternative: Use an OpenAI API key

If you prefer to pay-as-you-go, you can still authenticate with your OpenAI API key by setting it as an environment variable:

```shell
export OPENAI_API_KEY="your-api-key-here"
```

This key must, at minimum, have write access to the Responses API.

## Migrating to ChatGPT login from API key

If you've used the Codex CLI before with usage-based billing via an API key and want to switch to using your ChatGPT plan, follow these steps:

1. Update the CLI and ensure `codex --version` is `0.20.0` or later
2. Delete `~/.code/auth.json` (and remove the legacy `~/.codex/auth.json` if it exists; on Windows these live under `C:\\Users\\USERNAME\\.code\\auth.json` and `C:\\Users\\USERNAME\\.codex\\auth.json`)
3. Run `codex login` again

## Forcing a specific auth method (advanced)

You can explicitly choose which authentication Codex should prefer when both are available.

- To always use your API key (even when ChatGPT auth exists), set:

```toml
# ~/.code/config.toml (Code also reads legacy ~/.codex/config.toml)
preferred_auth_method = "apikey"
```

Or override ad-hoc via CLI:

```bash
codex --config preferred_auth_method="apikey"
```

- To prefer ChatGPT auth (default), set:

```toml
# ~/.code/config.toml (Code also reads legacy ~/.codex/config.toml)
preferred_auth_method = "chatgpt"
```

Notes:

- When `preferred_auth_method = "apikey"` and an API key is available, the login screen is skipped.
- When `preferred_auth_method = "chatgpt"` (default), Codex prefers ChatGPT auth if present; if only an API key is present, it will use the API key. Certain account types may also require API-key mode.
- To check which auth method is being used during a session, use the `/status` command in the TUI.

## Project .env safety (OPENAI_API_KEY)

By default, Codex will no longer read `OPENAI_API_KEY` or `AZURE_OPENAI_API_KEY` from a project’s local `.env` file.

Why: many repos include an API key in `.env` for unrelated tooling, which could cause Codex to silently use the API key instead of your ChatGPT plan in that folder.

What still works:

- `~/.code/.env` (or `~/.codex/.env`) is loaded first and may contain your `OPENAI_API_KEY` for global use.
- A shell-exported `OPENAI_API_KEY` is honored.

Project `.env` provider keys are always ignored — there is no opt‑in.

UI clarity:

- When Codex is using an API key, the chat footer shows a bold “Auth: API key” badge so it’s obvious which mode you’re in.

## Connecting on a "Headless" Machine

Today, the login process entails running a server on `localhost:1455`. If you are on a "headless" server, such as a Docker container or are `ssh`'d into a remote machine, loading `localhost:1455` in the browser on your local machine will not automatically connect to the webserver running on the _headless_ machine, so you must use one of the following workarounds:

### Authenticate locally and copy your credentials to the "headless" machine

The easiest solution is likely to run through the `codex login` process on your local machine such that `localhost:1455` _is_ accessible in your web browser. When you complete the authentication process, an `auth.json` file should be available at `$CODE_HOME/auth.json` (defaults to `~/.code/auth.json`; Code will still read `$CODEX_HOME`/`~/.codex/auth.json` if present).

Because the `auth.json` file is not tied to a specific host, once you complete the authentication flow locally, you can copy the `$CODEX_HOME/auth.json` file to the headless machine and then `codex` should "just work" on that machine. Note to copy a file to a Docker container, you can do:

```shell
# substitute MY_CONTAINER with the name or id of your Docker container:
CONTAINER_HOME=$(docker exec MY_CONTAINER printenv HOME)
docker exec MY_CONTAINER mkdir -p "$CONTAINER_HOME/.code"
docker cp auth.json MY_CONTAINER:"$CONTAINER_HOME/.code/auth.json"
```

whereas if you are `ssh`'d into a remote machine, you likely want to use [`scp`](https://en.wikipedia.org/wiki/Secure_copy_protocol):

```shell
ssh user@remote 'mkdir -p ~/.code'
scp ~/.code/auth.json user@remote:~/.code/auth.json
```

or try this one-liner:

```shell
ssh user@remote 'mkdir -p ~/.code && cat > ~/.code/auth.json' < ~/.code/auth.json
```

### Connecting through VPS or remote

If you run Codex on a remote machine (VPS/server) without a local browser, the login helper starts a server on `localhost:1455` on the remote host. To complete login in your local browser, forward that port to your machine before starting the login flow:

```bash
# From your local machine
ssh -L 1455:localhost:1455 <user>@<remote-host>
```

Then, in that SSH session, run `codex` and select "Sign in with ChatGPT". When prompted, open the printed URL (it will be `http://localhost:1455/...`) in your local browser. The traffic will be tunneled to the remote server.

---

## Multi-Provider Setup (SPEC-939)

Code supports orchestrating multiple AI providers for quality gates and multi-agent workflows.

### Supported Providers

**OpenAI (GPT-5, GPT-4o)**:
```bash
export OPENAI_API_KEY="sk-proj-..."
```
Get API key: https://platform.openai.com/api-keys

**Anthropic (Claude Sonnet, Haiku, Opus)**:
```bash
export ANTHROPIC_API_KEY="sk-ant-api03-..."
```
Get API key: https://console.anthropic.com/settings/keys

**Google (Gemini Pro, Flash)**:
```bash
export GOOGLE_API_KEY="AIza..."
```
Get API key: https://ai.google.dev/

### Install CLI Tools

For multi-agent commands (`/plan`, `/solve`, `/code`), install provider CLI tools:

```bash
npm install -g @anthropic-ai/claude-code @google/gemini-cli

# Verify installation
claude "test"
gemini -i "test"
```

---

## Quality Gate Configuration

Customize agent selection per quality checkpoint to balance cost and quality:

```toml
# ~/.code/config.toml

[quality_gates]
# Simple stages: Use cheap agents
tasks = ["gemini"]  # Gemini Flash: $0.075/1M tokens (12x cheaper)

# Complex stages: Multi-agent consensus
plan = ["gemini", "claude", "code"]
validate = ["gemini", "claude", "code"]

# Critical stages: Premium agents only
audit = ["gemini-pro", "claude-opus", "gpt-5"]
unlock = ["gemini-pro", "claude-opus", "gpt-5"]
```

**Benefits**:
- **Cost Control**: Use cheaper agents for simple stages, premium for critical
- **Experimentation**: Try different combinations per checkpoint
- **Flexibility**: Adjust quality/cost tradeoff dynamically

**Cost Examples**:
- Cheap strategy (Gemini Flash only): ~$0.10/full pipeline
- Balanced strategy (above config): ~$2.70/full pipeline
- Premium strategy (all top models): ~$11/full pipeline

---

## Canonical Agent Names

Configure agents with canonical names for consistent reference:

```toml
[[agents]]
name = "gemini-flash"
canonical_name = "gemini"  # Single source of truth
command = "gemini"
enabled = true

[[agents]]
name = "claude-sonnet"
canonical_name = "claude"
command = "anthropic"
enabled = true

[[agents]]
name = "gpt-5-turbo"
canonical_name = "code"
command = "openai"
enabled = true
```

**Benefits**:
- Eliminates naming confusion (config name vs command name vs model ID)
- Consistent references across codebase
- Easier debugging (logs use canonical names)

---

## Pricing Reference (January 2025)

| Provider | Model | Input (1M tokens) | Output (1M tokens) |
|----------|-------|-------------------|-------------------|
| **OpenAI** | GPT-5 | $5.00 | $15.00 |
| | GPT-4o | $2.50 | $10.00 |
| | GPT-4o-mini | $0.15 | $0.60 |
| **Anthropic** | Claude Sonnet 4.5 | $3.00 | $15.00 |
| | Claude Haiku 3.5 | $0.80 | $4.00 |
| | Claude Opus 3.5 | $15.00 | $75.00 |
| **Google** | Gemini Pro 1.5 | $1.25 | $5.00 |
| | Gemini Flash 1.5 | **$0.075** | **$0.30** |

**Cheapest option**: Gemini Flash (12x cheaper than GPT-4o)

---

## Troubleshooting Multi-Provider Setup

### Error: "Command 'gemini' not found"

**Solution**: Install CLI tools
```bash
npm install -g @google/gemini-cli @anthropic-ai/claude-code

# Verify
which gemini
which claude
```

### Error: "API key 'ANTHROPIC_API_KEY' not set"

**Solution 1**: Set environment variable
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

**Solution 2**: Add to shell profile for persistence
```bash
# ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="AIza..."
```

Then reload: `source ~/.bashrc`

### Error: "Rate limit exceeded"

**Cause**: Too many requests to API provider

**Solutions**:
1. **Wait**: Rate limits reset after time period
2. **Upgrade plan**: Higher tier = higher limits
3. **Retry**: Code automatically retries with exponential backoff

**Rate Limits** (typical free tiers):
- OpenAI: 3 requests/min, 200 requests/day
- Anthropic: 5 requests/min
- Google: 15 requests/min

---

## Additional Resources

- [Configuration Guide](../README.md#configuration)
- [Quality Gate Customization](../README.md#quality-gate-customization-spec-939)
- [SPEC-939 PRD](SPEC-KIT-939-configuration-management/PRD.md)
- [OpenAI API Docs](https://platform.openai.com/docs)
- [Anthropic API Docs](https://docs.anthropic.com)
- [Google AI Docs](https://ai.google.dev/docs)

---

**Last Updated**: 2025-11-15 (SPEC-939 Configuration Management)
