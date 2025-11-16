# PPP Framework Integration Feasibility Analysis
**Project**: theturtlecsz/code CLI
**Analysis Date**: 2025-11-16
**Analyst**: Claude (Sonnet 4.5)

---

## Executive Summary

**Overall Feasibility**: **MEDIUM-HIGH EFFORT** with significant architectural gaps

The `theturtlecsz/code` CLI has a robust multi-agent infrastructure and MCP integration that provides foundational capabilities for PPP framework integration. However, **critical components are missing**:

1. ✅ **Strong foundation**: Multi-agent consensus, MCP integration, configuration system
2. ⚠️ **Missing reasoning control**: No `/reasoning` command or vagueness detection exists
3. ⚠️ **No personalization layer**: Configuration supports agent instructions, not user preferences
4. ⚠️ **Binary consensus logic**: No interaction quality scoring in agent selection
5. ⚠️ **Logging exists but needs extension**: Execution logging present, but no interaction trajectory tracking

**Estimated Implementation Effort**: 3-6 months for full PPP integration (1 senior Rust engineer)

---

## Goal-by-Goal Feasibility Assessment

### Goal 1: Dynamic Proactivity Check in Reasoning Control
**Rating**: ⚠️ **HIGH EFFORT** (greenfield development)

#### Current State
- **FINDING**: No `/reasoning low|medium|high` command exists in codebase (verified via grep)
- **FINDING**: Slash command infrastructure exists (`slash_command.rs`, command registry)
- **FINDING**: No vagueness detection or prompt analysis logic found

#### Required Integration Points

**1. Command Registration**
- **File**: `codex-rs/tui/src/slash_command.rs`
- **Action**: Add `Reasoning(ReasoningLevel)` variant to `SlashCommand` enum
- **Complexity**: LOW (simple enum addition)

**2. Vagueness Detection Engine** (NEW MODULE)
- **Proposed Location**: `codex-rs/core/src/prompt_analysis.rs`
- **Required Functions**:
  ```rust
  pub enum VaguenessLevel { Precise, Vague, Ambiguous }
  pub fn analyze_prompt_vagueness(prompt: &str) -> VaguenessLevel;
  pub fn extract_missing_context(prompt: &str) -> Vec<String>;
  pub fn generate_low_effort_questions(prompt: &str, context: &Context) -> Vec<Question>;
  ```
- **Complexity**: HIGH (requires NLP heuristics or LLM call)
- **Dependencies**: May require adding `lingua` or `nlp` crate, or MCP tool call to analysis service

**3. Reasoning State Management**
- **File**: `codex-rs/tui/src/chatwidget/mod.rs` (ChatWidget struct)
- **Action**: Add `reasoning_level: ReasoningLevel` field to widget state
- **Complexity**: LOW

