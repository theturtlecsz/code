# Example Workflow: Complete OAuth Implementation

This document walks through a complete Spec-Kit workflow for implementing OAuth authentication, showing all stages, quality gates, and artifacts produced.

## Scenario

**Feature Request**: Add OAuth2 authentication with Google and GitHub providers to an existing application.

---

## Stage 1: Create SPEC

### Command

```bash
/speckit.new Add OAuth2 authentication supporting Google and GitHub providers
```

### Output

```
✓ Created SPEC-KIT-065
✓ Directory: docs/SPEC-KIT-065-oauth-authentication/
✓ Generated spec.md with template
✓ Updated SPEC.md tracker

Cost: $0.00
Time: 0.3 seconds
```

### Generated Artifact: `spec.md`

```markdown
# SPEC-KIT-065: OAuth Authentication

## Summary
Add OAuth2 authentication supporting Google and GitHub providers

## Requirements

### Functional Requirements
1. Users can sign in with Google OAuth2
2. Users can sign in with GitHub OAuth2
3. OAuth tokens are securely stored
4. Sessions persist across browser restarts
5. Users can logout and clear session

### Non-Functional Requirements
1. Authentication flow completes in < 3 seconds
2. Token storage meets security best practices
3. Support for token refresh without re-authentication

### Acceptance Criteria
- [ ] Google OAuth login/logout works end-to-end
- [ ] GitHub OAuth login/logout works end-to-end
- [ ] Tokens stored encrypted at rest
- [ ] Session survives page refresh
- [ ] Logout clears all auth state

### Examples
[To be filled during planning]

### Edge Cases
[To be filled during planning]
```

---

## Quality Gate: Pre-Planning

### Clarify Gate

```bash
# Automatically runs before planning
[CLARIFY GATE] Analyzing spec.md for ambiguities...

⚠ 3 clarifications needed:

1. Token storage approach:
   Requirement says "securely stored" but doesn't specify method.
   Options:
   [ ] Encrypted vault (HashiCorp Vault)
   [ ] Database with field encryption
   [ ] Cloud KMS (AWS/GCP)
   [ ] Browser secure storage (for tokens)

2. Session timeout:
   "Sessions persist" - for how long?
   [____] hours (recommended: 24-168)

3. Token refresh strategy:
   When should tokens refresh?
   [ ] On expiry (reactive)
   [ ] Before expiry (proactive, recommended)
   [ ] On each request (aggressive)

[Submit Answers]
```

### User Responses

```
1. Token storage: Encrypted vault
2. Session timeout: 72 hours
3. Token refresh: Before expiry (proactive)
```

### Clarify Gate Result

```
✓ Answers applied to spec.md
✓ 3 ambiguities resolved

Updated sections:
- "Tokens stored in encrypted vault"
- "Sessions timeout after 72 hours of inactivity"
- "Tokens refresh proactively 5 minutes before expiry"
```

### Checklist Gate

```bash
[CHECKLIST GATE] Scoring requirement quality...

| Dimension    | Score | Status |
|--------------|-------|--------|
| Completeness | 8/10  | ✓ PASS |
| Clarity      | 9/10  | ✓ PASS |
| Testability  | 8/10  | ✓ PASS |
| Consistency  | 9/10  | ✓ PASS |

Overall: 8.5/10 - PASS

✓ Quality gate passed, proceeding to planning
```

---

## Stage 2: Plan

### Command

```bash
/speckit.plan SPEC-KIT-065
```

### Execution

```
[PLANNING] Spawning 3 agents...
├─ gemini-flash: analyzing requirements...
├─ claude-haiku: checking edge cases...
└─ gpt5-medium: structuring plan...

[10:15:00] gemini-flash: COMPLETE
[10:16:30] claude-haiku: COMPLETE
[10:17:00] gpt5-medium: COMPLETE

[CONSENSUS] Synthesizing results...
✓ Agreement: 3/3 on core architecture
✓ Minor disagreement resolved: Error handling approach

Cost: $0.37
Time: 11 minutes
```

### Generated Artifact: `plan.md`

