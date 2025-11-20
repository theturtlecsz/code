# PRD: Multi-Provider OAuth Research & Architecture Validation

**SPEC-ID**: SPEC-KIT-951
**Status**: Backlog
**Created**: 2025-11-19
**Author**: Research Phase
**Priority**: P0 - CRITICAL (Blocks Implementation)
**Type**: RESEARCH SPEC

---

## Executive Summary

Research and validate the technical feasibility of multi-provider OAuth architecture for ChatGPT, Claude, and Gemini. This research phase will answer critical questions about OAuth credential acquisition, security patterns, token management, and provider-specific requirements BEFORE implementation begins.

**Success Criteria**: Comprehensive research report with clear GO/NO-GO decision on implementation approach, documented OAuth flows, credential acquisition process, and validated security patterns.

**Downstream Impact**: Informs SPEC-KIT-952 (Implementation) and validates SPEC-KIT-947 (Master Validation)

---

## Problem Statement

### Current Knowledge Gaps

**What We Don't Know**:
1. How to obtain OAuth client credentials for Claude and Gemini
2. Exact OAuth 2.0 flow requirements for each provider
3. Token storage security best practices for multi-provider scenario
4. Rate limits and API quotas for OAuth endpoints
5. Provider-specific authentication quirks or limitations
6. Whether platform keychain integration is feasible/necessary
7. Token refresh timing strategies across providers
8. Error scenarios and fallback behaviors

**Why This Matters**:
- Cannot implement without OAuth credentials
- Security mistakes in auth layer are critical vulnerabilities
- Provider-specific quirks could invalidate architecture assumptions
- Wrong token storage approach could compromise user security

**Impact of Not Researching First**:
- Build incorrect OAuth implementation → security vulnerabilities
- Discover credential roadblocks mid-implementation → wasted effort
- Miss provider-specific requirements → broken auth flows
- Choose wrong architecture patterns → costly refactor

---

## Research Objectives

### RO1: OAuth Credential Acquisition (CRITICAL)

**Question**: How do we obtain official OAuth client credentials for Claude and Gemini?

**Research Tasks**:
1. Investigate Anthropic Developer Portal
   - Does OAuth app registration exist?
   - What are the requirements (partnership, verification, etc.)?
   - Are there public OAuth apps or must we create our own?
   - Cost/approval process timeline

2. Investigate Google Cloud Console for Gemini
   - OAuth 2.0 credential creation process
   - OAuth consent screen approval requirements
   - API quota limits and costs
   - Required scopes for Gemini API access

3. Alternative Approaches
   - Can users provide their own OAuth credentials?
   - Are there developer sandboxes or test credentials?
   - What do other multi-provider apps do?

**Deliverable**: Document with:
- Step-by-step credential acquisition process for each provider
- Timeline estimates (hours, days, weeks?)
- Cost implications
- Blocker identification (partnerships required, manual approval, etc.)
- GO/NO-GO recommendation

---

### RO2: OAuth Flow Specifications (HIGH)

**Question**: What are the exact OAuth 2.0 flow requirements for Claude and Gemini?

**Research Tasks**:
1. Claude OAuth Documentation
   - Authorization endpoint URL
   - Token endpoint URL
   - Required scopes
   - PKCE requirement (mandatory or optional?)
   - Redirect URI requirements
   - Token format and lifetime
   - Refresh token behavior

2. Gemini OAuth Documentation
   - Authorization endpoint URL
   - Token endpoint URL
   - Required scopes (`generative-language` confirmed?)
   - PKCE requirement
   - Redirect URI requirements
   - Token format and lifetime
   - Refresh token behavior

3. ChatGPT OAuth (Baseline Comparison)
   - Document existing implementation as reference
   - Identify what patterns can be reused
   - Note any provider-specific differences

**Deliverable**: OAuth Flow Comparison Matrix
```
| Aspect              | ChatGPT | Claude | Gemini |
|---------------------|---------|--------|--------|
| Auth URL            | ...     | ...    | ...    |
| Token URL           | ...     | ...    | ...    |
| PKCE Required       | ...     | ...    | ...    |
| Scopes              | ...     | ...    | ...    |
| Token Lifetime      | ...     | ...    | ...    |
| Refresh Strategy    | ...     | ...    | ...    |
```

---

### RO3: Security Architecture (HIGH)

**Question**: What are the security best practices for multi-provider OAuth token storage?

**Research Tasks**:
1. Token Storage Options
   - Platform keychain integration (macOS Keychain, Linux Secret Service, Windows Credential Manager)
   - File-based encryption (what algorithms? key derivation?)
   - Hybrid approach (keychain primary, encrypted file fallback)
   - Industry best practices for desktop OAuth apps

2. Security Threat Model
   - What attacks are we defending against?
   - File system access protection
   - Memory security (token leakage)
   - Token transmission security

3. Rust Security Libraries
   - `keyring` crate evaluation
   - `secrecy` crate for in-memory protection
   - Encryption libraries (ring, sodiumoxide, etc.)
   - OAuth 2.0 client libraries with security features