**4. Integration with Multi-Agent Commands**
- **Files**:
  - `codex-rs/core/src/slash_commands.rs:66-97` (default_instructions_for)
  - `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- **Action**: Inject vagueness check before `/plan` or `/solve` execution
- **Complexity**: MEDIUM (requires async orchestration flow modification)

#### Risks
- **R1**: Vagueness detection accuracy depends on heuristic quality (false positives/negatives)
- **R2**: Low-effort question generation requires context awareness (spec files, codebase state)
- **R3**: User may find interruptions annoying if threshold too low (needs tuning)

---

### Goal 2: Personalization Profiles via Configuration
**Rating**: ⚠️ **MEDIUM EFFORT** (config extension + output formatting)

#### Current State
- **FINDING**: Configuration system is well-structured (`config_types.rs`, TOML-based)
- **FINDING**: Existing `AgentConfig.instructions` field for agent-level customization
- **FINDING**: No user-facing preference system exists

#### Required Integration Points

**1. Configuration Schema Extension**
- **File**: `codex-rs/core/src/config_types.rs:193-246`
- **Action**: Add new struct:
  ```rust
  #[derive(Deserialize, Debug, Clone, PartialEq)]
  pub struct UserPreferences {
      // Format constraints
      pub require_json_output: bool,
      pub prohibit_commas: bool,
      pub max_output_tokens: Option<usize>,

      // Question structure
      pub one_question_per_turn: bool,
      pub require_code_snippets: bool,

      // Language
      pub language: Option<String>, // e.g., "en", "it", "es"

      // Interaction style (20 PPP preferences from paper)
      pub brevity_level: BrevityLevel, // Terse, Normal, Verbose
      pub question_format: QuestionFormat, // MultipleChoice, OpenEnded, CodeExample
      // ... add remaining 16 preferences
  }

  impl Default for UserPreferences {
      fn default() -> Self { /* sensible defaults */ }
  }
  ```
- **Complexity**: MEDIUM (20+ preference fields, validation logic)

**2. Config Loading**
- **File**: `config.toml.example:1-277`
- **Action**: Add `[user_preferences]` section:
  ```toml
  [user_preferences]
  require_json_output = false
  one_question_per_turn = true
  language = "en"
  brevity_level = "normal"
  question_format = "code-example"
  ```
- **Complexity**: LOW (TOML addition)

**3. Output Formatting Middleware** (NEW MODULE)
- **Proposed Location**: `codex-rs/tui/src/output_formatter.rs`
- **Required Functions**:
  ```rust
  pub struct OutputFormatter {
      preferences: UserPreferences,
  }

  impl OutputFormatter {
      pub fn format_agent_response(&self, response: &str) -> Result<String>;
      pub fn validate_constraints(&self, response: &str) -> ValidationResult;
      pub fn apply_language_translation(&self, response: &str) -> Result<String>;
  }
  ```
- **Complexity**: HIGH (regex/AST parsing for format validation, translation service integration)

**4. Integration Points**
- **Files**:
  - `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs:220-249` (artifact processing)
  - `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (prompt injection)
- **Action**:
  - Inject preference constraints into agent prompts
  - Filter/format agent outputs before display
- **Complexity**: MEDIUM

#### Risks
- **R1**: 20 preferences may conflict (e.g., "brevity + require_code_snippets")
- **R2**: Agent compliance not guaranteed (LLMs may ignore constraints)
- **R3**: Translation service costs (if using external API)

---

### Goal 3: Interaction-Weighted Multi-Agent Solution Selection
**Rating**: ⚠️ **MEDIUM-HIGH EFFORT** (consensus logic redesign)

#### Current State
- **FINDING**: Consensus logic is binary (`consensus_ok`, `degraded`, `conflict`)
- **FINDING**: Selection based on completeness, not quality
- **Key Files**:
  - `consensus.rs:681-958` - `run_spec_consensus()` function
  - `consensus.rs:153-170` - `expected_agents_for_stage()`
  - `pipeline_coordinator.rs:233-235` - CheckingConsensus phase

#### Required Integration Points

**1. Interaction Quality Scorer** (NEW MODULE)
- **Proposed Location**: `codex-rs/tui/src/chatwidget/spec_kit/interaction_scorer.rs`
- **Required Functions**:
  ```rust
  pub struct InteractionScore {
      pub proactivity_score: f32,   // -0.5 per high-effort question
      pub personalization_score: f32, // +1.0 per satisfied preference
      pub total_score: f32,
  }

  pub fn score_agent_interaction(
      agent_trajectory: &AgentTrajectory,
      preferences: &UserPreferences,
  ) -> InteractionScore;

  pub fn classify_question_effort(question: &str, context: &Context) -> QuestionEffort;
  ```
- **Complexity**: HIGH (requires multi-turn trajectory tracking, effort classification)

