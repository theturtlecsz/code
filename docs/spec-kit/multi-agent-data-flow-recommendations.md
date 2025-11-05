# Multi-Agent Data Flow Architectural Recommendations

## Executive Summary

The spec-kit system faces exponential prompt growth due to naive concatenation of agent outputs, leading to OS argument limit errors. After researching industry best practices and analyzing the current implementation, I recommend a **tiered compression strategy** combining JSON extraction, semantic summarization, and structured field extraction.

## Current Problem Analysis

### The Exponential Growth Pattern

```rust
// Current naive implementation in agent_orchestrator.rs:
// Agent 1: 5KB output
// Agent 2: Gets Agent 1's full 5KB + generates 8KB = 13KB total
// Agent 3: Gets 13KB + generates 10KB = 23KB total
// Plan stage output: 50KB (includes all nested outputs)
// Tasks stage: Gets full 50KB plan.md → explodes
```

**Lines 273-306 in agent_orchestrator.rs** show the problematic pattern:
- Each agent receives ALL previous agent outputs via `${PREVIOUS_OUTPUTS}`
- plan.md/tasks.md files are included with only basic truncation (20KB limit)
- Nested agent outputs within these files compound the problem

### Specific Issues

1. **Sequential Accumulation** (Lines 274-323): Each agent in sequential mode gets injected with all previous outputs
2. **File Inclusion** (Lines 111-141): Prior stage outputs (plan.md, tasks.md) contain embedded agent outputs
3. **No Intelligent Extraction**: Full raw outputs passed, including timestamps, debug info, metadata
4. **Truncation Too Late**: 20KB limit applied AFTER exponential growth has occurred

## Research Findings

### Industry Best Practices (2024)

#### 1. **Microsoft LLMLingua Approach**
- **Prompt Compression**: Achieves 20x compression using small LMs to identify unimportant tokens
- **Semantic Preservation**: Maintains meaning while removing redundancy
- **Applicability**: Could use GPT-2 small for identifying key content

#### 2. **LangGraph Pattern (LangChain)**
- **Directed Graphs**: Each node processes specific data, edges control flow
- **State Management**: Shared state object, not full output passing
- **Key Insight**: Pass references and deltas, not full content

#### 3. **CrewAI Approach**
- **Structured Hand-offs**: JSON schemas define what passes between agents
- **Task-Specific Fields**: Only relevant fields extracted and passed
- **Two-Layer Architecture**: Crews (dynamic) vs Flows (deterministic)

#### 4. **Context Engineering (2024 Trend)**
- **"Just-Right Information"**: Fill context window with only essential data
- **Dynamic Selection**: Choose what to include based on current task
- **Deduplication**: Remove redundant information across sources

## Architectural Recommendations

### Pattern Hierarchy (Implement in Order)

#### **Pattern 1: JSON-Only Extraction** ⭐ PRIORITY 1
**Implementation Effort**: 2-4 hours
**Compression**: 60-80%
**Quality Loss**: Minimal

```rust
fn extract_json_only(raw_output: &str) -> Result<String, String> {
    // Find JSON boundaries
    let start = raw_output.find('{').ok_or("No JSON found")?;
    let end = raw_output.rfind('}').ok_or("No JSON close found")?;

    let json_str = &raw_output[start..=end];

    // Validate and clean
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Invalid JSON: {}", e))?;

    // Re-serialize compactly
    serde_json::to_string(&value)
        .map_err(|e| format!("Serialization failed: {}", e))
}

// Usage in agent_orchestrator.rs line 279:
let output_to_inject = extract_json_only(prev_output)
    .unwrap_or_else(|_| prev_output.to_string());
```

#### **Pattern 2: Structured Field Extraction** ⭐ PRIORITY 2
**Implementation Effort**: 4-6 hours
**Compression**: 70-85%
**Quality Loss**: Very Low

