# SPEC-936 Tmux Usage Inventory

**Created**: 2025-11-15
**Session**: 1 (Component 1.2)
**Purpose**: Complete inventory of all tmux dependencies for elimination

---

## Executive Summary

**Total Files with Tmux Dependencies**: 6 (5 code + 1 comments-only)
**Total Tmux Function Calls**: 7 distinct call sites
**Primary Integration Point**: `core/src/agent_tool.rs::execute_model_with_permissions()`
**Complexity Assessment**: **MEDIUM** - Well-isolated with clear boundaries

---

## 1. Core Module: `core/src/tmux.rs`

**Lines of Code**: 851 (including tests)
**Status**: Primary implementation, entire file to be deprecated/removed
**Exposed**: Yes, via `pub mod tmux;` in `core/src/lib.rs:91`

### Public API (11 Functions)

| Function | Line | Async | Purpose | Current Usage |
|----------|------|-------|---------|---------------|
| `is_tmux_available()` | 12 | ✅ | Check if tmux is installed | agent_tool.rs:1230 |
| `ensure_session()` | 27 | ✅ | Create/reuse tmux session (with stale killing) | agent_tool.rs:1237 |
| `create_pane()` | 108 | ✅ | Create pane in session with title | agent_tool.rs:1248 |
| `execute_in_pane()` | 164 | ✅ | **MAIN**: Execute command, capture output (452 LOC!) | agent_tool.rs:1328 |
| `capture_pane_output()` | 619 | ✅ | Capture final pane output | Unused (dead code?) |
| `kill_pane_process()` | 641 | ✅ | Kill zombie agent process (Ctrl+C + force) | Unused (dead code?) |
| `check_zombie_panes()` | 694 | ✅ | Count zombie panes in session | agent_orchestrator.rs:296 |
| `cleanup_zombie_panes()` | 723 | ✅ | Clean up all zombies (kills session) | agent_orchestrator.rs:304 |
| `kill_session()` | 743 | ✅ | Kill tmux session | Called by cleanup_zombie_panes() |
| `save_pane_evidence()` | 759 | ✅ | Save output to evidence file | Unused (dead code?) |
| `get_attach_instructions()` | 779 | ❌ | Generate tmux attach instructions | agent_tool.rs:1346 |

**Key Insights**:
- **execute_in_pane()** is the monster function (452 lines, 52% of file)
- Handles: Large argument heredoc wrapper scripts, output file redirection, completion marker polling, file size stability detection
- **3 functions appear unused**: capture_pane_output, kill_pane_process, save_pane_evidence (verify before removal)
- **Stale session killing**: ensure_session() kills sessions >5 minutes old (SPEC-KIT-925)

---

## 2. Primary Integration: `core/src/agent_tool.rs`

### Agent Struct Definition

**Location**: Line 56-59
**Field**: `pub tmux_enabled: bool`
**Default**: `false` (line 242)
**Usage**: Stored in Agent struct, passed through execution chain

### Main Execution Flow

```
Agent Creation:
  create_agent_from_config_name() [line 182]
    ↓ (receives tmux_enabled param)
  create_agent_internal() [line 247]
    ↓ (stores in Agent struct, line 280)

Agent Execution:
  execute_agent() [line 669]
    ↓ (extracts tmux_enabled, line 695)
  execute_model_with_permissions() [line 1061]
    ↓ (branches on use_tmux flag)

  if use_tmux && tmux_available:
    tmux::is_tmux_available() [line 1230]
    tmux::ensure_session() [line 1237]
    tmux::create_pane() [line 1248]
    tmux::execute_in_pane() [line 1328] ← MAIN WORK
    tmux::get_attach_instructions() [line 1346]
  else:
    Direct Command::spawn() execution (normal path)
```

### Tmux Function Calls

| Call Site | Line | Function | Purpose | Migration Strategy |
|-----------|------|----------|---------|-------------------|
| agent_tool.rs | 1230 | `is_tmux_available()` | Capability check | Remove branch entirely |
| agent_tool.rs | 1237 | `ensure_session()` | Session setup | Not needed for direct spawn |
| agent_tool.rs | 1248 | `create_pane()` | Pane creation | Not needed for direct spawn |
| agent_tool.rs | 1328 | `execute_in_pane()` | **Execute + capture** | **Replace with tokio::process::Command** |
| agent_tool.rs | 1346 | `get_attach_instructions()` | Debug instructions | Remove (use --debug-agent) |

**Migration Complexity**: **MEDIUM**
- Tmux path is well-isolated in single if-block (lines 1228-1359)
- Fallback to direct execution already exists (lines 1360+)
- **Strategy**: Remove if-block, keep direct execution path, enhance with async I/O streaming

---

## 3. Orchestration Layer: `tui/src/chatwidget/spec_kit/agent_orchestrator.rs`

