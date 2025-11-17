# Quality Gates

Comprehensive guide to the 3-checkpoint quality validation system.

---

## Overview

The **Quality Gates system** provides autonomous quality assurance with three strategic checkpoints:

- **BeforeSpecify** (Clarify): Resolve PRD ambiguities before planning
- **AfterSpecify** (Checklist): Validate PRD + plan quality before tasks
- **AfterTasks** (Analyze): Check cross-artifact consistency before code

**Key Features**:
- **Native heuristics**: Zero agents, $0 cost, <1s execution
- **5-phase state machine**: Executing → Processing → Validating → AwaitingHuman → Guardrail
- **GPT-5 validation**: Majority answer confirmation ($0.05/issue)
- **User escalation**: Modal UI for critical decisions
- **Checkpoint memoization**: Completed gates skipped on resume
- **Single-flight guard**: Prevents duplicate spawns

**Cost**: ~$0.20 total for 3 checkpoints (included in $2.70 /speckit.auto)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/`

---

## 3 Strategic Checkpoints

### Checkpoint Overview

| Checkpoint | Trigger | Gate Type | Purpose | Cost | Time |
|-----------|---------|-----------|---------|------|------|
| **BeforeSpecify** | Before Plan | Clarify | Ambiguity detection | $0 | <1s |
| **AfterSpecify** | Before Tasks | Checklist | Quality scoring | $0 | <1s |
| **AfterTasks** | Before Implement | Analyze | Consistency check | $0 | <1s |

**Philosophy**: "Fail fast, recover early" - catch issues before expensive stages

---

### Checkpoint 1: BeforeSpecify (Clarify)

**Trigger**: Before Plan stage

**Purpose**: Detect and resolve PRD ambiguities early

**Gate**: `/speckit.clarify` (native)

**When to Use**:
- New SPEC with complex requirements
- User-written PRD (not AI-generated)
- Cross-team specifications (unclear expectations)

**What It Checks**:
- **Vague language**: "should", "might", "consider", "probably", "maybe", "could"
- **Incomplete markers**: "TBD", "TODO", "FIXME", "XXX", "???"
- **Quantifier ambiguity**: "fast", "slow", "scalable", "responsive", "secure" (without metrics)
- **Scope gaps**: "etc.", "and so on", "similar", "various"
- **Time ambiguity**: "soon", "later", "eventually", "ASAP", "when possible"

**Example Output**:

```json
{
  "ambiguities": [
    {
      "id": "FR-001-perf-vague",
      "location": "spec.md:45 (Performance Requirements)",
      "text": "System should be fast and responsive",
      "severity": "Critical",
      "question": "What is the target response time?",
      "suggestion": "Specify: 'API response time <200ms (p95)'"
    },
    {
      "id": "FR-002-scale-vague",
      "location": "spec.md:67 (Scalability)",
      "text": "Must handle lots of users",
      "severity": "Important",
      "question": "How many concurrent users?",
      "suggestion": "Specify: '10,000 concurrent users'"
    }
  ],
  "total_count": 12,
  "critical_count": 3,
  "important_count": 5,
  "minor_count": 4
}
```

**Pass Criteria**: ≤2 critical ambiguities

---

### Checkpoint 2: AfterSpecify (Checklist)

**Trigger**: Before Tasks stage

**Purpose**: Validate PRD + plan quality against rubric

**Gate**: `/speckit.checklist` (native)

**When to Use**:
- After plan.md generated
- Before task decomposition
- Ensure completeness before implementation starts

**What It Checks**:

**Rubric** (100 points total):

| Category | Weight | Checks |
|----------|--------|--------|
| **Completeness** | 30% | Required sections present, all requirements addressed |
| **Clarity** | 20% | Specific metrics, clear acceptance criteria |
| **Testability** | 30% | Measurable outcomes, test scenarios defined |
| **Consistency** | 20% | Plan aligns with PRD, no contradictions |

**Detailed Scoring**:

```rust
// Completeness (30 points)
- PRD sections: Background, Requirements, Acceptance Criteria (10 pts)
- Plan sections: Work Breakdown, Acceptance Mapping, Risks (10 pts)
- All FR/NFR requirements addressed in plan (10 pts)

// Clarity (20 points)
- Quantified requirements (no "fast", "scalable" without metrics) (10 pts)
- Specific acceptance criteria (pass/fail clear) (10 pts)

// Testability (30 points)
- Each requirement has test scenario (15 pts)
- Acceptance mapping complete (FR → validation step → test artifact) (15 pts)

