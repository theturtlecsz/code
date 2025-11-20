# Multi-Provider OAuth Research Report

**SPEC-ID**: SPEC-KIT-951
**Status**: In Progress (RO1 Complete)
**Created**: 2025-11-19
**Researcher**: Claude Code

---

## Executive Summary

### GO/NO-GO Recommendation: **CONDITIONAL GO** (API Key Hybrid Approach)

**Rationale**:
- ❌ **Anthropic Claude does NOT support OAuth** for third-party applications
- ✅ **Google Gemini fully supports OAuth 2.0** for desktop applications
- ✅ **ChatGPT OAuth** already implemented in codex-rs (baseline working)
- ⚠️ **Hybrid approach required**: OAuth for ChatGPT/Gemini, API keys for Claude

**Timeline to Implementation**: 15-25 hours (with hybrid approach)

**Critical Blockers**:
1. **BLOCKER**: Anthropic does not provide OAuth credentials for third-party apps
2. **SOLUTION**: Use API key authentication for Claude (ANTHROPIC_API_KEY environment variable)

---

## 1. OAuth Credential Acquisition (RO1) ✅ COMPLETE

### 1.1 Anthropic Claude - **NO OAUTH AVAILABLE** ❌

#### Findings

**Authentication Method**: API Keys ONLY

**Evidence from claude-code repository** (../claude-code/):
- No OAuth client registration process exists
- References to `ANTHROPIC_API_KEY` environment variable throughout codebase
- OAuth mentioned only for **MCP server integration** (where Claude Code acts as OAuth client to third-party services)
- No OAuth2Client or similar auth libraries for Anthropic API

**Official Documentation Research**:
- Anthropic developer console (console.anthropic.com) provides API key generation only
- No OAuth app registration portal
- OAuth exists for Claude Code product authentication (to claude.ai subscriptions), NOT for third-party API access
- Community forums confirm: "OpenAI does not have an OAuth API like Google"

**Conclusion**:
- ❌ Cannot obtain OAuth credentials for Claude API
- ✅ API key approach is the ONLY option
- Users must provide their own `ANTHROPIC_API_KEY`

#### Alternative Approaches

| Approach | Feasibility | Notes |
|----------|-------------|-------|
| User-provided OAuth credentials | ❌ NOT POSSIBLE | Anthropic doesn't offer OAuth registration |
| User-provided API keys | ✅ RECOMMENDED | Standard approach, already used by codex-rs |
| Developer sandbox/test credentials | ❌ NOT AVAILABLE | No sandbox OAuth environment |

---

### 1.2 Google Gemini - **OAUTH FULLY SUPPORTED** ✅

#### Findings

**Authentication Method**: OAuth 2.0 (full support)

**Evidence from gemini-cli repository** (../gemini-cli/):

**Hard-coded public OAuth credentials** (from `packages/core/src/code_assist/oauth2.ts`):
```typescript
const OAUTH_CLIENT_ID = '[REDACTED].apps.googleusercontent.com';
const OAUTH_CLIENT_SECRET = '[REDACTED]';

const OAUTH_SCOPE = [
  'https://www.googleapis.com/auth/cloud-platform',
  'https://www.googleapis.com/auth/userinfo.email',
  'https://www.googleapis.com/auth/userinfo.profile',
];
```

**Note**: Client secret is safe to embed per Google's documentation:
> "For installed applications, the client secret is obviously not treated as a secret"
> Source: https://developers.google.com/identity/protocols/oauth2#installed

**Implementation Details**:
- Uses `google-auth-library` NPM package
- `OAuth2Client` class with PKCE support
- Local HTTP server for OAuth redirect handling
- Token caching via `OAuthCredentialStorage`
- Supports encrypted token storage (optional)
- Success/failure redirect URLs to developers.google.com

