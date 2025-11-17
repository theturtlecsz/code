# Threat Model

Attack vectors, risk assessment, and mitigation strategies.

---

## Overview

This document analyzes security threats for the **codex CLI**, an AI-powered coding assistant that:
- Executes AI-generated code in a sandboxed environment
- Sends code/context to external AI providers (OpenAI, Anthropic, Google)
- Accesses local filesystem and git repositories
- Integrates with external tools via MCP (Model Context Protocol)

**Threat Model Scope**: Codex CLI running on developer workstation

---

## Attack Surfaces

### 1. AI Provider Communication

**Attack Surface**: Network communication with AI providers (OpenAI, Anthropic, Google)

**Threat Actors**:
- Malicious AI provider (compromised or rogue)
- Man-in-the-middle attacker
- Network eavesdropper

**Attack Vectors**:
1. **Prompt Injection** - Attacker injects malicious instructions via code comments, filenames, or git commit messages
2. **Data Exfiltration** - AI provider logs/stores sensitive code or credentials
3. **Man-in-the-Middle** - Attacker intercepts API communication
4. **Provider Account Compromise** - Stolen API keys used to access AI services

---

### 2. Local Code Execution

**Attack Surface**: Execution of AI-generated code in sandbox

**Threat Actors**:
- Malicious AI model (compromised, adversarial, or buggy)
- Local attacker with code injection capability

**Attack Vectors**:
1. **Sandbox Escape** - AI-generated code breaks out of sandbox to access unauthorized files/network
2. **Data Destruction** - AI deletes/corrupts files outside sandbox restrictions
3. **Command Injection** - AI-generated shell commands exploit vulnerabilities
4. **Privilege Escalation** - AI-generated code gains unauthorized permissions

---

### 3. Filesystem Access

**Attack Surface**: Local filesystem read/write operations

**Threat Actors**:
- Malicious AI model
- Local attacker

**Attack Vectors**:
1. **Credential Theft** - AI reads `.env`, `~/.aws/credentials`, `~/.ssh/id_rsa`
2. **Source Code Exfiltration** - AI sends proprietary code to attacker-controlled server
3. **Malicious File Writes** - AI writes backdoors, malware, or corrupted files
4. **Symlink Attacks** - AI exploits symlinks to access files outside allowed paths

---

### 4. MCP Server Integration

**Attack Surface**: External MCP servers (local-memory, git-status, custom servers)

**Threat Actors**:
- Malicious MCP server author
- Compromised MCP server (supply chain)
- Local attacker

**Attack Vectors**:
1. **Malicious MCP Server** - Attacker-controlled server exfiltrates data or executes malicious code
2. **MCP Server Compromise** - Legitimate server hijacked via dependency vulnerability
3. **Tool Abuse** - AI misuses legitimate MCP tools to access unauthorized data
4. **Data Leakage** - MCP server logs sensitive information

---

### 5. Configuration and Secrets

**Attack Surface**: Configuration files, API keys, auth tokens

**Threat Actors**:
- Local attacker
- Accidental exposure (git commit)

**Attack Vectors**:
1. **API Key Theft** - Attacker steals `~/.code/config.toml` or environment variables
2. **Config File Manipulation** - Attacker modifies config to execute malicious code
3. **Secrets in Git** - API keys accidentally committed to public/private repositories
4. **Plaintext Storage** - Secrets stored unencrypted on disk

---

## Risk Assessment

### Risk Matrix

| Threat | Likelihood | Impact | Overall Risk | Mitigation Priority |
|--------|-----------|--------|--------------|-------------------|
| Prompt Injection | High | Medium | **High** | P0 (Critical) |
| Sandbox Escape | Medium | Critical | **High** | P0 (Critical) |
| API Key Theft | Medium | High | **High** | P1 (High) |
| Data Exfiltration (to AI provider) | High | Medium | **High** | P1 (High) |
| Malicious MCP Server | Low | Critical | **Medium** | P2 (Medium) |
| Config File Manipulation | Low | High | **Medium** | P2 (Medium) |
| Credential Theft (filesystem) | Medium | High | **High** | P1 (High) |
| Man-in-the-Middle | Low | Medium | **Low** | P3 (Low) |

---

### Risk Definitions

**Likelihood**:
- Low: Unlikely without specific attacker targeting
- Medium: Plausible in common scenarios
- High: Likely to occur in normal usage

