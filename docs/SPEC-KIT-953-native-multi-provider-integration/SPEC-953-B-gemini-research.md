# SPEC-KIT-953-B: Gemini CLI Architecture Analysis

**Parent**: SPEC-KIT-953
**Status**: Research Complete
**Priority**: High (Parallel with A, C)
**Target**: ~/gemini-cli
**Completed**: 2025-11-19

---

## License

**Apache 2.0** - Can extract and adapt patterns and code.

---

## Executive Summary

The Gemini CLI (`@google/gemini-cli`) is a TypeScript-based CLI distributed via npm. Key findings:

- **OAuth 2.0** with public embedded credentials
- **Direct Google AI API** access (generativelanguage.googleapis.com)
- **PKCE-protected authorization** for desktop
- **SSE streaming** for responses
- **No official Rust SDK** - but well-documented API

**Recommendation**: Rust-native rewrite (104-130 hours estimated)

---

## Research Findings

### 1. Repository Structure

```
~/gemini-cli/
├── packages/
│   ├── cli/
│   │   ├── bin/gemini.ts           # CLI entry point
│   │   └── src/
│   │       ├── commands/           # Command implementations
│   │       ├── output/             # Formatters
│   │       └── config/             # Config management
│   │
│   └── core/
│       └── src/
│           ├── code_assist/
│           │   ├── oauth2.ts       # OAuth implementation
│           │   └── api-client.ts   # Google AI API wrapper
│           ├── auth/
│           │   └── credential_storage.ts
│           └── streaming/          # SSE handling
│
├── docs/                           # API docs
├── GEMINI.md                       # Instructions
└── README.md
```

### 2. API Architecture

**Base URL**: `https://generativelanguage.googleapis.com/v1beta/`

**Authentication**: Bearer token (OAuth 2.0 access_token)

**Request Format**:
```json
POST /v1beta/models/{model-id}:generateContent
Authorization: Bearer {access_token}
Content-Type: application/json

{
  "contents": [
    {
      "role": "user",
      "parts": [
        {"text": "prompt text"}
      ]
    }
  ],
  "safetySettings": [...],
  "generationConfig": {
    "temperature": 1,
    "topP": 0.95,
    "topK": 64,
    "maxOutputTokens": 8192
  }
}
```

**Response Format**:
```json
{
  "candidates": [
    {
      "content": {
        "role": "model",
        "parts": [{"text": "response"}]
      },
      "finishReason": "STOP",
      "safetyRatings": [...]
    }
  ],
  "usageMetadata": {
    "promptTokenCount": 15,
    "candidatesTokenCount": 100,
    "totalTokenCount": 115
  }
}
```

**Message Roles**: `user`, `model` (not `assistant`)

### 3. Authentication Flow

**Public OAuth Credentials** (embedded in CLI):
```typescript
const OAUTH_CLIENT_ID = '[REDACTED].apps.googleusercontent.com';
const OAUTH_CLIENT_SECRET = '[REDACTED]';

const OAUTH_SCOPE = [
  'https://www.googleapis.com/auth/cloud-platform',
  'https://www.googleapis.com/auth/userinfo.email',
  'https://www.googleapis.com/auth/userinfo.profile',
];
```

**OAuth Flow**:
1. Check cached credentials
2. Generate PKCE challenge (SHA256)
3. Build authorization URL
4. Launch browser + local HTTP server (random port)
5. Receive callback with authorization code
6. Exchange code for tokens (with PKCE verifier)
7. Store tokens in `~/.gemini/credentials.json`

**Token Endpoint**: `POST https://oauth2.googleapis.com/token`

**Token Refresh**: Automatic when within 5 minutes of expiry
```typescript
POST https://oauth2.googleapis.com/token
grant_type=refresh_token
refresh_token={stored_refresh_token}
client_id={OAUTH_CLIENT_ID}
client_secret={OAUTH_CLIENT_SECRET}
```

