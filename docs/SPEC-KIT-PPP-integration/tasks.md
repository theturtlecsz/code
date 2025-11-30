# Tasks: Personalization, Proactivity & Precision

**SPEC-ID:** SPEC-KIT-PPP
**Status:** Draft
**Created:** 2025-11-30

---

## Engineering Tickets

> **Note:** This is a skeleton. Tasks will be populated during `/speckit.tasks SPEC-KIT-PPP`.

---

### Task 1: PPP-001 - Core Type Definitions

**Phase:** 1 (Config)
**Priority:** P0 (Blocker)
**Estimated Effort:** Small

**Description:**
Define `Verbosity`, `ProactivityLevel`, and `PersonalizationToml` structs in `codex-rs/core/src/config.rs`.

**Files to Touch:**
- `codex-rs/core/src/config.rs` (lines 1407-1600)

**Acceptance Criteria:**
- [ ] `Verbosity` enum with Terse/Balanced/Verbose variants
- [ ] `ProactivityLevel` enum with AskFirst/Suggest/Autonomous variants
- [ ] `PersonalizationToml` struct with all fields from spec
- [ ] Serde derive macros with proper defaults
- [ ] Unit tests for serialization/deserialization

---

### Task 2: PPP-002 - ConfigToml Extension

**Phase:** 1 (Config)
**Priority:** P0 (Blocker)
**Estimated Effort:** Small
**Depends On:** PPP-001

**Description:**
Add `personalization` field to `ConfigToml` struct and ensure backward compatibility.

**Files to Touch:**
- `codex-rs/core/src/config.rs` (line ~1541)

**Acceptance Criteria:**
- [ ] `pub personalization: Option<PersonalizationToml>` added
- [ ] `#[serde(default)]` annotation present
- [ ] Existing configs without `[personalization]` still parse
- [ ] Integration test with sample config

---

### Task 3: PPP-003 - ConfigOverrides Extension

**Phase:** 1 (Config)
**Priority:** P0 (Blocker)
**Estimated Effort:** Medium
**Depends On:** PPP-001

**Description:**
Extend `ConfigOverrides` for CLI flag support and update the exhaustive destructure pattern.

**Files to Touch:**
- `codex-rs/core/src/config.rs` (lines 1710-1800)

**Acceptance Criteria:**
- [ ] `personalization_language: Option<String>` added
- [ ] `personalization_verbosity: Option<Verbosity>` added
- [ ] `personalization_proactivity: Option<ProactivityLevel>` added
- [ ] Destructure pattern updated in `load_from_base_config_with_overrides()`
- [ ] Override logic merges CLI > TOML correctly

---

### Task 4: PPP-004 - CLI Argument Parsing

**Phase:** 1 (Config)
**Priority:** P1 (High)
**Estimated Effort:** Medium
**Depends On:** PPP-003

**Description:**
Add CLI flags for personalization: `--lang`, `--verbosity`, `--terse`, `--proactivity`, `--ask-first`.

**Files to Touch:**
- `codex-rs/tui/src/cli.rs` or `codex-rs/cli/src/main.rs`
- Help text/documentation

**Acceptance Criteria:**
- [ ] `--lang <code>` sets language
- [ ] `--verbosity terse|balanced|verbose` sets verbosity
- [ ] `--terse` shorthand for `--verbosity terse`
- [ ] `--proactivity ask_first|suggest|autonomous` sets level
- [ ] `--ask-first` shorthand for `--proactivity ask_first`
- [ ] Help text documents all flags

---

### Task 5: PPP-005 - Prompt Injection Logic

**Phase:** 2 (Logic)
**Priority:** P1 (High)
**Estimated Effort:** Medium
**Depends On:** PPP-002

**Description:**
Inject personalization instructions into agent prompts based on config.

**Files to Touch:**
- `codex-rs/core/src/codex.rs` (Prompt composition, ~line 4534)
- Potentially `core/src/prompt.rs` if exists

