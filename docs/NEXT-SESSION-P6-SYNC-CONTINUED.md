# P6 Sync Continuation Session (Part 2)

**Generated**: 2025-11-29
**Previous**: P6 Part 1 completed Auth Diff + Auto Drive Analysis
**Commit**: `a61b374f0` - docs(sync): Add P6 auth diff and Auto Drive analysis
**Estimated Effort**: 15-20 hours remaining
**Session Goals**: Implement Auto Drive patterns for /speckit.auto reliability

---

## Session Context

### What Was Completed (P6 Part 1)

1. **Auth Module Diff Report** - SYNC-016 unblocked
   - Found 4 of 5 original blockers don't exist in upstream
   - Created `docs/AUTH-MODULE-DIFF-REPORT.md` with migration path
   - Status: BLOCKED → READY

2. **Auto Drive Analysis** - Patterns identified
   - Created `docs/AUTO-DRIVE-SPECKIT-ANALYSIS.md`
   - Identified 3 high-value patterns + 1 QoL improvement
   - Architecture comparison complete

### Implementation Decisions Made

| Decision | Choice |
|----------|--------|
| Priority Order | Auto Drive patterns first, then SYNC-016 |
| Feature Flags | No gates (except dev-faults per upstream) |
| SYNC-018 Scope | Include branch-aware resume |
| Testing | Write tests alongside each pattern |

---

## Implementation Phases

### Phase 1: Decision Sequencing (4-6h) - CRITICAL

**Goal**: Prevent duplicate agent response processing in multi-agent consensus.

**Problem Statement**:
- Multiple agents can return responses out of order
- Retry logic can produce duplicate responses
- No current mechanism to guarantee exactly-once processing
- Risk: Same consensus response processed twice

**Implementation Location**:
- `codex-rs/tui/src/chatwidget/spec_kit/consensus_coordinator.rs`

**Tasks**:
```
[ ] 1.1 Add ConsensusSequence struct
    - decision_seq: AtomicU64 (monotonically increasing)
    - pending_ack_seq: AtomicU64 (awaiting acknowledgment)

[ ] 1.2 Add sequence number to agent responses
    - Modify run_consensus_with_retry to assign seq
    - Add seq field to consensus result type

[ ] 1.3 Add ACK mechanism
    - Create ConsensusAck enum { AckResponse { seq }, StopAck }
    - Gate processing on pending_ack_seq

[ ] 1.4 Add duplicate rejection logic
    - Check seq against processed set
    - Log and skip duplicate responses

[ ] 1.5 Write tests (alongside)
    - test_sequence_assignment
    - test_duplicate_rejection
    - test_out_of_order_handling
    - test_ack_gating
```

**Pattern Source**: `~/old/code/code-rs/code-auto-drive-core/src/auto_coordinator.rs:129-236`

**Success Criteria**:
- [ ] No duplicate consensus artifacts from retried agents
- [ ] Exactly-once processing guaranteed
- [ ] Tests pass for race condition scenarios
- [ ] Existing consensus tests still pass

---

### Phase 2: SessionMetrics (2-3h) - HIGH

**Goal**: Track token usage and enable pre-run cost estimation.

**Problem Statement**:
- `/speckit.auto` can cost $2.70+ per run
- No way to estimate cost before running
- No visibility into per-agent token distribution
- Duplicate detection not tracked centrally

**Implementation Location**:
- New: `codex-rs/spec-kit/src/telemetry/session_metrics.rs`
- Wire into: `codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

**Tasks**:
```
[ ] 2.1 Create session_metrics.rs module
    pub struct PipelineMetrics {
        running_total: TokenUsage,
        per_agent: HashMap<String, TokenUsage>,
        duplicate_responses: u32,
        retry_count: u32,
        recent_prompts: VecDeque<u64>,
        window: usize,
    }

[ ] 2.2 Implement core methods
    - record_turn(agent: &str, usage: &TokenUsage)
    - estimated_next_prompt_tokens() -> u64
    - record_duplicate()
    - estimated_cost() -> f64
    - agent_breakdown() -> Vec<(String, TokenUsage)>

[ ] 2.3 Wire into agent orchestrator
    - Record usage after each agent response
    - Track duplicates during consensus

[ ] 2.4 Add to /speckit.status output
    - Display running totals
    - Show per-agent breakdown

[ ] 2.5 Write tests (alongside)
    - test_record_turn_accumulates
    - test_sliding_window_estimate
    - test_duplicate_counting
    - test_cost_estimation
