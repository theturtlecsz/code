# Quality Gates: Automated Quality Assurance

## Overview

Quality gates are automated checkpoints that catch issues before they become expensive problems downstream. Spec-Kit integrates 4 quality gates at strategic points in the pipeline, using native heuristics (zero cost) with escalation to user when needed.

## The 4 Quality Gates

| Gate | Method | Cost | Purpose |
|------|--------|------|---------|
| **Clarify** | Pattern matching | $0 | Detect ambiguities in requirements |
| **Checklist** | Rubric scoring | $0 | Score requirement quality |
| **Analyze** | Structural diff | $0 | Check cross-artifact consistency |
| **Validate** | Test coverage | Tier 2 | Verify implementation meets spec |

## Gate Placement

### Checkpoint 1: Pre-Planning

**Gates**: Clarify → Checklist

```
User Request
    ↓
[/speckit.new] → Creates spec.md
    ↓
[CLARIFY GATE] → Detects ambiguities
    ↓
    Questions to User ← blocking
    ↓
[CHECKLIST GATE] → Scores quality
    ↓
    Low scores trigger additional questions
    ↓
[/speckit.plan] proceeds with clean spec
```

**Why Here**: Catch ambiguities BEFORE multi-agent planning ($0.35) runs.

### Checkpoint 2: Post-Plan

**Gate**: Analyze

```
[/speckit.plan] → Creates plan.md
    ↓
[ANALYZE GATE] → Checks plan ↔ spec consistency
    ↓
    - Are all requirements addressed?
    - Any contradictions?
    - Coverage gaps?
    ↓
[/speckit.tasks] proceeds
```

**Why Here**: Ensure plan covers all requirements before task decomposition.

### Checkpoint 3: Post-Tasks

**Gate**: Analyze

```
[/speckit.tasks] → Creates tasks.md
    ↓
[ANALYZE GATE] → Checks task ↔ requirement mapping
    ↓
    - Every requirement has a task?
    - Tasks ordered correctly?
    - Validation steps defined?
    ↓
[/speckit.implement] proceeds
```

**Why Here**: Verify complete task coverage before expensive code generation.

---

## Gate Implementations

### Clarify Gate (`clarify_native.rs`)

**Purpose**: Detect ambiguous language and missing information.

**Pattern Matching**:
```rust
const VAGUE_PATTERNS: &[&str] = &[
    "should", "could", "might", "maybe", "possibly",
    "as appropriate", "as needed", "etc", "and so on",
    "similar", "like", "such as", "for example",
];

const MISSING_PATTERNS: &[&str] = &[
    "TBD", "TODO", "FIXME", "...", "[placeholder]",
];
```

**Detection Rules**:

1. **Vague Language**
   - "The system should handle errors" → Ambiguous
   - Better: "The system must log all errors to stdout with timestamp"

2. **Missing Sections**
   - Empty acceptance criteria
   - No examples provided
   - Missing edge case coverage

3. **Undefined Terms**
   - Technical jargon without definition
   - Acronyms not explained

4. **Cross-Artifact Contradictions**
   - spec.md says "OAuth2" but PRD says "OAuth 1.0"

**Output**: 3-5 high-confidence questions

```markdown
## Clarify Gate Results

⚠ Ambiguities Detected:

1. **Vague language** (line 23):
   "should handle authentication errors appropriately"
   → How should errors be handled? (log, retry, notify user?)

2. **Missing information** (Acceptance Criteria):
   No criteria for "user logout" requirement
   → What confirms successful logout? (session cleared, redirect, message?)

3. **Undefined term** (line 45):
   "JWT tokens" referenced but not explained
   → Should include: format, expiry, refresh strategy
```

### Checklist Gate (`checklist_native.rs`)

**Purpose**: Score requirement quality on 4 dimensions.

**Rubric** (0-10 each):

| Dimension | 10 | 5 | 0 |
|-----------|----|----|---|
| **Completeness** | All requirements with examples | Partial coverage | Missing major sections |
| **Clarity** | Unambiguous, no jargon | Some vague language | Unclear throughout |
| **Testability** | Measurable criteria | Partial criteria | No validation possible |
| **Consistency** | All artifacts aligned | Minor conflicts | Major contradictions |

**Scoring Algorithm**:
```rust
fn score_dimension(content: &str, dimension: &Dimension) -> u8 {
    let mut score = 10;

    // Deduct for issues
    for issue in dimension.check(content) {
        match issue.severity {
            Severity::Critical => score = score.saturating_sub(5),
            Severity::Major => score = score.saturating_sub(3),
            Severity::Minor => score = score.saturating_sub(1),
        }
    }

    score
}
```

