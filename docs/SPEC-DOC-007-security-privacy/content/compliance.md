# Compliance

GDPR, SOC 2, and regulatory considerations for AI coding assistants.

---

## Overview

**Compliance** ensures the system meets regulatory and industry standards.

**Key Frameworks**:
1. **GDPR** (General Data Protection Regulation) - EU privacy law
2. **SOC 2** (System and Organization Controls 2) - US security standard
3. **CCPA** (California Consumer Privacy Act) - California privacy law
4. **ISO 27001** - International information security standard

**Applicability**:
- GDPR: If processing EU citizen data
- SOC 2: If selling to US enterprises
- CCPA: If processing California resident data
- ISO 27001: If required by customer contracts

---

## GDPR Compliance

### Requirements

**Core Principles**:
1. **Lawfulness, Fairness, Transparency**: Clear data usage policies
2. **Purpose Limitation**: Only collect data for specified purposes
3. **Data Minimization**: Collect only necessary data
4. **Accuracy**: Keep data accurate and up-to-date
5. **Storage Limitation**: Delete data when no longer needed
6. **Integrity and Confidentiality**: Protect data with security measures
7. **Accountability**: Demonstrate compliance

---

### Data Processing

**What Data is Processed**:
- User prompts (text input)
- Code files (source code)
- Conversation history
- API usage telemetry
- Agent outputs

**Legal Basis**:
- **Consent**: User explicitly agrees to use AI coding assistant
- **Legitimate Interest**: Providing coding assistance service
- **Contract**: Fulfilling user's request for assistance

**Recommendation**: Obtain explicit consent before processing code with PII

---

### Data Residency

**Requirement**: EU citizen data must stay in EU

**Compliance Strategy**:

**Option 1: Azure OpenAI (EU Region)**
```toml
[model_providers.azure]
api_key = "$AZURE_OPENAI_API_KEY"
endpoint = "https://my-eu-resource.openai.azure.com/"  # EU region
```

**Benefits**:
- ✅ Data stays in EU
- ✅ Microsoft GDPR compliance
- ✅ Data Processing Agreement (DPA) included

---

**Option 2: Ollama (Local)**
```toml
model_provider = "ollama"
model = "llama2"

[model_providers.ollama]
base_url = "http://localhost:11434"
```

**Benefits**:
- ✅ No data leaves machine (complete data residency)
- ❌ Lower quality than cloud models
- ❌ Requires powerful hardware

---

**Option 3: Anthropic (No Guarantee)**
```toml
model_provider = "anthropic"
```

**Warning**: Anthropic does NOT guarantee EU data residency

---

### User Rights

#### Right to Access (Article 15)

**Requirement**: Provide all user data upon request

**Implementation**:
```bash
# Export all user data
cat ~/.code/history.jsonl > user_data_export.jsonl
tar -czf user_evidence.tar.gz docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
```

**Provide to User**: `user_data_export.jsonl`, `user_evidence.tar.gz`

---

#### Right to Erasure (Article 17)

**Requirement**: Delete all user data upon request

**Implementation**:
```bash
# Delete local data
rm ~/.code/history.jsonl
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
rm ~/.code/debug.log
rm -rf ~/.code/mcp-memory/

# Request provider deletion
# OpenAI: support@openai.com (30-day retention)
# Anthropic: privacy@anthropic.com
# Google: (via Google Takeout or support)
# Azure: Not stored (no deletion needed)
```

**Timeline**: Complete within 30 days

---

#### Right to Portability (Article 20)

**Requirement**: Export user data in machine-readable format

**Implementation**:
```bash
# Export as JSON
tar -czf user_data_portable.tar.gz \
  ~/.code/history.jsonl \
  docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
```

**Provide to User**: `user_data_portable.tar.gz` (JSON format)

---

#### Right to Rectification (Article 16)

**Requirement**: Correct inaccurate data

**Implementation**:
- Edit session history: `nano ~/.code/history.jsonl`
- Edit evidence: `nano docs/SPEC-OPS-004-integrated-coder-hooks/evidence/.../execution.json`