```

**Pattern Source**: `~/old/code/code-rs/code-auto-drive-core/src/session_metrics.rs:1-125`

**Success Criteria**:
- [ ] Token usage tracked per pipeline run
- [ ] Duplicate responses detected and counted
- [ ] Cost estimation available in status
- [ ] Per-agent breakdown visible

---

### Phase 3: Fault Injection (3-4h) - HIGH

**Goal**: Enable deterministic testing of retry paths.

**Problem Statement**:
- Can't test retry paths without hitting real failures
- Integration tests are flaky due to real API behavior
- No way to test "agent 2 of 3 times out" scenario
- Quality gate recovery paths untestable

**Implementation Location**:
- New: `codex-rs/spec-kit/src/testing/faults.rs`
- Feature flag: `#[cfg(feature = "dev-faults")]`

**Tasks**:
```
[ ] 3.1 Create faults.rs module with feature gate
    #![cfg(feature = "dev-faults")]

    pub enum FaultScope {
        SpecKit,
        Consensus,
        Agent(String),  // Per-agent targeting
    }

    pub enum InjectedFault {
        AgentTimeout,
        ConsensusFailure,
        RateLimited { reset_hint: Option<Duration> },
        EmptyResponse,
    }

[ ] 3.2 Add environment variable parsing
    // SPEC_KIT_FAULTS=agent_timeout:3,consensus_fail:1
    // SPEC_KIT_FAULTS_SCOPE=spec_kit
    // SPEC_KIT_FAULTS_AGENT=gemini-flash
    fn parse_fault_config() -> HashMap<FaultScope, FaultConfig>

[ ] 3.3 Implement fault injection points
    pub fn next_fault(scope: FaultScope) -> Option<InjectedFault>
    pub fn fault_to_error(fault: InjectedFault) -> anyhow::Error

[ ] 3.4 Wire into agent execution path
    - Check for fault before agent spawn
    - Inject configured errors

[ ] 3.5 Add dev-faults feature to Cargo.toml
    [features]
    dev-faults = []

[ ] 3.6 Write integration tests using faults
    - test_agent_timeout_retry
    - test_rate_limit_backoff
    - test_partial_consensus_degradation
```

**Pattern Source**: `~/old/code/code-rs/code-auto-drive-core/src/faults.rs:1-158`

**Success Criteria**:
- [ ] Faults injectable via environment variables
- [ ] Retry paths testable deterministically
- [ ] No production impact (feature-gated)
- [ ] Integration tests use fault injection

---

### Phase 4: Branch-Aware Resume (2-3h) - QoL

**Goal**: Filter session resume by git branch for better multi-branch workflows.

**Problem Statement**:
- `codex resume` shows all sessions regardless of branch
- Confusing when working across feature branches
- Sessions not sorted by recent activity

**Implementation Location**:
- `codex-rs/tui/src/session/` (resume picker)

**Tasks**:
```
[ ] 4.1 Locate upstream implementation
    - Check ~/old/code/code-rs for resume changes
    - Identify branch detection approach

[ ] 4.2 Add branch field to session metadata
    - Capture current branch on session create
    - Store in session file

[ ] 4.3 Add branch filtering to resume picker
    - Filter by current branch by default
    - Add --all flag to show all branches

[ ] 4.4 Sort sessions by latest activity
    - Within filtered set, sort by last_modified

[ ] 4.5 Add branch display in resume UI
    - Show branch name in picker
    - Visual indicator for current branch matches

[ ] 4.6 Write tests
    - test_branch_capture
    - test_branch_filtering
    - test_activity_sorting
```

**Pattern Source**: Upstream CHANGELOG "add branch-aware filtering to `codex resume`" (v0.4.21)

**Success Criteria**:
- [ ] `codex resume` filters by current branch
- [ ] Sessions sorted by most recent activity
- [ ] Branch name visible in resume picker
- [ ] --all flag shows cross-branch sessions

---

### Phase 5: SYNC-016 Device Code Auth (2-3h) - UNBLOCKED

**Goal**: Port headless authentication for SSH/CI environments.

**Pre-Port Verification** (from AUTH-MODULE-DIFF-REPORT.md):
```
[ ] Verify codex_browser crate has global::get_or_create_browser_manager()
[ ] Verify ServerOptions struct is compatible
[ ] Verify persist_tokens_async() is exported from server.rs
```

