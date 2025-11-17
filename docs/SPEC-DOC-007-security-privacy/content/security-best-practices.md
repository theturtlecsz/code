# Security Best Practices

Configuration hardening, deployment patterns, and security checklists.

---

## Overview

**Security best practices** reduce attack surface and mitigate risks.

**Key Areas**:
1. Configuration hardening
2. Sandbox configuration
3. Secrets management
4. Network isolation
5. Dependency management
6. Incident response
7. Secure deployment

---

## Configuration Hardening

### Minimal Permissions

**Default Configuration** (recommended):
```toml
# Use balanced security
sandbox_mode = "workspace-write"  # Not read-only, not full-access
approval_policy = "on-request"    # Review operations before execution

[sandbox_workspace_write]
network_access = false       # Block network by default
allow_git_writes = false     # Protect .git/ folder
writable_roots = []          # No additional writable paths
```

**Avoid**:
```toml
sandbox_mode = "danger-full-access"  # ❌ Too permissive
approval_policy = "never"            # ❌ No safety gates
```

---

### Approval Policies

**Untrusted Environment** (maximum security):
```toml
approval_policy = "untrusted"  # Approve ALL operations (read, write, execute)
sandbox_mode = "read-only"     # No file writes
```

**Development** (balanced):
```toml
approval_policy = "on-request"  # Approve writes/commands
sandbox_mode = "workspace-write"  # Workspace-only writes
```

**Production/CI** (automation):
```toml
approval_policy = "on-failure"  # Only ask if command fails
sandbox_mode = "workspace-write"  # Workspace-only writes
```

**Never Use** (unsafe):
```toml
approval_policy = "never"              # ❌ No safety gates
sandbox_mode = "danger-full-access"    # ❌ Full system access
```

---

### Provider Selection

**Security Ranking** (privacy-focused):
1. ✅ **Ollama** (local) - No data leaves machine
2. ✅ **Azure OpenAI** (EU region) - GDPR-compliant, SOC 2, HIPAA
3. ⚠️ **Anthropic** - Privacy-focused, but no data residency guarantee
4. ⚠️ **Google Gemini** - 18-month retention
5. ⚠️ **OpenAI** - 30-day retention

**Recommendation**: Use Azure OpenAI for enterprise deployments

---

## Sandbox Configuration

### Workspace-Write Mode (Recommended)

**Configuration**:
```toml
sandbox_mode = "workspace-write"

[sandbox_workspace_write]
network_access = false       # Block network
allow_git_writes = false     # Protect .git/
writable_roots = []          # No additional paths
exclude_tmpdir_env_var = false  # Allow $TMPDIR writes
exclude_slash_tmp = false    # Allow /tmp writes
```

**Permissions**:
- ✅ Read any file on disk
- ✅ Write files in workspace (`cwd`)
- ✅ Write files in `/tmp` and `$TMPDIR`
- ❌ Write files outside workspace
- ❌ Access network
- ❌ Modify `.git/` folder

---

### Read-Only Mode (Maximum Security)

**Configuration**:
```toml
sandbox_mode = "read-only"
```

**Permissions**:
- ✅ Read any file on disk
- ❌ Write files
- ❌ Delete files
- ❌ Access network
- ❌ Execute privileged operations

**Use Case**: Code analysis, documentation generation, code review

---

### Full Access Mode (Docker Only)

**Configuration**:
```toml
sandbox_mode = "danger-full-access"
```

**WARNING**: Use ONLY in isolated environments (Docker, VM)

**Permissions**:
- ✅ Read any file
- ✅ Write any file
- ✅ Delete files
- ✅ Access network
- ✅ Execute privileged operations

**Use Case**: Running inside Docker container (container provides isolation)

---

## Secrets Management

### Environment Variables (Recommended)

**Setup**:
```bash
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."
```

**Benefits**:
- ✅ Not stored in files
- ✅ Excluded from AI context (shell_environment_policy)
- ✅ Easy to rotate

---

### .env Files (Local Development)

**Setup**:
```bash
# .env (git-ignored)
OPENAI_API_KEY=sk-proj-...
ANTHROPIC_API_KEY=sk-ant-...
```