**2. Trajectory Tracking** (NEW DATA STRUCTURE)
- **Proposed Location**: `codex-rs/tui/src/chatwidget/spec_kit/trajectory.rs`
- **Schema**:
  ```rust
  pub struct AgentTrajectory {
      pub agent_name: String,
      pub turns: Vec<Turn>,
      pub questions_asked: Vec<Question>,
      pub preference_violations: Vec<PreferenceViolation>,
  }

  pub struct Turn {
      pub timestamp: DateTime<Utc>,
      pub prompt: String,
      pub response: String,
      pub interaction_score: Option<InteractionScore>,
  }

  pub enum QuestionEffort {
      Low,    // Selection, accessible context
      Medium, // Some research required
      High,   // Deep investigation, blocking
  }
  ```
- **Complexity**: MEDIUM (data modeling + storage)

**3. Consensus Logic Modification**
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs:681-958`
- **Current**:
  ```rust
  // Line 789-808: Binary consensus determination
  let consensus_ok = summary.status.eq_ignore_ascii_case("ok");
  ```
- **Required Change**:
  ```rust
  // NEW: Calculate weighted consensus with interaction scores
  let weighted_consensus = calculate_interaction_weighted_consensus(
      &artifacts,
      &trajectories,
      &widget.config.user_preferences,
  );

  pub struct WeightedConsensus {
      pub best_agent: String,
      pub confidence: f32,
      pub interaction_score: InteractionScore,
      pub technical_quality: f32,
  }
  ```
- **Complexity**: HIGH (requires refactoring consensus logic, maintaining backward compatibility)

**4. Storage Integration**
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`
- **Action**: Extend SQLite schema to store interaction scores:
  ```sql
  ALTER TABLE consensus_artifacts ADD COLUMN interaction_score REAL;
  ALTER TABLE consensus_artifacts ADD COLUMN proactivity_score REAL;
  ALTER TABLE consensus_artifacts ADD COLUMN personalization_score REAL;
  ```
- **Complexity**: LOW (schema migration)

#### Risks
- **R1**: Interaction score may conflict with technical correctness (agent with best score != best solution)
- **R2**: Scoring subjectivity (what counts as "low-effort"?)
- **R3**: Backward compatibility with existing consensus artifacts

---

### Goal 4: MCP Integration for Interaction Logging
**Rating**: ✅ **LOW-MEDIUM EFFORT** (extension of existing MCP infrastructure)

#### Current State
- **FINDING**: MCP infrastructure is mature and operational
- **FINDING**: Execution logging exists (`execution_logger.rs`)
- **FINDING**: SQLite consensus storage operational (SPEC-934)
- **Key Components**:
  - `mcp_connection_manager.rs` - MCP client management
  - `consensus_db.rs` - SQLite storage
  - `execution_logger.rs` - run/stage/cost tracking

#### Required Integration Points

**1. Interaction Trajectory MCP Tool** (NEW MCP SERVER)
- **Approach**: External Node.js MCP server (follows local-memory pattern)
- **Tool Definitions**:
  ```typescript
  // interaction-logger-mcp/src/tools.ts
  {
    name: "log_interaction",
    description: "Log agent interaction event (question, response, score)",
    inputSchema: {
      type: "object",
      properties: {
        spec_id: { type: "string" },
        agent_name: { type: "string" },
        turn_number: { type: "integer" },
        prompt: { type: "string" },
        response: { type: "string" },
        questions_asked: { type: "array", items: { type: "string" } },
        interaction_score: { type: "number" }
      }
    }
  },
  {
    name: "query_trajectory",
    description: "Retrieve interaction trajectory for spec/agent",
    inputSchema: {
      spec_id: { type: "string" },
      agent_name: { type: "string" }
    }
  }
  ```
- **Complexity**: MEDIUM (new MCP server, ~500 LOC)