**Acceptance Criteria:**
- [ ] `Verbosity::prompt_instruction()` implemented
- [ ] `ProactivityLevel::prompt_instruction()` implemented
- [ ] Instructions injected into base_instructions
- [ ] Language preference appended if set
- [ ] Integration test verifies prompt contains instructions

---

### Task 6: PPP-006 - Vagueness Check Module

**Phase:** 3 (Consensus)
**Priority:** P2 (Medium)
**Estimated Effort:** Large
**Risk Level:** HIGH

**Description:**
Create vagueness analysis module with heuristic-based clarity scoring.

**Files to Touch:**
- `codex-rs/core/src/vagueness.rs` (NEW)
- `codex-rs/core/src/lib.rs` (module export)

**Acceptance Criteria:**
- [ ] `VaguenessCheckResult` struct defined
- [ ] `analyze_vagueness(prompt, threshold)` implemented
- [ ] Heuristic patterns: keyword density, specificity markers, question marks
- [ ] Threshold configuration working
- [ ] Unit tests for various prompt types
- [ ] False positive rate < 10% on test corpus

---

### Task 7: PPP-007 - Vagueness Middleware Integration

**Phase:** 3 (Consensus)
**Priority:** P2 (Medium)
**Estimated Effort:** Large
**Depends On:** PPP-006
**Risk Level:** HIGH

**Description:**
Hook vagueness check into submission loop with ClarificationNeeded event.

**Files to Touch:**
- `codex-rs/core/src/codex.rs` (submission loop, ~line 3054)
- `codex-rs/core/src/event.rs` (new event variant)
- `codex-rs/tui/src/chatwidget.rs` (event handling)

**Acceptance Criteria:**
- [ ] `ClarificationNeeded` event variant added
- [ ] Vagueness check called before `run_turn()` when enabled
- [ ] Feature flag `CODEX_ENABLE_VAGUENESS_CHECK` controls activation
- [ ] TUI displays clarification questions
- [ ] User can respond and continue
- [ ] Default is OFF (opt-in)

---

### Task 8: PPP-008 - Interaction Scoring & MCP Logging

**Phase:** 4 (Consensus)
**Priority:** P3 (Low)
**Estimated Effort:** Medium
**Depends On:** PPP-005

**Description:**
Add interaction scoring to ConsensusVerdict and fire-and-forget MCP logging.

**Files to Touch:**
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` (lines 49-71)
- `codex-rs/core/src/codex.rs` (ResponseEvent::Completed, ~line 5011)

**Acceptance Criteria:**
- [ ] `interaction_score: Option<f64>` added to ConsensusVerdict
- [ ] `score_breakdown` struct defined
- [ ] MCP logger call in Completed handler
- [ ] `tokio::spawn` ensures non-blocking
- [ ] Graceful degradation if MCP unavailable
- [ ] No UI latency increase (perf test)

---

## Task Dependency Graph

```
PPP-001 (Types)
    │
    ├──▶ PPP-002 (ConfigToml)
    │        │
    │        └──▶ PPP-005 (Prompt Injection)
    │                 │
    │                 └──▶ PPP-008 (Scoring/Logging)
    │
    └──▶ PPP-003 (ConfigOverrides)
             │
             └──▶ PPP-004 (CLI Args)

PPP-006 (Vagueness Module) ──▶ PPP-007 (Middleware) [HIGH RISK]
```

---

## Risk Summary by Task

| Task | Risk | Risk Factor |
|------|------|-------------|
| PPP-001 | LOW | Additive types |
| PPP-002 | LOW | Additive field |
| PPP-003 | MEDIUM | Exhaustive destructure |
| PPP-004 | LOW | Isolated CLI |
| PPP-005 | MEDIUM | codex.rs touch |
| PPP-006 | **HIGH** | NLP heuristics, false positives |
| PPP-007 | **HIGH** | Core loop modification |
| PPP-008 | MEDIUM | Response handler modification |

---

## Notes

- Tasks PPP-001 through PPP-004 can be completed in a single PR (Phase 1)
- Task PPP-005 should be a separate PR with careful review
- Tasks PPP-006 and PPP-007 require feature flag and extensive testing before merge
- Task PPP-008 can be done independently after Phase 2