```markdown
# Plan: SPEC-KIT-065 OAuth Authentication

## Inputs
- Spec: docs/SPEC-KIT-065/spec.md (hash: 7a3b2c1)

## Work Breakdown

### Phase 1: Core Infrastructure
1. Create OAuth configuration module
   - Provider configs (Google, GitHub)
   - Redirect URI handling
   - Client ID/secret management

2. Implement token storage
   - Vault integration for encrypted storage
   - Token model (access, refresh, expiry)
   - Encryption at rest

### Phase 2: OAuth Flows
3. Google OAuth implementation
   - Authorization URL generation
   - Callback handler
   - Token exchange

4. GitHub OAuth implementation
   - Authorization URL generation
   - Callback handler
   - Token exchange

### Phase 3: Session Management
5. Session handling
   - Session creation on login
   - Session persistence (72h timeout)
   - Session cleanup on logout

6. Token refresh mechanism
   - Proactive refresh (5 min before expiry)
   - Refresh failure handling
   - Re-authentication trigger

### Phase 4: Security Hardening
7. Security measures
   - CSRF protection
   - State parameter validation
   - Secure cookie settings

## Acceptance Mapping

| Requirement | Validation Step | Test Artifact |
|-------------|-----------------|---------------|
| R1: Google OAuth | E2E test flow | tests/e2e/google_oauth_test.rs |
| R2: GitHub OAuth | E2E test flow | tests/e2e/github_oauth_test.rs |
| R3: Token storage | Unit test encryption | tests/unit/token_storage_test.rs |
| R4: Session persist | Integration test | tests/integration/session_test.rs |
| R5: Logout | Unit test cleanup | tests/unit/logout_test.rs |

## Risks & Unknowns

### Technical Risks
- Vault availability: May need local fallback for development
- Provider API changes: Pin to specific API versions

### Mitigation
- Implement circuit breaker for Vault
- Add API version headers to OAuth requests

## Consensus Notes

### Agreement (3/3)
- Vault for token storage
- Proactive refresh strategy
- CSRF protection required

### Resolved Disagreement
- Error handling approach:
  - Gemini: Log and retry
  - Claude: Fail fast with user notification
  - GPT-5: Retry with exponential backoff
  - **Resolution**: Retry with backoff, notify user after 3 failures
```

---

## Quality Gate: Post-Plan

### Analyze Gate

```bash
[ANALYZE GATE] Checking plan ↔ spec consistency...

✓ All 5 requirements have plan items
✓ All plan items have validation steps
✓ No contradictions found

Coverage: 100%
Status: PASS
```

---

## Stage 3: Tasks

### Command

```bash
/speckit.tasks SPEC-KIT-065
```

### Execution

```
[TASKS] Decomposing plan into tasks...
├─ gpt5-low: generating task breakdown...

[10:20:00] gpt5-low: COMPLETE

Cost: $0.10
Time: 4 minutes
```

### Generated Artifact: `tasks.md`

```markdown
# Tasks: SPEC-KIT-065 OAuth Authentication

## Task Breakdown

| Order | Task ID | Title | Status | Validation | Est. Time |
|-------|---------|-------|--------|------------|-----------|
| 1 | T1 | Create OAuth config struct | Backlog | Unit test config parsing | 1h |
| 2 | T2 | Implement Vault integration | Backlog | Integration test storage | 2h |
| 3 | T3 | Create token model | Backlog | Unit test token ops | 1h |
| 4 | T4 | Google OAuth - auth URL | Backlog | Unit test URL generation | 1h |
| 5 | T5 | Google OAuth - callback | Backlog | Integration test flow | 2h |
| 6 | T6 | GitHub OAuth - auth URL | Backlog | Unit test URL generation | 1h |
| 7 | T7 | GitHub OAuth - callback | Backlog | Integration test flow | 2h |
| 8 | T8 | Session management | Backlog | Integration test persist | 2h |
| 9 | T9 | Token refresh mechanism | Backlog | Unit test proactive refresh | 2h |
| 10 | T10 | CSRF protection | Backlog | Security test | 1h |
| 11 | T11 | E2E test suite | Backlog | Full flow tests | 3h |

## Dependencies

```
T1 (config) → T4, T6 (OAuth URLs)
T2 (vault) → T3 (token model)
T3 (token) → T5, T7 (callbacks)
T5, T7 (callbacks) → T8 (session)
T8 (session) → T9 (refresh)
T1-T10 → T11 (E2E tests)
```

## Estimated Total: 18 hours
```

---

## Quality Gate: Post-Tasks

### Analyze Gate

```bash
[ANALYZE GATE] Checking task ↔ requirement coverage...

✓ Every requirement has corresponding task(s):
  - R1 (Google OAuth): T4, T5
  - R2 (GitHub OAuth): T6, T7
  - R3 (Token storage): T2, T3
  - R4 (Session persist): T8
  - R5 (Logout): T8 (includes cleanup)

✓ All tasks have validation steps
✓ Dependencies are logical

Coverage: 100%
Status: PASS
```

---

## Stage 4: Implement

### Command

```bash
/speckit.implement SPEC-KIT-065
```

### Execution

