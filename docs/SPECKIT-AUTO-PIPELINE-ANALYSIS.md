# Spec-Kit Auto Pipeline Analysis

> **Generated**: 2025-11-30
> **Purpose**: Complete analysis of `/speckit.auto` prompt creation flow

---

## Executive Summary

| Metric | Value |
|--------|-------|
| **Total Prompts (Regular)** | 19 LLM calls |
| **Total Prompts (with Quality Gates)** | 19-28 LLM calls |
| **Stages** | 6 (Plan → Tasks → Implement → Validate → Audit → Unlock) |
| **Agents per Stage** | 3-4 (varies by stage) |
| **Execution Pattern** | Sequential (early) → Parallel (late) |
| **Estimated Cost** | $15-40 per full run |

---

## 1. Pipeline Architecture

### High-Level Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           /speckit.auto SPEC-ID                              │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│  INITIALIZATION                                                              │
│  ├─ Config validation (T83)                                                  │
│  ├─ Pipeline config loading (SPEC-948)                                       │
│  ├─ Evidence size check (SPEC-909)                                           │
│  └─ SpecAutoState creation                                                   │
└─────────────────────────────────────┬───────────────────────────────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          │                           │                           │
          ▼                           ▼                           ▼
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│  STAGE 1: PLAN  │──────▶│  STAGE 2: TASKS │──────▶│ STAGE 3: IMPL   │
│  Sequential     │       │  Sequential     │       │  Sequential     │
│  3 agents       │       │  3 agents       │       │  4 agents       │
└─────────────────┘       └─────────────────┘       └─────────────────┘
          │                           │                           │
          ▼                           ▼                           ▼
    [Consensus]               [Consensus]               [Consensus]
    [Quality Gate?]           [Quality Gate?]           [Quality Gate?]
          │                           │                           │
          └───────────────────────────┼───────────────────────────┘
                                      │
          ┌───────────────────────────┼───────────────────────────┐
          │                           │                           │
          ▼                           ▼                           ▼
┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│ STAGE 4: VALID  │──────▶│  STAGE 5: AUDIT │──────▶│ STAGE 6: UNLOCK │
│  Parallel       │       │  Parallel       │       │  Parallel       │
│  3 agents       │       │  3 agents       │       │  3 agents       │
└─────────────────┘       └─────────────────┘       └─────────────────┘
          │                           │                           │
          ▼                           ▼                           ▼
    [Consensus]               [Consensus]               [Consensus]
          │                           │                           │
          └───────────────────────────┼───────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              PIPELINE COMPLETE                               │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Prompt Creation Points

### 2.1 Primary Prompt Builder

**Function**: `build_individual_agent_prompt()`
**Location**: `agent_orchestrator.rs:68`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                      build_individual_agent_prompt()                        │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  INPUTS:                                                                   │
│  ├─ stage: SpecStage (Plan/Tasks/Implement/etc.)                          │
│  ├─ agent_name: String (gemini/claude/gpt_pro/gpt_codex/code)             │
│  ├─ spec_id: String                                                        │
│  ├─ goal: String                                                           │
│  ├─ previous_outputs: Vec<(String, String)>  [sequential only]            │
│  └─ ace_bullets: Option<Vec<PlaybookBullet>>                              │
│                                                                            │
│  PROMPT ASSEMBLY:                                                          │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │ 1. Load agent template from prompts.json                              │ │
│  │ 2. Substitute variables:                                              │ │
│  │    ${SPEC_ID}, ${CONTEXT}, ${PROMPT_VERSION}                         │ │
│  │    ${MODEL_ID}, ${MODEL_RELEASE}, ${REASONING_MODE}                  │ │
│  │ 3. Inject previous agent outputs (if sequential)                      │ │
│  │ 4. Inject ACE playbook bullets (if available)                        │ │
│  │ 5. Apply context truncation (20KB per file, exponential backoff)     │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                                                            │
│  OUTPUT: Complete prompt string (5KB - 50KB typical)                       │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Quality Gate Prompt Builder

