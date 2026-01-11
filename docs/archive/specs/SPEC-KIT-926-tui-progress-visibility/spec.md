# SPEC-KIT-926: Comprehensive TUI Progress Tracking and Status Visibility

**Status**: Draft
**Priority**: High (Critical UX Issue)
**Created**: 2025-11-11
**Problem Type**: User Experience - Information Visibility
**Affects**: All /speckit.* commands, agent orchestration, multi-stage pipelines

---

## Problem Statement

The TUI provides insufficient visibility into what operations are being performed, leaving users confused about:
- What work is planned before execution begins
- Which operation is currently running
- Progress through multi-step operations
- Whether the system is working, waiting, or hung
- Which agent is executing and what it's doing
- How much time operations are taking
- What has completed vs what remains

**Current UX Pain Points**:
1. **No upfront summary** - Operations start without showing what will be done
2. **Silent execution** - Long periods with no output while agents run (3-8 minutes)
3. **Unclear waiting** - Polling loops provide no feedback that system is waiting
4. **Hidden agent work** - Agent execution happens invisibly in background
5. **No progress indicators** - Multi-stage pipelines show no N/M step tracking
6. **Ambiguous states** - Cannot distinguish "working" from "hung" from "waiting for API"

**Impact**: Users constantly wonder "is it working or broken?" leading to premature cancellation of valid operations.

---

## User Stories

### Story 1: Pipeline Overview
**As a user**, when I run `/speckit.plan SPEC-KIT-900`, **I want to see**:
```
📋 PLAN: Starting pipeline for SPEC-KIT-900
   Phase 1: Load context (spec.md, constitution.md)
   Phase 2: Spawn 3 agents sequentially (gemini → claude → gpt_pro)
   Phase 3: Synthesize consensus
   Phase 4: Write plan.md

   Estimated time: 5-8 minutes
   Press Ctrl+C to cancel

▶ Starting Phase 1: Load context...
```

### Story 2: Agent Execution Visibility
**As a user**, when an agent is running, **I want to see**:
```
🤖 Agent: gemini-2.5-pro (1/3)
   Started: 14:50:23
   Elapsed: 2m 15s
   Status: Thinking... (generating analysis)
   Model: gemini-2.5-pro (thinking mode)
   Output: /tmp/tmux-agent-output-4051297-181.txt

   [✓] Prompt sent (3.2KB)
   [✓] Agent spawned (id: a3f8b2...)
   [~] Waiting for completion... (polling every 500ms)

   Observable: tmux attach -t agents-gemini
```

### Story 3: Sequential Progress
**As a user**, during sequential agent execution, **I want to see**:
```
🎬 SEQUENTIAL EXECUTION: 3 agents planned

✅ gemini-2.5-pro: Completed in 3m 12s
   → Output: 2.3KB JSON (research_summary, questions)

▶ claude-haiku: Running... (1m 45s elapsed)
   Status: Generating response

⏸ gpt-5-medium: Waiting for claude to complete
```

### Story 4: Consensus Synthesis
**As a user**, during synthesis, **I want to see**:
```
🔄 CONSENSUS SYNTHESIS: Analyzing 3 agent outputs

   [✓] Loaded gemini output (2.3KB)
   [✓] Loaded claude output (1.8KB)
   [✓] Loaded gpt_pro output (2.1KB)
   [~] Identifying agreements...
   [~] Resolving conflicts...
   [~] Generating markdown...

   Output: docs/SPEC-KIT-900/plan.md
```

