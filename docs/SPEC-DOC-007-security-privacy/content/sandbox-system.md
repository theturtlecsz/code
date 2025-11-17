# Sandbox System

Three sandbox levels, configuration, and escape prevention.

---

## Overview

The **sandbox system** restricts what AI-generated code can access on your system.

**Purpose**: Prevent unauthorized file access, data exfiltration, and malicious code execution

**Implementation**: OS-level sandboxing (macOS Sandbox API, Linux landlock/seccomp)

---

## Sandbox Levels

### 1. Read-Only (Most Secure)

**Permissions**:
- ✅ Read any file on disk
- ❌ Write files
- ❌ Delete files
- ❌ Access network
- ❌ Execute privileged operations

**Use Cases**:
- Code analysis and questions
- Documentation generation (AI provides text, no writes)
- Code review and suggestions

**Configuration**:
```toml
sandbox_mode = "read-only"
```

**CLI**:
```bash
code --sandbox read-only "explain this code"
```

---

### 2. Workspace-Write (Balanced)

**Permissions**:
- ✅ Read any file on disk
- ✅ Write files in workspace (`cwd`)
- ✅ Write files in `/tmp` and `$TMPDIR`
- ❌ Write files outside workspace
- ❌ Access network (by default)
- ❌ Modify `.git/` folder (by default)

**Use Cases**:
- Code refactoring
- Bug fixes
- Feature implementation
- Test writing

**Configuration**:
```toml
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = false  # Block network (default)
allow_git_writes = false  # Protect .git/ folder (default)
writable_roots = []  # Additional writable paths
exclude_tmpdir_env_var = false  # Allow $TMPDIR writes
exclude_slash_tmp = false  # Allow /tmp writes
```

**CLI**:
```bash
code --sandbox workspace-write "refactor auth code"
```

---

### 3. Full Access (Least Secure)

**Permissions**:
- ✅ Read any file on disk
- ✅ Write any file on disk
- ✅ Delete files
- ✅ Access network
- ✅ Execute privileged operations

**Use Cases**:
- Running in Docker container (where container provides sandboxing)
- Older Linux kernels without landlock support
- Trust AI model completely (not recommended)

**Configuration**:
```toml
sandbox_mode = "danger-full-access"
```

**CLI**:
```bash
code --sandbox danger-full-access "task"
```

**Warning**: Use with extreme caution. Only appropriate when:
- Running in isolated environment (Docker, VM)
- Testing/development only
- You fully trust the AI model

---

## Approval Presets

### Read Only Preset

**Combination**: `approval_policy = "on-request"` + `sandbox_mode = "read-only"`

**Behavior**:
- AI can read files and answer questions
- Edits, commands, network access require approval

**Use Case**: Maximum safety, exploratory questions

---

### Auto Preset (Recommended)

**Combination**: `approval_policy = "on-request"` + `sandbox_mode = "workspace-write"`

**Behavior**:
- AI can read, edit, and run commands in workspace without approval
- Operations outside workspace or network access require approval

**Use Case**: Balanced productivity and safety

---

### Full Access Preset

**Combination**: `approval_policy = "never"` + `sandbox_mode = "danger-full-access"`

**Behavior**:
- AI has full disk and network access without prompts
- Extremely risky

**Use Case**: Docker containers, testing only

---

## File Access Rules

### Allowed Paths (Workspace-Write Mode)

**Always Allowed**:
- Current working directory (`cwd`) and subdirectories
- `/tmp` (unless `exclude_slash_tmp = true`)
- `$TMPDIR` (unless `exclude_tmpdir_env_var = true`)

**Example**:
```bash
cd /home/user/project
code "add tests"

# AI can write to:
# - /home/user/project/** (workspace)
# - /tmp/** (temp dir)
# - $TMPDIR/** (env temp dir)

# AI CANNOT write to:
# - /home/user/other-project/** (outside workspace)
# - /etc/** (system files)
# - /home/user/.ssh/** (credentials)
```

