# NotebookLM HTTP Service Analysis

**Date**: 2025-12-14 (P87 - Updated for v2.0.0)
**Source**: `/home/thetu/notebooklm-mcp/` (local installation via npm link)
**Version**: 2.0.0

---

## Executive Summary

| Aspect | Assessment | Notes |
|--------|------------|-------|
| Stability | Excellent | Mature implementation, 1287 tests, 50% coverage |
| Authentication | Chrome profile | Persistent cookies, no manual cookie management |
| Session management | Robust | Auto-cleanup, max sessions, shared browser context |
| Rate limits | External | 50 queries/day (NotebookLM free tier) |
| Latency | ~5-15s | Browser automation overhead |
| Interface | HTTP API | Primary interface; CLI also available |

**Verdict**: Production-ready for Tier 2 integration. Main constraint is NotebookLM rate limit.

---

## Breaking Change: MCP Removed in v2.0.0

As of v2.0.0 (December 2025), the MCP server interface was completely removed:

| v1.x | v2.0.0 |
|------|--------|
| `mcp__notebooklm__ask` | `POST /api/ask` |
| `mcp__notebooklm__get_health` | `GET /health` |
| `mcp__notebooklm__list_notebooks` | `GET /api/notebooks` |
| `notebooklm-mcp` binary | `notebooklm` CLI |

**Reason**: Simplified architecture, HTTP API is more universally accessible.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                   NotebookLM HTTP Service                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                     HTTP Server (:3456)                   │   │
│  │  ├── /health, /health/live, /health/ready                │   │
│  │  ├── /api/notebooks (GET, POST, DELETE)                  │   │
│  │  ├── /api/ask (POST)                                     │   │
│  │  ├── /api/sources (GET, POST)                            │   │
│  │  └── /api/research/fast, /api/research/deep (POST)       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            │                                     │
│                            v                                     │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │              Request Queue (serializes browser ops)       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                            │                                     │
│  ┌──────────────┐  ┌───────────────┐  ┌────────────────────┐   │
│  │ AuthManager  │  │ SessionManager│  │ NotebookLibrary    │   │
│  │ - chrome     │  │ - 10 max sess │  │ - notebook metadata│   │
│  │   profile    │  │ - 15min TTL   │  │ - library.json     │   │
│  └──────────────┘  └───────────────┘  └────────────────────┘   │
│           │                │                    │               │
│           v                v                    v               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              SharedContextManager                         │  │
│  │  - Single browser fingerprint                             │  │
│  │  - Patchright (stealth Playwright)                        │  │
│  │  - Human-like behavior (stealth-utils.ts)                 │  │
│  └──────────────────────────────────────────────────────────┘  │
│                            │                                    │
│                            v                                    │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │              Chrome Profile                               │  │
│  │  ~/.local/share/notebooklm-mcp/chrome_profile/           │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

---

## HTTP API Reference

### Core Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/health` | Full health check with auth status |
| GET | `/health/live` | Kubernetes liveness probe |
| GET | `/health/ready` | Kubernetes readiness probe |
| GET | `/api/notebooks` | List notebooks in library |
| POST | `/api/notebooks` | Add notebook to library |
| DELETE | `/api/notebooks/:id` | Remove notebook from library |
| PUT | `/api/notebooks/:id/select` | Set active notebook |
| POST | `/api/ask` | Ask a question |
| GET | `/api/sources` | List sources in notebook |
| POST | `/api/sources` | Add source to notebook |
| POST | `/api/research/fast` | Fast web research |
| POST | `/api/research/deep` | Deep autonomous research |

### Ask Endpoint (Primary for SPEC-KIT-102)

```http
POST /api/ask
Content-Type: application/json

{
  "notebookId": "my-notebook-id",
  "question": "What are the key architectural decisions?"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "answer": "Based on the sources...",
    "sources": [
      {"title": "Architecture Doc", "index": 0}
    ]
  }
}
```

### Health Check

```http
GET /health?deep_check=true
```

**Response**:
```json
{
  "status": "ok",
  "authenticated": true,
  "sessions": {
    "active": 1,
    "max": 10
  },
  "queue": {
    "pending": 0,
    "processing": 0
  }
}
```

---

## Key Components

### 1. Session Manager (`session/session-manager.ts`)

**Configuration**:
```typescript
maxSessions: 10          // Concurrent sessions
sessionTimeout: 900      // 15 minutes TTL
cleanupInterval: 60-300s // Auto-cleanup frequency
```

**Behavior**:
- Auto-generates session IDs (8 hex chars)
- Reuses existing sessions for same notebook URL
- Evicts oldest session when max reached
- Shared browser context across all sessions (single fingerprint)

### 2. Auth Manager (`auth/auth-manager.ts`)

**Authentication Methods**:
1. `setup-auth` - Interactive browser login (CLI)
2. `re-auth` - Clear and re-authenticate (CLI)

**Authentication via CLI**:
```bash
notebooklm setup-auth     # Opens browser for Google login
notebooklm health --deep  # Verify authentication
```

**Cookie Management**: Chrome profile handles cookies natively. No manual cookie import needed.

### 3. Stealth Settings (`config.ts`)

**Human-like Behavior**:
```typescript
stealthEnabled: true
stealthRandomDelays: true    // 100-400ms delays
stealthHumanTyping: true     // 160-240 WPM
stealthMouseMovements: true  // Realistic cursor
```

