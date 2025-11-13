# PRD: Automated Policy Compliance

**SPEC-ID**: SPEC-KIT-941
**Created**: 2025-11-13
**Status**: Draft - **MEDIUM PRIORITY**
**Priority**: **P2** (Quality + Policy Enforcement)
**Owner**: Code
**Estimated Effort**: 8-10 hours (1-2 days)
**Dependencies**: SPEC-934 (fixes initial violation, this prevents regression)
**Blocks**: None

---

## 🔥 Executive Summary

**Current State**: SPEC-KIT-072 policy violated (quality_gate_handler.rs:1775 stores consensus to MCP instead of SQLite). No automated enforcement (violations discovered manually during research). Risk of regression (after SPEC-934 fixes violation, could break again). Multiple policy dimensions (storage separation, memory importance threshold, tag schema) not validated automatically.

**Proposed State**: Automated policy compliance validation via CI checks. Static analysis for storage separation (grep for "mcp.*consensus" in spec_kit/ modules). Mandatory pre-commit hooks provide fast feedback (<5s checks before CI). Policy compliance dashboard visualizes status (pass/fail per rule). Prevents policy drift through continuous validation.

**Impact**:
- ✅ Prevents SPEC-KIT-072 violations (automated detection)
- ✅ Catches regressions early (CI fails on policy break)
- ✅ Developer-friendly (clear error messages, auto-fix suggestions)
- ✅ Comprehensive coverage (storage, importance, tags, retention)

**Source**: SPEC-931A architectural analysis identified policy violation (Q92 in QUESTION-CONSOLIDATION-ANALYSIS.md). SPEC-934 fixes violation, this SPEC prevents recurrence.

---

## 1. Problem Statement

### Issue #1: SPEC-KIT-072 Policy Violation (CRITICAL)

**Policy** (MEMORY-POLICY.md:351-375):
```markdown
# Storage Separation (SPEC-KIT-072)

**Workflow data** (transient, orchestration state):
- AGENT_MANAGER (in-memory coordination)
- SQLite (persistent workflow state)
- ❌ NOT MCP (human knowledge only)

**Knowledge** (human-curated, reusable insights):
- MCP local-memory (importance ≥8)
- ❌ NOT SQLite (workflow data only)
```

**Violation** (quality_gate_handler.rs:1775):
```rust
// Consensus artifacts going to MCP - VIOLATES POLICY!
mcp_client.store_memory(
    content: consensus_artifact_json,  // Workflow data
    domain: "spec-kit",
    tags: ["consensus", "stage:validate"],
    importance: 7  // Also violates importance threshold (should be ≥8)
);
```

**Evidence** (SPEC-931B-analysis.md:464-467):
```
SPEC-KIT-072 policy says: Consensus → SQLite, Knowledge → MCP
Reality shows: Consensus → MCP (WRONG SYSTEM!)
```

**Discovery Method**: Manual code review during SPEC-931 research.

**Problem**: No automated check to prevent this violation in future PRs.

---

### Issue #2: No Automated Enforcement (HIGH)

**Current Process**:
1. Developer writes code that violates policy
2. Code passes CI (no policy checks)
3. PR merged to main
4. Violation discovered weeks later during manual audit
5. Emergency fix required (disrupts planned work)

**Example Timeline** (hypothetical regression after SPEC-934):
```
Week 1: SPEC-934 fixes policy violation (consensus → SQLite)
Week 5: Developer adds new feature, accidentally stores consensus to MCP
Week 10: Manual audit discovers regression
Week 11: Emergency fix, delays SPEC-936 implementation
```

**Better Approach** (automated validation):
```
Week 1: SPEC-934 fixes policy violation + adds CI check (SPEC-941)
Week 5: Developer adds new feature with MCP storage
Week 5: CI fails with clear error: "Policy violation: Consensus storage to MCP (use SQLite)"
Week 5: Developer fixes immediately (no merge, no regression)
```

