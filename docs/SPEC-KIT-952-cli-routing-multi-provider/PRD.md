# PRD: CLI Routing for Multi-Provider Model Support

**SPEC-ID**: SPEC-KIT-952
**Status**: Backlog
**Created**: 2025-11-19
**Author**: Code
**Priority**: P1 - HIGH
**Type**: Implementation
**Based On**: SPEC-KIT-951 (Multi-Provider OAuth Research)

---

## Executive Summary

Implement CLI routing to enable multi-provider model support (ChatGPT, Claude, Gemini) by routing commands through native CLIs instead of implementing OAuth for all providers. This approach bypasses the Claude OAuth blocker discovered in SPEC-KIT-951 research.

**Success Criteria**: Users can select any model via `/model` command and execute spec-kit commands successfully, with authentication handled transparently by each provider's native CLI.

**Downstream Impact**: Enables SPEC-KIT-946 (/model command expansion) and unblocks SPEC-KIT-947 (master validation)

---

## Problem Statement

### Current State

**Working**:
- ✅ ChatGPT models work via native OAuth implementation
- ✅ `/model` command can display all 13 models (SPEC-KIT-946)

**Broken**:
- ❌ Selecting Claude models fails (no OAuth support from Anthropic)
- ❌ Selecting Gemini models fails (no OAuth integration)
- ❌ Users limited to ChatGPT models only

### Research Findings (SPEC-KIT-951)

**Critical Discovery**: Anthropic does NOT provide OAuth for third-party applications. Only API keys supported.

**Solution Identified**: Route commands through native CLIs:
- `claude` CLI (user already authenticated)
- `gemini` CLI (user already authenticated)
- Keep ChatGPT OAuth as-is (working)

---

## Solution: CLI Routing Architecture

### High-Level Design

```
User selects model (/model command)
         ↓
┌────────────────────┐
│   Model Router     │ ← New component
└────────┬───────────┘
         ↓
    Provider Detection
         ↓
   ┌─────┴─────┬──────────┬─────────┐
   │           │          │         │
ChatGPT     Claude     Gemini   (Future)
   │           │          │
Native      CLI        CLI
OAuth     Routing    Routing
   │           │          │
Existing   `claude`  `gemini`
  Code      -p flag   -p flag
```

### Component Breakdown

**1. ModelProvider Enum** (extend existing)
```rust
pub enum ModelProvider {
    ChatGPT,  // Existing: uses native OAuth
    Claude,   // New: routes through `claude` CLI
    Gemini,   // New: routes through `gemini` CLI
}
```

**2. CLI Executor** (new module)
- Detect CLI availability (`which claude`, `which gemini`)
- Execute non-interactive commands
- Parse CLI output
- Handle errors gracefully

**3. Model Router** (new component)
- Route based on selected model provider
- Delegate to appropriate execution path
- Normalize responses

---

## Requirements

### Functional Requirements

**FR1: CLI Detection**
- System must detect if `claude` and `gemini` CLIs are installed
- Provide clear error messages if CLI not found
- Include installation instructions in error messages

**FR2: CLI Execution**
- Execute CLI commands in non-interactive mode
- Capture stdout/stderr
- Apply timeouts (5 minutes default)
- Handle process errors

**FR3: Response Parsing**
- Parse Claude JSON output (`--output-format json`)
- Parse Gemini text output
- Normalize to common Response format
- Preserve model metadata

**FR4: Authentication Verification**
- Detect if CLI is authenticated
- Provide clear error if authentication required
- Include authentication instructions

**FR5: Model Router Integration**
- Integrate with `/model` command selection
- Route spec-kit commands through appropriate provider
- Maintain backward compatibility with ChatGPT

**FR6: Error Handling**
- Handle CLI not installed
- Handle CLI not authenticated
- Handle command execution failures
- Handle timeout scenarios

### Non-Functional Requirements

**NFR1: Performance**
- CLI execution overhead <2 seconds
- Acceptable for batch spec-kit operations
- No blocking on UI thread

**NFR2: Reliability**
- Graceful degradation if CLI fails
- Clear error messages
- Retry logic for transient failures

**NFR3: Maintainability**
- Abstract CLI execution (reusable for future providers)
- Well-documented CLI interface contracts
- Easy to add new CLI providers

