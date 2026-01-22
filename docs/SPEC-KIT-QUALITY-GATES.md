# Quality Gates Reference (v1.0.0)

**Feature**: T85 - Intelligent Quality Assurance
**Status**: Production Ready
**Last Updated**: 2026-01-22
**Requires**: OpenAI API key for GPT-5 validation

***

## Table of Contents

* [Overview](#overview)
  * [Problem Statement](#problem-statement)
  * [Value Proposition](#value-proposition)
* [Design Decisions](#design-decisions)
* [Architecture](#architecture)
  * [Pipeline Flow](#pipeline-flow)
  * [Quality Gate Checkpoints](#quality-gate-checkpoints)
* [Quality Gate Details](#quality-gate-details)
  * [QG1: Clarify (Pre-Planning)](#qg1-clarify-pre-planning)
  * [QG2: Checklist (Pre-Planning)](#qg2-checklist-pre-planning)
  * [QG3: Analyze Post-Plan](#qg3-analyze-post-plan)
  * [QG4: Analyze Post-Tasks](#qg4-analyze-post-tasks)
* [Resolution Logic](#resolution-logic)
  * [Classification Dimensions](#classification-dimensions)
  * [Escalation Decision Matrix](#escalation-decision-matrix)
  * [Resolution Algorithm](#resolution-algorithm)
* [State Machine](#state-machine)
  * [Phase Types](#phase-types)
  * [State Transitions](#state-transitions)
* [Agent Prompts](#agent-prompts)
  * [Clarify Gate Prompt Template](#clarify-gate-prompt-template)
  * [GPT-5 Validation Prompt](#gpt-5-validation-prompt)
* [Configuration](#configuration)
  * [Environment Variables](#environment-variables)
  * [Usage](#usage)
  * [Tuning](#tuning)
* [Telemetry](#telemetry)
  * [Storage Location](#storage-location)
  * [Schema (v1.1)](#schema-v11)
  * [Git Commits](#git-commits)
* [Implementation Breakdown](#implementation-breakdown)
* [Costs & Performance](#costs--performance)
  * [Per Checkpoint](#per-checkpoint)
  * [API Costs (Estimated)](#api-costs-estimated)
* [Troubleshooting](#troubleshooting)
  * [GPT-5 Validation Fails](#gpt-5-validation-fails)
  * [Quality Gate Hangs](#quality-gate-hangs)
  * [Too Many Escalations](#too-many-escalations)
* [Validation Results](#validation-results)
* [Change History](#change-history)

## Overview

### Problem Statement

**Current State:**

* `/speckit.auto` runs 6 stages: plan → tasks → implement → validate → audit → unlock
* Quality commands (`/speckit.clarify`, `/speckit.analyze`, `/speckit.checklist`) exist separately
* Users must manually run quality checks
* All issues escalated to humans (no auto-resolution)

**Desired State:**

* Quality gates integrated into automation pipeline
* Agents classify issues by confidence and magnitude
* Agents auto-resolve routine issues
* Only escalate high-uncertainty or critical issues to humans
* Fully autonomous for \~55% of quality concerns

### Value Proposition

* More autonomous automation
* Time savings (catch issues early)
* Higher quality outputs
* Only escalate what truly needs human judgment

***

## Design Decisions

All decisions finalized via CLEARFRAME process:

| # | Decision Point            | Choice                                | Rationale                               |
| - | ------------------------- | ------------------------------------- | --------------------------------------- |
| 1 | Auto-resolution threshold | **Majority (2/3) + GPT-5 validation** | Balances accuracy with automation       |
| 2 | Gate placement            | **Inline at 3 checkpoints**           | Maximum quality coverage                |
| 3 | Auto-resolution action    | **Modify files immediately**          | Real-time application                   |
| 4 | Review model              | **Post-pipeline summary**             | No interruptions during auto-resolution |
| 5 | GPT-5 validation context  | **Full (SPEC + PRD + reasoning)**     | Maximum accuracy                        |
| 6 | GPT-5 disagreement        | **Escalate immediately**              | Conservative on uncertain cases         |
| 7 | Git handling              | **Single commit at pipeline end**     | Clean history                           |
| 8 | Escalation behavior       | **Block pipeline until answered**     | Safe, no placeholder assumptions        |
| 9 | Rollback mechanism        | **Manual edit (no infrastructure)**   | Simplest approach                       |

***

## Architecture

### Pipeline Flow

```
/speckit.auto SPEC-KIT-065

┌─────────────────────────────────────┐
│ Checkpoint 1: Pre-Planning          │
├─────────────────────────────────────┤
│ → Clarify gate (3 agents)           │
│   - Identify ambiguities            │
│   - Classify by agreement           │
│   - Auto-resolve unanimous          │
│   - GPT-5 validate 2/3 majority     │
│   - Escalate if GPT-5 disagrees     │
│                                     │
│ → Checklist gate (3 agents)         │
│   - Score requirements (0-10)       │
│   - Auto-improve if fix clear       │
│   - Escalate if unclear             │
│                                     │
│ → Batch escalations                 │
│   [INTERRUPTION: Show N questions]  │
│   [BLOCK: Wait for human answers]   │
│                                     │
│ → Apply auto-resolutions to spec.md │
│ → Apply human answers to spec.md    │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Plan Stage                          │
│ (Uses updated spec.md)              │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Checkpoint 2: Post-Plan             │
├─────────────────────────────────────┤
│ → Analyze gate (3 agents)           │
│   - Check plan ↔ spec consistency   │
│   - Auto-fix terminology/minor      │
│   - Escalate missing requirements   │
│                                     │
│ → If escalations:                   │
│   [INTERRUPTION: Show N questions]  │
│   [BLOCK: Wait for answers]         │
│                                     │
│ → Apply to plan.md                  │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Tasks Stage                         │
│ (Uses updated plan.md)              │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Checkpoint 3: Post-Tasks            │
├─────────────────────────────────────┤
│ → Analyze gate (3 agents)           │
│   - Check task ↔ requirement map    │
│   - Auto-add obvious missing tasks  │
│   - Escalate coverage gaps          │
│                                     │
│ → If escalations:                   │
│   [INTERRUPTION: Show N questions]  │
│   [BLOCK: Wait for answers]         │
│                                     │
│ → Apply to tasks.md                 │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Implement → Validate → Audit        │
│ → Unlock Stages                     │
│ (Use updated spec/plan/tasks)       │
└─────────────────────────────────────┘

┌─────────────────────────────────────┐
│ Pipeline Complete                   │
├─────────────────────────────────────┤
│ → Git commit quality gate changes   │
│ → Show review summary               │
│ → Link to telemetry                 │
└─────────────────────────────────────┘
```

**Expected:**

* 3 potential interruption points
* \~5 questions total (batched at checkpoints)
* 12-17 auto-resolutions applied
* 40 minutes added to 60-minute pipeline = 100 minutes total

### Quality Gate Checkpoints

| Checkpoint   | When                          | Gates               | Purpose                                 |
| ------------ | ----------------------------- | ------------------- | --------------------------------------- |
| Pre-Planning | After SPEC, before plan       | Clarify + Checklist | Resolve ambiguities, score requirements |
| Post-Plan    | After plan, before tasks      | Analyze             | Check plan ↔ spec consistency           |
| Post-Tasks   | After tasks, before implement | Analyze             | Verify task coverage                    |

***

## Quality Gate Details

### QG1: Clarify (Pre-Planning)

**When:** After SPEC created/read, before planning begins
**Command:** `/speckit.clarify`
**Purpose:** Resolve ambiguities in requirements before planning

**Agent Analysis Output:**

```json
{
  "ambiguities": [
    {
      "question": "Should OAuth2 support multiple providers or just one?",
      "confidence": "low",
      "magnitude": "critical",
      "resolvability": "need-human",
      "context": "Spec doesn't specify provider count",
      "suggested_resolution": null
    },
    {
      "question": "What's the token expiry time?",
      "confidence": "medium",
      "magnitude": "important",
      "resolvability": "suggest-fix",
      "context": "Industry standard is 3600s",
      "suggested_resolution": "Use 3600s (1 hour) as default, configurable"
    }
  ],
  "auto_resolved": 1,
  "escalated": 1,
  "total": 3
}
```

### QG2: Checklist (Pre-Planning)

**When:** After clarify, before planning
**Command:** `/speckit.checklist`
**Purpose:** Validate requirement quality scores

**Agent Analysis Output:**

```json
{
  "requirements": [
    {
      "id": "R1",
      "text": "System shall authenticate users",
      "scores": {
        "specificity": 3.2,
        "testability": 4.1,
        "completeness": 3.8,
        "clarity": 4.5
      },
      "overall": 3.9,
      "threshold": 6.0,
      "needs_improvement": true,
      "suggested_improvement": "System shall authenticate users via OAuth2 with support for Google, GitHub, and Microsoft providers"
    }
  ],
  "below_threshold": 5,
  "auto_improved": 4,
  "escalated": 1
}
```

### QG3: Analyze Post-Plan

**When:** After plan created, before tasks generation
**Command:** `/speckit.analyze`
**Purpose:** Check plan consistency with SPEC

**Agent Analysis Output:**

```json
{
  "inconsistencies": [
    {
      "type": "missing_requirement",
      "severity": "critical",
      "description": "SPEC requires OAuth2 but plan doesn't mention it",
      "affected_artifacts": ["plan.md", "spec.md"],
      "suggested_fix": "Add OAuth2 implementation to work breakdown step 3"
    },
    {
      "type": "terminology_mismatch",
      "severity": "minor",
      "description": "SPEC uses 'user' but plan uses 'account'",
      "affected_artifacts": ["plan.md:15", "spec.md:8"],
      "suggested_fix": "Standardize on 'user' throughout plan.md"
    }
  ],
  "auto_resolved": 1,
  "escalated": 1
}
```

### QG4: Analyze Post-Tasks

**When:** After tasks created, before implementation
**Command:** `/speckit.analyze`
**Purpose:** Verify tasks cover all requirements

**Agent Analysis Output:**

```json
{
  "coverage_gaps": [
    {
      "requirement": "R3: Support MFA",
      "missing_task": true,
      "suggested_task": "T5: Implement TOTP-based MFA with QR code generation"
    }
  ],
  "task_conflicts": [
    {
      "task1": "T2: Create auth endpoints",
      "task2": "T4: Build API routes",
      "conflict": "Overlapping scope",
      "suggested_resolution": "Merge into single task: T2 - Auth API endpoints"
    }
  ],
  "auto_resolved": 2,
  "escalated": 0
}
```

***

## Resolution Logic

### Classification Dimensions

**1. Confidence** (How sure are agents about the issue)

* `high` (>90% agent agreement) - Clear, unambiguous
* `medium` (70-90% agreement) - Probable, reasonable assumptions
* `low` (<70% agreement) - Uncertain, conflicting opinions

**2. Magnitude** (Impact of the issue)

* `critical` - Blocks progress, affects core functionality
* `important` - Significant but not blocking
* `minor` - Nice-to-have, cosmetic, minor inconsistency

**3. Resolvability** (Can agents fix it)

* `auto-fix` - Straightforward, well-defined fix
* `suggest-fix` - Fix available but needs validation
* `need-human` - Requires domain knowledge or judgment

### Escalation Decision Matrix

```
┌─────────────┬──────────┬───────────┬──────────┐
│ Confidence  │ Magnitude│Resolvable │ Action   │
├─────────────┼──────────┼───────────┼──────────┤
│ high        │ minor    │ auto-fix  │ AUTO ✅  │
│ high        │ minor    │ suggest   │ AUTO ✅  │
│ high        │ important│ auto-fix  │ AUTO ✅  │
│ high        │ important│ suggest   │ CONFIRM  │
│ high        │ critical │ auto-fix  │ CONFIRM  │
│ high        │ critical │ any       │ ESCALATE │
│ medium      │ minor    │ auto-fix  │ AUTO ✅  │
│ medium      │ minor    │ suggest   │ CONFIRM  │
│ medium      │ important│ any       │ ESCALATE │
│ medium      │ critical │ any       │ ESCALATE │
│ low         │ any      │ any       │ ESCALATE │
├─────────────┴──────────┴───────────┴──────────┤
│ Actions:                                       │
│ AUTO ✅   - Apply fix, log, continue          │
│ CONFIRM  - Show fix, apply with approval      │
│ ESCALATE - Pause, show question, wait         │
└────────────────────────────────────────────────┘
```

### Resolution Algorithm

```rust
fn resolve_quality_issue(issue: &QualityIssue) -> Resolution {
    let agent_answers = &issue.agent_answers;  // [gemini, claude, code]
    let agreement_count = count_agreement(agent_answers);

    match agreement_count {
        // All 3 agents agree
        3 => Resolution::AutoApply {
            answer: agent_answers[0].clone(),
            confidence: Confidence::High,
            reason: "Unanimous (3/3 agents)",
            validation: None,
        },

        // 2 out of 3 agree (majority)
        2 => {
            let majority_answer = find_majority(agent_answers);
            let gpt5_result = validate_with_gpt5(issue, majority_answer);

            if gpt5_result.agrees_with_majority {
                Resolution::AutoApply {
                    answer: majority_answer,
                    confidence: Confidence::Medium,
                    reason: "Majority (2/3) + GPT-5 validated",
                    validation: Some(gpt5_result.reasoning),
                }
            } else {
                Resolution::Escalate {
                    reason: "GPT-5 rejected majority",
                    all_answers: agent_answers.clone(),
                    gpt5_reasoning: gpt5_result.reasoning,
                }
            }
        },

        // No consensus
        _ => Resolution::Escalate {
            reason: "No agent consensus",
            all_answers: agent_answers.clone(),
            gpt5_reasoning: None,
        }
    }
}
```

**Expected distribution:**

* Auto-apply: \~55%
* Escalate: \~45%

***

## State Machine

### Phase Types

```rust
#[derive(Debug, Clone)]
pub enum SpecAutoPhase {
    // Existing
    Guardrail,
    ExecutingAgents { ... },
    CheckingConsensus,

    // Quality gate phases
    QualityGateExecuting {
        checkpoint: QualityCheckpoint,
        gates: Vec<QualityGateType>,
        active_gates: HashSet<QualityGateType>,
        results: HashMap<QualityGateType, Vec<AgentQualityResult>>,
    },

    QualityGateProcessing {
        checkpoint: QualityCheckpoint,
        auto_resolved: Vec<QualityIssue>,
        escalated: Vec<QualityIssue>,
    },

    QualityGateAwaitingHuman {
        checkpoint: QualityCheckpoint,
        questions: Vec<EscalatedQuestion>,
        current_question_index: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityCheckpoint {
    PrePlanning,
    PostPlan,
    PostTasks,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityGateType {
    Clarify,
    Checklist,
    Analyze,
}
```

### State Transitions

```
Guardrail → QualityGateExecuting → QualityGateProcessing →
  ├─ QualityGateAwaitingHuman (if escalations) → Apply answers → Next stage
  └─ Next stage (if no escalations)
```

***

## Agent Prompts

### Clarify Gate Prompt Template

```
You are analyzing SPEC ${SPEC_ID} for ambiguities before planning begins.

Your task:
1. Identify ambiguous or unclear requirements
2. Classify each by:
   - confidence: how certain are you this is ambiguous? (high/medium/low)
   - magnitude: impact if unresolved? (critical/important/minor)
   - resolvability: can you fix it? (auto-fix/suggest-fix/need-human)
3. For resolvable issues, provide suggested_resolution
4. For unresolvable issues, formulate precise question for human

Output JSON:
{
  "ambiguities": [
    {
      "question": string,
      "confidence": "high" | "medium" | "low",
      "magnitude": "critical" | "important" | "minor",
      "resolvability": "auto-fix" | "suggest-fix" | "need-human",
      "context": string,
      "affected_requirements": [string],
      "suggested_resolution": string | null,
      "reasoning": string
    }
  ],
  "agent": "${AGENT_NAME}",
  "stage": "clarify-gate"
}

Auto-Resolution Guidelines:
- auto-fix: Industry standards, obvious answers (e.g., "log errors" → yes)
- suggest-fix: Reasonable defaults with rationale (e.g., "token expiry" → 3600s)
- need-human: Business decisions, architectural choices, trade-offs

Only escalate to human if:
- confidence = low OR
- magnitude = critical OR
- resolvability = need-human
```

### GPT-5 Validation Prompt

```
SPEC Content:
[Full spec.md]

PRD Content:
[Full PRD.md if exists]

Question: "{question}"

Agent Answers:
- Gemini (agrees): "{answer}" - Reasoning: "{reasoning}"
- Claude (agrees): "{answer}" - Reasoning: "{reasoning}"
- Code (disagrees): "{dissenting_answer}" - Reasoning: "{dissent_reasoning}"

Majority answer: "{majority_answer}"
Dissenting view: "{dissent_reasoning}"

Your task:
1. Analyze the SPEC's intent and requirements context
2. Evaluate whether the majority answer aligns with SPEC goals
3. Consider if the dissenting reasoning reveals a valid concern
4. Determine if majority answer should be applied or escalated

Output JSON:
{
  "agrees_with_majority": boolean,
  "reasoning": string,
  "recommended_answer": string (if disagree),
  "confidence": "high" | "medium" | "low"
}
```

***

## Configuration

### Environment Variables

**Required for GPT-5 Validation:**

```bash
export OPENAI_API_KEY="sk-..."
```

**Without API key:**

* Quality gates will fail when 2/3 majority issues are encountered
* Only unanimous (3/3) issues will be auto-resolved
* Auto-resolution rate drops from 60% to 45%

**Optional:**

```bash
# Disable quality gates entirely
export SPEC_KIT_QUALITY_GATES_DISABLED=1

# Or run without quality gates (flag not yet implemented)
/speckit.auto SPEC-ID --no-quality-gates
```

### Usage

**With Quality Gates (Default):**

```bash
export OPENAI_API_KEY="sk-..."
/speckit.auto SPEC-KIT-065
```

**Expected behavior:**

* 3 quality checkpoints run (pre-planning, post-plan, post-tasks)
* \~55% auto-resolved (unanimous)
* \~5-10% GPT-5 validated (2/3 majority)
* \~40% escalated to human
* \~40 minutes added to pipeline
* Git commit created with all quality modifications

### Tuning

**Adjust Auto-Resolution Threshold:**

Currently:

* Unanimous (3/3) → Auto-resolve
* Majority (2/3) → GPT-5 validate → Auto-resolve or escalate
* No consensus (0-1/3) → Escalate

**To be more aggressive** (auto-resolve more, interrupt less):

* Modify `should_auto_resolve()` in quality.rs
* Allow medium confidence + important magnitude

**To be more conservative** (safer, more interruptions):

* Only auto-resolve high confidence + minor magnitude
* Escalate everything else

***

## Telemetry

### Storage Location

```
docs/SPEC-KIT-*/evidence/consensus/
  └── quality-gate-pre-planning_TIMESTAMP.json
  └── quality-gate-post-plan_TIMESTAMP.json
  └── quality-gate-post-tasks_TIMESTAMP.json
```

### Schema (v1.1)

```json
{
  "command": "quality-gate",
  "specId": "SPEC-KIT-065",
  "checkpoint": "pre-planning",
  "gates": ["clarify", "checklist"],
  "timestamp": "2025-10-16T20:00:00Z",
  "schemaVersion": "v1.1",
  "agents": ["gemini", "claude", "code"],

  "results": {
    "clarify": {
      "total_issues": 5,
      "auto_resolved": 3,
      "escalated": 2,
      "auto_resolved_details": [...],
      "escalated_details": [...]
    },
    "checklist": {
      "total_requirements": 8,
      "below_threshold": 2,
      "auto_improved": 2,
      "escalated": 0
    }
  },

  "summary": {
    "total_issues": 7,
    "auto_resolved": 5,
    "escalated": 2,
    "files_modified": ["spec.md"],
    "human_time_seconds": 180
  }
}
```

### Git Commits

Quality gate modifications committed at pipeline end:

```
quality-gates: auto-resolved 12 issues, 5 human-answered

Checkpoint: pre-planning
- clarify: 3 auto-resolved, 2 human-answered
- checklist: 2 auto-improved, 0 escalated

Checkpoint: post-plan
- analyze: 2 auto-fixed, 1 human-answered

Checkpoint: post-tasks
- analyze: 1 auto-fixed, 0 escalated

Files modified:
- spec.md
- plan.md
- tasks.md

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

***

## Implementation Breakdown

| Phase                 | Hours     | Description                                      |
| --------------------- | --------- | ------------------------------------------------ |
| 1. State machine      | 6-8       | Types, enums, state transitions                  |
| 2. Agent prompts      | 4-6       | prompts.json entries, formatting                 |
| 3. Resolution logic   | 8-10      | Classification, GPT-5 validation, decision logic |
| 4. File modifications | 6-8       | Safe file updates, backups                       |
| 5. Escalation UI      | 8-10      | Modal, question view, input handling             |
| 6. Telemetry          | 4-6       | Logging, git commits                             |
| 7. Testing            | 10-12     | Unit, integration, E2E                           |
| **Total**             | **46-60** | **\~1.5 weeks focused work**                     |

***

## Costs & Performance

### Per Checkpoint

* Agent execution: 8-10 min (clarify/checklist/analyze)
* GPT-5 validations: 2-5 sec per issue, \~10-15 sec total
* File modifications: <1 sec
* **Total per checkpoint:** \~8-11 min

**3 checkpoints:** \~24-33 min total added

### API Costs (Estimated)

**Per pipeline with quality gates:**

* 4 quality gates × 3 agents × \~$0.10 = \~$1.20
* GPT-5 validations: 2-3 calls × \~$0.50 = \~$1.00-1.50
* **Total quality gates:** \~$2.20-2.70 per pipeline

**Full pipeline:**

* Regular stages: \~$11
* Quality gates: \~$2.50
* **Total:** \~$13.50 per SPEC

**At 30 SPECs/month:**

* Regular: $330/month
* Quality gates: $75/month
* **Total:** \~$405/month

**ROI:** Saves \~13.5 hours/month, pays for itself if your time is >$30/hour.

***

## Troubleshooting

### GPT-5 Validation Fails

**Error:** "OPENAI\_API\_KEY not set"
**Solution:** Export API key in environment before running

**Error:** "GPT-5 API call failed"
**Solutions:**

* Check API key is valid
* Check internet connection
* Check OpenAI API status
* Verify billing is active

**Fallback:** Quality gates will escalate all 2/3 majority issues if GPT-5 fails

### Quality Gate Hangs

**Symptom:** Pipeline stuck at "Waiting for quality gate agents"
**Cause:** Agents failed to complete
**Solution:** Check agent logs, retry pipeline

### Too Many Escalations

**Symptom:** Every checkpoint has 5+ questions
**Cause:** SPEC is poorly specified
**Solution:** Improve SPEC quality before running automation, or disable quality gates for this SPEC

***

## Validation Results

Quality gate viability was validated through an experiment analyzing 5 existing SPECs:

| Metric                            | Result |
| --------------------------------- | ------ |
| Unanimous agreement rate          | 45%    |
| Auto-resolution rate              | 55%    |
| Escalation rate                   | 45%    |
| Critical issues (always escalate) | 10%    |

**Key findings:**

* Agent agreement IS a viable confidence metric
* Auto-resolution rate is 55% (viable but not spectacular)
* Critical issues always escalate (correct behavior)
* High variance between SPECs (0-100% auto-resolution depending on SPEC quality)

**Full experiment data:** Archived in `docs/archive/quality-gate-experiment/`

***

## Change History

| Version | Date       | Changes                                                                                                                                     |
| ------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| v1.0.0  | 2026-01-22 | Initial canonical version (consolidated from QUALITY\_GATES\_DESIGN.md, QUALITY\_GATES\_SPECIFICATION.md, QUALITY\_GATES\_CONFIGURATION.md) |

***

**Navigation**: [INDEX.md](../INDEX.md) | [POLICY.md](POLICY.md) | [KEY\_DOCS.md](KEY_DOCS.md)