**Impact**:
- Low: Limited damage, easily reversible
- Medium: Significant damage, difficult to reverse
- High: Major damage, expensive to fix
- Critical: Complete compromise, irreversible harm

---

## Mitigations

### M1: Prompt Injection Defense

**Risk**: Prompt injection via code comments, filenames, git messages

**Mitigation**:
1. **Input Sanitization** - Strip/escape special characters in file paths, commit messages
2. **Context Isolation** - Separate system instructions from user code in AI prompts
3. **Output Validation** - Validate AI responses for suspicious patterns (URLs, shell commands)
4. **User Awareness** - Warn users to review AI-generated code before execution

**Status**: Partially implemented (user review required for all commands)

---

### M2: Sandbox Isolation

**Risk**: Sandbox escape leading to unauthorized file/network access

**Mitigation**:
1. **OS-Level Sandboxing** - Use macOS Sandbox API, Linux landlock/seccomp
2. **Filesystem Restrictions** - Whitelist writable paths, blacklist sensitive files (`.git/`, `~/.ssh/`)
3. **Network Isolation** - Block network by default, require explicit approval
4. **Git Write Protection** - Protect `.git/` folder in workspace-write mode

**Status**: Implemented (3 sandbox levels: read-only, workspace-write, full-access)

**Configuration**:
```toml
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = false  # Block network
allow_git_writes = false  # Protect .git/ folder
```

---

### M3: API Key Protection

**Risk**: API key theft or accidental exposure

**Mitigation**:
1. **Environment Variables** - Store API keys in env vars, not config files
2. **File Permissions** - Set `config.toml` to `0600` (owner read/write only)
3. **Git Ignore** - Add `.env`, `config.toml` to `.gitignore`
4. **Key Rotation** - Regularly rotate API keys (90-day max)
5. **Pre-Commit Hooks** - Block commits containing API key patterns

**Status**: Partially implemented (env var support, file permissions)

**Best Practice**:
```bash
# Store API keys in environment variables
export OPENAI_API_KEY="sk-proj-..."

# NEVER in config.toml:
# api_key = "sk-proj-..."  # ❌ BAD
```

---

### M4: Data Minimization

**Risk**: Sensitive data sent to AI providers

**Mitigation**:
1. **Local Processing** - Use local models (Ollama) for sensitive code
2. **Context Filtering** - Strip credentials, API keys, PII before sending to AI
3. **Zero Data Retention** - Enable ZDR mode for OpenAI accounts
4. **Selective Context** - Only send relevant files, not entire codebase

**Status**: Partially implemented (ZDR mode support, user controls context selection)

**Configuration**:
```toml
disable_response_storage = true  # ZDR mode (zero data retention)
```

---

### M5: MCP Server Vetting

**Risk**: Malicious or compromised MCP servers

**Mitigation**:
1. **Source Verification** - Only install MCP servers from trusted sources (npm official, GitHub verified)
2. **Code Review** - Review MCP server source code before installation
3. **Sandboxing** - Run MCP servers in isolated processes with limited permissions
4. **Permission System** - Require explicit approval for MCP tool calls (future)

**Status**: Partially implemented (process isolation)

**Best Practice**:
```toml
# Only use official MCP servers
[mcp_servers.local-memory]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-memory"]  # Official npm package

# Avoid untrusted servers:
# [mcp_servers.random]
# command = "/tmp/untrusted-script.sh"  # ❌ BAD
```

---

### M6: Least Privilege

**Risk**: Excessive permissions leading to unauthorized access

**Mitigation**:
1. **Read-Only Default** - Start with `sandbox_mode = "read-only"`
2. **Approval Gates** - Require approval for write/network operations
3. **Per-Command Permissions** - Grant permissions per-command, not globally
4. **Workspace Isolation** - Restrict writes to project directory only

**Status**: Implemented (3-tier approval system)

**Configuration**:
```toml
sandbox_mode = "read-only"  # Most restrictive
approval_policy = "on-request"  # Require approval for writes
```

---

### M7: Audit Logging

**Risk**: Undetected security incidents

**Mitigation**:
1. **Evidence Collection** - Log all AI-generated commands, file operations
2. **Telemetry** - Track quality gate decisions, consensus outcomes
3. **Session History** - Store command history in `~/.code/history.jsonl`
4. **Tamper Protection** - Write-once evidence files

