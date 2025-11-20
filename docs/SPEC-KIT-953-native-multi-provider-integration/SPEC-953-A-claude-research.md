# SPEC-KIT-953-A: Claude Code Architecture Analysis

**Parent**: SPEC-KIT-953
**Status**: Research Complete
**Priority**: High (Parallel with B, C)
**Target**: ~/claude-code (plugins repository)
**Completed**: 2025-11-19

---

## Constraint

**PROPRIETARY LICENSE** - Analyze for independent re-implementation only. Cannot extract or copy code.

---

## Critical Discovery

The `/home/thetu/claude-code` directory is **NOT the Claude Code source code** - it's a plugins repository. The actual Claude Code CLI is proprietary and distributed only as npm package `@anthropic-ai/claude-code`.

**Implication**: Must use public Anthropic API documentation for independent implementation.

---

## Research Findings

### 1. API Architecture

**Endpoint**: `POST https://api.anthropic.com/v1/messages`

**Authentication Headers**:
```
x-api-key: $ANTHROPIC_API_KEY
anthropic-version: 2023-06-01
content-type: application/json
```

**Request Format**:
```json
{
  "model": "claude-opus-4-1-20250805",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "max_tokens": 256,
  "stream": true
}
```

**Message Roles**: `user`, `assistant` (no separate `system` role - system prompt is first user message or `system` parameter)

### 2. Authentication

**CORRECTION**: Claude Code uses **full OAuth 2.0 with PKCE** - identical pattern to Gemini CLI!

#### OAuth 2.0 Configuration

**Client ID**: `9d1c250a-e61b-44d9-88ed-5944d1962f5e`

**Endpoints**:
- Authorization: `https://claude.ai/oauth/authorize`
- Token: `https://console.anthropic.com/v1/oauth/token`
- Redirect: `https://console.anthropic.com/oauth/code/callback`

**OAuth Parameters**:
```
response_type: code
client_id: 9d1c250a-e61b-44d9-88ed-5944d1962f5e
redirect_uri: https://console.anthropic.com/oauth/code/callback
scope: org:create_api_key user:profile user:inference
code_challenge: <PKCE_challenge>
code_challenge_method: S256
state: <verifier>
```

#### PKCE Implementation

```typescript
// Generate PKCE pair
const bytes = crypto.getRandomValues(new Uint8Array(32));
const verifier = base64url(bytes);
const challenge = base64url(SHA256(verifier));
```

#### Token Exchange

```typescript
POST https://console.anthropic.com/v1/oauth/token
{
  "code": "<authorization_code>",
  "state": "<state>",
  "grant_type": "authorization_code",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e",
  "redirect_uri": "https://console.anthropic.com/oauth/code/callback",
  "code_verifier": "<verifier>"
}
```

**Token Response**:
```json
{
  "access_token": "sk-ant-oat01-...",
  "refresh_token": "sk-ant-ort01-...",
  "expires_in": 3600
}
```

#### Token Refresh

```typescript
POST https://console.anthropic.com/v1/oauth/token
{
  "grant_type": "refresh_token",
  "refresh_token": "<refresh_token>",
  "client_id": "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
}
```

#### Token Storage (`~/.claude/.credentials.json`)

```json
{
  "claudeAiOauth": {
    "accessToken": "sk-ant-oat01-...",
    "refreshToken": "sk-ant-ort01-...",
    "expiresAt": 1763594437215,
    "scopes": [
      "user:inference",
      "user:profile",
      "user:sessions:claude_code"
    ],
    "subscriptionType": "max",
    "rateLimitTier": "default_claude_max_20x"
  }
}
```

**Comparison with Other Providers**:
| Aspect | Anthropic | ChatGPT | Gemini |
|--------|-----------|---------|--------|
| Auth Type | OAuth 2.0 PKCE | OAuth 2.0 | OAuth 2.0 PKCE |
| Token Refresh | ✅ Required | ✅ Required | ✅ Required |
| Public Client ID | ✅ Embedded | ✅ Embedded | ✅ Embedded |
| Complexity | Medium | Medium | Medium |