```rust
#[derive(Serialize, Deserialize)]
struct AgentSummary {
    agent: String,
    stage: String,
    key_decisions: Vec<String>,  // Max 3-5 bullets
    work_items: Vec<String>,      // For plan/tasks stages
    risks: Vec<String>,           // Critical risks only
    consensus_points: Vec<String>, // Areas of agreement
}

fn extract_stage_essentials(
    stage: SpecStage,
    raw_output: &str
) -> Result<AgentSummary, String> {
    let json: Value = serde_json::from_str(
        &extract_json_only(raw_output)?
    )?;

    match stage {
        SpecStage::Plan => {
            // Extract only: work_breakdown, risks, acceptance_criteria
            Ok(AgentSummary {
                agent: json["agent"].as_str().unwrap_or("unknown").to_string(),
                stage: "plan".to_string(),
                key_decisions: extract_array(&json, "decisions", 5),
                work_items: extract_array(&json, "work_breakdown", 10),
                risks: extract_array(&json, "risks", 3),
                consensus_points: vec![],
            })
        },
        SpecStage::Tasks => {
            // Extract only: task IDs, priorities, dependencies
            Ok(AgentSummary {
                agent: json["agent"].as_str().unwrap_or("unknown").to_string(),
                stage: "tasks".to_string(),
                key_decisions: extract_array(&json, "priorities", 3),
                work_items: extract_task_ids(&json),
                risks: vec![],
                consensus_points: vec![],
            })
        },
        _ => {
            // Generic extraction
            Ok(AgentSummary {
                agent: json["agent"].as_str().unwrap_or("unknown").to_string(),
                stage: stage.to_string(),
                key_decisions: extract_array(&json, "summary", 3),
                work_items: vec![],
                risks: extract_array(&json, "concerns", 3),
                consensus_points: extract_array(&json, "agreements", 5),
            })
        }
    }
}

fn extract_array(json: &Value, key: &str, max: usize) -> Vec<String> {
    json.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .take(max)
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}
```

#### **Pattern 3: Semantic Summarization** ⭐ PRIORITY 3
**Implementation Effort**: 6-8 hours
**Compression**: 90-95%
**Quality Loss**: Medium (requires validation)

```rust
async fn semantic_summarization(
    stage: SpecStage,
    agent_outputs: Vec<(String, String)>
) -> Result<String, String> {
    // Use a small, fast model for summarization
    let summary_prompt = format!(
        "Summarize these {} agent outputs into 3-5 key points:\n\n{}",
        stage,
        agent_outputs.iter()
            .map(|(name, out)| format!("## {}\n{}", name,
                extract_json_only(out).unwrap_or(out.clone())))
            .collect::<Vec<_>>()
            .join("\n\n")
    );

    // Call cheap model (e.g., Claude Haiku, Gemini Flash)
    // Cost: ~$0.001 per summarization
    let summary = call_summarizer_model(&summary_prompt).await?;

    Ok(summary)
}
```

#### **Pattern 4: Hierarchical Context Windows** ⭐ FUTURE
**Implementation Effort**: 8-12 hours
**Compression**: Dynamic (50-95%)
**Quality Loss**: Minimal with good heuristics

```rust
struct HierarchicalContext {
    // Level 1: Executive summary (50-100 chars)
    executive_summary: String,

    // Level 2: Key points per stage (500 chars)
    stage_summaries: HashMap<SpecStage, String>,

    // Level 3: Recent context window (last 2-3 agents)
    recent_context: VecDeque<AgentSummary>,

    // Level 4: Full context (on demand)
    full_context_path: Option<PathBuf>,
}

impl HierarchicalContext {
    fn for_agent(&self, agent_name: &str, stage: SpecStage) -> String {
        // Intelligent selection based on agent needs
        match (stage, agent_name) {
            (SpecStage::Implement, "gpt_codex") => {
                // Code generation needs detailed context
                self.get_detailed_context()
            },
            (SpecStage::Validate, _) => {
                // Validation needs plan + tasks summaries
                self.get_stage_summaries(&[SpecStage::Plan, SpecStage::Tasks])
            },
            _ => {
                // Most agents need just recent + executive
                format!("{}\n\nRecent:\n{}",
                    self.executive_summary,
                    self.format_recent_context())
            }
        }
    }
}
```

### Implementation Strategy

#### Phase 1: Immediate Fix (2-4 hours)
1. Implement JSON-only extraction
2. Apply to lines 279-291 in agent_orchestrator.rs
3. Add logging to track compression ratios

```rust
// Quick fix for agent_orchestrator.rs:
// Replace lines 279-291 with:
let output_to_inject = extract_json_only(prev_output)
    .map(|json| {
        let original_len = prev_output.len();
        let compressed_len = json.len();
        let ratio = 100.0 * (1.0 - compressed_len as f64 / original_len as f64);
        tracing::info!("Compressed {} output: {} → {} chars ({:.1}% reduction)",
            prev_agent_name, original_len, compressed_len, ratio);
        json
    })
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to extract JSON from {}: {}", prev_agent_name, e);
        prev_output.to_string()
    });
```

