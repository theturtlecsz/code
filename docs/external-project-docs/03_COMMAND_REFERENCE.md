# Command Reference: Spec-Kit Commands

## Command Tiers

| Tier | Agents | Cost | Time | Use Case |
|------|--------|------|------|----------|
| 0 (Native) | 0 | $0 | <1s | Pattern matching, validation |
| 1 (Single) | 1 | ~$0.10 | 3-5 min | Simple analysis |
| 2 (Multi) | 2-3 | ~$0.35 | 8-12 min | Complex planning/generation |
| 3 (Premium) | 3 | ~$0.80 | 10-12 min | Critical decisions |
| 4 (Pipeline) | varies | ~$2.70 | 45-50 min | Full automation |

---

## Core Commands

### `/speckit.new <description>`

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second

Create a new SPEC with template-based generation.

```bash
/speckit.new Add user authentication with OAuth2

# Creates:
# docs/SPEC-KIT-065-user-auth/
# ├── spec.md (populated template)
# └── README.md
```

**What It Does**:
- Scans existing SPECs to find next ID
- Creates directory structure
- Populates spec.md template with description
- Updates SPEC.md tracker

---

### `/speckit.specify SPEC-ID [description]`

**Tier**: 1 (Single Agent)
**Cost**: ~$0.10
**Time**: 3-5 minutes

Refine the PRD with single-agent analysis.

```bash
/speckit.specify SPEC-KIT-065 "Focus on security aspects"
```

**What It Does**:
- Reads existing spec.md
- Refines requirements with AI analysis
- Produces enhanced PRD.md
- Identifies missing acceptance criteria

---

### `/speckit.clarify SPEC-ID`

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second

Detect ambiguities using pattern matching.

```bash
/speckit.clarify SPEC-KIT-065
```

**What It Detects**:
- Vague language: "should", "could", "maybe", "possibly"
- Missing sections: empty acceptance criteria, no examples
- Undefined terms: technical jargon without definition
- Cross-artifact contradictions

**Output**: 3-5 clarifying questions (high confidence only)

---

### `/speckit.analyze SPEC-ID`

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second

Check consistency using structural diff.

```bash
/speckit.analyze SPEC-KIT-065
```

**What It Checks**:
- ID consistency across all artifacts
- Coverage gaps (requirements without plan items)
- Section completeness
- Field validation (IDs, dates, required fields)

**Output**: List of issues with auto-fix suggestions

---

### `/speckit.checklist SPEC-ID`

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second

Score requirement quality using rubric.

```bash
/speckit.checklist SPEC-KIT-065
```

**Scoring Rubric** (0-10 each):
- **Completeness**: All requirements present? Examples provided?
- **Clarity**: Language clear? Jargon explained?
- **Testability**: Acceptance criteria measurable?
- **Consistency**: No contradictions across artifacts?

**Thresholds**: Fail < 6, Warn 6-7, Pass ≥ 8

---

### `/speckit.plan SPEC-ID [context]`

**Tier**: 2 (Multi-Agent)
**Cost**: ~$0.35
**Time**: 10-12 minutes

Create work breakdown with multi-agent consensus.

```bash
/speckit.plan SPEC-KIT-065
/speckit.plan SPEC-KIT-065 "Prioritize security over performance"
```

**Agents**: gemini-flash, claude-haiku, gpt5-medium

**What It Produces**:
- Work breakdown structure
- Acceptance mapping table
- Risk analysis
- Consensus notes (agreements, resolved disagreements)

---

### `/speckit.tasks SPEC-ID`

**Tier**: 1 (Single Agent)
**Cost**: ~$0.10
**Time**: 3-5 minutes

Decompose plan into task list.

```bash
/speckit.tasks SPEC-KIT-065
```

**Agent**: gpt5-low

**What It Produces**:
- Ordered task list
- Task-to-requirement mapping
- Validation step for each task
- Status tracking (Backlog, In Progress, Done)

---

### `/speckit.implement SPEC-ID`

**Tier**: 2 (Multi-Agent)
**Cost**: ~$0.11
**Time**: 8-12 minutes

Generate code implementation.

```bash
/speckit.implement SPEC-KIT-065
```

**Agents**: gpt_codex (HIGH), claude-haiku (validator)

**What It Produces**:
- Code implementation with file paths
- Inline documentation
- Validation report from claude-haiku

**Automatic Validation**:
- `cargo fmt` - formatting
- `cargo clippy` - linting
- Build checks
- Unit tests

---

### `/speckit.validate SPEC-ID`

**Tier**: 2 (Multi-Agent)
**Cost**: ~$0.35
**Time**: 10-12 minutes

Create test strategy and coverage analysis.

```bash
/speckit.validate SPEC-KIT-065
```

**Agents**: gemini-flash, claude-haiku, gpt5-medium

**What It Produces**:
- Test scenarios (happy path, edge cases, error handling)
- Coverage analysis
- Missing test identification
- Validation report

---

### `/speckit.audit SPEC-ID`

**Tier**: 3 (Premium)
**Cost**: ~$0.80
**Time**: 10-12 minutes

