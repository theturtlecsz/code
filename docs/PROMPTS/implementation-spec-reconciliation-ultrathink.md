# PROMPT: Implementation SPEC Reconciliation & Unified Execution Planning

**Purpose**: Critically assess all implementation specs created in the last 7 days, identify gaps/contradictions/ambiguities, reconcile design decisions, and synthesize into a single coherent execution plan with dependency-aware sequencing.

**Context**: Multiple implementation specs may have been created in parallel or sequence, potentially containing inconsistent assumptions, duplicated work, or missing integration details. This prompt performs forensic analysis to ensure implementation readiness.

**Output**: Unified implementation plan with resolved ambiguities, rationalized design decisions, and execution-ready task breakdown.

---

## Phase 1: Discovery & Inventory (15-20 minutes)

### 1.1 Find All Recent Implementation Specs

**Search Criteria**:
- Modified in last 7 days: `find docs/ -name "*implementation*.md" -mtime -7`
- File patterns: `*-IMPL*.md`, `*implementation-plan.md`, `*-impl.md`
- Directories: `docs/SPEC-*/`, `docs/spec-kit/`

**Collect for Each Found SPEC**:
```bash
# Find all implementation specs modified in last 7 days
find docs/ -name "*implementation*.md" -mtime -7 -type f

# Alternative: Search by content (if file naming inconsistent)
grep -r "Implementation Plan" docs/ --include="*.md" -l | while read file; do
    stat -c "%y %n" "$file" | grep "2025-11-"  # Adjust date range
done
```

**Extract Metadata**:
- SPEC ID (e.g., SPEC-949-IMPL)
- Title
- Estimated effort (hours)
- Dependencies (other SPECs, infrastructure)
- Priority (P0/P1/P2)
- Created date
- File path

**Output**: Inventory table with all discovered specs

---

### 1.2 Query Local-Memory for Recent Planning Sessions

Use `mcp__local-memory__search`:
```json
{
  "query": "implementation plan SPEC integration analysis",
  "search_type": "semantic",
  "use_ai": true,
  "limit": 10,
  "start_date": "2025-11-09",
  "end_date": "2025-11-16",
  "tags": ["type:milestone", "planning", "integration-analysis"],
  "response_format": "concise"
}
```

**Purpose**: Retrieve planning context, design decisions, architectural discussions from recent sessions.

**Extract**:
- Key architectural decisions made
- Design trade-offs discussed
- Integration patterns identified
- Risks flagged
- Assumptions documented

---

### 1.3 Build SPEC Relationship Graph

**For Each Implementation SPEC Found**:

Extract dependency information:
- **Depends On**: Which SPECs/infrastructure must complete first?
- **Enables**: Which SPECs are unblocked by this completion?
- **Conflicts With**: Which SPECs modify the same files?
- **Shares Components With**: Which SPECs share code/data structures?

**Create Dependency Matrix**:
```
       │ SPEC-A │ SPEC-B │ SPEC-C │ SPEC-D
───────┼────────┼────────┼────────┼────────
SPEC-A │   -    │ enables│   -    │ blocks
SPEC-B │ depends│   -    │ shares │   -
SPEC-C │   -    │ shares │   -    │ enables
SPEC-D │ blocks │   -    │ depends│   -
```

**Identify**:
- **Critical path**: Longest dependency chain
- **Parallel opportunities**: Independent SPECs that can execute concurrently
- **Circular dependencies**: SPECs that depend on each other (RED FLAG)
- **Bottlenecks**: Single SPEC blocking multiple others

---

## Phase 2: Critical Analysis (30-45 minutes)

### 2.1 Completeness Audit

**For Each Implementation SPEC, Check**:

**❓ Question Resolution**:
- [ ] Are all technical questions from research SPEC answered?
- [ ] Are implementation approaches clearly specified (not "TBD" or "investigate")?
- [ ] Are file paths exact and validated (not "likely" or "somewhere in")?
- [ ] Are LOC estimates justified (not arbitrary round numbers)?

**📋 Task Breakdown Quality**:
- [ ] Is every task actionable (clear "what to do")?
- [ ] Does every task specify exact file and line ranges?
- [ ] Are tasks atomic (completable in <4 hours)?
- [ ] Are task dependencies explicit?
- [ ] Are "modify file" tasks specific about what changes?

**🧪 Test Coverage Adequacy**:
- [ ] Are test counts justified (not arbitrary)?
- [ ] Are test scenarios specific (not "add tests for X")?
- [ ] Do tests cover happy path + error cases + edge cases?
- [ ] Are integration tests specified for cross-module interactions?
- [ ] Are performance tests included for measurable claims?

**⚠️ Risk Assessment**:
- [ ] Are all identified risks from research SPEC addressed?
- [ ] Does each risk have specific mitigation (not generic "test well")?
- [ ] Are rollback procedures tested/validated?
- [ ] Are severity × probability ratings justified?

**📅 Timeline Realism**:
- [ ] Are phase estimates based on similar past work?
- [ ] Is there buffer time for unknowns (recommended: +20%)?
- [ ] Are dependencies sequenced correctly in timeline?
- [ ] Are validation checkpoints after each phase?

---

### 2.2 Consistency Cross-Check

**Across All Implementation SPECs, Verify**:

**Shared File Conflicts**:
```bash
# Extract all "Modified Files" from implementation specs
grep -A 20 "Modified Files" docs/SPEC-*-IMPL*/implementation-plan.md

# Check for same file modified by multiple SPECs
# Flag conflicts: If SPEC-A modifies handler.rs lines 100-150
#                 and SPEC-B modifies handler.rs lines 120-180
#                 → MERGE CONFLICT RISK
```

**Questions to Answer**:
- ❓ Do multiple SPECs modify the same files?
  - If YES: Are changes in different sections (safe) or overlapping (conflict)?
  - If overlapping: Which SPEC should execute first? Can changes be merged?

- ❓ Do multiple SPECs create the same file?
  - If YES: RED FLAG - duplicate work or naming collision
  - Resolution needed: Rename one, or consolidate into single SPEC

- ❓ Are cost estimates consistent across SPECs?
  - Example: If SPEC-A says gpt5_1 costs $0.10, does SPEC-B say same?
  - If inconsistent: Reconcile to single source of truth

