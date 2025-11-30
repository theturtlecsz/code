# Implementation Plan: Personalization, Proactivity & Precision

**SPEC-ID:** SPEC-KIT-PPP
**Version:** 1.0.0
**Status:** Draft
**Created:** 2025-11-30

---

## Inputs

- **Spec:** docs/SPEC-KIT-PPP-integration/spec.md (v1.0.0)
- **PRD:** docs/SPEC-KIT-PPP-integration/PRD.md (v1.0.0)
- **Constitution:** memory/constitution.md

---

## Phase 1: Configuration Foundation

**Goal:** Add `[personalization]` config support with full CLI override capability.

### Work Breakdown

1. **Define core types** (`core/src/config.rs`)
   - Add `Verbosity` enum
   - Add `ProactivityLevel` enum
   - Add `PersonalizationToml` struct

2. **Extend ConfigToml** (`core/src/config.rs:1540-1566`)
   - Add `pub personalization: Option<PersonalizationToml>` field
   - Add `#[serde(default)]` annotation

3. **Extend ConfigOverrides** (`core/src/config.rs:1710-1730`)
   - Add `personalization_language`, `personalization_verbosity`, `personalization_proactivity`
   - Update destructure pattern in `load_from_base_config_with_overrides()`

4. **Add CLI argument parsing** (`tui/src/cli.rs` or `cli/src/main.rs`)
   - `--lang <code>`
   - `--verbosity <level>` / `--terse`
   - `--proactivity <level>` / `--ask-first`

5. **Update config.toml.example**
   - Document all new options with examples

### Risk Assessment

| Step | Risk Level | Risk Factor | Mitigation |
|------|------------|-------------|------------|
| 1-2 | **LOW** | Type additions are additive | Unit tests for serde |
| 3 | **MEDIUM** | ConfigOverrides destructure requires all fields | Full destructure pattern match |
| 4 | **LOW** | CLI parsing is isolated | Integration tests |
| 5 | **LOW** | Documentation only | Review |

---

## Phase 2: Prompt Injection Logic

**Goal:** Inject personalization instructions into agent prompts.

### Work Breakdown

1. **Create prompt modifier functions**
   - `Verbosity::prompt_instruction() -> &'static str`
   - `ProactivityLevel::prompt_instruction() -> &'static str`

2. **Identify prompt composition point** (`core/src/codex.rs`)
   - Locate `Prompt` struct construction (~line 4534)
   - Find `user_instructions` concatenation

3. **Inject personalization into base instructions**
   - Append verbosity instruction
   - Append proactivity instruction
   - Inject language preference if set

4. **Add tests for prompt injection**
   - Verify instructions appear in composed prompt
   - Verify language preference works

### Risk Assessment

| Step | Risk Level | Risk Factor | Mitigation |
|------|------------|-------------|------------|
| 1 | **LOW** | Pure functions | Unit tests |
| 2-3 | **MEDIUM** | Touching codex.rs (large file) | Minimal diff, feature flag |
| 4 | **LOW** | Test coverage | Integration tests |

---

## Phase 3: Vagueness Check Middleware

**Goal:** Intercept vague prompts and request clarification before agent execution.

### Work Breakdown

1. **Create vagueness module** (`core/src/vagueness.rs`) **[HIGH RISK]**
   - `VaguenessCheckResult` struct
   - `analyze_vagueness(prompt: &str, threshold: f64) -> VaguenessCheckResult`
   - Heuristic-based analysis (keyword patterns, question marks, specificity)

2. **Add ClarificationNeeded event** (`core/src/event.rs`)
   - New event variant with questions and context

3. **Hook into submission loop** (`core/src/codex.rs:3054`) **[HIGH RISK]**
   - Before `run_turn()` call
   - Check `config.personalization.check_vagueness`
   - If vague, emit `ClarificationNeeded` and skip turn

4. **Handle ClarificationNeeded in TUI**
   - Display questions to user
   - Queue user response as continuation

5. **Add feature flag**
   - `CODEX_ENABLE_VAGUENESS_CHECK=1` environment variable
   - Default OFF until stable

### Risk Assessment

| Step | Risk Level | Risk Factor | Mitigation |
|------|------------|-------------|------------|
| 1 | **MEDIUM** | NLP heuristics prone to false positives | Configurable threshold, extensive testing |
| 2 | **LOW** | Enum variant addition | Additive change |
| 3 | **HIGH** | Core submission loop modification | Feature flag, extensive testing, rollback plan |
| 4 | **MEDIUM** | TUI event handling | Separate PR, feature flag |
| 5 | **LOW** | Safety mechanism | Required before merge |

