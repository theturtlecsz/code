# Audit Trail

Evidence collection, telemetry logging, and compliance tracking.

---

## Overview

**Audit trail** provides complete record of system activity for compliance, debugging, and security.

**Key Components**:
1. **Evidence Repository**: Telemetry, agent outputs, consensus artifacts
2. **Session History**: User prompts and AI responses
3. **Debug Logs**: System events and errors
4. **Quality Gate Results**: Checkpoint outcomes and validations
5. **Git Commits**: Code changes and commit messages

**Compliance Use Cases**:
- SOC 2 audit (demonstrate security controls)
- GDPR compliance (data access requests)
- Internal audits (cost tracking, quality validation)
- Incident investigation (root cause analysis)

---

## Evidence Repository

### Location and Structure

**Root**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/`

**Structure**:
```
evidence/
├── commands/                   # Per-SPEC command execution
│   ├── SPEC-KIT-001/
│   ├── SPEC-KIT-002/
│   └── SPEC-KIT-070/          # Example SPEC
│       ├── plan/
│       │   ├── plan_execution.json       # Telemetry
│       │   ├── agent_1_gemini-flash.txt  # Agent output
│       │   ├── agent_2_claude-haiku.txt
│       │   ├── agent_3_gpt5-medium.txt
│       │   └── consensus.json            # Consensus artifact
│       ├── tasks/
│       ├── implement/
│       ├── validate/
│       ├── audit/
│       └── unlock/
├── consensus/                  # MCP consensus artifacts
│   ├── runs/                  # Consensus run metadata
│   └── agents/                # Agent response cache
└── quality_gates/              # Quality gate checkpoint results
```

---

### Telemetry Schema (v1.0)

**All telemetry files** follow this base schema:

```json
{
  "command": "plan",
  "specId": "SPEC-KIT-070",
  "sessionId": "abc123",
  "timestamp": "2025-10-18T14:32:00Z",
  "schemaVersion": "1.0",
  "artifacts": ["docs/SPEC-KIT-070-dark-mode/plan.md"],
  "exit_code": 0
}
```

**Required Fields**:
- `command`: Stage name
- `specId`: SPEC-ID
- `sessionId`: Unique session identifier
- `timestamp`: ISO 8601 timestamp
- `schemaVersion`: "1.0"
- `artifacts`: Array of created files
- `exit_code`: 0 (success) or non-zero (failure)

**Stage-Specific Fields**: See [Evidence Repository](../SPEC-DOC-003-spec-kit-framework/content/evidence-repository.md)

---

### Agent Output Files

**Format**: `agent_{index}_{name}.txt`

**Contents**:
```
=== Agent Execution ===
Name: gemini-flash
Model: gemini-1.5-flash-latest
Stage: plan
Spec: SPEC-KIT-070
Session: abc123
Timestamp: 2025-10-18T14:32:15Z

=== Prompt ===
[Full prompt sent to agent...]