### Story 5: Multi-Stage Pipeline (/speckit.auto)
**As a user**, when running full pipeline, **I want to see**:
```
🚀 AUTO PIPELINE: SPEC-KIT-900 (6 stages)

Stage Progress: ████████░░░░░░░░░░░░ 2/6 (33%)
Time Elapsed: 12m 30s
Estimated Remaining: ~25 minutes

✅ 1. PLAN (Tier 2)      Completed in 5m 45s
✅ 2. TASKS (Tier 1)     Completed in 2m 10s
▶  3. IMPLEMENT (Tier 2) Running... (4m 35s)
   └─ Agent: gpt-codex (2/2) - Generating code
⏸  4. VALIDATE (Tier 2)  Waiting...
⏸  5. AUDIT (Tier 3)     Waiting...
⏸  6. UNLOCK (Tier 3)    Waiting...

Current Operation:
   🤖 gpt-codex: Implementing authentication handlers
   Files affected: 3 modified, 2 created
   Tests: 12 new integration tests
```

### Story 6: Error Context
**As a user**, when an operation fails, **I want to see**:
```
❌ FAILED: Stage IMPLEMENT

What was being attempted:
   Stage: IMPLEMENT (Tier 2)
   Agent: gpt-codex (2/2)
   Operation: Code generation for authentication handlers
   Duration: 4m 35s before failure

Error:
   Agent failed: Required dependency 'bcrypt' not found in package.json

Context:
   • Previous agent (claude-haiku) completed successfully
   • Output file written but contained error details
   • Consensus synthesis not attempted

Recovery options:
   1. Add 'bcrypt' to package.json and retry
   2. Skip IMPLEMENT stage with --from validate
   3. Cancel pipeline

Evidence: docs/SPEC-OPS-004-integrated-coder-hooks/evidence/
```

---

## Current Implementation Gaps

### 1. **No Status Bar**
**Location**: TUI main loop
**Missing**: Persistent status line showing current operation
```rust
// Current: No status indication
// Needed: Status bar showing:
// "▶ PLAN Stage | Agent: gemini (1/3) | 2m 15s | Waiting for completion..."
```

### 2. **Silent Agent Spawning**
**Location**: `agent_orchestrator.rs::spawn_and_wait_for_agent()`
**Current**: Only logs agent ID
**Needed**:
- Show agent config (model, reasoning mode)
- Show prompt size
- Show expected duration
- Show observable tmux session
- Show polling status every 10s

### 3. **No Pipeline Summary**
**Location**: All `/speckit.*` command handlers
**Current**: Immediate execution
**Needed**:
- Print what will be done before starting
- Show estimated time
- Show cost estimate (from tiering)
- Wait 2s before starting (allow cancel)

### 4. **Hidden Consensus**
**Location**: `consensus.rs::synthesize_consensus()`
**Current**: Single "Synthesizing..." message
**Needed**:
- Show each agent output being loaded
- Show conflict detection
- Show agreement extraction
- Show markdown generation

### 5. **No Progress Bars**
**Location**: Multi-stage operations
**Current**: Sequential text output
**Needed**:
- Visual progress bar for N/M stages
- Time elapsed / estimated remaining
- Stage status indicators (✓ done, ▶ running, ⏸ waiting)

### 6. **Ambiguous Waiting**
**Location**: All polling loops
**Current**: Silent polling
**Needed**:
- Explicit "Waiting for X..." messages
- Show polling frequency
- Show what's being checked
- Heartbeat indicator (show system is alive)

---

## Technical Requirements

### Requirement 1: Status Bar Widget
**Implementation**: Add persistent status bar to TUI
**Location**: `codex-rs/tui/src/app.rs`

```rust
pub struct StatusBar {
    current_operation: String,      // "PLAN Stage"
    sub_operation: Option<String>,  // "Agent: gemini (1/3)"
    elapsed: Duration,
    status_icon: StatusIcon,        // ▶ Running, ⏸ Waiting, ✓ Done, ❌ Failed
}

enum StatusIcon {
    Running,   // ▶
    Waiting,   // ⏸
    Success,   // ✓
    Failed,    // ❌
    Idle,      // ○
}
```

**Rendering**: Bottom row of terminal, always visible
**Update frequency**: 500ms refresh
**Content**: `"▶ PLAN Stage | Agent: gemini (1/3) | 2m 15s | Polling for completion"`