**Status**: Implemented (evidence repository, telemetry, history)

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

---

### M8: Secure Defaults

**Risk**: Insecure out-of-the-box configuration

**Mitigation**:
1. **Read-Only Default** - `sandbox_mode = "read-only"` by default
2. **Approval Required** - `approval_policy = "on-request"` by default
3. **Network Blocked** - `network_access = false` by default
4. **Git Protected** - `allow_git_writes = false` by default

**Status**: Implemented (secure defaults)

---

## Residual Risks

After applying all mitigations, the following **residual risks** remain:

### R1: AI Model Capability

**Risk**: AI models become capable enough to:
- Craft sophisticated sandbox escape exploits
- Social engineer users into approving malicious operations
- Hide malicious code in legitimate-looking changes

**Mitigation**: None (inherent risk of AI coding assistants)

**Acceptance Criteria**: Users must review all AI-generated code

---

### R2: Zero-Day Sandbox Escape

**Risk**: Unknown OS-level sandbox vulnerabilities

**Mitigation**: Limited (rely on OS vendor patches)

**Acceptance Criteria**: Monitor OS security advisories, apply patches promptly

---

### R3: Supply Chain Compromise

**Risk**: Compromised dependencies (npm packages, Rust crates)

**Mitigation**: Limited (rely on ecosystem security practices)

**Acceptance Criteria**: Pin dependencies, review changes on updates

---

### R4: Insider Threat (AI Provider)

**Risk**: AI provider employees access customer code/data

**Mitigation**: Limited (contractual data privacy agreements)

**Acceptance Criteria**: Use local models (Ollama) for highly sensitive code

---

## Threat Scenarios

### Scenario 1: Malicious AI Model

**Trigger**: Compromised AI model generates malicious code

**Attack Flow**:
```
1. User requests "refactor authentication code"
2. Compromised AI generates code with backdoor
3. User reviews code (may miss subtle backdoor)
4. User approves execution
5. Backdoor deployed to production
```

**Mitigations**:
- M1 (Prompt Injection Defense)
- M2 (Sandbox Isolation) - prevents backdoor from exfiltrating data
- M7 (Audit Logging) - evidence for post-incident forensics

**Residual Risk**: R1 (AI Model Capability) - users may miss subtle backdoors

---

### Scenario 2: API Key Theft

**Trigger**: Attacker gains access to developer workstation

**Attack Flow**:
```
1. Attacker compromises workstation via phishing/malware
2. Attacker reads ~/.code/config.toml or environment variables
3. Attacker steals OPENAI_API_KEY
4. Attacker uses stolen key for unauthorized AI access
```

**Mitigations**:
- M3 (API Key Protection) - env vars, file permissions
- M6 (Least Privilege) - limit blast radius

**Residual Risk**: None (workstation compromise is out of scope)

---

### Scenario 3: Sandbox Escape

**Trigger**: AI generates code that exploits OS sandbox vulnerability

**Attack Flow**:
```
1. User runs codex in workspace-write mode
2. AI generates exploit code targeting OS sandbox
3. Exploit breaks out of sandbox
4. Attacker gains access to entire filesystem
5. Attacker exfiltrates credentials from ~/.aws/, ~/.ssh/
```

**Mitigations**:
- M2 (Sandbox Isolation) - defense-in-depth
- M7 (Audit Logging) - detect anomalous behavior

**Residual Risk**: R2 (Zero-Day Sandbox Escape)

---

## Summary

**Critical Threats**:
1. Prompt Injection (High risk)
2. Sandbox Escape (High risk)
3. API Key Theft (High risk)
4. Data Exfiltration (High risk)

**Implemented Mitigations**:
- OS-level sandboxing (read-only, workspace-write, full-access)
- API key protection (env vars, file permissions)
- Data minimization (ZDR mode, local models)
- Audit logging (evidence repository, telemetry)
- Secure defaults (read-only, approval-required)

**Residual Risks**:
- AI model capability (inherent risk)
- Zero-day sandbox escape (OS-level)
- Supply chain compromise (ecosystem)
- Insider threat (AI provider)

**Acceptance Criteria**: Users must review all AI-generated code and understand inherent risks.

**Next**: [Sandbox System](sandbox-system.md)