**2. Rust Client Integration**
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/interaction_logger.rs` (NEW)
- **Implementation**:
  ```rust
  pub async fn log_interaction_event(
      mcp_manager: &McpConnectionManager,
      spec_id: &str,
      agent_name: &str,
      turn: &Turn,
      score: &InteractionScore,
  ) -> Result<()> {
      let args = json!({
          "spec_id": spec_id,
          "agent_name": agent_name,
          "turn_number": turn.turn_number,
          "prompt": turn.prompt,
          "response": turn.response,
          "interaction_score": score.total_score,
      });

      mcp_manager.call_tool(
          "interaction-logger",
          "log_interaction",
          Some(args),
          Some(Duration::from_secs(30)),
      ).await?;

      Ok(())
  }
  ```
- **Complexity**: LOW (follows existing MCP call pattern)

**3. Config Registration**
- **File**: `config.toml.example`
- **Addition**:
  ```toml
  [mcp_servers.interaction-logger]
  command = "npx"
  args = ["-y", "interaction-logger-mcp"]
  startup_timeout_sec = 20
  tool_timeout_sec = 60
  ```
- **Complexity**: TRIVIAL

**4. Integration with Agent Execution**
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- **Action**: Hook into agent spawn/complete lifecycle:
  ```rust
  // After agent completes
  let trajectory = extract_trajectory(&agent_result);
  let score = score_agent_interaction(&trajectory, &preferences);
  interaction_logger::log_interaction_event(
      &mcp_manager,
      spec_id,
      agent_name,
      &trajectory.turns.last().unwrap(),
      &score,
  ).await?;
  ```
- **Complexity**: MEDIUM (async orchestration)

**5. Alternative: Extend Existing SQLite Storage**
- **File**: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`
- **Action**: Add trajectory storage to existing DB instead of MCP
- **Pros**: No external service, simpler deployment
- **Cons**: Couples trajectory logging to consensus DB
- **Complexity**: LOW (schema extension + insert/query methods)

#### Recommended Approach
**Use SQLite extension** instead of MCP server for:
- Simpler deployment (no external dependency)
- Better performance (no IPC overhead)
- Unified storage with consensus artifacts

#### Risks
- **R1**: Trajectory storage growth (may bloat SQLite DB)
- **R2**: Multi-turn extraction complexity (how to parse agent conversations?)
- **R3**: Real-time scoring overhead (may slow agent execution)

---

## Key Integration Points Summary

| Goal | Primary Files | Effort | Risk |
|------|--------------|--------|------|
| **Goal 1: Proactivity** | `slash_command.rs`, `prompt_analysis.rs` (NEW), `agent_orchestrator.rs` | HIGH | Medium |
| **Goal 2: Personalization** | `config_types.rs`, `output_formatter.rs` (NEW), `consensus.rs` | MEDIUM | Low |
| **Goal 3: Interaction-Weighted Selection** | `consensus.rs`, `interaction_scorer.rs` (NEW), `trajectory.rs` (NEW) | MEDIUM-HIGH | High |
| **Goal 4: MCP Logging** | `consensus_db.rs` OR `interaction-logger-mcp` (NEW) | LOW-MEDIUM | Low |

---

## Recommended Implementation Phases

### Phase 1: Foundation (4-6 weeks)
1. **User Preferences Configuration** (Goal 2)
   - Extend `config_types.rs` with `UserPreferences`
   - Add TOML parsing and validation
   - Create basic output formatter (no translation)
   - **Deliverable**: Users can set preferences in `config.toml`

2. **Trajectory Storage** (Goal 4 - SQLite approach)
   - Extend `consensus_db.rs` with trajectory schema
   - Add insert/query methods
   - **Deliverable**: Interaction events are logged to SQLite

### Phase 2: Interaction Scoring (6-8 weeks)
3. **Proactivity Detection** (Goal 1 - Basic heuristics)
   - Create `prompt_analysis.rs` with regex-based vagueness detection
   - Add `/reasoning` command registration
   - Integrate with `/plan` command (ask clarifying questions before spawning agents)
   - **Deliverable**: System detects vague prompts and asks low-effort questions