### Requirement 2: Pipeline Preview
**Implementation**: Add preview phase before execution
**Location**: Each `/speckit.*` command handler

```rust
async fn preview_and_confirm(
    spec_id: &str,
    stage: SpecStage,
    agents: &[String],
    estimated_minutes: (u32, u32),
    cost_estimate: f64,
) -> Result<bool, String> {
    // Print what will be done
    // Show time estimate
    // Show cost estimate
    // Wait 2s or until user presses Enter
    // Return true if should proceed
}
```

**User can**:
- See what's about to happen
- Cancel with Ctrl+C before work starts
- Understand time commitment

### Requirement 3: Agent Execution Dashboard
**Implementation**: Detailed agent status during execution
**Location**: `agent_orchestrator.rs`

```rust
struct AgentExecutionStatus {
    agent_name: String,
    index: usize,              // Which agent (1 of 3)
    total: usize,              // Total agents
    start_time: Instant,
    status: AgentRunStatus,
    output_file: PathBuf,
    tmux_session: Option<String>,
    model_info: ModelMetadata,
}

enum AgentRunStatus {
    Spawning,
    Thinking,
    Writing,
    Completing,
    Done,
}
```

**Display updates**:
- Initial spawn: Show config, prompt size, model
- Every 10s: Show elapsed time, current status
- Completion: Show output size, duration, next agent

### Requirement 4: Sequential Progress Tracker
**Implementation**: Show all agents and their states
**Location**: `spawn_regular_stage_agents_sequential()`

```rust
struct SequentialTracker {
    agents: Vec<AgentStatus>,
    current_index: usize,
}

impl SequentialTracker {
    fn render(&self) -> String {
        // ✅ gemini: Completed in 3m 12s
        // ▶  claude: Running... (1m 45s)
        // ⏸  gpt_pro: Waiting...
    }
}
```

**Updates**: Every 500ms, re-render current state

### Requirement 5: Multi-Stage Pipeline Progress
**Implementation**: Progress bar for `/speckit.auto`
**Location**: `quality_gate_handler.rs::handle_auto_with_gates()`

```rust
struct PipelineProgress {
    stages: Vec<StageProgress>,
    current_stage: usize,
    start_time: Instant,
}

struct StageProgress {
    name: String,
    tier: u8,
    status: StageStatus,
    duration: Option<Duration>,
    estimated_duration: Duration,
}

enum StageStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}
```

**Display**:
- ASCII progress bar: `████████░░░░░░░░░░░░ 2/6 (33%)`
- Stage list with icons
- Time tracking (elapsed + estimated remaining)
- Current operation detail

### Requirement 6: Heartbeat Indicator
**Implementation**: Show system is alive during long waits
**Location**: All polling loops (agent completion, file I/O, etc.)

```rust
struct HeartbeatIndicator {
    frames: &'static [&'static str],  // ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    current_frame: usize,
    last_update: Instant,
}

impl HeartbeatIndicator {
    fn tick(&mut self) -> &str {
        // Rotate every 100ms
        // Returns current spinner frame
    }
}
```

**Usage**: During polling, show spinner + message:
```
⠙ Waiting for agent completion... (2m 15s)
```

### Requirement 7: Consensus Synthesis Visibility
**Implementation**: Step-by-step consensus tracking
**Location**: `consensus.rs::synthesize_consensus()`

```rust
async fn synthesize_consensus_verbose(
    agent_outputs: Vec<AgentOutput>,
    output_path: &Path,
) -> Result<(), String> {
    println!("🔄 CONSENSUS SYNTHESIS: Analyzing {} agent outputs\n", agent_outputs.len());

    for (i, output) in agent_outputs.iter().enumerate() {
        println!("   [✓] Loaded {} output ({}KB)", output.agent, output.size_kb());
    }

    println!("   [~] Identifying agreements...");
    let agreements = find_agreements(&agent_outputs)?;
    println!("       Found {} agreement points", agreements.len());

    println!("   [~] Resolving conflicts...");
    let conflicts = find_conflicts(&agent_outputs)?;
    println!("       Resolved {} conflicts", conflicts.len());

    println!("   [~] Generating markdown...");
    let markdown = generate_markdown(agreements, conflicts)?;

    println!("   [✓] Writing to {}", output_path.display());
    tokio::fs::write(output_path, markdown).await?;

    println!("\n✅ Consensus complete: {}", output_path.display());
    Ok(())
}
```

