# P6-SYNC Continuation: Phases 4-6

**Generated**: 2025-11-29
**Previous Session**: Phases 2-3 COMPLETE
**Commits Pending**: Single mega-commit after all phases complete

---

## Session Context

### Completed This Session
- **Phase 2: SessionMetrics** - Token usage tracking with sliding window estimation
  - `session_metrics.rs` (265 lines) with 8 passing tests
  - Integrated into SpecAutoState, wired to cost tracking
- **Phase 3: Fault Injection** - Deterministic error testing framework
  - `faults.rs` (270 lines) with 6 passing tests
  - Feature-gated `dev-faults`, 3 fault types (Disconnect, RateLimit, Timeout)

### Files Modified (Uncommitted)
```
codex-rs/spec-kit/src/faults.rs (NEW)
codex-rs/spec-kit/src/lib.rs
codex-rs/spec-kit/Cargo.toml
codex-rs/tui/src/chatwidget/spec_kit/session_metrics.rs (NEW)
codex-rs/tui/src/chatwidget/spec_kit/mod.rs
codex-rs/tui/src/chatwidget/spec_kit/state.rs
codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs
codex-rs/tui/src/chatwidget/spec_kit/agent_retry.rs
codex-rs/tui/Cargo.toml
```

---

## Session Startup Commands

```bash
# 1. Verify state and build
cd ~/code && git status
git diff --stat
~/code/build-fast.sh

# 2. Run tests to confirm baseline
cd ~/code/codex-rs && cargo test -p codex-tui session_metrics -- --nocapture
cd ~/code/codex-rs && cargo test -p codex-spec-kit --features dev-faults -- faults --nocapture

# 3. Load this prompt
# (You're already here!)
```

---

## Phase 4: Branch-Aware Resume Filtering (2-3h) - PRIORITY: QoL

### Goal
When resuming a pipeline, filter conversation history to only include items from the current branch path, avoiding confusion from abandoned branches. **Includes UI integration.**

### Implementation Tasks

#### 4.1 Add branch tracking to agent responses
Location: `codex-rs/tui/src/chatwidget/spec_kit/state.rs`

```rust
/// Branch identifier for pipeline run isolation
#[derive(Debug, Clone)]
pub struct PipelineBranch {
    pub branch_id: String,
    pub created_at: DateTime<Utc>,
    pub parent_branch: Option<String>,  // For nested retries
}

// Add to SpecAutoState:
pub current_branch: Option<PipelineBranch>,
```

#### 4.2 Generate branch IDs on pipeline start
Location: `codex-rs/tui/src/chatwidget/spec_kit/handler.rs`

When `handle_spec_auto()` starts a new pipeline:
```rust
fn generate_branch_id(spec_id: &str) -> String {
    format!("{}-{}-{}", spec_id, Utc::now().format("%Y%m%d%H%M%S"), Uuid::new_v4().simple())
}
```

Initialize in `SpecAutoState::with_quality_gates()`.

