# SPEC-KIT-102 Implementation Plan

**Date**: 2025-11-30 | **Status**: Ready for Approval

---

## 1. Architecture Assessment

### Current State

```
[local-memory daemon]        CLOSED-SOURCE (Go binary from localmemory.co)
├── MCP Interface            Full CRUD + graph + analysis tools
├── SQLite Database          Directly accessible (~/.local-memory/unified-memories.db)
├── Qdrant                   Running on localhost:6333
└── Ollama                   Configured (qwen2.5:3b, nomic-embed-text)
```

**Constraint**: We cannot modify the local-memory daemon's internals.

### Proposed Architecture

```
[codex-rs TUI / Claude Code]
        │
        │ (POST /api/v1/request_synthesis)
        ▼
┌─────────────────────────────────────────────────────────────┐
│           SPEC-KIT-102 ORCHESTRATOR (NEW)                   │
│                      Python Service                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │ Ingestion       │  │ Dynamic Context │                   │
│  │ Guardians       │  │ Compiler (DCC)  │                   │
│  │ - Metadata      │  │ - Hybrid Search │                   │
│  │ - Template      │  │ - Ranking       │                   │
│  └────────┬────────┘  └────────┬────────┘                   │
│           │                    │                             │
│  ┌────────▼────────────────────▼────────┐                   │
│  │         Synthesis Manager            │                   │
│  │  - Cache (SQLite)                    │                   │
│  │  - Dependency Tracking               │                   │
│  │  - NotebookLM Integration            │                   │
│  └──────────────────────────────────────┘                   │
└───────────────────────────┬─────────────────────────────────┘
                            │
          ┌─────────────────┼─────────────────┐
          ▼                 ▼                 ▼
    [local-memory]    [Ollama]         [NotebookLM MCP]
       (MCP)        (qwen2.5:3b)          (Tier 2)
```

---

## 2. Implementation Phases

### Phase 0: Schema Preparation (Direct SQLite)

**Objective**: Prepare database for new functionality without modifying daemon.

**Tasks**:

1. **Backup database**
   ```bash
   cp ~/.local-memory/unified-memories.db ~/.local-memory/unified-memories.db.pre-102
   ```