```
[IMPLEMENT] Generating code...
├─ gpt_codex (HIGH): writing implementation...
├─ claude-haiku: validating edge cases...

[10:30:00] gpt_codex: COMPLETE (primary code)
[10:32:00] claude-haiku: COMPLETE (validation)

[VALIDATION] Running code checks...
├─ cargo fmt: ✓ formatted
├─ cargo clippy: ✓ no warnings
├─ cargo build: ✓ success
├─ cargo test: ✓ 12/12 passing

Cost: $0.11
Time: 9 minutes
```

### Generated Artifact: `implementation.md`

```markdown
# Implementation: SPEC-KIT-065 OAuth Authentication

## Files Created

### src/auth/config.rs
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub provider: OAuthProvider,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OAuthProvider {
    Google,
    GitHub,
}

impl OAuthConfig {
    pub fn authorization_url(&self, state: &str) -> String {
        match self.provider {
            OAuthProvider::Google => format!(
                "https://accounts.google.com/o/oauth2/v2/auth?\
                client_id={}&redirect_uri={}&response_type=code&\
                scope={}&state={}",
                self.client_id,
                self.redirect_uri,
                self.scopes.join(" "),
                state
            ),
            OAuthProvider::GitHub => format!(
                "https://github.com/login/oauth/authorize?\
                client_id={}&redirect_uri={}&scope={}&state={}",
                self.client_id,
                self.redirect_uri,
                self.scopes.join(" "),
                state
            ),
        }
    }
}
```

### src/auth/token.rs
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub token_type: String,
}

impl AuthToken {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    pub fn needs_refresh(&self) -> bool {
        // Proactive refresh 5 minutes before expiry
        let refresh_threshold = self.expires_at - chrono::Duration::minutes(5);
        Utc::now() >= refresh_threshold
    }
}
```

[... additional implementation files ...]

## Validation Notes (Claude Haiku)

### Security Review
✓ No secrets logged
✓ CSRF state validation present
✓ Secure cookie settings

### Edge Cases Covered
✓ Token expiry during request
✓ Refresh token missing
✓ Network timeout handling

### Recommendations
⚠ Consider adding rate limiting for auth endpoints
⚠ Add telemetry for auth failures
```

---

## Stage 5: Validate

### Command

```bash
/speckit.validate SPEC-KIT-065
```

### Execution

```
[VALIDATE] Creating test strategy...
├─ gemini-flash: analyzing coverage...
├─ claude-haiku: edge case scenarios...
├─ gpt5-medium: test structure...

[10:45:00] All agents: COMPLETE

Cost: $0.35
Time: 10 minutes
```

### Generated Artifact: `validation-report.md`

```markdown
# Validation Report: SPEC-KIT-065 OAuth Authentication

## Test Scenarios

### Happy Path
| Scenario | Status | Test File |
|----------|--------|-----------|
| Google OAuth complete flow | ✓ Pass | tests/e2e/google_oauth_test.rs |
| GitHub OAuth complete flow | ✓ Pass | tests/e2e/github_oauth_test.rs |
| Session persists on reload | ✓ Pass | tests/integration/session_test.rs |
| Logout clears all state | ✓ Pass | tests/unit/logout_test.rs |
| Token refresh before expiry | ✓ Pass | tests/unit/refresh_test.rs |

### Edge Cases
| Scenario | Status | Test File |
|----------|--------|-----------|
| Expired token during request | ✓ Pass | tests/unit/token_test.rs |
| Missing refresh token | ✓ Pass | tests/unit/token_test.rs |
| Invalid CSRF state | ✓ Pass | tests/security/csrf_test.rs |
| Concurrent refresh attempts | ✓ Pass | tests/integration/concurrent_test.rs |
| Network timeout on OAuth | ✓ Pass | tests/integration/timeout_test.rs |

### Error Handling
| Scenario | Status | Test File |
|----------|--------|-----------|
| Invalid OAuth callback | ✓ Pass | tests/error/callback_test.rs |
| Vault unavailable | ✓ Pass | tests/error/vault_test.rs |
| Provider rate limiting | ✓ Pass | tests/error/rate_limit_test.rs |

## Coverage Analysis

- **Line coverage**: 94%
- **Branch coverage**: 87%
- **Requirement coverage**: 100%

### Gaps Identified
- No test for malformed tokens from provider
- Missing load test for concurrent sessions

### Recommendations
1. Add property-based tests for token parsing
2. Add load test with 100 concurrent OAuth flows
```

---

## Stage 6: Audit

### Command

```bash
/speckit.audit SPEC-KIT-065
```

### Execution

```
[AUDIT] Security and compliance review...
├─ gemini-pro: security analysis...
├─ claude-sonnet: compliance check...
├─ gpt5-high: production readiness...

[11:00:00] All premium agents: COMPLETE

Cost: $0.80
Time: 11 minutes
```