**Load with direnv**:
```bash
brew install direnv
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
echo 'dotenv' > .envrc
direnv allow
```

**Ensure git-ignored**:
```gitignore
.env
.env.*
```

---

### auth.json (Alternative)

**Setup**:
```json
{
  "providers": {
    "openai": {
      "api_key": "sk-proj-..."
    },
    "anthropic": {
      "api_key": "sk-ant-..."
    }
  }
}
```

**Permissions** (critical):
```bash
chmod 600 ~/.code/auth.json
```

---

### Never Commit Secrets

**Git Hooks** (automatic):
- Pre-commit hook checks for secrets
- Blocks commit if secrets detected

**Manual Check**:
```bash
# Search for API keys
grep -r "sk-proj-" .
grep -r "sk-ant-" .
grep -r "AIza" .  # Google API key pattern
```

---

## Network Isolation

### Block Network by Default

**Configuration**:
```toml
[sandbox_workspace_write]
network_access = false  # Block ALL network (default)
```

**Behavior**:
- AI cannot make HTTP requests
- AI cannot download files
- Prevents data exfiltration

---

### Allow Network (Temporarily)

**One-Time Override**:
```bash
code --config sandbox_workspace_write.network_access=true "npm install"
```

**Profile-Based**:
```toml
[profiles.network-allowed]
sandbox_mode = "workspace-write"

[profiles.network-allowed.sandbox_workspace_write]
network_access = true
```

**Usage**:
```bash
code --profile network-allowed "install dependencies"
```

---

## Dependency Management

### Regular Audits

**npm Packages**:
```bash
# Weekly audit
npm audit

# Update dependencies
npm update

# Check for outdated
npm outdated
```

**Rust Crates**:
```bash
# Install cargo-audit
cargo install cargo-audit

# Weekly audit
cargo audit

# Update dependencies
cargo update
```

---

### Dependency Pinning

**npm** (lock file):
```bash
# Commit lock file
git add package-lock.json
git commit -m "chore: update dependencies"

# Use ci for strict lock file adherence
npm ci
```

**Cargo** (lock file):
```bash
# Commit lock file
git add Cargo.lock
git commit -m "chore: update dependencies"

# Ensure reproducible builds
cargo build --locked
```

---

### Supply Chain Security

**Verify MCP Servers**:
```bash
# Check npm package metadata
npm info @modelcontextprotocol/server-memory

# Verify source
# - High download count (>1000/week)
# - Official GitHub repository
# - Trusted maintainer
# - MIT/Apache license
```

**Avoid**:
- ❌ Low download count (<100/week)
- ❌ No GitHub repository
- ❌ Suspicious maintainer
- ❌ Recently published (typosquatting risk)

---

## Incident Response

### Security Incident Workflow

**1. Detection**:
- Monitor debug logs for suspicious activity
- Review evidence repository for anomalies
- Check API usage for unexpected spikes

**2. Containment**:
```bash
# Revoke compromised API key immediately
# OpenAI: platform.openai.com → API Keys → Revoke
# Generate new key
export OPENAI_API_KEY="sk-proj-NEW_KEY"
```

**3. Investigation**:
```bash
# Review debug logs
grep ERROR ~/.code/debug.log | tail -n 100

# Review session history
tail -n 100 ~/.code/history.jsonl

# Review evidence
find docs/SPEC-OPS-004-integrated-coder-hooks/evidence/ -mtime -1
```

**4. Eradication**:
```bash
# Delete compromised data
rm ~/.code/history.jsonl
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/

# Update dependencies
npm audit fix
cargo update
```

**5. Recovery**:
```bash
# Test new API key
code "Hello world"

# Resume normal operations
```

**6. Lessons Learned**:
- Document incident in git
- Update security practices
- Share findings with team

---

### Incident Response Checklist

**Compromised API Key**:
- [ ] Revoke old key at provider dashboard
- [ ] Generate new key
- [ ] Update environment variable or auth.json
- [ ] Test new key works
- [ ] Review provider usage logs for unauthorized activity
- [ ] Notify team (if applicable)

**Data Breach** (code with PII sent to AI):
- [ ] Identify affected data
- [ ] Request provider deletion (support@openai.com)
- [ ] Delete local evidence
- [ ] Notify affected parties (if GDPR/CCPA applies)
- [ ] Update security practices (approval gates, PII redaction)