**Tasks**:
```
[ ] 5.1 Port core auth enhancements (~150 LOC)
    - Add CODEX_API_KEY_ENV_VAR constant
    - Add read_codex_api_key_from_env() function
    - Add RefreshTokenError + RefreshTokenErrorKind types
    - Add classify_refresh_failure() helper
    - Add adopt_rotated_refresh_token_from_disk() method

[ ] 5.2 Port device_code_auth.rs (~180 LOC)
    - Copy from ~/old/code/code-rs/login/src/device_code_auth.rs
    - Apply substitutions: code_ → codex_
    - Update imports

[ ] 5.3 Update login/src/lib.rs
    - Add mod device_code_auth
    - Export run_device_code_login, DeviceCodeSession

[ ] 5.4 Add CLI flag (optional)
    - Add --device-code to login command

[ ] 5.5 Write tests
    - test_device_code_session_creation
    - test_token_polling (mock)
```

**Pattern Source**: `~/old/code/code-rs/login/src/device_code_auth.rs`

**Success Criteria**:
- [ ] `cargo build -p codex-login` succeeds
- [ ] Device code flow works against auth server
- [ ] Cloudflare fallback path handled

---

## Session Workflow

### Startup Commands

```bash
# Load context
cat docs/NEXT-SESSION-P6-SYNC-CONTINUED.md

# Verify P6 Part 1 state
git log --oneline -5
git status --short

# Load reference files
cat docs/AUTH-MODULE-DIFF-REPORT.md
cat docs/AUTO-DRIVE-SPECKIT-ANALYSIS.md

# Verify build
cd ~/code && ~/code/build-fast.sh
```

### Local Memory Queries

```bash
# Check P6 progress
~/.claude/hooks/lm-search.sh "P6 sync auth"

# Check Auto Drive analysis
~/.claude/hooks/lm-search.sh "auto drive speckit"

# Check retry patterns (P5)
~/.claude/hooks/lm-search.sh "retry patterns spec-kit"
```

### Implementation Order

1. **Phase 1**: Decision Sequencing (critical path)
2. **Phase 2**: SessionMetrics (unlocks cost visibility)
3. **Phase 3**: Fault Injection (unlocks testing)
4. **Phase 4**: Branch-Aware Resume (QoL)
5. **Phase 5**: SYNC-016 Device Code Auth (unblocked)

### Commit Strategy

One commit per phase:
```
feat(spec-kit): Add decision sequencing for consensus (P6-SYNC)
feat(spec-kit): Add SessionMetrics for token tracking (P6-SYNC)
feat(spec-kit): Add fault injection framework (P6-SYNC)
feat(session): Add branch-aware resume filtering (P6-SYNC)
feat(login): Port device code auth from upstream (SYNC-016)
```

---

## Verification Checklist

### After Each Phase

```bash
# Build check
cd ~/code/codex-rs && cargo build -p codex-tui --quiet

# Clippy
cargo clippy -p codex-tui --quiet -- -D warnings

# Run new tests
cargo test -p codex-tui -- <test_name> --nocapture

# Run existing tests (regression check)
cargo test -p codex-spec-kit --quiet
```

### End of Session

```
[ ] All phases have commits
[ ] All tests pass
[ ] docs/SYNC-P4-DEFERRED.md updated with completed items
[ ] Local memory updated with milestone
[ ] NEXT-SESSION-P7-SYNC.md created if work remains
```

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Decision sequencing breaks consensus | Test extensively with existing pipeline |
| SessionMetrics overhead | Profile, ensure lazy initialization |
| Fault injection leaks to prod | Strict #[cfg(feature)] + CI check |
| Branch detection fails in detached HEAD | Graceful fallback to "no branch" |
| Device code auth browser dependency | Verify codex_browser crate first |

---

## Reference Files

| File | Purpose |
|------|---------|
| `docs/AUTH-MODULE-DIFF-REPORT.md` | SYNC-016 migration path |
| `docs/AUTO-DRIVE-SPECKIT-ANALYSIS.md` | Pattern analysis |
| `docs/SYNC-P4-DEFERRED.md` | Tracking document |
| `~/old/code/code-rs/code-auto-drive-core/src/auto_coordinator.rs` | Decision sequencing reference |
| `~/old/code/code-rs/code-auto-drive-core/src/session_metrics.rs` | SessionMetrics reference |
| `~/old/code/code-rs/code-auto-drive-core/src/faults.rs` | Fault injection reference |
| `~/old/code/code-rs/login/src/device_code_auth.rs` | Device code auth reference |

---

## Success Criteria Summary

| Phase | Primary Success Metric |
|-------|----------------------|
| 1. Decision Sequencing | No duplicate consensus artifacts |
| 2. SessionMetrics | Cost estimation in /speckit.status |
| 3. Fault Injection | Integration tests use deterministic faults |
| 4. Branch Resume | Resume filters by current branch |
| 5. SYNC-016 | Device code login works in SSH |

---

*Ready to begin. Start with Phase 1: Decision Sequencing.*
