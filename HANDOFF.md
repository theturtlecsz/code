# HANDOFF: SPEC-KIT-981/982 Implementation Progress

**Generated:** 2026-01-31
**Audience:** Next session
**Scope:** `codex-rs` / Spec-Kit config-driven agent mapping + unified prompt builder

***

## TL;DR (Current State)

**SPEC-KIT-981: Config-Driven Stage→Agent Mapping - 100% COMPLETE**

All 7 tasks finished:

* Config surface added (`[speckit.stage_agents]`)
* Defaults changed to GPT (gpt\_pro/gpt\_codex)
* GPT prompts added for clarify/analyze/checklist
* TUI/headless parity bug fixed (shared agent\_resolver)
* Guard test added

**SPEC-KIT-982: Unified Prompt-Vars Builder - 80% COMPLETE**

Core builder created with tests. Wiring into TUI/headless pending.

**Build Status:** `cargo check -p codex-tui` PASSES (74 warnings, 0 errors)

***

## What Was Implemented

### SPEC-KIT-981 (Complete)

1. **Config Types** (`codex-rs/core/src/config_types.rs:287-349`)
   * Added `SpecKitStageAgents` struct with fields for all 10 stages
   * Added `get_agent_for_stage()` method

2. **Config Parsing** (`codex-rs/core/src/config.rs`)
   * Added `SpecKitConfig` struct for `[speckit]` section
   * Added `speckit_stage_agents` field to `Config` struct
   * Updated 4 test fixtures with new field

3. **Agent Defaults** (`codex-rs/tui/src/chatwidget/spec_kit/gate_evaluation.rs:248-299`)
   * Updated `preferred_agent_for_stage()`:
     * All stages → GptPro (except Implement)
     * Implement → GptCodex
   * Added `agent_for_stage()` config-aware selector

4. **Prompts** (`docs/spec-kit/prompts.json`)
   * Added `gpt_pro` entries for: spec-clarify, spec-analyze, spec-checklist

5. **Parity Bug Fix** (NEW: `codex-rs/tui/src/chatwidget/spec_kit/agent_resolver.rs`)
   * Created shared `resolve_agent_config_name()` function
   * Uses canonical\_name fallback with error reporting
   * Updated `agent_orchestrator.rs` (lines \~515 and \~850)
   * Updated `headless/backend.rs` (line \~48)

6. **Config Examples**
   * `codex-rs/config.toml.example`: Added canonical\_name to GPT agents, added `[speckit.stage_agents]`
   * Root `config.toml.example`: Same updates

7. **Guard Test** (`codex-rs/tui/src/spec_prompts.rs:1322-1366`)
   * Added `all_stages_have_prompts_for_default_agents()` test

### SPEC-KIT-982 (Partial)

1. **Unified Builder** (NEW: `codex-rs/tui/src/chatwidget/spec_kit/prompt_vars.rs`)
   * Created `build_prompt_context()` function
   * Deterministic section order: Stage0 → Maieutic → ACE → spec/plan/tasks
   * Budget enforcement: ACE 4KB, Maieutic 4KB, per-file 20KB
   * ACE bullet deduplication and ID tracking
   * 7 unit tests included

**NOT YET DONE:**

* Wire `prompt_vars::build_prompt_context()` into `agent_orchestrator.rs`
* Wire into `headless/prompt_builder.rs`

### Documentation Updates

1. **codex-rs/SPEC.md** - Added SPEC-KIT-981/982/983 tracking
2. **docs/POLICY.md** - Added Section 2.8 "Stage→Agent Routing"

***

## Files Changed

```
Modified:
  codex-rs/SPEC.md
  codex-rs/core/src/config.rs
  codex-rs/core/src/config_types.rs
  codex-rs/tui/src/chatwidget/spec_kit/mod.rs
  codex-rs/tui/src/chatwidget/spec_kit/gate_evaluation.rs
  codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs
  codex-rs/tui/src/chatwidget/spec_kit/headless/backend.rs
  codex-rs/tui/src/spec_prompts.rs
  codex-rs/config.toml.example
  config.toml.example
  docs/spec-kit/prompts.json
  docs/POLICY.md

New:
  codex-rs/tui/src/chatwidget/spec_kit/agent_resolver.rs
  codex-rs/tui/src/chatwidget/spec_kit/prompt_vars.rs
```

***

## Remaining Work

1. **Wire unified builder into TUI** (`agent_orchestrator.rs`):
   * Refactor `build_individual_agent_prompt()` to call `prompt_vars::build_prompt_context()`
   * Return `(prompt, ace_bullet_ids)` tuple
   * Update callers to track `ace_bullet_ids_used`

2. **Wire unified builder into headless** (`headless/prompt_builder.rs`):
   * Refactor `build_headless_prompt()` to use unified builder
   * Add optional ACE client integration (safe fallback)

3. **Run validation suite**:
   ```bash
   cd codex-rs
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test -p codex-tui
   cargo test -p codex-cli --test speckit
   python3 scripts/doc_lint.py
   ```

4. **Commit** (if validation passes):
   ```
   feat(spec-kit): SPEC-KIT-981/982 config-driven stage agents + unified prompt builder

   - Add [speckit.stage_agents] config for per-stage agent override
   - Change defaults to GPT (gpt_pro for all, gpt_codex for implement)
   - Add gpt_pro prompts for clarify/analyze/checklist
   - Fix TUI/headless parity bug with shared agent_resolver
   - Create unified prompt_vars builder with ACE + maieutic sections
   ```

***

## Session Restart Prompt

Copy everything below the line into the first message of the next session:

***

Continue SPEC-KIT-981/982 implementation from HANDOFF.md

## Status

* **SPEC-KIT-981**: COMPLETE (config, defaults, prompts, parity fix, tests)
* **SPEC-KIT-982**: 80% - unified `prompt_vars.rs` builder created with 7 tests, wiring pending

## First, read:

* `HANDOFF.md` (shows exactly what was done)
* `codex-rs/tui/src/chatwidget/spec_kit/prompt_vars.rs` (new unified builder)
* `codex-rs/tui/src/chatwidget/spec_kit/agent_resolver.rs` (shared resolver)

## Remaining Tasks

1. **Wire unified builder into TUI `agent_orchestrator.rs`**:
   * In `build_individual_agent_prompt()`, call `prompt_vars::build_prompt_context()`
   * Pass maieutic\_spec and ace\_bullets (if available)
   * Return `(prompt, ace_bullet_ids)` to caller

2. **Wire unified builder into headless `prompt_builder.rs`**:
   * In `build_headless_prompt()`, call shared `prompt_vars::build_prompt_context()`
   * Handle ACE client safely (log and fallback if unavailable)

3. **Run validation**:
   ```bash
   cd codex-rs
   cargo fmt --all -- --check
   cargo test -p codex-tui
   python3 scripts/doc_lint.py
   ```

4. **If all passes, commit with**:
   ```
   feat(spec-kit): SPEC-KIT-981/982 config-driven stage agents + unified prompt builder
   ```

## Key Files

* `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (lines \~110-311 build\_individual\_agent\_prompt)
* `codex-rs/tui/src/chatwidget/spec_kit/headless/prompt_builder.rs` (lines \~34-192 build\_headless\_prompt)
* `codex-rs/tui/src/chatwidget/spec_kit/prompt_vars.rs` (new unified builder)

***

*Generated by Claude Code session 2026-01-31*