- ❓ Are test count totals consistent?
  - Example: If SPEC-A claims "604 existing tests", does SPEC-B agree?
  - If inconsistent: Which is correct? Update all to match reality

**Data Structure Compatibility**:
- ❓ If SPEC-A creates `struct Foo`, does SPEC-B use compatible definition?
- ❓ Are enum variants consistent across specs (e.g., StageType values)?
- ❓ Are validation rules consistent (same dependency graph)?

---

### 2.3 Design Clarity Assessment

**For Each Implementation SPEC, Flag**:

**Convoluted Design** (Signs of Over-Engineering):
- 🚩 More than 5 new files created for simple feature
- 🚩 Abstraction layers without clear need (interface for single impl)
- 🚩 >500 LOC for feature with narrow scope
- 🚩 Generic infrastructure for single use case
- 🚩 Configuration options without use cases

**Missing Clarifications**:
- 🚩 "Modify file.rs" without specifying what/where/why
- 🚩 "Integrate with existing code" without exact integration points
- 🚩 "Add validation" without specifying validation rules
- 🚩 "Update config" without showing exact TOML structure
- 🚩 "Handle errors" without error case enumeration

**Ambiguous Implementation**:
- 🚩 Multiple valid approaches mentioned, no clear choice
- 🚩 "Investigate" or "explore" in implementation plan (should be in research)
- 🚩 "Likely" or "probably" for critical design decisions
- 🚩 Conditional logic without clear conditions ("if needed", "possibly")

**For Each Flag, Document**:
- Location: SPEC-ID, Phase X, Task Y
- Issue: Specific problem (be precise)
- Impact: What breaks if this isn't resolved?
- Resolution needed: What question needs answering?

---

### 2.4 Dependency Validation

**Check Dependency Claims**:

For each "Depends On: SPEC-X" claim:
- ❓ Read SPEC-X: Is it actually complete? (Status = Done or In Progress?)
- ❓ What exactly is needed from SPEC-X? (API, data structure, infrastructure?)
- ❓ Is that component actually delivered by SPEC-X? (check deliverables)
- ❓ If SPEC-X incomplete, can implementation proceed with stubs/mocks?

**Validate Infrastructure Dependencies**:
- ❓ Claim: "Requires ProviderRegistry from SPEC-936 (95% complete)"
  - Verification: Read `async_agent_executor.rs`, confirm ProviderRegistry exists
  - Check: Is the 95% estimate accurate? What's the missing 5%?
  - Impact: Does missing 5% block this SPEC?

- ❓ Claim: "Requires config infrastructure (exists: config_types.rs)"
  - Verification: Read config_types.rs, confirm expected structs exist
  - Check: Are required traits/methods actually present?

**Flag Circular Dependencies**:
- 🔴 CRITICAL: SPEC-A depends on SPEC-B, SPEC-B depends on SPEC-A
- Resolution: Break cycle by extracting shared component or re-sequencing

---

### 2.5 Effort Estimation Validation

**Check Effort Estimates Against Benchmarks**:

**Historical Comparison** (from SPEC.md completed tasks):
- Similar complexity SPECs: What was actual effort vs estimated?
- Pattern: SPEC-938 estimated 8-10h, actual 4h (50% under)
- Pattern: SPEC-941 estimated 8-10h, actual 6h (40% under)
- Pattern: Test coverage Phase 2-4: accelerated significantly (4 months ahead)

**Sanity Checks**:
- ❓ Is 24-32 hours realistic for 800-1,050 LOC TUI widgets?
  - Benchmark: quality_gate_modal.rs = 304 LOC (effort unknown)
  - Calculation: 800 LOC ÷ 304 LOC = 2.6× complexity → 24-32h reasonable IF modal took ~10h
  - Need: Historical data on TUI widget development time

- ❓ Is 16-24 hours realistic for adding 5 models + 2 stubs?
  - Breakdown: Phase 1 (4-6h) + Phase 2 (6-8h) + Phase 3 (4-6h) + Phase 4 (2-4h) = 16-24h
  - Check: Is Phase 2 (agent config updates) really 6-8 hours for ~60 LOC TOML?
  - Possible over-estimate: Phase 2 might be 2-4h (config is straightforward)

**For Each Estimate**:
- ✅ Justified (has similar historical benchmark)
- ⚠️ Possibly high (recommend reduction)
- ⚠️ Possibly low (recommend increase or flag as risk)

---

## Phase 3: Reconciliation (45-60 minutes)

### 3.1 Resolve Inconsistencies

**For Each Inconsistency Found in Phase 2.2**:

**Document**:
```markdown
### Inconsistency #1: [Title]

**Location**:
- SPEC-A: [file:line]
- SPEC-B: [file:line]

**Issue**:
[Describe contradiction precisely]

**Impact**:
[What breaks if not resolved?]

**Analysis**:
- Option 1: [Approach] - Pros: [...], Cons: [...]
- Option 2: [Approach] - Pros: [...], Cons: [...]

**Resolution** (Recommended):
[Clear decision with rationale]

**Action Required**:
1. Update SPEC-A implementation-plan.md [specific change]
2. Update SPEC-B implementation-plan.md [specific change]
3. Create integration test to validate resolution
```

**Example Inconsistencies to Look For**:

1. **File Ownership Conflict**:
   - SPEC-948 creates `pipeline_config.rs` in Phase 1
   - SPEC-947 also lists `pipeline_config.rs` as "new file"
   - Resolution: SPEC-947 Phase 1 should be "Reuse SPEC-948 pipeline_config.rs (0h)"

2. **Test Count Mismatch**:
   - SPEC-A says "604 existing tests"
   - SPEC-B says "555 existing tests"
   - Resolution: Check actual count with `cargo test --list | wc -l`, update all SPECs

3. **Cost Estimate Divergence**:
   - SPEC-949 estimates /speckit.auto at $2.36 (post-GPT-5)
   - SPEC-948 uses $2.71 as baseline (pre-GPT-5)
   - Resolution: Establish single timeline - does SPEC-948 execute before or after SPEC-949?

