# Spec-Kit Workflow: Complete Development Pipeline

## Overview

Spec-Kit implements a 6-stage software development pipeline where each stage can be run independently or as part of an automated workflow. The system uses multi-agent consensus to produce high-quality artifacts at each stage.

## The 6 Stages

### Stage 1: Specify

**Purpose**: Create or refine the Product Requirements Document (PRD)

**Input**: User description or existing spec.md
**Output**: Refined PRD with clear requirements, acceptance criteria, examples

**Method**:
- `/speckit.new` - Native template generation (Tier 0, instant, free)
- `/speckit.specify` - Single-agent PRD refinement (Tier 1, ~$0.10)

**What Happens**:
1. Creates SPEC directory structure: `docs/SPEC-KIT-###-<slug>/`
2. Generates `spec.md` with requirement template
3. Optionally refines with AI analysis

### Stage 2: Plan

**Purpose**: Create architectural work breakdown

**Input**: spec.md (and PRD.md if exists)
**Output**: plan.md with work breakdown, acceptance mapping, risk analysis

**Method**: `/speckit.plan` (Tier 2, 3 agents, ~$0.35)

**Agents Used**: gemini-flash, claude-haiku, gpt5-medium

**What Happens**:
1. Three agents independently read spec and produce plans
2. Consensus synthesis blends perspectives
3. Plan includes: work breakdown, validation steps, risk analysis, disagreement notes

**Deliverable Format**:
```markdown
# Plan: SPEC-KIT-065

## Inputs
- Spec: docs/SPEC-KIT-065/spec.md (hash: abc123)

## Work Breakdown
1. Create database models for OAuth tokens
2. Implement OAuth callback handler
3. Add token refresh mechanism

## Acceptance Mapping
| Requirement | Validation Step | Test Artifact |
|-------------|-----------------|---------------|
| R1: OAuth login | Unit test OAuth flow | tests/auth/oauth_test.rs |

## Risks & Unknowns
- Provider-specific token handling differences

## Consensus Analysis
- Agreement: 3/3 on core architecture
- Disagreement: Token storage approach (resolved: use encrypted vault)
```

### Stage 3: Tasks

**Purpose**: Decompose plan into actionable task list

**Input**: plan.md
**Output**: tasks.md with ordered, validated task breakdown

**Method**: `/speckit.tasks` (Tier 1, 1 agent, ~$0.10)

**Agent Used**: gpt5-low (single agent for structured output)

**What Happens**:
1. Reads plan and produces ordered task list
2. Each task maps to acceptance criteria
3. Tasks tracked in SPEC.md table

**Task Format**:
```markdown
## Tasks for SPEC-KIT-065

| Order | Task ID | Title | Status | Validation |
|-------|---------|-------|--------|------------|
| 1 | T1 | Create OAuth config struct | Backlog | Unit test config parsing |
| 2 | T2 | Implement token storage | Backlog | Integration test with vault |
| 3 | T3 | Add callback endpoint | Backlog | E2E test OAuth flow |
```

### Stage 4: Implement

**Purpose**: Generate code implementation

**Input**: spec.md, plan.md, tasks.md
**Output**: implementation.md with code, implementation details

**Method**: `/speckit.implement` (Tier 2, 2 agents, ~$0.11)

**Agents Used**:
- gpt_codex (HIGH) - Primary code generation
- claude-haiku - Validation and edge cases

**What Happens**:
1. gpt-5-codex generates implementation
2. claude-haiku validates for edge cases, security issues
3. Code passes through: `cargo fmt`, `cargo clippy`, build checks, tests
4. Output includes code blocks with file paths

**Important**: Implementation validates automatically before returning.

### Stage 5: Validate

**Purpose**: Create test strategy and validate implementation

**Input**: implementation.md, spec.md
**Output**: validation-report.md with test scenarios, coverage analysis

**Method**: `/speckit.validate` (Tier 2, 3 agents, ~$0.35)

**Agents Used**: gemini-flash, claude-haiku, gpt5-medium

**What Happens**:
1. Three agents analyze implementation against requirements
2. Generate test scenarios covering happy path, edge cases, error handling
3. Produce coverage analysis and missing test identification

**Output Format**:
```markdown
## Validation Report: SPEC-KIT-065

### Test Scenarios
| Scenario | Type | Status | Notes |
|----------|------|--------|-------|
| Valid OAuth login | Happy path | Passed | |
| Expired token refresh | Edge case | Passed | |
| Invalid callback | Error | Passed | |

### Coverage Analysis
- Functional coverage: 95%
- Edge case coverage: 87%
- Missing: Rate limiting tests
```