### Requirement 8: Error Context Display
**Implementation**: Rich error reporting with context
**Location**: All error handling paths

```rust
struct ContextualError {
    operation: String,        // "IMPLEMENT Stage - gpt-codex agent"
    error_message: String,
    duration_before_failure: Duration,
    context: Vec<String>,     // ["Previous agent completed", "Output file written"]
    recovery_options: Vec<String>,
    evidence_path: PathBuf,
}

impl ContextualError {
    fn display(&self) {
        println!("❌ FAILED: {}\n", self.operation);
        println!("What was being attempted:");
        println!("   {}", self.operation);
        println!("   Duration: {:?} before failure\n", self.duration_before_failure);
        println!("Error:");
        println!("   {}\n", self.error_message);
        println!("Context:");
        for ctx in &self.context {
            println!("   • {}", ctx);
        }
        println!("\nRecovery options:");
        for (i, opt) in self.recovery_options.iter().enumerate() {
            println!("   {}. {}", i+1, opt);
        }
        println!("\nEvidence: {}", self.evidence_path.display());
    }
}
```

---

## Implementation Plan

### Phase 1: Status Bar Foundation (2-3 hours)
**Goal**: Add persistent status line to TUI

**Tasks**:
1. Create `StatusBar` struct with current operation tracking
2. Add status bar rendering to TUI bottom row
3. Add status update API for command handlers to use
4. Test with `/speckit.plan` showing stage + agent + elapsed time

**Files**:
- `codex-rs/tui/src/app.rs` - StatusBar widget
- `codex-rs/tui/src/chatwidget/mod.rs` - Status update calls

**Acceptance**:
- Bottom row shows current operation
- Updates every 500ms
- Shows operation, sub-operation, elapsed time
- Status icon changes (▶ → ✓)

### Phase 2: Agent Execution Visibility (3-4 hours)
**Goal**: Show detailed agent status during execution

**Tasks**:
1. Add agent execution dashboard to orchestrator
2. Show agent spawn details (model, prompt size, tmux session)
3. Add polling status updates every 10s
4. Show heartbeat indicator during waits
5. Display completion summary (duration, output size, next agent)

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`
- Integration with StatusBar from Phase 1

**Acceptance**:
- User sees agent spawn message with details
- Every 10s, shows "Waiting for completion... (Nm XXs)"
- Heartbeat spinner shows system is alive
- Completion shows duration and output size
- Next agent announcement clear

### Phase 3: Sequential Progress Tracker (2 hours)
**Goal**: Show all agents and their states in one view

**Tasks**:
1. Create `SequentialTracker` struct
2. Render agent list with status icons
3. Update on each agent state change
4. Integrate with StatusBar

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

**Acceptance**:
- Shows ✅/▶/⏸ for completed/running/waiting agents
- Updates in real-time as agents progress
- Clear indication of which agent is active

### Phase 4: Pipeline Preview (1-2 hours)
**Goal**: Show what will be done before starting

**Tasks**:
1. Add preview function to each `/speckit.*` command
2. Show phases, agents, estimated time, cost
3. Add 2s delay before starting (cancelable)
4. Test with all commands

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` - Preview function
- Each command handler (`plan_handler.rs`, etc.)

**Acceptance**:
- Each command shows upfront summary
- User can read what will happen
- 2s window allows cancellation
- Cost and time estimates shown

