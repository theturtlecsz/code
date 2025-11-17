# Secrets Management

API key storage, credential handling, and secret rotation practices.

---

## Overview

**Secrets management** protects sensitive credentials from unauthorized access.

**Critical Secrets**:
- API keys (OpenAI, Anthropic, Google, Azure)
- MCP server credentials
- HAL validation keys
- Database passwords (custom MCP servers)

**Storage Options** (security ranking):
1. ✅ Environment variables (recommended)
2. ⚠️ `.env` file (local development, git-ignored)
3. ❌ `config.toml` (NEVER store secrets)
4. ❌ Source code (NEVER hardcode secrets)

**Principle**: Secrets should NEVER be committed to version control

---

## API Key Management

### Environment Variables (Recommended)

**Usage**:
```bash
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."
```

**Benefits**:
- Not stored in files
- Inherited by child processes
- Easy to rotate (restart session)
- CI/CD friendly

**Limitations**:
- Lost on session close (unless in shell profile)
- Visible to all processes (security risk on shared systems)

---

### .env Files (Local Development)

**Setup**:
```bash
# .env (git-ignored)
OPENAI_API_KEY=sk-proj-...
ANTHROPIC_API_KEY=sk-ant-...
GOOGLE_API_KEY=...
HAL_SECRET_KAVEDARR_API_KEY=...
```

**Load with direnv**:
```bash
# Install direnv
brew install direnv  # macOS
apt install direnv   # Linux

# Enable for shell
echo 'eval "$(direnv hook bash)"' >> ~/.bashrc
source ~/.bashrc

# Create .envrc
echo 'dotenv' > .envrc
direnv allow
```

**Auto-loads** `.env` when entering directory

---

### Shell Environment Policy

**Default Behavior**: Excludes secrets from AI context

**Configuration**:
```toml
[shell_environment_policy]
inherit = "all"  # Inherit all env vars
ignore_default_excludes = false  # Exclude *KEY*, *TOKEN*, *SECRET*
exclude = ["AWS_*", "AZURE_*"]  # Additional exclusions
```

**Default Excludes** (case-insensitive):
- `*KEY*` (OPENAI_API_KEY, DB_KEY)
- `*TOKEN*` (GITHUB_TOKEN, ACCESS_TOKEN)
- `*SECRET*` (HAL_SECRET_KAVEDARR_API_KEY, DB_SECRET)

**Example**:
```bash
# Excluded by default
export OPENAI_API_KEY="sk-proj-..."  # Excluded (*KEY*)
export GITHUB_TOKEN="ghp_..."        # Excluded (*TOKEN*)
export DB_SECRET="password"          # Excluded (*SECRET*)

# Included (no KEY/TOKEN/SECRET)
export PATH="/usr/bin"               # Included
export HOME="/home/user"             # Included
export RUST_LOG="debug"              # Included
```

---

## Credential Storage Locations

### auth.json (Provider Credentials)

**Purpose**: Store provider API keys (alternative to environment variables)

**Location**: `~/.code/auth.json`

**Format**:
```json
{
  "providers": {
    "openai": {
      "api_key": "sk-proj-..."
    },
    "anthropic": {
      "api_key": "sk-ant-..."
    },
    "google": {
      "api_key": "..."
    },
    "azure": {
      "api_key": "...",
      "endpoint": "https://my-resource.openai.azure.com/"
    }
  }
}
```

**Permissions** (critical):
```bash
chmod 600 ~/.code/auth.json  # Owner read/write only
```

**Security**:
- ✅ Not committed to git (outside repo)
- ✅ Restricted file permissions
- ⚠️ Still stored on disk (vulnerable if disk compromised)
- ⚠️ No encryption at rest

**Precedence**: Environment variables > auth.json > config.toml

---

### MCP Server Credentials

**Environment Variables** (recommended):
```toml
[mcp_servers.database]
command = "/path/to/db-server"
# Server reads $DB_PASSWORD from environment
```

```bash
export DB_PASSWORD="secret"
```

**MCP env Field** (less secure):
```toml
[mcp_servers.database]
command = "/path/to/db-server"
env = { DB_PASSWORD = "secret" }  # ⚠️ Visible in config.toml
```