**Malicious Code Injection**:
- [ ] Identify malicious commits
- [ ] Revert commits
- [ ] Review all code generated by AI
- [ ] Re-audit codebase
- [ ] Update approval policy (more strict)

---

## Secure Deployment

### Docker Deployment

**Dockerfile**:
```dockerfile
FROM rust:1.70 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /app/target/release/code /usr/local/bin/code

# Create non-root user
RUN useradd -m -u 1000 coder
USER coder

# Set environment variables
ENV CODEX_HOME=/home/coder/.code
ENV RUST_LOG=info

ENTRYPOINT ["code"]
```

**Benefits**:
- ✅ Isolated environment
- ✅ Non-root user
- ✅ Reproducible builds

---

### Kubernetes Deployment

**Deployment YAML**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: code-assistant
spec:
  replicas: 1
  template:
    spec:
      containers:
      - name: code-assistant
        image: code-assistant:latest
        env:
        - name: OPENAI_API_KEY
          valueFrom:
            secretKeyRef:
              name: api-keys
              key: openai-api-key
        securityContext:
          runAsNonRoot: true
          runAsUser: 1000
          readOnlyRootFilesystem: true
        resources:
          limits:
            memory: "2Gi"
            cpu: "1000m"
```

**Secret Creation**:
```bash
kubectl create secret generic api-keys \
  --from-literal=openai-api-key="sk-proj-..."
```

---

### CI/CD Security

**GitHub Actions**:
```yaml
name: Test

on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run tests
        env:
          OPENAI_API_KEY: ${{ secrets.OPENAI_API_KEY }}
        run: |
          cargo test
```

**Security**:
- ✅ Secrets encrypted at rest
- ✅ Secrets masked in logs
- ✅ Secrets not visible in fork PRs

---

## Security Checklist

### Initial Setup

- [ ] **Sandbox Mode**: Use `workspace-write` (not `danger-full-access`)
- [ ] **Approval Policy**: Use `on-request` (not `never`)
- [ ] **Network Access**: Disable (`network_access = false`)
- [ ] **Git Protection**: Disable git writes (`allow_git_writes = false`)
- [ ] **Secrets**: Use environment variables (not config.toml)
- [ ] **API Keys**: Store in .env or auth.json (git-ignored)
- [ ] **File Permissions**: `chmod 600 ~/.code/auth.json`
- [ ] **Provider**: Use Azure OpenAI (for enterprise) or Ollama (for privacy)

---

### Weekly Maintenance

- [ ] **Audit Dependencies**: `npm audit`, `cargo audit`
- [ ] **Update Dependencies**: `npm update`, `cargo update`
- [ ] **Review Logs**: Check `~/.code/debug.log` for errors
- [ ] **Monitor Costs**: Review API usage dashboards
- [ ] **Evidence Footprint**: Run `/spec-evidence-stats`
- [ ] **Rotate Logs**: Archive old debug logs

---

### Monthly Review

- [ ] **Rotate API Keys**: Generate new keys, revoke old
- [ ] **Review Evidence**: Archive evidence >30 days old
- [ ] **Delete History**: Delete `~/.code/history.jsonl` if sensitive
- [ ] **Security Audit**: Review threat model, update mitigations
- [ ] **MCP Servers**: Audit MCP server configurations
- [ ] **Compliance**: Review GDPR/SOC 2 compliance status

---

### Quarterly Tasks

- [ ] **Threat Model Update**: Re-assess risks, update mitigations
- [ ] **Security Training**: Review security best practices
- [ ] **Incident Response Drill**: Test incident response procedures
- [ ] **Vendor Assessment**: Review AI provider security certifications
- [ ] **Compliance Audit**: GDPR, SOC 2, CCPA compliance check

---

## Common Security Mistakes

### Mistake 1: Using Full Access Mode

**Problem**:
```toml
sandbox_mode = "danger-full-access"  # ❌ Too permissive
```

**Fix**:
```toml
sandbox_mode = "workspace-write"  # ✅ Balanced security
```

---

### Mistake 2: Hardcoding Secrets

**Problem**:
```toml
[model_providers.openai]
api_key = "sk-proj-..."  # ❌ Committed to git
```

**Fix**:
```bash
export OPENAI_API_KEY="sk-proj-..."  # ✅ Environment variable
```

---

### Mistake 3: No Approval Gates

**Problem**:
```toml
approval_policy = "never"  # ❌ AI runs anything
```

**Fix**:
```toml
approval_policy = "on-request"  # ✅ Review before execution
```

---

### Mistake 4: Allowing Network Access

**Problem**:
```toml
[sandbox_workspace_write]
network_access = true  # ❌ Data exfiltration risk
```

**Fix**:
```toml
[sandbox_workspace_write]
network_access = false  # ✅ Block network
```

---

### Mistake 5: Not Rotating API Keys

**Problem**: Using same API key for months/years

**Fix**: Rotate quarterly
```bash
# Generate new key, update environment
export OPENAI_API_KEY="sk-proj-NEW_KEY"

