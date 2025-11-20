# NEXT SESSION PROMPT: SPEC-KIT-951 Multi-Provider OAuth Research

**Session Objective**: Complete research phase for multi-provider OAuth architecture and produce GO/NO-GO decision.

**Estimated Time**: 9-14 hours (1-2 full days)

---

## SESSION START PROMPT

Copy and paste this into your next Claude Code session:

```
I'm working on SPEC-KIT-951: Multi-Provider OAuth Research & Architecture Validation.

This is a RESEARCH SPEC to validate the technical feasibility of multi-provider OAuth
(ChatGPT, Claude, Gemini) BEFORE implementation begins.

CONTEXT:
- Current state: SPEC-KIT-946 expanded /model command to show all 13 models
- Problem: AuthMode enum only supports ChatGPT OAuth, users can't access Claude/Gemini
- Goal: Research whether multi-provider OAuth is feasible and how to implement it

I need to complete 6 research objectives (RO1-RO6) and produce:
1. Research Summary Report with GO/NO-GO decision
2. Technical Architecture Diagrams
3. Implementation Readiness Checklist

Read the PRD at: docs/SPEC-KIT-951-multi-provider-oauth-research/PRD.md
Read the spec at: docs/SPEC-KIT-951-multi-provider-oauth-research/spec.md

Let's start with RO1: OAuth Credential Acquisition (CRITICAL).

Research question: How do we obtain official OAuth client credentials for Claude and Gemini?

Tasks:
1. Investigate Anthropic Developer Portal for Claude OAuth app registration
2. Investigate Google Cloud Console for Gemini OAuth 2.0 credentials
3. Document credential acquisition process, timeline, costs, and blockers
4. Identify alternative approaches (user-provided credentials, API keys)

Please use WebSearch to research:
- Anthropic OAuth documentation and developer portal
- Google Cloud OAuth setup for Gemini/PaLM API
- Any existing examples of desktop apps using Claude/Gemini OAuth

Document your findings in a structured format and let me know what you discover.
```

---

## RESEARCH WORKFLOW (Execute in Order)

### Phase 1: OAuth Credential Acquisition (RO1) - 2-3 hours

**Critical Questions to Answer**:
1. Does Anthropic provide OAuth for Claude API access?
2. How do we register an OAuth app with Anthropic?
3. Does Google provide OAuth for Gemini API access?
4. What's the approval process and timeline?
5. Are there costs or partnership requirements?

**Research Actions**:
- [ ] Search: "Anthropic Claude API OAuth authentication"
- [ ] Search: "Google Gemini API OAuth 2.0 desktop app"
- [ ] Search: "Anthropic developer portal OAuth app registration"
- [ ] Search: "Google Cloud Console OAuth credentials Gemini"
- [ ] Check: docs.anthropic.com for OAuth documentation
- [ ] Check: ai.google.dev for OAuth documentation
- [ ] Search GitHub: "Claude OAuth desktop app" (look for examples)
- [ ] Search GitHub: "Gemini OAuth desktop app" (look for examples)

**Document In**: `RESEARCH-REPORT.md` Section 2 (OAuth Credential Acquisition)

**Decision Point**: Can we get credentials? If NO → consider API key fallback or NO-GO

---

### Phase 2: OAuth Flow Specifications (RO2) - 2-3 hours

**Critical Questions to Answer**:
1. What are the exact OAuth 2.0 endpoints for Claude?
2. What are the exact OAuth 2.0 endpoints for Gemini?
3. Is PKCE required or optional?
4. What scopes are needed?
5. What are token lifetimes and refresh requirements?

**Research Actions**:
- [ ] Document ChatGPT OAuth flow (baseline from existing code: `core/src/auth.rs`, `login/src/server.rs`)
- [ ] Search: "Claude API OAuth 2.0 endpoints authorization token"
- [ ] Search: "Gemini API OAuth 2.0 PKCE flow"
- [ ] Search: "Anthropic API authentication methods"
- [ ] Search: "Google generative AI API OAuth scopes"
- [ ] Create OAuth Flow Comparison Matrix (see PRD template)

**Document In**: `RESEARCH-REPORT.md` Section 3 (OAuth Flow Specifications)

**Deliverable**: Comparison matrix with all provider specifications

---

### Phase 3: Security Architecture (RO3) - 2-3 hours

**Critical Questions to Answer**:
1. How should we store OAuth tokens securely?
2. Platform keychain vs encrypted file - which is better?
3. What Rust libraries are available?
4. What's the threat model for desktop OAuth apps?
5. Any compliance considerations (GDPR, data protection)?