**Function**: `build_quality_gate_prompt()`
**Location**: `native_quality_gate_orchestrator.rs:214`

```
┌────────────────────────────────────────────────────────────────────────────┐
│                       build_quality_gate_prompt()                           │
├────────────────────────────────────────────────────────────────────────────┤
│                                                                            │
│  INPUTS:                                                                   │
│  ├─ checkpoint: QualityCheckpoint (Clarify/Checklist/Analyze)             │
│  ├─ spec_id: String                                                        │
│  └─ stage_context: String (previous stage outputs)                        │
│                                                                            │
│  PROMPT ASSEMBLY:                                                          │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │ 1. Load checkpoint-specific template                                  │ │
│  │ 2. Read spec.md + PRD.md context                                     │ │
│  │ 3. Enforce JSON-only output format                                   │ │
│  │ 4. Apply validation constraints                                       │ │
│  └──────────────────────────────────────────────────────────────────────┘ │
│                                                                            │
│  OUTPUT: Quality gate prompt (2KB - 10KB typical)                          │
│                                                                            │
└────────────────────────────────────────────────────────────────────────────┘
```

---

## 3. Stage-by-Stage Breakdown

### Stage Configuration

| Stage | Agents | Pattern | Prompt Size | Cost |
|-------|--------|---------|-------------|------|
| **Plan** | gemini, claude, code | Sequential | 15-30KB | $2-4 |
| **Tasks** | gemini, claude, code | Sequential | 20-40KB | $2-4 |
| **Implement** | gemini, claude, gpt_codex, code | Sequential | 30-50KB | $4-6 |
| **Validate** | gemini, claude, code | Parallel | 10-20KB | $1-2 |
| **Audit** | gemini, claude, code | Parallel | 10-20KB | $1-2 |
| **Unlock** | gemini, claude, code | Parallel | 5-15KB | $0.50-1 |

### Sequential vs Parallel Execution

```
SEQUENTIAL STAGES (Plan, Tasks, Implement):
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ Agent 1 │────▶│ Agent 2 │────▶│ Agent 3 │────▶│ Agent 4 │
│ gemini  │     │ claude  │     │ code    │     │gpt_codex│
└─────────┘     └─────────┘     └─────────┘     └─────────┘
     │               │               │               │
     │               ▼               ▼               ▼
     │         [sees Agent 1]  [sees 1+2]      [sees 1+2+3]
     │
     └─▶ Each agent sees ALL previous agent outputs
         More context = larger prompts = higher cost
         Better consensus = higher quality

PARALLEL STAGES (Validate, Audit, Unlock):
┌─────────┐
│ Agent 1 │──────┐
│ gemini  │      │
└─────────┘      │
                 │
┌─────────┐      ├────▶ [Consensus]
│ Agent 2 │──────┤
│ claude  │      │
└─────────┘      │
                 │
┌─────────┐      │
│ Agent 3 │──────┘
│ code    │
└─────────┘

     └─▶ All agents run independently
         Smaller prompts = lower cost
         Independent opinions = diverse perspectives
```

---

## 4. Prompt Count Calculation

### Regular Pipeline (No Quality Gates)

