# Stage 0: Tier 2 (Divine Truth) Prompt Specification

**Version**: 1.0
**Status**: Design Complete
**Last Updated**: 2025-12-01

---

## 1. Purpose

The Tier 2 prompt queries NotebookLM with the spec content and DCC-generated TASK_BRIEF to produce "Divine Truth" - a structured synthesis that guides downstream agents in the /speckit.auto pipeline.

### Goals

- **Opinionated but grounded**: Based on seeded knowledge, not hallucination
- **Structured output**: 5 sections with predictable format
- **Machine-parseable links**: JSON block for suggested causal relationships
- **Token-efficient**: Under 2000 words total

---

## 2. Seeded Knowledge Sources

NotebookLM notebooks contain these seeded documents:

| Document | Content | Use Case |
|----------|---------|----------|
| `NL_ARCHITECTURE_BIBLE.md` | Core architecture, module boundaries | System design decisions |
| `NL_STACK_JUSTIFICATION.md` | Tech stack choices, dependency rationale | Library/framework questions |
| `NL_BUG_RETROS_*.md` | Bug retrospectives, failure patterns | Anti-pattern avoidance |
| `NL_DEBT_LANDSCAPE.md` | TODO/FIXME clusters, known issues | Risk identification |
| `NL_PROJECT_DIARY_*.md` | Session summaries, progress history | Context and velocity |

### V1: Single Notebook

All documents in one notebook (`codex-rs – Shadow Stage 0`).

### V2+: Multi-Notebook Committee

Documents split by type:
- **Architecture & Stack**: `NL_ARCHITECTURE_BIBLE.md`, `NL_STACK_JUSTIFICATION.md`
- **Bugs & Anti-Patterns**: `NL_BUG_RETROS_*.md`
- **Diary & Ops**: `NL_PROJECT_DIARY_*.md`, `NL_DEBT_LANDSCAPE.md`

---

## 3. Divine Truth Output Schema

### 3.1 Expected Structure

```markdown
# Divine Truth Brief: {SPEC_ID}

## 1. Executive Summary
[3-7 bullet points summarizing the spec intent]

## 2. Architectural Guardrails
[Constraints, patterns, historical decisions that apply]

## 3. Historical Context & Lessons
[Relevant lessons from bugs, diary, debt]

## 4. Risks & Open Questions
[Concrete risks with mitigations or follow-up questions]

## 5. Suggested Causal Links
```json
[
  {
    "from_id": "mem-xxx",
    "to_id": "mem-yyy",
    "type": "causes|solves|contradicts|expands|supersedes",
    "confidence": 0.0-1.0,
    "reasoning": "short explanation"
  }
]
```
```

### 3.2 Causal Link Types

| Type | Meaning | Example |
|------|---------|---------|
| `causes` | A leads to B | "This bug causes that error" |
| `solves` | A resolves B | "This pattern solves that anti-pattern" |
| `contradicts` | A conflicts with B | "This decision contradicts that guideline" |
| `expands` | A builds on B | "This feature expands that module" |
| `supersedes` | A replaces B | "This approach supersedes the old method" |

### 3.3 Token Budget

- **Total**: Under 2000 words (~2500 tokens)
- **Per section**: ~300-400 words max
- **JSON block**: Valid JSON, memory IDs from TASK_BRIEF only

---

## 4. Prompt Template

### 4.1 Full Prompt (sent via NotebookLM MCP)

