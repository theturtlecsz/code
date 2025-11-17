# Data Flow

What data goes where, local vs cloud processing, and PII handling.

---

## Overview

**Data flow** describes what information leaves your machine and where it goes.

**Key Destinations**:
1. **AI Providers** (OpenAI, Anthropic, Google, Azure)
2. **MCP Servers** (local-memory, git-status, custom)
3. **Local Filesystem** (evidence, config, history)

**PII Risk**: Code may contain sensitive data (credentials, customer data, proprietary algorithms)

**Control**: Sandbox modes and approval gates limit data exposure

---

## Data Sent to AI Providers

### What Gets Sent

**Every API Request** includes:
1. **User Prompt**: Your question or task description
2. **File Contents**: Code files you're asking about
3. **Context**: Recent conversation history
4. **System Prompt**: Instructions for AI behavior
5. **Metadata**: Model name, temperature, max tokens

**Example Request**:
```json
{
  "model": "gpt-5",
  "messages": [
    {
      "role": "system",
      "content": "You are a helpful coding assistant..."
    },
    {
      "role": "user",
      "content": "Explain this function:\n\nfn authenticate(password: &str) -> bool {\n    password == \"SECRET_PASSWORD\"\n}"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 2000
}
```

**Sent to**: OpenAI servers (api.openai.com)

---

### What Does NOT Get Sent

**Never Sent**:
- ❌ API keys (only used for authentication header)
- ❌ Environment variables (excluded by shell_environment_policy)
- ❌ Files outside workspace (sandbox restrictions)
- ❌ Your entire codebase (only files you explicitly mention)
- ❌ MCP server data (stays local unless explicitly sent)

**Controlled by**:
- Sandbox mode (`read-only`, `workspace-write`, `danger-full-access`)
- Approval policy (`untrusted`, `on-failure`, `on-request`, `never`)

---

### Multi-Agent Data Flow

**Spec-Kit Pipeline** (6 stages):

```
User Request
    ↓
Plan Stage (3 agents: gemini, claude, gpt5)
    → Send: PRD content, constitution
    → Receive: 3 work breakdown plans
    → Local: Consensus synthesis (MCP local-memory)
    ↓
Tasks Stage (1 agent: gpt5-low)
    → Send: Plan output
    → Receive: Task breakdown
    ↓
Implement Stage (2 agents: gpt_codex, claude-haiku)
    → Send: Plan, tasks, existing code files
    → Receive: Code implementation
    → Local: Validation (cargo fmt, clippy, build)
    ↓
Validate Stage (3 agents: gemini, claude, gpt5)
    → Send: Implementation, test requirements
    → Receive: Test strategy
    ↓
Audit Stage (3 agents: gemini-pro, claude-sonnet, gpt5-high)
    → Send: All code, dependencies
    → Receive: Security/compliance analysis
    ↓
Unlock Stage (3 agents: gemini-pro, claude-sonnet, gpt5-high)
    → Send: All artifacts, audit results
    → Receive: Ship/no-ship decision
```

**Total Data Sent**: ~50-200 KB per stage (depends on code size)

---

## Provider Data Policies

### OpenAI

**Data Retention** (as of 2024):
- **API Requests**: Stored for 30 days (for abuse detection)
- **Training**: NOT used for training by default
- **Deletion**: Can request deletion after 30 days

**Control**:
```toml
[model_providers.openai]
api_key = "$OPENAI_API_KEY"
# No additional controls for data retention
```

**Privacy Policy**: https://openai.com/policies/privacy-policy

**Zero Data Retention** (ChatGPT Enterprise):
- Available for enterprise customers
- No data stored, used for training, or logged
- Requires separate agreement

---

### Anthropic

**Data Retention** (as of 2024):
- **API Requests**: Not used for training
- **Logging**: Minimal logging for debugging
- **Deletion**: Can request deletion

**Privacy Policy**: https://www.anthropic.com/privacy

**Trust**: Generally considered privacy-focused provider

---

### Google (Gemini)

**Data Retention** (as of 2024):
- **API Requests**: May be used for abuse detection
- **Training**: NOT used for training (Gemini API)
- **Retention**: 18 months (deletable on request)

**Privacy Policy**: https://policies.google.com/privacy

**Control**:
```toml
[model_providers.google]
api_key = "$GOOGLE_API_KEY"
# No additional controls for data retention
```

---

### Azure OpenAI

**Data Retention** (as of 2024):
- **API Requests**: NOT stored (Azure commitment)
- **Training**: NOT used for training
- **Data Residency**: Stays in Azure region (EU, US, etc.)

**Benefits**:
- ✅ GDPR compliant (data residency)
- ✅ Zero data retention
- ✅ SOC 2 certified

**Configuration**:
```toml
[model_providers.azure]
api_key = "$AZURE_OPENAI_API_KEY"
endpoint = "https://my-resource.openai.azure.com/"
```

