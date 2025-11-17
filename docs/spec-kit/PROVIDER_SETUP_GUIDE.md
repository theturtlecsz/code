# Provider Setup Guide

**SPEC**: SPEC-949 (Extended Model Support)
**Status**: ✅ Production Ready (2025-11-16)
**Audience**: Developers adding provider diversity to spec-kit framework

---

## Overview

The codex-rs spec-kit framework supports multiple AI providers through a unified `ProviderConfig` trait. This guide covers:

1. **GPT-5 Setup** (already active, validation steps)
2. **Deepseek Activation** (provider stub → production)
3. **Kimi Activation** (provider stub → production)
4. **Custom Provider Addition** (extend framework)
5. **Troubleshooting** (OpenAI-compatible API issues)

**Prerequisites**:
- Rust development environment
- Access to API keys for desired providers
- Familiarity with spec-kit architecture

---

## Section 1: GPT-5 Setup (Already Active)

### Status

✅ **Active by default** as of SPEC-949 Phase 2 (commit 43cbd35da)

**Models Available**:
- gpt-5 (flagship reasoning)
- gpt-5.1 (adaptive reasoning with 24h cache)
- gpt-5-codex (agentic software engineering)
- gpt-5.1-codex (enhanced agentic + tool use)
- gpt-5.1-codex-mini (cost-optimized)

**Provider**: OpenAI (uses existing `OpenAIProvider` in `core/src/async_agent_executor.rs`)

### Validation Steps

**1. Verify Model Recognition**:
```bash
# Build core library
cargo build -p codex-core

# Run model recognition tests
cargo test -p codex-core openai_model_info::tests::test_gpt5

# Expected output:
# test openai_model_info::tests::test_gpt5_base_model ... ok
# test openai_model_info::tests::test_gpt5_1_model ... ok
# test openai_model_info::tests::test_gpt5_codex_model ... ok
# test openai_model_info::tests::test_gpt5_1_codex_model ... ok
# test openai_model_info::tests::test_gpt5_1_codex_mini_model ... ok
```

**2. Verify Agent Routing**:
```bash
# Check agent configurations
grep -A 1 "gpt5" tui/src/chatwidget/spec_kit/subagent_defaults.rs

# Expected: 7 references to gpt5_* agents across stages
```

**3. Test API Access**:
```bash
# Set API key
export OPENAI_API_KEY="sk-..."

# Test GPT-5 model access
curl https://api.openai.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $OPENAI_API_KEY" \
  -d '{
    "model": "gpt-5",
    "messages": [{"role": "user", "content": "Test"}],
    "max_tokens": 10
  }'

# Expected: 200 OK with completion response
# If 404: GPT-5 not available for your account yet
```

**No configuration changes needed** - GPT-5 is active by default.

---

## Section 2: Deepseek Activation

### Overview

**Status**: 🟡 Stub implementation (SPEC-949 Phase 3)
**Provider**: Deepseek AI
**API Compatibility**: OpenAI-compatible (drop-in replacement)
**Base URL**: https://api.deepseek.com/v1

**Models** (when activated):
- `deepseek-chat` (V3, 64K context)
- `deepseek-v3.1` (enhanced reasoning)
- `deepseek-reasoner` (R1, chain-of-thought specialist)

**Cost Advantage**: ~10-20× cheaper than GPT-5 for comparable quality
**Use Case**: High-volume operations, cost-sensitive workflows

### Prerequisites

1. **Obtain Deepseek API key**:
   - Sign up at https://platform.deepseek.com
   - Navigate to API Keys section
   - Generate new key (format: `sk-...`)
   - Store securely (`.env` file or secret manager)

2. **Verify Deepseek stub exists**:
   ```bash
   grep -n "DeepseekProvider" core/src/async_agent_executor.rs
   # Expected: Struct definition at ~line 440, impl at ~450
   ```

### Activation Steps

**Step 1: Uncomment Provider Registration**

**File**: `core/src/async_agent_executor.rs`
**Location**: ~lines 665-666 (in `ProviderRegistry::with_defaults()`)

**Before**:
```rust
impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(Box::new(AnthropicProvider));
        registry.register(Box::new(GoogleProvider));
        registry.register(Box::new(OpenAIProvider));

        // Future providers (SPEC-949 - uncomment when API keys available)
        // registry.register(Box::new(DeepseekProvider));  // <-- COMMENTED OUT
        // registry.register(Box::new(KimiProvider));

        registry
    }
}
```