**Purpose**: Avoid bot detection, appear as normal user.

---

## Rate Limits

### NotebookLM Limits (External)

| Tier | Queries/Day | Notebooks | Sources/Notebook |
|------|-------------|-----------|------------------|
| Free | 50 | 100 | 50 |
| AI Pro/Ultra | ~250 | 500 | 100 |

**Implications for SPEC-KIT-102**:
- 50 queries/day = ~2 queries/hour for 24h operation
- Must cache aggressively
- Predictive prefetching critical

### HTTP Service Limits (Configurable)

| Parameter | Default | Environment Variable |
|-----------|---------|---------------------|
| Max sessions | 10 | `MAX_SESSIONS` |
| Session timeout | 900s | `SESSION_TIMEOUT` |
| Service port | 3456 | `NOTEBOOKLM_SERVICE_PORT` |
| Service host | 127.0.0.1 | `NOTEBOOKLM_SERVICE_HOST` |

---

## Latency Analysis

### Query Lifecycle

```
HTTP Request (POST /api/ask)
     │
     ▼ (~5ms)
Request Queue
     │
     ▼ (0-45s if cold start)
Browser Page Ready
     │
     ▼ (~500ms)
Input Question (human-like typing)
     │
     ▼ (~5-15s)
NotebookLM Response (Gemini processing)
     │
     ▼ (~200ms)
Extract Response
     │
     ▼ (~5ms)
HTTP Response
```

**Observed Latencies**:
| Phase | Cold | Warm |
|-------|------|------|
| Queue + session | 30-45s | 50ms |
| Question input | 500ms | 500ms |
| Gemini processing | 5-15s | 5-15s |
| Response extraction | 200ms | 200ms |
| **Total** | **36-61s** | **5.7-15.7s** |

---

## Error Handling

### Automatic Recovery

| Error | Handling |
|-------|----------|
| Session expired | Auto-cleanup, create new |
| Max sessions | Evict oldest |
| Auth expired | Returns 401, prompt re-auth |
| Browser crash | Recreate context |
| Network timeout | Configurable retry |

### Manual Intervention Required

| Error | Action |
|-------|--------|
| Rate limit hit | Wait or switch account |
| Google account locked | Manual recovery |
| NotebookLM unavailable | Fallback to Tier 1 only |

---

## Data Paths

```
~/.local/share/notebooklm-mcp/
├── library.json           # Saved notebooks
├── research_history.json  # Research history and cache
├── chrome_profile/        # Browser profile (cookies, auth)
├── browser_state/         # Session state
├── service.pid            # Service daemon PID
└── debug/                 # Screenshots on failures
```

---

## Integration Recommendations

### For SPEC-KIT-102 Orchestrator

1. **Service Mode** (Required)
   ```bash
   # Start before any queries
   notebooklm service start

   # Verify
   curl -s localhost:3456/health | jq .authenticated
   ```

2. **HTTP Client Pattern**
   ```python
   import httpx

   NOTEBOOKLM_URL = "http://localhost:3456"

   async def ask_notebooklm(notebook_id: str, question: str) -> str:
       async with httpx.AsyncClient() as client:
           response = await client.post(
               f"{NOTEBOOKLM_URL}/api/ask",
               json={"notebookId": notebook_id, "question": question},
               timeout=60.0  # Allow for cold start
           )
           response.raise_for_status()
           return response.json()["data"]["answer"]
   ```

3. **Cache Layer** (Critical)
   - Cache all NotebookLM responses
   - TTL: 24 hours minimum
   - Key: question hash + notebook ID

4. **Query Budget**
   - Reserve 30 queries for Stage 0 (pipeline)
   - Reserve 15 for user requests
   - Reserve 5 for emergencies

5. **Fallback Strategy**
   ```python
   async def get_synthesis(spec: str, brief: str) -> str:
       # Check cache first
       if cached := await cache.get(hash(spec, brief)):
           return cached

       # Check service health
       health = await httpx.get(f"{NOTEBOOKLM_URL}/health")
       if not health.json().get("authenticated"):
           return await fallback_to_tier1(spec, brief)

       # Query NotebookLM
       return await ask_notebooklm(NOTEBOOK_ID, brief)
   ```

### Health Monitoring

```python
async def check_notebooklm_health() -> bool:
    """Check before critical operations."""
    try:
        response = await httpx.get(
            f"{NOTEBOOKLM_URL}/health",
            params={"deep_check": "true"},
            timeout=10.0
        )
        health = response.json()
        return health.get("authenticated", False)
    except Exception:
        return False
```

---

## CLI Reference

For debugging and manual operations:

```bash
# Service management
notebooklm service start          # Start HTTP daemon
notebooklm service stop           # Stop daemon
notebooklm service status         # Check status

# Notebooks
notebooklm notebooks              # List notebooks
notebooklm add-notebook --name "SPEC-KIT" --url "..."

# Queries (direct, bypasses HTTP)
notebooklm ask -n my-notebook "question" --direct

# Health
notebooklm health --deep-check
```

---

## Open Questions

1. **Rate limit detection**: Monitor 429 responses or track client-side?
2. **Multi-account**: Account rotation for higher limits?
3. **Notebook sync**: Detect if notebook content changed?
4. **Circuit breaker**: Automatic fallback after N failures?

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-30 | Initial analysis (MCP interface) |
| 2.0 | 2025-12-14 | Rewritten for HTTP API (MCP removed) |
