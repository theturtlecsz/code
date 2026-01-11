# Implementation: Template Variable Substitution Fix

**SPEC-ID**: SPEC-KIT-924-template-variable-substitution  
**Status**: Implemented  
**Date**: 2025-11-11

## Problem Summary

Multi-agent consensus was producing synthesis files with unsubstituted template variables instead of actual agent metadata:

```json
{
  "prompt_version": "${PROMPT_VERSION}",  // ❌ Should be "20251028-plan-a"
  "model": "${MODEL_ID}",                 // ❌ Should be "gemini-2.5-pro"  
  "model_release": "${MODEL_RELEASE}",    // ❌ Should be "2025-05-14"
  "reasoning_mode": "${REASONING_MODE}"   // ❌ Should be "thinking"
}
```

## Root Cause

File: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs:277-282`

The `build_individual_agent_prompt` function was only replacing `${SPEC_ID}` and `${CONTEXT}`, but not the metadata template variables:

```rust
// OLD CODE (BROKEN):
let prompt = prompt_template
    .replace("${SPEC_ID}", spec_id)
    .replace("${CONTEXT}", &context);
```

## Solution

### Changes Made

**1. Added SpecAgent import** (`agent_orchestrator.rs:22`):
```rust
use crate::spec_prompts::{SpecStage, SpecAgent};
```

**2. Made model_metadata public** (`spec_prompts.rs:227`):
```rust
pub fn model_metadata(stage: SpecStage, agent: SpecAgent) -> Vec<(String, String)>
```

**3. Fixed substitution logic** (`agent_orchestrator.rs:277-310`):
```rust
// SPEC-KIT-924: Replace ALL template variables including metadata
// Parse agent name to SpecAgent enum to get metadata
let spec_agent = SpecAgent::from_string(agent_name)
    .ok_or_else(|| format!("Unknown agent name: {}", agent_name))?;

// Get prompt version
let prompt_version = crate::spec_prompts::stage_version_enum(stage)
    .unwrap_or_else(|| "unversioned".to_string());

// Get model metadata (MODEL_ID, MODEL_RELEASE, REASONING_MODE)
let metadata = crate::spec_prompts::model_metadata(stage, spec_agent);
let model_id = metadata.iter()
    .find(|(k, _)| k == "MODEL_ID")
    .map(|(_, v)| v.as_str())
    .unwrap_or("unknown");
let model_release = metadata.iter()
    .find(|(k, _)| k == "MODEL_RELEASE")
    .map(|(_, v)| v.as_str())
    .unwrap_or("unknown");
let reasoning_mode = metadata.iter()
    .find(|(k, _)| k == "REASONING_MODE")
    .map(|(_, v)| v.as_str())
    .unwrap_or("unknown");

// Replace all placeholders (including metadata variables)
let prompt = prompt_template
    .replace("${SPEC_ID}", spec_id)
    .replace("${CONTEXT}", &context)
    .replace("${PROMPT_VERSION}", &prompt_version)
    .replace("${MODEL_ID}", model_id)
    .replace("${MODEL_RELEASE}", model_release)
    .replace("${REASONING_MODE}", reasoning_mode);
```

## Expected Values (by agent and stage)

Based on `spec_prompts.rs:228-240`:

### Gemini
- **Plan/Implement/Validate/Audit**: 
  - MODEL_ID: `gemini-2.5-pro`
  - MODEL_RELEASE: `2025-05-14`
  - REASONING_MODE: `thinking`
- **Tasks/Unlock**: 
  - MODEL_ID: `gemini-2.5-flash`
  - MODEL_RELEASE: `2025-05-14`
  - REASONING_MODE: `fast`

### Claude
- **All stages**:
  - MODEL_ID: `claude-4.5-sonnet`
  - MODEL_RELEASE: `2025-09-29`
  - REASONING_MODE: `balanced`

### Code (Claude Code)
- **All stages**:
  - MODEL_ID: `claude-sonnet-4-5`
  - MODEL_RELEASE: `2025-10-22`
  - REASONING_MODE: `extended`

### GPT Codex
- **All stages**:
  - MODEL_ID: `gpt-5-codex`
  - MODEL_RELEASE: `2025-09-29`
  - REASONING_MODE: `auto`

### GPT Pro
- **All stages**:
  - MODEL_ID: `gpt-5`
  - MODEL_RELEASE: `2025-08-06`
  - REASONING_MODE: `high`

## Testing Instructions

### 1. Clean State
```bash
rm -f docs/SPEC-KIT-900/{plan,tasks,implement,validate,audit,unlock}.md
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/consensus/SPEC-KIT-900
rm -rf docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SPEC-KIT-900
tmux kill-session -t agents-gemini -t agents-claude -t agents-code 2>/dev/null
```

### 2. Run Test
```bash
export SPEC_KIT_OBSERVABLE_AGENTS=1
./target/dev-fast/code
# In TUI: /speckit.plan SPEC-KIT-900
```

### 3. Verify Success

**Check plan.md has proper content:**
```bash
cat docs/SPEC-KIT-900/plan.md | head -50
```

Expected output should show:
```json
{
  "stage": "spec-plan",
  "prompt_version": "20251028-plan-a",  // ✅ Not ${PROMPT_VERSION}
  "agent": "gemini",
  "model": "gemini-2.5-pro",            // ✅ Not ${MODEL_ID}
  "model_release": "2025-05-14",        // ✅ Not ${MODEL_RELEASE}
  "reasoning_mode": "thinking",         // ✅ Not ${REASONING_MODE}
  "research_summary": [ ... ],
  "questions": [ ... ]
}
```

**Check for template variables (should be empty):**
```bash
grep '${' docs/SPEC-KIT-900/plan.md
# Should return nothing
```

**Check file size (should be >5KB):**
```bash
wc -c docs/SPEC-KIT-900/plan.md
# Should show >5000 bytes (not ~1.5KB)
```

## Files Modified

1. `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` - Fixed substitution logic
2. `codex-rs/tui/src/spec_prompts.rs` - Made model_metadata public

## Success Criteria

- ✅ All `${...}` template variables replaced with actual values
- ✅ plan.md >5KB with work breakdown, risks, consensus
- ✅ tasks.md >5KB with task list and acceptance coverage  
- ✅ implement.md >5KB with code diffs and rationale
- ✅ No empty agent content sections
- ✅ Proper metadata values for each agent type

## Notes

- This fix leverages existing infrastructure in `spec_prompts.rs` that was already being used for the bundled prompt generation but not for individual agent prompts
- The `model_metadata` function provides environment variable override support (e.g., `SPECKIT_MODEL_ID_GEMINI_SPEC_PLAN`) for testing different model configurations
- Template variable `${CONTEXT}` still gets the full SPEC context (spec.md, plan.md, tasks.md) as before