4. **Dependency Sequence Contradiction**:
   - SPEC-A says "Depends: SPEC-B (complete before starting)"
   - SPEC-B says "Depends: SPEC-A Phase 2"
   - Resolution: Circular dependency - break cycle or re-sequence

---

### 3.2 Clarify Ambiguous Design Decisions

**Scan All Specs for Ambiguous Language**:

Pattern matching for ambiguity markers:
```bash
grep -i "likely\|probably\|maybe\|could\|might\|investigate\|TBD\|TODO" \
  docs/SPEC-*-IMPL*/implementation-plan.md
```

**For Each Match**:

**Document**:
```markdown
### Ambiguity #1: [Description]

**Location**: SPEC-ID, Phase X, Task Y, Line Z

**Ambiguous Text**:
"[Quote exact text]"

**Questions to Resolve**:
1. [Specific question that needs answering]
2. [Alternative approach unclear?]
3. [Implementation detail missing?]

**Resolution Options**:
- **Option A**: [Specific approach] - Effort: Xh, Risk: Low/Med/High
- **Option B**: [Alternative] - Effort: Yh, Risk: Low/Med/High

**Recommended Resolution**:
[Clear decision with rationale]

**Updated Task Text**:
"[Rewritten task with clarity]"
```

**Common Ambiguity Patterns**:

1. **Vague File References**:
   - ❌ "Modify handler.rs or router.rs (depending on subagent config location)"
   - ✅ "Modify handler.rs:250-280 (subagent_commands definition confirmed at line 267)"
   - Action: Grep codebase, find exact location, specify line range

2. **Conditional Implementation**:
   - ❌ "Add confirmation dialog (optional enhancement)"
   - ✅ "Add confirmation dialog IF warnings present (6-8h effort) OR defer to SPEC-950 (record as future enhancement)"
   - Action: Make decision now - include or defer, not "optional"

3. **Unspecified Validation**:
   - ❌ "Validate configuration"
   - ✅ "Validate: (1) All required fields present, (2) enabled_stages non-empty, (3) stage dependencies satisfied, (4) stage names match StageType enum"
   - Action: Enumerate exact validation rules

4. **Generic Test Descriptions**:
   - ❌ "Add 8-10 unit tests"
   - ✅ "Add 10 unit tests: (1) test_config_parsing_valid, (2) test_config_parsing_invalid_toml, (3) test_precedence_cli_overrides_per_spec, (4) test_dependency_validation_error, ..."
   - Action: Name each test with scenario

---

### 3.3 Rationalize Duplicated Work

**Scan for Potential Duplication**:

**Pattern 1: Similar Files Across SPECs**:
```bash
# Extract all "New Files" tables from implementation specs
grep -A 10 "New Files" docs/SPEC-*-IMPL*/implementation-plan.md

# Look for similar names:
# - config_loader.rs vs pipeline_config_loader.rs (duplicate?)
# - validation.rs in multiple SPECs (consolidate?)
```

**For Each Potential Duplicate**:

**Analyze**:
- Are these truly duplicates or different purposes?
- Can one SPEC create, other SPEC reuse?
- Should they be merged into shared module?

**Document**:
```markdown
### Duplication #1: Config Loading Logic

**SPECs Involved**: SPEC-A, SPEC-B

**SPEC-A Creates**: `config_loader.rs` (150 LOC) - loads agent configs
**SPEC-B Creates**: `pipeline_config.rs` (250 LOC) - loads pipeline configs

**Analysis**:
- Different domains: agent config vs pipeline config
- Different schemas: AgentConfig vs PipelineConfig
- Shared logic: TOML parsing, precedence merging

**Resolution Options**:
- Option 1: Keep separate (domain isolation)
- Option 2: Extract shared `config_utils.rs` (precedence logic)
- Option 3: SPEC-B reuses SPEC-A's loader (extend for pipeline)

**Recommended**: Option 1 (keep separate - different schemas justify separate modules)

**Action**: None (no duplication, justified separation)
```

**Pattern 2: Similar Test Scenarios**:
- Are multiple SPECs testing the same behavior?
- Example: SPEC-A tests "config precedence", SPEC-B tests "config precedence"
- Resolution: If testing different configs (agent vs pipeline), keep separate
- If testing same config: Consolidate into SPEC that owns the config module

---

### 3.4 Identify Missing Information

**Checklist for Each Implementation SPEC**:

**Missing Technical Details**:
- [ ] **API Contracts**: Are trait definitions complete with all methods?
- [ ] **Data Schemas**: Are struct fields specified (not "fields TBD")?
- [ ] **Error Handling**: Are all Result<T, E> error types specified?
- [ ] **Integration Points**: Are exact function signatures provided?
- [ ] **Migration Path**: Is upgrade process specified (if changing existing code)?

**Missing Validation Criteria**:
- [ ] **Compilation Checkpoints**: Is `cargo build -p <pkg>` specified after each phase?
- [ ] **Test Thresholds**: Is "100% pass rate" or "N tests passing" specified?
- [ ] **Performance Benchmarks**: Are exact metrics provided (not "faster" but "2× faster")?
- [ ] **Evidence Requirements**: Are telemetry/log files specified?

**Missing Documentation Plans**:
- [ ] **User Docs**: Is user-facing guide planned?
- [ ] **Migration Docs**: Is upgrade/rollback guide planned?
- [ ] **API Docs**: Are rustdoc comments planned for public APIs?
- [ ] **CHANGELOG**: Is entry specified?

**For Each Missing Item**:

```markdown
### Missing Information #1: [Category]

**Location**: SPEC-ID, Phase X

**What's Missing**:
[Specific missing detail]

**Why It Matters**:
[Impact of missing information on implementation]

**Resolution**:
[Specific content to add]

**Action**:
Update implementation-plan.md Phase X with: [exact addition]
```

---

### 3.5 Challenge Assumptions

**Extract All Assumptions from Implementation SPECs**:

Pattern matching:
```bash
grep -i "assume\|expect\|should\|likely\|typical\|usual\|generally" \
  docs/SPEC-*-IMPL*/implementation-plan.md
```

**For Each Assumption, Ask**:

1. **"GPT-5 models are 2-3× faster on simple tasks"**
   - ❓ Evidence: Where did this number come from? (Research SPEC? External source?)
   - ❓ Validation: How will we measure this? (SPEC-940 timing?)
   - ❓ Fallback: What if only 1.5× faster? Does it change estimates?

2. **"pipeline_config.rs will be ~250-300 LOC"**
   - ❓ Basis: Similar modules in codebase? Counted lines in pseudocode?
   - ❓ Impact: What if actual is 400-500 LOC? (effort estimate changes?)

3. **"Users will skip validate/audit to save cost"**
   - ❓ Validated: Did users actually request this workflow?
   - ❓ Risk: Will this lead to lower quality outputs?
   - ❓ Safeguard: Are warnings sufficient to prevent misuse?

4. **"SPEC-936 is 95% complete"**
   - ❓ Verification: Check SPEC.md status, git commits, actual code
   - ❓ Missing 5%: What exactly is incomplete?
   - ❓ Blocker: Does the 5% affect SPEC-949 implementation?

**Document Challenge Results**:
```markdown
### Assumption Challenge #1: [Assumption]

**Location**: SPEC-ID, Phase X

**Assumption**:
"[Quote assumption]"

**Challenge Questions**:
1. [Question 1]
2. [Question 2]

**Verification Method**:
[How to test this assumption]

**If Assumption False**:
- Impact: [What changes?]
- Mitigation: [Alternative approach]

**Resolution**:
- ✅ Assumption validated: [Evidence]
- ⚠️ Assumption weakened: [Update estimate/plan]
- ❌ Assumption false: [Major revision needed]
```

---

## Phase 4: Integration Synthesis (30-45 minutes)

### 4.1 Build Unified Dependency Graph

**Combine All SPEC Dependencies**:

```
SPEC-936 (95% complete)
    ↓
SPEC-949 (Week 1-2, 16-24h)
    ├─ Enables: SPEC-948 testing
    └─ Optional for: SPEC-948 (can use GPT-4)

Config Infrastructure (exists)
    ↓
SPEC-948 (Week 2-3, 20-28h)
    ├─ Creates: pipeline_config.rs (HARD DEP for SPEC-947)
    └─ Enables: SPEC-947

SPEC-948 Phase 1 (pipeline_config.rs)
    ↓
SPEC-947 (Week 3-4, 24-32h)
    └─ User-facing feature
```

**Identify**:
- **Critical Path**: Longest serial dependency chain
- **Parallelization**: Which SPECs can execute concurrently?
- **Bottlenecks**: Single task blocking multiple SPECs
- **Quick Wins**: SPECs with no dependencies (start immediately)

---

### 4.2 Calculate Aggregate Metrics

**Total Effort** (sum all implementation specs):
- Optimistic: Sum of minimum estimates (e.g., 16 + 20 + 24 = 60h)
- Realistic: Sum of maximum estimates (e.g., 24 + 28 + 32 = 84h)
- With buffer: Realistic × 1.2 (e.g., 84h × 1.2 = 101h)

**Total New Code**:
- New files: Count all "New Files" rows, sum LOC
- Modified files: Sum all "+LOC" estimates
- Net change: (New + Added) - Deleted

**Total New Tests**:
- Unit tests: Sum all unit test counts
- Integration tests: Sum all integration test counts
- Total: Unit + Integration
- New pass rate: (Current tests + New tests) with 100% maintained

**Total Timeline**:
- Sequential: Sum of all SPECs (if fully serial)
- Parallel: Critical path duration only (if parallelizable)
- Calendar time: Account for single developer, 40h/week, buffer

**Example Calculation**:
```
SPEC-A: 20h (Week 1)
SPEC-B: 30h (Week 2-3, depends on SPEC-A)
SPEC-C: 25h (Week 1-2, independent)

Sequential: 20 + 30 + 25 = 75h = 1.9 weeks (no parallelization)
Parallel: max(20 + 30, 25) = 50h = 1.25 weeks (SPEC-C parallel with SPEC-A)
With buffer: 50h × 1.2 = 60h = 1.5 weeks (recommended)
```

---

### 4.3 Consolidate Risk Assessments

**Aggregate All Risks from All SPECs**:

Create master risk matrix:
```markdown
| Risk ID | Description | SPECs Affected | Severity | Probability | Mitigation | Owner |
|---------|-------------|----------------|----------|-------------|------------|-------|
| R1 | SPEC-948 API incomplete | 947, 948 | High | Low | Phase 1 verification | SPEC-948 |
| R2 | GPT-5 model names change | 949 | Medium | Medium | Model aliases, monitoring | SPEC-949 |
| R3 | TUI rendering bugs | 947 | Medium | Medium | Follow proven patterns | SPEC-947 |
```

**Risk Prioritization**:
1. **High Severity × High Probability**: Address BEFORE implementation starts
2. **High Severity × Low Probability**: Have rollback plan ready
3. **Low Severity**: Monitor during implementation

**Cross-SPEC Risks** (affect multiple SPECs):
- Flag risks that cascade (e.g., SPEC-A failure blocks SPEC-B and SPEC-C)
- Create contingency: If SPEC-A fails, can SPEC-B/C proceed with alternative?

---

### 4.4 Reconcile Test Coverage

**Aggregate Test Counts**:
```markdown
| SPEC | Unit Tests | Integration Tests | Performance Tests | Total |
|------|------------|-------------------|-------------------|-------|
| 949  | 11-15      | 6                 | 3                 | 20-24 |
| 948  | 16-20      | 8                 | 3                 | 27-31 |
| 947  | 10-14      | 7                 | 0                 | 17-21 |
| **Total** | **37-49** | **21** | **6** | **64-76** |
```

**Check for**:
- ❓ Are tests testing overlapping functionality?
- ❓ Are integration tests at right boundaries (not testing internals)?
- ❓ Are performance tests measuring right metrics?

**Consolidation Opportunities**:
- If SPEC-A and SPEC-B both test "config loading", can tests be shared?
- If shared: Create `tests/shared_config_tests.rs`, both SPECs use
- If different: Keep separate, document why (different config types)