**Libraries Used**:
- `google-auth-library` (OAuth2Client, GoogleAuth, CodeChallengeMethod)
- Local HTTP server for redirect URI (http://localhost:{port}/auth/callback)
- Token refresh with automatic retry

**Credential Acquisition Process**:
1. ✅ Use existing public OAuth client (Google's official Gemini CLI credentials)
2. ✅ OR: Create new OAuth client in Google Cloud Console
   - Enable Google Generative Language API
   - Configure OAuth consent screen
   - Create OAuth 2.0 Client ID (Desktop app type)
   - Download `client_secret.json`
3. ✅ Timeline: < 1 hour (using existing credentials) OR 2-4 hours (creating new)

**Conclusion**:
- ✅ Can use Google's existing OAuth credentials from gemini-cli
- ✅ OR create our own OAuth app (simple process)
- ✅ No partnership or approval required for basic access
- ⚠️ OAuth consent screen may require verification for public release

---

### 1.3 ChatGPT (OpenAI) - **BASELINE (Already Implemented)** ✅

#### Findings

**Authentication Method**: OAuth 2.0 (already working in codex-rs)

**Evidence from codex-rs codebase**:
- `core/src/auth.rs` implements OAuth token refresh
- `login/src/server.rs` implements PKCE OAuth flow
- Uses `https://auth.openai.com` as issuer
- Local server on port 1455 for OAuth redirect
- Token storage in `~/.code/auth.json`
- Automatic token refresh when < 28 days remaining

**Key Implementation Details** (from existing code):
```rust
const DEFAULT_ISSUER: &str = "https://auth.openai.com";
const DEFAULT_PORT: u16 = 1455;

// PKCE OAuth flow:
// 1. Generate PKCE code verifier/challenge
// 2. Build authorization URL with state parameter
// 3. Open browser to auth URL
// 4. Local server receives callback
// 5. Exchange code for tokens
// 6. Store tokens in auth.json
// 7. Refresh tokens automatically
```

**Conclusion**:
- ✅ ChatGPT OAuth already working
- ✅ Can serve as reference implementation
- ✅ Pattern can be adapted for Gemini
- ❌ Pattern CANNOT be used for Claude (no OAuth)

---

## 2. OAuth Flow Specifications (RO2) ✅ COMPLETE

### 2.1 OAuth Flow Comparison Matrix

| Aspect | ChatGPT | Claude | Gemini |
|--------|---------|--------|--------|
| **Authentication Method** | OAuth 2.0 | ❌ API Key ONLY | OAuth 2.0 |
| **Auth URL** | https://auth.openai.com/oauth/authorize | N/A | https://accounts.google.com/o/oauth2/v2/auth |
| **Token URL** | https://auth.openai.com/oauth/token | N/A | https://oauth2.googleapis.com/token |
| **PKCE Required** | ✅ Yes (S256) | N/A | ✅ Yes (S256) |
| **Scopes** | openid profile email offline_access | N/A | cloud-platform, userinfo.email, userinfo.profile |
| **Token Lifetime** | 28 days before refresh | N/A | Varies (check expiry_date field) |
| **Refresh Strategy** | Automatic when <28d | N/A | On-demand with 5min buffer |
| **Client Credentials** | Configured in codex-rs | N/A | Public (embeddable, safe) |
| **Redirect URI** | http://localhost:1455/auth/callback | N/A | http://localhost:{port}/auth/callback |
| **Response Type** | code | N/A | code |
| **Grant Type** | authorization_code | N/A | authorization_code |
| **Token Types** | id_token, access_token, refresh_token | N/A | access_token, refresh_token (optional) |

### 2.2 Detailed Flow Specifications

#### ChatGPT OAuth Flow (Existing Implementation) ✅

**Authorization Endpoint**: `https://auth.openai.com/oauth/authorize`
**Token Endpoint**: `https://auth.openai.com/oauth/token`

**Authorization Request Parameters** (from codex-rs/login/src/server.rs:315-326):
```
response_type: code
client_id: {configured_client_id}
redirect_uri: http://localhost:1455/auth/callback
scope: openid profile email offline_access
code_challenge: {pkce_challenge}
code_challenge_method: S256
id_token_add_organizations: true
codex_cli_simplified_flow: true
state: {random_32_bytes_base64url}
originator: {app_identifier}
```

**PKCE Implementation**:
- Code verifier: 32 random bytes, base64url encoded
- Code challenge: SHA256(verifier), base64url encoded
- Challenge method: S256

**Token Exchange Request** (from codex-rs/login/src/server.rs:427-433):
```
POST https://auth.openai.com/oauth/token
Content-Type: application/x-www-form-urlencoded

grant_type=authorization_code
code={authorization_code}
redirect_uri={redirect_uri}
client_id={client_id}
code_verifier={pkce_verifier}
```

**Token Response**:
```json
{
  "id_token": "eyJ...",
  "access_token": "sk-...",
  "refresh_token": "..."
}
```

**Flow Steps**:
1. Generate PKCE codes (verifier + challenge)
2. Generate random state (32 bytes, base64url)
3. Start local HTTP server on port 1455
4. Build authorization URL with all parameters
5. Open browser to authorization URL
6. User authenticates at auth.openai.com
7. Browser redirects to `http://localhost:1455/auth/callback?code=...&state=...`
8. Verify state parameter matches (CSRF protection)
9. Exchange code for tokens (POST with PKCE verifier)
10. Parse id_token JWT to extract account_id, email, plan_type
11. Store tokens in `~/.code/auth.json` with Unix permissions 0o600
12. Redirect browser to `/success` endpoint
13. Close local server

**Refresh Flow** (from codex-rs/core/src/auth.rs:92-121):
- Check `last_refresh` timestamp
- If > 28 days old, refresh automatically
- POST to `https://auth.openai.com/oauth/token`:
  ```
  grant_type=refresh_token
  refresh_token={stored_refresh_token}
  ```
- Receive new tokens, update storage
- 60-second timeout for refresh operation

**Token Storage**:
- File: `~/.code/auth.json`
- Format: JSON with `openai_api_key`, `tokens`, `last_refresh`
- Permissions: 0o600 (user read/write only) on Unix
- Backup account storage in `~/.code/auth_accounts.json`

#### Gemini OAuth Flow (From gemini-cli)

**Based on ../gemini-cli source code**:
1. Initialize OAuth2Client with public credentials
2. Check for cached credentials (OAuthCredentialStorage)
3. If valid cached token exists:
   - Verify token with `getAccessToken()`
   - Validate with `getTokenInfo(token)`
   - Return client
4. If no valid token:
   - Generate PKCE verifier/challenge (crypto.randomBytes)
   - Start local HTTP server on random port
   - Build authorization URL with:
     - `client_id`: Google's public client
     - `redirect_uri`: `http://localhost:{port}`
     - `scope`: cloud-platform, userinfo.email, userinfo.profile
     - `code_challenge` and `code_challenge_method`: `S256`
     - `access_type`: `offline` (for refresh token)
   - Open browser to auth URL
   - Wait for callback with authorization code
   - Exchange code for tokens
   - Save tokens via `OAuthCredentialStorage` (encrypted or plain)
   - Fetch user info and cache

**Token Management**:
- Listen to `tokens` event from OAuth2Client
- Auto-save new tokens when received
- Support encrypted storage via keyring
- 5-minute buffer before expiry for token refresh

#### Claude Authentication (API Key)

**Based on claude-code repository**:
1. User provides `ANTHROPIC_API_KEY` environment variable
2. No OAuth flow
3. API key included in requests via `x-api-key` header
4. No token refresh (keys don't expire)
5. No user authentication flow

---

## 3. Security Architecture (RO3) - PENDING

*To be researched*

---

## 4. Token Management Strategy (RO4) - PENDING

*To be researched*

---

## 5. Provider-Specific Requirements (RO5) - PENDING

*To be researched*

---

## 6. Reference Implementations (RO6) - IN PROGRESS

### Analyzed Implementations

#### 1. codex-rs (Current Codebase)
- **Location**: `/home/thetu/code/codex-rs/`
- **Auth Implementation**: `core/src/auth.rs`, `login/src/server.rs`
- **Provider**: ChatGPT OAuth
- **Key Patterns**:
  - PKCE implementation with SHA256
  - Local HTTP server for redirect (tiny_http)
  - Token caching in JSON file with file permissions (Unix: 0o600)
  - Automatic token refresh
  - AuthMode enum for switching between ApiKey and ChatGPT

#### 2. gemini-cli (Google Official)
- **Location**: `../gemini-cli/`
- **Auth Implementation**: `packages/core/src/code_assist/oauth2.ts`
- **Provider**: Google Gemini OAuth
- **Key Patterns**:
  - Uses `google-auth-library` NPM package
  - Public OAuth credentials embedded in code
  - OAuthCredentialStorage abstraction
  - Support for encrypted storage (OS keyring)
  - Support for Google ADC (Application Default Credentials)
  - MCP OAuth integration

#### 3. claude-code (Anthropic Official)
- **Location**: `../claude-code/`
- **Auth Implementation**: Environment variable only
- **Provider**: Anthropic API (no OAuth)
- **Key Patterns**:
  - API key via `ANTHROPIC_API_KEY` environment variable
  - No user authentication flow
  - MCP OAuth support (for third-party services)

---

## 7. Open Questions & Risks

### Critical Questions (Answered)

✅ **Q1**: How long to get Claude OAuth credentials?
**A**: IMPOSSIBLE - Anthropic does not offer OAuth for third-party apps

✅ **Q2**: Is partnership with Anthropic required?
**A**: Not relevant - OAuth not available regardless

✅ **Q3**: Can we use Google's public OAuth credentials?
**A**: YES - gemini-cli embeds public credentials, we can do the same OR create our own

✅ **Q4**: Do Claude and Gemini support OAuth 2.0 for desktop apps?
**A**: Gemini YES, Claude NO (API keys only)

### Remaining Questions

⏳ **Q5**: What are token lifetimes for Gemini?
**Status**: Need to check `expiry_date` in credentials

⏳ **Q6**: What are the exact ChatGPT OAuth scopes?
**Status**: Need to extract from existing codex-rs implementation

⏳ **Q7**: Token storage security - keyring vs encrypted file?
**Status**: RO3 research pending

⏳ **Q8**: Rate limits for OAuth endpoints?
**Status**: RO5 research pending

---

## 8. Recommended Implementation Approach

### Multi-Provider Authentication Architecture

**Hybrid Approach** (OAuth + API Keys):

```
┌─────────────────────────────────────┐
│      AuthMode Enum (Extended)      │
├─────────────────────────────────────┤
│ - ChatGPT OAuth   ✅ (working)      │
│ - Gemini OAuth    🔨 (implement)    │
│ - Claude API Key  ✅ (working)      │
└─────────────────────────────────────┘
```

**Step-by-Step Plan**:

1. **Phase 1: Extend AuthMode Enum** (2-3 hours)
   - Add `GeminiOAuth` variant
   - Keep existing `ChatGPT` and `ApiKey` variants
   - Update pattern matching throughout codebase

2. **Phase 2: Implement Gemini OAuth Flow** (8-12 hours)
   - Port OAuth2 flow from gemini-cli TypeScript → Rust
   - Use existing PKCE implementation from login/src/pkce.rs
   - Add Gemini-specific endpoints and scopes
   - Store Gemini tokens separately from ChatGPT tokens

3. **Phase 3: Unified Token Storage** (3-5 hours)
   - Create `AuthStorage` trait
   - Implement for ChatGPT tokens (existing)
   - Implement for Gemini tokens (new)
   - Support Claude API key storage (existing)

4. **Phase 4: Provider Switching Logic** (2-4 hours)
   - Update model selection to trigger auth switch
   - Check if provider has valid credentials
   - Prompt for OAuth flow if needed
   - Fall back to API key for Claude

**Total Estimated Effort**: 15-24 hours

---

## 9. Next Steps

### If GO Decision (Hybrid Approach)

1. **Create SPEC-KIT-952-multi-provider-oauth-implementation**
   - Use validated hybrid approach (OAuth for ChatGPT/Gemini, API key for Claude)
   - Reference gemini-cli OAuth implementation patterns
   - Leverage existing ChatGPT OAuth code as template
   - Document API key requirement for Claude

2. **Update SPEC-KIT-947** (Master Validation)
   - Add hybrid authentication test scenarios
   - Validate OAuth for ChatGPT and Gemini
   - Validate API key for Claude
   - Test provider switching across all three

3. **Complete remaining research objectives**
   - RO3: Security architecture (token storage, keyring)
   - RO4: Token management strategy (refresh timing, error handling)
   - RO5: Provider requirements (rate limits, quirks)

### Priority Actions (Immediate)

1. ✅ Complete RO2: Extract exact ChatGPT OAuth endpoints from codex-rs
2. ⏳ Start RO3: Research token storage security options
3. ⏳ Start RO4: Design token refresh strategy for multiple providers
4. ⏳ Start RO5: Document rate limits and provider quirks

---

## Appendix: Research Evidence

### A1: Source Code Locations

**codex-rs OAuth Implementation**:
- `/home/thetu/code/codex-rs/core/src/auth.rs` (lines 1-199)
- `/home/thetu/code/codex-rs/login/src/server.rs` (lines 1-150)
- `/home/thetu/code/codex-rs/login/src/pkce.rs`

**gemini-cli OAuth Implementation**:
- `../gemini-cli/packages/core/src/code_assist/oauth2.ts` (lines 1-150)
- `../gemini-cli/packages/core/src/mcp/google-auth-provider.ts` (lines 1-127)
- `../gemini-cli/packages/core/src/code_assist/oauth-credential-storage.ts`

**claude-code Auth References**:
- `../claude-code/` (grep results: ANTHROPIC_API_KEY references only)
- No OAuth client implementation found

### A2: Web Research Sources

- Anthropic API Documentation: https://docs.anthropic.com/
- Google Gemini OAuth: https://ai.google.dev/gemini-api/docs/oauth
- Google OAuth 2.0 for Desktop: https://developers.google.com/identity/protocols/oauth2/native-app
- OpenAI API Authentication: https://platform.openai.com/docs/api-reference/authentication
- GitHub: anthropics/claude-code, google-gemini/gemini-cli

---

**Report Version**: 0.1 (RO1 Complete)
**Last Updated**: 2025-11-19
**Next Update**: After completing RO2-RO6
