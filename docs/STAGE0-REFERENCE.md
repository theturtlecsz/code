# Stage 0 Reference

> **Version**: 1.0.0 (2026-01-22)
>
> **Purpose**: Technical reference for the Stage 0 overlay engine integration, internal contracts, and configuration.
>
> **Supersedes**: All files in `docs/stage0/`

***

## Table of Contents

* [Part I: Architecture & Integration](#part-i-architecture--integration)
  * [1. Overview](#1-overview)
    * [Design Goals](#design-goals)
    * [Architecture](#architecture)
    * [Component Layers](#component-layers)
  * [2. Core Types](#2-core-types)
    * [Stage0Result](#stage0result)
    * [Stage0Explain](#stage0explain)
    * [Stage0Config](#stage0config)
    * [EnvContext](#envcontext)
  * [3. Engine API](#3-engine-api)
    * [Stage0Engine](#stage0engine)
    * [Orchestration Steps](#orchestration-steps)
  * [4. Pipeline Integration](#4-pipeline-integration)
    * [Integration Point](#integration-point)
    * [State Changes](#state-changes)
    * [Context Injection](#context-injection)
* [Part II: Dynamic Context Compiler (DCC)](#part-ii-dynamic-context-compiler-dcc)
  * [5. Intent Query Object (IQO)](#5-intent-query-object-iqo)
    * [Schema](#schema)
    * [Field Definitions](#field-definitions)
    * [Domain Taxonomy](#domain-taxonomy)
    * [Tag Namespaces](#tag-namespaces)
    * [IQO Generation Prompt](#iqo-generation-prompt)
    * [Validation](#validation)
    * [Fallback IQO](#fallback-iqo)
  * [6. Dynamic Relevance Scoring](#6-dynamic-relevance-scoring)
    * [Overlay Data](#overlay-data)
    * [Scoring Formula](#scoring-formula)
    * [Default Weights](#default-weights)
  * [7. DCC Pipeline](#7-dcc-pipeline)
    * [Pipeline Steps](#pipeline-steps)
  * [8. Task Brief Output](#8-task-brief-output)
    * [File Structure](#file-structure)
  * [SPEC CONTENT:](#spec-content)
  * [{{SPEC\_CONTENT}}](#spec_content)
  * [TASK BRIEF:](#task-brief)
  * [{{TASK\_BRIEF\_MD}}](#task_brief_md)
    * [Parsing Logic](#parsing-logic)
    * [Link Validation](#link-validation)
    * [Fallback Handling](#fallback-handling)
* [Part IV: Guardians & Quality](#part-iv-guardians--quality)
  * [12. Metadata Guardian](#12-metadata-guardian)
    * [Responsibilities](#responsibilities)
    * [Implementation](#implementation)
  * [13. Template Guardian](#13-template-guardian)
    * [Template Format](#template-format)
    * [Prompt](#prompt)
    * [Processing Flow](#processing-flow)
  * [14. Error Taxonomy](#14-error-taxonomy)
    * [Error Categories](#error-categories)
    * [Error-to-Behavior Matrix](#error-to-behavior-matrix)
    * [Error Logging](#error-logging)
    * [Soft Failure Policy](#soft-failure-policy)
* [Part V: Configuration](#part-v-configuration)
  * [15. Configuration Schema](#15-configuration-schema)
    * [Config File Location](#config-file-location)
    * [Full Schema](#full-schema)
  * [16. Environment Variables](#16-environment-variables)
  * [17. CLI Flags](#17-cli-flags)
    * [Example Usage](#example-usage)
    * [TUI Feedback](#tui-feedback)
* [Appendices](#appendices)
  * [A. Implementation Checklist](#a-implementation-checklist)
    * [Files to Create (Stage 0 crate)](#files-to-create-stage-0-crate)
    * [Files to Modify (TUI integration)](#files-to-modify-tui-integration)
  * [B. Related Documentation](#b-related-documentation)
  * [C. Change History](#c-change-history)

# Part I: Architecture & Integration

## 1. Overview

Stage 0 is a **Shadow Stage** that runs BEFORE the Plan stage in `/speckit.auto`. It provides enriched context (Divine Truth + TASK\_BRIEF) to all subsequent stages.

### Design Goals

1. **Non-invasive**: Minimal changes to existing pipeline code
2. **Graceful degradation**: Pipeline continues if Stage 0 fails
3. **Observable**: Clear logging and TUI feedback
4. **Configurable**: Toggles for Stage 0 and Tier 2 (NotebookLM)
5. **Cacheable**: Results persist for repeated runs

### Architecture

```
/speckit.auto (codex-rs)
     │
     ▼
[Stage0 Overlay Engine]
     │
     ├──> Guardians on new memories → store via CLI/REST
     │
     ├──> DCC:
     │       - IQO via local LLM
     │       - local-memory.search / analysis
     │       - overlay scores
     │       - diversity reranking
     │       - summarization → TASK_BRIEF.md
     │
     ├──> Tier 2:
     │       - check overlay Tier2 cache
     │       - call NotebookLM (on miss)
     │       - store synthesis + dependencies
     │       - ingest suggested links
     │
     ▼
Divine Truth + TASK_BRIEF → fed into Stages 1–6
```

### Component Layers

| Layer    | Component             | Description                     |
| -------- | --------------------- | ------------------------------- |
| Tier 0.9 | local-memory daemon   | Black box, REST/CLI API only    |
| Tier 1   | Stage0 Overlay Engine | Scoring, DCC, caching (you own) |
| Tier 2   | NotebookLM            | "Staff Engineer" synthesis      |

***

## 2. Core Types

### Stage0Result

```rust
/// Result of a Stage 0 execution
#[derive(Debug, Clone)]
pub struct Stage0Result {
    /// Tier 2 synthesis from NotebookLM (or Tier 1 fallback)
    pub divine_truth: String,

    /// DCC-compiled context brief in markdown
    pub task_brief_md: String,

    /// IDs of local-memory memories used (for cache invalidation)
    pub memories_used: Vec<String>,

    /// Whether Tier 2 cache was hit
    pub cache_hit: bool,

    /// Whether Tier 2 (NotebookLM) was actually used
    pub tier2_used: bool,

    /// Execution latency
    pub latency_ms: u64,

    /// Optional explainability data (when explain=true)
    pub explain: Option<Stage0Explain>,
}
```

### Stage0Explain

```rust
/// Explainability data for debugging/tuning
#[derive(Debug, Clone)]
pub struct Stage0Explain {
    /// Generated Intent Query Object
    pub iqo: IntentQueryObject,

    /// Top-K candidates with scores
    pub candidates: Vec<ScoredCandidate>,

    /// Scoring component breakdown per candidate
    pub score_breakdown: Vec<ScoreBreakdown>,
}
```

### Stage0Config

```rust
/// Per-run configuration (from config + CLI flags)
#[derive(Debug, Clone)]
pub struct Stage0Config {
    /// Master enable switch
    pub enabled: bool,

    /// Enable explainability output
    pub explain: bool,

    /// Enable Tier 2 (NotebookLM) calls
    pub tier2_enabled: bool,

    /// Tier 2 cache TTL in hours
    pub tier2_cache_ttl_hours: u32,

    /// Maximum memories to retrieve
    pub max_candidates: usize,

    /// Top-K after reranking
    pub top_k: usize,
}

impl Default for Stage0Config {
    fn default() -> Self {
        Self {
            enabled: true,
            explain: false,
            tier2_enabled: true,
            tier2_cache_ttl_hours: 24,
            max_candidates: 50,
            top_k: 10,
        }
    }
}
```

### EnvContext

```rust
/// Environment context passed from /speckit.auto
#[derive(Debug, Clone)]
pub struct EnvContext {
    /// Current working directory
    pub cwd: PathBuf,

    /// Git branch (if available)
    pub git_branch: Option<String>,

    /// Recent files touched (for relevance hints)
    pub recent_files: Vec<PathBuf>,

    /// SPEC directory path
    pub spec_dir: PathBuf,
}
```

***

## 3. Engine API

### Stage0Engine

```rust
impl Stage0Engine {
    /// Create a new Stage0Engine instance
    pub async fn new(overlay_db_path: &Path) -> anyhow::Result<Self>;

    /// Main entry point - called by /speckit.auto before Plan stage
    pub async fn run_stage0(
        &self,
        spec_id: &str,
        spec_content: &str,
        env: &EnvContext,
        config: &Stage0Config,
    ) -> anyhow::Result<Stage0Result>;

    /// Check if Stage 0 is available (overlay DB initialized, etc.)
    pub fn is_available(&self) -> bool;

    /// Get config from file (with defaults)
    pub fn load_config() -> Stage0Config;
}
```

### Orchestration Steps

The `run_stage0` function executes:

1. **Compile Context** - Call DCC to produce `task_brief_md` and `memories_used`
2. **Hashing & Cache Lookup** - Compute `input_hash = hash(spec_hash + brief_hash)`
3. **Tier 2 Call** (on cache miss) - Query NotebookLM with Tier 2 prompt
4. **Cache Store** - Insert into `tier2_synthesis_cache` with dependencies
5. **Causal Link Ingestion** - Push suggested relationships to local-memory
6. **Observability** - Record structured `stage0_run` event

***

## 4. Pipeline Integration

### Integration Point

Stage 0 executes in `handle_spec_auto()` AFTER state creation but BEFORE `advance_spec_auto()`:

```
handle_spec_auto()
    │
    ├─ Validate config
    ├─ Load PipelineConfig
    ├─ Check evidence size limits
    ├─ Create SpecAutoState
    │
    ├─ ═══════════════════════════════════
    │   STAGE 0 INSERTION POINT
    │   - Check if Stage0 enabled in config
    │   - Call Stage0Engine::run_stage0()
    │   - Store Stage0Result in SpecAutoState
    │   - Write task_brief.md to SPEC directory
    │   - Display TUI status
    │  ═══════════════════════════════════
    │
    └─ advance_spec_auto()
           └─ Plan → Tasks → Implement → ...
```

### State Changes

Add to `SpecAutoState`:

```rust
pub struct SpecAutoState {
    // ... existing fields ...

    /// Stage 0 result (Divine Truth + TASK_BRIEF)
    pub stage0_result: Option<Stage0Result>,

    /// Stage 0 config used for this run
    pub stage0_config: Stage0Config,

    /// Whether Stage 0 was skipped (disabled or failed)
    pub stage0_skipped: bool,

    /// Stage 0 skip reason (for logging/display)
    pub stage0_skip_reason: Option<String>,
}
```

### Context Injection

Two injection points for downstream agents:

**1. File-based**: Write to SPEC directory (persists, inspectable)

```
docs/SPEC-{ID}/
├── spec.md           # Original spec
├── task_brief.md     # DCC-compiled context (NEW)
├── divine_truth.md   # Tier 2 synthesis (NEW, optional)
├── plan.md           # Plan stage output
└── ...
```

**2. Prompt-based**: Inject into agent prompts (runtime)

```rust
if let Some(stage0_result) = &state.stage0_result {
    context.push_str("## Stage 0: Task Context Brief\n");
    context.push_str(&stage0_result.task_brief_md);
    context.push_str("\n\n");

    if stage0_result.tier2_used && !stage0_result.divine_truth.is_empty() {
        context.push_str("## Stage 0: Divine Truth (NotebookLM)\n");
        context.push_str(&stage0_result.divine_truth);
        context.push_str("\n\n");
    }
}
```

***

# Part II: Dynamic Context Compiler (DCC)

## 5. Intent Query Object (IQO)

### Schema

```jsonc
{
  "domains": ["spec-kit", "infrastructure"],
  "required_tags": ["spec:SPEC-KIT-102"],
  "optional_tags": ["stage:plan", "type:pattern"],
  "keywords": ["NotebookLM", "Tiered Memory", "Stage 0"],
  "max_candidates": 150,
  "notebook_focus": ["architecture", "bugs"],
  "confidence": 0.85
}
```

### Field Definitions

| Field            | Type      | Required | Description                                 |
| ---------------- | --------- | -------- | ------------------------------------------- |
| `domains`        | string\[] | Yes      | 0-3 high-level project areas                |
| `required_tags`  | string\[] | Yes      | Tags that MUST be present in candidates     |
| `optional_tags`  | string\[] | Yes      | Tags that bias retrieval (not hard filters) |
| `keywords`       | string\[] | Yes      | 3-10 phrases for semantic search            |
| `max_candidates` | number    | Yes      | Upper bound for pre-filter (max 150)        |
| `notebook_focus` | string\[] | No       | Hints for Tier 2 notebook routing           |
| `confidence`     | number    | No       | 0.0-1.0, overall IQO confidence             |

### Domain Taxonomy

```
spec-kit       # Spec-kit pipeline, automation
infrastructure # Build, CI/CD, deployment
tui            # TUI implementation, widgets
cli            # CLI commands, parsing
core           # Core library, protocols
mcp            # MCP client/server
ollama         # Ollama integration
testing        # Test infrastructure
docs           # Documentation, specs
```

### Tag Namespaces

| Prefix       | Example             | Meaning                   |
| ------------ | ------------------- | ------------------------- |
| `spec:`      | `spec:SPEC-KIT-102` | Links to a specific SPEC  |
| `stage:`     | `stage:plan`        | Pipeline stage context    |
| `type:`      | `type:pattern`      | Memory classification     |
| `component:` | `component:dcc`     | Codebase component        |
| `agent:`     | `agent:claude`      | Agent that created memory |

### IQO Generation Prompt

```text
You are a search-intent extraction assistant for the codex-rs project.

Given the following specification and environment context, generate an
Intent Query Object (IQO).

The IQO JSON MUST have this exact shape:

{
  "domains": string[],          // 0-3 high-level project areas
  "required_tags": string[],    // tags that MUST match
  "optional_tags": string[],    // tags that bias retrieval
  "keywords": string[],         // 3-10 phrases for semantic search
  "max_candidates": number,     // integer, max 150
  "notebook_focus": string[],   // hints: "architecture", "bugs", "diary"
  "confidence": number          // 0.0-1.0, your confidence in this IQO
}

SPEC CONTENT:
---
{{SPEC_CONTENT}}
---

ENVIRONMENT CONTEXT:
- cwd: {{CWD}}
- branch: {{BRANCH}}
- recent_files: {{RECENT_FILES}}

Output ONLY the JSON object. No commentary.
```

### Validation

```rust
fn validate_iqo(mut iqo: IQO) -> IQO {
    // Clamp max_candidates to 150
    const MAX_CANDIDATES_LIMIT: usize = 150;
    if iqo.max_candidates > MAX_CANDIDATES_LIMIT {
        iqo.max_candidates = MAX_CANDIDATES_LIMIT;
    }

    // Filter unknown domains
    let known_domains = ["spec-kit", "infrastructure", "tui", "cli",
                         "core", "mcp", "ollama", "testing", "docs"];
    iqo.domains.retain(|d| known_domains.contains(&d.as_str()));

    // Filter unknown notebook_focus
    let known_focus = ["architecture", "bugs", "diary"];
    iqo.notebook_focus.retain(|f| known_focus.contains(&f.as_str()));

    // Clamp confidence
    iqo.confidence = iqo.confidence.clamp(0.0, 1.0);

    // Limit keywords to 10
    iqo.keywords.truncate(10);

    iqo
}
```

### Fallback IQO

If parsing fails, use minimal fallback:

```rust
fn fallback_iqo(spec_id: &str, spec_content: &str) -> IQO {
    IQO {
        domains: vec![],
        required_tags: vec![format!("spec:{}", spec_id)],
        optional_tags: vec![],
        keywords: extract_nouns_heuristically(spec_content, 5),
        max_candidates: 150,
        notebook_focus: vec!["architecture".to_string()],
        confidence: 0.3,
    }
}
```

***

## 6. Dynamic Relevance Scoring

### Overlay Data

Each overlay row stores:

| Field              | Description                   |
| ------------------ | ----------------------------- |
| `memory_id`        | local-memory's ID             |
| `initial_priority` | 1–10 (from importance)        |
| `usage_count`      | Times used in Stage 0 context |
| `last_accessed_at` | Last Stage 0 use              |
| `dynamic_score`    | Computed utility score        |

### Scoring Formula

Let:

* `U = usage_count`
* `P = initial_priority` (1–10)
* `T_access = last_accessed_at` or `created_at`
* `T_create = created_at`

```
usage_score   = min(1.0, log(1 + U) / log(6))
recency_days  = max(0, days(now - T_access))
recency_score = exp(-ln(2) * recency_days / 7)
priority_score= clamp(P, 1, 10) / 10.0
age_days      = max(0, days(now - T_create))
age_penalty   = 1.0 - exp(-ln(2) * age_days / 30)
```

Novelty boost (for fresh memories):

```
if U < novelty_boost_threshold:
    novelty_factor = 1.0 + novelty_boost_factor_max * (1 - U / threshold)
else:
    novelty_factor = 1.0
```

Final score:

```
base_score = w_usage * usage_score
           + w_recency * recency_score
           + w_priority * priority_score
           - w_decay * age_penalty

dynamic_score = clamp(base_score * novelty_factor, 0.0, 1.5)
```

### Default Weights

```yaml
weights:
  usage: 0.30
  recency: 0.30
  priority: 0.25
  decay: 0.15
novelty_boost_threshold: 5
novelty_boost_factor_max: 0.5
```

***

## 7. DCC Pipeline

### Pipeline Steps

```rust
pub async fn compile_context(
    &self,
    spec: &str,
    env: &EnvCtx,
    explain: bool,
) -> CompileContextResult
```

**Step 1: IQO Generation**

* Call local LLM with spec + env to get IQO JSON
* Fallback to simple heuristic if LLM fails

**Step 2: Local-Memory Query**

* Use IQO to construct search requests
* Retrieve candidate memories (ID + content + tags)

**Step 3: Join & Score Combination**

```
final_score = semantic_similarity_weight * similarity_score
            + dynamic_score_weight * dynamic_score
```

**Step 4: Diversity Reranking (MMR)**

```
Selected = []
Candidates = sort_by(final_score desc)

while len(Selected) < top_k and Candidates not empty:
    for c in Candidates:
        diversity_penalty = max_{s in Selected} sim(c, s)
        mmr_score = λ * c.final_score - (1-λ) * diversity_penalty
    pick highest mmr_score → move to Selected
```

**Step 5: Summarization**

* Summarize each selected memory
* Combine into TASK\_BRIEF.md
* Enforce `max_tokens` by dropping lowest-value entries

**Step 6: Explainability (Optional)**

* If `explain=true`, return per-memory score breakdown

***

## 8. Task Brief Output

### File Structure

````markdown
# Task Brief: {{SPEC_ID}}

## 1. Spec Snapshot
### 1.1 Summary
- <3–7 bullet summary>

### 1.2 Key Objectives
- <bullet list of goals>

### 1.3 Non-Goals
- <optional out-of-scope items>

## 2. Relevant Context (Memories)
### 2.1 High-Priority Memories
#### Memory 1 – {{MEMORY_ID}}
- **Type:** [PATTERN | DECISION | PROBLEM | INSIGHT]
- **Score:** {{combined_score}} (sim={{similarity}}, dyn={{dynamic}})
- **Tags:** {{tags}}
- **Summary:** <2–4 sentences>
> Excerpt: "{{short excerpt}}"

### 2.2 Supporting Memories
- {{MEMORY_ID}} – {{type}} – {{1-line summary}} (score={{score}})

## 3. Code Context (Optional)
### 3.1 Key Code Units
- **Location:** `{{repo}}/{{path}}`
- **Role:** <description>
- **Why relevant:** <link to spec>

## 4. Docs & Issues Context (Optional)
- **{{DOC_ID}} – {{title}}**
  - Type: [SPEC | ADR | DOC]
  - Summary: <1–3 sentences>

## 5. Inferred Constraints & Assumptions
### 5.1 Hard Constraints
- [C1] <constraint> – Backed by: `mem-123`

### 5.2 Working Assumptions
- [A1] <assumption>

## 6. Known Risks & Pitfalls
### 6.1 Risks
- [R1] <risk> – Impact: [low|medium|high] – Source: <IDs>

### 6.2 Historical Pitfalls
- [P1] <anti-pattern> – Linked to: `mem-abc`

## 7. Metadata (Machine-Readable)
```json
{
  "spec_id": "{{SPEC_ID}}",
  "stage0_version": "{{VERSION}}",
  "dcc_config": { ... },
  "memories_used": [ ... ],
  "code_refs": [ ... ]
}
````

````

### Constraints

- Respect `context_compiler.max_tokens`
- Never hallucinate memory IDs
- Prefer concise summaries over full dumps

---

# Part III: Tier 2 (NotebookLM)

## 9. Divine Truth Schema

### Expected Structure

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
    "confidence": 0.85,
    "reasoning": "short explanation"
  }
]
````

````

### Causal Link Types

| Type | Meaning |
|------|---------|
| `causes` | A leads to B |
| `solves` | A resolves B |
| `contradicts` | A conflicts with B |
| `expands` | A builds on B |
| `supersedes` | A replaces B |

### Token Budget

- **Total**: Under 2000 words (~2500 tokens)
- **Per section**: ~300-400 words max
- **JSON block**: Valid JSON, memory IDs from TASK_BRIEF only

---

## 10. Prompt Specification

### Tier 2 "Staff Engineer" Prompt

```text
You are the "Shadow Staff Engineer" for the codex-rs project.

You have access to seeded knowledge files:
- Architecture Bible (system design, module boundaries)
- Stack Justification (tech choices, dependency rationale)
- Bug Retrospectives (failure patterns, anti-patterns)
- Technical Debt Landscape (TODO clusters, known issues)
- Project Diary (session history, progress patterns)

Your job is to synthesize a "Divine Truth" brief for the /speckit.auto pipeline.

=== OUTPUT FORMAT ===

# Divine Truth Brief: {{SPEC_ID}}

## 1. Executive Summary
- Summarize the spec intent in 3-7 bullet points.
- Focus on WHAT is changing and WHY it matters.
- Keep to ~200 words.

## 2. Architectural Guardrails
- List architectural constraints or patterns that MUST be respected.
- Reference relevant historical decisions from Architecture Bible.
- Keep to ~300 words.

## 3. Historical Context & Lessons
- Summarize relevant lessons from:
  - Bug Retrospectives / Anti-Patterns
  - Project Diary entries
  - Technical Debt Landscape
- Keep to ~300 words.

## 4. Risks & Open Questions
- List concrete risks to: correctness, performance, maintainability.
- For each risk, suggest mitigations or validation strategies.
- Keep to ~300 words.

## 5. Suggested Causal Links
JSON array of relationships between memories:

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
````

Rules for JSON:

* ONLY use memory IDs from TASK\_BRIEF.md
* If no IDs available, output empty array: \[]
* Include 0-5 relationships (quality over quantity)

\=== INPUT DATA ===

SPEC ID: {{SPEC\_ID}}

## SPEC CONTENT:

## {{SPEC\_CONTENT}}

## TASK BRIEF:

## {{TASK\_BRIEF\_MD}}

Output ONLY the Divine Truth brief. No preamble, no closing remarks.

````

---

## 11. Response Parsing

### Divine Truth Types

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
````

### Parsing Logic

```rust
fn parse_divine_truth(response: &str) -> Result<DivineTruth> {
    let raw_markdown = response.to_string();

    // Extract sections by header
    let sections = extract_sections_by_header(response);

    let executive_summary = sections.get("1. Executive Summary")
        .cloned().unwrap_or_default();
    let architectural_guardrails = sections.get("2. Architectural Guardrails")
        .cloned().unwrap_or_default();
    let historical_context = sections.get("3. Historical Context & Lessons")
        .cloned().unwrap_or_default();
    let risks_and_questions = sections.get("4. Risks & Open Questions")
        .cloned().unwrap_or_default();

    // Extract JSON from Section 5
    let suggested_links = extract_causal_links(
        sections.get("5. Suggested Causal Links").unwrap_or(&String::new())
    )?;

    Ok(DivineTruth { ... })
}
```

### Link Validation

```rust
fn validate_causal_links(
    links: Vec<CausalLink>,
    valid_memory_ids: &HashSet<String>,
) -> Vec<CausalLink> {
    links.into_iter()
        .filter(|link| {
            valid_memory_ids.contains(&link.from_id) &&
            valid_memory_ids.contains(&link.to_id)
        })
        .map(|mut link| {
            link.confidence = link.confidence.clamp(0.0, 1.0);
            link
        })
        .collect()
}
```

### Fallback Handling

**When NotebookLM unavailable**:

```rust
fn tier1_fallback(spec_id: &str, task_brief: &str) -> DivineTruth {
    DivineTruth {
        executive_summary: format!(
            "Tier 2 unavailable. Using DCC brief only.\n\n{}",
            extract_summary_from_brief(task_brief)
        ),
        architectural_guardrails: "See TASK_BRIEF.md for relevant memories.".into(),
        historical_context: "Historical analysis requires Tier 2.".into(),
        risks_and_questions: "Risk analysis requires Tier 2.".into(),
        suggested_links: vec![],
        raw_markdown: format!("# Divine Truth Brief: {}\n\n[Tier 2 unavailable]", spec_id),
    }
}
```

***

# Part IV: Guardians & Quality

## 12. Metadata Guardian

Invoked before any `store_memory` or `update_memory` calls.

### Responsibilities

* Ensure timestamps are valid UTC RFC3339 strings with `Z` suffix
* Attach `agent_type` tag according to conventions
* Choose initial priority (1–10) for overlay scoring

### Implementation

```rust
fn normalize_metadata(mut memory: NewMemory) -> NewMemory {
    // Auto-fill created_at if missing
    if memory.created_at.is_none() {
        memory.created_at = Some(Utc::now());
    }

    // Enrich tags with agent type
    memory.tags.push("agent:llm_claude".to_string());

    // Set initial priority (default 7, or 9 for spec artifacts)
    if memory.initial_priority.is_none() {
        memory.initial_priority = Some(7);
    }

    memory
}
```

***

## 13. Template Guardian

Transforms arbitrary text into structured memory template.

### Template Format

```text
[PATTERN|DECISION|PROBLEM|INSIGHT]: <One-line Summary>

CONTEXT: <The situation, trigger, or environment>

REASONING: <WHY this approach was taken, alternatives rejected>

OUTCOME: <The measurable result or expected impact>
```

### Prompt

```text
You are restructuring a memory into a strict template.

RULES:
- DO NOT invent or guess any details not present in the input.
- DO NOT change technical content.
- You MAY lightly normalize wording for clarity.
- If a section has no information, leave it empty or use "TODO".
- Keep the output as concise as possible while preserving nuance.

REQUIRED TEMPLATE:

[PATTERN|DECISION|PROBLEM|INSIGHT]: <One-line Summary>

CONTEXT: <The situation>

REASONING: <Why this approach>

OUTCOME: <The result>

INPUT:
--- BEGIN INPUT ---
{{CONTENT}}
--- END INPUT ---

Produce ONLY the filled-in template. Do not add explanations.
```

### Processing Flow

1. Save raw text to overlay DB (`content_raw`)
2. Call local LLM with Template Guardian prompt
3. Send structured content to local-memory
4. Record `structure_status='structured'` in overlay

***

## 14. Error Taxonomy

### Error Categories

| Category             | Description                    | Behavior                    |
| -------------------- | ------------------------------ | --------------------------- |
| `CONFIG_ERROR`       | Config misconfigured           | Skip Stage 0, degraded mode |
| `OVERLAY_DB_ERROR`   | SQLite connection/query errors | Skip Stage 0, degraded mode |
| `LOCAL_MEMORY_ERROR` | REST/CLI communication errors  | Degraded if DCC unusable    |
| `DCC_ERROR`          | DCC logic failures             | Skip Stage 0, degraded mode |
| `TIER2_ERROR`        | NotebookLM HTTP errors         | Use Tier 1-only brief       |
| `PROMPT_ERROR`       | IQO or Tier 2 format errors    | Fallback to heuristics      |
| `INTERNAL_ERROR`     | Unexpected logic bugs          | Skip Stage 0, degraded mode |

### Error-to-Behavior Matrix

| Category             | DCC Output? | Tier2 Output? | /speckit.auto Mode            |
| -------------------- | ----------- | ------------- | ----------------------------- |
| CONFIG\_ERROR        | no          | no            | Degraded (skip Stage 0)       |
| OVERLAY\_DB\_ERROR   | no          | no            | Degraded                      |
| LOCAL\_MEMORY\_ERROR | maybe       | no            | Degraded if DCC unusable      |
| DCC\_ERROR           | no          | no            | Degraded                      |
| TIER2\_ERROR         | yes         | fallback      | Stage0 Tier1-only brief       |
| PROMPT\_ERROR        | maybe       | partial       | Continue with partial context |
| INTERNAL\_ERROR      | no          | no            | Degraded                      |

### Error Logging

```json
{
  "timestamp": "2025-11-30T15:42:01Z",
  "request_id": "3c1e31c4-9f13-4f2b-...",
  "spec_id": "SPEC-KIT-102",
  "category": "TIER2_ERROR",
  "code": "notebooklm_timeout",
  "message": "NotebookLM timed out after 30 seconds",
  "context": {
    "notebook_id": "...",
    "timeout_seconds": 30
  }
}
```

### Soft Failure Policy

**V1 Decision**: Soft failure — log error and continue in degraded mode.

Rationale:

* Stage 0 is an enhancement, not a requirement
* Pipeline should never be blocked by Stage 0 issues
* Users can debug via logs and retry

***

# Part V: Configuration

## 15. Configuration Schema

### Config File Location

```
~/.config/codex/stage0.toml
```

### Full Schema

```toml
[stage0]
# Master enable switch
enabled = true

# Enable explainability (debugging)
explain = false

# DCC configuration
max_candidates = 50
top_k = 10

[stage0.tier2]
# Enable NotebookLM Tier 2 calls
enabled = true

# NotebookLM notebook ID
notebook_id_shadow = "YOUR-NOTEBOOKLM-NOTEBOOK-ID"

# Cache TTL in hours
cache_ttl_hours = 24

# Timeout for NotebookLM calls (seconds)
timeout_secs = 60

[stage0.scoring]
# Dynamic scoring weights (sum to 1.0)
usage_weight = 0.30
recency_weight = 0.30
priority_weight = 0.25
age_decay_weight = 0.15

# Novelty boost for fresh memories (< 5 uses)
novelty_boost = 1.2

[stage0.context_compiler]
max_tokens = 8000
top_k = 15
dynamic_score_weight = 0.40
semantic_similarity_weight = 0.60
pre_filter_limit = 150
diversity_lambda = 0.70
iqo_llm_enabled = true

[stage0.overlay]
# Overlay DB location
db_path = "~/.config/codex/local-memory-overlay.db"
```

***

## 16. Environment Variables

| Variable                     | Description                 |
| ---------------------------- | --------------------------- |
| `CODEX_STAGE0_ENABLED`       | Override enabled flag (0/1) |
| `CODEX_STAGE0_TIER2_ENABLED` | Override Tier 2 flag (0/1)  |
| `CODEX_STAGE0_NOTEBOOK_ID`   | Override notebook ID        |
| `CODEX_STAGE0_EXPLAIN`       | Enable explain mode (0/1)   |

***

## 17. CLI Flags

| Flag                  | Description                  | Default |
| --------------------- | ---------------------------- | ------- |
| `--no-stage0`         | Disable Stage 0 entirely     | false   |
| `--stage0-explain`    | Enable explainability output | false   |
| `--stage0-tier1-only` | Skip Tier 2 (NotebookLM)     | false   |
| `--stage0-debug`      | Dump IQO/candidates to file  | false   |

### Example Usage

```bash
/speckit.auto SPEC-123 --stage0-explain
/speckit.auto SPEC-123 --no-stage0
/speckit.auto SPEC-123 --stage0-tier1-only
```

### TUI Feedback

```
/spec-auto SPEC-KIT-123
Goal: Add user authentication
Resume from: Plan
HAL mode: mock (default)

Stage 0: ✓ NotebookLM (cache miss, 8.3s)
         10 memories used, 2 anti-patterns identified
         task_brief.md written to SPEC directory

Launching 3 agents in sequential pipeline mode...
```

Status messages:

* `Stage 0: ✓ Local brief only (Tier 2 disabled)`
* `Stage 0: ✓ NotebookLM (cache hit, 0.1s)`
* `Stage 0: ⚠ Skipped (DCC failed: timeout)`
* `Stage 0: ⚠ Tier 2 unavailable, using local brief`
* `Stage 0: ○ Disabled`

***

# Appendices

## A. Implementation Checklist

### Files to Create (Stage 0 crate)

* [ ] `codex-rs/stage0/Cargo.toml`
* [ ] `codex-rs/stage0/src/lib.rs` - Public API
* [ ] `codex-rs/stage0/src/config.rs` - Configuration loading
* [ ] `codex-rs/stage0/src/overlay_db/mod.rs` - SQLite operations
* [ ] `codex-rs/stage0/src/dcc/mod.rs` - Dynamic Context Compiler
* [ ] `codex-rs/stage0/src/tier2/mod.rs` - NotebookLM orchestration
* [ ] `codex-rs/stage0/src/scoring.rs` - Dynamic scoring
* [ ] `codex-rs/stage0/src/guardians/mod.rs` - Metadata/Template guardians

### Files to Modify (TUI integration)

* [ ] `codex-rs/Cargo.toml` - Add stage0 to workspace
* [ ] `codex-rs/tui/Cargo.toml` - Add stage0 dependency
* [ ] `codex-rs/tui/src/chatwidget/spec_kit/state.rs` - Add Stage0 fields
* [ ] `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - Stage0 call
* [ ] `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` - Context injection
* [ ] `codex-rs/tui/src/chatwidget/spec_kit/execution_logger.rs` - Stage0 events
* [ ] `codex-rs/tui/src/slash_command.rs` - CLI flags

***

## B. Related Documentation

* **OPERATIONS.md** - Stage 0 observability, metrics, NotebookLM workflow
* **POLICY.md** - Gate policy, evidence policy
* **ARCHITECTURE.md** - System architecture, async/sync boundaries
* **docs/SPEC-KIT-102-notebooklm-integration/** - Full NotebookLM spec

***

## C. Change History

| Version | Date       | Changes                                              |
| ------- | ---------- | ---------------------------------------------------- |
| 1.0.0   | 2026-01-22 | Initial consolidation from `docs/stage0/` (11 files) |

***

*Last Updated: 2026-01-22*