// Consistency (20 points)
- Plan features match PRD scope (no extras, no missing) (10 pts)
- No contradictions between PRD and plan (10 pts)
```

**Example Output**:

```json
{
  "score": 82,
  "grade": "B",
  "category_scores": {
    "completeness": 27,
    "clarity": 15,
    "testability": 25,
    "consistency": 15
  },
  "issues": [
    {
      "category": "clarity",
      "severity": "Important",
      "description": "FR-003 uses 'fast' without metric",
      "location": "spec.md:78",
      "suggestion": "Specify: '<2s processing time'"
    },
    {
      "category": "testability",
      "severity": "Important",
      "description": "NFR-002 has no test scenario",
      "location": "plan.md:145",
      "suggestion": "Add load testing scenario for 10k users"
    }
  ]
}
```

**Pass Criteria**: Score ≥80 (grade B or better)

---

### Checkpoint 3: AfterTasks (Analyze)

**Trigger**: Before Implement stage

**Purpose**: Cross-artifact consistency validation

**Gate**: `/speckit.analyze` (native)

**When to Use**:
- After tasks.md generated
- Before code generation
- Final check before committing to implementation

**What It Checks**:

| Check Type | Description | Example |
|-----------|-------------|---------|
| **ID consistency** | Referenced IDs exist in source docs | FR-001 in plan must exist in PRD |
| **Requirement coverage** | All PRD requirements addressed | No orphaned requirements |
| **Contradiction detection** | Conflicting statements | Plan says 3-tier, tasks say monolithic |
| **Version drift** | File modification time anomalies | PRD modified after plan created |
| **Orphan tasks** | Tasks without PRD backing | Task for feature not in scope |
| **Scope creep** | Plan features not in PRD | Extra features added during planning |

**Example Output**:

```json
{
  "issues": [
    {
      "type": "id_consistency",
      "severity": "Critical",
      "description": "plan.md references FR-005, but spec.md only defines FR-001 through FR-004",
      "locations": ["plan.md:89", "spec.md:50-120"],
      "fix": "Either add FR-005 to spec.md or remove from plan.md"
    },
    {
      "type": "contradiction",
      "severity": "Important",
      "description": "spec.md specifies 'RESTful API', plan.md mentions 'GraphQL endpoint'",
      "locations": ["spec.md:67", "plan.md:123"],
      "fix": "Align on single API approach"
    },
    {
      "type": "orphan_task",
      "severity": "Important",
      "description": "Task T-15 implements 'Dark mode toggle', but no FR/NFR covers UI theming",
      "locations": ["tasks.md:45"],
      "fix": "Add NFR-009 for dark mode support"
    }
  ],
  "critical_count": 1,
  "important_count": 2,
  "minor_count": 0
}
```

**Pass Criteria**: 0 critical issues

---

## 5-Phase State Machine

### Phase Transitions

```
Phase 1: QualityGateExecuting
    ↓ (all agents complete)
Phase 2: QualityGateProcessing
    ↓ (classification done)
Phase 3: QualityGateValidating
    ↓ (GPT-5 validation complete OR no medium-confidence issues)
Phase 4: QualityGateAwaitingHuman
    ↓ (user answers all questions OR no escalations)
Phase 5: Guardrail (checkpoint complete, return to pipeline)
```

---

### Phase 1: QualityGateExecuting

**Purpose**: Spawn native gate agents (clarify, checklist, analyze)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:1121-1173`

**State**:

```rust
QualityGateExecuting {
    checkpoint: QualityCheckpoint,           // BeforeSpecify, AfterSpecify, AfterTasks
    gates: Vec<QualityGateType>,             // [Clarify] or [Checklist] or [Analyze]
    expected_agents: Vec<String>,            // ["clarify-native"] (no external agents)
    completed_agents: HashSet<String>,       // Agents that finished
    results: HashMap<String, Value>,         // Agent outputs (JSON)
    native_agent_ids: Option<Vec<String>>,   // SPEC-KIT-900: Native agent tracking
}
```

**Single-Flight Guard**:

```rust
// Check for already-running agents (prevent duplicates)
let already_running = {
    if let Ok(manager_check) = AGENT_MANAGER.try_read() {
        let running_agents = manager_check.get_running_agents();
        let mut matched = Vec::new();

        for (agent_id, model, _status) in running_agents {
            for expected in &expected_agents {
                if model.to_lowercase().contains(expected) {
                    matched.push((expected.to_string(), agent_id));
                    break;
                }
            }
        }
        matched
    } else {
        Vec::new()
    }
};

if !already_running.is_empty() {
    tracing::warn!(
        "DUPLICATE SPAWN DETECTED: {} quality gate agents already running",
        already_running.len()
    );
    return; // Skip duplicate spawn
}
```