### Environment Variable Control

**Location**: Lines 286-288, 619-623
**Env Var**: `SPEC_KIT_OBSERVABLE_AGENTS`
**Default**: `true` (enabled by default since SPEC-KIT-927)
**Disable**: Set to `"0"` or `"false"`

### Zombie Cleanup Integration

| Call Site | Line | Function | Purpose | Migration Strategy |
|-----------|------|----------|---------|-------------------|
| agent_orchestrator.rs | 296 | `check_zombie_panes()` | Pre-spawn zombie detection | **Remove** (no tmux = no zombies) |
| agent_orchestrator.rs | 304 | `cleanup_zombie_panes()` | Zombie cleanup before spawn | **Remove** (no tmux = no zombies) |

**Context**: Lines 290-310
**Trigger**: Only runs if `tmux_enabled == true`
**Function**: `spawn_and_wait_for_agent()` (lines 254-440)
**Migration**: Remove zombie cleanup logic entirely (not needed for direct process spawning)

---

## 4. Quality Gate Layers

### Native Quality Gate Orchestrator

**File**: `tui/src/chatwidget/spec_kit/native_quality_gate_orchestrator.rs`
**Line**: 128
**Setting**: `tmux_enabled: true` (hardcoded)
**Comment**: "needed for output capture and debugging (SPEC-KIT-928)"
**Migration**: Change to `false` after implementing async I/O capture

### Quality Gate Handler

**File**: `tui/src/chatwidget/spec_kit/quality_gate_handler.rs`
**Line**: 956
**Setting**: `false` for single validation agent
**Comment**: "No tmux for single validation agent"
**Migration**: No change needed (already false)

---

## 5. Environment Detection: `tui/src/terminal_info.rs`

**Purpose**: Detect if running inside tmux/screen for terminal capability detection
**Lines**: 244, 254
**Type**: Environmental check, NOT agent execution

```rust
// Line 244: Check if running inside tmux/screen
if env::var("TMUX").is_ok() || env::var("STY").is_ok() {
    // Handle terminal multiplexer environment
}

// Line 254: Terminal type filtering
const UNSUPPORTED_PREFIXES: [&str; 2] = ["screen", "tmux"];
```

**Migration**: **KEEP** - This is about terminal detection, not agent execution
**No Action Required**: Unrelated to SPEC-936 scope

---

## 6. Documentation/Comments Only

### `codex-rs/spec-kit/src/timing.rs`

**Lines**: 38, 43 (documentation examples only)
**Type**: Example code in doc comments showing how to use timing macros
**Migration**: Update examples to use async process spawning instead of tmux

### `codex-rs/tui/src/tui.rs`

**Line**: 421 (comment only)
**Content**: "// Default: enabled for modern terminals (xterm-256color, iTerm2, Alacritty, kitty, tmux, etc.)"
**Migration**: No action needed (just listing terminal types)

---

## 7. Dependency Map

```
┌─────────────────────────────────────────────────┐
│ HIGH-LEVEL FLOW                                 │
│                                                 │
│ User triggers /speckit.auto                     │
│   ↓                                             │
│ agent_orchestrator::spawn_and_wait_for_agent()  │
│   ├─> Check SPEC_KIT_OBSERVABLE_AGENTS env     │
│   ├─> IF tmux_enabled:                          │
│   │     └─> check_zombie_panes()    ──────┐    │
│   │     └─> cleanup_zombie_panes()  ──────┤    │
│   ↓                                        ↓    │
│ AGENT_MANAGER.create_agent_from_config_name()   │
│   └─> Store tmux_enabled in Agent struct        │
│   ↓                                             │
│ execute_agent()                                 │
│   └─> Extract tmux_enabled flag                 │
│   ↓                                             │
│ execute_model_with_permissions()                │
│   ├─> IF use_tmux && tmux_available:           │
│   │     ├─> is_tmux_available()     ──────┐    │
│   │     ├─> ensure_session()        ──────┤    │
│   │     ├─> create_pane()           ──────┤    │
│   │     ├─> execute_in_pane()       ──────┤    │
│   │     └─> get_attach_instructions() ────┤    │
│   └─> ELSE: Direct Command::spawn()        ↓    │
│                                                 │
└────────────────────────────────────┬────────────┘
                                     │
                    ┌────────────────┴─────────────┐
                    │ core/src/tmux.rs             │
                    │ (851 LOC - entire file)      │
                    │                              │
                    │ 11 public functions:         │
                    │ - Session management         │
                    │ - Pane creation/control      │
                    │ - Output capture/polling     │
                    │ - Zombie cleanup             │
                    │ - Evidence storage           │
                    └──────────────────────────────┘
```

---

## 8. Call Site Summary

### Direct Tmux Function Calls