# Revoke old key at provider dashboard
```

---

### Mistake 6: Not Auditing Dependencies

**Problem**: Vulnerable dependencies undetected

**Fix**: Weekly audits
```bash
npm audit
cargo audit
```

---

### Mistake 7: Committing .env Files

**Problem**: `.env` file committed to git

**Fix**: Ensure git-ignored
```gitignore
.env
.env.*
```

**Cleanup** (if already committed):
```bash
git rm --cached .env
git commit -m "chore: remove .env from git"

# Remove from history
bfg --delete-files .env
git push --force
```

---

## Advanced Security

### Encryption at Rest (Future)

**Goal**: Encrypt local files

**Configuration** (future):
```toml
[security]
encrypt_at_rest = true
encryption_key = "$ENCRYPTION_KEY"
```

**Status**: Not yet implemented

---

### PII Detection (Future)

**Goal**: Automatically detect PII before sending to AI

**Usage** (future):
```bash
code --detect-pii "task"
# WARNING: Detected PII in code:
# - Email addresses (3 occurrences)
# - Phone numbers (1 occurrence)
# Redact before proceeding? [yes/no]
```

**Status**: Not yet implemented

---

### Network Allowlisting (Future)

**Goal**: Allow specific hosts only

**Configuration** (future):
```toml
[sandbox_workspace_write]
network_access = true
allowed_hosts = [
    "api.openai.com",
    "api.anthropic.com"
]
```

**Status**: Not yet implemented

---

## Summary

**Security Best Practices** highlights:

1. **Configuration Hardening**: `workspace-write` + `on-request` approval
2. **Sandbox Configuration**: Block network, protect .git/, workspace-only writes
3. **Secrets Management**: Environment variables, .env files, `chmod 600`
4. **Network Isolation**: Block network by default, temporary overrides
5. **Dependency Management**: Weekly audits (`npm audit`, `cargo audit`)
6. **Incident Response**: Revoke → Regenerate → Update → Audit → Notify
7. **Secure Deployment**: Docker (non-root user), Kubernetes (secrets), CI/CD (encrypted secrets)

**Security Checklist**:
- ✅ Use `workspace-write` sandbox mode
- ✅ Enable approval gates (`on-request`)
- ✅ Block network access
- ✅ Protect .git/ folder
- ✅ Store secrets in environment variables
- ✅ Audit dependencies weekly
- ✅ Rotate API keys quarterly
- ✅ Delete sensitive history periodically

**Common Mistakes**:
- ❌ Using full access mode
- ❌ Hardcoding secrets in config.toml
- ❌ No approval gates
- ❌ Allowing network access
- ❌ Not rotating API keys
- ❌ Not auditing dependencies
- ❌ Committing .env files

**Provider Recommendations**:
- **Enterprise**: Azure OpenAI (GDPR, SOC 2, HIPAA)
- **Privacy**: Ollama (local models, no data leaves machine)
- **General**: Anthropic (privacy-focused, but no data residency guarantee)

---

**See Also**:
- [Threat Model](threat-model.md) - Attack surfaces and risk assessment
- [Sandbox System](sandbox-system.md) - Detailed sandbox configuration
- [Secrets Management](secrets-management.md) - API key storage and rotation
- [Compliance](compliance.md) - GDPR, SOC 2, regulatory requirements