**Agent Submission**:

```rust
// Native gates are instant (no async agents)
let result = match checkpoint {
    QualityCheckpoint::BeforeSpecify => {
        clarify_native::detect_ambiguities(spec_id, working_dir)?
    }
    QualityCheckpoint::AfterSpecify => {
        checklist_native::compute_quality_score(spec_id, working_dir)?
    }
    QualityCheckpoint::AfterTasks => {
        analyze_native::check_consistency(spec_id, working_dir)?
    }
};

// Store result
results.insert("native".to_string(), result);
completed_agents.insert("native".to_string());

// Transition to Processing
advance_to_processing(ctx, checkpoint, results)?;
```

**Duration**: <1 second (native operations)

---

### Phase 2: QualityGateProcessing

**Purpose**: Classify issues by severity and confidence

**State**:

```rust
QualityGateProcessing {
    checkpoint: QualityCheckpoint,
    auto_resolved: Vec<QualityIssue>,   // High confidence + minor severity
    escalated: Vec<QualityIssue>,       // Requires human decision
}
```

**Classification Algorithm**:

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/quality.rs:200-350`

```rust
pub fn classify_issues(
    results: &HashMap<String, Value>,
    checkpoint: QualityCheckpoint,
) -> (Vec<QualityIssue>, Vec<QualityIssue>) {
    let mut auto_resolved = Vec::new();
    let mut escalated = Vec::new();

    // Parse native gate output
    let issues = parse_gate_results(results, checkpoint)?;

    for issue in issues {
        match (issue.confidence, issue.severity) {
            // Auto-resolve: High confidence + Minor severity
            (Confidence::High, Severity::Minor) => {
                auto_resolved.push(issue);
            }

            // Auto-resolve: Unanimous agreement (3/3 agents)
            (Confidence::High, _) if issue.unanimous => {
                auto_resolved.push(issue);
            }

            // Medium confidence: Submit to GPT-5
            (Confidence::Medium, _) => {
                // Will be validated in next phase
                escalated.push(issue);
            }

            // Escalate: Low confidence OR Critical severity
            (Confidence::Low, _) | (_, Severity::Critical) => {
                escalated.push(issue);
            }
        }
    }

    (auto_resolved, escalated)
}
```

**Confidence Levels**:

```rust
pub enum Confidence {
    High,       // Unanimous (3/3 agents agree) OR pattern match (native)
    Medium,     // Majority (2/3 agents agree)
    Low,        // No consensus (1/1/1 split)
}
```

**Severity Levels**:

```rust
pub enum Severity {
    Critical,   // Blocks progress (ID mismatch, contradiction)
    Important,  // Should fix (vague requirements, missing tests)
    Minor,      // Nice to have (typos, formatting)
}
```

**Transition**:
- If `escalated` contains Medium confidence issues → **Phase 3: Validating**
- If only Low/Critical in `escalated` → **Phase 4: AwaitingHuman**
- If `escalated` is empty → **Phase 5: Guardrail**

---

### Phase 3: QualityGateValidating

**Purpose**: GPT-5 validates medium-confidence majority answers

**State**:

```rust
QualityGateValidating {
    checkpoint: QualityCheckpoint,
    auto_resolved: Vec<QualityIssue>,
    pending_validations: Vec<(QualityIssue, String)>,  // (issue, validation_id)
    completed_validations: HashMap<usize, GPT5ValidationResult>,
}
```

**GPT-5 Validation Submission**:

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:889-996`

```rust
fn submit_gpt5_validations(
    widget: &mut ChatWidget,
    majority_issues: &[QualityIssue],
    spec_id: &str,
    cwd: &Path,
    checkpoint: QualityCheckpoint,
) {
    for (idx, issue) in majority_issues.iter().enumerate() {
        // Build GPT-5 prompt
        let prompt = format!(
            "Review this quality gate issue and majority answer:\n\n\
             Issue: {}\n\
             Severity: {:?}\n\
             Majority Answer (2/3 agents): {}\n\n\
             Context:\n{}\n\n\
             Question:\n\
             1. Does the majority answer align with the spec's intent?\n\
             2. Should we auto-apply this answer or escalate to human?\n\n\
             Respond with JSON:\n\
             {{\n\
               \"agrees_with_majority\": bool,\n\
               \"reasoning\": string,\n\
               \"recommended_answer\": string|null,\n\
               \"confidence\": \"high\"|\"medium\"|\"low\"\n\
             }}",
            issue.description,
            issue.severity,
            issue.majority_answer.as_ref().unwrap(),
            read_context_files(spec_id, cwd)?,
        );

        // Submit to gpt5-medium
        let validation_id = widget.submit_prompt(
            "GPT-5 Validation".to_string(),
            prompt,
        );

        // Track pending validation
        pending_validations.push((issue.clone(), validation_id));
    }
}
```

