# CLI Routing Approach - RECOMMENDED SOLUTION

**SPEC-ID**: SPEC-KIT-951 (Updated Approach)
**Created**: 2025-11-19
**Status**: RECOMMENDED

---

## Executive Summary

**Recommendation**: **GO - CLI Routing Approach** (bypasses OAuth blocker)

Instead of implementing OAuth for all providers, **route commands through native CLIs** that already handle authentication.

**Key Insight**: Since codex-rs uses `/model` command to select providers for spec-kit operations, we can delegate execution to the native CLIs rather than making direct API calls.

---

## Architecture Comparison

### ❌ Original OAuth Approach (BLOCKED)
```
User → codex-rs TUI → OAuth flow → Direct API calls
                         ↓
                    BLOCKED: Claude has no OAuth
```

**Problems**:
- Claude doesn't support OAuth for third-party apps
- Complex implementation (15-24 hours)
- Must manage tokens for multiple providers
- Must handle refresh logic for each provider

### ✅ CLI Routing Approach (RECOMMENDED)
```
User → codex-rs TUI → Route to native CLI → CLI handles auth
                         ↓
                    ✅ Works for all providers!
```

**Advantages**:
- ✅ Bypasses Claude OAuth blocker completely
- ✅ Simpler implementation (5-8 hours)
- ✅ Users authenticate once per CLI (not per app)
- ✅ CLIs handle token management automatically
- ✅ Reduces codebase complexity dramatically

---

## Implementation Design

### Model Selection Flow

```rust
// When user selects model via /model command
pub enum ModelProvider {
    ChatGPT,  // Native (existing OAuth)
    Claude,   // CLI routing (new)
    Gemini,   // CLI routing (new)
}

impl ModelProvider {
    pub async fn execute_prompt(&self, prompt: &str, config: &ModelConfig) -> Result<Response> {
        match self {
            Self::ChatGPT => {
                // Use existing codex-rs OAuth implementation
                execute_native_chatgpt_api(prompt, config).await
            }
            Self::Claude => {
                // Route through claude CLI
                execute_cli_command(
                    "claude",
                    &["-p", prompt, "--output-format", "json"]
                ).await
            }
            Self::Gemini => {
                // Route through gemini CLI
                execute_cli_command(
                    "gemini",
                    &["-p", prompt, "-m", &config.model_name]
                ).await
            }
        }
    }
}
```

### CLI Command Execution

```rust
async fn execute_cli_command(cli: &str, args: &[&str]) -> Result<Response> {
    use tokio::process::Command;

    // 1. Check if CLI is available
    let cli_path = which::which(cli)
        .map_err(|_| format!("{} CLI not found in PATH", cli))?;

    // 2. Execute with timeout
    let output = tokio::time::timeout(
        Duration::from_secs(300), // 5 minute timeout
        Command::new(cli_path)
            .args(args)
            .output()
    ).await??;

    // 3. Parse output
    if output.status.success() {
        let response_text = String::from_utf8(output.stdout)?;
        Ok(parse_cli_response(cli, &response_text)?)
    } else {
        let error = String::from_utf8(output.stderr)?;
        Err(format!("{} CLI error: {}", cli, error).into())
    }
}
```

### Response Parsing

```rust
fn parse_cli_response(cli: &str, output: &str) -> Result<Response> {
    match cli {
        "claude" => {
            // Claude with --output-format json
            let json: serde_json::Value = serde_json::from_str(output)?;
            Ok(Response {
                content: json["response"].as_str().unwrap_or("").to_string(),
                model: json["model"].as_str().unwrap_or("claude").to_string(),
                // ... extract other fields
            })
        }
        "gemini" => {
            // Gemini returns plain text by default
            Ok(Response {
                content: output.trim().to_string(),
                model: "gemini".to_string(),
                // ...
            })
        }
        _ => Err("Unknown CLI".into())
    }
}
```

---

## CLI Interface Specifications

### Claude CLI

**Command**: `claude`
**Installation**: User must install claude-code CLI and authenticate
**Location**: `/home/thetu/.local/bin/claude`

**Non-Interactive Execution**:
```bash
claude -p "your prompt here" --output-format json
```

**Key Flags**:
- `-p, --print`: Print response and exit (non-interactive)
- `--output-format <format>`: `text` | `json` | `stream-json`
- `--json-schema <schema>`: Structured output validation
- `--tools <tools...>`: Control which tools are available
- `--system-prompt <prompt>`: Custom system prompt
- `--model <model>`: Select specific Claude model

**Authentication**:
- User authenticates once: `claude` (interactive login)
- Credentials stored in Claude Code config
- No additional auth needed from codex-rs

---

### Gemini CLI

**Command**: `gemini`
**Installation**: User must install gemini-cli and authenticate
**Location**: `/home/thetu/.nvm/versions/node/v22.18.0/bin/gemini`

**Non-Interactive Execution**:
```bash
gemini -p "your prompt here" -m "gemini-2.0-flash"
```

**Key Flags**:
- `-p, --prompt`: Prompt text (non-interactive one-shot)
- `-m, --model`: Model selection
- `-y, --yolo`: Auto-approve all actions
- `--approval-mode`: `default` | `auto_edit` | `yolo`
- `--allowed-tools`: Tools allowed without confirmation
- `--extensions <list>`: Extensions to use

**Authentication**:
- User authenticates once via OAuth flow in gemini CLI
- Credentials managed by gemini-cli
- No additional auth needed from codex-rs

---

### ChatGPT (Native)

**Implementation**: Existing codex-rs OAuth (keep as-is)
**Authentication**: Current OAuth 2.0 flow already working
**No changes needed**: Continue using native implementation

---

## Implementation Checklist