---

### Protected Paths

**Always Protected** (even in workspace-write):
- `.git/` folder (unless `allow_git_writes = true`)
- `.env` files (credential protection)
- `~/.ssh/` (SSH keys)
- `~/.aws/` (AWS credentials)

**Git Protection Example**:
```toml
[sandbox_workspace_write]
allow_git_writes = false  # Default: protect .git/
```

**Behavior**:
```bash
# AI cannot run:
git commit  # ❌ Writes to .git/
git checkout  # ❌ Modifies .git/

# AI CAN run (read-only):
git status  # ✅ Read-only
git diff  # ✅ Read-only
```

**Override** (when safe):
```toml
[sandbox_workspace_write]
allow_git_writes = true  # Allow git commits
```

---

### Additional Writable Roots

**Use Case**: Allow writes outside workspace (specific paths)

**Configuration**:
```toml
[sandbox_workspace_write]
writable_roots = [
    "/home/user/.pyenv/shims",  # Python shims
    "/usr/local/share/data"      # Shared data dir
]
```

**Warning**: Only add trusted paths. Each additional root increases attack surface.

---

## Network Access Control

### Default: Network Blocked

**Configuration**:
```toml
[sandbox_workspace_write]
network_access = false  # Default
```

**Behavior**:
- All outbound network connections blocked
- `curl`, `wget`, `http` requests fail
- Prevents data exfiltration

---

### Enable Network Access

**Use Case**: AI needs to fetch data (APIs, package managers)

**Configuration**:
```toml
[sandbox_workspace_write]
network_access = true  # Enable network
```

**Risks**:
- AI can exfiltrate data to external servers
- AI can download malicious code
- Increased attack surface

**Mitigation**: Review all network operations before approval

---

## Sandbox Escape Prevention

### Defense-in-Depth

**Layer 1: OS Sandbox**
- macOS: Sandbox API (`sandbox_init`)
- Linux: landlock + seccomp-bpf

**Layer 2: Path Validation**
- Canonicalize all file paths
- Block symlink attacks
- Verify paths are within allowed roots

**Layer 3: Command Validation**
- Validate shell commands before execution
- Block dangerous commands (`rm -rf /`, `dd if=/dev/zero`)
- Require approval for privileged operations

**Layer 4: User Approval**
- Prompt user before executing AI commands
- Show full command before approval
- Log all approved commands

---

### Symlink Attack Prevention

**Attack**: AI creates symlink to escape sandbox

**Example**:
```bash
# Attacker tries:
ln -s /etc/passwd workspace/passwd  # Create symlink
cat workspace/passwd  # Read /etc/passwd via symlink
```

**Prevention**:
1. Canonicalize paths (resolve symlinks)
2. Check final path is within allowed roots
3. Block symlink creation in workspace-write mode

**Status**: Implemented (path canonicalization)

---

### Sandbox Escape Detection

**Indicators**:
- File access outside allowed paths
- Network connections when network_access = false
- Privilege escalation attempts
- Unusual system calls

**Logging**:
```bash
export RUST_LOG=debug
code

# Check logs for sandbox violations:
tail -f ~/.code/debug.log | grep -i "sandbox\|violation"
```

---

## Platform Differences

### macOS

**Sandbox Implementation**: Sandbox API (`sandbox_init`)

**Features**:
- Filesystem restrictions (allow/deny paths)
- Network restrictions (allow/deny domains)
- IPC restrictions (process isolation)

**Limitations**:
- Complex sandbox profile syntax
- Limited runtime modification

---

### Linux

**Sandbox Implementation**: landlock + seccomp-bpf

**Features**:
- landlock: Filesystem access control (kernel 5.13+)
- seccomp-bpf: Syscall filtering

**Limitations**:
- Requires recent kernel (landlock support)
- Older kernels fall back to seccomp-only