```
┌────────────────────────────────────────────────────────────────┐
│                    PROMPT COUNT: REGULAR RUN                    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Stage 1: PLAN                                                 │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt (+ gemini output) .......... 1               │
│  └─ code prompt (+ gemini + claude) .......... 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Stage 2: TASKS                                                │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt (+ gemini output) .......... 1               │
│  └─ code prompt (+ gemini + claude) .......... 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Stage 3: IMPLEMENT                                            │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt (+ gemini output) .......... 1               │
│  ├─ gpt_codex prompt (+ gemini + claude) ..... 1               │
│  └─ code prompt (+ all previous) ............. 1               │
│                                            ─────               │
│                                    Subtotal: 4 prompts         │
│                                                                │
│  Stage 4: VALIDATE (parallel)                                  │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt ............................ 1               │
│  └─ code prompt .............................. 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Stage 5: AUDIT (parallel)                                     │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt ............................ 1               │
│  └─ code prompt .............................. 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Stage 6: UNLOCK (parallel)                                    │
│  ├─ gemini prompt ............................ 1               │
│  ├─ claude prompt ............................ 1               │
│  └─ code prompt .............................. 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  TOTAL REGULAR PROMPTS: 19                                     │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Quality Gates (Optional)

```
┌────────────────────────────────────────────────────────────────┐
│                  PROMPT COUNT: QUALITY GATES                    │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Quality Gate: CLARIFY (after Plan)                            │
│  ├─ agent 1 (cheap model) .................... 1               │
│  ├─ agent 2 (cheap model) .................... 1               │
│  └─ agent 3 (cheap model) .................... 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Quality Gate: CHECKLIST (after Tasks)                         │
│  ├─ agent 1 (cheap model) .................... 1               │
│  ├─ agent 2 (cheap model) .................... 1               │
│  └─ agent 3 (cheap model) .................... 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
│  Quality Gate: ANALYZE (after Implement)                       │
│  ├─ agent 1 (cheap model) .................... 1               │
│  ├─ agent 2 (cheap model) .................... 1               │
│  └─ agent 3 (cheap model) .................... 1               │
│                                            ─────               │
│                                    Subtotal: 3 prompts         │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  TOTAL QUALITY GATE PROMPTS: 0-9 (configurable)                │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Grand Total