**Validation Response Format**:

```json
{
  "agrees_with_majority": true,
  "reasoning": "The majority answer '10,000 concurrent users' is specific and measurable, aligns with typical e-commerce scale, and resolves the ambiguity effectively.",
  "recommended_answer": "10,000 concurrent users (95th percentile)",
  "confidence": "high"
}
```

**Processing Validation Results**:

```rust
fn process_gpt5_validations(
    completed_validations: &HashMap<usize, GPT5ValidationResult>,
    auto_resolved: &mut Vec<QualityIssue>,
    escalated: &mut Vec<QualityIssue>,
    pending_issues: Vec<QualityIssue>,
) {
    for (idx, issue) in pending_issues.into_iter().enumerate() {
        if let Some(validation) = completed_validations.get(&idx) {
            if validation.agrees_with_majority && validation.confidence == "high" {
                // GPT-5 agrees: Auto-apply
                auto_resolved.push(issue.with_answer(validation.recommended_answer.clone()));
            } else {
                // GPT-5 disagrees: Escalate to human
                escalated.push(issue.with_gpt5_reasoning(validation.reasoning.clone()));
            }
        }
    }
}
```

**Transition**:
- All validations complete → **Phase 4: AwaitingHuman** (if any escalated issues)
- OR → **Phase 5: Guardrail** (if all auto-resolved)

**Cost**: ~$0.05 per medium-confidence issue (gpt5-medium validation)

---

### Phase 4: QualityGateAwaitingHuman

**Purpose**: Escalate critical/low-confidence issues to user

**State**:

```rust
QualityGateAwaitingHuman {
    checkpoint: QualityCheckpoint,
    escalated_issues: Vec<QualityIssue>,
    escalated_questions: Vec<EscalatedQuestion>,
    answers: HashMap<String, String>,  // question_id → user answer
}
```

**UI Modal**:

**Location**: `codex-rs/tui/src/bottom_pane/quality_gate_modal.rs:50-200`

```
┌────────────────────────────────────────────────────────────┐
│ Quality Gate: AfterSpecify (Checklist)                    │
├────────────────────────────────────────────────────────────┤
│ Issue 1 of 3: Critical                                     │
│                                                            │
│ Description:                                               │
│ spec.md references FR-005, but spec.md only defines       │
│ FR-001 through FR-004.                                    │
│                                                            │
│ Locations:                                                 │
│ - plan.md:89                                              │
│ - spec.md:50-120                                          │
│                                                            │
│ Suggested Fix:                                             │
│ Either add FR-005 to spec.md or remove from plan.md      │
│                                                            │
│ How should we resolve this?                                │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ [Your answer here]                                     │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                            │
│ [Tab] Next   [Shift+Tab] Previous   [Enter] Submit        │
└────────────────────────────────────────────────────────────┘
```

**Question Collection Flow**:

```rust
pub fn collect_user_answers(
    ctx: &mut impl SpecKitContext,
    escalated_questions: &[EscalatedQuestion],
) -> Result<HashMap<String, String>> {
    let mut answers = HashMap::new();

    // Show modal for each question
    for (idx, question) in escalated_questions.iter().enumerate() {
        ctx.show_quality_gate_modal(QualityGateModal {
            checkpoint: question.checkpoint,
            current_index: idx,
            total_questions: escalated_questions.len(),
            question: question.clone(),
        });

        // Wait for user input (blocking)
        let answer = ctx.wait_for_modal_input()?;

        answers.insert(question.id.clone(), answer);
    }

    Ok(answers)
}
```

**Auto-Apply Changes**:

```rust
pub fn apply_user_answers(
    spec_id: &str,
    working_dir: &Path,
    answers: &HashMap<String, String>,
    escalated_issues: &[QualityIssue],
) -> Result<Vec<PathBuf>> {
    let mut modified_files = Vec::new();

    for issue in escalated_issues {
        if let Some(answer) = answers.get(&issue.id) {
            // Apply answer to appropriate file
            let file_path = issue.location.file_path();
            let modified = apply_answer_to_file(file_path, answer, &issue)?;

            if modified {
                modified_files.push(file_path.to_path_buf());
            }
        }
    }

    // Git commit quality gate changes
    if !modified_files.is_empty() {
        git_commit_quality_gate_changes(spec_id, &modified_files)?;
    }

    Ok(modified_files)
}
```