**Token Storage Format**:
```json
{
  "access_token": "ya29.a0AfH6SMBx...",
  "refresh_token": "1//0gF...",
  "expiry_date": 1699999999000,
  "token_type": "Bearer",
  "scopes": ["https://www.googleapis.com/auth/cloud-platform", ...]
}
```

### 4. Conversation Context Management

**Stateless API**: Full history sent each request.

**Message Format**:
```typescript
interface Message {
  role: 'user' | 'model';
  parts: Part[];
}

interface Part {
  text?: string;
  inlineData?: { mimeType: string; data: string };  // base64 images
  fileData?: { mimeType: string; fileUri: string }; // gs:// URIs
}
```

**Token Counting**: Use `countTokens` API endpoint
```
POST /v1beta/models/{model}:countTokens
```

**Context Limits**:
- `gemini-2.0-flash`: 1M input, 8K output
- `gemini-2.5-pro`: 2M input, 8K output
- `gemini-3.0-pro`: 2M input, 8K output

**Truncation**: Client-side, oldest-first recommended.

### 5. Streaming Response Handling

**Streaming Endpoint**: `/v1beta/models/{model}:streamGenerateContent`

**Protocol**: Server-Sent Events (SSE)

**Event Format**:
```
data: {"candidates":[{"content":{"role":"model","parts":[{"text":"Hello"}]},"finishReason":null}]}

data: {"candidates":[{"content":{"role":"model","parts":[{"text":", how"}]},"finishReason":null}]}

data: {"candidates":[...],"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":8,"totalTokenCount":18}}
```

**Assembly Logic**: Concatenate text parts from each delta event.

### 6. Rate Limiting and Error Handling

**Common Errors**:
- 429 Too Many Requests: Rate limited
- 401 Unauthorized: Invalid/expired token
- 403 Forbidden: Permission denied
- 400 Bad Request: Invalid prompt

**Retry Strategy**: Exponential backoff (1s, 2s, 4s) for 429/503.

**Default Quota**: 60 requests/minute per API key (configurable via Google Cloud Console).

### 7. Model Configuration

**Model IDs** (from existing codex-rs integration):
- `gemini-3.0-pro`
- `gemini-2.5-pro`
- `gemini-2.5-flash`
- `gemini-2.0-flash`

**API Format**: `models/{model-name}`

---

## Implementation Feasibility

### Rust Implementation Path

**Required Crates**:
```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json", "stream"] }
oauth2 = "4.4"
sha2 = "0.10"
base64 = "0.21"
rand = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
dirs = "5.0"
futures = "0.3"
axum = "0.7"  # for OAuth callback server
chrono = "0.4"
```

### Rust Rewrite Feasibility: **8/10** (Very Good)

**Why High Score**:
- Well-documented Google AI API
- Standard OAuth2 with existing Rust crates
- SSE streaming (standard pattern)
- Apache 2.0 allows code extraction
- No proprietary protocols

**Complexity**:
- OAuth2 flow more complex than Anthropic API key
- Must implement PKCE challenge
- Token refresh required

**Effort Estimate**:
| Component | Hours |
|-----------|-------|
| OAuth2 flow (PKCE) | 16-20 |
| Google AI API client | 20-24 |
| Streaming handler | 12-16 |
| Token storage & refresh | 8-10 |
| Context management | 16-20 |
| TUI integration | 12-16 |
| Testing | 20-24 |
| **Total** | **104-130h** |

---

## Key Code Patterns for Rust

### OAuth2 with PKCE

```rust
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret,
    PkceCodeChallenge, RedirectUrl, TokenUrl,
    reqwest::async_http_client,
};

let client = BasicClient::new(
    ClientId::new(OAUTH_CLIENT_ID.to_string()),
    Some(ClientSecret::new(OAUTH_CLIENT_SECRET.to_string())),
    AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())?,
    Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string())?),
)
.set_redirect_uri(RedirectUrl::new(format!("http://localhost:{}/callback", port))?);

// Generate PKCE challenge
let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

// Build auth URL
let (auth_url, csrf_state) = client
    .authorize_url(CsrfToken::new_random)
    .add_scope(Scope::new("https://www.googleapis.com/auth/cloud-platform".to_string()))
    .set_pkce_challenge(pkce_challenge)
    .url();

// Exchange code for tokens
let token = client
    .exchange_code(code)
    .set_pkce_verifier(pkce_verifier)
    .request_async(async_http_client)
    .await?;
```

