# P67 SYNC CONTINUATION PROMPT

**Previous Session**: P66
**Commit**: `a79573561` - fix(spec-kit): Wire missing slash commands + UX improvements
**Date**: 2025-11-30

---

## Session P66 Summary

**Completed:**
- SPEC-KIT-961 marked Done in SPEC.md
- Created `docs/spec-kit/HERMETIC-ISOLATION.md` (architecture doc)
- Updated `docs/spec-kit/COMMAND_INVENTORY.md` (22→23 commands, 11→14 templates)
- **Fixed 4 missing SlashCommand enum variants:**
  - `SpecKitProject` (/speckit.project)
  - `SpecKitVerify` (/speckit.verify)
  - `SpecKitInstallTemplates` (/speckit.install-templates)
  - `SpecKitTemplateStatus` (/speckit.template-status)
- **UX fix**: /speckit.project now auto-switches to created directory
- **Bug fix**: SPEC slug truncation to 60 chars (prevents "filename too long")

**Deferred:**
- Ferris-clone benchmark test (setup complete, test not run)

---

## P67 Task List (Prioritized)

### Priority 1: Research Interactive PRD Builder (60 min)

**Goal**: Research upstream spec-kit implementation and design interactive Q&A for /speckit.new

**Context**: Current /speckit.new takes a description string and generates PRD instantly. The upstream version has an interactive Q&A flow that builds a more comprehensive PRD through guided questions.

**Research tasks:**
1. Web search for "spec-kit interactive PRD" and similar
2. Analyze what questions should be asked (scope, constraints, acceptance criteria, etc.)
3. Document findings in `docs/SPEC-KIT-970-interactive-prd/research.md`

**Design decisions needed:**
- Modal dialog vs inline Q&A in chat
- Required vs optional questions
- How to handle "skip" for optional questions
- Whether to support editing answers before generation

### Priority 2: Create SPEC for Interactive PRD (30 min)

```
/speckit.new Interactive PRD builder with guided Q&A flow for /speckit.new command
```

**Expected SPEC-ID**: SPEC-KIT-970 (or next available)

### Priority 3 (Optional): Ferris-Clone Benchmark

**Setup already complete:**
- Reference: `/home/thetu/benchmark/reference/ferris-says`
- Project scaffold: `/home/thetu/benchmark/ferris-clone` (has CLAUDE.md, AGENTS.md, etc.)

**To run:**
```bash
cd /home/thetu/benchmark/ferris-clone
/home/thetu/code/build-fast.sh run

# In TUI:
/speckit.new ferris-clone - Rust cowsay with Ferris ASCII art and text wrapping
/speckit.auto SPEC-KIT-XXX
```

---

## Environment State

```bash
# Verify state
git log --oneline -1
# → a79573561 fix(spec-kit): Wire missing slash commands...

# Build verification
~/code/build-fast.sh

# Test new commands work
cd /tmp && ~/code/build-fast.sh run
# In TUI: /speckit.project rust test-project
# Should auto-cd to /tmp/test-project
```

---

## Known Issues / Notes

1. **Registry vs Enum duplication**: Commands exist in both `SPEC_KIT_REGISTRY` and `SlashCommand` enum. Registry was intended to replace enum but routing wasn't implemented. Current fix adds to enum. Consider consolidating in future.

2. **Benchmark reference project**: ferris-says cloned to `/home/thetu/benchmark/reference/ferris-says`

3. **Pre-commit hooks**: Working correctly, validating config isolation and storage policy.

---

## Copy-Paste Continuation Prompt

```
load docs/HANDOFF-P67.md

Begin P67. Execute in order:
1. Web research: upstream spec-kit interactive PRD implementation
2. Document findings in docs/SPEC-KIT-970-interactive-prd/research.md
3. Create SPEC: /speckit.new Interactive PRD builder with guided Q&A
4. Design the Q&A flow (questions, modal vs inline, required vs optional)
5. If time permits, start implementation planning

Track progress with TodoWrite.
```