| File | Function Called | Line | Removable? |
|------|----------------|------|------------|
| agent_tool.rs | is_tmux_available() | 1230 | ✅ YES (remove if-block) |
| agent_tool.rs | ensure_session() | 1237 | ✅ YES (remove if-block) |
| agent_tool.rs | create_pane() | 1248 | ✅ YES (remove if-block) |
| agent_tool.rs | execute_in_pane() | 1328 | ✅ YES (replace with tokio spawn) |
| agent_tool.rs | get_attach_instructions() | 1346 | ✅ YES (remove if-block) |
| agent_orchestrator.rs | check_zombie_panes() | 296 | ✅ YES (no tmux = no zombies) |
| agent_orchestrator.rs | cleanup_zombie_panes() | 304 | ✅ YES (no tmux = no zombies) |

**Total Removal**: 7 call sites across 2 files
**Plus**: Entire `core/src/tmux.rs` module (851 LOC)

### Indirect Usage (Agent struct field)

| File | Location | Usage | Migration |
|------|----------|-------|-----------|
| agent_tool.rs | Line 58 | `tmux_enabled: bool` field definition | **REMOVE** field entirely |
| agent_tool.rs | Line 242 | Default `false` in create_agent_with_config() | Remove parameter |
| agent_orchestrator.rs | Line 286-288 | Read `SPEC_KIT_OBSERVABLE_AGENTS` env | **REMOVE** env var check |
| agent_orchestrator.rs | Line 347, 674 | Pass tmux_enabled to create_agent | Remove parameter |
| native_quality_gate_orchestrator.rs | Line 128 | Hardcode `true` | Remove parameter |
| quality_gate_handler.rs | Line 956 | Set `false` | Remove parameter |

---

## 9. Migration Complexity Assessment

### Complexity Ratings

| Component | Complexity | Reason | Estimated Hours |
|-----------|-----------|--------|----------------|
| **core/src/tmux.rs** | LOW | Delete entire file | 0.5h (verification) |
| **agent_tool.rs** | MEDIUM | Remove if-block (130 lines), enhance else-block with async I/O | 4-6h |
| **agent_orchestrator.rs** | LOW | Remove zombie cleanup (15 lines) | 1h |
| **Quality gate files** | LOW | Remove/update tmux_enabled parameters (4 locations) | 1h |
| **Agent struct** | LOW | Remove tmux_enabled field + update all constructors | 1-2h |
| **Testing** | MEDIUM | Verify no regressions, test async I/O capture | 3-4h |
| **Documentation** | LOW | Update timing.rs examples | 0.5h |

**Total Estimated**: **11-15 hours** for complete tmux removal

### Risk Factors

**LOW RISK**:
- ✅ Well-isolated: Primary logic in single if-block (agent_tool.rs:1228-1359)
- ✅ Fallback exists: Direct execution path already implemented and working
- ✅ Clear boundaries: Only 2 files have actual function calls
- ✅ No database dependencies: Tmux state not persisted

**MEDIUM RISK**:
- ⚠️ Output capture: Need to replicate file size stability logic (SPEC-KIT-927)
- ⚠️ Large argument handling: execute_in_pane() has complex heredoc wrapper (lines 194-283)
- ⚠️ Completion detection: Polling for `___AGENT_COMPLETE___` marker (lines 383-615)

**MITIGATION**:
- Implement async stdout/stderr streaming with tokio::process::Command
- Use process exit codes + timeout for completion detection (simpler than tmux polling)
- Test large prompt handling (>1000 chars) to ensure no command-line length issues

---

## 10. Dead Code Analysis

**Potentially Unused Functions** (verify with grep before removal):

1. **capture_pane_output()** (line 619)
   - No direct calls found
   - May be legacy from pre-execute_in_pane() era

2. **kill_pane_process()** (line 641)
   - No direct calls found
   - Zombie cleanup uses cleanup_zombie_panes() which calls kill_session() instead

3. **save_pane_evidence()** (line 759)
   - No direct calls found
   - Evidence saving may have moved to different mechanism

**Recommendation**: Verify with comprehensive grep, then remove if truly unused.

---

## 11. Environment Variables

| Variable | Purpose | Default | Set By | Impact |
|----------|---------|---------|--------|--------|
| `SPEC_KIT_OBSERVABLE_AGENTS` | Enable tmux mode | `true` (if unset) | User/scripts | Controls zombie cleanup + tmux execution |
| `TMUX` | Detect if running inside tmux | N/A (set by tmux) | Tmux itself | Terminal detection only (KEEP) |

**Migration**:
- **REMOVE**: `SPEC_KIT_OBSERVABLE_AGENTS` (no longer needed)
- **KEEP**: `TMUX` (terminal detection unrelated to agent execution)

---

## 12. Testing Requirements

### Before Removal (Baseline)