**Benefit**: Shift left (catch violations at PR time, not weeks later).

---

### Issue #3: Policy Drift Risk (MEDIUM)

**Current Policy Dimensions**:
1. **Storage separation** (workflow → SQLite, knowledge → MCP)
2. **Importance threshold** (MCP storage requires importance ≥8)
3. **Tag schema compliance** (namespaced, no dates, no task IDs)
4. **Memory retention** (quarterly cleanup, target 120-150 entries)

**Problems**:
- Only storage separation documented in MEMORY-POLICY.md
- Other dimensions scattered across CLAUDE.md, local docs
- No single source of truth for policy rules
- Manual enforcement prone to inconsistency

**Proposed**: Codify all policy rules in automated checks (single source of truth).

---

### Issue #4: Developer Experience (MEDIUM)

**Current Error** (if violation detected manually):
```
"Your code violates SPEC-KIT-072 storage separation policy."
```

**Problems**:
- Generic error (which line? which file?)
- No hint (how to fix?)
- No context (why is this wrong?)

**Better Error** (automated check):
```
❌ FAILED: Storage Policy Violation

File: codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs:1775
Line: mcp_client.store_memory(content: consensus_artifact_json, ...)

Rule: SPEC-KIT-072 Storage Separation
  - Workflow data (consensus artifacts) → SQLite
  - Knowledge (human insights) → MCP local-memory

Fix:
  - Replace: mcp_client.store_memory(...)
  - With: db.execute("INSERT INTO consensus_artifacts ...")

See: docs/MEMORY-POLICY.md#storage-separation
```

**Benefit**: Developers can fix immediately (no guessing, no context switching).

---

## 2. Proposed Solution

### Component 1: Storage Separation Validator (CRITICAL - 3-4h)

**Implementation** (`scripts/validate_storage_policy.sh`):
```bash
#!/bin/bash
# Validate SPEC-KIT-072 storage separation policy

set -e

echo "🔍 Checking SPEC-KIT-072 storage policy compliance..."

VIOLATIONS=0

# Check 1: No consensus artifacts in MCP calls (spec_kit modules only)
echo "  → Checking consensus storage to MCP..."
CONSENSUS_MCP=$(grep -rn "mcp.*consensus\|store_memory.*consensus" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    --exclude-dir=tests \
    | grep -v "^//" \
    | grep -v "// Knowledge storage" \
    || true)

if [ -n "$CONSENSUS_MCP" ]; then
    echo "❌ FAILED: Consensus artifacts stored to MCP (violates SPEC-KIT-072)"
    echo ""
    echo "$CONSENSUS_MCP"
    echo ""
    echo "Rule: Workflow data → SQLite, Knowledge → MCP"
    echo "Fix: Use db.execute(\"INSERT INTO consensus_artifacts ...\") instead"
    echo "See: docs/MEMORY-POLICY.md#storage-separation"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Check 2: Consensus artifacts go to SQLite
echo "  → Checking consensus storage to SQLite..."
SQLITE_CALLS=$(grep -rn "consensus_artifacts" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | wc -l)

if [ "$SQLITE_CALLS" -lt 3 ]; then
    echo "⚠️  WARNING: Expected ≥3 consensus_artifacts SQLite calls (insert/query/update), found $SQLITE_CALLS"
    echo "This may indicate missing consensus storage implementation."
fi

# Check 3: MCP importance threshold (≥8 for storage)
echo "  → Checking MCP importance threshold..."
LOW_IMPORTANCE=$(grep -rn "mcp_client.store_memory\|mcp.*store" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    -A 5 \
    | grep -E "importance:\s*[0-7]($|,)" \
    || true)

if [ -n "$LOW_IMPORTANCE" ]; then
    echo "❌ FAILED: MCP storage with importance <8 (violates memory bloat policy)"
    echo ""
    echo "$LOW_IMPORTANCE"
    echo ""
    echo "Rule: MCP storage requires importance ≥8 (quality over quantity)"
    echo "Fix: Increase importance or store to SQLite instead"
    echo "See: MEMORY-POLICY.md#importance-calibration"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Summary
echo ""
if [ $VIOLATIONS -eq 0 ]; then
    echo "✅ PASSED: Storage policy compliance validated"
    echo "   - Consensus → SQLite ✓"
    echo "   - MCP importance ≥8 ✓"
    echo "   - $SQLITE_CALLS consensus_artifacts calls found"
    exit 0
else
    echo "❌ FAILED: $VIOLATIONS policy violations detected"
    echo ""
    echo "Storage Policy (SPEC-KIT-072):"
    echo "  - Workflow data (consensus, routing, state) → SQLite"
    echo "  - Knowledge (human insights, patterns) → MCP local-memory"
    echo "  - MCP storage requires importance ≥8 (prevent bloat)"
    echo ""
    echo "Documentation: docs/MEMORY-POLICY.md"
    exit 1
fi
```