=== Response ===
[Agent's complete response...]

=== Metadata ===
Input tokens: 5000
Output tokens: 1500
Cost: $0.12
Duration: 8500ms
Status: success
```

**Use Case**: Reproduce agent decisions, audit AI reasoning

---

### Consensus Artifacts

**Format**: `consensus.json` (per stage)

```json
{
  "spec_id": "SPEC-KIT-070",
  "stage": "plan",
  "run_id": "run-abc123",
  "timestamp": "2025-10-18T14:35:00Z",
  "inputs": {
    "agent_count": 3,
    "agents": ["gemini-flash", "claude-haiku", "gpt5-medium"],
    "artifacts": ["docs/SPEC-KIT-070-dark-mode/spec.md"]
  },
  "verdict": {
    "status": "ok",
    "present_agents": ["gemini-flash", "claude-haiku", "gpt5-medium"],
    "missing_agents": [],
    "degraded": false,
    "conflicts": []
  },
  "synthesized_output": "[Full consensus synthesis...]",
  "cost": 0.40,
  "duration_ms": 11200
}
```

**Use Case**: Verify multi-agent consensus, audit decision-making process

---

### Quality Gate Evidence

**Format**: `quality_gates/{checkpoint}_{gate_type}.json`

**Example**: `quality_gates/AfterSpecify_checklist.json`

```json
{
  "checkpoint": "AfterSpecify",
  "spec_id": "SPEC-KIT-070",
  "gate_type": "checklist",
  "timestamp": "2025-10-18T14:40:00Z",
  "native_result": {
    "overall_score": 82.0,
    "grade": "B",
    "issues": [
      {
        "id": "CHK-001",
        "severity": "IMPORTANT",
        "description": "3 quantifiers without metrics"
      }
    ]
  },
  "gpt5_validations": [...],
  "user_escalations": [...],
  "outcome": {
    "status": "passed",
    "initial_score": 82.0,
    "final_score": 95.0,
    "grade_change": "B → A"
  },
  "cost": 0.05,
  "duration_ms": 1200
}
```

**Use Case**: Demonstrate quality gate compliance, audit checkpoint results

---

## Session History

### Location

**File**: `~/.code/history.jsonl`

**Format**: JSONL (JSON Lines)

---

### Contents

```json
{"timestamp":"2025-10-18T14:32:00Z","role":"user","content":"Explain this code..."}
{"timestamp":"2025-10-18T14:32:15Z","role":"assistant","content":"This function authenticates..."}
{"timestamp":"2025-10-18T14:35:00Z","role":"user","content":"Add error handling"}
{"timestamp":"2025-10-18T14:35:20Z","role":"assistant","content":"I'll add error handling..."}
```

**Fields**:
- `timestamp`: ISO 8601 timestamp
- `role`: "user" or "assistant"
- `content`: Message text

---

### Use Cases

**Debugging**:
- Reproduce user interactions
- Investigate AI misbehavior
- Analyze conversation flow

**Compliance**:
- GDPR data access request (show all user interactions)
- Internal audit (review AI usage)

**Cost Tracking**:
- Extract prompts to estimate token usage
- Identify expensive queries

---

### Privacy Considerations

**PII Risk**: May contain sensitive prompts/code

**Mitigation**:
```bash
# Delete history
rm ~/.code/history.jsonl

# Or anonymize
jq '.content = "[REDACTED]"' ~/.code/history.jsonl > history_anonymized.jsonl
```

---

## Debug Logs

### Location

**File**: `~/.code/debug.log`

**Auto-Created**: When `RUST_LOG=debug` or `--debug` flag used

---

### Contents

```
[2025-10-18T14:32:00Z DEBUG codex_cli] Starting session...
[2025-10-18T14:32:01Z DEBUG codex_config] Loading config from ~/.code/config.toml
[2025-10-18T14:32:02Z DEBUG codex_mcp_client] Starting MCP server: local-memory
[2025-10-18T14:32:03Z DEBUG codex_mcp_client] MCP server ready: local-memory (PID: 12345)
[2025-10-18T14:32:15Z INFO  codex_api] API request to openai: gpt-5 (prompt: 1234 tokens)
[2025-10-18T14:32:20Z INFO  codex_api] API response: 567 tokens, cost: $0.05
[2025-10-18T14:32:21Z DEBUG codex_tui] Rendering response...
```

**Fields**:
- Timestamp: `[2025-10-18T14:32:00Z]`
- Level: `DEBUG`, `INFO`, `WARN`, `ERROR`
- Module: `codex_cli`, `codex_config`, `codex_mcp_client`
- Message: Log content

---

### Use Cases

**Debugging**:
- Investigate crashes
- Trace execution flow
- Identify performance bottlenecks

**Security**:
- Detect unauthorized access attempts
- Audit MCP server activity
- Monitor API usage

**Compliance**:
- Demonstrate logging controls (SOC 2)
- Audit trail for security events

---

### Log Rotation

**Manual Rotation**:
```bash
# Archive old logs
mv ~/.code/debug.log ~/.code/debug.log.$(date +%Y%m%d)
gzip ~/.code/debug.log.$(date +%Y%m%d)

# Delete old archives (>90 days)
find ~/.code/ -name "debug.log.*.gz" -mtime +90 -delete
```

**Automated Rotation** (future enhancement):
```toml
[logging]
max_size_mb = 100  # Rotate after 100 MB
max_age_days = 30  # Delete logs older than 30 days
```

---

## Git Commit History

### Audit Trail

**Complete History**:
```bash
git log --all --decorate --oneline --graph
```

**Commit Details**:
```bash
git log --format="%H %an %ae %ai %s" > commit_audit.txt
```

**Output**:
```
06f5c4b John Doe john@example.com 2025-10-18 14:32:00 +0000 docs(SPEC-DOC-004): add performance testing guide
ffbd393 Jane Smith jane@example.com 2025-10-17 10:15:00 +0000 docs(SPEC-DOC-004): add CI/CD integration guide
```

---

### Evidence Commits

**Spec-Kit Evidence**: Committed to git repository

**Example**:
```bash
git log --all --grep="SPEC-KIT-070" --oneline
```

**Output**:
```
a1b2c3d feat(SPEC-KIT-070): implement dark mode toggle
d4e5f6g docs(SPEC-KIT-070): add plan and tasks
```

**Use Case**: Trace SPEC evolution, audit code changes

---

## Audit Queries

### Evidence Queries

**Find All Consensus Runs for SPEC**:
```bash
find docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/ -name "consensus.json"
```

**Extract Total Cost for SPEC**:
```bash
jq -s 'map(.total_cost) | add' docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/*/execution.json
```

**Output**: `2.71` (total cost for full pipeline)

---

**Find Failed Stages**:
```bash
grep -r '"exit_code": [^0]' docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/
```

---

**List Quality Gate Results**:
```bash
ls -lh docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/quality_gates/
```

---

### Session History Queries

**Extract All User Prompts**:
```bash
jq 'select(.role == "user") | .content' ~/.code/history.jsonl
```

**Count Messages by Role**:
```bash
jq -s 'group_by(.role) | map({role: .[0].role, count: length})' ~/.code/history.jsonl
```

**Output**:
```json
[
  {"role": "user", "count": 45},
  {"role": "assistant", "count": 45}
]
```

---

### Debug Log Queries

**Extract API Requests**:
```bash
grep "API request" ~/.code/debug.log
```

**Count API Requests by Provider**:
```bash
grep "API request" ~/.code/debug.log | awk '{print $8}' | sort | uniq -c
```

**Output**:
```
  25 openai
  15 anthropic
   5 google
```

---

**Find Errors**:
```bash
grep ERROR ~/.code/debug.log
```

---

## Compliance Reporting

### SOC 2 Audit

**Required Evidence**:
1. **Access Controls**: Who can use the system?
2. **Audit Logging**: Complete record of operations
3. **Change Management**: Code review process
4. **Incident Response**: Security event handling

**Provided by Audit Trail**:
- ✅ Evidence repository (complete operation logs)
- ✅ Session history (user activity tracking)
- ✅ Debug logs (security events)
- ✅ Git commits (change tracking)

**Gaps**:
- ❌ Access controls (single-user tool)
- ⚠️ Encryption at rest (logs unencrypted)

**Recommendation**: Use Azure OpenAI for SOC 2 compliance

---

### GDPR Data Access Request

**User Rights**:
- Right to access (provide all user data)
- Right to erasure (delete all user data)
- Right to portability (export user data)

**Compliance**:

1. **Access Request**:
```bash
# Export all user data
cat ~/.code/history.jsonl > user_data_export.jsonl
find docs/SPEC-OPS-004-integrated-coder-hooks/evidence/ -type f -exec cat {} \; > evidence_export.txt
```

2. **Erasure Request**:
```bash
# Delete all user data
rm ~/.code/history.jsonl
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
rm ~/.code/debug.log

# Request provider deletion (OpenAI, Anthropic, Google)
# Email: support@openai.com, privacy@anthropic.com
```

3. **Portability Request**:
```bash
# Export in machine-readable format
tar -czf user_data.tar.gz ~/.code/history.jsonl docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
```

---

### Cost Audit

**Total Cost by SPEC**:
```bash
# Extract costs from evidence
for spec in docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/*/; do
  spec_id=$(basename "$spec")
  total_cost=$(jq -s 'map(.total_cost // 0) | add' "$spec"/*/execution.json 2>/dev/null || echo "0")
  echo "$spec_id: \$$total_cost"
done
```

**Output**:
```
SPEC-KIT-001: $1.20
SPEC-KIT-002: $2.71
SPEC-KIT-070: $2.65
```

---

**Total Cost by Provider**:
```bash
# Extract from debug logs
grep "API response" ~/.code/debug.log | awk '{print $8, $12}' | awk '{sum[$1]+=$2} END {for (p in sum) print p": $"sum[p]}'
```

**Output**:
```
openai: $15.50
anthropic: $8.20
google: $3.10
```

---

## Evidence Retention

### Retention Policy

**Evidence Types**:

| Type | Retention Period | Storage | Reason |
|------|-----------------|---------|--------|
| Telemetry JSON | Indefinite | Git repo | Audit trail |
| Agent Outputs | 30 days | Git repo (archived after) | Debugging |
| Consensus Artifacts | Indefinite | Git repo | Reproducibility |
| Session History | 90 days | Local (`~/.code/`) | Privacy |
| Debug Logs | 30 days | Local (`~/.code/`) | Debugging |
| Quality Gate Results | Indefinite | Git repo | Compliance |

---

### Archival Strategy

**After 30 Days**:
```bash
# Archive old evidence
mv docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-070/ \
   docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/SPEC-KIT-070-2025-10-18/

# Compress
tar -czf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/SPEC-KIT-070-2025-10-18.tar.gz \
        docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/SPEC-KIT-070-2025-10-18/

# Delete uncompressed
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/SPEC-KIT-070-2025-10-18/
```

---

**After 90 Days**:
```bash
# Delete archived evidence
find docs/SPEC-OPS-004-integrated-coder-hooks/evidence/archive/ -name "*.tar.gz" -mtime +90 -delete

# Delete old session history
find ~/.code/ -name "history.jsonl.*.gz" -mtime +90 -delete

# Delete old debug logs
find ~/.code/ -name "debug.log.*.gz" -mtime +90 -delete
```

---

## Monitoring and Alerting

### Evidence Footprint Monitoring

**Command**: `/spec-evidence-stats`

**Usage**:
```bash
/spec-evidence-stats --spec SPEC-KIT-070
```

**Output**:
```
SPEC-KIT-070 Detail:
  Total: 580 KB (2.3% of 25 MB limit)
  Breakdown:
    plan/           120 KB
    tasks/           45 KB
    implement/      110 KB
    validate/       135 KB
    audit/           95 KB
    unlock/          50 KB
    quality_gates/   25 KB

Status: ✅ OK (within 25 MB soft limit)
```

**Alert**: When SPEC exceeds 20 MB (80% of limit)

---

### Cost Monitoring

**Track Costs**:
```bash
# Daily cost
grep "API response" ~/.code/debug.log | \
  awk -v today="$(date +%Y-%m-%d)" '$1 ~ today {sum+=$12} END {print "$"sum}'
```

**Alert**: When daily cost exceeds $10

---

### Error Monitoring

**Track Errors**:
```bash
# Count errors today
grep ERROR ~/.code/debug.log | grep "$(date +%Y-%m-%d)" | wc -l
```

**Alert**: When error count exceeds 10 per day

---

## Best Practices

### 1. Enable Comprehensive Logging

```bash
# Always use debug logging
export RUST_LOG=debug
code
```

**Or**:
```bash
code --debug
```

---

### 2. Commit Evidence to Git

```bash
# After each stage
git add docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
git commit -m "evidence(SPEC-KIT-070): add plan stage evidence"
```

**Benefit**: Version-controlled audit trail

---

### 3. Monitor Evidence Footprint

```bash
# Weekly check
/spec-evidence-stats
```

**Action**: Archive evidence when approaching 25 MB limit

---

### 4. Rotate Logs Regularly

```bash
# Monthly rotation
mv ~/.code/debug.log ~/.code/debug.log.$(date +%Y%m%d)
gzip ~/.code/debug.log.$(date +%Y%m%d)
```

---

### 5. Protect Audit Logs

```bash
# Restrict permissions
chmod 600 ~/.code/history.jsonl
chmod 600 ~/.code/debug.log
```

**Prevents**: Unauthorized access to audit logs

---

## Summary

**Audit Trail** components:

1. **Evidence Repository**: Telemetry, agent outputs, consensus artifacts, quality gates
2. **Session History**: User prompts and AI responses (`~/.code/history.jsonl`)
3. **Debug Logs**: System events and errors (`~/.code/debug.log`)
4. **Git Commits**: Code changes and commit messages
5. **Quality Gates**: Checkpoint results and validations

**Compliance Support**:
- ✅ SOC 2: Complete audit trail, change management
- ✅ GDPR: Data access, erasure, portability
- ✅ Cost Audit: Per-SPEC cost tracking
- ⚠️ Gaps: No access controls, no encryption at rest

**Retention Policy**:
- Telemetry/consensus: Indefinite (git)
- Agent outputs: 30 days (archived)
- Session history: 90 days (local)
- Debug logs: 30 days (local)

**Best Practices**:
- ✅ Enable debug logging (`RUST_LOG=debug`)
- ✅ Commit evidence to git
- ✅ Monitor footprint (`/spec-evidence-stats`)
- ✅ Rotate logs regularly (monthly)
- ✅ Protect audit logs (`chmod 600`)

**Next**: [Compliance](compliance.md)