**Research Actions**:
- [ ] Search: "Rust keyring crate platform keychain integration"
- [ ] Search: "OAuth token storage best practices desktop apps"
- [ ] Search: "Rust encryption libraries ring sodiumoxide"
- [ ] Search: "desktop app token security threat model"
- [ ] Evaluate: `keyring` crate documentation (docs.rs/keyring)
- [ ] Evaluate: `oauth2` crate security features (docs.rs/oauth2)
- [ ] Document threat model (file access, memory leakage, transmission)

**Document In**: `RESEARCH-REPORT.md` Section 4 (Security Architecture)

**Deliverable**: Security architecture recommendation with threat model

---

### Phase 4: Token Management Strategy (RO4) - 1-2 hours

**Critical Questions to Answer**:
1. When should we refresh tokens (background, lazy, expiry-based)?
2. How to coordinate refresh across multiple providers?
3. What error scenarios need handling?
4. How to notify users of auth issues?

**Research Actions**:
- [ ] Search: "OAuth token refresh strategy best practices"
- [ ] Search: "multi-provider OAuth token management"
- [ ] Design refresh strategy (recommend one approach with rationale)
- [ ] Map error scenarios (network failure, revoked token, rate limits)
- [ ] Design user notification approach

**Document In**: `RESEARCH-REPORT.md` Section 5 (Token Management Strategy)

**Deliverable**: Token management design with refresh strategy and error handling

---

### Phase 5: Provider-Specific Requirements (RO5) - 1-2 hours

**Critical Questions to Answer**:
1. What are OAuth endpoint rate limits for each provider?
2. Are there redirect URI restrictions?
3. Which models require OAuth vs API keys?
4. Any geographic restrictions?

**Research Actions**:
- [ ] Search: "Anthropic API rate limits OAuth"
- [ ] Search: "Google Gemini API rate limits quotas"
- [ ] Search: "OAuth localhost redirect URI restrictions"
- [ ] Document rate limits comparison
- [ ] Document authentication quirks
- [ ] Create model availability matrix

**Document In**: `RESEARCH-REPORT.md` Section 6 (Provider Requirements)

**Deliverable**: Provider requirements matrix with rate limits and quirks

---

### Phase 6: Reference Implementation Analysis (RO6) - 1-2 hours

**Critical Questions to Answer**:
1. What patterns do successful multi-provider apps use?
2. Are there Rust OAuth examples we can adapt?
3. What UX patterns work well for provider switching?

**Research Actions**:
- [ ] Search GitHub: "rust oauth2 pkce desktop" (code examples)
- [ ] Search: "multi-provider authentication architecture patterns"
- [ ] Search: "desktop app oauth ux patterns provider switching"
- [ ] Identify 2-3 reference implementations
- [ ] Document recommended patterns
- [ ] Note anti-patterns to avoid

**Document In**: `RESEARCH-REPORT.md` Section 7 (Reference Implementations)

**Deliverable**: Architecture patterns document with code examples

---

## FINAL DELIVERABLES

### 1. Research Summary Report

**File**: `docs/SPEC-KIT-951-multi-provider-oauth-research/RESEARCH-REPORT.md`

**Template Structure**:
```markdown
# Multi-Provider OAuth Research Report

## Executive Summary
- GO/NO-GO Recommendation: [GO | NO-GO | CONDITIONAL]
- Rationale: [1-2 paragraphs]
- Timeline to Implementation: [estimate]
- Critical Blockers: [list]

## 1. OAuth Credential Acquisition (RO1)
### Findings
- Anthropic: [process, timeline, costs, blockers]
- Google: [process, timeline, costs, blockers]

### Recommendation
[Clear path forward or alternatives]

## 2. OAuth Flow Specifications (RO2)
### OAuth Flow Comparison Matrix
| Aspect | ChatGPT | Claude | Gemini |
|--------|---------|--------|--------|
| Auth URL | ... | ... | ... |
| Token URL | ... | ... | ... |
| PKCE Required | ... | ... | ... |
| Scopes | ... | ... | ... |
| Token Lifetime | ... | ... | ... |

### Recommendation
[Can we implement these flows?]

## 3. Security Architecture (RO3)
### Threat Model
[What attacks are we defending against?]

### Recommended Approach
- Token Storage: [keychain | encrypted file | hybrid]
- Encryption: [algorithm, key derivation]
- Libraries: [keyring, secrecy, etc.]

## 4. Token Management Strategy (RO4)
### Recommended Strategy
[Background | Lazy | Expiry-based]

### Error Handling
[Flowchart or decision tree]

## 5. Provider Requirements (RO5)
### Rate Limits
[Comparison matrix]

### Authentication Quirks
[Provider-specific notes]

## 6. Reference Implementations (RO6)
### Recommended Patterns
[List 3-5 patterns with examples]

### Anti-Patterns to Avoid
[List 2-3 anti-patterns]

## 7. Open Questions & Risks
[Remaining unknowns]

## 8. Recommended Implementation Approach
[Step-by-step if GO decision]

## 9. Next Steps
- If GO: Create SPEC-952 (Implementation)
- If NO-GO: Document alternatives
```