**Gap Analysis**:
- ❓ Are there integration points WITHOUT integration tests?
- Example: SPEC-948 creates pipeline_config.rs, SPEC-947 uses it → Need integration test validating the interface
- Add missing test: "test_pipeline_config_ui_integration" (load in TUI, modify, save, reload)

---

## Phase 5: Unified Execution Plan Generation (60-90 minutes)

### 5.1 Create Master Implementation Schedule

**Synthesize All SPECs into Single Timeline**:

```markdown
# Unified Implementation Plan: [Title]

**SPECs Covered**: [List of SPEC-IDs]
**Total Effort**: Xh-Yh (Z-W weeks at 40h/week)
**Timeline**: [Start Date] → [End Date]
**Developer**: Single contributor (assumes 40h/week, 20% buffer for unknowns)

---

## Week-by-Week Schedule

### Week 1: [Primary SPEC-ID] + [Parallel SPEC-ID if applicable]

**Monday** (8h):
- AM (4h): [SPEC-ID Phase X Task 1]
  - File: path/to/file.rs
  - Deliverable: [Specific output]
  - Validation: `cargo test -p <pkg> <module>`
- PM (4h): [SPEC-ID Phase X Task 2]
  - File: path/to/file.rs
  - Deliverable: [Specific output]
  - Validation: [Specific check]

**Tuesday** (8h):
- AM (4h): [Task 3]
- PM (4h): [Task 4]

**Wednesday** (8h):
- AM (4h): [Task 5]
- PM (4h): Phase X validation + commit
  - Checkpoint: [Milestone achieved]
  - Evidence: [Files created/tests passing]

**Thursday** (8h):
- AM (4h): [SPEC-ID Phase Y Task 1]
- PM (4h): [Task 2]

**Friday** (8h):
- AM (4h): [Task 3]
- PM (4h): Buffer time / code review / documentation

**Weekend** (optional, 0-4h):
- Catch-up if week ran over
- OR start next week's Phase 1 early

**Week 1 Milestone**: [Major deliverable]
**Validation**: [How to verify week 1 success]

---

### Week 2: [Primary SPEC-ID]

**Monday-Friday** (40h):
[Repeat daily breakdown structure]

**Week 2 Milestone**: [Major deliverable]

---

### Week 3: [Final SPEC-ID]

[Repeat structure]

---

### Week 4: Integration & Validation

**Monday-Wednesday** (24h):
- Cross-SPEC integration testing
- Performance validation (SPEC-940 benchmarks)
- Documentation completion
- User acceptance testing

**Thursday-Friday** (16h):
- Bug fixes from validation
- Evidence collection
- CHANGELOG updates
- Production deployment preparation

**Week 4 Milestone**: All SPECs complete, validated, production-ready

---

## Daily Task Breakdown (Complete)

[For each day, enumerate every task with:]
- Task ID: SPEC-ID-PX-TY (e.g., SPEC-949-P1-T1)
- Description: [What to do]
- File: [Exact path]
- Changes: [Specific code changes]
- Time: [Hours estimate]
- Dependencies: [Previous task IDs that must complete]
- Validation: [How to verify completion]
- Checkpoint: [If this task completes a phase/milestone]
```

---

### 5.2 Identify Critical Path

**Calculate Critical Path**:

Using dependency graph from Phase 4.1:

1. **List all execution paths** (from start to final deliverable):
   - Path 1: SPEC-949 → SPEC-948 → SPEC-947 (sequential)
   - Path 2: SPEC-949 || SPEC-948 → SPEC-947 (parallel start)

2. **Calculate duration for each path**:
   - Path 1 (sequential): 24h + 28h + 32h = 84h
   - Path 2 (parallel): max(24h, 28h) + 32h = 60h

3. **Identify critical path** (longest duration):
   - Critical Path: Path 1 (if no parallelization) = 84h
   - Optimized: Path 2 (with parallelization) = 60h

4. **Mark critical tasks** (tasks on critical path cannot slip):
   - SPEC-948 Phase 1 (pipeline_config.rs creation) - blocks SPEC-947
   - SPEC-947 Phase 4 (final integration) - final deliverable

**Slack Analysis**:
- Tasks NOT on critical path have slack (can delay without affecting end date)
- Example: SPEC-949 has 4h slack (can finish in 24h instead of 20h, still parallel)

---

### 5.3 Create Integration Test Matrix

**Cross-SPEC Integration Testing**:

```markdown
## Integration Test Matrix

| Test ID | Description | SPECs Involved | Validates | When to Run | Success Criteria |
|---------|-------------|----------------|-----------|-------------|------------------|
| INT-1 | GPT-5 model in multi-agent consensus | 949, 948 | Model registration + stage execution | After SPEC-949 Phase 2 | /speckit.plan uses gpt5_1, cost $0.30 |
| INT-2 | CLI flag filtering with GPT-5 | 949, 948 | Model + stage filtering | After SPEC-948 Phase 3 | --skip-validate works with gpt5_1_mini |
| INT-3 | TUI configurator loads pipeline config | 948, 947 | Config data layer + UI | After SPEC-947 Phase 2 | Modal displays enabled_stages from TOML |
| INT-4 | End-to-end workflow | 949, 948, 947 | Full integration | After SPEC-947 Phase 4 | Configure via UI → Execute → GPT-5 models used |
```

**Execution Plan**:
- Run INT-1 after SPEC-949 Phase 2 (validates model integration)
- Run INT-2 after SPEC-948 Phase 3 (validates CLI + models)
- Run INT-3 after SPEC-947 Phase 2 (validates UI + backend)
- Run INT-4 at end (final validation)

**Failure Handling**:
- If INT-1 fails: Block SPEC-948 Phase 2 until SPEC-949 fixed
- If INT-3 fails: Indicates SPEC-948 API insufficient, extend before SPEC-947 Phase 3

---

### 5.4 Synthesize Documentation Plan

**Aggregate All Documentation Requirements**:

**User-Facing Docs** (from all SPECs):
1. GPT5_MIGRATION_GUIDE.md (SPEC-949, 200-300 lines)
2. PROVIDER_SETUP_GUIDE.md (SPEC-949, 300-400 lines)
3. PIPELINE_CONFIGURATION_GUIDE.md (SPEC-948, 300-400 lines)
4. Workflow examples (SPEC-948, 4 × ~40 lines = 160 lines)
5. Command reference updates in CLAUDE.md (all SPECs, ~50 lines)

