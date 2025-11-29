# P6 Sync Continuation Session

**Generated**: 2025-11-29
**Previous Session**: P5 Sync (Auto Drive retry patterns ported)
**Estimated Effort**: 13-19 hours
**Priority Order**: Auth unblock → Auto Drive patterns → Branch-aware resume

---

## Session Objective

Complete the Auto Drive pattern integration and unblock Device Code Auth:

1. **Auth module diff report** - Unblock SYNC-016 (Priority 1)
2. **Decision sequencing** - Consensus coordinator enhancement (Priority 2)
3. **SessionMetrics** - Token/cost tracking for spec-kit (Priority 3)
4. **Fault injection framework** - Deterministic retry testing (Priority 4)
5. **SYNC-018 Branch-aware resume** - QoL improvement (Priority 5)

---

## Phase 1: Auth Module Diff Report (2-3h)

### Goal
Create comprehensive diff between fork and upstream auth modules to unblock SYNC-016 (Device Code Auth).

### Missing Dependencies (from SYNC-P4-DEFERRED.md)
1. `AuthCredentialsStoreMode` enum in codex_core::auth
2. `save_auth` helper function
3. `cli_auth_credentials_store_mode` field in ServerOptions
4. `ensure_workspace_allowed` function
5. `CODEX_API_KEY_ENV_VAR` constant

### Investigation Steps

```bash
# Fork auth module
ls -la ~/code/codex-rs/core/src/auth/
cat ~/code/codex-rs/core/src/auth/mod.rs

# Upstream auth module (if available)
ls -la ~/old/code/code-rs/core/src/auth/ 2>/dev/null
ls -la ~/old/code/code-rs/login/src/ 2>/dev/null

# Check upstream device_code_auth.rs
cat ~/old/code/code-rs/login/src/device_code_auth.rs 2>/dev/null | head -50
```

### Deliverable
- `docs/AUTH-MODULE-DIFF-REPORT.md` with:
  - Types present in fork vs upstream
  - Functions present in fork vs upstream
  - Breaking vs additive changes
  - Migration path for Device Code Auth
  - Go/no-go decision for SYNC-016

### Success Criteria
- [ ] All 5 missing dependencies documented
- [ ] Migration path identified
- [ ] SYNC-016 status updated (blocked → ready OR blocked → needs-work)

---

## Phase 2: Decision Sequencing (4-6h)

### Goal
Port Auto Drive's decision sequencing pattern to spec-kit consensus coordinator to prevent race conditions and duplicate processing.

### Pattern Source
- Upstream: `~/old/code/code-rs/code-auto-drive-core/src/auto_coordinator.rs`
- Docs: `~/old/code/docs/auto-drive-reliability.md`

### Key Concepts
```rust
// Decision sequence for exactly-once processing
struct ConsensusSequence {
    decision_seq: u64,      // Monotonically increasing
    pending_ack_seq: u64,   // Awaiting acknowledgment
}

// ACK flow
enum ConsensusAck {
    AckDecision { seq: u64 },
    StopAck,
}
```

### Implementation Location
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs`

### Tasks
1. [ ] Add `ConsensusSequence` struct to track decision ordering
2. [ ] Add `pending_ack_seq` gating before processing next agent response
3. [ ] Add ACK mechanism for completed consensus rounds
4. [ ] Add tests for out-of-order response handling
5. [ ] Add tests for duplicate response rejection

### Success Criteria
- [ ] No duplicate consensus artifacts from retried agents
- [ ] Exactly-once processing guaranteed
- [ ] Tests cover race condition scenarios

---

## Phase 3: SessionMetrics (2-3h)

### Goal
Port Auto Drive's SessionMetrics for spec-kit telemetry - token tracking, duplicate detection, cost estimation.

### Pattern Source
- Upstream: `~/old/code/code-rs/code-auto-drive-core/src/session_metrics.rs`

### Key Features
```rust
pub struct SessionMetrics {
    running_total: TokenUsage,      // Cumulative tokens
    last_turn: TokenUsage,          // Most recent turn
    turn_count: u32,                // Pipeline turn counter
    replay_updates: u32,            // Replay/retry counter
    duplicate_items: u32,           // Dedup tracking
    recent_prompt_tokens: VecDeque<u64>, // Sliding window
}

impl SessionMetrics {
    pub fn estimated_next_prompt_tokens(&self) -> u64;  // Cost prediction
    pub fn record_duplicate_items(&mut self, count: usize);
}
```

### Implementation Location
- New: `codex-rs/spec-kit/src/telemetry/session_metrics.rs`
- Wire into: `spec_kit/agent_orchestrator.rs`

### Tasks
1. [ ] Create `session_metrics.rs` module
2. [ ] Implement token tracking with sliding window
3. [ ] Add duplicate detection counters
4. [ ] Wire into agent orchestrator
5. [ ] Add cost estimation for pipeline runs
6. [ ] Add tests

### Success Criteria
- [ ] Token usage tracked per pipeline run
- [ ] Duplicate responses detected and counted
- [ ] Cost estimation available via `/speckit.status`

---

## Phase 4: Fault Injection Framework (3-4h)

### Goal
Port Auto Drive's fault injection for deterministic retry testing.

### Pattern Source
- Upstream: `~/old/code/code-rs/code-auto-drive-core/src/faults.rs`

### Key Features
```rust
// Environment-driven fault injection
// SPEC_KIT_FAULTS=agent_timeout:3,consensus_fail:1
// SPEC_KIT_FAULTS_SCOPE=spec_kit