### Phase 5: Consensus Visibility (1 hour)
**Goal**: Show step-by-step consensus synthesis

**Tasks**:
1. Add verbose logging to `synthesize_consensus()`
2. Show loading, analysis, conflict resolution, markdown generation
3. Display output path when complete

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus.rs`

**Acceptance**:
- Each step explicitly shown
- User sees progress through synthesis
- No silent gaps during consensus

### Phase 6: Multi-Stage Pipeline Progress (3-4 hours)
**Goal**: Visual progress bar for `/speckit.auto`

**Tasks**:
1. Create `PipelineProgress` tracker
2. Add ASCII progress bar rendering
3. Track stage status (pending/running/completed/failed)
4. Show time elapsed and estimated remaining
5. Display current operation details

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/quality_gate_handler.rs`

**Acceptance**:
- Progress bar shows N/6 stages
- Stage list with icons
- Time tracking accurate
- Current operation detail visible

### Phase 7: Error Context Display (2 hours)
**Goal**: Rich error reporting with recovery options

**Tasks**:
1. Create `ContextualError` struct
2. Wrap all error returns with context
3. Add recovery option suggestions
4. Test with intentional failures

**Files**:
- `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` - Error handling
- All command handlers

**Acceptance**:
- Errors show what was attempted
- Context explains what worked before failure
- Recovery options suggested
- Evidence path provided

---

## Success Criteria

### Criterion 1: No Silent Periods
**Test**: Run `/speckit.plan SPEC-KIT-900`
**Expected**: Output every 10s minimum showing:
- What's happening
- How long it's been
- Heartbeat indicator

**Failure Mode**: More than 15s with no output = user assumes hung

### Criterion 2: Clear Current Operation
**Test**: At any point during execution, ask "what is it doing?"
**Expected**: Status bar + recent output clearly answers question

**Failure Mode**: User cannot tell from screen what operation is running

### Criterion 3: Upfront Transparency
**Test**: Run any `/speckit.*` command
**Expected**: Before work starts, see:
- What will be done
- How long it will take
- What it will cost (for paid models)

**Failure Mode**: Work starts immediately without preview

### Criterion 4: Progress Awareness
**Test**: Run `/speckit.auto SPEC-KIT-900`
**Expected**: At any point, clearly see:
- Which stage (N/6)
- Current operation
- Time elapsed
- Time remaining (estimate)

**Failure Mode**: Cannot tell how far along pipeline is

### Criterion 5: Error Understanding
**Test**: Cause a failure (missing dependency, network error, etc.)
**Expected**: Error message shows:
- What was being attempted
- Why it failed
- What worked before failure
- How to recover

**Failure Mode**: Error message only shows error text without context

---

## Non-Goals

**Not in scope for this SPEC**:
1. **Interactive UI** - Not building TUI controls, just better text output
2. **Graphical Progress** - ASCII art only, no terminal graphics libraries
3. **Real-time Log Streaming** - Just summary updates, not full log tailing
4. **Pause/Resume** - Not adding operation control, just visibility
5. **Historical View** - Only showing current operation, not past runs

---

## Testing Strategy

### Unit Tests
**Location**: Each new module (`status_bar.rs`, `progress_tracker.rs`, etc.)

**Coverage**:
- StatusBar rendering with different states
- Progress calculation (N/M, time estimates)
- HeartbeatIndicator frame rotation
- ContextualError formatting

### Integration Tests
**Location**: `codex-rs/tui/tests/visibility_tests.rs`

**Scenarios**:
1. **Test: Single Agent Execution**
   - Run `/speckit.tasks SPEC-KIT-900`
   - Verify status bar updates
   - Verify agent spawn message
   - Verify completion summary

2. **Test: Sequential Multi-Agent**
   - Run `/speckit.plan SPEC-KIT-900`
   - Verify sequential tracker shows all 3 agents
   - Verify status transitions (⏸ → ▶ → ✅)
   - Verify timing updates

