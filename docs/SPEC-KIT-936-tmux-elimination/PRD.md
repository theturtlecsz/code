# PRD: Tmux Elimination & Async Orchestration

**SPEC-ID**: SPEC-KIT-936
**Created**: 2025-11-13
**Status**: Draft - **HIGH PRIORITY**
**Priority**: **P1** (Performance + Technical Debt)
**Owner**: Code
**Estimated Effort**: 45-65 hours (2-3 weeks)
**Dependencies**: None (can proceed independently)
**Blocks**: SPEC-934 filesystem cleanup benefits from this

---

## 🔥 Executive Summary

**Current State**: Agent spawning uses tmux sessions and panes for process management, adding significant overhead. Estimated 93% of 7s orchestration time (~6.5s) spent on tmux session creation, pane initialization, and stability polling. Filesystem collection provides legacy fallback (duplicates native orchestrator path). Observable execution via `tmux attach` useful for debugging but comes at performance cost.

**Proposed State**: Direct async API calls to provider CLIs eliminate tmux overhead entirely. Target: 6.5s → 0.1s (65× speedup). Filesystem collection removed (native orchestrator only). OAuth2 device code flows investigated for non-interactive execution. Trade-off: Lose observable tmux panes for debugging (require alternative diagnostics).

**Impact**:
- ✅ 65× faster agent spawning (6.5s → 0.1s target)
- ✅ Eliminates filesystem collection duplication
- ✅ Simpler architecture (no tmux session management)
- ⚠️ Measurement gap: Claims ESTIMATED not MEASURED (proceed with risk acceptance)

**Source**: SPEC-931A architectural analysis identified tmux overhead (Q3, Q71-Q74). Measurement gap acknowledged in holistic analysis (Block 1).