**NFR4: User Experience**
- Transparent authentication (users don't re-auth)
- Clear setup instructions
- Helpful error messages

---

## Technical Design

### CLI Interface Specifications

#### Claude CLI

**Command**: `claude -p "{prompt}" --output-format json`

**Key Flags**:
- `-p, --print`: Non-interactive output
- `--output-format json`: Structured response
- `--tools <tools...>`: Control tool availability
- `--system-prompt <prompt>`: Custom system prompt

**Example**:
```bash
claude -p "Write a hello world function" --output-format json
```

**Output Format**:
```json
{
  "response": "Here's a hello world function...",
  "model": "claude-sonnet-4-5",
  "usage": {
    "input_tokens": 15,
    "output_tokens": 42
  }
}
```

#### Gemini CLI

**Command**: `gemini -p "{prompt}" -m "{model}"`

**Key Flags**:
- `-p, --prompt`: Prompt text
- `-m, --model`: Model selection
- `-y, --yolo`: Auto-approve actions
- `--approval-mode auto_edit`: Auto-approve edits

**Example**:
```bash
gemini -p "Write a hello world function" -m "gemini-2.0-flash"
```

**Output Format**: Plain text (parse as-is)

---

### Implementation Plan

#### Phase 1: CLI Infrastructure (3-4 hours)

**Files to Create**:
- `codex-rs/tui/src/cli_executor.rs` (new module)
- `codex-rs/tui/src/model_router.rs` (new module)

**Tasks**:
1. Create `CliExecutor` trait
2. Implement `execute_command` with timeout
3. Implement CLI detection (`which` command)
4. Add process output capture
5. Unit tests for CLI execution

**Acceptance Criteria**:
- Can detect if `claude` CLI installed
- Can detect if `gemini` CLI installed
- Can execute commands and capture output
- Handles timeouts correctly

---

#### Phase 2: Provider Implementations (3-4 hours)

**Files to Create**:
- `codex-rs/tui/src/providers/claude.rs`
- `codex-rs/tui/src/providers/gemini.rs`

**Tasks**:
1. Implement `ClaudeProvider`
   - Build command with proper flags
   - Parse JSON response
   - Extract model, usage, response text
2. Implement `GeminiProvider`
   - Build command with model selection
   - Parse text response
   - Create normalized Response object
3. Add authentication verification
4. Add error handling
5. Integration tests

**Acceptance Criteria**:
- Can execute prompt via `claude` CLI
- Can execute prompt via `gemini` CLI
- Responses normalized to common format
- Errors handled gracefully

---

#### Phase 3: Model Router Integration (2-3 hours)

**Files to Modify**:
- `codex-rs/tui/src/chatwidget/mod.rs` (integrate router)
- `codex-rs/common/src/model_presets.rs` (extend with provider info)

**Tasks**:
1. Create `ModelRouter` component
2. Add provider detection logic
3. Route based on selected model
4. Update spec-kit command execution
5. Maintain ChatGPT OAuth path
6. Add logging for debugging

**Acceptance Criteria**:
- ChatGPT models still work (native OAuth)
- Claude models route through CLI
- Gemini models route through CLI
- Model selection via `/model` triggers routing

---

#### Phase 4: Error Handling & UX (1-2 hours)

**Files to Modify**:
- Add user-facing error messages
- Create setup documentation

**Tasks**:
1. Detect CLI not installed → show install instructions
2. Detect CLI not authenticated → show auth instructions
3. Handle execution failures → clear error messages
4. Add retry logic for transient failures
5. Create user documentation

**Error Message Examples**:
```
Claude CLI not found. Install it from:
  https://claude.ai/download

Then authenticate:
  claude

After authentication, retry your command.
```

---

#### Phase 5: Testing & Validation (1-2 hours)

**Test Scenarios**:
1. ✅ ChatGPT model selection (existing flow)
2. ✅ Claude model selection → CLI routing
3. ✅ Gemini model selection → CLI routing
4. ✅ CLI not installed → clear error
5. ✅ CLI not authenticated → clear error
6. ✅ Command timeout → graceful handling
7. ✅ CLI execution failure → error message
8. ✅ All spec-kit commands work with all providers

**Validation**:
- Test with `/speckit.plan SPEC-KIT-XXX` using each provider
- Verify response parsing
- Verify error messages
- Performance testing

---

## Dependencies

### Upstream
- **SPEC-KIT-951**: Multi-Provider OAuth Research (COMPLETE)
- **SPEC-KIT-946**: /model Command Expansion (COMPLETE)

### Downstream (Unblocked by this SPEC)
- **SPEC-KIT-947**: Multi-Provider OAuth Master Validation
- Future: Any spec-kit command can use any model

---

## Success Criteria

### Must Have (MC)

**MC1**: CLI Routing Works
- ✅ Can execute prompts via `claude` CLI
- ✅ Can execute prompts via `gemini` CLI
- ✅ Responses parsed correctly
- ✅ Errors handled gracefully

**MC2**: Model Selection Integration
- ✅ `/model` command triggers provider routing
- ✅ Spec-kit commands use selected model
- ✅ ChatGPT OAuth still works

**MC3**: User Experience
- ✅ Clear error messages
- ✅ Setup instructions provided
- ✅ Authentication transparent

### Should Have (SC)

**SC1**: Performance
- ⭕ CLI execution overhead <2 seconds
- ⭕ No UI blocking during execution

**SC2**: Reliability
- ⭕ Retry logic for transient failures
- ⭕ Graceful degradation

### Could Have (CH)

**CH1**: Advanced Features
- ⏸️ Connection pooling (keep CLI process alive)
- ⏸️ Streaming output support
- ⏸️ Model capability detection

---

## Risks & Mitigation

### Risk 1: CLI Not Installed (HIGH)

**Impact**: Users can't use Claude/Gemini models

**Mitigation**:
- Clear error messages with install instructions
- Documentation in CLAUDE.md
- Pre-flight check on first use

**Owner**: Implementation phase 4

---

### Risk 2: CLI Not Authenticated (HIGH)

**Impact**: Commands fail even though CLI installed

**Mitigation**:
- Detect authentication status
- Provide clear auth instructions
- Test command to verify

**Owner**: Implementation phase 2

---

### Risk 3: CLI Output Format Changes (MEDIUM)

**Impact**: Response parsing breaks on CLI updates

**Mitigation**:
- Use stable output formats (`--output-format json`)
- Version compatibility checks
- Fallback parsers for different versions
- Automated tests

**Owner**: Implementation phase 2

---

### Risk 4: Performance Overhead (LOW)

**Impact**: CLI subprocess slower than direct API

**Mitigation**:
- Acceptable for batch spec-kit operations
- Can implement connection pooling if needed
- Trade-off for simplified auth

**Owner**: Implementation phase 5 (validate acceptable)

---

## Estimated Effort

**Total**: 10-15 hours

**Breakdown**:
- Phase 1 (CLI Infrastructure): 3-4 hours
- Phase 2 (Provider Implementations): 3-4 hours
- Phase 3 (Model Router Integration): 2-3 hours
- Phase 4 (Error Handling & UX): 1-2 hours
- Phase 5 (Testing & Validation): 1-2 hours

**Timeline**: 2-3 days (part-time) or 1-2 days (full-time)

---

## Open Questions

### Q1: Model Capability Detection
**Q**: How do we know which models support which features (tools, streaming, etc.)?
**A**: Start simple - assume all models support basic prompts. Add capability detection later if needed.

### Q2: Streaming Output Support
**Q**: Should we support streaming responses from CLIs?
**A**: No for v1. Batch mode sufficient for spec-kit commands. Add later if needed.

### Q3: Connection Pooling
**Q**: Should we keep CLI processes alive to reduce overhead?
**A**: No for v1. Subprocess overhead acceptable for spec-kit use case. Optimize if performance issue.

### Q4: Multiple Provider Versions
**Q**: How to handle different CLI versions?
**A**: Document minimum required versions. Add version checks if compatibility issues arise.

---

## Success Metrics

### Functional Metrics
- ✅ All 3 providers (ChatGPT, Claude, Gemini) working
- ✅ 100% of spec-kit commands work with all providers
- ✅ Error rate <5% (excluding user auth issues)

### Performance Metrics
- ⭕ CLI execution overhead <2 seconds (90th percentile)
- ⭕ Total command time competitive with native OAuth

### User Experience Metrics
- ⭕ Clear error messages (user testing)
- ⭕ Setup instructions sufficient (no support requests)

---

## Next Steps After Completion

### If Successful
1. Update SPEC-KIT-947 with CLI routing test scenarios
2. Document CLI setup in CLAUDE.md
3. Update /model command documentation
4. Announce multi-provider support

### If Issues Arise
1. Document blockers
2. Implement fallbacks
3. Consider hybrid approach (some providers CLI, some OAuth)

---

## Appendix: Research References

- **SPEC-KIT-951**: Multi-Provider OAuth Research (research findings)
- **SPEC-KIT-951/CLI-ROUTING-APPROACH.md**: Detailed architecture design
- **SPEC-KIT-951/RESEARCH-REPORT.md**: OAuth landscape analysis

---

**END OF PRD**