**Deliverable**: Security Architecture Document
- Threat model
- Recommended token storage approach
- Required Rust dependencies
- Security checklist for implementation
- Compliance considerations (GDPR, data protection)

---

### RO4: Token Management Strategy (MEDIUM)

**Question**: How should we handle token refresh, expiry, and lifecycle across multiple providers?

**Research Tasks**:
1. Token Refresh Patterns
   - Background refresh (all providers daily?)
   - Lazy refresh (on provider switch?)
   - Expiry-based refresh (when <24h remaining?)
   - Proactive vs reactive strategies

2. Multi-Provider Coordination
   - How to avoid refresh storms (all providers at once)
   - Retry logic for failed refreshes
   - Graceful degradation (one provider down, others work)
   - User notification strategies

3. Error Scenarios
   - Network failures during refresh
   - Invalid/revoked refresh tokens
   - Provider rate limiting
   - Concurrent refresh requests

**Deliverable**: Token Management Design
- Refresh strategy decision with rationale
- Error handling flowchart
- User notification mockups
- Performance considerations (refresh overhead)

---

### RO5: Provider-Specific Requirements (MEDIUM)

**Question**: What provider-specific quirks or requirements exist that could impact architecture?

**Research Tasks**:
1. API Rate Limits
   - OAuth endpoint rate limits (token requests, refresh requests)
   - Model API rate limits (impact on usage patterns)
   - How rate limits affect token refresh strategy

2. Authentication Quirks
   - Redirect URI restrictions (localhost, custom protocols, etc.)
   - Browser integration requirements
   - State parameter requirements
   - Custom headers or parameters

3. Model Availability
   - Which models require OAuth vs API keys?
   - Are all models available with OAuth?
   - Any geographic restrictions?
   - Beta/preview access requirements

**Deliverable**: Provider Requirements Matrix
- Rate limit comparison
- Authentication requirements comparison
- Model availability matrix
- Workaround strategies for limitations

---

### RO6: Reference Implementation Analysis (LOW)

**Question**: What can we learn from existing multi-provider authentication apps?

**Research Tasks**:
1. Identify Reference Apps
   - Desktop apps with multi-provider OAuth (Slack, Discord, etc.)
   - Open-source OAuth implementations
   - Rust OAuth examples

2. Architecture Patterns
   - How do they structure multi-provider auth?
   - Token storage approaches
   - Provider switching UX
   - Error handling patterns

3. Code Examples
   - Rust OAuth 2.0 client implementations
   - PKCE implementation examples
   - Multi-token storage patterns

**Deliverable**: Architecture Patterns Document
- Recommended patterns from successful apps
- Code snippets to adapt
- Anti-patterns to avoid
- UX patterns for provider switching

---

## Research Deliverables

### RD1: Research Summary Report

**Format**: Markdown document (5-10 pages)

**Sections**:
1. Executive Summary (GO/NO-GO recommendation)
2. OAuth Credential Acquisition (RO1 findings)
3. OAuth Flow Specifications (RO2 comparison matrix)
4. Security Architecture (RO3 recommendations)
5. Token Management Strategy (RO4 design)
6. Provider Requirements (RO5 matrix)
7. Reference Implementations (RO6 patterns)
8. Open Questions & Risks
9. Recommended Implementation Approach
10. Next Steps

**Target Audience**: Implementation developer (SPEC-KIT-952)

---

### RD2: Technical Architecture Diagram

**Visual Representation** (Mermaid or similar):
- Component diagram showing AuthManager, OAuth flows, token storage
- Sequence diagram for model selection → auth switch flow
- State diagram for token lifecycle
- Error handling flowchart

---

### RD3: Implementation Readiness Checklist

**Checklist Format**:
- [ ] OAuth credentials obtained for Claude
- [ ] OAuth credentials obtained for Gemini
- [ ] OAuth flow specifications documented
- [ ] Security approach validated
- [ ] Token storage library selected
- [ ] Refresh strategy defined
- [ ] Error scenarios mapped
- [ ] Provider quirks documented
- [ ] Reference code identified
- [ ] GO decision confirmed

---

## Research Methodology

### Phase 1: Web Research (4-6 hours)

**Activities**:
1. Search official documentation:
   - Anthropic docs for Claude OAuth
   - Google Cloud docs for Gemini OAuth
   - OAuth 2.0 RFC specifications (PKCE)

2. Community research:
   - GitHub issues/discussions about Claude/Gemini OAuth
   - Reddit/HN discussions about multi-provider auth
   - Stack Overflow questions

3. Code search:
   - GitHub search for Claude OAuth implementations
   - Rust OAuth examples
   - Multi-provider auth patterns

**Tools**:
- WebSearch for official docs
- GitHub search for code examples
- Developer forums and communities

---

### Phase 2: Hands-On Validation (3-5 hours)

**Activities**:
1. Attempt to register OAuth apps:
   - Try Anthropic Developer Portal registration
   - Create Google Cloud project
   - Document actual steps and blockers

2. Test OAuth flows:
   - Use Postman/curl to test endpoints (if accessible)
   - Validate PKCE implementation requirements
   - Test token refresh flows

3. Evaluate Rust libraries:
   - Test `oauth2` crate with mock providers
   - Test `keyring` crate on target platforms
   - Benchmark token storage approaches