4. **Interaction Quality Scorer** (Goal 3 - Core logic)
   - Create `interaction_scorer.rs`
   - Implement question effort classification
   - Calculate proactivity/personalization scores
   - **Deliverable**: Each agent trajectory has interaction score

### Phase 3: Weighted Consensus (4-6 weeks)
5. **Consensus Logic Refactor** (Goal 3 - Integration)
   - Modify `consensus.rs` to use interaction-weighted selection
   - Add A/B testing flag (enable/disable weighting)
   - Add telemetry to compare outcomes
   - **Deliverable**: Agent selection considers interaction quality

6. **Advanced Proactivity** (Goal 1 - LLM-based)
   - Replace regex heuristics with LLM-based analysis (MCP call to analysis service)
   - Add context-aware question generation
   - **Deliverable**: Higher-quality vagueness detection

### Phase 4: Polish & Evaluation (3-4 weeks)
7. **User Testing & Tuning**
   - Adjust scoring weights based on real usage
   - Add user feedback mechanism (`/ppp-feedback` command)
   - Iterate on preference set (may remove/add from 20 PPP preferences)

8. **Documentation & Migration**
   - Update `CLAUDE.md` with PPP workflow
   - Create migration guide for existing SPECs
   - Add PPP metrics to `/speckit.status` dashboard

---

## Technical Challenges & Mitigation

### Challenge 1: Vagueness Detection Accuracy
- **Problem**: False positives annoy users, false negatives miss opportunities
- **Mitigation**:
  - Start with high threshold (only flag extremely vague prompts)
  - Add `/ppp-tune` command for users to adjust sensitivity
  - Log user overrides to improve heuristics

### Challenge 2: Agent Compliance with Preferences
- **Problem**: LLMs may ignore output format constraints
- **Mitigation**:
  - Add post-processing layer (parse JSON, strip commas, etc.)
  - Penalize agents that violate constraints in interaction score
  - Add retry logic with stronger constraint language

### Challenge 3: Interaction Score vs. Technical Correctness
- **Problem**: Best-scored agent may not have best solution
- **Mitigation**:
  - Weighted combination: `final_score = 0.7 * technical + 0.3 * interaction`
  - Add override command: `/speckit.prefer-agent gemini` for manual selection
  - Show both scores in consensus report