**Developer Docs**:
1. API documentation (rustdoc) for all public APIs (inline)
2. Architecture decision records (why these designs?)
3. Migration guides (for breaking changes)

**Total Documentation**: ~1,400-1,700 lines markdown + inline rustdoc

**Documentation Schedule**:
- SPEC-949 Phase 4: Migration + provider guides (6h)
- SPEC-948 Phase 4: Configuration guide + examples (4h)
- SPEC-947 Phase 4: Inline help text + CLAUDE.md updates (2h)
- Week 4: CHANGELOG consolidation, architecture docs (4h)

**Total Documentation Effort**: ~16h (already included in phase estimates)

---

## Phase 6: Validation & Refinement (30-45 minutes)

### 6.1 Implementation Plan Completeness Verification

**For Each Implementation SPEC**:

Run through checklist:

```markdown
### SPEC-[ID]-IMPL Completeness

**Research Findings Addressed**:
- [ ] All technical questions answered
- [ ] All architectural decisions translated to tasks
- [ ] All identified risks have mitigation plans
- [ ] All success criteria measurable

**Task Breakdown Quality**:
- [ ] Every task has exact file path
- [ ] Every task has LOC estimate
- [ ] Every task has rationale
- [ ] Every task has dependencies listed
- [ ] Every task <4 hours (atomic)

**Test Coverage**:
- [ ] Unit test scenarios specific (not generic "add tests")
- [ ] Integration test scenarios enumerate workflows
- [ ] Performance tests measure specific metrics
- [ ] Test counts justified (not arbitrary)

**Timeline**:
- [ ] Phase estimates have basis (similar work or breakdown)
- [ ] Dependencies sequenced correctly
- [ ] Validation checkpoints after each phase
- [ ] Buffer time included (20% recommended)

**Documentation**:
- [ ] User-facing guide planned
- [ ] Migration/rollback guide planned
- [ ] API documentation planned (rustdoc)
- [ ] CHANGELOG entry specified

**Score**: X/15 (aim for 15/15 before implementation)
```

**If Score <12/15**:
- Identify gaps (which checkboxes unchecked?)
- Resolve before proceeding to Phase 7

---

### 6.2 Cross-Reference with Project Standards

**Check Against SPEC-Kit Standards**:

From `CLAUDE.md` and `memory/constitution.md`:

- [ ] **6-Stage Template**: Do implementations preserve existing pipeline structure?
- [ ] **Telemetry v1 Schema**: Do new telemetry files use schemaVersion: "1.0"?
- [ ] **Quality Gates**: Is quality gate integration specified?
- [ ] **Evidence Policy**: Are all SPECs <25MB soft limit?
- [ ] **Test Coverage**: Do new tests maintain 40%+ coverage target?
- [ ] **Backward Compatibility**: Is existing functionality preserved?
- [ ] **FORK-SPECIFIC Markers**: Are fork-only changes marked?

**Check Against Coding Standards**:

- [ ] **SOLID Principles**: Single responsibility per module?
- [ ] **DRY**: No duplicated logic across SPECs?
- [ ] **Error Handling**: All Result<T, String> or Result<T, SpecKitError>?
- [ ] **Validation**: Fail-fast on invalid inputs?

**If Any Standard Violated**:
- Document violation
- Justify (intentional deviation with rationale) OR fix (update implementation plan)

---

### 6.3 Validate Against Historical Data

**Compare to Similar Completed SPECs** (from SPEC.md):

**Effort Accuracy**:
- SPEC-938: Estimated 8-10h, Actual 4h (50% under-estimate)
- SPEC-941: Estimated 8-10h, Actual 6h (40% under)
- Pattern: Implementation tends to be faster than estimates

**Implication for Current Plans**:
- SPEC-949 estimated 16-24h → Likely 12-20h actual (apply 0.75× multiplier?)
- SPEC-948 estimated 20-28h → Likely 15-22h actual
- SPEC-947 estimated 24-32h → Likely 18-26h actual (TUI is higher uncertainty)

**Question**: Should estimates be adjusted based on historical pattern?
- **Option 1**: Keep conservative estimates (better to finish early than late)
- **Option 2**: Adjust down by 25%, add as "optimistic" column
- **Recommended**: Keep current estimates (buffer is valuable for unknowns)

---

## Phase 7: Final Synthesis & Recommendations (30-45 minutes)

### 7.1 Create Executive Summary