3. **Test: Full Pipeline**
   - Run `/speckit.auto SPEC-KIT-900`
   - Verify progress bar advances
   - Verify stage transitions
   - Verify time estimates
   - Verify completion

4. **Test: Error Handling**
   - Trigger agent failure (bad API key)
   - Verify error context displayed
   - Verify recovery options shown
   - Verify evidence path provided

### Manual Testing Checklist
- [ ] Status bar visible and updating
- [ ] No silent periods >15s
- [ ] Preview shows before work starts
- [ ] Agent details clear during execution
- [ ] Sequential progress tracker accurate
- [ ] Pipeline progress bar renders correctly
- [ ] Consensus steps visible
- [ ] Errors provide context and recovery options
- [ ] Heartbeat indicator shows during long waits
- [ ] Time estimates reasonably accurate

---

## Migration Path

### Backward Compatibility
**Impact**: Pure addition - no breaking changes

**Existing behavior**:
- All current output remains
- New visibility features added on top
- No changes to command arguments
- No changes to output files

### Rollout
**Phase 1** (Phase 1-3): Core visibility (status bar, agent details, sequential)
**Phase 2** (Phase 4-5): Preview and consensus
**Phase 3** (Phase 6-7): Pipeline progress and errors

**Each phase**:
- Can be deployed independently
- Improves UX incrementally
- No flags needed (always on)

---

## Open Questions

1. **Q**: Should status bar be at top or bottom of terminal?
   **A**: Bottom - less jarring for scrolling output

2. **Q**: What refresh rate for status updates?
   **A**: 500ms - fast enough to feel live, not overwhelming

3. **Q**: Should heartbeat be opt-in or always on?
   **A**: Always on during waits >5s - critical for "is it hung?" question

4. **Q**: Should pipeline preview have configurable delay?
   **A**: No - 2s fixed delay, can skip with Enter, cancel with Ctrl+C

5. **Q**: Should we track actual vs estimated time for future accuracy?
   **A**: Yes - store in telemetry for improving estimates

---

## Related Work

**SPEC-KIT-923**: Observable Agent Execution (tmux visibility)
- Complements this SPEC
- Provides low-level agent observability
- This SPEC provides high-level pipeline observability

**SPEC-KIT-070**: Tiered Model Strategy
- Provides cost estimates needed for preview
- This SPEC displays those estimates to user

**SPEC-KIT-900**: Proper Agent Abstraction
- Enables agent metadata tracking
- This SPEC displays that metadata

---

## References

**User Feedback**:
> "Often the TUI is doing something and I have zero idea what it is doing. I want full end to end output showing me what's being worked on."

**Key Insight**: Silence = Ambiguity = User assumes system is broken

**Design Principle**: "Show, don't hide" - every operation should be visible to user

**Inspiration**:
- Cargo build output (progress bars, timing)
- npm install (package counts, progress)
- cargo test (test counts, pass/fail)
- Modern CI systems (GitHub Actions, GitLab CI)

---

## Appendix: Output Examples