### Challenge 4: Performance Overhead
- **Problem**: Scoring, formatting, logging adds latency
- **Mitigation**:
  - Score asynchronously after agent completes (don't block consensus)
  - Cache preference compilations (don't re-parse TOML every turn)
  - Use SQLite instead of MCP for trajectory logging (faster)

---

## Dependencies & Infrastructure

### New Rust Crates
```toml
[dependencies]
# For vagueness detection (NLP heuristics)
lingua = "1.6"         # Language detection
regex = "1.10"         # Pattern matching (already in workspace)

# For JSON output validation
serde_json = "1.0"     # Already in workspace
jsonschema = "0.18"    # JSON schema validation

# For translation (if implementing lang_ita preference)
reqwest = "0.12"       # HTTP client for translation API (already in workspace)
```

### External Services (Optional)
- **Translation API**: Google Translate API or LibreTranslate (self-hosted)
- **Vagueness Analysis Service**: Custom MCP server with fine-tuned LLM
- **Interaction Logger MCP** (if not using SQLite): Node.js MCP server

### Configuration Changes
```toml
# NEW: User preferences section
[user_preferences]
require_json_output = false
one_question_per_turn = true
language = "en"
brevity_level = "normal"
question_format = "code-example"
max_clarifying_questions = 3
proactivity_threshold = "medium"  # low/medium/high

# NEW: PPP framework settings
[ppp]
enabled = true
interaction_weight = 0.3           # 0.0-1.0 (0.0 = ignore interaction, 1.0 = only interaction)
vagueness_threshold = "medium"     # low/medium/high
question_effort_classifier = "heuristic"  # heuristic|llm-mcp

# NEW: Interaction logger (if using MCP approach)
[mcp_servers.interaction-logger]
command = "npx"
args = ["-y", "interaction-logger-mcp"]
startup_timeout_sec = 20
```

---

## Migration Path for Existing System

### Backward Compatibility Strategy
1. **Feature Flag**: `ppp.enabled = false` by default
2. **Dual-Mode Consensus**:
   ```rust
   if config.ppp.enabled {
       run_interaction_weighted_consensus()
   } else {
       run_legacy_consensus()  // Existing binary logic
   }
   ```
3. **Gradual Rollout**:
   - Phase 1: Logging only (no behavior change)
   - Phase 2: Opt-in for `/speckit.auto --ppp`
   - Phase 3: Default on after validation

### Data Migration
- **No breaking changes**: Existing consensus artifacts remain valid
- **Additive schema**: New SQLite columns have defaults (NULL or 0.0)
- **Telemetry comparison**: Run A/B test (PPP vs. legacy) for 50 SPECs, compare:
  - User satisfaction (manual feedback)
  - Technical correctness (test pass rate)
  - Cost (tokens used for questions)
  - Time (consensus duration)

---

## Cost-Benefit Analysis

### Implementation Cost
| Phase | Engineer-Weeks | Calendar Time |
|-------|----------------|---------------|
| Phase 1: Foundation | 4-6 weeks | 1-1.5 months |
| Phase 2: Scoring | 6-8 weeks | 1.5-2 months |
| Phase 3: Consensus | 4-6 weeks | 1-1.5 months |
| Phase 4: Polish | 3-4 weeks | 0.75-1 month |
| **Total** | **17-24 weeks** | **5-6 months** |

**Assumptions**: 1 senior Rust engineer, full-time, familiar with codebase

### Runtime Cost Impact
- **Token Usage**: +10-20% (clarifying questions, preference injection)
- **Latency**: +5-10% (scoring, formatting overhead)
- **Storage**: +50-100 MB per 100 SPECs (trajectory data)

### Expected Benefits
- **Fewer Frustrated Users**: Proactivity reduces "Why didn't you just ask?" moments
- **Better Agent Selection**: Interaction-weighted consensus prefers less disruptive agents
- **Higher Satisfaction**: Personalization matches user's preferred interaction style
- **Quantifiable Improvement**: PPP paper reports 15-30% satisfaction increase

---

## Risks & Open Questions

### High-Risk Items
1. **Scoring Subjectivity**: How do we validate that interaction scores align with user satisfaction?
   - **Mitigation**: User feedback loop (`/ppp-feedback good|bad` command)

2. **Agent Non-Compliance**: What if agents ignore preference constraints?
   - **Mitigation**: Post-processing + penalty in interaction score

3. **Complexity Creep**: PPP adds 3-4 new modules, increases maintenance burden
   - **Mitigation**: Comprehensive test coverage (unit + integration tests)

### Open Questions
1. **Question Effort Classification**: Should we use LLM or heuristics?
   - **Recommendation**: Start with heuristics (fast, cheap), upgrade to LLM if accuracy insufficient

2. **Interaction Weight Tuning**: What's the optimal `interaction_weight` (0.0-1.0)?
   - **Recommendation**: A/B test with 0.2, 0.3, 0.5 and measure outcomes

3. **MCP vs. SQLite for Logging**: Which storage backend?
   - **Recommendation**: SQLite (simpler, faster, already integrated)

4. **Translation Service**: Self-hosted vs. API?
   - **Recommendation**: Start without translation (only English), add if users request

---

## Next Steps

### Immediate Actions (Week 1)
1. **Validate Analysis**: Review this document with project maintainers
2. **Prioritize Goals**: Decide which goals are must-have vs. nice-to-have
3. **Prototype**: Build quick PoC for Goal 2 (personalization config) - 2 days
4. **Design Review**: Create RFC for trajectory schema + interaction scorer

### Month 1 Milestones
- [ ] `UserPreferences` struct merged to main
- [ ] Trajectory SQLite schema designed and reviewed
- [ ] PoC: `/reasoning` command detects vague prompts (regex-based)
- [ ] Decision: MCP vs. SQLite for trajectory logging

### Month 3 Milestones
- [ ] Full Phase 1 complete (preferences + trajectory storage)
- [ ] Phase 2 in progress (scoring logic 50% done)
- [ ] A/B testing framework ready

### Month 6 Milestones
- [ ] Full PPP framework operational
- [ ] User feedback: 10+ users tested, satisfaction metrics collected
- [ ] Documentation: PPP workflow guide published
- [ ] Decision: Promote to default or keep opt-in

---

## Conclusion

The PPP framework integration is **feasible but non-trivial**, requiring:
- **3-6 months** implementation time
- **4 new Rust modules** (prompt_analysis, output_formatter, interaction_scorer, trajectory)
- **1 optional MCP server** (interaction-logger) OR SQLite extension
- **Configuration schema extension** (20+ preference fields)
- **Consensus logic refactor** (interaction-weighted selection)

**Strengths of Current Codebase**:
- ✅ Mature multi-agent infrastructure
- ✅ MCP integration operational
- ✅ SQLite consensus storage (SPEC-934)
- ✅ Execution logging framework
- ✅ Configuration system extensible

**Major Gaps**:
- ❌ No reasoning control mechanism (Goal 1 is greenfield)
- ❌ No user preference system (Goal 2 needs design)
- ❌ Binary consensus logic (Goal 3 requires refactor)
- ❌ No interaction trajectory tracking (Goal 4 needs new schema)

**Recommendation**: **Proceed with phased rollout**, starting with Goal 2 (personalization) as it has highest ROI and lowest risk. Validate user demand before investing in complex proactivity detection (Goal 1).

**Risk Level**: MEDIUM (technical feasibility is high, but user adoption and tuning may be challenging)

---

## Appendix A: File Inventory

**Configuration System**:
- `codex-rs/core/src/config_types.rs` (245 lines) - Type definitions
- `config.toml.example` (277 lines) - Example config

**Multi-Agent Consensus**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs` (1160 lines) - Consensus logic
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs` - SQLite storage
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs` - Orchestration
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` - Agent spawning

**Execution & Logging**:
- `codex-rs/tui/src/chatwidget/spec_kit/execution_logger.rs` - Run/stage tracking
- `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` (300+ lines) - State machine

**MCP Integration**:
- `codex-rs/core/src/mcp_connection_manager.rs` (150+ lines) - Client management
- `codex-rs/core/src/mcp_tool_call.rs` - Tool invocation

**Command Infrastructure**:
- `codex-rs/tui/src/slash_command.rs` - Command enum
- `codex-rs/core/src/slash_commands.rs` - Multi-agent prompts
- `codex-rs/tui/src/chatwidget/spec_kit/command_handlers.rs` - Entry points

---

## Appendix B: PPP Framework References

**Paper Citations** (from user prompt):
- [3-5]: Frontier LLMs struggle with proactivity and personalization
- [4, 7, 8]: Low-effort questions preferred over high-effort
- [4, 9]: Personalization improves satisfaction
- [24]: $R_{Proact}$ penalty of -0.5 per high-effort query

**Key Concepts**:
- **Productivity ($R_{Prod}$)**: Task completion success
- **Proactivity ($R_{Proact}$)**: Strategic clarifying questions (minimize high-effort)
- **Personalization ($R_{Pers}$)**: Adapt to user preferences (format, language, style)
- **USERVILLE Feedback**: Simulated user feedback mechanism (paper reference [26])

**Implementation Strategy**:
- Low-effort questions: Selection questions, context-accessible info
- High-effort questions: Deep investigation, blocking clarifications
- Preference constraints: JSON format, language, question structure, brevity