```markdown
# Implementation Readiness Report: [SPEC-IDs]

**Analysis Date**: 2025-11-16
**SPECs Analyzed**: [Count] implementation specs
**Status**: ✅ Ready for Implementation | ⚠️ Gaps Identified | ❌ Major Issues

---

## Summary

[2-3 paragraph executive summary]
- What: [What features are being implemented]
- Why: [Strategic value, cost/performance impacts]
- How: [High-level approach, sequence]
- When: [Timeline with milestones]

---

## Readiness Assessment

**Completeness**: X/Y specs fully detailed (aim for 100%)
**Consistency**: Z inconsistencies found, W resolved (aim for 0 unresolved)
**Clarity**: A ambiguities found, B resolved (aim for 0 unresolved)
**Validation**: C validation criteria specified (aim for 100% of claims)

**Overall Score**: XX/100 (aim for ≥90 before implementation)

---

## Critical Findings

### ✅ Strengths

1. [Strength 1: e.g., "Comprehensive test coverage planned (64-76 new tests)"]
2. [Strength 2: e.g., "Clear dependency sequencing (949→948→947)"]
3. [Strength 3: e.g., "Shared component reuse prevents duplication"]

### ⚠️ Gaps Requiring Resolution

1. **Gap 1**: [Specific missing information]
   - Impact: [What's affected]
   - Resolution: [What needs to be added]
   - Owner: [Which SPEC needs update]

2. **Gap 2**: [...]

### ❌ Blockers (Must Resolve Before Implementation)

1. **Blocker 1**: [Critical issue]
   - Impact: [Why this blocks implementation]
   - Resolution: [What must be done]
   - Effort: [Time to resolve]

---

## Recommendations

### Immediate Actions (Before Implementation Starts)

1. **[Action 1]**: [Specific task]
   - Effort: [Hours]
   - Owner: [SPEC-ID]
   - Why: [Rationale]

2. **[Action 2]**: [...]

### Implementation Sequence (Optimized)

**Phase 1 (Week 1-2)**: [SPEC-ID]
- Justification: [Why this first]
- Parallel opportunities: [What else can run concurrently]
- Checkpoint: [Milestone]

**Phase 2 (Week 2-3)**: [SPEC-ID]
- Dependencies: [What must be done first]
- Parallel opportunities: [...]
- Checkpoint: [Milestone]

**Phase 3 (Week 3-4)**: [SPEC-ID]
- Dependencies: [...]
- Checkpoint: [Final deliverable]

### Risk Mitigation Priority

**High Priority** (address before starting):
1. [Risk with mitigation plan]
2. [...]

**Medium Priority** (monitor during implementation):
1. [Risk with contingency]

**Low Priority** (accept risk):
1. [Risk with note]

---

## Success Criteria (Consolidated)

**Functional**:
1. [Measurable outcome 1]
2. [Measurable outcome 2]

**Quality**:
1. [Test pass rate target]
2. [Coverage target]
3. [No regressions]

**Performance**:
1. [Metric 1: baseline → target]
2. [Metric 2: baseline → target]

**Documentation**:
1. [Guide 1 complete]
2. [Guide 2 complete]

---

## Effort Summary (Reconciled)

| SPEC | Optimistic | Realistic | With Buffer | Actual (Post-Impl) |
|------|------------|-----------|-------------|-------------------|
| SPEC-A | Xh | Yh | Zh | [Fill after] |
| SPEC-B | Xh | Yh | Zh | [Fill after] |
| **Total** | **Xh** | **Yh** | **Zh** | [Fill after] |

**Timeline**:
- Sequential: Z weeks (no parallelization)
- Optimized: W weeks (with parallelization)
- Recommended: W weeks (assumes 40h/week, buffer included)

---

## Appendix: Reconciliation Log

### Inconsistencies Resolved

1. **[Inconsistency description]**: [Resolution]
2. [...]

### Ambiguities Clarified

1. **[Ambiguity description]**: [Clarification]
2. [...]

### Assumptions Validated

1. **[Assumption]**: ✅ Validated / ⚠️ Weakened / ❌ False
2. [...]

### Missing Information Added

1. **[What was missing]**: [What was added]
2. [...]
```

---

### 7.2 Generate Prioritized Action Items

**Output**: Ranked list of next steps

```markdown
## Immediate Actions (Next 24 Hours)

**Priority 1 (MUST DO - Blockers)**:
1. [ ] Resolve [Blocker identified in Phase 6]
   - Impact: Blocks implementation start
   - Effort: Xh
   - Owner: [SPEC-ID or person]

**Priority 2 (SHOULD DO - Risk Mitigation)**:
1. [ ] Validate [Critical assumption]
   - Impact: If false, changes estimates by Y%
   - Effort: Xh
   - Method: [How to validate]

**Priority 3 (NICE TO DO - Optimization)**:
1. [ ] Optimize [Identified inefficiency]
   - Impact: Saves Xh implementation time
   - Effort: Yh upfront
   - ROI: [Justify if worth it]

---

## This Week (Implementation Kickoff)

1. [ ] Begin SPEC-[ID] Phase 1
   - File: [First file to create/modify]
   - Deliverable: [First concrete output]
   - Validation: [How to verify]
   - Time: Xh

2. [ ] Set up validation infrastructure
   - Create test SPEC for validation (e.g., SPEC-900)
   - Set up SPEC-940 timing if not exists
   - Prepare evidence directories

3. [ ] Document baseline metrics
   - Current test count: `cargo test --list | wc -l`
   - Current cost per /speckit.auto: [Measure with test run]
   - Current coverage: `cargo tarpaulin` (if available)

---

## Next 30 Days (Full Implementation)

**Week 1**: [SPEC-ID primary + SPEC-ID parallel if applicable]
**Week 2**: [SPEC-ID]
**Week 3**: [SPEC-ID]
**Week 4**: Integration validation + documentation completion

**Checkpoints**:
- Week 1 end: [Milestone 1]
- Week 2 end: [Milestone 2]
- Week 3 end: [Milestone 3]
- Week 4 end: Production ready
```

---

### 7.3 Create Risk Monitoring Dashboard

**Output**: Risk tracking template for during implementation

```markdown
## Risk Monitoring Dashboard

**Update Frequency**: Daily during implementation
**Review Trigger**: Any risk moves from Low→Medium or Medium→High

---

### Active Risks (Monitoring Required)

| Risk ID | Status | Severity | Probability | Mitigation Status | Last Updated | Notes |
|---------|--------|----------|-------------|-------------------|--------------|-------|
| R1 | 🟡 Active | High | Low | In Progress | 2025-11-16 | [Current status] |
| R2 | 🟢 Mitigated | Medium | Medium | Complete | 2025-11-15 | [How mitigated] |
| R3 | 🔴 Triggered | Medium | High | Failed | 2025-11-14 | [Rollback initiated] |

**Legend**:
- 🟢 Mitigated: Risk addressed, monitoring only
- 🟡 Active: Risk present, mitigation in progress
- 🔴 Triggered: Risk materialized, response active

---

### Risk Response Procedures

**If Risk Triggers**:
1. Assess impact (blocks current task? blocks phase? blocks SPEC?)
2. Execute mitigation plan (from implementation spec)
3. Update dashboard status
4. If mitigation fails: Execute rollback procedure
5. Document in evidence: `docs/SPEC-ID/evidence/risk_R#_triggered.md`

