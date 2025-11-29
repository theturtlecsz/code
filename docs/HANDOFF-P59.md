# P59 SYNC CONTINUATION PROMPT

**Generated**: 2025-11-29
**Previous Session**: P58 (SPEC-KIT-957 + SPEC-KIT-960 complete)
**Model**: Claude Opus 4.5 (ultrathink recommended)

---

## Session Summary from P58

### Completed
1. **SPEC-KIT-957**: Nativized `speckit.specify` - ALL speckit.* commands now use native Rust routing
2. **SPEC-KIT-960**: Created `/speckit.project` command with complete templates
3. **Template Fix**: Added missing CLAUDE.md Section 1 required files:
   - `product-requirements.md` (PRD structure)
   - `PLANNING.md` (architecture, type-specific build commands)
   - `templates/PRD-template.md` and `spec-template.md`

### Commits Pushed
```
1cdb575ce - feat(spec-kit): Nativize speckit.specify (SPEC-KIT-957)
789f06b7a - docs: Update SPEC.md and CLAUDE.md for SPEC-KIT-957
0316aa037 - feat(spec-kit): Add /speckit.project command (SPEC-KIT-960)
62db41efa - docs: Mark SPEC-KIT-960 as Done
55ca08937 - fix(spec-kit): Complete /speckit.project templates (SPEC-KIT-960)
```

---

## Session Goals (Priority Order)

### 1. TUI Manual Testing - /speckit.project (15 min)
Test all 4 template types in the TUI to verify end-to-end functionality:

```bash
# Build and run TUI
~/code/build-fast.sh run

# Inside TUI, test each template type:
/speckit.project rust test-rust-lib
/speckit.project python test-py-app
/speckit.project typescript test-ts-lib
/speckit.project generic test-minimal

# Verify created structures
ls -la test-rust-lib/
cat test-rust-lib/product-requirements.md
cat test-rust-lib/PLANNING.md
ls -la test-rust-lib/templates/

# Cleanup after testing
rm -rf test-rust-lib test-py-app test-ts-lib test-minimal
```

**Success Criteria:**
- [ ] All 4 commands execute without error
- [ ] Output shows correct file counts and paths
- [ ] Created files have correct content
- [ ] Templates directory includes PRD-template.md and spec-template.md

### 2. Create SPEC-KIT-961: Template Structure Analysis (ultrathink)

**Objective**: Create a SPEC that:
1. Documents the complete template ecosystem
2. Analyzes current template structure for consistency
3. Ensures all LLM agents (gemini, claude, code) are aware of templates on session start
4. Integrates template context into agent prompts

**Use `/speckit.new` to create:**
```
/speckit.new Analyze and document the spec-kit template ecosystem. Create comprehensive
template documentation that ensures LLM agents (gemini, claude, code) understand template
structure, variable substitution, and usage patterns. Include: (1) Template inventory and
purpose mapping, (2) Variable substitution schema, (3) Agent prompt integration for
template awareness, (4) Consistency validation between templates and prompts.json.
```

**PRD Should Cover:**

#### Functional Requirements
1. **Template Inventory** - Document all templates in `~/code/templates/`:
   - PRD-template.md, spec-template.md (intake)
   - plan-template.md, tasks-template.md (development)
   - implement-template.md, validate-template.md (execution)
   - audit-template.md, unlock-template.md (approval)
   - clarify-template.md, analyze-template.md, checklist-template.md (quality)

2. **Variable Schema** - Document all `[PLACEHOLDER]` variables:
   - Which templates use which variables
   - How variables are substituted (see `new_native.rs:102-230`)
   - Default values and fallbacks

3. **Agent Awareness Integration** - Ensure agents know templates:
   - Add template summary to `prompts.json` stage prompts
   - Consider adding to CLAUDE.md Section 1 "Load These References"
   - Verify ACE playbook has template bullets

4. **Consistency Validation** - Cross-check:
   - Templates match prompts.json structure expectations
   - Variable names consistent across templates
   - `/speckit.project` templates match main templates

#### Analysis Tasks (Multi-Agent Consensus)
- **Gemini**: Research template best practices, analyze structure
- **Claude**: Synthesize findings, propose improvements
- **Code**: Validate against codebase, check variable usage

### 3. Add Go Template to /speckit.project (30 min)

Add Go project scaffolding:

**Files to create:**
```go
// go.mod
module [PROJECT_NAME]

go 1.22

// main.go
package main

func main() {
    println("Hello from [PROJECT_NAME]!")
}

// main_test.go
package main

import "testing"

func TestMain(t *testing.T) {
    // Add tests here
}
```