**Best Practice**: Use environment variables, not `env` field

---

### HAL Validation Keys

**Purpose**: Kavedarr API key for HAL policy validation

**Storage**:
```bash
export HAL_SECRET_KAVEDARR_API_KEY="..."
```

**Usage**:
```bash
export SPEC_OPS_TELEMETRY_HAL=1
/guardrail.plan SPEC-KIT-065
```

**Fallback**: Set `SPEC_OPS_HAL_SKIP=1` if key unavailable

---

## Secret Rotation

### API Key Rotation

**Procedure**:
1. Generate new API key (provider dashboard)
2. Update environment variable or auth.json
3. Test new key works
4. Revoke old key (provider dashboard)

**Example**:
```bash
# Update environment variable
export OPENAI_API_KEY="sk-proj-NEW_KEY"

# Test
code "Hello world"

# If successful, revoke old key at platform.openai.com
```

**Frequency**: Rotate quarterly or after suspected compromise

---

### auth.json Rotation

**Procedure**:
```bash
# Backup
cp ~/.code/auth.json ~/.code/auth.json.bak

# Edit with new keys
nano ~/.code/auth.json

# Test
code "Test message"

# If successful, delete backup
rm ~/.code/auth.json.bak
```

---

## Secret Leakage Prevention

### Git Hooks

**Pre-commit Hook** (automatic):
```bash
# Checks for common secret patterns
grep -r "sk-proj-" .
grep -r "sk-ant-" .
grep -r "AIza" .  # Google API key pattern
```

**Blocks commit** if secrets detected

---

### .gitignore

**Critical Entries**:
```gitignore
# Secrets
.env
.env.*
auth.json
*.key
*.pem

# Credential directories
~/.code/auth.json
.aws/
.ssh/
```

**Verify**:
```bash
git status --ignored
```

**Ensure** `.env` and `auth.json` are ignored

---

### Secret Scanning

**GitHub Secret Scanning** (automatic):
- Detects API keys in commits
- Alerts repository owner
- Provider may revoke key

**Tools**:
```bash
# TruffleHog (detect secrets in history)
pip install trufflehog
trufflehog filesystem .

# gitleaks (detect secrets in commits)
brew install gitleaks
gitleaks detect --source .
```

---

## Security Best Practices

### 1. Never Commit Secrets

**Bad**:
```toml
# config.toml
[model_providers.openai]
api_key = "sk-proj-..."  # ❌ NEVER DO THIS
```

**Good**:
```bash
export OPENAI_API_KEY="sk-proj-..."
```

---

### 2. Use Least Privilege Keys

**OpenAI Example**:
- ✅ Create project-specific API keys
- ✅ Set usage limits ($10/month)
- ❌ Don't use account-level keys

**Google Example**:
- ✅ Restrict API key to specific APIs
- ✅ Set referrer restrictions
- ❌ Don't use unrestricted keys

---

### 3. Restrict File Permissions

**auth.json**:
```bash
chmod 600 ~/.code/auth.json  # Owner read/write only
```

**.env**:
```bash
chmod 600 .env
```

**Verify**:
```bash
ls -la ~/.code/auth.json
# Should show: -rw------- (600)
```

---

### 4. Use Environment-Specific Keys

**Development**:
```bash
export OPENAI_API_KEY="sk-proj-dev-..."
```

**Production**:
```bash
export OPENAI_API_KEY="sk-proj-prod-..."
```

**Benefit**: Limit damage if dev key compromised

---

### 5. Audit API Key Usage

**OpenAI Dashboard**:
- Monitor usage by API key
- Set usage alerts
- Review logs for suspicious activity

**Google Cloud Console**:
- Check API key usage metrics
- Set quotas and rate limits
- Review access logs

---

## CI/CD Secret Management

### GitHub Actions

**Secrets Storage**:
```yaml
# .github/workflows/test.yml
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

**Set Secret**:
1. Repository → Settings → Secrets → New repository secret
2. Name: `OPENAI_API_KEY`
3. Value: `sk-proj-...`

**Benefits**:
- Encrypted at rest
- Masked in logs
- Not visible in fork PRs (security)

---

### GitLab CI

**Variables Storage**:
```yaml
# .gitlab-ci.yml
test:
  script:
    - cargo test
  variables:
    OPENAI_API_KEY: $OPENAI_API_KEY  # From GitLab CI/CD settings