**Recommended**: For enterprise/GDPR-sensitive deployments

---

### Ollama (Local)

**Data Retention**: ZERO (runs entirely locally)

**Configuration**:
```toml
[model_providers.ollama]
base_url = "http://localhost:11434"
```

**Benefits**:
- ✅ No data leaves your machine
- ✅ No API costs
- ✅ No internet required
- ❌ Requires powerful hardware (GPU)
- ❌ Lower quality than cloud models

**Use Case**: Privacy-critical deployments

---

## Local Data Processing

### MCP Server Data

**local-memory** (knowledge persistence):
- **Storage**: `~/.code/mcp-memory/` (SQLite database)
- **Contents**: High-value knowledge (architecture decisions, patterns, bug fixes)
- **Never Sent**: To AI providers (unless explicitly included in prompt)
- **Encryption**: None (unencrypted on disk)

**git-status** (repository inspection):
- **Storage**: In-memory (not persisted)
- **Contents**: Git status, diffs, commit logs
- **Never Sent**: To AI providers (unless explicitly included)

**HAL** (policy validation):
- **Storage**: None (validation results ephemeral)
- **Contents**: Local-memory analysis, tag schema checks
- **Credentials Required**: `HAL_SECRET_KAVEDARR_API_KEY` (sent to Kavedarr API)

---

### Evidence Repository

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Contents**:
- Telemetry JSON (execution metadata)
- Agent outputs (AI responses)
- Consensus artifacts
- Quality gate results
- Guardrail logs

**Visibility**:
- ✅ Stored locally
- ❌ Not sent to AI providers
- ⚠️ Committed to git (may be pushed to GitHub)

**PII Risk**: May contain code snippets sent to AI providers

**Mitigation**: Use `.gitignore` to exclude evidence/ if sensitive

---

### Session History

**Location**: `~/.code/history.jsonl`

**Contents**:
- User prompts
- AI responses
- Command history
- Timestamps

**Format** (JSONL):
```json
{"timestamp":"2025-10-18T14:32:00Z","role":"user","content":"Explain this code..."}
{"timestamp":"2025-10-18T14:32:15Z","role":"assistant","content":"This function authenticates..."}
```

**PII Risk**: May contain sensitive prompts/code

**Mitigation**: Delete history file if sensitive
```bash
rm ~/.code/history.jsonl
```

---

## PII and Sensitive Data Handling

### What is PII?

**Personal Identifiable Information**:
- Customer names, emails, addresses
- Social Security numbers
- Credit card numbers
- Medical records
- Login credentials (username/password)

**Proprietary Information**:
- Trade secrets
- Proprietary algorithms
- Customer data
- Internal business logic

---

### PII Risk Scenarios

**HIGH RISK**:
```bash
# ❌ Asking about code with customer data
code "Explain this user authentication function" < user_table.sql
# Sends SQL table schema with customer emails to AI provider
```

**MEDIUM RISK**:
```bash
# ⚠️ Asking about business logic
code "Refactor pricing calculation" < pricing.rs
# Sends proprietary pricing algorithm to AI provider
```

**LOW RISK**:
```bash
# ✅ Generic code assistance
code "How do I read a CSV file in Rust?"
# No sensitive data sent
```

---

### PII Mitigation Strategies

#### 1. Sanitize Before Asking

**Redact Sensitive Data**:
```rust
// Before asking AI
fn authenticate(password: &str) -> bool {
    password == "SECRET_PASSWORD"  // ❌ Real secret
}

// Redact
fn authenticate(password: &str) -> bool {
    password == "REDACTED"  // ✅ Safe to send
}
```

---

#### 2. Use Approval Gates

**Configuration**:
```toml
approval_policy = "untrusted"  # Approve every operation
```

**Behavior**: Review prompt BEFORE sending to AI provider

**Example**:
```
─────────────────────────────────────────
Approve this operation?

Command: Read file
File: src/auth.rs
Action: Send file contents to OpenAI API

[View File] [Approve] [Deny]
─────────────────────────────────────────
```

**Opportunity**: Review for PII before approving

---

#### 3. Use Local Models (Ollama)

**Configuration**:
```toml
model_provider = "ollama"
model = "llama2"

[model_providers.ollama]
base_url = "http://localhost:11434"
```

**Benefit**: No data leaves your machine

**Trade-off**: Lower quality, requires GPU

---

#### 4. Limit File Access (Sandbox)

**Configuration**:
```toml
sandbox_mode = "read-only"  # No file writes
# or
sandbox_mode = "workspace-write"  # Only workspace access
```

**Behavior**: AI can only read/write files in workspace, not system-wide

**Benefit**: Limits data exposure if AI misbehaves

---

## Data Deletion

### Delete Session History

```bash
# Delete conversation history
rm ~/.code/history.jsonl

# Or truncate
> ~/.code/history.jsonl
```

---

### Delete Evidence

```bash
# Delete evidence for specific SPEC
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/

# Or delete all evidence
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
```