### Streaming with reqwest

```rust
use futures::stream::StreamExt;

let response = client
    .post(format!("https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent", model))
    .bearer_auth(&access_token)
    .json(&payload)
    .send()
    .await?;

let mut stream = response.bytes_stream();
let mut buffer = String::new();

while let Some(chunk) = stream.next().await {
    let text = String::from_utf8(chunk?.to_vec())?;
    buffer.push_str(&text);

    for line in buffer.lines() {
        if line.starts_with("data: ") {
            let json = &line[6..];
            let event: GeminiStreamEvent = serde_json::from_str(json)?;
            if let Some(text) = event.get_text() {
                yield text;
            }
        }
    }
}
```

### Token Storage

```rust
use dirs::config_dir;
use std::fs;

let creds_path = config_dir()
    .unwrap()
    .join("gemini")
    .join("credentials.json");

// Save
fs::create_dir_all(creds_path.parent().unwrap())?;
fs::write(&creds_path, serde_json::to_string_pretty(&creds)?)?;

#[cfg(unix)]
{
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&creds_path, fs::Permissions::from_mode(0o600))?;
}

// Load
let credentials: GoogleCredentials = serde_json::from_str(
    &fs::read_to_string(&creds_path)?
)?;
```

---

## Deliverables Summary

- [x] Repository structure analysis
- [x] API client extraction feasibility (extractable under Apache 2.0)
- [x] OAuth flow documentation (PKCE, scopes, endpoints)
- [x] Conversation context patterns (stateless, full history)
- [x] Message history format (`role`/`parts` structure)
- [x] Streaming architecture (SSE with deltas)
- [x] Rate limiting patterns (exponential backoff)
- [x] **Implementation recommendation**: **Rust rewrite preferred**

---

## Implications for SPEC-953

### For SPEC-953-D (Auth Framework)

- Add `GoogleOAuth` variant to `AuthMode`
- OAuth2 flow with PKCE required
- Token refresh with 5-minute pre-expiry window
- Store in `~/.codex/google_credentials.json` or unified `auth_accounts.json`

### For SPEC-953-E (Context Manager)

- Gemini format: `{"role": "user|model", "parts": [{"text": "..."}]}`
- Supports multi-modal (inline images, file URIs)
- Use `countTokens` endpoint for accurate counting
- Context limits: 1M-2M tokens (very generous)

### For SPEC-953-G (Native Gemini Provider)

- **Rust rewrite recommended**
- OAuth2 complexity higher than Anthropic but manageable
- Can extract patterns from gemini-cli (Apache 2.0)
- SSE streaming standard pattern

---

## Architecture Decision Input

**For Checkpoint 1**:

| Criterion | Rust Rewrite | FFI Bridge |
|-----------|--------------|------------|
| Complexity | Medium (OAuth2) | High (Node.js + IPC) |
| Performance | Native | Subprocess overhead |
| Auth | OAuth2 + refresh | Same |
| Streaming | Direct SSE | Complex IPC |
| Dependencies | oauth2, reqwest | Node.js runtime |
| Maintenance | Self-contained | Two codebases |
| License | ✅ Apache 2.0 allows | Same |

**Recommendation**: **Rust-native rewrite** for Gemini provider (consistent with Claude).

---

## References

- Google AI API Docs: https://ai.google.dev/docs
- oauth2 crate: https://crates.io/crates/oauth2
- SPEC-KIT-951: Multi-Provider OAuth Research
- codex-rs/tui/src/providers/gemini.rs (current CLI integration)

---

## Document History

| Date | Status | Notes |
|------|--------|-------|
| 2025-11-19 | Complete | Comprehensive analysis - Rust rewrite recommended |