**Note**: Rarely applicable (AI assistant stores minimal personal data)

---

### Data Protection Impact Assessment (DPIA)

**Required If**: High-risk processing (e.g., code with customer PII)

**DPIA Template**:

```markdown
# Data Protection Impact Assessment

## Processing Description
- **Purpose**: AI-assisted code development
- **Data Types**: User prompts, code files, conversation history
- **Data Subjects**: Developers using the system
- **Storage**: Local filesystem + AI provider servers
- **Retention**: 30-90 days (local), 30 days (AI providers)

## Necessity and Proportionality
- **Necessity**: Required to provide coding assistance
- **Proportionality**: Minimal data collected (only user prompts + code)

## Risks to Data Subjects
- **Risk 1**: Code may contain customer PII → Mitigation: Approval gates
- **Risk 2**: Data sent to AI providers → Mitigation: Azure EU region
- **Risk 3**: Data stored unencrypted → Mitigation: Encrypt at rest (future)

## Measures to Address Risks
- Approval gates (review prompts before sending)
- Azure OpenAI (EU data residency)
- Data deletion after 90 days
- User consent before processing

## Compliance
- ✅ Data minimization
- ✅ Purpose limitation
- ✅ Storage limitation
- ⚠️ Encryption at rest (not yet implemented)
```

---

### Consent Management

**Consent Requirement**: Explicit, informed, freely given

**Implementation**:
```toml
# First-run consent prompt
[gdpr]
require_consent = true
consent_text = """
This AI coding assistant sends your prompts and code to AI providers
(OpenAI, Anthropic, Google). By using this tool, you consent to:

1. Processing of your code and prompts by AI providers
2. Storage of conversation history for 90 days
3. Evidence collection for quality assurance

You can withdraw consent at any time by deleting ~/.code/

Do you consent? [yes/no]
"""
```

**Status**: Not yet implemented (future enhancement)

---

## SOC 2 Compliance

### Trust Service Criteria

**SOC 2 Type II** requires controls in 5 categories:

---

#### 1. Security (CC6.0)

**Requirement**: Protect system against unauthorized access

**Implementation**:
- ✅ API key authentication
- ✅ File permissions (chmod 600)
- ✅ Sandbox restrictions (workspace-write mode)
- ⚠️ No multi-user access controls

**Gap**: Single-user tool (no role-based access control)

---

#### 2. Availability (A1.0)

**Requirement**: System available as agreed

**Implementation**:
- ✅ Local installation (no SaaS downtime)
- ✅ Offline mode (Ollama)
- ⚠️ Dependent on AI provider availability

**Monitoring**:
```bash
# Check API provider status
curl -I https://api.openai.com/v1/models
```

---

#### 3. Processing Integrity (PI1.0)

**Requirement**: Processing is complete, valid, accurate, timely

**Implementation**:
- ✅ Evidence repository (complete audit trail)
- ✅ Quality gates (validation checkpoints)
- ✅ Multi-agent consensus (accuracy)
- ✅ Telemetry schema validation

**Evidence**: All processing captured in evidence repository

---

#### 4. Confidentiality (C1.0)

**Requirement**: Protect confidential information

**Implementation**:
- ✅ API keys in environment variables (not config files)
- ✅ Shell environment policy (excludes secrets)
- ✅ File permissions (600 for sensitive files)
- ⚠️ No encryption at rest

**Gap**: Unencrypted local storage

---

#### 5. Privacy (P1.0)

**Requirement**: Protect personal information

**Implementation**:
- ✅ Data minimization (only necessary data collected)
- ✅ Data retention policy (30-90 days)
- ✅ User rights (access, erasure, portability)
- ⚠️ No data anonymization

**Gap**: No automatic PII detection/redaction

---

### SOC 2 Evidence

**Required Artifacts**:
1. **Access Logs**: Session history, debug logs
2. **Change Logs**: Git commits, evidence repository
3. **Incident Logs**: Error logs, security events
4. **Configuration Management**: config.toml, version control
5. **Risk Assessment**: Threat model, DPIA