pub enum FaultScope {
    SpecKit,
}

pub enum InjectedFault {
    AgentTimeout,
    ConsensusFailure,
    RateLimited { reset_hint: Option<Duration> },
}

pub fn next_fault(scope: FaultScope) -> Option<InjectedFault>;
```

### Implementation Location
- New: `codex-rs/spec-kit/src/testing/faults.rs`
- Feature flag: `#[cfg(feature = "dev-faults")]`

### Tasks
1. [ ] Create `faults.rs` module with feature gate
2. [ ] Implement `FaultScope::SpecKit`
3. [ ] Implement fault types (AgentTimeout, ConsensusFailure, RateLimited)
4. [ ] Add environment variable parsing
5. [ ] Wire into agent execution path
6. [ ] Add integration tests using fault injection

### Success Criteria
- [ ] Faults injectable via environment variables
- [ ] Retry paths testable deterministically
- [ ] No production impact (feature-gated)

---

## Phase 5: SYNC-018 Branch-Aware Resume (2-3h)

### Goal
Add branch filtering to session resume for better multi-branch workflow support.

### Pattern Source
- Upstream CHANGELOG: "add branch-aware filtering to `codex resume`" (v0.4.21)

### Key Features
```rust
// Filter sessions by git branch
pub struct SessionFilter {
    branch: Option<String>,
    sort_by: SortOrder,  // LatestActivity
}

// Resume picker shows branch context
// Sessions sorted by last activity within branch
```

### Implementation Location
- `codex-rs/tui/src/session/` (resume picker)
- May need git integration for branch detection

### Tasks
1. [ ] Locate upstream implementation for reference
2. [ ] Add branch field to session metadata
3. [ ] Add branch filtering to resume picker
4. [ ] Sort sessions by latest activity
5. [ ] Add branch display in resume UI
6. [ ] Add tests

### Success Criteria
- [ ] `codex resume` filters by current branch
- [ ] Sessions sorted by most recent activity
- [ ] Branch name visible in resume picker

---

## Local Memory Queries

```bash
# Check P5 completion
~/.claude/hooks/lm-search.sh "P5 sync retry patterns"

# Check auth-related memories
~/.claude/hooks/lm-search.sh "auth module device code"

# Check consensus coordinator memories
~/.claude/hooks/lm-search.sh "consensus coordinator"

# Check overall sync status
~/.claude/hooks/lm-search.sh "upstream sync milestone"
```

---

## Files to Load

1. `~/.claude/CLEARFRAME.md` - Operating mode
2. `docs/NEXT-SESSION-P6-SYNC.md` - This document
3. `docs/SYNC-P4-DEFERRED.md` - Deferred items tracker
4. `docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md` - Full gap analysis
5. `~/old/code/docs/auto-drive-reliability.md` - Decision sequencing reference
6. `codex-rs/spec-kit/src/retry/strategy.rs` - P5 retry enhancements (reference)

---

## Reference Commits (P5)

```
35355ffa9 feat(sync): Port Auto Drive retry patterns to spec-kit (P5-SYNC)
5ea5bfdb0 docs(sync): Add P4/P5 session and deferred tracking docs
c5ea37a71 feat(sync): Complete SYNC-009 footer module integration
```

---

## Uncommitted State Check

Before starting, verify clean state:
```bash
git status --short
# Expected: Only docs/NEXT-SESSION-P6-SYNC.md (this file) if any
```

---

## Session Start Commands

```bash
# Load context
load ~/.claude/CLEARFRAME.md
load docs/NEXT-SESSION-P6-SYNC.md

# Verify P5 state
cd ~/code/codex-rs && cargo test -p codex-spec-kit -- retry --quiet
git log --oneline -5

# Begin Phase 1 (Auth Diff)
ls -la codex-rs/core/src/auth/
```

---

## Success Criteria Summary

### Phase 1 (Auth Diff)
- [ ] `docs/AUTH-MODULE-DIFF-REPORT.md` created
- [ ] All 5 missing dependencies documented
- [ ] SYNC-016 status updated

### Phase 2 (Decision Sequencing)
- [ ] `ConsensusSequence` implemented
- [ ] ACK gating prevents duplicates
- [ ] Race condition tests pass

### Phase 3 (SessionMetrics)
- [ ] Token tracking operational
- [ ] Cost estimation available
- [ ] Wired into orchestrator

### Phase 4 (Fault Injection)
- [ ] Feature-gated implementation
- [ ] Environment-driven injection
- [ ] Integration tests use faults

### Phase 5 (Branch Resume)
- [ ] Branch filtering works
- [ ] Sessions sorted by activity
- [ ] Branch visible in UI

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Auth module divergence too large | Document minimum viable port, defer full sync |
| Decision sequencing breaks existing consensus | Add feature flag, test extensively before enabling |
| SessionMetrics overhead | Benchmark, make optional via config |
| Fault injection leaks to prod | Strict feature gate, CI ensures flag off |
| Branch detection fails in detached HEAD | Graceful fallback to "no branch" |

---

## Post-Session

After completing P6:
1. Update `docs/SYNC-P4-DEFERRED.md` with completed items
2. Create `docs/NEXT-SESSION-P7-SYNC.md` if work remains
3. Store milestone in local-memory
4. Consider creating PR for upstream-compatible changes

---

*Generated by P5 session analysis*