**Escalation Criteria**:
- 2+ High-severity risks triggered simultaneously
- Critical path task blocked >24 hours
- Rollback fails (cannot restore working state)
```

---

## Execution Instructions

### Prerequisites

**Required Context**:
1. All implementation specs from last 7 days (read completely)
2. Related research SPECs (to verify claims)
3. SPEC.md current state (for baseline test counts, priorities)
4. Local-memory recent entries (for architectural decisions)

**Required Tools**:
- `find`, `grep`, `stat` for file discovery
- Local-memory MCP for planning session retrieval
- Read tool for implementation spec analysis
- Code graph analysis for dependency verification (optional)

**Required Time**: 3-4 hours (comprehensive analysis, cannot be rushed)

---

### Execution Mode

**Sequential Phases** (must complete in order):
1. Discovery (Phase 1): Find all specs, build inventory
2. Analysis (Phase 2): Critical assessment, flag issues
3. Reconciliation (Phase 3): Resolve inconsistencies, clarify ambiguities
4. Synthesis (Phase 4): Build unified plan
5. Validation (Phase 5): Cross-check against standards
6. Output (Phase 6): Generate final report

**Deep Analysis Required**:
- Read every implementation spec completely (not skimming)
- Cross-reference claims (verify dependencies actually exist)
- Challenge assumptions (don't accept at face value)
- Think adversarially (what could go wrong?)

**Evidence-Based**:
- Every claim should be verifiable (code exists, tests run, metrics measured)
- Flag unverified claims explicitly
- Distinguish between "researched" and "assumed"

---

### Output Format

**Primary Deliverable**:
`docs/IMPLEMENTATION-READINESS-REPORT-[DATE].md` (3,000-5,000 words)

**Structure**:
1. Executive Summary (readiness score, go/no-go recommendation)
2. SPEC Inventory (all specs analyzed)
3. Critical Findings (strengths, gaps, blockers)
4. Reconciliation Log (inconsistencies resolved, ambiguities clarified)
5. Unified Execution Plan (week-by-week, day-by-day)
6. Risk Monitoring Dashboard (active risks, response procedures)
7. Success Criteria (consolidated, measurable)
8. Immediate Actions (prioritized next steps)

**Secondary Deliverables**:
- Updated implementation-plan.md files (if gaps found)
- Integration test specifications (cross-SPEC tests)
- Risk response procedures (rollback scripts, validation checks)

---

### Quality Gates (Self-Check)

**Before Submitting Final Report**:

- [ ] All discovered implementation specs analyzed (100% coverage)
- [ ] All inconsistencies either resolved or documented as blockers
- [ ] All ambiguities either clarified or escalated for decision
- [ ] Unified execution plan has daily task breakdown (not just week-by-week)
- [ ] Critical path identified with slack analysis
- [ ] Integration test matrix specifies exact test cases
- [ ] Risk dashboard has response procedures (not just "monitor")
- [ ] Go/no-go recommendation has clear criteria

**If Any Checklist Item Unchecked**: Explain why in report (scope limitation, missing data, etc.)

---

## Success Criteria (This Analysis)

**Analysis Complete When**:
1. All implementation specs from last 7 days discovered and read
2. All inconsistencies documented and resolved (or escalated)
3. All ambiguities clarified (or flagged for decision)
4. All missing information identified (or filled in)
5. Unified execution plan created (week-by-week, day-by-day)
6. Risk monitoring dashboard created
7. Readiness score calculated (≥90/100 for go-ahead)
8. Implementation readiness report written (3,000-5,000 words)

**Estimated Duration**: 3-4 hours (cannot be rushed, deep analysis required)

**Recommended Approach**:
- Block dedicated time (no interruptions)
- Work through phases sequentially (don't skip reconciliation)
- Challenge assumptions aggressively (better to find issues now than during implementation)
- Document everything (audit trail for decisions)

---

**END OF PROMPT**

---

## Usage Example

```bash
# 1. Create fresh session (avoid context pollution)
# In Claude Code TUI:

# 2. Paste this entire prompt (or load from file)

# 3. Agent will execute 7 phases over 3-4 hours:
#    - Discovery: Find all implementation specs from last week
#    - Analysis: Critical assessment of completeness, consistency
#    - Reconciliation: Resolve issues, clarify ambiguities
#    - Synthesis: Build unified execution plan
#    - Validation: Check against standards
#    - Output: Generate readiness report

# 4. Review output:
#    - Implementation readiness report (3,000-5,000 words)
#    - Go/no-go recommendation with score
#    - Prioritized action items

# 5. If score ≥90/100: Proceed with implementation
#    If score <90/100: Resolve blockers first
```

---

## Expected Output Preview

**Implementation Readiness Report** will include:

- **Readiness Score**: 87/100 (example)
  - Completeness: 18/20 (90%)
  - Consistency: 9/10 (90%)
  - Clarity: 15/20 (75%) ⚠️
  - Validation: 19/20 (95%)
  - Standards: 14/15 (93%)
  - Timeline: 12/15 (80%) ⚠️

- **Recommendation**: ⚠️ CONDITIONAL GO
  - Resolve 5 ambiguities in SPEC-947 Phase 2 (2h effort)
  - Validate timeline assumptions with historical data
  - Then proceed with implementation

- **Critical Path**: SPEC-948 Phase 1 → SPEC-947 (6-8h delay acceptable)

- **Immediate Actions**:
  1. Clarify TUI event loop integration (SPEC-947 ambiguity #3)
  2. Validate GPT-5 model name format (SPEC-949 assumption #1)
  3. Verify ProviderRegistry completion status (SPEC-936 dependency)

---

## Customization Notes

**Adjust for Your Context**:
- Change date range in Phase 1.1 (`-mtime -7` → `-mtime -14` for 2 weeks)
- Add project-specific standards to Phase 6.2 validation
- Include your historical benchmarks in Phase 6.3 (effort validation)
- Customize readiness threshold (90/100 vs 80/100 depending on risk tolerance)

**Heavy vs Light Analysis**:
- **Heavy** (3-4h): Execute all 7 phases, comprehensive reconciliation
- **Light** (1-2h): Skip Phase 3.3 (duplication check) and Phase 4.3 (test consolidation)
- **Critical-Only** (30-60min): Only Phase 2.2 (consistency) and Phase 2.4 (dependency validation)

---

**Prompt Version**: 1.0
**Created**: 2025-11-16
**Estimated Execution Time**: 3-4 hours (comprehensive mode)