**Provided by System**:
- ✅ Evidence repository (telemetry, agent outputs)
- ✅ Session history (`~/.code/history.jsonl`)
- ✅ Debug logs (`~/.code/debug.log`)
- ✅ Git commits (change tracking)
- ✅ Threat model (documented)

---

### SOC 2 Gaps

**Missing Controls**:
1. ❌ Multi-user access controls (single-user tool)
2. ❌ Encryption at rest (local files unencrypted)
3. ❌ Formal incident response plan
4. ❌ Security awareness training (N/A for single user)
5. ❌ Vendor management (AI provider assessments)

**Recommendation**: For SOC 2 compliance, use Azure OpenAI (SOC 2 certified)

---

## CCPA Compliance

### Requirements

**CCPA** (California Consumer Privacy Act) similar to GDPR:

1. **Right to Know**: What data is collected
2. **Right to Delete**: Delete all user data
3. **Right to Opt-Out**: Opt-out of data selling (N/A - no data selling)
4. **Right to Non-Discrimination**: No discrimination for exercising rights

---

### Implementation

**Right to Know**:
- Provide data inventory: user prompts, code files, conversation history
- Document: See [Data Flow](data-flow.md)

**Right to Delete**:
- Same as GDPR Right to Erasure
- Implementation: See [GDPR Compliance](#right-to-erasure-article-17)

**Right to Opt-Out**:
- N/A (no data selling)

**Right to Non-Discrimination**:
- N/A (single-user tool)

---

## ISO 27001 Compliance

### Requirements

**ISO 27001** (Information Security Management System):

1. **Information Security Policy**: Documented security policies
2. **Risk Assessment**: Identify and assess risks
3. **Security Controls**: Implement controls to mitigate risks
4. **Audit and Review**: Regular security audits
5. **Continuous Improvement**: Update controls based on audits

---

### Implementation

**Information Security Policy**:
- Document: See [Security Best Practices](security-best-practices.md)

**Risk Assessment**:
- Document: See [Threat Model](threat-model.md)

**Security Controls**:
- Sandbox system (file access restrictions)
- Approval gates (user review)
- Secrets management (environment variables)
- Audit logging (evidence repository)

**Audit and Review**:
- Evidence repository (complete audit trail)
- Quality gates (validation checkpoints)

**Continuous Improvement**:
- Git commits (track security improvements)
- Security patches (dependency updates)

---

## Industry-Specific Compliance

### HIPAA (Healthcare)

**Requirement**: Protect Protected Health Information (PHI)

**Risk**: Code may contain patient data

**Mitigation**:
- ✅ Business Associate Agreement (BAA) with AI provider (Azure OpenAI supports HIPAA)
- ✅ Encryption in transit (HTTPS)
- ❌ Encryption at rest (not yet implemented)
- ✅ Audit logging (evidence repository)
- ✅ Access controls (file permissions)

**Recommendation**: Use Azure OpenAI with BAA for HIPAA compliance

---

### PCI DSS (Payment Card Industry)

**Requirement**: Protect credit card data

**Risk**: Code may contain payment processing logic with test card numbers

**Mitigation**:
- ⚠️ Redact test card numbers before asking AI
- ✅ Approval gates (review prompts)
- ✅ Audit logging (evidence repository)
- ❌ No PCI DSS certification (not designed for payment processing)

**Recommendation**: Do NOT process live payment card data with AI coding assistant

---

### FERPA (Education)

**Requirement**: Protect student education records

**Risk**: Code may contain student data

**Mitigation**:
- ✅ Redact student data before asking AI
- ✅ Approval gates (review prompts)
- ✅ Data deletion after 90 days

---

## Compliance Checklist

### GDPR

- [ ] **Data Residency**: Use Azure OpenAI (EU region) or Ollama (local)
- [ ] **Consent**: Obtain user consent before processing code
- [ ] **User Rights**: Implement access, erasure, portability
- [ ] **Data Minimization**: Only collect necessary data
- [ ] **Storage Limitation**: Delete data after 90 days
- [ ] **DPIA**: Conduct Data Protection Impact Assessment
- [ ] **DPA**: Sign Data Processing Agreement with AI provider

---

### SOC 2

- [ ] **Access Controls**: Restrict file permissions (chmod 600)
- [ ] **Audit Logging**: Enable debug logging, commit evidence to git
- [ ] **Change Management**: Use git for all changes
- [ ] **Incident Response**: Document incident response plan
- [ ] **Vendor Management**: Assess AI provider security (Azure recommended)
- [ ] **Encryption**: Encrypt at rest (future enhancement)

---

### CCPA

- [ ] **Privacy Policy**: Document data collection practices
- [ ] **Right to Delete**: Implement data deletion upon request
- [ ] **Right to Know**: Provide data inventory upon request

---

### ISO 27001

- [ ] **Information Security Policy**: Document security policies
- [ ] **Risk Assessment**: Complete threat model
- [ ] **Security Controls**: Implement sandbox, approval gates, secrets management
- [ ] **Audit and Review**: Regular evidence repository reviews
- [ ] **Continuous Improvement**: Track security improvements in git

---

## Compliance Gaps

### Current Limitations

1. **No Encryption at Rest**: Local files unencrypted
2. **No Multi-User Access Controls**: Single-user tool
3. **No Formal Incident Response Plan**: Ad-hoc security event handling
4. **No Automatic PII Detection**: Manual PII redaction required
5. **No Data Anonymization**: No automatic data anonymization

---

### Future Enhancements

**Encryption at Rest**:
```toml
[security]
encrypt_at_rest = true
encryption_key = "$ENCRYPTION_KEY"  # From environment
```

**Status**: Not yet implemented

---

**PII Detection**:
```bash
# Automatically detect PII before sending to AI
code --detect-pii "task"
```

**Status**: Not yet implemented

---

**Data Anonymization**:
```bash
# Anonymize code before sending to AI
code --anonymize "task"
```

**Status**: Not yet implemented

---

## Vendor Compliance

### AI Provider Certifications

| Provider | GDPR | SOC 2 | HIPAA | ISO 27001 |
|----------|------|-------|-------|-----------|
| OpenAI | ⚠️ (no guarantee) | ✅ | ❌ | ✅ |
| Anthropic | ⚠️ (no guarantee) | ✅ | ❌ | ❌ |
| Google | ✅ | ✅ | ✅ (Google Cloud) | ✅ |
| Azure OpenAI | ✅ | ✅ | ✅ | ✅ |
| Ollama | N/A (local) | N/A | N/A | N/A |

**Recommendation**: Use Azure OpenAI for enterprise compliance

---

## Summary

**Compliance** framework support:

1. **GDPR**: EU data residency (Azure), user rights (access, erasure, portability), DPIA
2. **SOC 2**: Audit logging, change management, processing integrity, confidentiality
3. **CCPA**: Privacy policy, right to delete, right to know
4. **ISO 27001**: Information security policy, risk assessment, security controls

**Compliance Strategy**:
- ✅ Use Azure OpenAI (EU region) for GDPR compliance
- ✅ Enable approval gates (review prompts before sending)
- ✅ Evidence repository (complete audit trail)
- ✅ Data deletion after 90 days
- ⚠️ No encryption at rest (future enhancement)
- ⚠️ No automatic PII detection (manual redaction required)

**Vendor Recommendations**:
- **GDPR**: Azure OpenAI (EU region)
- **SOC 2**: Azure OpenAI
- **HIPAA**: Azure OpenAI (with BAA)
- **Complete Privacy**: Ollama (local models)

**Gaps**:
- ❌ No encryption at rest
- ❌ No multi-user access controls
- ❌ No automatic PII detection
- ❌ No formal incident response plan

**Next**: [Security Best Practices](security-best-practices.md)