```text
You are the "Shadow Staff Engineer" for the codex-rs project.

You have access to seeded knowledge files:
- Architecture Bible (system design, module boundaries)
- Stack Justification (tech choices, dependency rationale)
- Bug Retrospectives (failure patterns, anti-patterns)
- Technical Debt Landscape (TODO clusters, known issues)
- Project Diary (session history, progress patterns)

Your job is to synthesize a "Divine Truth" brief for the /speckit.auto pipeline.
This brief guides multiple agents to plan, implement, and validate the spec.

=== OUTPUT FORMAT ===

Follow this structure EXACTLY. Do not add extra sections.

# Divine Truth Brief: {{SPEC_ID}}

## 1. Executive Summary
- Summarize the spec intent in 3-7 bullet points.
- Focus on WHAT is changing and WHY it matters.
- Assume the reader knows codex-rs generally but not this spec.
- Keep to ~200 words.

## 2. Architectural Guardrails
- List architectural constraints or patterns that MUST be respected.
- Reference relevant historical decisions from Architecture Bible.
- Call out potential conflicts with prior decisions.
- Keep to ~300 words.

## 3. Historical Context & Lessons
- Summarize relevant lessons from:
  - Bug Retrospectives / Anti-Patterns
  - Project Diary entries
  - Technical Debt Landscape
- Highlight past failures that intersect with this spec's scope.
- Explain what to do differently this time.
- Keep to ~300 words.

## 4. Risks & Open Questions
- List concrete risks to: correctness, performance, maintainability, DX.
- For each risk, suggest: mitigations, validation strategies, or questions.
- If there are major unknowns, call them out explicitly.
- Keep to ~300 words.

## 5. Suggested Causal Links
Provide a JSON array of suggested relationships between memories.

```json
[
  {
    "from_id": "mem-xxx",
    "to_id": "mem-yyy",
    "type": "causes|solves|contradicts|expands|supersedes",
    "confidence": 0.85,
    "reasoning": "short explanation"
  }
]
```

Rules for JSON:
- ONLY use memory IDs that appear in TASK_BRIEF.md below.
- If no IDs are available, output empty array: []
- Ensure JSON is valid and parseable.
- Include 0-5 relationships (quality over quantity).

=== INPUT DATA ===

SPEC ID: {{SPEC_ID}}

SPEC CONTENT (spec.md):
---
{{SPEC_CONTENT}}
---

TASK BRIEF (TASK_BRIEF.md):
---
{{TASK_BRIEF_MD}}
---

=== INSTRUCTIONS ===

1. Read the spec and task brief carefully.
2. Cross-reference with your seeded knowledge.
3. Synthesize into the 5-section format above.
4. Prefer to mark uncertainties as "Open Questions" rather than hallucinate.
5. Keep total output under 2000 words.
6. Output ONLY the Divine Truth brief. No preamble, no closing remarks.
```

---

## 5. Response Parsing

### 5.1 Section Extraction

```rust
pub struct DivineTruth {
    pub executive_summary: String,
    pub architectural_guardrails: String,
    pub historical_context: String,
    pub risks_and_questions: String,
    pub suggested_links: Vec<CausalLink>,
    pub raw_markdown: String,
}

pub struct CausalLink {
    pub from_id: String,
    pub to_id: String,
    pub link_type: LinkType,
    pub confidence: f64,
    pub reasoning: String,
}

pub enum LinkType {
    Causes,
    Solves,
    Contradicts,
    Expands,
    Supersedes,
}
```

### 5.2 Parsing Logic

```rust
fn parse_divine_truth(response: &str) -> Result<DivineTruth> {
    let raw_markdown = response.to_string();

    // Extract sections by header
    let sections = extract_sections_by_header(response);

    let executive_summary = sections.get("1. Executive Summary")
        .cloned()
        .unwrap_or_default();
    let architectural_guardrails = sections.get("2. Architectural Guardrails")
        .cloned()
        .unwrap_or_default();
    let historical_context = sections.get("3. Historical Context & Lessons")
        .cloned()
        .unwrap_or_default();
    let risks_and_questions = sections.get("4. Risks & Open Questions")
        .cloned()
        .unwrap_or_default();

    // Extract JSON from Section 5
    let suggested_links = extract_causal_links(
        sections.get("5. Suggested Causal Links").unwrap_or(&String::new())
    )?;

    Ok(DivineTruth {
        executive_summary,
        architectural_guardrails,
        historical_context,
        risks_and_questions,
        suggested_links,
        raw_markdown,
    })
}

fn extract_causal_links(section: &str) -> Result<Vec<CausalLink>> {
    // Look for JSON code fence
    let json_pattern = r"```json\s*([\s\S]*?)\s*```";
    let re = Regex::new(json_pattern)?;

    if let Some(caps) = re.captures(section) {
        let json_str = caps.get(1).map(|m| m.as_str()).unwrap_or("[]");
        let links: Vec<CausalLink> = serde_json::from_str(json_str)?;
        return Ok(links);
    }

    // Fallback: try parsing raw section as JSON
    if let Ok(links) = serde_json::from_str::<Vec<CausalLink>>(section.trim()) {
        return Ok(links);
    }

    Ok(vec![]) // No links found
}
```

### 5.3 Validation

```rust
fn validate_causal_links(
    links: Vec<CausalLink>,
    valid_memory_ids: &HashSet<String>,
) -> Vec<CausalLink> {
    links.into_iter()
        .filter(|link| {
            // Only keep links with valid memory IDs
            valid_memory_ids.contains(&link.from_id) &&
            valid_memory_ids.contains(&link.to_id)
        })
        .map(|mut link| {
            // Clamp confidence
            link.confidence = link.confidence.clamp(0.0, 1.0);
            link
        })
        .collect()
}
```

---

## 6. Fallback Handling

### 6.1 When NotebookLM is Unavailable

If Tier 2 call fails, Stage 0 returns a Tier 1-only result:

```rust
fn tier1_fallback(spec_id: &str, task_brief: &str) -> DivineTruth {
    DivineTruth {
        executive_summary: format!(
            "Tier 2 (NotebookLM) unavailable. Using DCC brief only.\n\n{}",
            extract_summary_from_brief(task_brief)
        ),
        architectural_guardrails: "See TASK_BRIEF.md for relevant memories.".to_string(),
        historical_context: "Historical analysis requires Tier 2.".to_string(),
        risks_and_questions: "Risk analysis requires Tier 2.".to_string(),
        suggested_links: vec![],
        raw_markdown: format!("# Divine Truth Brief: {}\n\n[Tier 2 unavailable]", spec_id),
    }
}
```

### 6.2 When Response Doesn't Follow Format

If parsing fails, use the raw response as `raw_markdown` with empty structured fields:

```rust
fn fallback_parse(response: &str, spec_id: &str) -> DivineTruth {
    DivineTruth {
        executive_summary: "Unable to parse structured response.".to_string(),
        architectural_guardrails: String::new(),
        historical_context: String::new(),
        risks_and_questions: String::new(),
        suggested_links: vec![],
        raw_markdown: response.to_string(),
    }
}
```

---

## 7. Notebook Routing (V2+)

When multi-notebook is enabled, use IQO's `notebook_focus` to route:

```rust
fn select_notebooks(iqo: &IQO, config: &Stage0Config) -> Vec<String> {
    if !config.multi_notebook_enabled {
        return vec![config.notebook_id_shadow.clone()];
    }

    let mut notebooks = vec![];

    for focus in &iqo.notebook_focus {
        match focus.as_str() {
            "architecture" => {
                if let Some(id) = &config.notebook_arch_id {
                    notebooks.push(id.clone());
                }
            }
            "bugs" => {
                if let Some(id) = &config.notebook_bugs_id {
                    notebooks.push(id.clone());
                }
            }
            "diary" => {
                if let Some(id) = &config.notebook_diary_id {
                    notebooks.push(id.clone());
                }
            }
            _ => {}
        }
    }

    // Fallback to shadow if no specific notebooks selected
    if notebooks.is_empty() {
        notebooks.push(config.notebook_id_shadow.clone());
    }

    notebooks
}
```

### 7.1 Committee Merging (Future)

When querying multiple notebooks:

```rust
async fn query_committee(
    notebooks: &[String],
    prompt: &str,
) -> Result<DivineTruth> {
    let mut responses = vec![];

    for notebook_id in notebooks {
        let response = notebooklm_ask(notebook_id, prompt).await?;
        responses.push((notebook_id.clone(), response));
    }

    merge_divine_truths(responses)
}

fn merge_divine_truths(
    responses: Vec<(String, DivineTruth)>
) -> Result<DivineTruth> {
    // Merge strategy:
    // - Concatenate sections with source attribution
    // - Union causal links (dedupe by from_id + to_id)
    // - Combine raw_markdown with separators
    todo!("V2.8 implementation")
}
```

---

## 8. Integration

### 8.1 Code Location

```
codex-rs/stage0/src/tier2/
├── mod.rs
├── prompt.rs         # Prompt templates (this spec)
├── notebook.rs       # NotebookLM MCP client
├── cache.rs          # Tier 2 cache operations
└── parser.rs         # Divine Truth parsing
```

### 8.2 Usage in Stage 0

```rust
// In tier2/mod.rs
pub async fn get_divine_truth(
    &self,
    spec_id: &str,
    spec_content: &str,
    task_brief: &str,
    iqo: &IQO,
    config: &Stage0Config,
) -> Result<DivineTruthResult> {
    // Check cache first
    let cache_key = compute_cache_key(spec_content, task_brief);
    if let Some(cached) = self.cache.get(&cache_key).await? {
        return Ok(DivineTruthResult {
            divine_truth: cached,
            cache_hit: true,
            tier2_used: true,
            latency_ms: 0,
        });
    }

    // Select notebook(s)
    let notebooks = select_notebooks(iqo, config);

    // Build prompt
    let prompt = build_tier2_prompt(spec_id, spec_content, task_brief);

    // Query NotebookLM
    let start = Instant::now();
    let response = self.query_notebooks(&notebooks, &prompt).await;
    let latency_ms = start.elapsed().as_millis() as u64;

    match response {
        Ok(raw_response) => {
            let divine_truth = parse_divine_truth(&raw_response)?;

            // Cache the result
            self.cache.set(&cache_key, &divine_truth).await?;

            Ok(DivineTruthResult {
                divine_truth,
                cache_hit: false,
                tier2_used: true,
                latency_ms,
            })
        }
        Err(e) => {
            // Tier 2 failed, use fallback
            tracing::warn!("Tier 2 failed: {}, using fallback", e);
            Ok(DivineTruthResult {
                divine_truth: tier1_fallback(spec_id, task_brief),
                cache_hit: false,
                tier2_used: false,
                latency_ms,
            })
        }
    }
}
```

