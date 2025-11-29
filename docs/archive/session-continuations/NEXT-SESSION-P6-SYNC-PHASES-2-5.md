# P6-SYNC Continuation: Phases 2-5

**Generated**: 2025-11-29
**Previous Session**: Phase 1 (Decision Sequencing) - COMPLETE
**Commit**: `097dfaffe` feat(sync): Add decision sequencing for consensus (P6-SYNC Phase 1)

---

## Session Startup Commands

```bash
# 1. Verify build and recent changes
cd ~/code && git log --oneline -3
~/code/build-fast.sh

# 2. Load reference files
cat ~/old/code/code-rs/code-auto-drive-core/src/session_metrics.rs
cat ~/old/code/code-rs/code-auto-drive-core/src/faults.rs
```

---

## Phase 2: SessionMetrics (2-3h) - PRIORITY: HIGH

### Goal
Port Auto Drive's SessionMetrics pattern to spec-kit for tracking token usage, costs, and providing predictive estimates.

### User Preferences (from previous session)
- **Full integration** with existing cost_recorded_agents
- Wire to TokenMetrics events for UI display

### Reference Implementation
`~/old/code/code-rs/code-auto-drive-core/src/session_metrics.rs` (~179 lines)

### Implementation Tasks

#### 2.1 Create SessionMetrics struct
Location: `codex-rs/tui/src/chatwidget/spec_kit/session_metrics.rs` (new file)

```rust
// Port from Auto Drive - key fields:
pub struct SessionMetrics {
    running_total: TokenUsage,
    last_turn: TokenUsage,
    turn_count: u32,
    replay_updates: u32,
    duplicate_items: u32,
    recent_prompt_tokens: VecDeque<u64>,  // Sliding window
    window: usize,
}
```

Key methods to implement:
- `record_turn(&mut self, usage: &TokenUsage)` - Update totals and window
- `estimated_next_prompt_tokens(&self) -> u64` - Sliding window average
- `blended_total(&self) -> u64` - Combined metric
- `record_duplicate_items(&mut self, count: usize)` - Track duplicates
- `sync_absolute(&mut self, ...)` - Sync from external source
- `reset(&mut self)` - Clear for new pipeline run

#### 2.2 Integrate with SpecAutoState
Location: `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

Add field to SpecAutoState:
```rust
pub session_metrics: SessionMetrics,
```

Initialize in `with_quality_gates()` constructor.

#### 2.3 Wire to cost tracking
Location: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

Connect to existing `cost_recorded_agents` HashMap:
- When agent cost is recorded, also update SessionMetrics
- After consensus completion, emit TokenMetrics summary

#### 2.4 Add TokenMetrics event (optional UI)
If time permits, add event to display in status bar:
```rust
pub struct TokenMetricsEvent {
    pub total_usage: TokenUsage,
    pub last_turn_usage: TokenUsage,
    pub turn_count: u32,
    pub estimated_next: u64,
}
```

#### 2.5 Write tests
Port tests from Auto Drive session_metrics.rs:
- `record_turn_tracks_totals_and_estimate`
- `sync_absolute_resets_window`
- `record_replay_increments_counter`

### Acceptance Criteria
- [ ] SessionMetrics struct with all methods
- [ ] Integrated into SpecAutoState
- [ ] Connected to cost tracking
- [ ] Tests passing
- [ ] Build clean

---

## Phase 3: Fault Injection Framework (3-4h) - PRIORITY: HIGH

### Goal
Enable deterministic testing of error handling via injectable faults (disconnect, rate limit, timeout).

### User Preferences (from previous session)
- **All 3 fault types**: Disconnect, RateLimit (429), Timeout

### Reference Implementation
`~/old/code/code-rs/code-auto-drive-core/src/faults.rs` (~158 lines)

### Implementation Tasks

#### 3.1 Create faults module
Location: `codex-rs/spec-kit/src/faults.rs` (new file, feature-gated)

```rust
#![cfg(feature = "dev-faults")]

pub enum FaultScope {
    SpecKit,  // Renamed from AutoDrive
}