### Example 1: /speckit.plan with Full Visibility
```
📋 PLAN: SPEC-KIT-900 (Clean Architecture Implementation)

Pipeline Overview:
   Phase 1: Load context (spec.md, constitution.md)
   Phase 2: Spawn 3 agents sequentially
      • gemini-2.5-pro (thinking mode)
      • claude-haiku (general)
      • gpt-5-medium (general)
   Phase 3: Synthesize consensus
   Phase 4: Write plan.md

Estimated time: 5-8 minutes
Estimated cost: ~$0.35 (Tier 2)

Press Enter to start, Ctrl+C to cancel...

▶ Phase 1: Loading context...
   [✓] Loaded docs/SPEC-KIT-900-clean-architecture/spec.md (8.2KB)
   [✓] Loaded memory/constitution.md (12.5KB)
   [✓] Context prepared: 20.7KB total

▶ Phase 2: Sequential Agent Execution

🤖 Agent: gemini-2.5-pro (1/3)
   Model: gemini-2.5-pro (thinking mode, 2025-05-14)
   Prompt: 3,247 characters
   Config: gemini_flash
   Output: /tmp/tmux-agent-output-4051297-181.txt
   Observable: tmux attach -t agents-gemini

   [✓] Prompt sent
   [✓] Agent spawned (id: a3f8b294)
   [~] Waiting for completion... (polling every 500ms)

   ⠙ Running... 0m 30s
   ⠹ Running... 1m 00s
   ⠸ Running... 1m 30s
   ⠼ Running... 2m 00s
   ⠴ Running... 2m 30s
   ⠦ Running... 3m 00s

   [✓] Completed in 3m 12s
   [✓] Output: 2.3KB JSON (research_summary, questions)
   [✓] Recorded to consensus DB

🤖 Agent: claude-haiku (2/3)
   Model: claude-3-5-haiku-20241022 (general)
   Prompt: 5,123 characters (includes gemini output)
   Config: claude_haiku

   [✓] Prompt sent
   [✓] Agent spawned (id: b4d9c185)
   [~] Waiting for completion...

   ⠙ Running... 0m 30s
   ⠹ Running... 1m 00s

   [✓] Completed in 1m 45s
   [✓] Output: 1.8KB JSON (critical_path, dependencies)
   [✓] Recorded to consensus DB

🤖 Agent: gpt-5-medium (3/3)
   Model: gpt-5-medium (general)
   Prompt: 6,891 characters (includes gemini + claude)
   Config: gpt_pro

   [✓] Prompt sent
   [✓] Agent spawned (id: c7e2a936)
   [~] Waiting for completion...

   ⠙ Running... 0m 30s
   ⠹ Running... 1m 00s
   ⠸ Running... 1m 30s

   [✓] Completed in 2m 05s
   [✓] Output: 2.1KB JSON (implementation_strategy, risks)
   [✓] Recorded to consensus DB

▶ Phase 3: Consensus Synthesis

🔄 Analyzing 3 agent outputs...
   [✓] Loaded gemini-2.5-pro output (2.3KB)
   [✓] Loaded claude-haiku output (1.8KB)
   [✓] Loaded gpt-5-medium output (2.1KB)

   [~] Identifying agreements...
       Found 8 agreement points

   [~] Resolving conflicts...
       Found 2 conflicts
       ├─ Conflict: Implementation timeline (3 weeks vs 5 weeks)
       │  Resolution: Conservative estimate (5 weeks) with 3-week stretch goal
       └─ Conflict: Testing strategy (unit-first vs integration-first)
          Resolution: Hybrid approach - critical path integration, then unit coverage

   [~] Generating markdown...
       Sections: Consensus (8 points), Work Breakdown (12 tasks), Risks (5 items)

   [✓] Writing to docs/SPEC-KIT-900-clean-architecture/plan.md

▶ Phase 4: Finalization
   [✓] Validated markdown structure
   [✓] Recorded consensus metadata
   [✓] Updated SPEC.md status

✅ PLAN COMPLETE

Duration: 7m 15s
Output: docs/SPEC-KIT-900-clean-architecture/plan.md (6.2KB)
Cost: $0.32 (3 agents × Tier 2)
Consensus: 8 agreements, 2 conflicts resolved
Next: /speckit.tasks SPEC-KIT-900

────────────────────────────────────────────────────────────────────────
Status: ✓ PLAN Complete | 7m 15s | Next: TASKS
```