**Git Commit Example**:

```bash
git add spec.md plan.md
git commit -m "fix(SPEC-KIT-070): resolve AfterSpecify quality gate issues

- Added FR-005 to spec.md (user escalation)
- Clarified 10,000 concurrent users (GPT-5 validated)
- Fixed dark mode task scope (user escalation)

Quality gate: AfterSpecify (Checklist)
Score: 82 → 95 (B → A)
"
```

**Transition**: After all answers applied → **Phase 5: Guardrail**

---

### Phase 5: Guardrail (Checkpoint Complete)

**Purpose**: Mark checkpoint as complete, return to pipeline

**Actions**:

```rust
pub fn complete_quality_gate(
    ctx: &mut impl SpecKitContext,
    checkpoint: QualityCheckpoint,
) -> Result<()> {
    // Mark checkpoint complete (memoization)
    ctx.spec_auto_state_mut()
        .as_mut()?
        .completed_checkpoints
        .insert(checkpoint);

    // Clear quality gate state
    ctx.spec_auto_state_mut()
        .as_mut()?
        .quality_gate_processing = None;

    // Transition to Guardrail phase
    ctx.spec_auto_state_mut()
        .as_mut()?
        .phase = SpecAutoPhase::Guardrail;

    // Continue pipeline advancement
    advance_spec_auto(ctx)?;

    Ok(())
}
```

**Evidence Recording**:

```bash
docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/{SPEC-ID}/quality_gates/
├── AfterSpecify_checkpoint.json       # Checkpoint metadata
├── checklist_result.json              # Native gate output
├── gpt5_validations/
│   ├── issue_001_validation.json     # GPT-5 validation
│   └── issue_002_validation.json
└── user_escalations/
    ├── issue_003_question.json       # Escalated question
    └── issue_003_answer.json         # User answer
```

**Checkpoint Metadata Example**:

```json
{
  "checkpoint": "AfterSpecify",
  "spec_id": "SPEC-KIT-070",
  "gate_type": "checklist",
  "status": "passed",
  "score": 95,
  "initial_score": 82,
  "issues_found": 3,
  "auto_resolved": 1,
  "gpt5_validated": 1,
  "user_escalated": 1,
  "modified_files": ["spec.md", "plan.md"],
  "total_time_ms": 1200,
  "cost": 0.05,
  "timestamp": "2025-10-18T15:45:00Z"
}
```

---

## Native Heuristics

### Clarify Gate Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs:15-200`

```rust
pub struct Ambiguity {
    pub id: String,                 // e.g., "FR-001-perf-vague"
    pub location: String,           // "spec.md:45"
    pub text: String,               // Original vague text
    pub severity: Severity,
    pub question: String,           // Clarifying question
    pub suggestion: String,         // Specific alternative
}

pub fn detect_ambiguities(
    spec_id: &str,
    working_dir: &Path,
) -> Result<Vec<Ambiguity>> {
    let spec_path = working_dir.join(format!("docs/{}/spec.md", spec_id));
    let content = std::fs::read_to_string(spec_path)?;

    let mut ambiguities = Vec::new();

    // Pattern 1: Vague language
    ambiguities.extend(detect_vague_language(&content)?);

    // Pattern 2: Incomplete markers
    ambiguities.extend(detect_incomplete_markers(&content)?);

    // Pattern 3: Quantifier ambiguity
    ambiguities.extend(detect_quantifier_ambiguity(&content)?);

    // Pattern 4: Scope gaps
    ambiguities.extend(detect_scope_gaps(&content)?);

    // Pattern 5: Time ambiguity
    ambiguities.extend(detect_time_ambiguity(&content)?);

    Ok(ambiguities)
}
```

**Pattern Matching Examples**:

```rust
fn detect_vague_language(content: &str) -> Result<Vec<Ambiguity>> {
    let vague_words = [
        "should", "might", "consider", "probably", "maybe", "could",
        "possibly", "potentially", "hopefully", "ideally"
    ];

    let mut ambiguities = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for word in &vague_words {
            if line.to_lowercase().contains(word) {
                ambiguities.push(Ambiguity {
                    id: format!("vague-{}", line_num),
                    location: format!("spec.md:{}", line_num + 1),
                    text: line.to_string(),
                    severity: Severity::Important,
                    question: format!("Is this a firm requirement or optional?"),
                    suggestion: "Replace with 'must' (required) or 'may' (optional)".to_string(),
                });
            }
        }
    }

    Ok(ambiguities)
}

fn detect_quantifier_ambiguity(content: &str) -> Result<Vec<Ambiguity>> {
    let quantifiers = [
        ("fast", "What is the target response time? (e.g., <200ms p95)"),
        ("slow", "What is the maximum acceptable latency?"),
        ("scalable", "How many users/requests? (e.g., 10k concurrent)"),
        ("responsive", "What is the target interaction latency?"),
        ("secure", "Which security standards? (e.g., OWASP Top 10)"),
        ("reliable", "What is the target uptime? (e.g., 99.9%)"),
        ("efficient", "What are the resource constraints? (e.g., <100MB RAM)"),
    ];

    let mut ambiguities = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        for (word, question) in &quantifiers {
            if line.to_lowercase().contains(word) && !has_metric_nearby(line, word) {
                ambiguities.push(Ambiguity {
                    id: format!("quant-{}-{}", word, line_num),
                    location: format!("spec.md:{}", line_num + 1),
                    text: line.to_string(),
                    severity: Severity::Critical,
                    question: question.to_string(),
                    suggestion: format!("Add specific metric after '{}'", word),
                });
            }
        }
    }

    Ok(ambiguities)
}

fn has_metric_nearby(line: &str, word: &str) -> bool {
    // Check if line contains numbers, units, or comparisons near the word
    let patterns = [
        r"\d+", r"<\s*\d+", r">\s*\d+", r"\d+\s*ms", r"\d+\s*MB",
        r"\d+\s*%", r"\d+\s*users", r"\d+\s*requests",
    ];

    patterns.iter().any(|pattern| {
        regex::Regex::new(pattern).unwrap().is_match(line)
    })
}
```

---

### Checklist Gate Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/checklist_native.rs:15-300`

```rust
pub struct QualityReport {
    pub score: u8,              // 0-100
    pub grade: char,            // A, B, C, D, F
    pub category_scores: CategoryScores,
    pub issues: Vec<QualityIssue>,
}

pub struct CategoryScores {
    pub completeness: u8,       // 0-30
    pub clarity: u8,            // 0-20
    pub testability: u8,        // 0-30
    pub consistency: u8,        // 0-20
}

pub fn compute_quality_score(
    spec_id: &str,
    working_dir: &Path,
) -> Result<QualityReport> {
    let spec_path = working_dir.join(format!("docs/{}/spec.md", spec_id));
    let plan_path = working_dir.join(format!("docs/{}/plan.md", spec_id));

    let spec_content = std::fs::read_to_string(spec_path)?;
    let plan_content = std::fs::read_to_string(plan_path)?;

    // Score each category
    let completeness = score_completeness(&spec_content, &plan_content)?;
    let clarity = score_clarity(&spec_content)?;
    let testability = score_testability(&spec_content, &plan_content)?;
    let consistency = score_consistency(&spec_content, &plan_content)?;

    let total_score = completeness + clarity + testability + consistency;
    let grade = match total_score {
        90..=100 => 'A',
        80..=89 => 'B',
        70..=79 => 'C',
        60..=69 => 'D',
        _ => 'F',
    };

    Ok(QualityReport {
        score: total_score,
        grade,
        category_scores: CategoryScores {
            completeness,
            clarity,
            testability,
            consistency,
        },
        issues: collect_issues(&spec_content, &plan_content)?,
    })
}
```

**Completeness Scoring**:

```rust
fn score_completeness(spec: &str, plan: &str) -> Result<u8> {
    let mut score = 0u8;

    // PRD sections (10 points)
    let required_prd_sections = [
        "Background", "Requirements", "Acceptance Criteria",
        "Constraints", "Out of Scope"
    ];
    let prd_sections_present = required_prd_sections.iter()
        .filter(|section| spec.contains(section))
        .count();
    score += (prd_sections_present as u8 * 10) / required_prd_sections.len() as u8;

    // Plan sections (10 points)
    let required_plan_sections = [
        "Work Breakdown", "Acceptance Mapping", "Risks",
        "Exit Criteria", "Consensus"
    ];
    let plan_sections_present = required_plan_sections.iter()
        .filter(|section| plan.contains(section))
        .count();
    score += (plan_sections_present as u8 * 10) / required_plan_sections.len() as u8;

    // All requirements addressed (10 points)
    let spec_requirements = extract_requirements(spec);
    let plan_requirements = extract_requirements(plan);
    let coverage_ratio = plan_requirements.len() as f32 / spec_requirements.len() as f32;
    score += (coverage_ratio * 10.0) as u8;

    Ok(score.min(30))
}
```

---