### Phase 1: CLI Detection & Validation (2 hours)

- [ ] Implement CLI availability check (`which claude`, `which gemini`)
- [ ] Provide clear error messages if CLI not found
- [ ] Add documentation for installing CLIs
- [ ] Verify user has authenticated (test command execution)

### Phase 2: Command Execution (2 hours)

- [ ] Implement `execute_cli_command` with timeout
- [ ] Add stdout/stderr capture
- [ ] Handle process errors gracefully
- [ ] Add logging for debugging

### Phase 3: Response Parsing (2 hours)

- [ ] Parse Claude JSON output
- [ ] Parse Gemini text/JSON output
- [ ] Normalize responses to common format
- [ ] Handle edge cases (empty responses, errors)

### Phase 4: Model Router Integration (2 hours)

- [ ] Update `ModelProvider` enum with CLI routing
- [ ] Integrate with `/model` command selection
- [ ] Update spec-kit command execution to use router
- [ ] Maintain backward compatibility with ChatGPT OAuth

### Phase 5: Testing & Validation (2 hours)

- [ ] Test with all three providers
- [ ] Validate error handling
- [ ] Performance testing (CLI overhead)
- [ ] User documentation

**Total Estimated Effort**: **8-12 hours**

---

## Advantages Over OAuth Approach

| Aspect | OAuth Approach | CLI Routing Approach |
|--------|----------------|---------------------|
| **Claude Support** | ❌ BLOCKED (no OAuth) | ✅ Works via CLI |
| **Implementation Time** | 15-24 hours | 8-12 hours |
| **Code Complexity** | HIGH (OAuth flows, token mgmt) | LOW (command execution) |
| **User Setup** | Multi-step OAuth | One-time CLI auth |
| **Token Management** | We handle it | CLI handles it |
| **Maintenance** | High (3 OAuth implementations) | Low (delegate to CLIs) |
| **Testing** | Complex (mock OAuth) | Simple (mock subprocess) |

---

## Potential Challenges & Solutions

### Challenge 1: CLI Not Installed

**Problem**: User doesn't have `claude` or `gemini` CLI installed

**Solution**:
```rust
fn check_cli_available(cli: &str) -> Result<PathBuf> {
    which::which(cli).map_err(|_| {
        format!(
            "{} CLI not found. Install it from:\n  \
            Claude: https://claude.ai/download\n  \
            Gemini: npm install -g @google/gemini-cli",
            cli
        )
    })
}
```

### Challenge 2: CLI Not Authenticated

**Problem**: CLI installed but user hasn't authenticated

**Solution**:
```rust
async fn verify_cli_auth(cli: &str) -> Result<()> {
    let test_output = execute_cli_command(
        cli,
        &["-p", "test", "--output-format", "text"]
    ).await;

    match test_output {
        Err(e) if e.contains("auth") || e.contains("login") => {
            Err(format!(
                "Please authenticate {} CLI first:\n  {}",
                cli,
                if cli == "claude" { "claude" } else { "gemini" }
            ))
        }
        _ => Ok(())
    }
}
```

### Challenge 3: CLI Output Format Changes

**Problem**: CLI updates may change output format

**Solution**:
- Use stable output formats (`--output-format json` for Claude)
- Version-check CLIs and warn about compatibility
- Maintain fallback parsers for different versions

### Challenge 4: Performance Overhead

**Problem**: Spawning subprocess has overhead vs direct API calls

**Solution**:
- CLI startup time negligible for spec-kit operations (batch processing)
- Can implement connection pooling if needed (keep CLI process alive)
- Trade-off acceptable for simplified implementation

---

## Migration Path

### Step 1: Implement CLI Routing (Week 1)
- Add CLI execution infrastructure
- Implement Claude and Gemini routing
- Keep ChatGPT OAuth as-is

### Step 2: Test with Spec-Kit Commands (Week 1)
- Validate `/speckit.plan`, `/speckit.tasks`, etc. work with all providers
- Performance testing
- Error handling refinement

### Step 3: Documentation (Week 1)
- User guide for CLI installation
- Authentication setup instructions
- Troubleshooting guide

### Step 4: Optional Future Optimization
- If performance becomes issue, implement direct API calls for ChatGPT
- For Claude/Gemini, CLI routing remains best option

---

## Decision Matrix

| Criterion | OAuth Hybrid | CLI Routing | Winner |
|-----------|-------------|-------------|---------|
| **Feasibility** | Blocked for Claude | ✅ Works for all | **CLI** |
| **Implementation Time** | 15-24 hours | 8-12 hours | **CLI** |
| **Code Complexity** | High | Low | **CLI** |
| **User Experience** | Multi-auth flows | One-time CLI auth | **CLI** |
| **Maintenance Burden** | High | Low | **CLI** |
| **Performance** | Fast (direct API) | Good (subprocess overhead minimal) | OAuth (slight edge) |
| **Reliability** | We manage tokens | CLI manages tokens | **CLI** |

**Overall Winner**: **CLI Routing** (6/7 criteria)

---

## Recommendation

**GO Decision**: Implement CLI Routing Approach

**Rationale**:
1. Only viable solution for Claude (bypasses OAuth blocker)
2. Simpler and faster to implement
3. Better user experience (authenticate once per CLI)
4. Lower maintenance burden
5. Leverages existing, battle-tested CLI tools

**Next Steps**:
1. Close SPEC-KIT-951 (research complete)
2. Create SPEC-KIT-952-cli-routing-implementation
3. Implement in 8-12 hours
4. Ship multi-provider support quickly

---

**Report Version**: 1.0 (CLI Routing)
**Last Updated**: 2025-11-19
**Status**: RECOMMENDED for implementation