```
┌────────────────────────────────────────────────────────────────┐
│                      GRAND TOTAL PROMPTS                        │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Minimum (no quality gates):     19 prompts                    │
│  Maximum (all quality gates):    28 prompts                    │
│  Typical (1-2 quality gates):    22-25 prompts                 │
│                                                                │
│  ┌──────────────────────────────────────────────────────────┐ │
│  │                                                          │ │
│  │   19 base + (0-9 quality gates) = 19-28 total prompts   │ │
│  │                                                          │ │
│  └──────────────────────────────────────────────────────────┘ │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

## 5. Visual Flow: Complete Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        /speckit.auto SPEC-KIT-XXX                            │
│                                                                              │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 1: PLAN (Sequential - 3 prompts)                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐    ┌────────┐    ┌────────┐                                ││
│  │  │ GEMINI │───▶│ CLAUDE │───▶│  CODE  │                                ││
│  │  │ $0.50  │    │ $1.00  │    │ $0.80  │                                ││
│  │  └────────┘    └────────┘    └────────┘                                ││
│  │       │             │             │                                     ││
│  │       └─────────────┴─────────────┴────────▶ [CONSENSUS] ──▶ plan.md   ││
│  │                                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│                              ▼ (optional)                                    │
│                    ┌──────────────────┐                                      │
│                    │ QUALITY: CLARIFY │ (+3 prompts, ~$0.30)                 │
│                    └──────────────────┘                                      │
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 2: TASKS (Sequential - 3 prompts)                                     │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐    ┌────────┐    ┌────────┐                                ││
│  │  │ GEMINI │───▶│ CLAUDE │───▶│  CODE  │                                ││
│  │  │ $0.60  │    │ $1.20  │    │ $0.90  │                                ││
│  │  └────────┘    └────────┘    └────────┘                                ││
│  │       │             │             │                                     ││
│  │       └─────────────┴─────────────┴────────▶ [CONSENSUS] ──▶ tasks.md  ││
│  │                                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│                              ▼ (optional)                                    │
│                   ┌───────────────────┐                                      │
│                   │ QUALITY: CHECKLIST│ (+3 prompts, ~$0.30)                 │
│                   └───────────────────┘                                      │
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 3: IMPLEMENT (Sequential - 4 prompts)                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐    ┌────────┐    ┌──────────┐    ┌────────┐                ││
│  │  │ GEMINI │───▶│ CLAUDE │───▶│ GPT_CODEX│───▶│  CODE  │                ││
│  │  │ $1.00  │    │ $2.00  │    │  $1.50   │    │ $1.20  │                ││
│  │  └────────┘    └────────┘    └──────────┘    └────────┘                ││
│  │       │             │             │               │                     ││
│  │       └─────────────┴─────────────┴───────────────┴──▶ [CONSENSUS]     ││
│  │                                                              │          ││
│  │                                                              ▼          ││
│  │                                                         impl.md        ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│                              ▼ (optional)                                    │
│                    ┌──────────────────┐                                      │
│                    │ QUALITY: ANALYZE │ (+3 prompts, ~$0.30)                 │
│                    └──────────────────┘                                      │
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 4: VALIDATE (Parallel - 3 prompts)                                    │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐                                                             ││
│  │  │ GEMINI │──────┐                                                      ││
│  │  │ $0.40  │      │                                                      ││
│  │  └────────┘      │                                                      ││
│  │                  │                                                      ││
│  │  ┌────────┐      ├────────▶ [CONSENSUS] ──▶ validate.md                ││
│  │  │ CLAUDE │──────┤                                                      ││
│  │  │ $0.80  │      │                                                      ││
│  │  └────────┘      │                                                      ││
│  │                  │                                                      ││
│  │  ┌────────┐      │                                                      ││
│  │  │  CODE  │──────┘                                                      ││
│  │  │ $0.50  │                                                             ││
│  │  └────────┘                                                             ││
│  │                                                                         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 5: AUDIT (Parallel - 3 prompts)                                       │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐  ┌────────┐  ┌────────┐                                    ││
│  │  │ GEMINI │  │ CLAUDE │  │  CODE  │     All parallel                   ││
│  │  │ $0.40  │  │ $0.80  │  │ $0.50  │     ────────────▶ [CONSENSUS]      ││
│  │  └────────┘  └────────┘  └────────┘                        │           ││
│  │                                                            ▼           ││
│  │                                                       audit.md         ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                                                                              │
│  STAGE 6: UNLOCK (Parallel - 3 prompts)                                      │
│  ┌─────────────────────────────────────────────────────────────────────────┐│
│  │                                                                         ││
│  │  ┌────────┐  ┌────────┐  ┌────────┐                                    ││
│  │  │ GEMINI │  │ CLAUDE │  │  CODE  │     All parallel                   ││
│  │  │ $0.20  │  │ $0.40  │  │ $0.30  │     ────────────▶ [CONSENSUS]      ││
│  │  └────────┘  └────────┘  └────────┘                        │           ││
│  │                                                            ▼           ││
│  │                                                       unlock.md        ││
│  └─────────────────────────────────────────────────────────────────────────┘│
│                              │                                               │
│  ════════════════════════════════════════════════════════════════════════   │
│                              │                                               │
│                              ▼                                               │
│                    ┌──────────────────┐                                      │
│                    │ PIPELINE COMPLETE │                                     │
│                    │                  │                                      │
│                    │ Total: 19 prompts│                                      │
│                    │ Cost: ~$15-20    │                                      │
│                    └──────────────────┘                                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 6. Cost Breakdown

### Per-Stage Costs (Estimated)

| Stage | Agents | Tokens (approx) | Cost |
|-------|--------|-----------------|------|
| Plan | 3 | 45K input, 15K output | $2-4 |
| Tasks | 3 | 60K input, 20K output | $2-4 |
| Implement | 4 | 120K input, 40K output | $4-6 |
| Validate | 3 | 30K input, 10K output | $1-2 |
| Audit | 3 | 30K input, 10K output | $1-2 |
| Unlock | 3 | 15K input, 5K output | $0.50-1 |
| **Subtotal** | **19** | **~300K input, ~100K output** | **$11-19** |

### Quality Gate Costs (If Enabled)

| Gate | Agents | Tokens | Cost |
|------|--------|--------|------|
| Clarify | 3 | 10K input, 3K output | $0.20-0.40 |
| Checklist | 3 | 10K input, 3K output | $0.20-0.40 |
| Analyze | 3 | 15K input, 5K output | $0.30-0.50 |
| **Subtotal** | **9** | **~35K input, ~11K output** | **$0.70-1.30** |

### Total Cost Range

```
┌─────────────────────────────────────────────────────────────────┐
│                        COST SUMMARY                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Minimum (no quality gates):          $11-19                    │
│  With all quality gates:              $12-21                    │
│  Typical run:                         $15-20                    │
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Cost Optimization (SPEC-948):                             │ │
│  │ - Skip Validate/Audit/Unlock: saves ~$3-5                 │ │
│  │ - Disable quality gates: saves ~$1                        │ │
│  │ - Use fewer agents: saves 20-40%                          │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. Key Files Reference