2. **Add orchestrator-specific columns** (additive, won't break daemon)
   ```sql
   -- Dynamic scoring columns
   ALTER TABLE memories ADD COLUMN usage_count INTEGER DEFAULT 0;
   ALTER TABLE memories ADD COLUMN last_accessed_at DATETIME;
   ALTER TABLE memories ADD COLUMN dynamic_score FLOAT DEFAULT 0.0;
   ALTER TABLE memories ADD COLUMN structure_status TEXT DEFAULT 'unstructured';

   -- Index for efficient sorting
   CREATE INDEX IF NOT EXISTS idx_dynamic_score ON memories(dynamic_score DESC);
   CREATE INDEX IF NOT EXISTS idx_structure_status ON memories(structure_status);
   ```

3. **Create orchestrator cache tables** (separate file)
   ```sql
   -- ~/.local-memory/orchestrator-cache.db
   CREATE TABLE tier2_synthesis_cache (
       input_hash TEXT PRIMARY KEY,
       synthesis_result TEXT NOT NULL,
       suggested_links TEXT,
       created_at DATETIME NOT NULL,
       hit_count INTEGER DEFAULT 0,
       last_hit_at DATETIME
   );

   CREATE TABLE cache_memory_dependencies (
       cache_hash TEXT NOT NULL,
       memory_id TEXT NOT NULL,
       PRIMARY KEY (cache_hash, memory_id)
   );
   CREATE INDEX idx_dependency_memory_id ON cache_memory_dependencies(memory_id);
   ```

4. **Normalize existing timestamps** (safe transformation)
   ```sql
   UPDATE memories SET
     created_at = substr(created_at, 1, 26)
   WHERE created_at LIKE '%m=+%';
   ```

5. **Extract agent_type from tags**
   ```sql
   UPDATE memories
   SET agent_type = (
     SELECT replace(json_each.value, 'agent:', '')
     FROM json_each(memories.tags)
     WHERE json_each.value LIKE 'agent:%'
     LIMIT 1
   )
   WHERE tags LIKE '%agent:%' AND agent_type = 'unknown';
   ```

**Deliverables**:
- `scripts/phase0_migration.sql`
- `scripts/phase0_migrate.py` (safe runner with rollback)

---

### Phase 1: Orchestrator Core (Python Service)

**Objective**: Create the foundation service that wraps local-memory.

**Location**: `/home/thetu/code/spec-kit-102-orchestrator/`

**Structure**:
```
spec-kit-102-orchestrator/
├── pyproject.toml
├── src/
│   ├── __init__.py
│   ├── main.py                 # FastAPI app
│   ├── config.py               # Configuration
│   ├── models.py               # Pydantic models
│   ├── database.py             # SQLite connections
│   ├── guardians/
│   │   ├── __init__.py
│   │   ├── metadata.py         # Metadata Guardian
│   │   └── template.py         # Template Guardian (Ollama)
│   ├── dcc/
│   │   ├── __init__.py
│   │   └── compiler.py         # Dynamic Context Compiler
│   ├── synthesis/
│   │   ├── __init__.py
│   │   ├── cache.py            # Cache manager
│   │   └── notebooklm.py       # Tier 2 integration
│   └── local_memory/
│       ├── __init__.py
│       └── client.py           # MCP/CLI wrapper
└── tests/
```

**Key Components**:

1. **Local Memory Client** (`local_memory/client.py`)
   - Wrapper around `local-memory` CLI for fastest operations
   - Fallback to MCP for complex operations
   - Connection pooling for SQLite direct access

2. **Configuration** (`config.py`)
   ```python
   class Config:
       LOCAL_MEMORY_DB = "~/.local-memory/unified-memories.db"
       ORCHESTRATOR_CACHE_DB = "~/.local-memory/orchestrator-cache.db"
       OLLAMA_BASE_URL = "http://localhost:11434"
       OLLAMA_MODEL = "qwen2.5:3b"
       NOTEBOOKLM_ENABLED = True
   ```

3. **FastAPI Endpoints**:
   - `POST /api/v1/memories` - Guarded storage
   - `POST /api/v1/compile_context` - DCC
   - `POST /api/v1/request_synthesis` - Full Stage 0 workflow
   - `GET /api/v1/health` - Service health

**Deliverables**:
- Working service skeleton
- Local memory client wrapper
- Basic health endpoint

---

### Phase 2: Ingestion Guardians

**Objective**: Implement quality control for memory ingestion.

**Metadata Guardian** (`guardians/metadata.py`):
```python
class MetadataGuardian:
    def validate(self, memory: MemoryInput) -> ValidationResult:
        errors = []

        # Timestamp validation
        if not self._is_valid_timestamp(memory.created_at):
            errors.append("Invalid timestamp format")

        # Agent attribution
        if memory.agent_type in (None, 'unknown'):
            errors.append("Agent attribution required")

        # Importance threshold
        if memory.importance < 8:
            errors.append(f"Importance {memory.importance} below threshold 8")

        return ValidationResult(valid=len(errors) == 0, errors=errors)
```

**Template Guardian** (`guardians/template.py`):
```python
class TemplateGuardian:
    REQUIRED_TEMPLATE = """
    [PATTERN|DECISION|PROBLEM|INSIGHT]: <summary>
    CONTEXT: <situation>
    REASONING: <why>
    OUTCOME: <result>
    """

    async def process(self, memory: MemoryInput) -> MemoryInput:
        if self._matches_template(memory.content):
            memory.structure_status = 'structured'
            return memory

        # Queue for restructuring
        memory.structure_status = 'pending'
        await self._queue_restructure(memory)
        return memory

    async def _restructure_with_ollama(self, content: str) -> str:
        prompt = f"""Restructure this memory. Be concise (max 100 words):

"{content}"

Format:
[TYPE]: <one line summary>
CONTEXT: <situation>
REASONING: <why this approach>
OUTCOME: <result or impact>
"""
        return await ollama.generate(model="qwen2.5:3b", prompt=prompt)
```

**Background Worker**:
- Process `structure_status='pending'` memories
- Update to 'structured' after Ollama processing
- ~4.7s per memory (acceptable for background)

**Deliverables**:
- MetadataGuardian with validation rules
- TemplateGuardian with Ollama integration
- Background restructuring worker

---

### Phase 3: Dynamic Context Compiler (DCC)

**Objective**: Generate task-specific context briefs.

**DCC Pipeline** (`dcc/compiler.py`):

```python
class DynamicContextCompiler:
    async def compile(self, task_spec: str, top_k: int = 20) -> ContextBrief:
        # 1. Intent Analysis
        intent = self._analyze_intent(task_spec)

        # 2. Metadata Pre-filter (SQLite - fast)
        candidates = await self._prefilter_by_metadata(
            domains=intent.domains,
            tags=intent.tags,
            min_importance=7
        )

        # 3. Semantic Search (Qdrant via local-memory)
        if len(candidates) > top_k * 3:
            candidates = await self._semantic_rank(
                query=task_spec,
                memory_ids=[m.id for m in candidates],
                limit=top_k * 2
            )

        # 4. Dynamic Score Ranking
        ranked = self._rank_by_dynamic_score(candidates)[:top_k]

        # 5. Update usage tracking
        await self._update_access_tracking(ranked)

        # 6. Format output
        return ContextBrief(
            task_brief=self._format_task_brief(ranked),
            memories_used=[m.id for m in ranked],
            explain=self._generate_explanation(ranked) if self.explain_mode else None
        )

    def _calculate_dynamic_score(self, memory: Memory) -> float:
        """Utility-based scoring with novelty boost"""
        base_score = memory.initial_priority / 10.0

        # Usage weight
        usage_weight = min(memory.usage_count / 10.0, 1.0) * 0.3

        # Recency weight
        days_old = (now() - memory.last_accessed_at).days
        recency_weight = max(0, 1 - days_old / 30) * 0.2

        # Novelty boost (cold start solution)
        novelty_boost = 0.2 if memory.usage_count < 5 else 0

        return min(1.0, base_score + usage_weight + recency_weight + novelty_boost)
```

**Output Format** (`TASK_BRIEF.md`):
```markdown
# Task Brief: [SPEC-ID]

## Relevant Historical Context

### Memory 1: [ID] (Score: 0.87)
[PATTERN]: Native routing beats AI consensus
CONTEXT: SPEC-KIT-070 optimization
REASONING: 10,000x faster, $8.30 savings
OUTCOME: Adopted as default approach

### Memory 2: [ID] (Score: 0.82)
...

## Intent Analysis
- Domains: spec-kit, infrastructure
- Related SPECs: SPEC-KIT-070, SPEC-KIT-067
- Potential Risks: [identified patterns]
```

**Deliverables**:
- DCC implementation with hybrid search
- Dynamic scoring algorithm with novelty boost
- Explainability mode (`?explain=true`)
- TASK_BRIEF.md formatter

---

### Phase 4: Synthesis Manager & NotebookLM Integration

**Objective**: Implement cache and Tier 2 integration.

**Cache Manager** (`synthesis/cache.py`):
```python
class SynthesisCache:
    def __init__(self, db_path: str):
        self.db = sqlite3.connect(db_path)

    def get(self, input_hash: str) -> Optional[SynthesisResult]:
        row = self.db.execute(
            "SELECT * FROM tier2_synthesis_cache WHERE input_hash = ?",
            (input_hash,)
        ).fetchone()
        if row:
            self.db.execute(
                "UPDATE tier2_synthesis_cache SET hit_count = hit_count + 1, last_hit_at = ? WHERE input_hash = ?",
                (datetime.now(), input_hash)
            )
            return SynthesisResult.from_row(row)
        return None

    def store(self, input_hash: str, result: SynthesisResult, memory_ids: List[str]):
        # Store result
        self.db.execute(
            "INSERT INTO tier2_synthesis_cache VALUES (?, ?, ?, ?, 0, NULL)",
            (input_hash, result.synthesis, result.suggested_links, datetime.now())
        )
        # Record dependencies (CRITICAL for invalidation)
        for mem_id in memory_ids:
            self.db.execute(
                "INSERT OR IGNORE INTO cache_memory_dependencies VALUES (?, ?)",
                (input_hash, mem_id)
            )
        self.db.commit()

    def invalidate_for_memory(self, memory_id: str):
        """MANDATORY: Called when any memory is updated/deleted"""
        affected = self.db.execute(
            "SELECT cache_hash FROM cache_memory_dependencies WHERE memory_id = ?",
            (memory_id,)
        ).fetchall()
        for (cache_hash,) in affected:
            self.db.execute("DELETE FROM tier2_synthesis_cache WHERE input_hash = ?", (cache_hash,))
        self.db.commit()
```

**NotebookLM Integration** (`synthesis/notebooklm.py`):
```python
class NotebookLMSynthesizer:
    TIER2_PROMPT = """You are the "Staff Engineer" (Tier 2 Reasoning Layer)...
    [Full prompt from Artifact D]
    """

    async def synthesize(self, spec: str, task_brief: str) -> SynthesisResult:
        # Use existing MCP bridge
        response = await mcp_notebooklm_ask_question(
            question=self._format_synthesis_request(spec, task_brief),
            notebook_id=self.config.SPECKIT_NOTEBOOK_ID
        )
        return self._parse_response(response)
```

**Main Endpoint** (`main.py`):
```python
@app.post("/api/v1/request_synthesis")
async def request_synthesis(request: SynthesisRequest) -> SynthesisResponse:
    # 1. Compile context
    context = await dcc.compile(request.spec_content)

    # 2. Check cache
    input_hash = hash_inputs(request.spec_content, context.task_brief)
    cached = cache.get(input_hash)
    if cached:
        return SynthesisResponse(
            divine_truth=cached.synthesis,
            source="cache",
            memories_used=context.memories_used
        )

    # 3. Query Tier 2 (NotebookLM)
    if notebooklm.is_available() and notebooklm.budget_remaining > 0:
        result = await notebooklm.synthesize(request.spec_content, context.task_brief)

        # 4. Store with dependencies
        cache.store(input_hash, result, context.memories_used)

        # 5. Ingest causal links back to Tier 1
        if result.suggested_links:
            await local_memory.ingest_relationships(result.suggested_links)

        return SynthesisResponse(
            divine_truth=result.synthesis,
            source="notebooklm",
            memories_used=context.memories_used
        )

    # Fallback: Return task brief only
    return SynthesisResponse(
        divine_truth=context.task_brief,
        source="tier1_only",
        memories_used=context.memories_used
    )
```

**Deliverables**:
- SynthesisCache with dependency-aware invalidation
- NotebookLM integration via MCP
- `/api/v1/request_synthesis` endpoint
- Fallback handling

---

### Phase 5: Observability & Integration

**Objective**: Add logging, metrics, and codex-rs integration.

**Logging Schema** (for V3 learning):
```python
@dataclass
class QueryLog:
    timestamp: datetime
    query_type: str  # 'dcc', 'synthesis', 'search'
    input_hash: str
    memories_retrieved: List[str]
    cache_hit: bool
    tier2_used: bool
    latency_ms: int
    success: bool
```

**Codex-RS Integration**:
- Add orchestrator endpoint to `/speckit.auto` Stage 0
- Configure via environment variable: `SPECKIT_ORCHESTRATOR_URL`

**Health Monitoring**:
```python
@app.get("/api/v1/health")
async def health():
    return {
        "status": "ok",
        "local_memory": await check_local_memory(),
        "ollama": await check_ollama(),
        "notebooklm": await check_notebooklm_mcp(),
        "cache_stats": cache.get_stats(),
        "tier2_budget": notebooklm.get_remaining_budget()
    }
```

**Deliverables**:
- Structured logging for all operations
- Health endpoint with component status
- Integration documentation for codex-rs

---

## 3. Timeline & Dependencies

```
Phase 0 (Prerequisites)     ─────┐
  Schema migration               │
  Data cleanup                   │
                                 ▼
Phase 1 (Core)             ──────┬─────
  Service skeleton               │
  Local memory client            │
                                 ▼
Phase 2 (Guardians)        ──────┬─────
  Metadata Guardian              │
  Template Guardian + Ollama     │
                                 ▼
Phase 3 (DCC)              ──────┬─────
  Hybrid search                  │
  Dynamic scoring                │
  TASK_BRIEF formatter           │
                                 ▼
Phase 4 (Synthesis)        ──────┬─────
  Cache + Invalidation           │
  NotebookLM integration         │
  Main synthesis endpoint        │
                                 ▼
Phase 5 (Observability)    ──────┘
  Logging, metrics
  Codex-RS integration
```

---

## 4. Decisions Made

| Question | Decision | Notes |
|----------|----------|-------|
| Orchestrator Location | **Inside codex-rs** | `codex-rs/orchestrator/` (monorepo approach) |
| NotebookLM Setup | TBD at implementation | Create new notebook when implementing |
| Phase 0 Migration | TBD at implementation | Test on copy first recommended |
| Service Startup | TBD | Options: systemd, alongside local-memory, on-demand |

**Status**: Research phase complete. Implementation pending.

---

## 5. Risk Assessment

| Risk | Mitigation |
|------|------------|
| Schema migration breaks daemon | Additive columns only, test on backup first |
| NotebookLM rate limits | Cache aggressively, 30/day budget allocation |
| Ollama latency | Background processing, ~4.7s acceptable |
| Cold start (novelty) | Explicit novelty boost in scoring |
| Cache staleness | Dependency-aware invalidation (mandatory) |

---

## 6. Success Criteria

- [ ] Phase 0: All 1161 memories have normalized timestamps
- [ ] Phase 1: Orchestrator responds on `/api/v1/health`
- [ ] Phase 2: Invalid memories rejected with clear errors
- [ ] Phase 3: DCC returns ranked context in <500ms
- [ ] Phase 4: Cache hit rate >60% after warmup
- [ ] Phase 5: Full Stage 0 workflow completes successfully

---

**Ready for implementation approval.**