---

## Phase 4: Consensus & Telemetry Integration

**Goal:** Add interaction scoring to consensus system and non-blocking MCP logging.

### Work Breakdown

1. **Extend ConsensusVerdict** (`tui/src/chatwidget/spec_kit/consensus.rs:49-71`)
   - Add `interaction_score: Option<f64>`
   - Add `score_breakdown: Option<InteractionScoreBreakdown>`

2. **Compute interaction scores**
   - Token efficiency metric
   - Response time metric
   - User signal extraction (regenerates, edits, stops)

3. **Add MCP logger hook** (`core/src/codex.rs:5011-5030`) **[MEDIUM RISK]**
   - In `ResponseEvent::Completed` handler
   - `tokio::spawn` fire-and-forget call
   - Graceful degradation if MCP unavailable

4. **SQLite schema decision**
   - Option A: Store in `synthesis_json` (no migration) - **PREFERRED**
   - Option B: Add column via migration_v3

5. **Create interaction-logger MCP tool spec**
   - Input: response_id, tokens, duration, user_signals
   - Output: confirmation

### Risk Assessment

| Step | Risk Level | Risk Factor | Mitigation |
|------|------------|-------------|------------|
| 1-2 | **LOW** | Additive struct changes | Unit tests |
| 3 | **MEDIUM** | Touching response handler | Fire-and-forget pattern, error isolation |
| 4A | **LOW** | JSON storage, no schema change | Preferred approach |
| 4B | **MEDIUM** | Schema migration | Full testing, rollback migration |
| 5 | **LOW** | External tool spec | Documentation |

---

## Acceptance Mapping

| Requirement (PRD) | Validation Step | Test/Check Artifact |
|-------------------|-----------------|---------------------|
| Verbosity config works | Unit test: terse/balanced/verbose | `tests/personalization_config.rs` |
| CLI overrides work | Integration test: `--lang it` | `tests/cli_overrides.rs` |
| Vagueness check fires | Integration test: vague prompt | `tests/vagueness_middleware.rs` |
| ClarificationNeeded event | TUI test: event handling | `tests/tui_clarification.rs` |
| Interaction scoring | Consensus test: score present | `tests/consensus_scoring.rs` |
| MCP logging non-blocking | Performance test: no latency | `tests/mcp_logger_perf.rs` |

---

## Risks & Unknowns

### Technical Risks

1. **Vagueness false positives** - Users may find frequent clarification requests annoying
   - Mitigation: Threshold tuning, "never ask again for this session" option

2. **ConfigOverrides exhaustive match** - Adding fields requires updating destructure
   - Mitigation: Compiler will catch, but adds maintenance burden

3. **codex.rs complexity** - File is ~12k LOC, changes are risky
   - Mitigation: Minimal changes, feature flags, thorough testing

### Process Risks

1. **Rollback complexity** - Multiple files touched across phases
   - Mitigation: One PR per phase, feature flags

2. **Testing coverage gaps** - Hard to test UX improvements quantitatively
   - Mitigation: A/B telemetry, user feedback collection

---

## Consensus & Risks (Multi-AI)

*To be populated after `/speckit.plan SPEC-KIT-PPP` execution*

- **Agreement:** TBD
- **Disagreement & resolution:** TBD

---

## Exit Criteria (Done)

- [ ] All acceptance checks pass
- [ ] `cargo fmt --all` passes
- [ ] `cargo clippy --workspace` passes
- [ ] `cargo test --workspace` passes
- [ ] Feature flags documented
- [ ] config.toml.example updated
- [ ] CLAUDE.md updated with new CLI flags
- [ ] PR prepared with phase-by-phase commits

---

## Rollback Plan

### Phase 1 Rollback
- Revert config.rs changes
- No data migration needed

### Phase 2 Rollback
- Revert prompt injection code
- Config fields remain (harmless)

### Phase 3 Rollback
- Set `CODEX_ENABLE_VAGUENESS_CHECK=0` (immediate)
- Revert middleware code (full rollback)

### Phase 4 Rollback
- MCP logger: Just remove spawn (non-blocking, no side effects)
- Consensus score: Field is `Option<f64>`, existing data unaffected