### Analyze Gate Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/analyze_native.rs:15-400`

```rust
pub fn check_consistency(
    spec_id: &str,
    working_dir: &Path,
) -> Result<Vec<ConsistencyIssue>> {
    let spec_path = working_dir.join(format!("docs/{}/spec.md", spec_id));
    let plan_path = working_dir.join(format!("docs/{}/plan.md", spec_id));
    let tasks_path = working_dir.join(format!("docs/{}/tasks.md", spec_id));

    let spec_content = std::fs::read_to_string(spec_path)?;
    let plan_content = std::fs::read_to_string(plan_path)?;
    let tasks_content = std::fs::read_to_string(tasks_path)?;

    let mut issues = Vec::new();

    // Check 1: ID consistency
    issues.extend(check_id_consistency(&spec_content, &plan_content, &tasks_content)?);

    // Check 2: Requirement coverage
    issues.extend(check_requirement_coverage(&spec_content, &plan_content)?);

    // Check 3: Contradictions
    issues.extend(detect_contradictions(&spec_content, &plan_content)?);

    // Check 4: Version drift
    issues.extend(check_version_drift(spec_id, working_dir)?);

    // Check 5: Orphan tasks
    issues.extend(find_orphan_tasks(&spec_content, &tasks_content)?);

    // Check 6: Scope creep
    issues.extend(detect_scope_creep(&spec_content, &plan_content)?);

    Ok(issues)
}
```

**ID Consistency Check**:

```rust
fn check_id_consistency(
    spec: &str,
    plan: &str,
    tasks: &str,
) -> Result<Vec<ConsistencyIssue>> {
    let mut issues = Vec::new();

    // Extract all FR/NFR IDs from spec
    let spec_ids = extract_requirement_ids(spec);

    // Find references in plan and tasks
    for doc in [plan, tasks] {
        let referenced_ids = extract_referenced_ids(doc);

        for referenced in referenced_ids {
            if !spec_ids.contains(&referenced) {
                issues.push(ConsistencyIssue {
                    type_: IssueType::IdConsistency,
                    severity: Severity::Critical,
                    description: format!(
                        "References {}, but spec only defines {:?}",
                        referenced,
                        spec_ids
                    ),
                    locations: vec![
                        find_location(doc, &referenced),
                        "spec.md:1".to_string(),
                    ],
                    fix: format!(
                        "Either add {} to spec.md or remove from {}",
                        referenced,
                        if doc == plan { "plan.md" } else { "tasks.md" }
                    ),
                });
            }
        }
    }

    Ok(issues)
}

fn extract_requirement_ids(content: &str) -> HashSet<String> {
    let re = regex::Regex::new(r"(FR|NFR)-\d+").unwrap();
    re.find_iter(content)
        .map(|m| m.as_str().to_string())
        .collect()
}
```

**Contradiction Detection** (keyword-based):

```rust
fn detect_contradictions(spec: &str, plan: &str) -> Result<Vec<ConsistencyIssue>> {
    let mut issues = Vec::new();

    // Architecture contradictions
    let arch_pairs = [
        ("monolithic", "microservices"),
        ("REST", "GraphQL"),
        ("SQL", "NoSQL"),
        ("synchronous", "asynchronous"),
    ];

    for (term_a, term_b) in &arch_pairs {
        if spec.to_lowercase().contains(term_a) && plan.to_lowercase().contains(term_b) {
            issues.push(ConsistencyIssue {
                type_: IssueType::Contradiction,
                severity: Severity::Important,
                description: format!(
                    "spec.md mentions '{}', plan.md mentions '{}'",
                    term_a, term_b
                ),
                locations: vec![
                    find_location(spec, term_a),
                    find_location(plan, term_b),
                ],
                fix: "Align on single architectural approach".to_string(),
            });
        }
    }

    Ok(issues)
}
```

---

## Checkpoint Memoization

### Completed Checkpoint Tracking

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/state.rs:433-479`

```rust
pub struct SpecAutoState {
    // Memoization: Set of completed checkpoints (never run twice)
    pub completed_checkpoints: HashSet<QualityCheckpoint>,

    // Currently processing checkpoint (prevents recursion)
    pub quality_gate_processing: Option<QualityCheckpoint>,
}