**Implication for SPEC-953**: Must implement OAuth 2.0 with PKCE, similar to Gemini. Can share OAuth infrastructure.

### 3. Conversation Context

**Stateless API**: Each request must include full conversation history.

**Message Array Format**:
```json
{
  "messages": [
    {"role": "user", "content": "What is 2+2?"},
    {"role": "assistant", "content": "2+2 equals 4."},
    {"role": "user", "content": "And 3+3?"}
  ]
}
```

**Context Window Limits** (as of 2025):
- Claude Opus 4: 200K tokens
- Claude Sonnet 4.5: 200K tokens
- Claude Haiku 4.5: 200K tokens

**Token Counting**: No official Anthropic tokenizer library. Use:
1. Anthropic's `/count_tokens` endpoint (if available)
2. Unofficial estimation (Claude uses similar tokenization to GPT)

**Truncation Strategy**: Client-side responsibility. Recommended: oldest-first removal.

### 4. Streaming Responses

**Protocol**: Server-Sent Events (SSE)

**Enabling**: Set `"stream": true` in request body

**Event Types**:
```
message_start
content_block_start
content_block_delta
content_block_stop
message_delta
message_stop
ping
error
```

**Content Block Delta Format**:
```
event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}
```

**Token Usage** (in message_delta):
```json
{
  "type": "message_delta",
  "usage": {
    "input_tokens": 25,
    "output_tokens": 50
  }
}
```

**Error Events** (during streaming):
```
event: error
data: {"type":"error","error":{"type":"overloaded_error","message":"..."}}
```

### 5. Implementation Feasibility

**Rust SDK Options**:

| Crate | Status | Features |
|-------|--------|----------|
| `anthropic-sdk-rust` | Unofficial | Full feature parity claimed, streaming, tools, vision |
| `anthropic-ai-sdk` | Unofficial | Async, token counting, builder patterns |
| `anthropic` | Unofficial | Inspired by async-openai |
| `oauth2` | Official | OAuth 2.0 with PKCE support |

**Recommendation**: Use `oauth2` crate for auth, `anthropic-sdk-rust` as API reference.

**Rust Rewrite Feasibility**: **8/10** (Very Good)

**Why High Score**:
- OAuth 2.0 PKCE has excellent Rust support (`oauth2` crate)
- Standard SSE streaming
- Well-documented public API
- Existing Rust crates as reference
- **Can share OAuth infrastructure with Gemini provider!**

**Implementation Effort Estimate**:
| Component | Hours |
|-----------|-------|
| OAuth2 flow (PKCE) | 16-20 |
| API Client | 16-20 |
| SSE Streaming | 8-12 |
| Token Storage & Refresh | 8-10 |
| Context Management | 12-16 |
| TUI Integration | 8-12 |
| **Total** | **68-90h** |

**Shared Infrastructure with Gemini**: OAuth PKCE flow can be shared between providers, reducing total effort.

---

## Deliverables Summary

- [x] Repository structure analysis (discovered plugins-only, not source)
- [x] API client pattern documentation (direct Anthropic API)
- [x] OAuth flow documentation (**OAuth 2.0 with PKCE** - same as Gemini!)
- [x] Conversation state management analysis (stateless, client accumulates)
- [x] Message history format documentation (role/content array)
- [x] Streaming architecture documentation (SSE with typed events)
- [x] **Implementation recommendation**: **Rust rewrite - share OAuth infrastructure with Gemini**

---

## Key Code Patterns for Rust

### API Request

```rust
use reqwest::Client;
use serde_json::json;

let response = client
    .post("https://api.anthropic.com/v1/messages")
    .header("x-api-key", &api_key)
    .header("anthropic-version", "2023-06-01")
    .header("content-type", "application/json")
    .json(&json!({
        "model": model_id,
        "messages": conversation_history,
        "max_tokens": max_tokens,
        "stream": true
    }))
    .send()
    .await?;
```

### SSE Streaming

