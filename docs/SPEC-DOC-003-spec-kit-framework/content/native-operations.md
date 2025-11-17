# Native Operations

Comprehensive guide to Tier 0 FREE instant operations.

---

## Overview

**Native Operations** are Tier 0 commands implemented in pure Rust with:

- **Zero agents**: No AI models, pure pattern matching and logic
- **Zero cost**: $0 per execution (vs $0.10-0.80 for agent-based)
- **Instant**: <1 second execution time
- **100% deterministic**: Same input → same output
- **Offline-capable**: No network required

**5 Native Commands**:

| Command | Purpose | Time | Replaced |
|---------|---------|------|----------|
| `/speckit.new` | Create SPEC | <1s | 2 agents ($0.15) |
| `/speckit.clarify` | Ambiguity detection | <1s | 3 agents ($0.80) |
| `/speckit.analyze` | Consistency check | <1s | 3 agents ($0.35) |
| `/speckit.checklist` | Quality scoring | <1s | 3 agents ($0.35) |
| `/speckit.status` | Status dashboard | <1s | N/A (new feature) |

**Total Savings**: $1.65 per full pipeline (was $11, now $2.70 with native ops)

**Principle**: "Agents for reasoning, NOT transactions" (SPEC-KIT-070)

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/*_native.rs`

---

## Philosophy: When to Use Native vs Agents

### Decision Framework

```
Decision Flow:
    ↓
Is the task deterministic? (same input → same output)
    ├─ YES → Native operation ($0, <1s)
    └─ NO  → Agent-based ($0.10-0.80, 3-10min)
          ↓
      Does it require reasoning/judgment?
          ├─ YES → Agents (creativity, analysis)
          └─ NO  → Rethink if native is possible
```

### Examples

**Native (deterministic, pattern-matching)**:
- ✅ Generate SPEC-ID (increment last ID)
- ✅ Detect "TODO" markers in PRD
- ✅ Check if FR-001 exists in spec.md
- ✅ Count required sections present
- ✅ Calculate quality score from rubric

**Agent-Based (reasoning, judgment)**:
- ❌ Draft PRD from user description (creative writing)
- ❌ Architectural planning (strategic decisions)
- ❌ Code generation (complex logic)
- ❌ Ship/no-ship decision (risk assessment)

**Cost Comparison**:

| Task | Native | Agent-Based |
|------|--------|-------------|
| Generate SPEC-ID | $0, <1s | $0.15, 3min |
| Detect ambiguities | $0, <1s | $0.80, 10min |
| Check consistency | $0, <1s | $0.35, 8min |
| Quality scoring | $0, <1s | $0.35, 8min |

**Cumulative Savings**: Native operations save $1.65 per /speckit.auto pipeline

---

## /speckit.new - SPEC Creation

### Purpose

Create new SPEC with instant template filling.

**Replaced**: 2 agents ($0.15, 3min) → Native ($0, <1s)

**Steps**:
1. Generate SPEC-ID (find max ID, increment)
2. Create slug from description
3. Create directory structure
4. Fill PRD template
5. Create spec.md
6. Update SPEC.md tracker

---

### Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:37-97`

```rust
pub fn create_spec(description: &str, cwd: &Path) -> Result<SpecCreationResult, SpecKitError> {
    let description = description.trim();
    if description.is_empty() {
        return Err(SpecKitError::Other("Description cannot be empty".to_string()));
    }

    // Step 1: Generate SPEC-ID
    let spec_id = generate_next_spec_id(cwd)?;

    // Step 2: Create slug
    let slug = create_slug(description);
    let feature_name = capitalize_words(description);

    // Step 3: Create directory
    let dir_name = format!("{}-{}", spec_id, slug);
    let spec_dir = cwd.join("docs").join(&dir_name);
    fs::create_dir_all(&spec_dir)?;

    // Step 4: Fill PRD template
    let prd_path = spec_dir.join("PRD.md");
    let prd_content = fill_prd_template(&spec_id, &feature_name, description)?;
    fs::write(&prd_path, prd_content)?;

    // Step 5: Create spec.md
    let spec_path = spec_dir.join("spec.md");
    let spec_content = fill_spec_template(&spec_id, &feature_name, description)?;
    fs::write(&spec_path, spec_content)?;

    // Step 6: Update SPEC.md tracker
    update_spec_tracker(cwd, &spec_id, &feature_name, &dir_name)?;

    Ok(SpecCreationResult {
        spec_id,
        directory: spec_dir,
        files_created: vec!["PRD.md".to_string(), "spec.md".to_string()],
        feature_name,
        slug,
    })
}
```

---

### SPEC-ID Generation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/spec_id_generator.rs:15-80`

```rust
pub fn generate_next_spec_id(cwd: &Path) -> Result<String> {
    let docs_dir = cwd.join("docs");
    if !docs_dir.exists() {
        return Ok("SPEC-KIT-001".to_string());  // First SPEC
    }

    // Scan all SPEC directories
    let entries = fs::read_dir(&docs_dir)?;
    let mut max_id = 0;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Match SPEC-KIT-XXX pattern
        if let Some(caps) = SPEC_ID_PATTERN.captures(&name) {
            if let Some(num_str) = caps.get(1) {
                if let Ok(num) = num_str.as_str().parse::<usize>() {
                    max_id = max_id.max(num);
                }
            }
        }
    }

    // Increment
    let next_id = max_id + 1;
    Ok(format!("SPEC-KIT-{:03}", next_id))
}
```

**Example**:
```
Existing SPECs: SPEC-KIT-001, SPEC-KIT-002, SPEC-KIT-005
Next ID: SPEC-KIT-006 (not 003 or 004)
```

---

### Slug Generation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/spec_id_generator.rs:82-120`

```rust
pub fn create_slug(description: &str) -> String {
    description
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '\0'  // Remove non-alphanumeric
            }
        })
        .filter(|&c| c != '\0')
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .take(5)  // Max 5 words
        .collect::<Vec<_>>()
        .join("-")
}
```

**Examples**:

| Description | Slug |
|-------------|------|
| "Add user authentication" | `add-user-authentication` |
| "Improve API performance (200ms p95)" | `improve-api-performance-200ms` |
| "Fix bug: null pointer in parser" | `fix-bug-null-pointer-in` |

---

### PRD Template

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:100-200`

```rust
fn fill_prd_template(spec_id: &str, feature_name: &str, description: &str) -> Result<String> {
    let date = Local::now().format("%Y-%m-%d").to_string();

    Ok(format!(r#"# {feature_name}

**SPEC-ID**: {spec_id}
**Created**: {date}
**Status**: Draft

---

## Background

{description}

## Requirements

### Functional Requirements

- **FR-001**: [Describe first functional requirement]
- **FR-002**: [Describe second functional requirement]

### Non-Functional Requirements

- **NFR-001**: [Performance, scalability, security, etc.]

## Acceptance Criteria

### FR-001
- [ ] [Specific measurable criterion]
- [ ] [Another criterion]

### FR-002
- [ ] [Criterion]

## Constraints

- [Technical constraints]
- [Business constraints]
- [Time/resource constraints]

## Out of Scope

- [Explicitly state what's NOT included]

---

**Next Steps**: Run `/speckit.clarify {spec_id}` to detect ambiguities
"#, feature_name = feature_name, spec_id = spec_id, date = date, description = description))
}
```

---

### SPEC.md Tracker Update

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:250-320`

```rust
fn update_spec_tracker(cwd: &Path, spec_id: &str, feature_name: &str, dir_name: &str) -> Result<()> {
    let spec_md_path = cwd.join("SPEC.md");

    // Read existing SPEC.md
    let content = if spec_md_path.exists() {
        fs::read_to_string(&spec_md_path)?
    } else {
        // Create initial SPEC.md if missing
        String::from("# SPEC Tracker\n\n| SPEC-ID | Feature | Status | Directory |\n|---------|---------|--------|----------|\n")
    };

    // Append new row
    let new_row = format!(
        "| {} | {} | Draft | [docs/{}](docs/{}) |\n",
        spec_id, feature_name, dir_name, dir_name
    );

    let updated = content.trim_end().to_string() + "\n" + &new_row;
    fs::write(&spec_md_path, updated)?;

    Ok(())
}
```

**Example SPEC.md**:

```markdown
# SPEC Tracker

| SPEC-ID | Feature | Status | Directory |
|---------|---------|--------|-----------|
| SPEC-KIT-001 | User Authentication | Complete | [docs/SPEC-KIT-001-user-authentication](docs/SPEC-KIT-001-user-authentication) |
| SPEC-KIT-002 | API Performance | In Progress | [docs/SPEC-KIT-002-api-performance](docs/SPEC-KIT-002-api-performance) |
| SPEC-KIT-003 | Cost Optimization | Draft | [docs/SPEC-KIT-003-cost-optimization](docs/SPEC-KIT-003-cost-optimization) |
```

---

### Usage Example

```bash
# User command
/speckit.new Add dark mode toggle to settings page

# Native execution (<1s)
Generated SPEC-ID: SPEC-KIT-070
Created slug: add-dark-mode-toggle-to
Created directory: docs/SPEC-KIT-070-add-dark-mode-toggle-to/
  ├─ PRD.md (850 bytes, template filled)
  └─ spec.md (1200 bytes, minimal template)
Updated SPEC.md tracker

✅ SPEC-KIT-070 created successfully!

Next steps:
  1. Edit docs/SPEC-KIT-070-add-dark-mode-toggle-to/PRD.md
  2. Run /speckit.clarify SPEC-KIT-070 to detect ambiguities
  3. Run /speckit.auto SPEC-KIT-070 for full pipeline

Cost: $0.00 (saved $0.15 vs 2-agent consensus)
Time: 0.8s (saved 2min 59s)
```

---

## /speckit.clarify - Ambiguity Detection

### Purpose

Detect vague, incomplete, or ambiguous language in PRD using pattern matching.

**Replaced**: 3 agents ($0.80, 10min) → Native ($0, <1s)

**5 Pattern Categories**:
1. **Vague language**: "should", "might", "probably"
2. **Incomplete markers**: "TBD", "TODO", "XXX"
3. **Quantifier ambiguity**: "fast", "scalable" (without metrics)
4. **Scope gaps**: "etc.", "and so on"
5. **Time ambiguity**: "soon", "ASAP"

---

### Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs:54-200`

```rust
struct PatternDetector {
    vague_language: Regex,
    incomplete_markers: Regex,
    quantifier_ambiguity: Regex,
    scope_gaps: Regex,
    time_ambiguity: Regex,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self {
            vague_language: Regex::new(r"(?i)\b(should|might|consider|probably|maybe|could)\b")
                .unwrap(),

            incomplete_markers: Regex::new(r"\b(TBD|TODO|FIXME|XXX|\?\?\?)\b|\[placeholder\]")
                .unwrap(),

            quantifier_ambiguity: Regex::new(
                r"(?i)\b(fast|slow|quick|scalable|responsive|performant|efficient|secure|robust|simple|complex)\b"
            ).unwrap(),

            scope_gaps: Regex::new(r"\b(etc\.|and so on|similar|other|various)\b").unwrap(),

            time_ambiguity: Regex::new(r"(?i)\b(soon|later|eventually|ASAP|when possible)\b")
                .unwrap(),
        }
    }
}
```

---

### Pattern 1: Vague Language

**Triggers**: "should", "might", "consider", "probably", "maybe", "could"

**Severity**: Important

```rust
fn check_vague_language(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
    if let Some(mat) = self.vague_language.find(content) {
        let word = mat.as_str();
        issues.push(Ambiguity {
            id: format!("AMB-{:03}", issues.len() + 1),
            question: format!("What is the specific requirement? '{}' is vague", word),
            location: format!("line {}", line_num),
            severity: Severity::Important,
            pattern: "vague_language".to_string(),
            context: truncate_context(content, 80),
            suggestion: Some(format!(
                "Replace '{}' with measurable criteria (e.g., 'must', 'will', specific metric)",
                word
            )),
        });
    }
}
```

**Example**:

```markdown
# PRD.md (before)
NFR-001: System should be fast

# Ambiguity detected
AMB-001:
  Pattern: vague_language
  Severity: IMPORTANT
  Question: What is the specific requirement? 'should' is vague
  Suggestion: Replace 'should' with 'must' (required) or 'may' (optional)

# Fixed
NFR-001: System must respond within 200ms (p95)
```

---

### Pattern 2: Incomplete Markers

**Triggers**: "TBD", "TODO", "FIXME", "XXX", "???", "[placeholder]"

**Severity**: Critical

```rust
fn check_incomplete_markers(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
    if let Some(mat) = self.incomplete_markers.find(content) {
        let marker = mat.as_str();
        issues.push(Ambiguity {
            id: format!("AMB-{:03}", issues.len() + 1),
            question: format!("Incomplete specification: marker '{}'", marker),
            location: format!("line {}", line_num),
            severity: Severity::Critical,
            pattern: "incomplete_markers".to_string(),
            context: truncate_context(content, 80),
            suggestion: Some("Complete this requirement before implementation".to_string()),
        });
    }
}
```

**Example**:

```markdown
# PRD.md (before)
FR-003: Authentication mechanism - TBD

# Ambiguity detected
AMB-002:
  Pattern: incomplete_markers
  Severity: CRITICAL
  Question: Incomplete specification: marker 'TBD'
  Suggestion: Complete this requirement before implementation

# Fixed
FR-003: Authentication using OAuth 2.0 with JWT tokens
```

---

### Pattern 3: Quantifier Ambiguity

**Triggers**: "fast", "slow", "scalable", "responsive", "secure" (without nearby metrics)

**Severity**: Critical

```rust
fn check_quantifier_ambiguity(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
    if let Some(mat) = self.quantifier_ambiguity.find(content) {
        let word = mat.as_str();

        // Check if metric is nearby (same line)
        if !has_metric_nearby(content, word) {
            issues.push(Ambiguity {
                id: format!("AMB-{:03}", issues.len() + 1),
                question: format!("What is the specific metric for '{}'?", word),
                location: format!("line {}", line_num),
                severity: Severity::Critical,
                pattern: "quantifier_ambiguity".to_string(),
                context: truncate_context(content, 80),
                suggestion: Some(format!("Add specific metric after '{}'", word)),
            });
        }
    }
}

fn has_metric_nearby(line: &str, word: &str) -> bool {
    let patterns = [
        r"\d+", r"<\s*\d+", r">\s*\d+", r"\d+\s*ms", r"\d+\s*MB",
        r"\d+\s*%", r"\d+\s*users", r"\d+\s*requests",
    ];

    patterns.iter().any(|pattern| {
        Regex::new(pattern).unwrap().is_match(line)
    })
}
```

**Example**:

```markdown
# PRD.md (before)
NFR-002: System must be scalable

# Ambiguity detected
AMB-003:
  Pattern: quantifier_ambiguity
  Severity: CRITICAL
  Question: What is the specific metric for 'scalable'?
  Suggestion: Add specific metric after 'scalable'

# Fixed
NFR-002: System must support 10,000 concurrent users (95th percentile)
```

**Not Triggered** (metrics present):

```markdown
# These are OK (metrics nearby)
"System must be fast (<200ms response time)"
"Scalable to 10,000 users"
"Responsive UI (60 FPS)"
```

---

### Pattern 4: Scope Gaps

**Triggers**: "etc.", "and so on", "similar", "other", "various"

**Severity**: Important

```rust
fn check_scope_gaps(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
    if let Some(mat) = self.scope_gaps.find(content) {
        let word = mat.as_str();
        issues.push(Ambiguity {
            id: format!("AMB-{:03}", issues.len() + 1),
            question: format!("Scope unclear: '{}'", word),
            location: format!("line {}", line_num),
            severity: Severity::Important,
            pattern: "scope_gaps".to_string(),
            context: truncate_context(content, 80),
            suggestion: Some("List all items explicitly or define clear boundary".to_string()),
        });
    }
}
```

**Example**:

```markdown
# PRD.md (before)
FR-005: Support authentication via OAuth, SAML, etc.

# Ambiguity detected
AMB-004:
  Pattern: scope_gaps
  Severity: IMPORTANT
  Question: Scope unclear: 'etc.'
  Suggestion: List all items explicitly or define clear boundary

# Fixed
FR-005: Support authentication via OAuth 2.0 and SAML 2.0 only
```

---

### Pattern 5: Time Ambiguity

**Triggers**: "soon", "later", "eventually", "ASAP", "when possible"

**Severity**: Important

```rust
fn check_time_ambiguity(&self, content: &str, line_num: usize, issues: &mut Vec<Ambiguity>) {
    if let Some(mat) = self.time_ambiguity.find(content) {
        let word = mat.as_str();
        issues.push(Ambiguity {
            id: format!("AMB-{:03}", issues.len() + 1),
            question: format!("Time frame unclear: '{}'", word),
            location: format!("line {}", line_num),
            severity: Severity::Important,
            pattern: "time_ambiguity".to_string(),
            context: truncate_context(content, 80),
            suggestion: Some("Specify concrete deadline or milestone".to_string()),
        });
    }
}
```

**Example**:

```markdown
# PRD.md (before)
FR-007: Implement caching soon

# Ambiguity detected
AMB-005:
  Pattern: time_ambiguity
  Severity: IMPORTANT
  Question: Time frame unclear: 'soon'
  Suggestion: Specify concrete deadline or milestone

# Fixed
FR-007: Implement caching in Phase 2 (Sprint 3)
```

---

### Output Format

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs:250-300`

```rust
pub struct AmbiguityReport {
    pub spec_id: String,
    pub total_count: usize,
    pub critical_count: usize,
    pub important_count: usize,
    pub minor_count: usize,
    pub ambiguities: Vec<Ambiguity>,
}

impl AmbiguityReport {
    pub fn summary(&self) -> String {
        format!(
            "{} ambiguities: {} CRITICAL, {} IMPORTANT, {} MINOR",
            self.total_count, self.critical_count, self.important_count, self.minor_count
        )
    }

    pub fn is_clean(&self) -> bool {
        self.critical_count == 0 && self.important_count <= 2
    }
}
```

---

### Usage Example

```bash
# User command
/speckit.clarify SPEC-KIT-070

# Native execution (<1s)
Scanning docs/SPEC-KIT-070-dark-mode-toggle/PRD.md...

Found 5 ambiguities:

AMB-001 [CRITICAL] line 12
  Pattern: quantifier_ambiguity
  Text: "System must be performant"
  Question: What is the specific metric for 'performant'?
  Suggestion: Add specific metric after 'performant'

AMB-002 [CRITICAL] line 18
  Pattern: incomplete_markers
  Text: "Authentication method: TBD"
  Question: Incomplete specification: marker 'TBD'
  Suggestion: Complete this requirement before implementation

AMB-003 [IMPORTANT] line 25
  Pattern: vague_language
  Text: "UI should be intuitive"
  Question: What is the specific requirement? 'should' is vague
  Suggestion: Replace 'should' with 'must' (required) or 'may' (optional)

AMB-004 [IMPORTANT] line 34
  Pattern: scope_gaps
  Text: "Support various color schemes"
  Question: Scope unclear: 'various'
  Suggestion: List all items explicitly or define clear boundary

AMB-005 [IMPORTANT] line 41
  Pattern: time_ambiguity
  Text: "Implement caching soon"
  Question: Time frame unclear: 'soon'
  Suggestion: Specify concrete deadline or milestone

Summary: 5 ambiguities: 2 CRITICAL, 3 IMPORTANT, 0 MINOR

❌ Quality gate: FAIL (≤2 critical required, found 2)

Recommendation: Fix critical ambiguities before running /speckit.plan

Cost: $0.00 (saved $0.80 vs 3-agent consensus)
Time: 0.6s (saved 9min 59s)
```

---

## /speckit.analyze - Consistency Checking

### Purpose

Cross-artifact consistency validation using structural diff.

**Replaced**: 3 agents ($0.35, 8min) → Native ($0, <1s)

**6 Check Categories**:
1. **ID consistency**: Referenced IDs exist in source docs
2. **Requirement coverage**: All PRD requirements addressed
3. **Contradiction detection**: Conflicting statements
4. **Version drift**: File modification time anomalies
5. **Orphan tasks**: Tasks without PRD backing
6. **Scope creep**: Plan features not in PRD

---

### Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/analyze_native.rs:15-400`

```rust
pub fn check_consistency(
    spec_id: &str,
    cwd: &Path,
) -> Result<Vec<InconsistencyIssue>> {
    let spec_dir = find_spec_directory(cwd, spec_id)?;

    // Load artifacts
    let prd = load_artifact(&spec_dir, "PRD.md")?;
    let plan = load_artifact_optional(&spec_dir, "plan.md");
    let tasks = load_artifact_optional(&spec_dir, "tasks.md");

    let mut issues = Vec::new();

    // Check 1: ID consistency
    if let Some(plan_content) = &plan {
        issues.extend(check_id_consistency(&prd, plan_content)?);
    }

    if let Some(tasks_content) = &tasks {
        issues.extend(check_id_consistency(&prd, tasks_content)?);
    }

    // Check 2: Requirement coverage
    if let Some(plan_content) = &plan {
        issues.extend(check_requirement_coverage(&prd, plan_content)?);
    }

    // Check 3: Contradictions
    if let Some(plan_content) = &plan {
        issues.extend(detect_contradictions(&prd, plan_content)?);
    }

    // Check 4: Version drift
    issues.extend(check_version_drift(&spec_dir)?);

    // Check 5: Orphan tasks
    if let Some(tasks_content) = &tasks {
        issues.extend(find_orphan_tasks(&prd, tasks_content)?);
    }

    // Check 6: Scope creep
    if let Some(plan_content) = &plan {
        issues.extend(detect_scope_creep(&prd, plan_content)?);
    }

    Ok(issues)
}
```

---

### Check 1: ID Consistency

**Purpose**: Ensure FR-001, NFR-002, etc. references exist

```rust
fn check_id_consistency(prd: &str, doc: &str) -> Result<Vec<InconsistencyIssue>> {
    let mut issues = Vec::new();

    // Extract all requirement IDs from PRD
    let prd_ids = extract_requirement_ids(prd);

    // Find references in document
    let referenced_ids = extract_referenced_ids(doc);

    for referenced in referenced_ids {
        if !prd_ids.contains(&referenced) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                type_: IssueType::IdConsistency,
                severity: Severity::Critical,
                description: format!(
                    "References {}, but PRD only defines {:?}",
                    referenced,
                    prd_ids.iter().collect::<Vec<_>>()
                ),
                locations: vec![find_location(doc, &referenced)],
                fix: format!("Either add {} to PRD or remove reference", referenced),
            });
        }
    }

    Ok(issues)
}

fn extract_requirement_ids(content: &str) -> HashSet<String> {
    let re = Regex::new(r"(FR|NFR)-\d+").unwrap();
    re.find_iter(content)
        .map(|m| m.as_str().to_string())
        .collect()
}
```

**Example**:

```markdown
# PRD.md
- FR-001: User login
- FR-002: User logout
- NFR-001: 200ms response time

# plan.md (WRONG)
FR-003 will be implemented in Phase 2

# Issue detected
INC-001:
  Type: IdConsistency
  Severity: CRITICAL
  Description: References FR-003, but PRD only defines ["FR-001", "FR-002", "NFR-001"]
  Fix: Either add FR-003 to PRD or remove reference
```

---

### Check 2: Requirement Coverage

**Purpose**: Ensure all PRD requirements addressed in plan

```rust
fn check_requirement_coverage(prd: &str, plan: &str) -> Result<Vec<InconsistencyIssue>> {
    let mut issues = Vec::new();

    let prd_ids = extract_requirement_ids(prd);
    let plan_ids = extract_referenced_ids(plan);

    for prd_id in prd_ids {
        if !plan_ids.contains(&prd_id) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                type_: IssueType::RequirementCoverage,
                severity: Severity::Critical,
                description: format!("{} in PRD but not addressed in plan", prd_id),
                locations: vec![find_location(prd, &prd_id)],
                fix: format!("Add {} to plan's Work Breakdown", prd_id),
            });
        }
    }

    Ok(issues)
}
```

**Example**:

```markdown
# PRD.md
- FR-001: Login
- FR-002: Logout
- FR-003: Password reset

# plan.md (missing FR-003)
## Work Breakdown
1. Implement FR-001 (login flow)
2. Implement FR-002 (logout)

# Issue detected
INC-002:
  Type: RequirementCoverage
  Severity: CRITICAL
  Description: FR-003 in PRD but not addressed in plan
  Fix: Add FR-003 to plan's Work Breakdown
```

---

### Check 3: Contradiction Detection

**Purpose**: Find conflicting architectural decisions

```rust
fn detect_contradictions(prd: &str, plan: &str) -> Result<Vec<InconsistencyIssue>> {
    let mut issues = Vec::new();

    // Architecture contradictions
    let arch_pairs = [
        ("monolithic", "microservices"),
        ("REST", "GraphQL"),
        ("SQL", "NoSQL"),
        ("synchronous", "asynchronous"),
        ("stateful", "stateless"),
    ];

    for (term_a, term_b) in &arch_pairs {
        if prd.to_lowercase().contains(term_a) && plan.to_lowercase().contains(term_b) {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                type_: IssueType::Contradiction,
                severity: Severity::Important,
                description: format!("PRD mentions '{}', plan mentions '{}'", term_a, term_b),
                locations: vec![
                    format!("PRD: {}", find_location(prd, term_a)),
                    format!("plan: {}", find_location(plan, term_b)),
                ],
                fix: "Align on single architectural approach".to_string(),
            });
        }
    }

    Ok(issues)
}
```

**Example**:

```markdown
# PRD.md
NFR-004: Use REST API for all endpoints

# plan.md
Implement GraphQL resolvers for data fetching

# Issue detected
INC-003:
  Type: Contradiction
  Severity: IMPORTANT
  Description: PRD mentions 'REST', plan mentions 'GraphQL'
  Locations: ["PRD: line 45", "plan: line 89"]
  Fix: Align on single architectural approach
```

---

### Check 4: Version Drift

**Purpose**: Detect PRD modified after plan/tasks created

```rust
fn check_version_drift(spec_dir: &Path) -> Result<Vec<InconsistencyIssue>> {
    let mut issues = Vec::new();

    let prd_path = spec_dir.join("PRD.md");
    let plan_path = spec_dir.join("plan.md");
    let tasks_path = spec_dir.join("tasks.md");

    if !plan_path.exists() {
        return Ok(issues);  // No plan yet, no drift possible
    }

    let prd_modified = get_modified_time(&prd_path)?;
    let plan_modified = get_modified_time(&plan_path)?;

    if prd_modified > plan_modified {
        issues.push(InconsistencyIssue {
            id: format!("INC-{:03}", issues.len() + 1),
            type_: IssueType::VersionDrift,
            severity: Severity::Important,
            description: format!(
                "PRD modified {} after plan created {}",
                format_time(prd_modified),
                format_time(plan_modified)
            ),
            locations: vec!["PRD.md".to_string(), "plan.md".to_string()],
            fix: "Re-run /speckit.plan to sync with updated PRD".to_string(),
        });
    }

    // Similar check for tasks.md
    if tasks_path.exists() {
        let tasks_modified = get_modified_time(&tasks_path)?;
        if plan_modified > tasks_modified {
            issues.push(InconsistencyIssue {
                id: format!("INC-{:03}", issues.len() + 1),
                type_: IssueType::VersionDrift,
                severity: Severity::Important,
                description: format!(
                    "plan modified {} after tasks created {}",
                    format_time(plan_modified),
                    format_time(tasks_modified)
                ),
                locations: vec!["plan.md".to_string(), "tasks.md".to_string()],
                fix: "Re-run /speckit.tasks to sync with updated plan".to_string(),
            });
        }
    }

    Ok(issues)
}
```

**Example**:

```
PRD.md modified: 2025-10-18 15:30:00
plan.md created: 2025-10-18 14:00:00

INC-004:
  Type: VersionDrift
  Severity: IMPORTANT
  Description: PRD modified 2025-10-18 15:30 after plan created 2025-10-18 14:00
  Fix: Re-run /speckit.plan to sync with updated PRD
```

---

### Usage Example

```bash
# User command
/speckit.analyze SPEC-KIT-070

# Native execution (<1s)
Checking consistency for SPEC-KIT-070...

Found 3 issues:

INC-001 [CRITICAL] ID Consistency
  Description: plan.md references FR-005, but PRD only defines ["FR-001", "FR-002", "FR-003", "FR-004"]
  Locations: plan.md:89
  Fix: Either add FR-005 to PRD or remove reference

INC-002 [IMPORTANT] Contradiction
  Description: PRD mentions 'REST', plan mentions 'GraphQL'
  Locations: ["PRD: line 45", "plan: line 123"]
  Fix: Align on single architectural approach

INC-003 [IMPORTANT] Version Drift
  Description: PRD modified 2025-10-18 15:30 after plan created 2025-10-18 14:00
  Fix: Re-run /speckit.plan to sync with updated PRD

Summary: 3 issues: 1 CRITICAL, 2 IMPORTANT, 0 MINOR

❌ Quality gate: FAIL (0 critical required, found 1)

Recommendation: Fix critical issues before running /speckit.implement

Cost: $0.00 (saved $0.35 vs 3-agent consensus)
Time: 0.9s (saved 7min 59s)
```

---

## /speckit.checklist - Quality Scoring

### Purpose

Rubric-based quality evaluation (0-100 score).

**Replaced**: 3 agents ($0.35, 8min) → Native ($0, <1s)

**4 Rubric Categories** (100 points total):
1. **Completeness** (30%): Required sections present
2. **Clarity** (20%): Specific metrics, no vague language
3. **Testability** (30%): Measurable acceptance criteria
4. **Consistency** (20%): Cross-artifact alignment

---

### Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/checklist_native.rs:62-150`

```rust
pub fn score_quality(spec_id: &str, cwd: &Path) -> Result<QualityReport> {
    let spec_dir = find_spec_directory(cwd, spec_id)?;
    let mut issues = Vec::new();

    // Load PRD
    let prd_path = spec_dir.join("PRD.md");
    let prd_content = fs::read_to_string(&prd_path)?;

    // Score each dimension
    let completeness = score_completeness(&prd_content, &mut issues);
    let clarity = score_clarity(&prd_content, &mut issues);
    let testability = score_testability(&prd_content, &mut issues);
    let consistency = score_consistency(spec_id, cwd, &mut issues)?;

    // Overall score (weighted average)
    let overall_score =
        (completeness * 0.3) + (clarity * 0.2) + (testability * 0.3) + (consistency * 0.2);

    Ok(QualityReport {
        spec_id: spec_id.to_string(),
        overall_score,
        completeness,
        clarity,
        testability,
        consistency,
        issues,
        recommendations: generate_recommendations(completeness, clarity, testability, consistency),
    })
}
```

---

### Completeness Scoring (30 points)

```rust
fn score_completeness(prd: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let required_sections = [
        ("Background", 5.0),
        ("Requirements", 10.0),
        ("Functional Requirements", 5.0),
        ("Non-Functional Requirements", 3.0),
        ("Acceptance Criteria", 7.0),
    ];

    let mut score = 0.0;

    for (section, points) in &required_sections {
        if prd.contains(section) {
            score += points;
        } else {
            issues.push(QualityIssue {
                id: format!("CHK-{:03}", issues.len() + 1),
                category: "completeness".to_string(),
                severity: Severity::Important,
                description: format!("Missing required section: {}", section),
                impact: format!("-{:.1} points", points),
                suggestion: format!("Add '{}' section to PRD", section),
            });
        }
    }

    // Convert to 0-100 scale (30 max → 100%)
    (score / 30.0) * 100.0
}
```

---

### Clarity Scoring (20 points)

```rust
fn score_clarity(prd: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 100.0;

    // Deduct for vague language
    let vague_count = count_vague_language(prd);
    let vague_deduction = (vague_count as f32).min(50.0);
    score -= vague_deduction;

    if vague_count > 0 {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Important,
            description: format!("Found {} instances of vague language", vague_count),
            impact: format!("-{:.1} points", vague_deduction),
            suggestion: "Replace vague terms with specific metrics".to_string(),
        });
    }

    // Deduct for missing metrics on quantifiers
    let unquantified_count = count_unquantified_terms(prd);
    let metric_deduction = (unquantified_count as f32 * 10.0).min(50.0);
    score -= metric_deduction;

    if unquantified_count > 0 {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "clarity".to_string(),
            severity: Severity::Critical,
            description: format!("{} quantifiers without metrics", unquantified_count),
            impact: format!("-{:.1} points", metric_deduction),
            suggestion: "Add specific metrics to 'fast', 'scalable', etc.".to_string(),
        });
    }

    score.max(0.0)
}
```

---

### Testability Scoring (30 points)

```rust
fn score_testability(prd: &str, issues: &mut Vec<QualityIssue>) -> f32 {
    let mut score = 100.0;

    // Extract requirements
    let requirements = extract_requirement_ids(prd);

    // Check acceptance criteria coverage
    let ac_section = extract_section(prd, "Acceptance Criteria");
    let ac_count = if let Some(ac) = ac_section {
        requirements.iter().filter(|req| ac.contains(*req)).count()
    } else {
        0
    };

    let coverage_ratio = ac_count as f32 / requirements.len().max(1) as f32;
    let coverage_deduction = (1.0 - coverage_ratio) * 50.0;
    score -= coverage_deduction;

    if coverage_ratio < 1.0 {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "testability".to_string(),
            severity: Severity::Important,
            description: format!(
                "Acceptance criteria covers {} of {} requirements ({:.0}%)",
                ac_count,
                requirements.len(),
                coverage_ratio * 100.0
            ),
            impact: format!("-{:.1} points", coverage_deduction),
            suggestion: "Add acceptance criteria for all requirements".to_string(),
        });
    }

    score.max(0.0)
}
```

---

### Consistency Scoring (20 points)

```rust
fn score_consistency(spec_id: &str, cwd: &Path, issues: &mut Vec<QualityIssue>) -> Result<f32> {
    // Reuse analyze_native for consistency checks
    let consistency_issues = super::analyze_native::check_consistency(spec_id, cwd)?;

    let mut score = 100.0;

    let critical_count = consistency_issues.iter().filter(|i| i.severity == Severity::Critical).count();
    let important_count = consistency_issues.iter().filter(|i| i.severity == Severity::Important).count();

    score -= critical_count as f32 * 20.0;  // -20 per critical
    score -= important_count as f32 * 10.0; // -10 per important

    if critical_count > 0 || important_count > 0 {
        issues.push(QualityIssue {
            id: format!("CHK-{:03}", issues.len() + 1),
            category: "consistency".to_string(),
            severity: Severity::Important,
            description: format!(
                "{} consistency issues ({} critical, {} important)",
                consistency_issues.len(),
                critical_count,
                important_count
            ),
            impact: format!("-{:.1} points", (critical_count * 20 + important_count * 10) as f32),
            suggestion: "Run /speckit.analyze for details".to_string(),
        });
    }

    Ok(score.max(0.0))
}
```

---

### Usage Example

```bash
# User command
/speckit.checklist SPEC-KIT-070

# Native execution (<1s)
Scoring quality for SPEC-KIT-070...

Overall: 82.0% (B)
  Completeness: 90.0%
  Clarity: 65.0%
  Testability: 85.0%
  Consistency: 80.0%

Issues:

CHK-001 [IMPORTANT] completeness
  Description: Missing required section: Non-Functional Requirements
  Impact: -3.0 points
  Suggestion: Add 'Non-Functional Requirements' section to PRD

CHK-002 [CRITICAL] clarity
  Description: 3 quantifiers without metrics
  Impact: -30.0 points
  Suggestion: Add specific metrics to 'fast', 'scalable', etc.

CHK-003 [IMPORTANT] testability
  Description: Acceptance criteria covers 3 of 4 requirements (75%)
  Impact: -12.5 points
  Suggestion: Add acceptance criteria for all requirements

CHK-004 [IMPORTANT] consistency
  Description: 1 consistency issues (0 critical, 1 important)
  Impact: -10.0 points
  Suggestion: Run /speckit.analyze for details

Recommendations:
  - Remove vague language and add specific metrics
  - Add measurable acceptance criteria for all requirements

✅ Quality gate: PASS (≥80 required, scored 82)

Cost: $0.00 (saved $0.35 vs 3-agent consensus)
Time: 0.8s (saved 7min 59s)
```

---

## /speckit.status - Status Dashboard

### Purpose

Display current state of SPEC pipeline (native TUI dashboard).

**No Agent Equivalent**: New feature (Tier 0)

**Information Displayed**:
- Current stage and phase
- Completed stages (✅)
- Artifacts created
- Quality gate results
- Cost summary
- Time elapsed

---

### Implementation

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/status_native.rs:15-200`

```rust
pub fn render_status(spec_id: &str, cwd: &Path) -> Result<StatusDashboard> {
    let spec_dir = find_spec_directory(cwd, spec_id)?;

    // Scan artifacts
    let artifacts = scan_artifacts(&spec_dir)?;

    // Check quality gate results
    let quality_gates = scan_quality_gate_results(spec_id, cwd)?;

    // Determine current stage
    let current_stage = infer_current_stage(&artifacts);

    Ok(StatusDashboard {
        spec_id: spec_id.to_string(),
        current_stage,
        completed_stages: artifacts.keys().cloned().collect(),
        artifacts,
        quality_gates,
        total_cost: calculate_total_cost(&artifacts)?,
        total_time: calculate_total_time(&artifacts)?,
    })
}
```

---

### Output Format

```bash
# User command
/speckit.status SPEC-KIT-070

# Native execution (<1s)
┌──────────────────────────────────────────────────────────┐
│ SPEC-KIT-070: Dark Mode Toggle                          │
├──────────────────────────────────────────────────────────┤
│ Status: In Progress (Implement stage)                   │
│ Progress: 3 of 6 stages complete (50%)                  │
└──────────────────────────────────────────────────────────┘

Pipeline:
  ✅ Plan       (completed, 10min, $0.35)
  ✅ Tasks      (completed, 3min, $0.10)
  🔄 Implement  (in progress, started 5min ago)
  ⏳ Validate   (pending)
  ⏳ Audit      (pending)
  ⏳ Unlock     (pending)

Artifacts:
  ✅ PRD.md               (created)
  ✅ plan.md              (created, 2.5 KB)
  ✅ tasks.md             (created, 1.8 KB)
  🔄 src/ui/dark_mode.rs  (in progress)

Quality Gates:
  ✅ BeforeSpecify (Clarify)   - PASS (0 critical, 2 important)
  ✅ AfterSpecify (Checklist)  - PASS (score: 95/100, grade: A)
  ⏳ AfterTasks (Analyze)      - pending

Cost Summary:
  Stages: $0.45 ($0.35 plan + $0.10 tasks)
  Quality Gates: $0.10 (GPT-5 validations)
  Total: $0.55 (estimated final: ~$2.70)

Time Elapsed: 13min (estimated completion: 32min remaining)

Next Action: Wait for implement stage to complete, then /speckit.validate

Cost: $0.00 (instant)
Time: 0.5s
```

---

## Performance Summary

### Native vs Agent Comparison

| Operation | Native | Agent-Based | Time Saved | Cost Saved |
|-----------|--------|-------------|------------|------------|
| `/speckit.new` | <1s, $0 | 3min, $0.15 | 2min 59s | $0.15 |
| `/speckit.clarify` | <1s, $0 | 10min, $0.80 | 9min 59s | $0.80 |
| `/speckit.analyze` | <1s, $0 | 8min, $0.35 | 7min 59s | $0.35 |
| `/speckit.checklist` | <1s, $0 | 8min, $0.35 | 7min 59s | $0.35 |
| **Total** | **<4s, $0** | **29min, $1.65** | **28min 56s** | **$1.65** |

**Per /speckit.auto Pipeline**:
- **Before**: $11 (all agent-based)
- **After**: $2.70 (with native operations)
- **Savings**: $8.30 (75% reduction)

---

## Summary

**Native Operations Highlights**:

1. **Tier 0: FREE**: Zero agents, $0 cost, <1s execution time
2. **5 Commands**: new, clarify, analyze, checklist, status
3. **Pattern Matching**: Deterministic, no AI reasoning required
4. **Massive Savings**: $1.65 per pipeline, 28min 56s time saved
5. **Quality Assurance**: Ambiguity detection, consistency checks, quality scoring
6. **Offline Capable**: No network required (pure file operations)
7. **Philosophy**: "Agents for reasoning, NOT transactions"

**Next Steps**:
- [Evidence Repository](evidence-repository.md) - Artifact storage system
- [Cost Tracking](cost-tracking.md) - Per-stage cost breakdown
- [Agent Orchestration](agent-orchestration.md) - Multi-agent coordination

---

**File References**:
- SPEC creation: `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs:37-97`
- Clarify: `codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs:54-200`
- Analyze: `codex-rs/tui/src/chatwidget/spec_kit/analyze_native.rs:15-400`
- Checklist: `codex-rs/tui/src/chatwidget/spec_kit/checklist_native.rs:62-150`
- Status: `codex-rs/tui/src/chatwidget/spec_kit/status_native.rs:15-200`