Security and compliance review.

```bash
/speckit.audit SPEC-KIT-065
```

**Agents**: gemini-pro, claude-sonnet, gpt5-high

**What It Checks**:
- Security vulnerabilities (OWASP Top 10)
- Compliance with coding standards
- Potential production issues
- Performance concerns

---

### `/speckit.unlock SPEC-ID`

**Tier**: 3 (Premium)
**Cost**: ~$0.80
**Time**: 10-12 minutes

Final ship/no-ship decision.

```bash
/speckit.unlock SPEC-KIT-065
```

**Agents**: gemini-pro, claude-sonnet, gpt5-high

**What It Produces**:
- Binary SHIP/NO-SHIP decision
- Rationale for decision
- Any final conditions or requirements

---

### `/speckit.auto SPEC-ID [flags]`

**Tier**: 4 (Full Pipeline)
**Cost**: ~$2.70
**Time**: 45-50 minutes

Run complete automated pipeline.

```bash
# Full pipeline
/speckit.auto SPEC-KIT-065

# Skip validation stages
/speckit.auto SPEC-KIT-065 --skip-validate --skip-audit

# Run only specific stages
/speckit.auto SPEC-KIT-065 --stages=plan,tasks,implement

# Skip to later stage (preserves earlier artifacts)
/speckit.auto SPEC-KIT-065 --from implement
```

**Available Flags**:
- `--skip-validate` - Skip validate stage
- `--skip-audit` - Skip audit stage
- `--skip-unlock` - Skip unlock stage
- `--stages=LIST` - Run only specified stages
- `--from STAGE` - Resume from specific stage

**Cost Examples**:
- Full pipeline: $2.70
- Skip validate+audit+unlock: $0.66
- Only plan: $0.35
- Only plan+tasks: $0.45

---

### `/speckit.status SPEC-ID`

**Tier**: 0 (Native)
**Cost**: $0
**Time**: <1 second

Show status dashboard.

```bash
/speckit.status SPEC-KIT-065
```

**Shows**:
- Stage completion status
- Current stage and progress
- Artifact locations
- Evidence paths
- Cost incurred so far

---

## Guardrail Commands

Wrapper commands that run validation before stages:

### `/guardrail.plan SPEC-ID`
### `/guardrail.tasks SPEC-ID`
### `/guardrail.implement SPEC-ID`
### `/guardrail.validate SPEC-ID`
### `/guardrail.audit SPEC-ID`
### `/guardrail.unlock SPEC-ID`
### `/guardrail.auto SPEC-ID [--from STAGE]`

These wrap the corresponding `/speckit.*` commands with:
- Clean tree validation (unless `SPEC_OPS_ALLOW_DIRTY=1`)
- Baseline checks
- Telemetry collection
- Evidence management

---

## Utility Commands

### `/spec-evidence-stats [--spec SPEC-ID]`

Monitor evidence footprint.

```bash
/spec-evidence-stats
/spec-evidence-stats --spec SPEC-KIT-065
```

**Shows**: Storage used per SPEC (25 MB soft limit)

---

### `/spec-consensus SPEC-ID STAGE`

Inspect consensus artifacts for a stage.

```bash
/spec-consensus SPEC-KIT-065 plan
```

**Shows**: Agent outputs, synthesis result, agreement analysis

---

## Legacy Commands (Deprecated)

These still work but will be removed in a future release:

| Legacy | Current |
|--------|---------|
| `/new-spec` | `/speckit.new` |
| `/spec-plan` | `/speckit.plan` |
| `/spec-tasks` | `/speckit.tasks` |
| `/spec-implement` | `/speckit.implement` |
| `/spec-validate` | `/speckit.validate` |
| `/spec-audit` | `/speckit.audit` |
| `/spec-unlock` | `/speckit.unlock` |
| `/spec-auto` | `/speckit.auto` |
| `/spec-status` | `/speckit.status` |

---

## Common Workflows

### Quick Start (New Feature)

```bash
# Create SPEC
/speckit.new Add user authentication with OAuth2

# Optional: Quality checks
/speckit.clarify SPEC-KIT-065
/speckit.checklist SPEC-KIT-065

# Full automation
/speckit.auto SPEC-KIT-065
```

### Manual Stage-by-Stage

```bash
/speckit.plan SPEC-KIT-065
/speckit.tasks SPEC-KIT-065
/speckit.implement SPEC-KIT-065
/speckit.validate SPEC-KIT-065
/speckit.audit SPEC-KIT-065
/speckit.unlock SPEC-KIT-065
```

### Rapid Prototyping (Skip Validation)

```bash
/speckit.auto SPEC-KIT-065 --skip-validate --skip-audit --skip-unlock
# Cost: $0.66 (75% savings)
```

### Documentation Only

```bash
/speckit.auto SPEC-KIT-065 --stages=specify,plan,unlock
# Cost: $1.15 (57% savings)
```

### Code Refactoring Focus

```bash
/speckit.auto SPEC-KIT-065 --stages=implement,validate,unlock
# Cost: $1.06 (61% savings)
```