**After**:
```rust
impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(Box::new(AnthropicProvider));
        registry.register(Box::new(GoogleProvider));
        registry.register(Box::new(OpenAIProvider));

        // Deepseek provider (SPEC-949 Phase 3, activated 2025-XX-XX)
        registry.register(Box::new(DeepseekProvider));  // <-- ACTIVATED
        // registry.register(Box::new(KimiProvider));    // <-- Keep commented for now

        registry
    }
}
```

**Step 2: Remove Dead Code Attribute**

**File**: `core/src/async_agent_executor.rs`
**Location**: ~line 440

**Before**:
```rust
#[allow(dead_code)]  // <-- REMOVE THIS LINE
pub struct DeepseekProvider;
```

**After**:
```rust
pub struct DeepseekProvider;
```

**Step 3: Set Environment Variable**

**Add to `.env` file** (create if doesn't exist):
```bash
# Deepseek API Configuration (SPEC-949)
DEEPSEEK_API_KEY=sk-your-deepseek-api-key-here
```

**Or export in shell**:
```bash
export DEEPSEEK_API_KEY="sk-..."
```

**Step 4: Rebuild**

```bash
# Rebuild core library
cargo build -p codex-core

# Expected: No warnings (dead_code attribute removed)

# Rebuild TUI (uses codex-core)
cargo build -p codex-tui

# Expected: Clean build, no errors
```

**Step 5: Verify Provider Registration**

```bash
# Test provider detection (add debug logging if needed)
cargo test -p codex-core async_agent_executor::tests --nocapture

# Or manual verification:
# The ProviderRegistry::with_defaults() should now include DeepseekProvider
```

**Step 6: Test Deepseek API Access**

```bash
# Test basic access
curl https://api.deepseek.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
  -d '{
    "model": "deepseek-chat",
    "messages": [{"role": "user", "content": "Test"}],
    "max_tokens": 10
  }'

# Expected: 200 OK with completion response
```

### Agent Configuration (Optional)

**To use Deepseek models in spec-kit stages**:

**Option 1: Per-User Config Override** (recommended)

Create `~/.code/config.toml` (or wherever your config lives):
```toml
# Deepseek agents (cost-optimized alternatives)
[agents.deepseek_chat]
command = "chatgpt"  # Uses OpenAI-compatible API
model = "deepseek-chat"
temperature = 0.7
args = ["--base-url", "https://api.deepseek.com/v1", "--model", "deepseek-chat"]
env = { DEEPSEEK_API_KEY = "${DEEPSEEK_API_KEY}" }

[agents.deepseek_reasoner]
command = "chatgpt"
model = "deepseek-reasoner"
temperature = 0.5
args = ["--base-url", "https://api.deepseek.com/v1", "--model", "deepseek-reasoner"]
```

**Option 2: Update Default Routing** (system-wide)

**File**: `tui/src/chatwidget/spec_kit/subagent_defaults.rs`

**Example** (replace gpt5_1_mini with deepseek_chat for cost savings):
```rust
SpecKitCommand::Specify => SubagentCommand {
    agents: &["deepseek_chat"],  // Was: &["gpt5_1_mini"]
    prompt: "Use Deepseek Chat (cost-optimized) for specification elaboration...",
},
```

**Rebuild after changes**:
```bash
cargo build -p codex-tui
```

### Validation

**Run test SPEC with Deepseek agent**:
```bash
code  # Launch TUI

# In TUI:
/speckit.specify SPEC-900  # Or any test SPEC

# Verify Deepseek used:
cat ../docs/SPEC-900/evidence/specify/consensus_*.json | jq '.agent'
# Expected: "deepseek_chat" (if configured)
```

**Cost comparison**:
- GPT-5.1-mini: ~$0.08 per specify stage
- Deepseek Chat: ~$0.004-0.01 per specify stage (10-20× cheaper)

---

## Section 3: Kimi Activation

### Overview

**Status**: 🟡 Stub implementation (SPEC-949 Phase 3)
**Provider**: Moonshot AI (Kimi)
**API Compatibility**: OpenAI-compatible
**Base URL**: https://platform.moonshot.ai/v1

**Models** (when activated):
- `kimi-k2` (256K context, fast inference)
- `kimi-k2-thinking` (chain-of-thought reasoning)

**Cost Advantage**: 5-15× cheaper than GPT-5, optimized for long-context
**Use Case**: Large codebase analysis, documentation generation

### Prerequisites

1. **Obtain Moonshot API key**:
   - Sign up at https://platform.moonshot.ai
   - Generate API key (format: varies)
   - Store as `MOONSHOT_API_KEY`

2. **Verify Kimi stub exists**:
   ```bash
   grep -n "KimiProvider" core/src/async_agent_executor.rs
   # Expected: Struct definition at ~line 521, impl at ~530
   ```

### Activation Steps

**Follow same procedure as Deepseek** (Section 2), with adjustments:

**Step 1: Uncomment Provider Registration**

**File**: `core/src/async_agent_executor.rs` (~line 666)

**Change**:
```rust
// registry.register(Box::new(KimiProvider));  // <-- COMMENTED
```

**To**:
```rust
registry.register(Box::new(KimiProvider));  // <-- ACTIVATED
```

**Step 2: Remove Dead Code Attribute**

**File**: `core/src/async_agent_executor.rs` (~line 521)

**Remove**: `#[allow(dead_code)]`

**Step 3: Set Environment Variable**

```bash
# Add to .env
MOONSHOT_API_KEY=your-moonshot-api-key-here
```

**Step 4-6: Rebuild, Test**

```bash
cargo build -p codex-core
cargo build -p codex-tui

# Test API access
curl https://platform.moonshot.ai/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $MOONSHOT_API_KEY" \
  -d '{
    "model": "kimi-k2",
    "messages": [{"role": "user", "content": "Test"}],
    "max_tokens": 10
  }'
```

### Agent Configuration

**Add Kimi agents to config.toml**:
```toml
[agents.kimi_k2]
command = "chatgpt"
model = "kimi-k2"
temperature = 0.7
args = ["--base-url", "https://platform.moonshot.ai/v1", "--model", "kimi-k2"]
env = { MOONSHOT_API_KEY = "${MOONSHOT_API_KEY}" }

[agents.kimi_k2_thinking]
command = "chatgpt"
model = "kimi-k2-thinking"
temperature = 0.5
args = ["--base-url", "https://platform.moonshot.ai/v1", "--model", "kimi-k2-thinking"]
```

**Use for**: Long-context operations (plan stage with large codebase references)

---

## Section 4: Troubleshooting OpenAI-Compatible APIs

### Common Issues

#### Issue 1: Authentication Failed

**Symptoms**:
```
Error: authentication_failed
Error: invalid_api_key
```

**Diagnosis**:
```bash
# Check if API key is set
echo $DEEPSEEK_API_KEY
# or
echo $MOONSHOT_API_KEY

# Verify key format (usually starts with sk- or similar)
```

**Solutions**:
1. **Set environment variable correctly**:
   ```bash
   export DEEPSEEK_API_KEY="sk-..."  # No spaces around =
   source .env  # If using .env file
   ```
2. **Verify key is valid** (test with curl, see above)
3. **Check provider dashboard** for key status (active? expired?)

#### Issue 2: Base URL Incorrect

**Symptoms**:
```
Error: Connection refused
Error: timeout
```

**Diagnosis**:
```bash
# Test base URL reachability
curl -I https://api.deepseek.com/v1/chat/completions
# Expected: 401 Unauthorized (auth required, but URL works)

# Not expected: Connection refused, timeout
```

**Solutions**:
1. **Verify base URL** in provider stub:
   ```rust
   // DeepseekProvider::format_small_prompt_args()
   "--base-url".to_string(),
   "https://api.deepseek.com/v1".to_string(),  // <-- CHECK THIS
   ```
2. **Check provider documentation** for API endpoint changes
3. **Test with curl** (see activation steps above)

#### Issue 3: Model Not Found

**Symptoms**:
```
Error: model 'deepseek-chat' not found
```

**Diagnosis**:
- Model name typo
- Model deprecated/renamed by provider
- Account doesn't have access to model

**Solutions**:
1. **List available models**:
   ```bash
   curl https://api.deepseek.com/v1/models \
     -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
     | jq '.data[] | .id'
   ```
2. **Update model name** in agent config or provider stub
3. **Contact provider support** if model should be available

#### Issue 4: Rate Limits

**Symptoms**:
```
Error: 429 Too Many Requests
Error: rate_limit_exceeded
```

**Solutions**:
1. **Check rate limits** in provider dashboard
2. **Add retry logic** (already implemented in spec-kit, but may need tuning)
3. **Reduce concurrent requests** (limit parallel /speckit.* commands)
4. **Upgrade account tier** (if provider offers higher limits)

#### Issue 5: Response Format Incompatible

**Symptoms**:
- Parsing errors
- Unexpected JSON structure
- Missing fields in response

**Diagnosis**:
```bash
# Test raw API response
curl https://api.deepseek.com/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $DEEPSEEK_API_KEY" \
  -d '{
    "model": "deepseek-chat",
    "messages": [{"role": "user", "content": "Test"}],
    "max_tokens": 10
  }' | jq '.'

# Compare to OpenAI response structure
# Expect: {choices: [{message: {content: "..."}}], usage: {...}}
```

**Solutions**:
1. **Verify OpenAI compatibility**: Provider should match OpenAI's response schema
2. **Add response transformation** in provider stub (if needed):
   ```rust
   impl ProviderConfig for DeepseekProvider {
       // Add method to transform response if schema differs
       fn transform_response(&self, raw: &str) -> Result<String, String> {
           // Custom transformation logic
       }
   }
   ```
3. **Report to provider** if claiming OpenAI compatibility but schema differs

---

## Section 5: Adding Custom Providers

### Prerequisites

- Familiarity with Rust
- Provider has OpenAI-compatible API (or you're willing to write adapter)
- API documentation available

### Steps to Add New Provider

**Step 1: Create Provider Struct**

**File**: `core/src/async_agent_executor.rs` (add after existing providers)

```rust
/// MyCustomProvider configuration
///
/// Status: Custom provider added <DATE>
/// API Compatibility: OpenAI-compatible
/// Base URL: https://api.mycustomprovider.com/v1
/// Models: model-a, model-b
pub struct MyCustomProvider;

impl MyCustomProvider {
    pub fn new() -> Self {
        Self
    }
}
```

**Step 2: Implement ProviderConfig Trait**

```rust
impl ProviderConfig for MyCustomProvider {
    fn name(&self) -> &str {
        "mycustom"  // Identifier for this provider
    }

    fn required_env_vars(&self) -> Vec<&str> {
        vec!["MYCUSTOM_API_KEY"]  // Environment variables required
    }

    fn detect_oauth2_error(&self, stderr: &str) -> bool {
        // Detect authentication errors in stderr
        stderr.contains("invalid_api_key")
            || stderr.contains("authentication_failed")
            || stderr.contains("unauthorized")
    }

    fn format_small_prompt_args(&self, prompt: &str) -> Vec<String> {
        // CLI args for small prompts (passed inline)
        vec![
            "--base-url".to_string(),
            "https://api.mycustomprovider.com/v1".to_string(),
            "--model".to_string(),
            "model-a".to_string(),  // Default model
            "-p".to_string(),
            prompt.to_string(),
        ]
    }

    fn format_large_prompt_args(&self) -> Vec<String> {
        // CLI args for large prompts (passed via stdin)
        vec![
            "--base-url".to_string(),
            "https://api.mycustomprovider.com/v1".to_string(),
            "--model".to_string(),
            "model-a".to_string(),
            // Prompt sent via stdin, not in args
        ]
    }
}
```

**Step 3: Register Provider**

**File**: `core/src/async_agent_executor.rs` (~line 660)

```rust
impl ProviderRegistry {
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        registry.register(Box::new(AnthropicProvider));
        registry.register(Box::new(GoogleProvider));
        registry.register(Box::new(OpenAIProvider));
        registry.register(Box::new(MyCustomProvider));  // <-- ADD HERE

        registry
    }
}
```

**Step 4: Add Agent Configuration**

**Create agent config** in `~/.code/config.toml`:

```toml
[agents.mycustom_model_a]
command = "chatgpt"  # Or custom binary if needed
model = "model-a"
temperature = 0.7
args = ["--base-url", "https://api.mycustomprovider.com/v1", "--model", "model-a"]
env = { MYCUSTOM_API_KEY = "${MYCUSTOM_API_KEY}" }
```

**Step 5: Test**

```bash
# Set API key
export MYCUSTOM_API_KEY="..."

# Rebuild
cargo build -p codex-core
cargo build -p codex-tui

# Test with spec-kit
code

# In TUI, use custom agent (if configured in subagent_defaults.rs or via override)
/speckit.specify SPEC-900
```

### Advanced: Non-OpenAI-Compatible Providers

If provider doesn't match OpenAI's API schema:

**Option 1: Write Adapter Binary**

Create a wrapper binary (`mycustom-adapter`) that:
1. Accepts OpenAI-compatible input (CLI args, stdin)
2. Transforms to provider's format
3. Calls provider API
4. Transforms response back to OpenAI format
5. Outputs to stdout

**Option 2: Extend ProviderConfig Trait**

Add methods to trait definition:
```rust
pub trait ProviderConfig: Send + Sync {
    // ... existing methods ...

    // Optional: Custom request transformation
    fn transform_request(&self, openai_format: &str) -> String {
        openai_format.to_string()  // Default: no transformation
    }

    // Optional: Custom response transformation
    fn transform_response(&self, provider_format: &str) -> String {
        provider_format.to_string()  // Default: no transformation
    }
}
```

---

## Best Practices

### Provider Selection Strategy

**Use GPT-5 for**:
- High-stakes decisions (audit, unlock)
- Complex reasoning
- Reliability-critical workflows

**Use Deepseek for**:
- High-volume operations
- Cost-sensitive workflows
- Acceptable 10-20% quality tradeoff for massive cost savings

**Use Kimi for**:
- Long-context operations (>100K tokens)
- Documentation generation
- Codebase analysis

### Security

1. **Never commit API keys** to version control
2. **Use environment variables** or secret managers
3. **Rotate keys regularly** (quarterly recommended)
4. **Monitor usage** for anomalies (provider dashboards)
5. **Restrict key permissions** (read-only if possible)

### Cost Optimization

1. **Mix providers** by task complexity:
   - Simple: Deepseek/Kimi
   - Medium: GPT-5.1-mini
   - Complex: GPT-5-codex
2. **Monitor spend per provider** (telemetry)
3. **Set budget alerts** (provider dashboards)
4. **Use caching** (24h for GPT-5, varies by provider)

### Reliability

1. **Test provider stability** before production use
2. **Have fallback providers** configured
3. **Monitor error rates** (provider-specific)
4. **Document provider SLAs** and maintenance windows

---

## Provider Comparison Matrix

| Provider | Cost Factor | Context | Strengths | Weaknesses | Best For |
|----------|-------------|---------|-----------|------------|----------|
| **OpenAI GPT-5** | Baseline | 272K | Quality, reliability, caching | Expensive | High-stakes, production |
| **Deepseek** | 0.05-0.10× | 64K | Ultra-cheap, OpenAI-compatible | Lower quality, smaller context | High-volume, cost-sensitive |
| **Kimi** | 0.10-0.20× | 256K | Long-context, fast | Less tested, regional | Codebase analysis, docs |
| **Anthropic Claude** | 0.80-1.20× | 200K | Safety, instruction-following | Expensive, not OpenAI-compatible | Validation, review |
| **Google Gemini** | 0.20-0.40× | 1M | Huge context, multimodal | Slower, less reliable | Research, exploration |

---

## FAQ

**Q: Can I use multiple providers simultaneously?**
A: Yes. Configure agents for each provider, use in multi-agent consensus (e.g., [deepseek, gpt5, claude]).

**Q: Which provider should I activate first?**
A: Start with Deepseek (easiest activation, biggest cost savings). Then Kimi if you need long-context.

**Q: What if a provider's API changes?**
A: Update provider stub (`format_*_args` methods) and rebuild. Pin API version if provider supports it.

**Q: Can I disable a provider after activation?**
A: Yes. Comment out registration in `ProviderRegistry::with_defaults()` and rebuild. Or remove agent configs.

**Q: How do I choose which agent to use for each stage?**
A: Modify `subagent_defaults.rs` or create per-user config overrides. See PIPELINE_CONFIGURATION_GUIDE.md for advanced routing.

---

## Next Steps

1. ✅ **Activate Deepseek** (Section 2) for cost savings
2. ✅ **Test with small SPEC** (validation)
3. 📊 **Measure cost reduction** (compare to GPT-5 baseline)
4. 🧪 **Experiment with Kimi** (Section 3) for long-context tasks
5. 🔧 **Add custom provider** (Section 5) if needed
6. 📘 **Read GPT5_MIGRATION_GUIDE.md** for GPT-5 best practices
7. 📘 **Read PIPELINE_CONFIGURATION_GUIDE.md** for advanced agent routing

---

## Support & Resources

**Issues**: https://github.com/theturtlecsz/code/issues
**SPEC**: docs/SPEC-949-extended-model-support/spec.md
**Implementation**: docs/SPEC-949-extended-model-support/implementation-plan.md

**Provider Documentation**:
- OpenAI: https://platform.openai.com/docs
- Deepseek: https://platform.deepseek.com/docs
- Kimi/Moonshot: https://platform.moonshot.ai/docs

**Related SPECs**:
- SPEC-948: Modular Pipeline Logic (per-stage provider selection)
- SPEC-947: Pipeline UI Configurator (visual agent configuration)

---

**Document Version**: 1.0.0
**Last Updated**: 2025-11-16
**Status**: ✅ Production Ready