**Implementation:**
1. Add `ProjectType::Go` variant to enum
2. Add `create_go_files()` function
3. Update PLANNING.md to include Go-specific build commands
4. Add test case `test_create_go_project()`

### 4. Add Full Templates Directory (45 min)

Copy all 11 templates from `~/code/templates/` to scaffolded projects:

**Current in ~/code/templates/:**
```
PRD-template.md        # Already included
spec-template.md       # Already included
plan-template.md       # NEW - add
tasks-template.md      # NEW - add
implement-template.md  # NEW - add
validate-template.md   # NEW - add
audit-template.md      # NEW - add
unlock-template.md     # NEW - add
clarify-template.md    # NEW - add
analyze-template.md    # NEW - add
checklist-template.md  # NEW - add
```

**Implementation:**
1. Read existing templates from `~/code/templates/`
2. Add to `create_templates_dir()` function
3. Update test to verify all 11 templates exist

### 5. Documentation Updates (30 min)

#### Update CLAUDE.md Section 2
Add `/speckit.project` to the command listing:

```markdown
**Intake & Creation:**
- `/speckit.new <description>` – Native SPEC creation (Tier 0)
- `/speckit.specify SPEC-ID` – PRD refinement (Tier 1)
- `/speckit.project <type> <name>` – **NEW** Scaffold project with spec-kit infrastructure (Tier 0: instant, $0)
  - Types: `rust`, `python`, `typescript`, `generic`, `go`
  - Creates: CLAUDE.md, SPEC.md, product-requirements.md, PLANNING.md, templates/, docs/, memory/
```

#### Update PRD for SPEC-KIT-960
Add completion notes with final file list.

---

## Key Files Reference

### Existing Files to Modify
```
codex-rs/tui/src/chatwidget/spec_kit/project_native.rs  # Add Go, full templates
CLAUDE.md                                                # Add /speckit.project docs
docs/SPEC-KIT-960-speckit-project/PRD.md                # Update with completion
```

### Reference Files
```
~/code/templates/                                        # Source for full templates
codex-rs/tui/src/chatwidget/spec_kit/new_native.rs      # Variable substitution patterns
docs/spec-kit/prompts.json                               # Prompt definitions
```

---

## Verification Commands

```bash
# Build
~/code/build-fast.sh

# Run tests
cd ~/code/codex-rs && cargo test -p codex-tui project_native

# Test TUI manually
~/code/build-fast.sh run

# Check template inventory
ls -la ~/code/templates/
wc -l ~/code/templates/*.md
```

---

## Success Criteria

### TUI Testing:
- [ ] All 4 template types create projects without error
- [ ] Output displays correctly in TUI
- [ ] Files have correct content

### SPEC-KIT-961 Created:
- [ ] PRD covers template ecosystem documentation
- [ ] Includes multi-agent analysis plan
- [ ] Addresses agent prompt integration

### Go Template:
- [ ] `ProjectType::Go` added
- [ ] go.mod, main.go, main_test.go created
- [ ] Test passes

### Full Templates:
- [ ] All 11 templates copied to scaffolded projects
- [ ] Test verifies all templates exist

### Documentation:
- [ ] CLAUDE.md Section 2 updated with /speckit.project
- [ ] PRD updated with completion notes

---

## Context Preservation

### Architecture Decisions
- `/speckit.project` is Tier 0 (native Rust, $0, <1s)
- Templates embedded in binary, not external files
- CLAUDE.md Section 1 compliance drives file requirements

### Template Ecosystem Understanding
- `~/code/templates/` contains 11 stage templates
- `/speckit.new` uses templates for PRD generation (`new_native.rs`)
- Variable substitution follows `[PLACEHOLDER]` pattern
- `prompts.json` defines agent prompts per stage

### Multi-Agent Template Awareness
The user wants LLM agents to understand templates on session start:
- Consider adding to CLAUDE.md Section 1 references
- Consider adding template bullets to ACE playbook
- Consider adding template overview to prompts.json

---

## Start Commands

```bash
# 1. Load this handoff
cat docs/HANDOFF-P59.md

# 2. Verify current state
git status
~/code/build-fast.sh

# 3. Start TUI testing
~/code/build-fast.sh run

# 4. Create SPEC-KIT-961
/speckit.new [description from above]

# 5. Implement Go template
# 6. Add full templates
# 7. Update documentation
```

---

**Estimated Session Time**: 3-4 hours
**Priority**: Documentation + Template ecosystem analysis