pub fn determine_quality_checkpoint(
    stage: SpecStage,
    completed: &HashSet<QualityCheckpoint>,
) -> Option<QualityCheckpoint> {
    let checkpoint = match stage {
        SpecStage::Plan => QualityCheckpoint::BeforeSpecify,
        SpecStage::Tasks => QualityCheckpoint::AfterSpecify,
        SpecStage::Implement => QualityCheckpoint::AfterTasks,
        _ => return None,  // No checkpoint for Validate, Audit, Unlock
    };

    // Skip if already completed
    if completed.contains(&checkpoint) {
        None
    } else {
        Some(checkpoint)
    }
}
```

**Persistence**: Evidence file tracks completed checkpoints

```bash
docs/SPEC-OPS-004.../evidence/commands/{SPEC-ID}/quality_gates/completed_checkpoints.json
```

```json
{
  "spec_id": "SPEC-KIT-070",
  "completed": [
    {
      "checkpoint": "BeforeSpecify",
      "timestamp": "2025-10-18T14:30:00Z",
      "status": "passed"
    },
    {
      "checkpoint": "AfterSpecify",
      "timestamp": "2025-10-18T14:45:00Z",
      "status": "passed",
      "initial_score": 82,
      "final_score": 95
    }
  ]
}
```

**Resume Behavior**:

```bash
# First run
/speckit.auto SPEC-KIT-070
  → BeforeSpecify (Clarify): Runs, passes, marked complete
  → Plan stage: Runs
  → AfterSpecify (Checklist): Runs, passes, marked complete
  → Tasks stage: Runs
  → AfterTasks (Analyze): Runs, FAILS (user fixes issues)

# Resume after fixing issues
/speckit.auto SPEC-KIT-070 --from tasks
  → BeforeSpecify: SKIPPED (already complete)
  → AfterSpecify: SKIPPED (already complete)
  → AfterTasks (Analyze): Runs again (not marked complete yet)
  → Passes, marked complete
  → Implement stage: Continues
```

---

## Cost & Performance

### Cost Breakdown

| Component | Cost | Time |
|-----------|------|------|
| **Native gates** (Clarify, Analyze, Checklist) | $0.00 | <1s each |
| **GPT-5 validation** (per medium-confidence issue) | ~$0.05 | 3-5s |
| **Total per checkpoint** (typical) | ~$0.05-0.10 | 1-5s |
| **Total for 3 checkpoints** | ~$0.20 | 3-15s |

**Example** (AfterSpecify with 2 medium-confidence issues):
```
Checklist native:         $0.00 (0.8s)
GPT-5 validation (2×):    $0.10 (6s)
User escalation (1 issue): $0.00 (30s user time)
TOTAL:                    $0.10 (37s)
```

### Performance Metrics

**Native Gate Execution** (<1s):
- Clarify: ~600ms (pattern matching on spec.md)
- Checklist: ~800ms (scoring 4 categories)
- Analyze: ~900ms (cross-artifact consistency)

**GPT-5 Validation** (3-5s per issue):
- Prompt construction: 50ms
- GPT-5 inference: 2-4s
- Response parsing: 100ms

**User Escalation** (variable):
- Modal display: 50ms
- User reading + answering: 30-120s (human time)
- Auto-apply changes: 200ms
- Git commit: 100ms

---

## Summary

**Quality Gates System Highlights**:

1. **3 Strategic Checkpoints**: BeforeSpecify (Clarify), AfterSpecify (Checklist), AfterTasks (Analyze) - fail fast, recover early
2. **5-Phase State Machine**: Executing → Processing → Validating → AwaitingHuman → Guardrail
3. **Native Heuristics**: Zero agents, $0 cost, <1s execution (pattern matching, rubric scoring, consistency checks)
4. **GPT-5 Validation**: Majority answer confirmation for medium-confidence issues (~$0.05 each)
5. **User Escalation**: Modal UI for critical/low-confidence decisions, auto-apply + git commit
6. **Checkpoint Memoization**: Completed gates skipped on resume (evidence persistence)
7. **Single-Flight Guard**: Prevents duplicate agent spawns during concurrent operations

**Next Steps**:
- [Native Operations](native-operations.md) - Clarify, Analyze, Checklist deep dive
- [Evidence Repository](evidence-repository.md) - Artifact storage and retrieval
- [Cost Tracking](cost-tracking.md) - Per-stage cost breakdown

---

**File References**:
- Quality gate handler: `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs:50-1200`
- Clarify native: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs:15-200`
- Checklist native: `codex-rs/tui/src/chatwidget/spec_kit/checklist_native.rs:15-300`
- Analyze native: `codex-rs/tui/src/chatwidget/spec_kit/analyze_native.rs:15-400`
- State machine: `codex-rs/tui/src/chatwidget/spec_kit/state.rs:15-479`
- Quality modal: `codex-rs/tui/src/bottom_pane/quality_gate_modal.rs:50-200`