| File | Purpose | Key Functions |
|------|---------|---------------|
| `pipeline_coordinator.rs` | Main orchestration | `handle_spec_auto()`, `advance_spec_auto()` |
| `agent_orchestrator.rs` | Agent spawning | `build_individual_agent_prompt()`, `auto_submit_spec_stage_prompt()` |
| `consensus.rs` | Agent config | `get_stage_agents()`, `AgentConfig` |
| `quality_gate_handler.rs` | Quality gates | `execute_quality_checkpoint()` |
| `native_quality_gate_orchestrator.rs` | QG spawning | `spawn_quality_gate_agents_native()` |
| `state.rs` | Pipeline state | `SpecAutoState`, `SpecAutoPhase` |

---

## 8. Prompt Creation Timeline

```
TIME ──────────────────────────────────────────────────────────────────▶

t=0     t=30s   t=60s   t=90s   t=2m    t=3m    t=4m    t=5m    t=6m
│       │       │       │       │       │       │       │       │
▼       ▼       ▼       ▼       ▼       ▼       ▼       ▼       ▼

[PLAN─────────────────────]
P1 ──▶ P2 ──▶ P3
gemini  claude  code

                [TASKS────────────────────]
                P4 ──▶ P5 ──▶ P6
                gemini  claude  code

                                [IMPLEMENT─────────────────────────]
                                P7 ──▶ P8 ──▶ P9 ──▶ P10
                                gem    cla    codex   code

                                                [VAL──────]  [AUD──────]
                                                P11-P13      P14-P16
                                                parallel     parallel

                                                             [UNLOCK───]
                                                             P17-P19
                                                             parallel

PROMPT COUNT:
├─ Sequential: 10 prompts (P1-P10)
├─ Parallel: 9 prompts (P11-P19)
└─ Total: 19 prompts

ELAPSED TIME: ~6-10 minutes (depending on model latency)
```

---

## 9. Summary Table

| Metric | Value |
|--------|-------|
| **Total Stages** | 6 |
| **Agents per Stage** | 3-4 |
| **Sequential Stages** | 3 (Plan, Tasks, Implement) |
| **Parallel Stages** | 3 (Validate, Audit, Unlock) |
| **Regular Prompts** | 19 |
| **Quality Gate Prompts** | 0-9 |
| **Total Prompts** | 19-28 |
| **Estimated Cost** | $15-40 |
| **Estimated Time** | 6-15 minutes |

---

## 10. SPEC-KIT-099 Integration Point

With the Research-to-Code Context Bridge (SPEC-KIT-099), the pipeline becomes:

```
┌────────────────────────────────────────────────────────────────┐
│            PROPOSED 7-STAGE PIPELINE (with Research)            │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  Stage 0: RESEARCH (NEW)                                       │
│  ├─ MCP call to NotebookLM ..................... 1 (not LLM)   │
│  ├─ "Divine Truth" injection into all stages                   │
│  └─ No additional LLM prompts                                  │
│                                                                │
│  Stages 1-6: Same as before                                    │
│  ├─ But each prompt NOW includes research context              │
│  └─ Slightly larger prompts (~5-10% increase)                  │
│                                                                │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  NEW TOTAL: 19-28 LLM prompts + 1 MCP call                     │
│  COST IMPACT: +$1-3 (larger context in each prompt)            │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

---

*Document generated for pipeline analysis and optimization planning.*