```rust
use futures::StreamExt;

let mut stream = response.bytes_stream();
let mut buffer = String::new();

while let Some(chunk) = stream.next().await {
    let text = String::from_utf8(chunk?.to_vec())?;
    buffer.push_str(&text);

    for line in buffer.lines() {
        if line.starts_with("data: ") {
            let json = &line[6..];
            let event: StreamEvent = serde_json::from_str(json)?;
            // Process event
        }
    }
}
```

### OAuth2 with PKCE (Same Pattern as Gemini!)

```rust
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId,
    PkceCodeChallenge, RedirectUrl, TokenUrl,
    reqwest::async_http_client,
};

let client = BasicClient::new(
    ClientId::new("9d1c250a-e61b-44d9-88ed-5944d1962f5e".to_string()),
    None, // No client secret (public client)
    AuthUrl::new("https://claude.ai/oauth/authorize".to_string())?,
    Some(TokenUrl::new("https://console.anthropic.com/v1/oauth/token".to_string())?),
)
.set_redirect_uri(
    RedirectUrl::new("https://console.anthropic.com/oauth/code/callback".to_string())?
);

// Generate PKCE challenge
let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

// Build auth URL
let (auth_url, csrf_state) = client
    .authorize_url(CsrfToken::new_random)
    .add_scope(Scope::new("org:create_api_key".to_string()))
    .add_scope(Scope::new("user:profile".to_string()))
    .add_scope(Scope::new("user:inference".to_string()))
    .set_pkce_challenge(pkce_challenge)
    .url();

// Exchange code for tokens
let token = client
    .exchange_code(code)
    .set_pkce_verifier(pkce_verifier)
    .request_async(async_http_client)
    .await?;
```

### Token Storage

```rust
// Store in ~/.claude/.credentials.json or unified auth storage
#[derive(Serialize, Deserialize)]
struct AnthropicOAuthTokens {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
    scopes: Vec<String>,
}
```

---

## Implications for SPEC-953

### For SPEC-953-D (Auth Framework)

- Add `AnthropicOAuth` variant to `AuthMode`
- **Requires OAuth 2.0 with PKCE** - same flow as Gemini!
- Can share OAuth infrastructure between Anthropic and Google
- Token refresh logic required (similar to OpenAI ChatGPT)
- Store tokens in unified `auth_accounts.json` or separate `~/.claude/.credentials.json`

### For SPEC-953-E (Context Manager)

- Anthropic message format: `{"role": "user|assistant", "content": "..."}`
- Token counting: Use estimation or unofficial tokenizer
- Context limit: 200K tokens (generous)
- Truncation: Client responsibility

### For SPEC-953-F (Native Claude Provider)

- **Rust rewrite recommended** (not FFI)
- Use `anthropic-sdk-rust` as reference or direct dependency
- SSE streaming integration with TUI
- Simple authentication flow

---

## Architecture Decision Input

**For Checkpoint 1**:

| Criterion | Rust Rewrite | FFI Bridge |
|-----------|--------------|------------|
| Complexity | Medium (OAuth2 PKCE) | High (Node.js dependency) |
| Performance | Native | Subprocess overhead |
| Auth | OAuth2 PKCE (same as Gemini) | Same |
| Streaming | Direct SSE | Complex IPC |
| Dependencies | oauth2, reqwest, serde | Node.js runtime |
| Maintenance | Self-contained | Two codebases |
| Code Sharing | ✅ Share OAuth with Gemini | ❌ Separate implementations |

**Recommendation**: **Rust-native rewrite** for Claude provider.

**Key Insight**: Both Claude and Gemini use OAuth 2.0 with PKCE. The shared infrastructure reduces total implementation effort significantly.

---

## References

- Anthropic API Docs: https://docs.anthropic.com/en/api/
- anthropic-sdk-rust: https://crates.io/crates/anthropic-sdk-rust
- Claude Code plugins: /home/thetu/claude-code/plugins/

---

## Document History

| Date | Status | Notes |
|------|--------|-------|
| 2025-11-19 | Complete | Initial research - incorrectly assumed API key only |
| 2025-11-19 | **Revised** | **MAJOR CORRECTION**: Claude Code uses OAuth 2.0 with PKCE, same as Gemini. Effort estimate increased from 48-66h to 68-90h. Can share OAuth infrastructure between providers. |