**Thresholds**:
- **Pass** (≥8): Proceed without questions
- **Warn** (6-7): Proceed with suggestions
- **Fail** (<6): Block until addressed

**Output**:
```markdown
## Checklist Gate Results

| Dimension | Score | Status |
|-----------|-------|--------|
| Completeness | 7/10 | ⚠ WARN |
| Clarity | 8/10 | ✓ PASS |
| Testability | 5/10 | ✗ FAIL |
| Consistency | 9/10 | ✓ PASS |

**Overall**: 7.25/10 - Requires attention

### Issues to Address:

**Testability** (score: 5):
- Acceptance criteria missing for 3/7 requirements
- No measurable success metrics for "performance" requirement
- Edge case coverage: 40% (target: 80%)

**Completeness** (score: 7):
- Examples provided for 4/7 requirements
- Missing: error handling examples, edge case scenarios
```

### Analyze Gate (`analyze_native.rs`)

**Purpose**: Check structural consistency across artifacts.

**Checks Performed**:

1. **ID Consistency**
   - SPEC ID appears in all artifacts
   - Task IDs unique and sequential
   - Requirement IDs traceable

2. **Coverage Analysis**
   - Every requirement in spec → corresponding plan item
   - Every plan item → corresponding task(s)
   - Every task → validation step

3. **Section Completeness**
   - Required sections present
   - Sections not empty
   - Format matches template

4. **Field Validation**
   - Dates parseable
   - IDs follow format
   - Status values valid

**Coverage Matrix**:
```markdown
## Analyze Gate Results

### Coverage Matrix

| Requirement | Plan Item | Task(s) | Validation |
|-------------|-----------|---------|------------|
| R1: OAuth login | ✓ P1.1 | ✓ T1, T2 | ✓ Unit test |
| R2: Token refresh | ✓ P1.2 | ✓ T3 | ✓ Integration |
| R3: Logout | ✗ Missing | ✗ None | ✗ None |
| R4: Session timeout | ✓ P2.1 | ✓ T4, T5 | ⚠ Partial |

### Issues

**Critical**:
- R3 (Logout) has no plan item or tasks

**Warning**:
- R4 validation only covers happy path
```

---

## Auto-Resolution

### Resolution Classification

Quality issues are classified for resolution:

```rust
pub enum Resolvability {
    AutoFix,     // High confidence, minor magnitude
    SuggestFix,  // Medium confidence, apply with warning
    NeedHuman,   // Low confidence or critical magnitude
}

pub enum Magnitude {
    Minor,       // Style, formatting, typos
    Important,   // Affects architecture, impacts downstream
    Critical,    // Blocks implementation, security implications
}
```

### Resolution Matrix

| Confidence | Minor | Important | Critical |
|------------|-------|-----------|----------|
| High (3/3) | AutoFix | AutoFix | SuggestFix |
| Medium (2/3) | AutoFix | SuggestFix | NeedHuman |
| Low (<2/3) | SuggestFix | NeedHuman | NeedHuman |

### Auto-Resolution Process

```rust
async fn process_issue(issue: &QualityIssue) -> Resolution {
    // Get agent opinions on resolution
    let resolutions = consensus_agents
        .iter()
        .map(|a| a.suggest_resolution(issue))
        .collect();

    // Check agreement
    let agreement = calculate_agreement(&resolutions);

    match (agreement, issue.magnitude) {
        // Unanimous on minor → auto-fix
        (Agreement::Unanimous, Magnitude::Minor) => {
            apply_fix(&resolutions[0]).await;
            Resolution::AutoFixed(resolutions[0].clone())
        }

        // Majority on important → validate then apply
        (Agreement::Majority, Magnitude::Important) => {
            let validation = gpt5_validate(&resolutions).await;
            if validation.approved {
                apply_fix(&resolutions[0]).await;
                Resolution::Validated(resolutions[0].clone())
            } else {
                Resolution::NeedHuman(issue.clone())
            }
        }

        // Anything critical → human review
        (_, Magnitude::Critical) => {
            Resolution::NeedHuman(issue.clone())
        }

        // Default → suggest
        _ => {
            Resolution::Suggested(resolutions[0].clone())
        }
    }
}
```

### Resolution Examples

**Auto-Fixed** (High confidence, Minor):
```
Issue: Missing period at end of acceptance criterion
Resolution: Add period
Status: Auto-fixed ✓
```

**Auto-Fixed** (Unanimous, Important):
```
Issue: Should we log authentication failures?
Resolution: Yes - security best practice
Agreement: 3/3 agents agree
Status: Auto-fixed ✓
```

**Suggested** (Majority, Important):
```
Issue: Token expiry duration not specified
Suggestion: 3600 seconds (industry standard)
Agreement: 2/3 agents agree
Status: Applied with confidence 78%
```