**Alternative Rejected**: Pre-warm tmux sessions (Q39 NO-GO - doesn't solve root cause, adds complexity).

---

## 1. Problem Statement

### Issue #1: Tmux Orchestration Overhead (CRITICAL - ESTIMATED)

**Current Behavior** (SPEC-931A phase1-inventory.md:1145-1170):
```rust
// Phase 1: Create tmux session (~2-3s ESTIMATED)
tmux new-session -d -s "agent-session-{id}"

// Phase 2: Create panes per agent (~1-2s each × 3 agents = 3-6s ESTIMATED)
tmux split-window -t "agent-session-{id}"
tmux send-keys -t "agent-session-{id}:0.0" "agent-cli execute ..."

// Phase 3: Poll for stability (~0.5-1s ESTIMATED)
loop {
    tmux list-panes -t "agent-session-{id}"
    if stable { break }
}

// Total ESTIMATED: 6.5-10s per quality gate
```

**Evidence Gap** (QUESTION-CONSOLIDATION-ANALYSIS.md:254-274):
```
MEASUREMENT GAP IDENTIFIED:
- Claim: "93% overhead (6.5s of 7s total)"
- Reality: ESTIMATED, not MEASURED (no Instant::now() instrumentation)
- Evidence: Session reports show 77s total, but no per-step breakdown
- Statistical rigor: Needs n≥10 runs with mean±stddev

DECISION: Proceed with tmux elimination despite measurement gap
- Rationale: Even if estimate is 50% off, still significant speedup (3-4s savings)
- Target: 6.5s → 0.1s (65× speedup)
- Risk: Acceptable (worst case: 3× speedup instead of 65×)

POST-IMPLEMENTATION: Add instrumentation to validate actual gains (SPEC-940)
```

**Real-World Impact**:
- `/speckit.auto` pipeline: 6 stages × 6.5s overhead = 39s wasted on tmux
- Quality gate execution: 3 agents × 6.5s = 19.5s overhead per gate
- User experience: Noticeable delay (6.5s is human-perceptible)

**Frequency**: 10 quality gates/day = 65s/day overhead = 6.5 hours/year wasted.

---

### Issue #2: Filesystem Collection Duplication (MEDIUM)

**Current Architecture** (SPEC-931A phase1-dataflows.md:723-746):

**Native Orchestrator Path** (preferred):
1. Agent executes → writes to AGENT_MANAGER (HashMap)
2. TUI polls AGENT_MANAGER directly (in-memory, instant)
3. Results rendered in TUI

**Legacy Filesystem Path** (fallback):
1. Agent executes → writes to `~/.code/agents/{agent_id}/result.txt`
2. TUI scans filesystem (`fetch_agent_payloads_from_filesystem()`)
3. Results parsed from text files

**Duplication Cost**:
- Two collection paths to maintain
- Filesystem I/O overhead (~50-100ms to scan directory + read files)
- Confusion: Which path is source of truth?
- Dead code: Legacy path deprecated after native orchestrator adoption

**Proposed**: Remove filesystem collection entirely (SPEC-931J approved, Q206-Q223).

---

### Issue #3: Observable Execution Trade-Off (HIGH)

**Current Benefit** (tmux attach for debugging):
```bash
# Developer can attach to live agent execution
tmux attach -t "agent-session-spec-936"

# See real-time output in panes (useful for debugging)
# - Agent prompts
# - API responses
# - Error messages
```

**Value**:
- Transparent execution (see what agents are doing)
- Live debugging (no need to wait for completion)
- Error diagnosis (inspect failures in real-time)

**Proposed Alternative** (after tmux elimination):
- Structured logging (tracing::info! with agent context)
- Evidence files (capture prompts, responses, errors)
- TUI log viewer (render agent execution history)
- Manual debug mode: `codex-tui --debug-agent <agent_id>` (spawn with verbose logging)

**Trade-Off**: Lose real-time pane visibility → Gain 65× speedup + diagnostic logs.

---

### Issue #4: OAuth2 Non-Interactive Execution Gap (MEDIUM)

**Context**: Some provider CLIs (Google, GitHub) require OAuth2 authentication.

**Current Flow** (interactive prompts):
```bash
# Google CLI authentication
gcloud auth login
# Opens browser → User consents → Token saved

# Problem: Can't run in tmux session (no browser access)
# Current workaround: Pre-authenticate manually
```

**Non-Interactive Challenge**:
- Tmux sessions don't have browser access
- Direct async calls also can't open browsers
- Need programmatic authentication

**OAuth2 Device Code Flow** (RFC 8628):
```
1. App requests device code from provider
2. Provider returns: device_code + user_code + verification_url
3. App displays: "Visit https://example.com/device and enter code: ABC-123"
4. User completes authentication in browser (separate session)
5. App polls token endpoint until user confirms
6. Provider issues access_token → App can execute
```

**Investigation Needed**:
- Which providers support device code flow? (Google ✅, Anthropic ?, OpenAI ?)
- Fallback for providers without device code? (Manual pre-auth + refresh tokens)
- Timeout handling: User has 15 minutes to confirm, then retry

**Scope**: +5h investigation, +3-4h implementation if feasible.

---

## 2. Proposed Solution

### Component 1: Direct Async API Calls (CRITICAL - 20-30h)

**Implementation**:
```rust
// OLD (quality_gate_handler.rs)
async fn spawn_agent_with_tmux(agent: &str) -> Result<AgentHandle> {
    // Create tmux session
    Command::new("tmux")
        .args(["new-session", "-d", "-s", &session_id])
        .spawn()?
        .wait_with_output().await?;

    // Create pane
    Command::new("tmux")
        .args(["split-window", "-t", &session_id])
        .spawn()?
        .wait_with_output().await?;

    // Send command to pane
    Command::new("tmux")
        .args(["send-keys", "-t", &pane_id, &agent_command])
        .spawn()?
        .wait_with_output().await?;

    // Poll for stability
    loop {
        let status = check_tmux_pane_status(&pane_id).await?;
        if status.stable { break }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(AgentHandle { pane_id, session_id })
}

// NEW (direct async execution)
async fn spawn_agent_direct(agent: &str, config: &AgentConfig) -> Result<AgentHandle> {
    let agent_cli = resolve_agent_cli(agent, config)?; // e.g., "gcloud", "anthropic", "openai"

    // Spawn process directly (no tmux wrapper)
    let mut child = tokio::process::Command::new(&agent_cli.path)
        .args(&agent_cli.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Stream output to AGENT_MANAGER (in-memory)
    let agent_id = generate_agent_id();
    AGENT_MANAGER.lock().unwrap().insert(agent_id.clone(), AgentState::Running {
        pid: child.id().unwrap(),
        started_at: Utc::now(),
    });

    // Capture stdout/stderr asynchronously
    tokio::spawn(async move {
        let stdout = BufReader::new(child.stdout.take().unwrap());
        while let Some(line) = stdout.lines().next_line().await? {
            AGENT_MANAGER.lock().unwrap().append_output(&agent_id, line);
        }
    });

    Ok(AgentHandle { agent_id, child })
}
```

**Key Changes**:
1. **No tmux** - Direct `tokio::process::Command::spawn()`
2. **Async I/O** - Stream stdout/stderr to AGENT_MANAGER (no filesystem)
3. **Instant spawn** - No session creation, pane initialization, stability polling
4. **In-memory state** - All output in HashMap (60 FPS TUI rendering)

**Performance Target**:
- Tmux overhead: ~6.5s (current)
- Direct spawn: ~0.1s (target)
- **Speedup**: 65× faster

**Error Handling**:
- CLI not found → Clear error message ("Install {agent_cli} to use {agent}")
- OAuth2 required → Guide user to `--auth` command or device code flow
- Process crash → Capture exit code, stderr, store in AGENT_MANAGER

---

### Component 2: Filesystem Collection Removal (MEDIUM - 5-7h)

**Deprecated Methods** (codex-core/src/orchestrator.rs):
```rust
// REMOVE: fetch_agent_payloads_from_filesystem()
// REMOVE: scan_agent_result_files()
// REMOVE: parse_result_txt()
```

**Files to Delete**:
- `~/.code/agents/{agent_id}/result.txt` (legacy output files)
- `~/.code/agents/{agent_id}/prompt.txt` (legacy input files)

**Migration**:
```rust
// Check if any quality gates still use legacy path
let legacy_usage = db.execute(
    "SELECT COUNT(*) FROM agent_executions WHERE filesystem_collection = 1",
    [],
)?;

if legacy_usage > 0 {
    warn!("Found {} agents using legacy filesystem collection. Migrating...", legacy_usage);
    // Migrate to native orchestrator (re-spawn if needed)
}

// Remove legacy directory
std::fs::remove_dir_all("~/.code/agents/")?;
```

**Validation**:
- All quality gate tests pass without filesystem collection
- No references to `fetch_agent_payloads_from_filesystem()` in codebase
- `~/.code/agents/` directory removed successfully

---

### Component 3: OAuth2 Device Code Flow Investigation (MEDIUM - 8-12h)

**Phase A: Provider Support Investigation (3-4h)**

Research which providers support RFC 8628 device code flow:

| Provider | Device Code Support | Documentation | Implementation Complexity |
|----------|---------------------|---------------|---------------------------|
| Google (Gemini) | ✅ Yes | https://developers.google.com/identity/protocols/oauth2/limited-input-device | LOW (official library) |
| Anthropic | ⏳ Unknown | (investigate) | MEDIUM (custom implementation) |
| OpenAI | ⏳ Unknown | (investigate) | MEDIUM (custom implementation) |
| GitHub Copilot | ✅ Yes | https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/generating-a-user-access-token-for-a-github-app#using-the-device-flow-to-generate-a-user-access-token | LOW (official library) |

**Phase B: Implementation (if supported) (5-8h)**

```rust
// Example: Google OAuth2 device code flow
async fn authenticate_google_device_flow() -> Result<AccessToken> {
    // Step 1: Request device code
    let device_code_response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/device/code")
        .form(&[
            ("client_id", GOOGLE_CLIENT_ID),
            ("scope", "https://www.googleapis.com/auth/generative-language"),
        ])
        .send().await?
        .json::<DeviceCodeResponse>().await?;

    // Step 2: Display user instructions
    println!("🔐 Google Authentication Required");
    println!("   1. Visit: {}", device_code_response.verification_url);
    println!("   2. Enter code: {}", device_code_response.user_code);
    println!("   3. Waiting for confirmation (15 minutes)...");

    // Step 3: Poll token endpoint
    let poll_interval = device_code_response.interval; // Usually 5 seconds
    let expires_at = Instant::now() + Duration::from_secs(device_code_response.expires_in);

    loop {
        if Instant::now() > expires_at {
            return Err("Device code expired (user did not authenticate within 15 minutes)");
        }

        tokio::time::sleep(Duration::from_secs(poll_interval)).await;

        let token_response = reqwest::Client::new()
            .post("https://oauth2.googleapis.com/token")
            .form(&[
                ("client_id", GOOGLE_CLIENT_ID),
                ("client_secret", GOOGLE_CLIENT_SECRET),
                ("device_code", &device_code_response.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send().await?;

        match token_response.status() {
            StatusCode::OK => {
                let token = token_response.json::<AccessToken>().await?;
                println!("✅ Authentication successful!");
                return Ok(token);
            }
            StatusCode::BAD_REQUEST => {
                // "authorization_pending" - user hasn't confirmed yet, keep polling
                continue;
            }
            _ => {
                return Err(format!("Unexpected status: {}", token_response.status()));
            }
        }
    }
}
```

**Fallback Strategy** (if device code not supported):
1. **Manual Pre-Authentication**: User runs `codex-tui --auth <provider>` once
2. **Refresh Tokens**: Store long-lived refresh tokens, rotate automatically
3. **Token Expiry Handling**: Detect 401 errors, prompt re-auth
4. **Documentation**: Clear guide on authentication setup per provider

---

### Component 4: Alternative Diagnostics (MEDIUM - 7-10h)

**Structured Logging** (replace tmux observability):
```rust
// In spawn_agent_direct()
tracing::info!(
    agent_id = %agent_id,
    agent_name = %agent,
    command = %agent_cli.path,
    args = ?agent_cli.args,
    "Spawning agent"
);

// In output streaming
tracing::debug!(
    agent_id = %agent_id,
    line = %line,
    "Agent output"
);

// In completion
tracing::info!(
    agent_id = %agent_id,
    exit_code = %exit_code,
    elapsed_ms = %elapsed.as_millis(),
    "Agent completed"
);
```

**TUI Log Viewer** (new widget):
```rust
// Show live agent execution logs in TUI
struct AgentLogViewer {
    agent_id: String,
    log_buffer: VecDeque<LogEntry>,
    filter_level: LogLevel,
}

impl AgentLogViewer {
    fn render(&self, frame: &mut Frame, area: Rect) {
        // Render log lines in scrollable list
        // Color-code by level (INFO=green, WARN=yellow, ERROR=red)
        // Filter by agent_id, stage, timestamp
    }
}
```

**Evidence Files** (persistent diagnostics):
```
~/.code/evidence/SPEC-KIT-936/validate/
  ├── gemini_prompt.md       (agent input)
  ├── gemini_response.json   (agent output)
  ├── gemini_execution.log   (stdout/stderr)
  ├── claude_prompt.md
  ├── claude_response.json
  └── claude_execution.log
```

**Manual Debug Mode**:
```bash
# Verbose logging for specific agent
codex-tui --debug-agent gemini

# Attaches live log stream to terminal (like tmux attach)
# Shows:
# - Prompt sent
# - API calls made
# - Response received
# - Errors/warnings
```

---

## 3. Acceptance Criteria

### AC1: Tmux Elimination ✅
- [ ] All agent spawning uses direct async calls (no tmux)
- [ ] Tmux session creation code removed from codebase
- [ ] Pane management code removed
- [ ] Stability polling removed (instant spawn)

### AC2: Performance ✅
- [ ] Agent spawn time: <200ms (down from 6.5s, target 0.1s ±100ms variance)
- [ ] End-to-end quality gate: 20-30% faster (6.5s overhead eliminated)
- [ ] No performance regression on single-agent operations

### AC3: Filesystem Cleanup ✅
- [ ] Legacy filesystem collection removed (`fetch_agent_payloads_from_filesystem()`)
- [ ] `~/.code/agents/` directory deleted
- [ ] All quality gate tests pass without filesystem fallback

### AC4: Authentication ✅
- [ ] OAuth2 device code flow investigated (providers documented)
- [ ] Implementation for ≥1 provider (Google minimum)
- [ ] Fallback strategy documented (manual pre-auth)
- [ ] Clear error messages when authentication required

### AC5: Diagnostics ✅
- [ ] Structured logging with agent context (tracing::info!)
- [ ] TUI log viewer widget implemented
- [ ] Evidence files capture prompts + responses
- [ ] `--debug-agent` manual mode works

---

## 4. Technical Implementation

### Phase 1: Direct Async Spawning (Week 1 - 20-30h)

**Files to Modify**:
- `codex-tui/src/chatwidget/spec_kit/agent_orchestrator.rs` (spawn logic)
- `codex-tui/src/chatwidget/spec_kit/quality_gate_handler.rs` (remove tmux calls)
- `codex-core/src/orchestrator.rs` (direct process execution)

**New Code** (~1200-1500 LOC):
```rust
// orchestrator.rs - Direct async execution
pub async fn spawn_agent_direct(
    agent_name: &str,
    config: &AgentConfig,
    prompt: &str,
) -> Result<AgentHandle> {
    // Resolve agent CLI path
    let agent_cli = resolve_agent_cli(agent_name, config)?;

    // Spawn process directly
    let mut child = tokio::process::Command::new(&agent_cli.path)
        .args(&agent_cli.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Write prompt to stdin
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await?;
    }

    // Stream output to AGENT_MANAGER
    let agent_id = generate_agent_id();
    let (tx, rx) = mpsc::channel(1000);

    tokio::spawn(async move {
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut lines = stdout.lines();
        while let Some(line) = lines.next_line().await? {
            tx.send(AgentOutput::Stdout(line)).await?;
        }
    });

    // Update AGENT_MANAGER in real-time
    tokio::spawn(async move {
        while let Some(output) = rx.recv().await {
            AGENT_MANAGER.lock().unwrap().append_output(&agent_id, output);
        }
    });

    Ok(AgentHandle { agent_id, child })
}
```

**Testing**:
- Unit tests: Spawn single agent, verify output capture
- Integration tests: Multi-agent orchestration (3 agents parallel)
- Performance tests: Measure spawn time (target <200ms)

---

### Phase 2: Filesystem Cleanup (Week 2 - 5-7h)

**Deprecation**:
```rust
// Mark as deprecated first (warn users)
#[deprecated(since = "1.2.0", note = "Use AGENT_MANAGER direct reads instead")]
pub fn fetch_agent_payloads_from_filesystem() -> Result<Vec<AgentPayload>> {
    // ... existing code ...
}
```

**Removal** (after 1 release cycle):
```bash
# Remove legacy methods
git rm codex-core/src/orchestrator/filesystem_collection.rs

# Remove filesystem writes
grep -r "write.*result.txt" codex-core/ | xargs sed -i '/result.txt/d'

# Remove directory
rm -rf ~/.code/agents/
```

**Migration Guide** (`docs/migration/936-filesystem-removal.md`):
```markdown
# Filesystem Collection Removal

**Deprecated**: `fetch_agent_payloads_from_filesystem()`
**Replacement**: Direct AGENT_MANAGER reads

## Before (Legacy)
```rust
let payloads = fetch_agent_payloads_from_filesystem()?;
```

## After (Native)
```rust
let payloads = AGENT_MANAGER.lock().unwrap().get_all_agent_outputs()?;
```

## Migration
No action needed - native orchestrator already default. If you're still using filesystem collection, upgrade to v1.2.0+.
```

---

### Phase 3: OAuth2 Device Code (Week 2-3 - 8-12h)

**Investigation** (3-4h):
- Test Google device code flow with gcloud CLI
- Test Anthropic CLI (if device code supported)
- Test OpenAI CLI (if device code supported)
- Document findings in `docs/authentication/oauth2-device-code.md`

**Implementation** (5-8h):
```rust
// auth.rs - Device code flow
pub async fn authenticate_provider_device_flow(provider: &str) -> Result<AccessToken> {
    match provider {
        "google" => authenticate_google_device_flow().await,
        "anthropic" => {
            // If device code supported
            authenticate_anthropic_device_flow().await
        }
        _ => {
            // Fallback to manual pre-auth
            Err(format!("Provider '{}' does not support device code flow. Run: codex-tui --auth {}", provider, provider))
        }
    }
}
```

**CLI Integration**:
```bash
# Manual authentication command
codex-tui --auth google
# Output:
# 🔐 Google Authentication Required
#    1. Visit: https://google.com/device
#    2. Enter code: ABC-123
#    3. Waiting... ✅ Success!

# Verify authentication
codex-tui --auth-status
# Output:
# ✅ google: Authenticated (expires in 3600s)
# ⚠️  anthropic: Not authenticated (run: codex-tui --auth anthropic)
```

---

### Phase 4: Alternative Diagnostics (Week 3 - 7-10h)

**Structured Logging** (2-3h):
- Add `tracing::info!` calls throughout agent orchestration
- Include: agent_id, stage, elapsed_ms, exit_code
- Filter by agent_id in TUI log viewer

**TUI Log Viewer** (4-5h):
- New widget: `AgentLogViewer`
- Keybinding: `L` to open log viewer
- Features: Scroll, filter by level, search, export

**Evidence Files** (1-2h):
- Already implemented in SPEC-KIT-069 (evidence framework)
- Add agent execution logs to evidence directory
- Capture: prompt, response, stdout, stderr, exit_code

---

## 5. Success Metrics

### Performance Metrics
- **Agent Spawn Time**: 6.5s → <200ms (32× faster minimum, 65× target)
- **Quality Gate Time**: 10-15% faster end-to-end (tmux overhead eliminated)
- **Parallel Spawn**: 3 agents in <200ms (vs 19.5s sequential tmux)

### Architecture Metrics
- **Code Reduction**: ~500 LOC removed (tmux wrapper, filesystem collection)
- **Complexity Reduction**: Eliminate tmux dependency, simplify orchestration
- **Storage Systems**: 4 → 2 (after SPEC-934, this enables final filesystem removal)

### Correctness Metrics
- **Authentication Success Rate**: 100% (device code flow works for ≥1 provider)
- **Quality Gate Pass Rate**: 100% (all tests pass without tmux)

---

## 6. Risk Analysis

### Risk 1: Performance Claims Unvalidated (HIGH)

**Scenario**: 65× speedup is ESTIMATED, actual speedup may be lower (10×, 5×, or even 2×).

**Mitigation**:
- Acknowledge measurement gap upfront (documented in this PRD)
- SPEC-940 (performance instrumentation) validates claims post-implementation
- Even if estimate is 50% wrong, still significant benefit (3-4s savings)

**Likelihood**: High (measurement gap is known), but acceptable risk.

---

### Risk 2: OAuth2 Provider Support Gaps (MEDIUM)

**Scenario**: Key providers (Anthropic, OpenAI) don't support device code flow.

**Mitigation**:
- Fallback to manual pre-authentication (`codex-tui --auth <provider>`)
- Refresh token strategy (long-lived credentials)
- Clear documentation on authentication setup
- Google support minimum (covers Gemini, most-used provider)

**Likelihood**: Medium (unknown until investigation)

---

### Risk 3: Lost Observable Execution (MEDIUM)

**Scenario**: Developers miss `tmux attach` for debugging, structured logs insufficient.

**Mitigation**:
- `--debug-agent` mode provides live log stream (similar to tmux attach)
- Evidence files capture full execution context
- TUI log viewer provides post-execution debugging
- Documentation: "How to debug agents without tmux"

**Likelihood**: Low (alternatives are comprehensive)

---

## 7. Open Questions

### Q1: Should we support tmux mode as optional fallback?

**Context**: Some developers may prefer tmux observability despite performance cost.

**Decision**: NO - Maintaining two code paths doubles complexity. Provide equivalent diagnostics via logs/evidence instead.

---

### Q2: What's the migration path for existing tmux sessions?

**Context**: If user has active tmux sessions when upgrading, what happens?

**Decision**: Clean shutdown of old sessions on first launch. Display migration notice: "Tmux orchestration deprecated. Using direct async execution."

---

### Q3: Should device code flow be mandatory or optional?

**Context**: Some users may not want to authenticate via device code (security policy, air-gapped environments).

**Decision**: OPTIONAL - Manual pre-auth remains supported. Device code is convenience feature.

---

## 8. Implementation Strategy

### Week 1: Direct Async Spawning (30h)
- **Mon-Tue**: Implement `spawn_agent_direct()`, remove tmux wrapper
- **Wed**: Integrate into quality_gate_handler.rs
- **Thu**: Integration tests (multi-agent orchestration)
- **Fri**: Performance benchmarks, optimization

### Week 2: Cleanup + OAuth2 Investigation (15h)
- **Mon**: Remove filesystem collection (deprecate, then delete)
- **Tue**: Migration tests (verify no filesystem fallback needed)
- **Wed-Thu**: OAuth2 device code investigation (all providers)
- **Fri**: Implement device code for ≥1 provider (Google minimum)

### Week 3: Diagnostics + Finalization (20h)
- **Mon**: Structured logging throughout orchestration
- **Tue**: TUI log viewer widget
- **Wed**: Evidence file integration, `--debug-agent` mode
- **Thu**: Documentation (authentication guide, migration guide, debugging guide)
- **Fri**: Final testing, PR preparation

**Total**: 65h (within 45-65h estimate, upper bound)

---

## 9. Deliverables

1. **Code Changes**:
   - `codex-core/src/orchestrator.rs` - Direct async execution
   - `codex-tui/src/chatwidget/spec_kit/*.rs` - Remove tmux calls
   - `codex-core/src/auth.rs` - OAuth2 device code flow
   - `codex-tui/src/widgets/agent_log_viewer.rs` - TUI log viewer

2. **Documentation**:
   - `docs/authentication/oauth2-device-code.md` - Provider support matrix
   - `docs/migration/936-filesystem-removal.md` - Legacy cleanup guide
   - `docs/debugging/agents-without-tmux.md` - Alternative diagnostics guide

3. **Scripts**:
   - `scripts/cleanup_tmux_sessions.sh` - One-time migration cleanup

4. **Tests**:
   - Integration tests (direct async spawning, multi-agent)
   - Performance tests (spawn time benchmarks)
   - Authentication tests (device code flow, manual pre-auth)

---

## 10. Validation Plan

### Performance Tests (5 benchmarks)
- Single agent spawn time (<200ms)
- Multi-agent spawn time (3 agents <200ms)
- End-to-end quality gate (10-15% faster)
- Memory usage (no regression)
- CPU usage (no regression)

### Integration Tests (10 tests)
- Direct async spawning (single agent)
- Multi-agent orchestration (3 agents parallel)
- Output streaming to AGENT_MANAGER
- Error handling (CLI not found, OAuth2 required)
- Authentication flows (device code, manual pre-auth)
- Filesystem collection removed (no legacy fallback)

### Authentication Tests (3 tests)
- Google device code flow (success, timeout, error)
- Manual pre-auth (store token, verify refresh)
- Token expiry handling (detect 401, prompt re-auth)

**Total**: 18 tests

---

## 11. Conclusion

SPEC-936 eliminates tmux orchestration overhead, removes filesystem collection duplication, and investigates OAuth2 device code flows for non-interactive execution. **Estimated effort: 45-65 hours over 3 weeks.**

**Key Benefits**:
- ✅ 65× faster agent spawning (6.5s → 0.1s target, ESTIMATED)
- ✅ Eliminates filesystem collection (simplifies architecture)
- ✅ Alternative diagnostics (logs, TUI viewer, evidence files)
- ⚠️ Measurement gap acknowledged (SPEC-940 validates claims)

**Next Steps**:
1. Review and approve SPEC-936 (acknowledge measurement gap risk)
2. Schedule Week 1 kickoff (direct async spawning)
3. Coordinate OAuth2 investigation (Week 2)
4. Plan SPEC-940 (performance instrumentation) for validation

**Risk Acceptance**: Proceed despite measurement gap (even 50% error yields significant speedup).