**Fallback**: If landlock unavailable, use `danger-full-access` with warning

---

### Windows

**Status**: Limited sandboxing support

**Fallback**: Rely on user approval gates

---

## Configuration Examples

### Maximum Security

```toml
sandbox_mode = "read-only"
approval_policy = "always"  # Approve everything
```

**Use Case**: Untrusted AI models, exploratory analysis

---

### Balanced (Recommended)

```toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = false
allow_git_writes = false
exclude_tmpdir_env_var = false
exclude_slash_tmp = false
```

**Use Case**: Day-to-day development

---

### Development (Permissive)

```toml
sandbox_mode = "workspace-write"
approval_policy = "on-failure"  # Only ask if command fails

[sandbox_workspace_write]
network_access = true
allow_git_writes = true
```

**Use Case**: Rapid iteration, trusted environment

---

### Docker Container

```toml
sandbox_mode = "danger-full-access"
approval_policy = "never"
```

**Use Case**: Running inside Docker container (container provides isolation)

---

## Debugging Sandbox Issues

### Check Sandbox Status

```bash
code --sandbox-status
```

**Output**:
```
Sandbox Mode: workspace-write
Allowed Write Paths:
  - /home/user/project (workspace)
  - /tmp (temp)
  - $TMPDIR=/var/folders/... (env temp)

Protected Paths:
  - /home/user/project/.git (git protection)

Network Access: Blocked
Git Writes: Blocked
```

---

### Test Sandbox Restrictions

```bash
# Test write outside workspace
code --sandbox workspace-write "write test file to /etc/test"
# Expected: ❌ Permission denied

# Test network access
code --sandbox workspace-write "curl https://example.com"
# Expected: ❌ Network blocked

# Test git writes
code --sandbox workspace-write "git commit -m 'test'"
# Expected: ❌ Git writes blocked
```

---

### Enable Debug Logging

```bash
export RUST_LOG=codex_exec::sandbox=debug
code
```

**Log Output**:
```
[DEBUG] Sandbox mode: workspace-write
[DEBUG] Allowed paths: ["/home/user/project", "/tmp"]
[DEBUG] Network access: false
[DEBUG] Checking file access: /home/user/project/main.rs
[DEBUG] Access granted: within workspace
```

---

## Best Practices

### 1. Start with Read-Only

**Workflow**:
```
1. Start with read-only mode
2. Ask AI questions, get suggestions
3. Upgrade to workspace-write when ready to make changes
4. Review changes before approval
```

---

### 2. Never Use Full Access in Production

**Good**:
```toml
sandbox_mode = "workspace-write"  # Balanced
```

**Bad**:
```toml
sandbox_mode = "danger-full-access"  # ❌ Too permissive
```

---

### 3. Keep Git Protected

**Good**:
```toml
[sandbox_workspace_write]
allow_git_writes = false  # Protect .git/
```

**Why**: Prevents AI from:
- Creating malicious commits
- Modifying git history
- Corrupting repository

---

### 4. Block Network by Default

**Good**:
```toml
[sandbox_workspace_write]
network_access = false  # Block network
```

**Enable only when needed**:
```bash
# One-time override
code --sandbox workspace-write --config sandbox_workspace_write.network_access=true "npm install"
```

---

## Summary

**Sandbox Levels**:
1. Read-Only (most secure) - No writes
2. Workspace-Write (balanced) - Writes in project only
3. Full Access (least secure) - Unrestricted

**Key Features**:
- OS-level sandboxing (macOS Sandbox, Linux landlock)
- Filesystem restrictions (allowed paths, protected paths)
- Network isolation (block by default)
- Git protection (`.git/` folder)
- Symlink attack prevention

**Recommended Configuration**:
```toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"

[sandbox_workspace_write]
network_access = false
allow_git_writes = false
```

**Next**: [Secrets Management](secrets-management.md)