---

### Phase 3: Analysis & Documentation (2-3 hours)

**Activities**:
1. Synthesize findings into report
2. Create architecture diagrams
3. Complete implementation readiness checklist
4. Draft GO/NO-GO recommendation
5. Identify remaining risks and blockers

---

## Success Criteria

### Must Have (GO Criteria)

**MC1**: Clear path to OAuth credentials
- Either: Official credentials obtained OR
- Clear process documented with timeline <2 weeks OR
- User-provided credentials approach validated

**MC2**: OAuth flows fully documented
- All endpoints identified and validated
- PKCE requirements confirmed
- Token refresh strategy defined

**MC3**: Security approach approved
- Token storage method selected and validated
- Threat model documented
- Security review completed

---

### Should Have (Quality Criteria)

**SC1**: Provider quirks identified
- Rate limits documented
- Authentication quirks mapped
- Workarounds defined

**SC2**: Reference implementations found
- Code examples identified
- Patterns documented
- Anti-patterns noted

---

### Could Have (Nice to Have)

**CH1**: Hands-on OAuth testing
- Test apps registered with providers
- OAuth flows validated end-to-end
- Token refresh tested

---

## Risks & Mitigation

### Risk 1: Cannot Obtain OAuth Credentials (CRITICAL)

**Impact**: Blocks entire implementation

**Mitigation**:
- Research alternative: User-provided credentials
- Investigate: API key approach for Claude/Gemini
- Fallback: Implement partial solution (ChatGPT only + API keys)

**Owner**: Research phase must answer this definitively

---

### Risk 2: Provider OAuth Not Available (HIGH)

**Impact**: Architecture redesign required

**Mitigation**:
- Early validation during Phase 1
- Quick pivot to API key approach if needed
- Document provider limitations clearly

---

### Risk 3: Security Approach Infeasible (MEDIUM)

**Impact**: Delayed implementation for security rework

**Mitigation**:
- Research multiple security approaches
- Get security review during research phase
- Have fallback options (encrypted file if keychain fails)

---

## Dependencies

### Upstream
- None (pure research)

### Downstream (Blocked by this SPEC)
- **SPEC-KIT-952**: Multi-Provider OAuth Implementation
- **SPEC-KIT-947**: Multi-Provider OAuth Master Validation

---

## Estimated Effort

**Total**: 9-14 hours

**Breakdown**:
- Phase 1 (Web Research): 4-6 hours
- Phase 2 (Hands-On Validation): 3-5 hours
- Phase 3 (Analysis & Documentation): 2-3 hours

**Timeline**: 2-3 days (part-time) or 1-2 days (full-time)

---

## Open Questions (To Be Answered by Research)

### Q1: OAuth Credential Acquisition
- How long to get Claude OAuth credentials?
- Is partnership with Anthropic required?
- What's the Google Cloud OAuth consent screen approval process?
- Can we use developer/sandbox credentials for testing?

### Q2: Technical Feasibility
- Do Claude and Gemini support OAuth 2.0 for desktop apps?
- Are refresh tokens provided or do we need to re-authenticate?
- What are the token lifetimes?
- Are there localhost redirect URI restrictions?

### Q3: Security Implementation
- Which platform keychain libraries work reliably in Rust?
- What's the fallback if keychain unavailable?
- How do we handle token encryption keys?
- What's the threat model for desktop OAuth apps?

### Q4: Provider Limitations
- Are there OAuth rate limits we need to handle?
- Do providers support PKCE (or is it mandatory)?
- Are there geographic restrictions on OAuth apps?
- What happens when one provider is down?

---

## Next Steps After Research

### If GO Decision

1. Create **SPEC-KIT-952-multi-provider-oauth-implementation**
   - Based on research findings
   - Include validated architecture
   - Reference security recommendations
   - Use documented OAuth flows

2. Update **SPEC-KIT-947** (Master Validation)
   - Update dependencies to reference 952
   - Refine validation criteria based on research
   - Add provider-specific test scenarios

3. Begin implementation with confidence
   - All blockers identified
   - Architecture validated
   - Security approach approved

---

### If NO-GO Decision

1. Document reasons (credential acquisition blocked, provider limitations, etc.)
2. Recommend alternative approaches:
   - API key approach for Claude/Gemini
   - Single-provider OAuth (ChatGPT only)
   - Partial implementation (ChatGPT OAuth + API keys)
3. Close or rescope SPEC-KIT-947 accordingly

---

## Appendix: Research Resources

### Official Documentation
- Anthropic API Docs: https://docs.anthropic.com/
- Google AI Studio: https://ai.google.dev/
- OAuth 2.0 RFC: https://oauth.net/2/
- PKCE RFC: https://oauth.net/2/pkce/

### Code Resources
- `oauth2` crate: https://docs.rs/oauth2/
- `keyring` crate: https://docs.rs/keyring/
- Rust OAuth examples: GitHub search "rust oauth2 pkce"

### Community Resources
- Anthropic Discord/Community
- Google AI Developer Forums
- Reddit r/rust, r/webdev

---

**END OF RESEARCH PRD**
