# Migration Analysis: Source A (~/code) vs Source B (~/old/code)

_Generated: 2025-11-27_

## 1. File Structure Comparison

**Source B has a dual-crate architecture** documented in `prompts/MERGE.md`:
- `codex-rs/` = Pure upstream mirror (OpenAI/codex)
- `code-rs/` = Fork-specific code (renamed crates like `code-auto-drive-core`)

**Source A diverged** from this model - it has only `codex-rs/` with fork changes mixed in.

### New Top-Level Directories in Source B

| Directory | Purpose |
|-----------|---------|
| `sdk/typescript/` | TypeScript SDK package (`@openai/codex-sdk`) |
| `shell-tool-mcp/` | MCP shell tool server with patched Bash wrappers |
| `prompts/` | Workflow docs: `MERGE.md`, `TRIAGE.md` |
| `code-rs/` | Fork-specific crates (parallel to codex-rs) |

### New Rust Crates in Source B's `codex-rs/`

| Crate | Purpose | Priority |
|-------|---------|----------|
| `process-hardening` | Security: disable core dumps, ptrace, env sanitization | **CRITICAL** |
| `codex-api` | API abstraction with rate limiting, SSE, telemetry | **HIGH** |
| `feedback` | Ring buffer logging + Sentry integration | **HIGH** |
| `keyring-store` | System keyring credential storage | **HIGH** |
| `async-utils` | Cancellation token extensions (`.or_cancel()`) | MEDIUM |
| `app-server` | Application server infrastructure | MEDIUM |
| `backend-client` | Backend client abstraction | MEDIUM |
| `exec-server` | Execution server | MEDIUM |
| `stdio-to-uds` | Stdio to Unix domain socket | LOW |
| `responses-api-proxy` | API proxy | LOW |
| `rmcp-client` | RMCP client | LOW |
| `lmstudio` | LM Studio integration | LOW |
| `windows-sandbox-rs` | Windows sandboxing | LOW |

---

## 2. Key Feature Differences (Logic, Not Formatting)

### **CRITICAL: Security Enhancements**

**1. Dangerous Command Detection** (`core/src/command_safety/is_dangerous_command.rs`)
```
Source B has:
- Detects `git reset`, `git rm`, `rm -rf`, `rm -f`
- Handles nested shell commands (`bash -lc "git reset"`)
- Recursive `sudo` detection
- Windows-specific dangerous commands
```
**Files to patch**: `codex-rs/core/src/command_safety/`

**2. Process Hardening** (`process-hardening/src/lib.rs`)
```
- Disables core dumps (RLIMIT_CORE=0)
- Prevents ptrace attach (Linux PR_SET_DUMPABLE, macOS PT_DENY_ATTACH)
- Removes LD_PRELOAD, DYLD_* environment variables
- Cross-platform: Linux, macOS, BSD, Windows stubs
```
**Files to patch**: New crate, integrate into startup

**3. Cargo Deny Configuration** (`codex-rs/deny.toml`)
```
- License auditing
- Vulnerability scanning (RustSec)
- Banned crate detection
```
**Files to patch**: Add `deny.toml` to your `codex-rs/`

### **HIGH: Core Functionality**

**4. Context Compaction** (`core/src/compact.rs`, `compact_remote.rs`)
```
- Summarizes conversation history when approaching context limits
- Remote compaction via ChatGPT auth
- Uses templates from `templates/compact/`
```
**Benefit**: Handles long sessions without context window overflow

**5. API Bridge** (`core/src/api_bridge.rs`)
```
- Unified error mapping from codex-api to core errors
- Rate limit parsing with retry hints
- Usage limit detection with plan type info
```

**6. Feedback System** (`feedback/`)
```
- Ring buffer logging (4MB cap)
- Sentry upload with attachments
- Session classification (bug/bad_result/good_result)
```

**7. Keyring Store** (`keyring-store/`)
```
- System keyring abstraction
- Mock implementation for testing
- Cross-platform credential storage
```

### **MEDIUM: TUI Improvements**

