# STAGE0_ERROR_TAXONOMY.md

## Purpose

Define Stage0 error categories and how they map to:

- `Stage0Engine::run_stage0` return values,
- logs / observability,
- `/speckit.auto` behavior (soft vs hard failure).

Default policy: **soft failure** → log + skip Stage 0 (degraded mode) unless otherwise specified.

---

## Error Categories

Each error has:

- `category`: one of the categories below,
- `code`: machine-readable string (e.g. `"overlay_db_connection_failed"`),
- `message`: human-readable description,
- `context`: structured fields (spec_id, request_id, etc.).

### 1. CONFIG_ERROR

**Definition:** `stage0.toml` or env misconfigured.

Examples:

- Missing `db_path`,
- invalid `notebook_id_shadow`,
- parse error in config file.

**Behavior:**

- Log `stage0_error` with `category=CONFIG_ERROR`.
- `run_stage0` returns an error.
- `/speckit.auto`:
  - shows a short warning ("Stage 0 disabled due to config error"),
  - runs in degraded mode (no Stage 0) for that run.

---

### 2. OVERLAY_DB_ERROR

**Definition:** Errors creating/connecting/querying overlay SQLite.

Examples:

- DB file not found / permission denied,
- migration failure,
- SQL errors.

**Behavior:**

- Log `stage0_error` with `category=OVERLAY_DB_ERROR`.
- `run_stage0` returns an error.
- `/speckit.auto`: degraded mode (skip Stage 0 for this run).

---

### 3. LOCAL_MEMORY_ERROR

**Definition:** Failures when Stage0 talks to local-memory via MCP/REST.

Examples:

- connection refused / timeout,
- malformed response,
- MCP tool not found.

**Behavior:**

- If Stage0 can't get any usable candidates:
  - log LOCAL_MEMORY_ERROR,
  - return error → `/speckit.auto` degraded.
- If partial data:
  - log warning,
  - continue DCC with reduced context if still meaningful.

---

### 4. DCC_ERROR

**Definition:** Failures inside DCC logic.

Examples:

- IQO JSON from LLM not parseable,
- ranking pipeline panicked,
- summarization prompt failed repeatedly.

**Behavior:**

- If IQO fails:
  - log PROMPT_ERROR (see below),
  - fall back to a heuristic IQO (simple domains/tags/keywords).
- If ranking/summarization fails:
  - log DCC_ERROR,
  - `run_stage0` returns error → `/speckit.auto` degraded.

---

### 5. TIER2_ERROR (NotebookLM / MCP)

**Definition:** Errors calling NotebookLM via MCP.

Examples:

- network issues,
- MCP tool not registered,
- timeout,
- invalid response.

**Behavior:**

- DCC already produced `TASK_BRIEF.md`:
  - on Tier2 failure:
    - log TIER2_ERROR,
    - Stage0 returns:
      - `divine_truth` = Tier1-only fallback brief (small summary derived locally),
      - `cache_hit = false`,
      - error context in logs.
- `/speckit.auto` continues using Stage 0 context, minus Tier2 enrichment.

---

### 6. PROMPT_ERROR

**Definition:** Prompt/response formatting issues for IQO or Tier2.

Examples:

- IQO LLM returned non-JSON,
- Tier2 JSON causal links section not parseable.

**Behavior:**

- IQO:
  - log PROMPT_ERROR,
  - fallback to heuristic IQO.
- Tier2 links JSON:
  - ignore links (treat as empty array),
  - still use textual parts of Divine Truth.

---

### 7. INTERNAL_ERROR

**Definition:** Unexpected logic bugs/panics in Stage0.

**Behavior:**

- Log INTERNAL_ERROR with as much diagnostic detail as possible.
- `run_stage0` returns error.
- `/speckit.auto`: degraded mode for that run.

---

## Error-to-Behavior Matrix

| Category          | DCC Output? | Tier2 Output? | /speckit.auto Mode            |
|-------------------|------------:|--------------:|-------------------------------|
| CONFIG_ERROR      | no          | no            | Degraded (skip Stage 0)       |
| OVERLAY_DB_ERROR  | no          | no            | Degraded                      |
| LOCAL_MEMORY_ERROR| maybe       | no            | Degraded if DCC unusable      |
| DCC_ERROR         | no          | no            | Degraded                      |
| TIER2_ERROR       | yes         | fallback      | Stage0 Tier1-only brief       |
| PROMPT_ERROR      | maybe       | partial       | Continue with partial context |
| INTERNAL_ERROR    | no          | no            | Degraded                      |

"Degraded" = `/speckit.auto` continues without Stage 0 output (or with Tier1-only brief, in the Tier2_ERROR case), but records the reason.

---

## Logging Shape

Each error yields a `stage0_error` event, for example:

```json
{
  "timestamp": "2025-11-30T15:42:01Z",
  "request_id": "3c1e31c4-9f13-4f2b-9e9a-0a034f3b9c5b",
  "spec_id": "SPEC-KIT-102",
  "category": "TIER2_ERROR",
  "code": "notebooklm_timeout",
  "message": "NotebookLM MCP timed out after 30 seconds",
  "context": {
    "notebook_id": "YOUR-NOTEBOOK-ID",
    "timeout_seconds": 30
  }
}
```

`STAGE0_OBSERVABILITY.md` defines the broader event schema; this file just defines categories and expected behaviors.