### Stage 6: Audit

**Purpose**: Security and compliance review

**Input**: All previous artifacts
**Output**: Audit report with security findings, compliance status

**Method**: `/speckit.audit` (Tier 3, 3 premium agents, ~$0.80)

**Agents Used**: gemini-pro, claude-sonnet, gpt5-high

**What Happens**:
1. Premium agents review for security vulnerabilities
2. Check compliance with coding standards
3. Identify potential production issues

**Why Premium**: Critical security decisions require higher reasoning capability.

### Stage 7: Unlock

**Purpose**: Final ship/no-ship decision

**Input**: All artifacts + audit report
**Output**: Ship decision with rationale

**Method**: `/speckit.unlock` (Tier 3, 3 premium agents, ~$0.80)

**Agents Used**: gemini-pro, claude-sonnet, gpt5-high

**What Happens**:
1. Premium agents review complete package
2. Binary ship/no-ship decision
3. Rationale and any final conditions documented

---

## Quality Gate Checkpoints

Quality gates are integrated throughout the pipeline:

### Checkpoint 1: Pre-Planning
**Gates**: Clarify → Checklist

1. **Clarify** (native): Detect ambiguities in spec
   - Pattern matching for vague language
   - Missing sections identification
   - Undefined terms flagging

2. **Checklist** (native): Score requirement quality
   - Completeness (0-10)
   - Clarity (0-10)
   - Testability (0-10)
   - Consistency (0-10)

**Blocking**: User must answer clarifying questions before planning proceeds.

### Checkpoint 2: Post-Plan
**Gate**: Analyze

- Check plan ↔ spec consistency
- Identify coverage gaps
- Validate all requirements have validation steps

### Checkpoint 3: Post-Tasks
**Gate**: Analyze

- Check task ↔ requirement mapping
- Ensure all requirements have corresponding tasks
- Validate task ordering makes sense

---

## Automated Pipeline: `/speckit.auto`

Run the complete pipeline automatically:

```bash
/speckit.auto SPEC-KIT-065
```

**Total Cost**: ~$2.70
**Total Time**: 45-50 minutes

### Flow

```
Start
  ↓
[Quality Gate: Clarify + Checklist]
  ↓ (user answers questions)
Plan (3 agents)
  ↓
[Quality Gate: Analyze]
  ↓
Tasks (1 agent)
  ↓
[Quality Gate: Analyze]
  ↓
Implement (2 agents)
  ↓
Validate (3 agents)
  ↓
Audit (3 premium)
  ↓
Unlock (3 premium)
  ↓
Complete (artifacts committed to git)
```

### Stage Skipping (SPEC-948)

```bash
# Skip expensive validation stages
/speckit.auto SPEC-KIT-065 --skip-validate --skip-audit

# Run only specific stages
/speckit.auto SPEC-KIT-065 --stages=plan,tasks,implement

# Cost savings examples:
# - Full pipeline: $2.70
# - Skip validate+audit+unlock: $0.66 (75% savings)
# - Only plan: $0.35 (87% savings)
```

---

## Artifact Flow

```
User Request
    ↓
docs/SPEC-KIT-065/
├── spec.md        ← /speckit.new
├── PRD.md         ← /speckit.specify
├── plan.md        ← /speckit.plan
├── tasks.md       ← /speckit.tasks
├── implementation.md  ← /speckit.implement
├── validation-report.md  ← /speckit.validate
└── audit-report.md   ← /speckit.audit

Evidence (captured automatically):
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-065/
├── plan-gemini-*.json
├── plan-claude-*.json
├── plan-gpt_pro-*.json
├── plan-synthesis-*.json
└── telemetry.json
```

---

## Retry and Degradation

### Agent Failure Handling
- **3 automatic retries** with exponential backoff (2s → 4s → 8s)
- **Detects**: timeout, empty result, malformed JSON
- **Re-prompts** with guidance on failure

### Graceful Degradation
- **1/3 fails**: Continue with 2/3 consensus (still valid)
- **2/3 fail**: Escalate with warning, suggest manual intervention
- **All fail**: Stop pipeline, alert user

### Empty Result Detection
- Detects agents returning empty outputs
- Retries with storage guidance
- Escalates after 3 failures
