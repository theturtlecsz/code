# PRD: SPEC-KIT-904 - Deprecate Manual Quality Commands

**Priority**: P2 (Low Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The spec-kit currently offers both manual and automatic quality commands, creating user confusion:

**Manual Commands** (explicit invocation):
- `/speckit.clarify SPEC-ID` - Run clarify gate independently
- `/speckit.analyze SPEC-ID` - Run analyze gate independently
- `/speckit.checklist SPEC-ID` - Run checklist gate independently

**Automatic Quality Gates** (run during `/speckit.auto`):
- BeforeSpecify checkpoint → clarify
- AfterSpecify checkpoint → checklist
- AfterTasks checkpoint → analyze

This duplication causes several issues:

1. **User Confusion**: "Which should I use? Are they different?"
2. **Inconsistent Behavior**: Manual commands just expand prompts, automatic gates run as checkpoints with different integration
3. **Maintenance Burden**: Two code paths for same functionality
4. **Documentation Complexity**: Need to explain when to use which approach
5. **Feature Redundancy**: `/speckit.auto` already provides comprehensive quality workflow

Since quality gates (T85) are now operational and run automatically, manual commands serve limited purpose except for debugging/iteration.

---

## Goals

### Primary Goal
Deprecate manual quality commands (`/speckit.clarify`, `/speckit.analyze`, `/speckit.checklist`) in favor of automatic quality gates, reducing user confusion and maintenance burden.

### Secondary Goals
- Simplify documentation (one quality workflow to explain)
- Reduce code paths (remove manual command implementations)
- Optional: Repurpose manual commands as "force re-run" for debugging
- Guide users toward `/speckit.auto` as canonical workflow

---

## Requirements

### Functional Requirements

1. **Deprecation Warnings**
   - Mark manual commands as deprecated in code (`#[deprecated]` attribute)
   - Display warning message when manual commands invoked
   - Suggest alternative: "Use /speckit.auto instead - quality gates run automatically"

2. **Backward Compatibility** (6-month grace period)
   - Manual commands continue to work (warning only)
   - Existing workflows not broken immediately
   - Deprecation notice in release notes

3. **Documentation Updates**
   - Remove manual commands from primary workflow documentation
   - Add deprecation notice to CLAUDE.md
   - Update tutorial/quickstart to use `/speckit.auto` only

4. **Option A: Full Deprecation** (Recommended)
   - Manual commands show deprecation warning → exit
   - Remove from help text after 6 months
   - Delete implementation code after 12 months

5. **Option B: Repurpose as Debug Commands** (Alternative)
   - Repurpose manual commands as "force re-run" for debugging
   - Example: `/speckit.clarify SPEC-ID` → Force re-run clarify gate independently
   - Useful for iterating on quality issues before full pipeline

### Non-Functional Requirements

1. **User Experience**
   - Clear migration path for users relying on manual commands
   - Helpful error messages explaining alternatives
   - Smooth transition (no abrupt breakage)

2. **Maintainability**
   - Reduce complexity by removing duplicate code paths (eventual goal)
   - Clear timeline for removal (6-12 months)

---

## Technical Approach

### Option A: Full Deprecation (Recommended)

```rust
// commands/quality.rs
#[deprecated(
    since = "2025-10-30",
    note = "Use /speckit.auto instead - quality gates run automatically. Manual quality commands will be removed in 6 months (April 2026)."
)]
pub struct SpecKitClarifyCommand;

impl SpecKitCommand for SpecKitClarifyCommand {
    fn execute(&self, widget: &mut ChatWidget, spec_id: String) {
        widget.append_markdown_cell(
            "⚠️ **Deprecated Command**\n\n\
             `/speckit.clarify` is deprecated and will be removed in April 2026.\n\n\
             **Use `/speckit.auto SPEC-ID` instead** - quality gates (clarify, checklist, analyze) run automatically during the pipeline.\n\n\
             For debugging individual quality issues, run `/speckit.auto SPEC-ID --from specify` to re-run from the specify stage with quality gates."
        );
    }
}

// Similar for SpecKitAnalyzeCommand, SpecKitChecklistCommand
```

### Option B: Repurpose as Debug Commands (Alternative)

```rust
// commands/quality.rs
pub struct SpecKitClarifyCommand;

impl SpecKitCommand for SpecKitClarifyCommand {
    fn execute(&self, widget: &mut ChatWidget, spec_id: String) {
        widget.append_markdown_cell(
            "🔧 **Debug Mode: Force Re-run Clarify Gate**\n\n\
             Running clarify quality gate independently for debugging.\n\n\
             Note: For normal workflow, use `/speckit.auto` - quality gates run automatically."
        );

        // Force re-run clarify gate
        quality_gate_handler::force_rerun_checkpoint(
            widget,
            &spec_id,
            QualityCheckpoint::BeforeSpecify,
        );
    }
}
```

### Documentation Updates

**CLAUDE.md** (before):
```markdown
**Quality Commands:**
- /speckit.clarify SPEC-ID – Structured ambiguity resolution
- /speckit.analyze SPEC-ID – Cross-artifact consistency checking
- /speckit.checklist SPEC-ID – Requirement quality scoring
```

**CLAUDE.md** (after):
```markdown
**Quality Commands (DEPRECATED):**
- /speckit.clarify SPEC-ID – **DEPRECATED**: Use `/speckit.auto` instead
- /speckit.analyze SPEC-ID – **DEPRECATED**: Use `/speckit.auto` instead
- /speckit.checklist SPEC-ID – **DEPRECATED**: Use `/speckit.auto` instead

**Note**: Quality gates (clarify, checklist, analyze) run automatically during `/speckit.auto` pipeline. Manual commands no longer needed.
```

### Help Text Updates

```rust
// slash_command.rs
impl SlashCommand {
    pub fn help_text(&self) -> &'static str {
        match self {
            Self::SpecKitClarify =>
                "⚠️ DEPRECATED: Use /speckit.auto - quality gates run automatically",
            Self::SpecKitAnalyze =>
                "⚠️ DEPRECATED: Use /speckit.auto - quality gates run automatically",
            Self::SpecKitChecklist =>
                "⚠️ DEPRECATED: Use /speckit.auto - quality gates run automatically",
            // ... other commands
        }
    }
}
```

---

## Acceptance Criteria

- [ ] Manual commands marked as `#[deprecated]` in code
- [ ] Deprecation warnings implemented (show message → exit or redirect)
- [ ] Help text updated to show deprecation status
- [ ] Documentation updated (CLAUDE.md, README, guides)
- [ ] Release notes include deprecation notice
- [ ] Migration guide provided (how to use `/speckit.auto` instead)
- [ ] Timeline established: Deprecation now → Warning period 6 months → Removal 12 months
- [ ] Option A vs Option B decision documented
- [ ] Unit tests updated (deprecation warnings tested)

---

## Out of Scope

- **Immediate removal**: Commands deprecated but functional for 6-12 months
- **Quality gate changes**: Focus is deprecation, not enhancing quality system
- **Alternative quality workflows**: `/speckit.auto` is the canonical workflow

---

## Success Metrics

1. **User Clarity**: 90% reduction in "which quality command to use" questions
2. **Adoption**: 100% of documentation recommends `/speckit.auto` only
3. **Code Simplification**: Remove 200-300 lines of duplicate quality command code (after 12 months)
4. **Migration**: <5% of users rely on manual commands after 6 months

---

## Dependencies

### Prerequisites
- Quality gates operational and stable (done ✅ 2025-10-29)
- `/speckit.auto` pipeline stable

### Downstream Dependencies
- Future quality enhancements will focus on automatic gates only
- Manual command removal enables further code simplification

---

## Estimated Effort

**2-3 hours** (as per architecture review)

**Breakdown**:
- Add deprecation warnings: 30 min
- Update documentation: 1 hour
- Update help text: 30 min
- Release notes: 30 min

---

## Priority

**P2 (Low Priority)** - Nice-to-have for clarity and simplification, but not blocking. Can defer until after higher-priority 30/60-day actions complete.

---

## Related Documents

- Architecture Review: Section "Incongruencies, Issue 10"
- `commands/quality.rs` - Manual command implementations
- `quality_gate_handler.rs` - Automatic quality gate system
- SPEC-KIT-085: Quality gates implementation (T85)