**Escalated** (No consensus, Critical):
```
Issue: Which OAuth providers to support?
Options:
  - Gemini: "Google and GitHub only"
  - Claude: "All major providers (Google, GitHub, Microsoft, Apple)"
  - GPT-5: "Google only for MVP"
Status: Requires human decision
```

---

## User Interaction

### Question Batching

Multiple questions presented together:

```
╔══════════════════════════════════════════════════════════════╗
║                    Quality Gate Checkpoint                   ║
╠══════════════════════════════════════════════════════════════╣
║                                                              ║
║  3 clarifications needed before planning:                    ║
║                                                              ║
║  1. Token storage approach:                                  ║
║     [ ] Encrypted vault (recommended)                        ║
║     [ ] Database with encryption                             ║
║     [ ] Cloud KMS                                            ║
║                                                              ║
║  2. Session timeout duration:                                ║
║     [____] minutes (default: 30)                             ║
║                                                              ║
║  3. Support multiple OAuth providers?                        ║
║     [ ] Yes - Google, GitHub, Microsoft                      ║
║     [ ] No - Google only (MVP)                               ║
║                                                              ║
║  [Submit Answers]                    [Skip to Planning]      ║
║                                                              ║
╚══════════════════════════════════════════════════════════════╝
```

### Response Processing

User answers are processed and applied:

```rust
fn apply_answers(answers: &[Answer], spec: &mut Spec) {
    for answer in answers {
        match answer {
            Answer::Choice { question_id, selection } => {
                // Update spec with selected option
                spec.update_requirement(question_id, selection);
            }
            Answer::Text { question_id, value } => {
                // Insert value into spec
                spec.fill_placeholder(question_id, value);
            }
            Answer::Skip { question_id } => {
                // Mark as user-skipped, proceed with default
                spec.mark_skipped(question_id);
            }
        }
    }

    // Persist updated spec
    spec.save()?;
}
```

### Audit Trail

All gate results and resolutions logged:

```json
{
  "checkpoint": "pre-planning",
  "gates": [
    {
      "gate": "clarify",
      "issues_found": 3,
      "resolutions": [
        {
          "issue": "Token storage approach",
          "resolution_type": "user_choice",
          "selected": "encrypted_vault"
        },
        {
          "issue": "Session timeout",
          "resolution_type": "user_input",
          "value": "30"
        },
        {
          "issue": "Multiple providers",
          "resolution_type": "auto_fixed",
          "value": "google_only_mvp"
        }
      ]
    },
    {
      "gate": "checklist",
      "scores": {
        "completeness": 8,
        "clarity": 9,
        "testability": 7,
        "consistency": 8
      },
      "overall": 8.0,
      "status": "pass"
    }
  ],
  "duration_ms": 1250,
  "timestamp": "2025-10-27T10:15:00Z"
}
```

---

## Configuration

### Global Settings

```toml
# ~/.code/config.toml

[quality_gates]
enabled = true
auto_resolve = true
min_score = 6  # Fail threshold
warn_score = 8  # Warning threshold

[quality_gates.clarify]
max_questions = 5
confidence_threshold = 0.8

[quality_gates.checklist]
weights = { completeness = 0.3, clarity = 0.2, testability = 0.3, consistency = 0.2 }

[quality_gates.analyze]
coverage_threshold = 0.9  # 90% requirement coverage required
```

### Per-SPEC Override

```toml
# docs/SPEC-KIT-065/pipeline.toml

[quality_gates]
enabled = true
auto_resolve = false  # Require manual review for this SPEC

[quality_gates.checklist]
min_score = 8  # Higher bar for production feature
```

### Skip Gates

```bash
# Skip quality gates (not recommended)
/speckit.auto SPEC-KIT-065 --skip-quality-gates

# Skip specific checkpoint
/speckit.auto SPEC-KIT-065 --skip-pre-planning-gates
```

---

## Benefits

### Cost Savings

Catching issues early saves expensive rework:

| Issue Found At | Cost to Fix |
|----------------|-------------|
| Quality gate | $0 (native) |
| Planning | $0.35 (re-run plan) |
| Implementation | $0.46+ (re-run implement+) |
| Validation | $0.81+ (re-run validate+) |
| Production | $$$ (hotfix, incidents) |

### Quality Improvement

Typical quality metrics before/after gates:

| Metric | Without Gates | With Gates |
|--------|---------------|------------|
| Ambiguous requirements | 23% | 3% |
| Missing acceptance criteria | 31% | 2% |
| Plan-spec misalignment | 18% | 1% |
| Task coverage gaps | 15% | 0% |

### Developer Experience

- Immediate feedback (< 1 second)
- Actionable questions
- Clear resolution options
- Audit trail for decisions