---

### Delete MCP Memory

```bash
# Delete local-memory database
rm -rf ~/.code/mcp-memory/

# Or delete specific memories (via MCP)
# Use mcp__local-memory__delete_memory (if available)
```

---

### Request Provider Deletion

**OpenAI**:
1. Contact support@openai.com
2. Request deletion of API requests after 30-day retention
3. Provide API key ID

**Anthropic**:
1. Contact privacy@anthropic.com
2. Request data deletion

**Google**:
1. Use Google Takeout (if personal account)
2. Contact support (if enterprise)

**Azure**:
- Data not retained (no deletion needed)

---

## Network Isolation

### Block All Network Access

**Configuration**:
```toml
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = false  # Block all network (default)
```

**Behavior**:
- AI cannot make HTTP requests
- AI cannot download files
- Prevents data exfiltration

---

### Allow Specific Hosts

**Future Enhancement** (not yet implemented):
```toml
[sandbox_workspace_write]
network_access = true
allowed_hosts = [
    "api.openai.com",
    "api.anthropic.com",
    "generativelanguage.googleapis.com"
]
```

**Status**: Currently all-or-nothing (allow all or block all)

---

## Data Flow Diagram

```
┌──────────────────────────────────────────────────────────┐
│                       User Machine                        │
│                                                           │
│  ┌─────────────┐      ┌─────────────┐                    │
│  │   User      │      │  Code       │                    │
│  │   Prompts   │──────▶  Assistant  │                    │
│  └─────────────┘      └─────────────┘                    │
│                            │  │  │                        │
│        ┌───────────────────┘  │  └────────────┐          │
│        │                      │               │          │
│        ▼                      ▼               ▼          │
│  ┌────────────┐       ┌─────────────┐  ┌─────────────┐  │
│  │  Session   │       │   MCP       │  │  Evidence   │  │
│  │  History   │       │   Servers   │  │  Repository │  │
│  │ (local)    │       │  (local)    │  │  (local)    │  │
│  └────────────┘       └─────────────┘  └─────────────┘  │
│                                                           │
│        Network Boundary (sandbox controls)               │
└────────────────┬──────────────────┬──────────────────────┘
                 │                  │
                 ▼                  ▼
┌─────────────────────────┐  ┌──────────────────────┐
│   AI Providers          │  │   MCP External       │
│                         │  │   Servers            │
│  • OpenAI API           │  │                      │
│  • Anthropic API        │  │  • HAL (Kavedarr)    │
│  • Google Gemini API    │  │  • Custom APIs       │
│  • Azure OpenAI         │  │                      │
│                         │  │                      │
│  Data Sent:             │  │  Data Sent:          │
│  - User prompts         │  │  - MCP requests      │
│  - File contents        │  │  - Credentials       │
│  - Context              │  │                      │
└─────────────────────────┘  └──────────────────────┘
```

---

## Compliance Implications

### GDPR (EU)

**Requirements**:
- Right to erasure (delete all user data)
- Data minimization (only collect necessary data)
- Data residency (EU customer data stays in EU)

**Compliance Strategy**:
- ✅ Use Azure OpenAI (EU region) for data residency
- ✅ Enable approval gates to review prompts
- ✅ Regular data deletion (history, evidence)
- ⚠️ Provider data retention (request deletion after 30 days)

---

### SOC 2 (US)

**Requirements**:
- Access controls (who can use AI features)
- Audit trail (log all AI interactions)
- Data encryption (in transit and at rest)

**Compliance Strategy**:
- ✅ Evidence repository (complete audit trail)
- ✅ HTTPS for API requests (encryption in transit)
- ⚠️ No encryption at rest (local files unencrypted)
- ⚠️ No access controls (single-user tool)

**Recommendation**: Use Azure OpenAI for SOC 2 compliance

---

## Summary

**Data Flow** highlights:

1. **Sent to AI Providers**: User prompts, file contents, conversation history
2. **NOT Sent**: API keys, environment variables, entire codebase, MCP data
3. **Provider Policies**: 30-day retention (OpenAI), no training (Anthropic), GDPR-compliant (Azure)
4. **Local Processing**: MCP servers, evidence repository, session history (all local)
5. **PII Risk**: Code may contain sensitive data (customer info, proprietary algorithms)
6. **Mitigation**: Sanitize data, approval gates, local models (Ollama), sandbox restrictions
7. **Data Deletion**: Delete history, evidence, MCP memory, request provider deletion
8. **Network Isolation**: Block network access in sandbox mode

**Best Practices**:
- ⚠️ Review prompts before sending (approval gates)
- ✅ Sanitize PII before asking AI
- ✅ Use Azure OpenAI for GDPR/SOC 2 compliance
- ✅ Use Ollama for complete privacy (local models)
- ✅ Delete history/evidence periodically
- ✅ Block network access in sandbox mode

**Next**: [MCP Security](mcp-security.md)