---

### 2. Technical Architecture Diagrams

**File**: `docs/SPEC-KIT-951-multi-provider-oauth-research/ARCHITECTURE-DIAGRAMS.md`

**Include**:
- Component diagram (AuthManager, OAuth flows, token storage)
- Sequence diagram (user selects model → auth switches)
- State diagram (token lifecycle: fresh → active → expiring → refreshed → expired)
- Error handling flowchart

**Use Mermaid diagrams** for clarity

---

### 3. Implementation Readiness Checklist

**File**: `docs/SPEC-KIT-951-multi-provider-oauth-research/IMPLEMENTATION-READINESS.md`

**Checklist**:
```markdown
# Implementation Readiness Checklist

## OAuth Credentials
- [ ] Claude OAuth credentials obtainable (process documented)
- [ ] Gemini OAuth credentials obtainable (process documented)
- [ ] Credential acquisition timeline acceptable (<2 weeks)

## Technical Feasibility
- [ ] OAuth flow specifications fully documented
- [ ] PKCE requirements confirmed for all providers
- [ ] Token refresh strategy defined
- [ ] Security approach validated
- [ ] Token storage library selected

## Architecture Validation
- [ ] AuthManager design approved
- [ ] Multi-provider coordination pattern defined
- [ ] Error scenarios mapped
- [ ] Provider quirks documented

## GO/NO-GO Decision
- [ ] All critical blockers resolved OR
- [ ] Alternative approach defined (API keys, partial implementation)
- [ ] Security review completed
- [ ] Effort estimate validated (19-29h for implementation)

## Final Decision
**Decision**: [ ] GO (proceed to SPEC-952) | [ ] NO-GO (alternatives only) | [ ] CONDITIONAL (list conditions)

**Rationale**: [1-2 paragraphs explaining the decision]
```

---

## SUCCESS CRITERIA

### Must Have (GO Criteria)
- ✅ Clear path to OAuth credentials (timeline <2 weeks OR user-provided approach validated)
- ✅ OAuth flows fully documented (endpoints, PKCE, scopes, token lifetime)
- ✅ Security approach approved (threat model, storage method, libraries)

### Should Have (Quality Criteria)
- ✅ Provider quirks identified (rate limits, authentication quirks, workarounds)
- ✅ Reference implementations found (code examples, patterns documented)

### Could Have (Nice to Have)
- ⭕ Hands-on OAuth testing (test apps registered, flows validated)

---

## NEXT STEPS AFTER RESEARCH

### If GO Decision
1. **Create SPEC-KIT-952-multi-provider-oauth-implementation**
   - Use validated research findings
   - Include proven architecture patterns
   - Reference security recommendations

2. **Update SPEC-KIT-947** (Master Validation)
   - Refine validation criteria based on research
   - Add provider-specific test scenarios

3. **Begin implementation** with confidence (all blockers resolved)

---

### If NO-GO Decision
1. **Document reasons** (credential acquisition blocked, provider limitations, etc.)

2. **Recommend alternatives**:
   - API key approach for Claude/Gemini
   - Single-provider OAuth (ChatGPT only)
   - Partial implementation (ChatGPT OAuth + API keys)

3. **Close or rescope** SPEC-KIT-947 accordingly

---

## TIPS FOR EFFICIENT RESEARCH

### Web Search Strategy
1. Start with official documentation (docs.anthropic.com, ai.google.dev)
2. Search GitHub for real-world examples ("Claude OAuth", "Gemini OAuth desktop")
3. Check developer forums (Reddit r/MachineLearning, HN)
4. Look for Rust crate documentation (docs.rs)

### Documentation Strategy
1. **Answer questions first** (bullet points, rough notes)
2. **Organize findings** into report structure
3. **Add diagrams** to clarify complex flows
4. **Complete checklist** to validate readiness

### Time Management
- **Phase 1-2** (Credentials + Flows): 4-6 hours → Most critical, front-load effort
- **Phase 3-4** (Security + Token Mgmt): 3-5 hours → Can be done in parallel
- **Phase 5-6** (Provider + References): 2-4 hours → Fill in details, validate assumptions

---

## DONE CRITERIA

You're done when:
1. ✅ Research Summary Report complete with GO/NO-GO decision
2. ✅ Architecture Diagrams created
3. ✅ Implementation Readiness Checklist complete
4. ✅ All 6 research objectives (RO1-RO6) answered
5. ✅ Clear path forward documented (either implementation or alternatives)

---

**Ready to start? Use the SESSION START PROMPT above to begin your research session!**