#### Phase 2: Structured Extraction (4-6 hours)
1. Define `AgentSummary` struct
2. Implement stage-specific extractors
3. Update prompt templates to expect summaries
4. Test with each stage type

#### Phase 3: Cross-Stage Optimization (6-8 hours)
1. Clean plan.md before including in tasks context
2. Extract only work_breakdown and risks
3. Store full outputs separately, pass references
4. Implement summary persistence

#### Phase 4: Advanced Patterns (Future)
1. Implement semantic summarization service
2. Add vector store for RAG-based retrieval
3. Build hierarchical context management
4. Add adaptive compression based on token count

### Monitoring & Validation

```rust
#[derive(Debug, Serialize)]
struct CompressionMetrics {
    stage: SpecStage,
    agent: String,
    original_size: usize,
    compressed_size: usize,
    compression_ratio: f64,
    extraction_time_ms: u64,
    quality_score: Option<f64>,  // From consensus checking
}

impl CompressionMetrics {
    fn log(&self) {
        tracing::info!(
            "Compression: {} {} - {}→{} chars ({:.1}% reduction) in {}ms",
            self.stage, self.agent, self.original_size, self.compressed_size,
            self.compression_ratio * 100.0, self.extraction_time_ms
        );

        // Alert if compression is insufficient
        if self.compressed_size > 10_000 {
            tracing::warn!("⚠️ Large output after compression: {} chars", self.compressed_size);
        }
    }
}
```

## Decision Principles

### Principle 1: Preserve Signal, Eliminate Noise
- Keep: Decisions, work items, risks, consensus points
- Remove: Timestamps, debug info, repetitive content, metadata

### Principle 2: Stage-Aware Compression
- Plan/Tasks: Aggressive compression (90%+)
- Implement: Moderate compression (70%)
- Validate/Audit: Preserve more detail (60%)

### Principle 3: Progressive Enhancement
- Start with simple JSON extraction
- Add intelligence incrementally
- Measure impact at each step

### Principle 4: Fail Gracefully
- If extraction fails, fall back to truncation
- Log all compression failures
- Never lose critical information

### Principle 5: Cost-Aware Summarization
- Use cheapest model that maintains quality
- Batch summarizations when possible
- Cache summaries for reuse

## Success Metrics

| Metric | Current | Target | Stretch Goal |
|--------|---------|--------|--------------|
| Avg prompt size | 50KB+ | 10KB | 5KB |
| OS limit errors | Frequent | Zero | Zero |
| Compression ratio | 0% | 70% | 85% |
| Quality degradation | N/A | <5% | <2% |
| Implementation time | - | 8 hrs | 4 hrs |
| Cost per stage | $0.35 | $0.35 | $0.30 |

## Risk Assessment

### Low Risk
- JSON extraction: Well-understood, minimal quality impact
- Field extraction: Predictable, schema-based

### Medium Risk
- Semantic summarization: Quality depends on model
- Cross-stage cleaning: May lose some context

### High Risk
- Full RAG implementation: Complex, may not be worth it
- Aggressive compression: Could lose critical information

## Recommended Action Plan

### Immediate (Today)
1. ✅ Implement JSON-only extraction
2. ✅ Deploy to agent_orchestrator.rs
3. ✅ Monitor compression metrics

### Short Term (Week 1)
4. Build AgentSummary struct and extractors
5. Update prompt templates
6. Test with all stage types
7. Measure quality impact

### Medium Term (Week 2)
8. Implement cross-stage optimization
9. Add semantic summarization for plan→tasks
10. Build compression monitoring dashboard

### Long Term (Month 1+)
11. Evaluate need for advanced patterns
12. Consider RAG if data volume justifies
13. Optimize based on usage patterns

## Code Locations to Modify

1. **agent_orchestrator.rs:279-291** - Primary injection point
2. **agent_orchestrator.rs:111-141** - File inclusion logic
3. **consensus.rs** - Add compression before storage
4. **native_consensus_executor.rs** - Apply to native aggregation
5. **prompts.json** - Update templates to expect summaries

## Conclusion

The exponential prompt growth is solvable with a tiered compression strategy. Start with simple JSON extraction (2-4 hours, 70% compression), then progressively add intelligence. This maintains quality while eliminating the OS argument limit errors.

**Recommended first step**: Implement Pattern 1 (JSON extraction) immediately to unblock the system, then iterate based on measured impact.