pub enum InjectedFault {
    Disconnect,
    RateLimit { reset_hint: Option<FaultReset> },
    Timeout { duration_ms: u64 },  // Extension
}
```

Key functions:
- `next_fault(scope: FaultScope) -> Option<InjectedFault>`
- `fault_to_error(fault: InjectedFault) -> anyhow::Error`

#### 3.2 Environment variable parsing
```bash
CODEX_FAULTS_SCOPE=spec_kit
CODEX_FAULTS=disconnect:3,429:1,timeout:2
CODEX_FAULTS_429_RESET=now+30s
```

#### 3.3 Add feature flag to Cargo.toml
Location: `codex-rs/spec-kit/Cargo.toml`
```toml
[features]
dev-faults = []
```

#### 3.4 Inject into agent execution
Location: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

Add fault check before/after agent spawns:
```rust
#[cfg(feature = "dev-faults")]
if let Some(fault) = crate::faults::next_fault(FaultScope::SpecKit) {
    return Err(fault_to_error(fault));
}
```

#### 3.5 Write tests
- Test fault counter decrement
- Test error conversion
- Test env var parsing
- Test scope filtering

### Acceptance Criteria
- [ ] Faults module with feature gate
- [ ] All 3 fault types working
- [ ] Env var configuration
- [ ] Integration points in agent orchestrator
- [ ] Tests passing with `--features dev-faults`

---

## Phase 4: Branch-Aware Resume Filtering (2-3h) - PRIORITY: QoL

### Goal
When resuming a pipeline, filter conversation history to only include items from the current branch path, avoiding confusion from abandoned branches.

### Reference
Auto Drive handles this via `branch_id` tracking on conversation items.

### Implementation Tasks

#### 4.1 Add branch tracking to agent responses
Location: `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

```rust
pub struct AgentResponse {
    pub agent_name: String,
    pub content: String,
    pub branch_id: Option<String>,  // New field
    pub timestamp: DateTime<Utc>,
}
```

#### 4.2 Generate branch IDs
- New branch ID when pipeline starts
- Same branch ID for all agents in that run
- Store in SpecAutoState

#### 4.3 Filter on resume
When loading cached responses, filter by current branch_id:
```rust
fn filter_responses_for_branch(
    responses: &[(String, String)],
    current_branch: &str,
) -> Vec<(String, String)>
```

#### 4.4 Update SQLite schema (if needed)
Add `branch_id` column to agent_outputs table.

### Acceptance Criteria
- [ ] Branch ID generation and tracking
- [ ] Filtering logic implemented
- [ ] SQLite schema updated
- [ ] Resume correctly filters old branches
- [ ] Tests for branch filtering

---

## Phase 5: SYNC-016 Device Code Auth (2-3h) - PRIORITY: UNBLOCKED

### Goal
Port device code authentication flow from upstream for OAuth-based model providers.

### Reference
Upstream auth handling in `code-rs/` auth modules.

### Implementation Tasks

#### 5.1 Research upstream implementation
```bash
ls ~/old/code/code-rs/*/src/*auth* 2>/dev/null || echo "Find auth files"
grep -r "device_code" ~/old/code/code-rs/ --include="*.rs" | head -20
```

#### 5.2 Identify integration points
- Login flow
- Token refresh
- Provider-specific handling

#### 5.3 Port relevant code
- Device code request
- Polling logic
- Token storage

#### 5.4 Test with real provider
- Test OAuth flow end-to-end
- Verify token persistence

### Acceptance Criteria
- [ ] Device code auth working for at least one provider
- [ ] Token refresh handling
- [ ] Error handling for expired/invalid codes
- [ ] Documentation updated

---

## Execution Order

1. **Phase 2 first** - SessionMetrics is foundational for observability
2. **Phase 3 second** - Fault injection enables testing of retry patterns
3. **Phase 4 third** - QoL improvement, lower priority
4. **Phase 5 last** - SYNC-016 unblocked but separate concern

## Estimated Total Time
- Phase 2: 2-3h
- Phase 3: 3-4h
- Phase 4: 2-3h
- Phase 5: 2-3h
- **Total**: 9-13h

## Notes
- Each phase should have its own commit
- Run tests after each phase: `cargo test -p codex-tui`
- Keep memory updated with milestones

---

## Quick Reference: File Locations

| Component | Path |
|-----------|------|
| State (SpecAutoState) | `codex-rs/tui/src/chatwidget/spec_kit/state.rs` |
| Pipeline Coordinator | `codex-rs/tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` |
| Agent Orchestrator | `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs` |
| Consensus Coordinator | `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs` |
| Consensus DB | `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs` |
| Spec-kit crate | `codex-rs/spec-kit/src/` |
| Auto Drive Reference | `~/old/code/code-rs/code-auto-drive-core/src/` |

---

**To start next session:**
```
load docs/NEXT-SESSION-P6-SYNC-PHASES-2-5.md
```