---

### Component 2: CI Integration (MEDIUM - 2h)

**GitHub Actions** (`.github/workflows/ci.yml`):
```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  policy-compliance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Validate Storage Policy
        run: bash scripts/validate_storage_policy.sh

      - name: Validate Tag Schema
        run: bash scripts/validate_tag_schema.sh

      - name: Validate Memory Hygiene
        run: bash scripts/validate_memory_hygiene.sh

  # Existing CI jobs (build, test, clippy)
  build:
    runs-on: ubuntu-latest
    needs: policy-compliance  # Block merge if policy fails
    steps:
      - uses: actions/checkout@v3
      - name: Build
        run: cargo build --release
```

**Benefit**: Policy violations block PR merge (can't violate policy accidentally).

---

### Component 3: Pre-Commit Hook (Mandatory) (MEDIUM - 2h)

**Hook** (`.githooks/pre-commit`):
```bash
#!/bin/bash
# Mandatory pre-commit policy check (fast feedback)

# Only run if spec_kit modules modified
SPEC_KIT_CHANGES=$(git diff --cached --name-only | grep "spec_kit" || true)

if [ -n "$SPEC_KIT_CHANGES" ]; then
    echo "🔍 Running storage policy checks (spec_kit modified)..."

    if ! bash scripts/validate_storage_policy.sh; then
        echo ""
        echo "❌ Storage policy violation detected"
        echo "Fix violations before committing, or skip with: git commit --no-verify"
        exit 1
    fi
fi

echo "✅ Pre-commit policy checks passed"
```

**Installation** (Mandatory Setup):
```bash
# Update scripts/setup-hooks.sh to enforce hooks
git config core.hooksPath .githooks
chmod +x .githooks/pre-commit

# Verify installation
if [ ! -f .githooks/pre-commit ]; then
    echo "❌ ERROR: Pre-commit hooks not installed"
    echo "Run: bash scripts/setup-hooks.sh"
    exit 1
fi
```

**Enforcement**: README.md and onboarding docs require running `bash scripts/setup-hooks.sh` as first setup step.

**Benefit**: Developers get feedback in <5s (before pushing to CI). Prevents wasted CI cycles on trivial violations.

---

### Component 4: Policy Compliance Dashboard (LOW - 1-2h)

**Script** (`scripts/policy_compliance_dashboard.sh`):
```bash
#!/bin/bash
# Generate policy compliance dashboard

echo "# Policy Compliance Dashboard"
echo ""
echo "Generated: $(date)"
echo ""

# Rule 1: Storage Separation
echo "## Rule 1: Storage Separation (SPEC-KIT-072)"
if bash scripts/validate_storage_policy.sh > /dev/null 2>&1; then
    echo "Status: ✅ PASS"
else
    echo "Status: ❌ FAIL"
fi
echo ""

# Rule 2: Tag Schema
echo "## Rule 2: Tag Schema Compliance"
FORBIDDEN_TAGS=$(grep -rn "2025-\|2024-\|t[0-9]\+\|T[0-9]\+" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep "tags:" \
    || true)

if [ -z "$FORBIDDEN_TAGS" ]; then
    echo "Status: ✅ PASS (no date tags, task IDs)"
else
    echo "Status: ❌ FAIL (forbidden tags found)"
fi
echo ""

# Rule 3: MCP Importance Threshold
echo "## Rule 3: MCP Importance Threshold (≥8)"
LOW_IMP=$(grep -rn "importance:\s*[0-7]" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep mcp \
    || true)

if [ -z "$LOW_IMP" ]; then
    echo "Status: ✅ PASS (all MCP storage ≥8)"
else
    echo "Status: ❌ FAIL (low importance storage found)"
fi
echo ""

# Summary
echo "## Summary"
echo "- Storage Separation: $(bash scripts/validate_storage_policy.sh > /dev/null 2>&1 && echo '✅ PASS' || echo '❌ FAIL')"
echo "- Tag Schema: $([ -z "$FORBIDDEN_TAGS" ] && echo '✅ PASS' || echo '❌ FAIL')"
echo "- MCP Importance: $([ -z "$LOW_IMP" ] && echo '✅ PASS' || echo '❌ FAIL')"
```

**Usage**:
```bash
# Generate dashboard
bash scripts/policy_compliance_dashboard.sh > docs/policy-compliance-status.md

# Review status
cat docs/policy-compliance-status.md
```

---

### Component 5: Tag Schema Validator (LOW - 1-2h)

**Script** (`scripts/validate_tag_schema.sh`):
```bash
#!/bin/bash
# Validate tag schema compliance

echo "🔍 Checking tag schema compliance..."

VIOLATIONS=0

# Forbidden: Date tags (2025-10-20, 2024-12-31)
echo "  → Checking for date tags..."
DATE_TAGS=$(grep -rn "tags.*\(2025-\|2024-\|2023-\)" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    || true)

if [ -n "$DATE_TAGS" ]; then
    echo "❌ FAILED: Date tags detected (forbidden)"
    echo ""
    echo "$DATE_TAGS"
    echo ""
    echo "Rule: No date tags (not useful for retrieval, proliferate over time)"
    echo "Fix: Use date range filters in search queries instead"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Forbidden: Task ID tags (t84, T12, t21)
echo "  → Checking for task ID tags..."
TASK_TAGS=$(grep -rn "tags.*\(\"t[0-9]\+\"\|\"T[0-9]\+\"\)" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | grep -v "^//" \
    || true)

if [ -n "$TASK_TAGS" ]; then
    echo "❌ FAILED: Task ID tags detected (forbidden)"
    echo ""
    echo "$TASK_TAGS"
    echo ""
    echo "Rule: No task ID tags (ephemeral, not useful long-term)"
    echo "Fix: Use spec: namespace instead (e.g., spec:SPEC-KIT-071)"
    VIOLATIONS=$((VIOLATIONS + 1))
fi

# Encouraged: Namespaced tags (spec:, type:, component:)
echo "  → Checking for namespaced tags..."
NAMESPACED=$(grep -rn "tags.*\(spec:\|type:\|component:\)" \
    codex-tui/src/chatwidget/spec_kit/ \
    --include="*.rs" \
    | wc -l)

echo "   Found $NAMESPACED namespaced tags (encouraged)"

# Summary
if [ $VIOLATIONS -eq 0 ]; then
    echo ""
    echo "✅ PASSED: Tag schema compliance validated"
    echo "   - No date tags ✓"
    echo "   - No task ID tags ✓"
    echo "   - $NAMESPACED namespaced tags found"
    exit 0
else
    echo ""
    echo "❌ FAILED: $VIOLATIONS tag schema violations"
    echo ""
    echo "Tag Schema Rules:"
    echo "  - ✅ Namespaced tags (spec:, type:, component:)"
    echo "  - ❌ No date tags (2025-10-20, 2024-12-31)"
    echo "  - ❌ No task ID tags (t84, T12, t21)"
    echo ""
    echo "See: MEMORY-POLICY.md#tag-schema"
    exit 1
fi
```

---

## 3. Acceptance Criteria

### AC1: Storage Separation Validation ✅
- [ ] Script detects MCP consensus storage (quality_gate_handler.rs:1775 caught)
- [ ] Script verifies SQLite consensus storage (≥3 call sites)
- [ ] Script checks MCP importance threshold (≥8 required)
- [ ] Clear error messages with file paths, line numbers, fix hints

### AC2: CI Integration ✅
- [ ] GitHub Actions job runs on every PR
- [ ] Policy violations block PR merge
- [ ] CI job completes in <30s (fast feedback)

### AC3: Pre-Commit Hook (Mandatory) ✅
- [ ] Hook runs on spec_kit module changes
- [ ] Hook provides <5s feedback
- [ ] Hook allows bypass (--no-verify for emergencies only)
- [ ] Installation mandatory via setup-hooks.sh
- [ ] README.md enforces installation as first setup step

### AC4: Policy Dashboard ✅
- [ ] Dashboard shows all policy rules (storage, tags, importance)
- [ ] Status per rule (✅ PASS / ❌ FAIL)
- [ ] Generated as Markdown (easy to review)

### AC5: Tag Schema Validation ✅
- [ ] Detects forbidden date tags (2025-10-20)
- [ ] Detects forbidden task ID tags (t84, T12)
- [ ] Encourages namespaced tags (spec:, type:, component:)

---

## 4. Technical Implementation

### Day 1: Storage Validation + CI (5-6h)

**Morning (3-4h)**:
- Create `scripts/validate_storage_policy.sh`
- Implement checks (consensus MCP, SQLite calls, importance threshold)
- Test against quality_gate_handler.rs:1775 (should fail)
- Test after SPEC-934 fix (should pass)

**Afternoon (2h)**:
- Add to `.github/workflows/ci.yml`
- Test CI job (trigger PR, verify failure)
- Document CI integration in README.md

**Files**:
- `scripts/validate_storage_policy.sh` (~100 lines)
- `.github/workflows/ci.yml` (+10 lines)

---

### Day 2: Tag Schema + Dashboard + Hooks (3-4h)

**Morning (2h)**:
- Create `scripts/validate_tag_schema.sh`
- Implement checks (date tags, task IDs, namespaced tags)
- Test against codebase (verify catches violations)

**Afternoon (1-2h)**:
- Create `scripts/policy_compliance_dashboard.sh`
- Generate sample dashboard (docs/policy-compliance-status.md)
- Add mandatory pre-commit hook, update README.md enforcement

**Files**:
- `scripts/validate_tag_schema.sh` (~80 lines)
- `scripts/policy_compliance_dashboard.sh` (~60 lines)
- `.githooks/pre-commit` (+15 lines)

---

## 5. Success Metrics

### Policy Compliance
- **Violation Detection Rate**: 100% (quality_gate_handler.rs:1775 caught)
- **False Positives**: 0% (no legitimate code flagged)
- **Regression Prevention**: 100% (CI blocks policy violations)

### Developer Experience
- **Feedback Time**: <5s (pre-commit) or <30s (CI)
- **Fix Clarity**: 90%+ developers fix on first try (clear error messages)
- **Bypass Rate**: <10% (developers rarely skip checks)

### Coverage
- **Storage Separation**: 100% validated (MCP vs SQLite)
- **Importance Threshold**: 100% validated (≥8 for MCP)
- **Tag Schema**: 100% validated (no dates, no task IDs)

---

## 6. Risk Analysis

### Risk 1: False Positives (LOW)

**Scenario**: Legitimate code flagged as policy violation (e.g., comment containing "mcp.*consensus").

**Mitigation**:
- Exclude comments (`grep -v "^//"`)
- Exclude test files (`--exclude-dir=tests`)
- Manual review for edge cases (developer can bypass with `--no-verify` if needed)

**Likelihood**: Low (grep filters cover most cases)

---

### Risk 2: Policy Evolution (MEDIUM)

**Scenario**: Policy rules change (e.g., new storage system added), automated checks become outdated.

**Mitigation**:
- Document policy rules in MEMORY-POLICY.md (single source of truth)
- Update validation scripts when policy changes
- Quarterly policy review (ensure scripts align with policy)

**Likelihood**: Medium (policy will evolve, but manageable)

---

## 7. Open Questions

### Q1: Should policy checks be blocking or warnings?

**Context**: CI could fail (block merge) or warn (allow merge with alert).

**Decision**: BLOCKING - Policy violations should not merge to main. Critical for data integrity.

---

### Q2: Should pre-commit hook be mandatory or optional?

**Context**: Mandatory hooks can frustrate developers (slow commits), optional hooks may be ignored.

**Decision**: MANDATORY - All developers must install hooks via setup-hooks.sh. Provides fast feedback (<5s) before CI. Prevents wasted CI cycles on trivial policy violations.

---

## 8. Implementation Strategy

### Day 1: Storage Validation + CI (6h)
- **Hour 1-4**: Create validate_storage_policy.sh, test against violations
- **Hour 5-6**: CI integration, verify blocks PRs

### Day 2: Tag Schema + Dashboard + Hooks (4h)
- **Hour 1-2**: Create validate_tag_schema.sh
- **Hour 3**: Policy dashboard
- **Hour 4**: Pre-commit hook, documentation

**Total**: 10h (within 8-10h estimate, upper bound)

---

## 9. Deliverables

1. **Scripts**:
   - `scripts/validate_storage_policy.sh` - Storage separation validator
   - `scripts/validate_tag_schema.sh` - Tag schema validator
   - `scripts/policy_compliance_dashboard.sh` - Dashboard generator
   - `scripts/validate_memory_hygiene.sh` - Memory bloat prevention (future)

2. **CI Integration**:
   - `.github/workflows/ci.yml` - Policy validation job

3. **Hooks**:
   - `.githooks/pre-commit` - Mandatory fast feedback (<5s)
   - Updated README.md with installation requirement

4. **Documentation**:
   - `docs/policy-enforcement.md` - Validation rules, bypass instructions
   - `MEMORY-POLICY.md` - Updated with validation references

---

## 10. Validation Plan

### Script Tests (6 tests)
- Storage validator catches MCP consensus (positive)
- Storage validator passes on SQLite consensus (negative)
- Tag validator catches date tags (positive)
- Tag validator catches task ID tags (positive)
- Tag validator passes on namespaced tags (negative)
- Dashboard generates correct status

### CI Tests (3 tests)
- CI fails on storage violation
- CI fails on tag schema violation
- CI passes on compliant code

### Pre-Commit Tests (2 tests)
- Hook blocks commit on violation
- Hook allows commit on compliant code

**Total**: 11 tests

---

## 11. Conclusion

SPEC-941 automates policy compliance validation through CI checks, static analysis, and mandatory pre-commit hooks. **Estimated effort: 8-10 hours over 2 days.**

**Key Benefits**:
- ✅ Prevents SPEC-KIT-072 violations (automated detection)
- ✅ Catches regressions early (CI blocks policy breaks)
- ✅ Developer-friendly (clear errors, fix hints)
- ✅ Comprehensive coverage (storage, tags, importance)

**Next Steps**:
1. Review and approve SPEC-941
2. Schedule Day 1 (storage validation + CI)
3. Coordinate with SPEC-934 (validate fixes, prevent regression)
4. Document policy enforcement in MEMORY-POLICY.md