1. ✅ Run full test suite with `SPEC_KIT_OBSERVABLE_AGENTS=1` (tmux mode)
2. ✅ Run `/speckit.auto SPEC-KIT-900` with tmux enabled
3. ✅ Capture baseline metrics:
   - Agent spawn time
   - Quality gate execution time
   - Output file size and content
4. ✅ Verify zombie cleanup logic works (create orphaned panes, verify cleanup)

### After Removal (Validation)

1. ✅ Run full test suite (604 tests) - must maintain 100% pass rate
2. ✅ Run `/speckit.auto SPEC-KIT-900` with direct execution
3. ✅ Measure performance improvement (expect 6.5s → <200ms per agent)
4. ✅ Test large prompts (>50KB) - ensure no command-line length issues
5. ✅ Test concurrent multi-agent execution (3 agents parallel)
6. ✅ Verify output capture completeness (no truncation, no missing data)
7. ✅ Test timeout handling (agents that exceed timeout_secs)
8. ✅ Test error handling (CLI not found, OAuth2 required, process crash)

---

## 13. Replacement Strategy

### Current (Tmux-Based)

```rust
// agent_tool.rs:1228-1359 (130 lines)
if use_tmux && crate::tmux::is_tmux_available().await {
    let session_name = format!("agents-{}", model);
    crate::tmux::ensure_session(&session_name).await?;
    let pane_id = crate::tmux::create_pane(&session_name, &pane_title, false).await?;

    // Build command + args + env (45 lines)
    let output = crate::tmux::execute_in_pane(
        &session_name,
        &pane_id,
        &program,
        &args,
        &env,
        working_dir.as_deref(),
        timeout_secs,
    ).await?;

    return Ok(output);
}
```

### Proposed (Direct Async)

```rust
// New implementation (estimated 80-100 lines)
// Spawn process directly with tokio
let mut child = tokio::process::Command::new(&command)
    .args(&args)
    .envs(env)
    .current_dir(working_dir.unwrap_or_else(|| PathBuf::from(".")))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

// Stream stdout/stderr asynchronously
let stdout_handle = tokio::spawn(async move {
    let mut reader = BufReader::new(child.stdout.take().unwrap());
    let mut output = String::new();
    reader.read_to_string(&mut output).await?;
    Ok(output)
});

let stderr_handle = tokio::spawn(async move {
    let mut reader = BufReader::new(child.stderr.take().unwrap());
    let mut errors = String::new();
    reader.read_to_string(&mut errors).await?;
    Ok(errors)
});

// Wait for completion with timeout
let timeout_duration = Duration::from_secs(timeout_secs);
let result = tokio::time::timeout(timeout_duration, child.wait_with_output()).await??;

// Collect output
let stdout = stdout_handle.await??;
let stderr = stderr_handle.await??;

if !result.status.success() {
    return Err(format!("Agent failed: {}", stderr));
}

Ok(stdout)
```

**Benefits**:
- ✅ No tmux dependency
- ✅ Instant spawn (<10ms vs ~6.5s)
- ✅ Simpler code (80 lines vs 452 in execute_in_pane)
- ✅ Native async I/O (no polling required)
- ✅ Process exit codes (reliable completion detection)

---

## 14. Next Steps (Component 1.3-1.5)

### Session 2 Tasks

**Component 1.3**: Design async orchestration architecture (3-4h)
- [ ] Create AsyncAgentExecutor trait design
- [ ] Design stdout/stderr streaming approach
- [ ] Plan completion detection mechanism (exit codes vs markers)
- [ ] Error handling strategy (CLI missing, timeout, crash)

**Component 1.4**: OAuth2 device code flow research (2-3h)
- [ ] Test Google OAuth2 device code flow with gcloud CLI
- [ ] Investigate Anthropic CLI authentication
- [ ] Investigate OpenAI CLI authentication
- [ ] Document provider support matrix

**Component 1.5**: Create implementation task list (1h)
- [ ] Write detailed tasks.md with phase breakdown
- [ ] Update SPEC.md task tracker
- [ ] Link to this inventory document

---

## Conclusion

Tmux usage is well-contained and straightforward to remove:

1. **7 function call sites** across 2 primary files
2. **1 module to delete** (core/src/tmux.rs, 851 LOC)
3. **Medium complexity** due to output capture requirements
4. **Clear replacement path** via tokio::process::Command
5. **Estimated 11-15 hours** for complete elimination

**Key Success Factors**:
- Tmux logic is well-isolated (single if-block in agent_tool.rs)
- Fallback direct execution already exists and works
- No database/state dependencies on tmux
- Clear performance target (6.5s → <200ms)

**Primary Challenges**:
- Replicating file size stability detection (SPEC-KIT-927)
- Handling large prompts (>50KB heredoc wrapper scripts)
- Ensuring complete output capture (no truncation)

**Recommendation**: **PROCEED** with Phase 2 (Core Async Infrastructure) implementation.