**8. Footer Rework** (`tui/src/bottom_pane/footer.rs`)
```
- FooterMode enum: CtrlCReminder, ShortcutSummary, EscHint, ContextOnly
- Context window percentage display
- Improved keyboard hint system
```

**9. ASCII Animation** (`tui/src/ascii_animation.rs`)
```
- Frame-based animation driver
- Multiple variant support
- Proper timing with frame scheduling
```

**10. Approval Overlay** (`tui/src/bottom_pane/approval_overlay.rs`)
```
- Improved approval UI (vs your approval_modal_view.rs)
- May have upstream bug fixes
```

---

## 3. Prioritized Recommendations

### **P0 - CRITICAL (Security)**

| # | Update | Files | Benefit |
|---|--------|-------|---------|
| 1 | Add `is_dangerous_command.rs` | `core/src/command_safety/` | Prevents destructive git/rm commands |
| 2 | Add `windows_dangerous_commands.rs` | `core/src/command_safety/` | Windows safety parity |
| 3 | Add `process-hardening` crate | New crate + startup integration | Core dumps, ptrace, env hardening |
| 4 | Add `deny.toml` | `codex-rs/deny.toml` | Dependency vulnerability scanning |

### **P1 - HIGH (Core Features)**

| # | Update | Files | Benefit |
|---|--------|-------|---------|
| 5 | Add `codex-api` crate | New crate | Clean API abstraction, rate limiting |
| 6 | Add `compact.rs` + templates | `core/src/compact*.rs`, `templates/compact/` | Long session support |
| 7 | Add `api_bridge.rs` | `core/src/api_bridge.rs` | Better error handling |
| 8 | Add `feedback` crate | New crate | Bug reporting with Sentry |
| 9 | Add `keyring-store` crate | New crate | Secure credential storage |
| 10 | Add `async-utils` crate | New crate | Clean cancellation patterns |

### **P2 - MEDIUM (UX/Tooling)**

| # | Update | Files | Benefit |
|---|--------|-------|---------|
| 11 | Port `footer.rs` improvements | `tui/src/bottom_pane/` | Context indicator, better hints |
| 12 | Add `ascii_animation.rs` | `tui/src/ascii_animation.rs` | Polished loading animations |
| 13 | Add TypeScript SDK | `sdk/typescript/` | External integration support |
| 14 | Add `shell-tool-mcp` | `shell-tool-mcp/` | MCP shell tool server |
| 15 | Adopt merge playbook | `prompts/MERGE.md` | Upstream sync process |

### **P3 - LOW (Nice to Have)**

| # | Update | Files | Benefit |
|---|--------|-------|---------|
| 16 | Add `app-server` crates | 3 new crates | Server infrastructure |
| 17 | Add `backend-client` | New crate | Backend abstraction |
| 18 | Add `lmstudio` integration | New crate | LM Studio provider |
| 19 | Add Windows sandbox | `windows-sandbox-rs/` | Windows sandboxing |

---

## 4. Recommended Merge Strategy

Based on Source B's `prompts/MERGE.md`, consider restructuring to:

1. **Keep `codex-rs/` as upstream mirror** (rsync from upstream)
2. **Create `code-rs/` for fork-specific code** (your spec-kit, browser, multi-agent)
3. **Use `scripts/check-codex-path-deps.sh`** to validate isolation

This prevents merge conflicts when syncing upstream changes.

---

## 5. Source Locations Reference

### Source A (Your Fork)
- Path: `~/code`
- Codex-rs: `~/code/codex-rs/`

### Source B (Upstream Reference)
- Path: `~/old/code`
- Codex-rs: `~/old/code/codex-rs/`
- Code-rs: `~/old/code/code-rs/`

### Key Files for P0 Items
```
# Dangerous command detection
~/old/code/codex-rs/core/src/command_safety/is_dangerous_command.rs
~/old/code/codex-rs/core/src/command_safety/windows_dangerous_commands.rs

# Process hardening
~/old/code/codex-rs/process-hardening/src/lib.rs
~/old/code/codex-rs/process-hardening/Cargo.toml

# Cargo deny
~/old/code/codex-rs/deny.toml
```