### Example 2: /speckit.auto with Pipeline Progress
```
🚀 AUTO PIPELINE: SPEC-KIT-900 (Full 6-stage pipeline)

Pipeline Overview:
   1. PLAN (Tier 2)      → 3 agents, ~5-8min, ~$0.35
   2. TASKS (Tier 1)     → 1 agent, ~3-5min, ~$0.10
   3. IMPLEMENT (Tier 2) → 2 agents, ~8-12min, ~$0.11
   4. VALIDATE (Tier 2)  → 3 agents, ~10-12min, ~$0.35
   5. AUDIT (Tier 3)     → 3 premium, ~10-12min, ~$0.80
   6. UNLOCK (Tier 3)    → 3 premium, ~10-12min, ~$0.80

Total Estimated Time: 45-50 minutes
Total Estimated Cost: $2.51
Quality Gates: Enabled (will pause on failures)

Press Enter to start, Ctrl+C to cancel...

────────────────────────────────────────────────────────────────────────
Stage Progress: ████████░░░░░░░░░░░░ 2/6 (33%)
Time Elapsed: 12m 30s
Estimated Remaining: ~35 minutes
────────────────────────────────────────────────────────────────────────

✅ 1. PLAN (Tier 2)      Completed in 5m 45s ($0.32)
   └─ Output: docs/SPEC-KIT-900-clean-architecture/plan.md (6.2KB)

✅ 2. TASKS (Tier 1)     Completed in 2m 10s ($0.09)
   └─ Output: docs/SPEC-KIT-900-clean-architecture/tasks.md (4.1KB)
   └─ SPEC.md updated: 12 tasks added

▶  3. IMPLEMENT (Tier 2) Running... (4m 35s elapsed)
   ├─ Agent: gpt-codex (1/2) ✅ Completed in 2m 20s
   │  └─ Output: 3 files modified, 2 created
   └─ Agent: claude-haiku (2/2) ⠙ Running... (2m 15s)
      └─ Status: Validating code structure

⏸  4. VALIDATE (Tier 2)  Waiting for IMPLEMENT to complete...

⏸  5. AUDIT (Tier 3)     Waiting...

⏸  6. UNLOCK (Tier 3)    Waiting...

Current Operation:
   🤖 claude-haiku: Validating clean architecture implementation
   Files affected: 5 total (3 modified, 2 created)
   Tests: 12 new integration tests planned
   Validation: Running cargo fmt, cargo clippy, cargo build

────────────────────────────────────────────────────────────────────────
Status: ▶ IMPLEMENT Stage (2/6) | claude-haiku (2/2) | 4m 35s | Validating...
```

### Example 3: Error with Full Context
```
❌ FAILED: Stage IMPLEMENT - Agent claude-haiku (2/2)

What was being attempted:
   Stage: IMPLEMENT (Tier 2)
   Agent: claude-haiku (validator)
   Operation: Code validation and quality checks
   Duration: 2m 35s before failure

Error:
   Validation failed: cargo clippy found 3 errors

   Error 1: src/domain/auth.rs:45
      unused import: `std::collections::HashMap`

   Error 2: src/domain/auth.rs:112
      variable does not need to be mutable: `user_id`

   Error 3: src/application/handlers.rs:67
      this expression creates a reference which is immediately dereferenced

Context:
   • Previous agent (gpt-codex) completed successfully
   • 5 files affected: 3 modified, 2 created
   • Tests planned: 12 new integration tests
   • cargo fmt: passed ✓
   • cargo build: passed ✓
   • cargo clippy: failed ✗ (3 errors)

Recovery options:
   1. Auto-fix: Run cargo clippy --fix (recommended)
   2. Manual fix: Edit files and retry IMPLEMENT stage
   3. Skip validation: Continue to VALIDATE stage (not recommended)
   4. Abort: Stop pipeline and review

Evidence:
   Output: docs/SPEC-OPS-004-integrated-coder-hooks/evidence/SPEC-KIT-900/implement/
   Agent outputs: Available in consensus DB
   Validation logs: Available in evidence directory

Recommendation: Run `cargo clippy --fix` to auto-fix these issues, then retry.

────────────────────────────────────────────────────────────────────────
Status: ✗ IMPLEMENT Failed | 2m 35s | Validation errors
```

---

**End of SPEC-KIT-926**
