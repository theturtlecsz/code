# Local LLM Requirements Analysis

**Date**: 2025-11-30 (P72)
**Use Case**: Template Guardian for memory restructuring

---

## Executive Summary

| Metric | Value | Assessment |
|--------|-------|------------|
| Available GPU | None | CPU-only inference required |
| System RAM | 64GB (58GB free) | Exceeds requirements |
| CPU | Intel Xeon Gold 6132 @ 2.60GHz | Server-grade, good for inference |
| Model tested | qwen2.5:3b (1.9GB) | ✓ Suitable |
| Warm latency | **4.67 seconds** | Acceptable |
| Output quality | Good (with proper prompting) | ✓ Production-ready |

**Recommendation**: Use qwen2.5:3b for Template Guardian. CPU inference is acceptable for batch processing.

---

## Hardware Profile

```
CPU:    Intel(R) Xeon(R) Gold 6132 CPU @ 2.60GHz
        14 cores, server-grade, AVX-512 support
RAM:    64GB total, ~58GB available
GPU:    None (CPU-only inference)
Disk:   SSD (fast model loading)
```

---

## Model Benchmarks

### qwen2.5:3b (1.9GB)

**Available locally**: Yes

**Benchmark Results**:

| Condition | Latency | Notes |
|-----------|---------|-------|
| Cold start | ~45s | Model loading into RAM |
| Warm (cached) | **4.67s** | Production performance |

**Output Quality Test**:

Input:
```
"Discovered that Qdrant vector search returns stale results when
embeddings updated without explicit flush. Fixed by adding
collection.flush() call after update operations."
```

Output:
```
WHAT: Ensured Qdrant's embeddings are flushed to maintain up-to-date search results.
WHY: Without explicit flush, outdated embedding data led to stale search results.
TAGS: Qdrant, vector-search, stale-results, update-operations, collection.flush()
```

**Assessment**: Good quality restructuring with concise prompting.

---

## Alternative Models (Not Tested)

| Model | Size | VRAM/RAM | Notes |
|-------|------|----------|-------|
| qwen2.5:7b | ~4.5GB | 5GB+ | Higher quality, 2x latency |
| llama3.2:3b | ~2GB | 2GB+ | Similar to qwen, different style |
| phi-3-mini | ~2.3GB | 2.5GB+ | Microsoft, good at structured output |

**Recommendation**: qwen2.5:3b is sufficient. Only consider 7b variant if quality issues arise in production.

---

## Integration Architecture

### Template Guardian Design

```
┌─────────────────────────────────────────────────────────────┐
│                    Template Guardian                         │
├─────────────────────────────────────────────────────────────┤
│  Input: Raw memory content                                   │
│                                                              │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────────┐  │
│  │ Validation  │ -> │ qwen2.5:3b   │ -> │ Schema Check  │  │
│  │ (fast)      │    │ restructure  │    │ (fast)        │  │
│  └─────────────┘    └──────────────┘    └───────────────┘  │
│                                                              │
│  Output: Structured memory with WHAT/WHY/CONTEXT/TAGS       │
└─────────────────────────────────────────────────────────────┘
```

### Processing Strategy

1. **Batch Processing** (recommended for bulk operations)
   - Queue incoming memories
   - Process in batches of 10-20
   - Amortize cold start cost
   - Target: 50-100 memories/minute

2. **On-Demand Processing** (for single memories)
   - Keep model warm with keepalive
   - Latency: 4-5 seconds acceptable
   - Timeout: 30 seconds

### Ollama Configuration

```bash
# Keep model warm (5 minute keepalive)
OLLAMA_KEEP_ALIVE=5m ollama serve

# Or set per-request
curl -X POST http://localhost:11434/api/generate \
  -d '{"model": "qwen2.5:3b", "keep_alive": "5m", ...}'
```

---

## Prompt Engineering

### Optimal Prompt Template

```
Restructure this memory. Be extremely concise (max 100 words total):

"{raw_memory_content}"

Format:
WHAT: <one line factual summary>
WHY: <reasoning or motivation>
TAGS: <comma separated, lowercase>
```

### Anti-Patterns

- ❌ Asking for multiple formats (causes duplication)
- ❌ Long system prompts (increases latency)
- ❌ Open-ended requests (verbose output)

---

## Capacity Planning

### Current Dataset

- 1,161 memories (existing)
- ~15-20 new memories/day

### Processing Times

| Operation | Time | Notes |
|-----------|------|-------|
| Full reprocessing | ~90 minutes | 1,161 × 4.67s |
| Daily batch | ~2 minutes | 20 × 4.67s |
| Single memory | 4.67s | Warm model |

### Resource Usage During Inference

```
Model RAM:     ~2GB (loaded)
Peak RAM:      ~3GB (during generation)
CPU Usage:     100% (single core)
Disk I/O:      Minimal (model cached)
```

---

## Open Questions

1. **Model persistence**: Should we run ollama as a daemon with warm model?
2. **Fallback**: What happens if Ollama unavailable? Skip restructuring or queue?
3. **Quality monitoring**: How to detect restructuring quality degradation?
4. **Batch size**: What's the optimal batch size for throughput?

---

## Recommendations

1. **Use qwen2.5:3b** - Already installed, good quality, acceptable latency
2. **Run Ollama as daemon** - Keep model warm for <5s latency
3. **Batch processing** - Process 10-20 memories at once for efficiency
4. **Add timeout handling** - 30s max per memory, skip on failure
5. **Monitor quality** - Sample review of restructured outputs monthly

---

## Next Steps

- [ ] Create Template Guardian implementation spec
- [ ] Design batch processing queue
- [ ] Add Ollama health check to local-memory startup
- [ ] Create restructuring quality test suite