### Generated Artifact: `audit-report.md`

```markdown
# Audit Report: SPEC-KIT-065 OAuth Authentication

## Security Assessment

### OWASP Top 10 Coverage

| Vulnerability | Status | Notes |
|---------------|--------|-------|
| Injection | ✓ Safe | Parameterized queries |
| Broken Auth | ✓ Safe | Proper token validation |
| Sensitive Data | ✓ Safe | Vault encryption |
| XXE | N/A | No XML processing |
| Broken Access | ✓ Safe | Session validation |
| Security Misconfig | ✓ Safe | Secure defaults |
| XSS | ✓ Safe | No user input reflection |
| Insecure Deserial | ✓ Safe | Typed deserialization |
| Components | ⚠ Review | Check OAuth lib versions |
| Logging | ✓ Safe | No secrets in logs |

### Findings

**Critical**: None

**High**: None

**Medium**:
1. OAuth library version should be pinned
   - Recommendation: Add version constraints in Cargo.toml

**Low**:
1. Consider adding request signing for Vault calls
2. Add timeout on OAuth callback endpoint

## Compliance

### Standards Checked
- OWASP ASVS Level 2: ✓ Compliant
- OAuth 2.0 RFC 6749: ✓ Compliant
- PKCE (RFC 7636): ⚠ Not implemented

### Recommendation
Add PKCE support for enhanced security (not blocking for launch)

## Production Readiness

| Criterion | Status |
|-----------|--------|
| Error handling | ✓ Complete |
| Logging | ✓ Complete |
| Monitoring hooks | ✓ Complete |
| Rollback plan | ⚠ Document needed |
| Load tested | ⚠ Pending |

## Verdict

**APPROVED FOR LAUNCH** with conditions:
1. Pin OAuth library version
2. Document rollback plan
3. Complete load testing
```

---

## Stage 7: Unlock

### Command

```bash
/speckit.unlock SPEC-KIT-065
```

### Execution

```
[UNLOCK] Final ship decision...
├─ gemini-pro: reviewing all artifacts...
├─ claude-sonnet: final security check...
├─ gpt5-high: ship decision...

[11:15:00] All premium agents: COMPLETE

[CONSENSUS] Ship decision...
✓ Agreement: 3/3 APPROVE

Cost: $0.80
Time: 10 minutes
```

### Final Decision

```markdown
# Ship Decision: SPEC-KIT-065 OAuth Authentication

## Decision: ✅ APPROVED TO SHIP

## Rationale

### Quality Assessment
- All acceptance criteria met
- Test coverage: 94% line, 87% branch
- No critical or high security issues
- 100% requirement coverage

### Conditions Met
- ✓ OAuth library version pinned (Cargo.toml updated)
- ✓ Rollback plan documented (ROLLBACK.md added)
- ✓ Load test passed (100 concurrent flows)

### Risk Assessment
- Low risk: Well-tested, follows standards
- Mitigations in place for known risks

## Post-Launch Actions

1. Monitor auth failure rates for first 24 hours
2. Review Vault performance under load
3. Schedule PKCE implementation for next sprint

## Approvals

| Agent | Decision | Confidence |
|-------|----------|------------|
| gemini-pro | SHIP | 0.92 |
| claude-sonnet | SHIP | 0.88 |
| gpt5-high | SHIP | 0.90 |

**Consensus**: UNANIMOUS SHIP
```

---

## Summary

### Total Pipeline Execution

| Stage | Cost | Time |
|-------|------|------|
| New | $0.00 | <1s |
| Clarify Gate | $0.00 | <1s |
| Checklist Gate | $0.00 | <1s |
| Plan | $0.37 | 11 min |
| Analyze Gate | $0.00 | <1s |
| Tasks | $0.10 | 4 min |
| Analyze Gate | $0.00 | <1s |
| Implement | $0.11 | 9 min |
| Validate | $0.35 | 10 min |
| Audit | $0.80 | 11 min |
| Unlock | $0.80 | 10 min |
| **Total** | **$2.53** | **~55 min** |

### Artifacts Produced

```
docs/SPEC-KIT-065-oauth-authentication/
├── spec.md
├── plan.md
├── tasks.md
├── implementation.md
├── validation-report.md
├── audit-report.md
└── ROLLBACK.md

Evidence:
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-065/
├── plan-*.json (3 agent outputs + synthesis)
├── tasks-*.json
├── implement-*.json
├── validate-*.json
├── audit-*.json
├── unlock-*.json
└── telemetry.json
```

### Quality Metrics

- Requirements coverage: 100%
- Test coverage: 94% line, 87% branch
- Security issues: 0 critical, 0 high, 1 medium
- Agent agreement: Unanimous on ship decision