```

**Set Variable**:
1. Project → Settings → CI/CD → Variables
2. Key: `OPENAI_API_KEY`
3. Value: `sk-proj-...`
4. ✅ Protected (only available to protected branches)
5. ✅ Masked (hidden in logs)

---

## Incident Response

### Suspected Key Compromise

**Immediate Actions**:
1. **Revoke Key**: Provider dashboard → Revoke API key
2. **Generate New Key**: Create replacement
3. **Update Config**: Environment variables or auth.json
4. **Audit Logs**: Check provider usage logs for unauthorized activity
5. **Notify Team**: Alert collaborators to rotate their keys

**Example (OpenAI)**:
```bash
# 1. Revoke at platform.openai.com
# 2. Generate new key
# 3. Update
export OPENAI_API_KEY="sk-proj-NEW_KEY"
# 4. Test
code "Test message"
# 5. Notify team via Slack/email
```

---

### Key Found in Git History

**Remove from History**:
```bash
# BFG Repo-Cleaner (recommended)
brew install bfg
bfg --replace-text secrets.txt  # List of secrets to remove
git reflog expire --expire=now --all
git gc --prune=now --aggressive

# Force push (WARNING: rewrites history)
git push --force
```

**Alternative (git-filter-repo)**:
```bash
pip install git-filter-repo
git filter-repo --path auth.json --invert-paths
git push --force
```

**Critical**: Revoke exposed key FIRST, then clean history

---

## Secret Rotation Schedule

### Recommended Frequency

| Secret Type | Rotation Frequency | Trigger |
|-------------|-------------------|---------|
| API Keys (prod) | Quarterly | Or after compromise |
| API Keys (dev) | Annually | Or after team change |
| MCP Server Credentials | Quarterly | Or after compromise |
| HAL Keys | Annually | Or after team change |

---

### Automated Rotation

**Future Enhancement**:
```bash
# Rotate API keys automatically
code --rotate-api-key openai

# Prompts:
# 1. Generate new key at provider
# 2. Enter new key
# 3. Test new key
# 4. Revoke old key
```

**Status**: Not yet implemented (manual rotation required)

---

## Debugging Secret Issues

### API Key Not Working

**Check**:
```bash
# 1. Verify environment variable exists
echo $OPENAI_API_KEY

# 2. Check auth.json
cat ~/.code/auth.json | jq .providers.openai.api_key

# 3. Test with curl
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"
```

**Common Causes**:
- Key revoked at provider
- Typo in key
- Wrong environment variable name
- Shell environment policy excluded key

---

### "Unauthorized" Errors

**Causes**:
- API key revoked
- Usage limit exceeded
- Incorrect provider (using OpenAI key with Anthropic provider)

**Fix**:
```bash
# Check provider match
code --config-dump | grep -A 5 model_provider

# Ensure correct key for provider
export OPENAI_API_KEY="sk-proj-..."  # For model_provider = "openai"
export ANTHROPIC_API_KEY="sk-ant-..."  # For model_provider = "anthropic"
```

---

## Summary

**Secrets Management** best practices:

1. **Storage**: Environment variables > .env file > NEVER config.toml
2. **Shell Environment Policy**: Auto-excludes *KEY*, *TOKEN*, *SECRET* patterns
3. **auth.json**: Alternative storage, requires `chmod 600` permissions
4. **Rotation**: Quarterly for production, annually for development
5. **Leakage Prevention**: Git hooks, .gitignore, secret scanning
6. **CI/CD**: Use encrypted secret storage (GitHub Secrets, GitLab Variables)
7. **Incident Response**: Revoke → Regenerate → Update → Audit → Notify

**Critical Rules**:
- ❌ NEVER commit secrets to git
- ❌ NEVER store secrets in config.toml
- ❌ NEVER hardcode secrets in source code
- ✅ Use environment variables
- ✅ Restrict file permissions (600)
- ✅ Rotate keys quarterly

**Next**: [Data Flow](data-flow.md)
