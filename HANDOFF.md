# Session 35 Prompt - GPU Activation & Source Registry

**Last updated:** 2026-01-04
**Status:** S35 Active - RTX 5090 INSTALLED ✅
**Primary SPEC:** SPEC-SOURCE-MGMT + GPU Activation

---

## 🎉 MILESTONE: RTX 5090 32GB Installed

**Verified:** 2026-01-04

```
NVIDIA GeForce RTX 5090
├── VRAM: 32607 MiB (32GB)
├── Driver: 580.105.08
├── CUDA: 13.0
└── Status: Idle, ready for workloads
```

### What This Unlocks

| Blocked Task | Previous State | Now Possible |
|--------------|---------------|--------------|
| **vLLM runtime** | Not possible (no GPU) | Default runtime per MODEL-POLICY.md |
| **fast_local tier** | Ollama CPU (4.67s latency) | 14B planner + 32B coder on GPU |
| **Stage 0 IQO** | qwen2.5:3b CPU (4.67s) | GPU-accelerated inference (<100ms) |
| **Embeddings (bge-m3)** | CPU-only | GPU acceleration |
| **Template Guardian** | CPU batch processing | Real-time GPU inference |

### Priority: vLLM Setup Tasks

| Order | Task | Status | Notes |
|-------|------|--------|-------|
| 1 | Install vLLM + CUDA dependencies | Pending | `pip install vllm` (Python 3.10+) |
| 2 | Download 14B planner model | Pending | Qwen2.5-14B-Instruct or similar |
| 3 | Download 32B coder model | Pending | DeepSeek-Coder-33B-Instruct or CodeLlama-34B |
| 4 | Configure vLLM server | Pending | OpenAI-compatible API on localhost:8000 |
| 5 | Wire TUI to vLLM endpoint | Pending | Update model router for fast_local tier |
| 6 | Update local-llm-requirements.md | Pending | Document new hardware profile |
| 7 | Benchmark fast_local latency | Pending | Target: <500ms for 14B, <1s for 32B |

### Model Selection (MODEL-POLICY.md §8)

```
fast_local tier:
├── Planner (14B): Qwen2.5-14B-Instruct @ FP16 (~28GB VRAM)
│   └── Alternative: Mistral-7B-Instruct (~14GB) for faster inference
├── Coder (32B): DeepSeek-Coder-33B-Instruct @ INT8 (~17GB VRAM)
│   └── Alternative: CodeLlama-34B-Instruct (~17GB)
└── Total: Fits in 32GB with model swapping
```

---

## Session 34 Accomplishments

| Item | Status | Location |
|------|--------|----------|
| CLI delete-source bug | ✅ FIXED & VERIFIED | service-client.ts:627-631 |
| Upsert title return | ✅ FIXED & VERIFIED | sources.ts:389-418 |
| Off-by-one delete | ✅ FIXED & VERIFIED | sources.ts:361 |
| Source registry schema | Designed | `docs/SPEC-SOURCE-MGMT/spec.md` |
| Source cleanup | Done | 13 → 5 sources |

---

## Bugs Fixed in notebooklm-client (All Verified)

### 1. CLI delete-source "Invalid JSON response" ✅ FIXED

**Fix applied (service-client.ts:627-631):**
```typescript
if (body) {
  const bodyStr = JSON.stringify(body);
  req.setHeader("Content-Length", Buffer.byteLength(bodyStr).toString());
  req.write(bodyStr);
}
```

**Verified:** `notebooklm delete-source -i 2 --json` returns valid JSON

### 2. Upsert returns NLM-generated title ✅ FIXED + REFINED

**Initial fix (sources.ts:389-418):**
- Added `listSources()` call after adding source
- Returns `nlmTitle` field

**Refinement applied (sources.ts:389-435):**
- Before/after source comparison for reliable title discovery
- Compares source list before vs after add
- Finds new sources by title difference
- Correctly returns NLM-generated title even when unpredictable

**Verified:** Input "S34_FINAL_TEST" → returns "The Final Synthesis of Source Comparison"

**Remaining limitation:** Upsert delete fuzzy matching still fails when NLM titles lack matching words. Registry needed for reliable updates.

### 3. Off-by-one in upsert delete ✅ FIXED

**Fix applied (sources.ts:361):**
```typescript
// Removed incorrect -1 offset
await sourceManager.deleteSource(notebookUrl, existingSource.index, {...})
```

**Verified:** Delete works correctly with 1-based index

---

## Source Registry Implementation (S35)

### Phase 1: Prerequisites ✅ COMPLETE

All bugs fixed and verified. Ready for registry implementation.

### Phase 2: Add better-sqlite3 to notebooklm-client

```bash
cd ~/notebooklm-client
npm install better-sqlite3
npm install -D @types/better-sqlite3
```

### Phase 3: Implement SourceRegistry class

**Location:** `~/notebooklm-client/src/sources/source-registry.ts`

**Schema:** See `~/code/docs/SPEC-SOURCE-MGMT/spec.md`

### Phase 4: Integrate with handleUpsertSource

1. Lookup existing source by (notebook_id, spec_id, source_type)
2. If found, delete by stored notebooklm_title
3. After add, update registry with new title

---

## Current Source State

```
Sources in notebook (5 total):
  1. Divine Truth Tier 2 SPEC Analysis Framework [ready]
  2. Golden Path Dogfooding Validation: Stage0 Verification Spec [ready]
  3. Protocol for Active Testing Specifications [ready]
  4. The Codex TUI Dogfooding Protocol [ready]
  5. Golden Path Dogfooding Validation: SPEC-DOGFOOD-001 [ready]
```

**Static sources (don't delete):** 1, 3, 4
**Dynamic sources (managed by registry):** 2, 5

---

## Key Files

| Location | Purpose |
|----------|---------|
| `~/code/docs/SPEC-SOURCE-MGMT/spec.md` | Registry architecture spec |
| `~/notebooklm-client/src/client/service-client.ts` | HTTP client (Content-Length fix) |
| `~/notebooklm-client/src/service/handlers/sources.ts` | Upsert handler (title return) |
| `~/notebooklm-client/src/sources/source-deleter.ts` | Delete implementation |

---

## Commits (S34)

| Commit | Description |
|--------|-------------|
| (pending) | docs: SPEC-SOURCE-MGMT and S34 milestone |

---

## Test Results

All fixes verified:
```bash
# Delete-source CLI works
notebooklm delete-source -i 2 --json
# Returns: {"success": true, "data": {"deletedTitle": "..."}}

# Upsert returns NLM title
curl -X POST ".../api/sources/upsert" -d '{"name": "...", "content": "..."}'
# Returns: {"data": {"nlmTitle": "...", "action": "created"}}
```