#### 4.3 Store branch_id with agent outputs
Location: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`

Add column to `agent_executions` table:
```sql
ALTER TABLE agent_executions ADD COLUMN branch_id TEXT;
```

Update `record_agent_spawn()` and `record_agent_completion()` to include branch_id.

#### 4.4 Filter on resume
Location: `codex-rs/tui/src/chatwidget/spec_kit/consensus_db.rs`

```rust
pub fn get_responses_for_branch(
    &self,
    spec_id: &str,
    stage: &str,
    branch_id: &str,
) -> SqlResult<Vec<AgentExecution>>
```

#### 4.5 UI Integration - Status Bar
Location: `codex-rs/tui/src/bottom_pane/status_bar.rs` (or similar)

Display current branch info:
```
[SPEC-KIT-XXX] Stage: Plan | Branch: 20251129... | Agents: 2/3
```

#### 4.6 Write tests
- `branch_id_generation_unique`
- `filter_responses_by_branch`
- `resume_excludes_old_branches`
- `nested_branch_tracking`

### Acceptance Criteria
- [ ] Branch ID generated on pipeline start
- [ ] Stored with agent outputs in SQLite
- [ ] Filtering on resume working
- [ ] UI shows branch info
- [ ] Tests passing

---

## Phase 5: Device Code Auth - All Providers (2-3h) - PRIORITY: UNBLOCKED

### Goal
Port device code authentication flow from upstream for OAuth-based model providers.
**Scope: ChatGPT, Gemini, and future-proof architecture.**

### Research First

```bash
# Find auth-related files in upstream
ls ~/old/code/code-rs/*/src/*auth* 2>/dev/null
grep -r "device_code\|oauth\|token" ~/old/code/code-rs/ --include="*.rs" | head -30

# Check existing login module
ls -la ~/code/codex-rs/login/src/
```

### Implementation Tasks

#### 5.1 Create provider-agnostic auth trait
Location: `codex-rs/login/src/device_code.rs` (NEW)

```rust
#[async_trait]
pub trait DeviceCodeAuth {
    /// Get device code from provider
    async fn request_device_code(&self) -> Result<DeviceCodeResponse>;

    /// Poll for token completion
    async fn poll_for_token(&self, device_code: &str) -> Result<AuthToken>;

    /// Refresh expired token
    async fn refresh_token(&self, refresh_token: &str) -> Result<AuthToken>;
}

pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}
```

#### 5.2 Implement ChatGPT provider
Location: `codex-rs/login/src/providers/openai.rs` (NEW or update existing)

- OAuth endpoints for OpenAI
- Token storage in secure location
- Refresh flow

#### 5.3 Implement Gemini provider
Location: `codex-rs/login/src/providers/google.rs` (NEW)

- Google OAuth device flow
- GCP credentials handling
- Token refresh

#### 5.4 Provider registry
Location: `codex-rs/login/src/providers/mod.rs`

```rust
pub enum AuthProvider {
    OpenAI,
    Google,
    // Future: Anthropic, etc.
}

pub fn get_auth_handler(provider: AuthProvider) -> Box<dyn DeviceCodeAuth>
```

#### 5.5 CLI integration
Update model selection to trigger auth flow when needed:
- `/model claude-opus-4-5` → Check token, prompt auth if missing
- Display device code and verification URL
- Poll in background

#### 5.6 Token persistence
Location: `codex-rs/login/src/token_store.rs`

- Secure storage (keyring or encrypted file)
- Token refresh on expiry
- Clear tokens on logout

### Acceptance Criteria
- [ ] Device code auth trait defined
- [ ] ChatGPT OAuth working end-to-end
- [ ] Gemini OAuth working end-to-end
- [ ] Token persistence implemented
- [ ] Token refresh on expiry
- [ ] CLI integration complete
- [ ] Tests for auth flows

---

## Phase 6: TokenMetrics UI Integration (1-2h) - PRIORITY: POLISH

### Goal
Wire SessionMetrics from Phase 2 to the TUI status bar for real-time token tracking and predictive estimates.

### Implementation Tasks

#### 6.1 Create TokenMetrics display component
Location: `codex-rs/tui/src/bottom_pane/token_metrics.rs` (NEW)

```rust
pub struct TokenMetricsWidget {
    pub total_input: u64,
    pub total_output: u64,
    pub turn_count: u32,
    pub estimated_next: u64,
    pub context_utilization: f64,  // % of context window used
}

impl TokenMetricsWidget {
    pub fn from_session_metrics(metrics: &SessionMetrics, context_window: u64) -> Self
    pub fn render(&self, area: Rect, buf: &mut Buffer)
}
```

#### 6.2 Integrate into status bar
Location: `codex-rs/tui/src/bottom_pane/` (status bar file)

Display format:
```
Tokens: 12.5k in / 3.2k out | Turn 5 | Est. next: ~4k | Context: 45%
```

Compact format for narrow terminals:
```
12.5k/3.2k | T5 | ~4k | 45%
```

#### 6.3 Wire to ChatWidget state
- Read from `spec_auto_state.session_metrics`
- Update on each `record_agent_costs()` call
- Clear on pipeline reset

#### 6.4 Add context window awareness
- Get model's context window from config
- Calculate utilization percentage
- Warn when approaching limit (>80%)

### Acceptance Criteria
- [ ] TokenMetrics widget created
- [ ] Integrated into status bar
- [ ] Updates in real-time during pipeline
- [ ] Context utilization calculated
- [ ] Warning at high utilization
- [ ] Tests for rendering

---

## Commit Strategy

**Single mega-commit after all phases complete:**

```bash
# After Phase 6 is done and all tests pass:
cd ~/code
git add -A
git commit -m "$(cat <<'EOF'
feat(sync): Add P6-SYNC Phases 2-6 infrastructure

Phase 2: SessionMetrics
- Token usage tracking with sliding window estimation
- Predictive next-prompt estimates
- Integrated into SpecAutoState and cost tracking

Phase 3: Fault Injection Framework
- Feature-gated dev-faults for testing error handling
- 3 fault types: Disconnect, RateLimit, Timeout
- Injection point in agent_retry.rs

Phase 4: Branch-Aware Resume Filtering
- Pipeline branch isolation for clean resume
- SQLite schema update for branch tracking
- UI integration in status bar

Phase 5: Device Code Auth (All Providers)
- Provider-agnostic DeviceCodeAuth trait
- ChatGPT and Gemini OAuth implementations
- Token persistence and refresh

Phase 6: TokenMetrics UI
- Real-time token display in status bar
- Context utilization tracking
- Predictive estimates visualization

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

---

## Quick Reference: File Locations

| Component | Path |
|-----------|------|
| SessionMetrics | `tui/src/chatwidget/spec_kit/session_metrics.rs` |
| Faults | `spec-kit/src/faults.rs` |
| State | `tui/src/chatwidget/spec_kit/state.rs` |
| Agent Retry | `tui/src/chatwidget/spec_kit/agent_retry.rs` |
| Consensus DB | `tui/src/chatwidget/spec_kit/consensus_db.rs` |
| Login module | `login/src/` |
| Status bar | `tui/src/bottom_pane/` |

---

## Execution Order

1. **Phase 4 first** - Branch isolation is foundational for resume reliability
2. **Phase 5 second** - Auth unblocks multi-provider usage
3. **Phase 6 last** - UI polish builds on Phase 2's SessionMetrics

## Estimated Time
- Phase 4: 2-3h (with UI)
- Phase 5: 2-3h (all providers)
- Phase 6: 1-2h (UI polish)
- **Total**: 5-8h

---

## Notes
- Build after each phase: `~/code/build-fast.sh`
- Run all tests before commit: `cargo test -p codex-tui -p codex-spec-kit`
- Keep local-memory updated with milestones

---

**To start next session:**
```
load docs/NEXT-SESSION-P6-SYNC-PHASES-4-6.md
```