---

## 9. Testing

### 9.1 Unit Tests

```rust
#[test]
fn test_parse_divine_truth_extracts_sections() {
    let response = include_str!("../test_fixtures/divine_truth_sample.md");
    let dt = parse_divine_truth(response).unwrap();

    assert!(!dt.executive_summary.is_empty());
    assert!(!dt.architectural_guardrails.is_empty());
}

#[test]
fn test_parse_causal_links_from_json() {
    let section = r#"
```json
[{"from_id": "mem-1", "to_id": "mem-2", "type": "causes", "confidence": 0.9, "reasoning": "test"}]
```
"#;
    let links = extract_causal_links(section).unwrap();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].link_type, LinkType::Causes);
}

#[test]
fn test_validate_causal_links_filters_invalid() {
    let links = vec![
        CausalLink { from_id: "mem-1".into(), to_id: "mem-2".into(), ..Default::default() },
        CausalLink { from_id: "mem-1".into(), to_id: "mem-invalid".into(), ..Default::default() },
    ];
    let valid_ids: HashSet<_> = ["mem-1", "mem-2"].iter().map(|s| s.to_string()).collect();

    let validated = validate_causal_links(links, &valid_ids);
    assert_eq!(validated.len(), 1);
}
```

### 9.2 Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires NotebookLM access
async fn test_tier2_full_flow() {
    let spec_content = "Add user authentication feature";
    let task_brief = "## Relevant Memories\n- mem-123: Auth patterns...";

    let result = get_divine_truth(
        "SPEC-TEST-001",
        spec_content,
        task_brief,
        &IQO::default(),
        &Stage0Config::default(),
    ).await?;

    assert!(!result.divine_truth.raw_markdown.is_empty());
}
```

---

## 10. Example Output

### Input

**SPEC-KIT-102**: Add NotebookLM integration for Stage 0

**TASK_BRIEF.md** (excerpt):
```markdown
## Relevant Memories
- mem-abc123: Decision to use overlay pattern for local-memory
- mem-def456: Anti-pattern: Don't modify closed-source daemon internals
- mem-ghi789: Architecture decision for MCP-based integrations
```

### Expected Divine Truth

```markdown
# Divine Truth Brief: SPEC-KIT-102

## 1. Executive Summary
- This spec adds NotebookLM as a Tier 2 deep research layer for Stage 0.
- NotebookLM acts as a "Shadow Staff Engineer" providing architectural guidance.
- Integration uses MCP tools (ask_question, list_notebooks, etc.).
- Results are cached in overlay DB with TTL-based invalidation.
- This enables richer context than local-memory alone can provide.

## 2. Architectural Guardrails
- **Overlay pattern required**: local-memory is closed-source; all intelligence lives in Stage 0's overlay layer (ref: Architecture Bible).
- **MCP-only access**: Use public MCP tools, never internal APIs (ref: mem-ghi789).
- **Cache-first**: Always check cache before calling NotebookLM to respect rate limits.
- **Graceful degradation**: If NotebookLM is unavailable, fall back to Tier 1 (DCC-only).

## 3. Historical Context & Lessons
- **Prior attempt at daemon modification failed** (ref: mem-def456). The overlay approach was adopted specifically to avoid this.
- **MCP integrations have been reliable** (ref: Project Diary). The pattern is proven.
- **Rate limits are real**: NotebookLM has daily query limits. Previous integrations that ignored this caused service disruptions.

## 4. Risks & Open Questions
- **Risk: NotebookLM rate limits** - Mitigation: Aggressive caching (24h TTL), cache warming.
- **Risk: Response format instability** - Mitigation: Robust parsing with fallbacks.
- **Open question**: Which notebook should be the primary? Single vs. committee?
- **Open question**: How to handle notebook source management (currently manual)?

## 5. Suggested Causal Links
```json
[
  {
    "from_id": "mem-abc123",
    "to_id": "mem-def456",
    "type": "causes",
    "confidence": 0.85,
    "reasoning": "Overlay decision was made because modifying daemon failed"
  },
  {
    "from_id": "mem-ghi789",
    "to_id": "mem-abc123",
    "type": "expands",
    "confidence": 0.75,
    "reasoning": "MCP architecture decision informed overlay implementation"
  }
]
```
```

---

*Spec generated from research session P73 (2025-12-01)*
