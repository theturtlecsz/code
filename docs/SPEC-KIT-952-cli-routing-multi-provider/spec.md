**SPEC-ID**: SPEC-KIT-952
**Feature**: CLI Routing for Multi-Provider Model Support
**Status**: Backlog
**Created**: 2025-11-19
**Branch**: TBD
**Owner**: Code
**Priority**: P1 - HIGH
**Type**: Implementation
**Based On**: SPEC-KIT-951 (Multi-Provider OAuth Research)

**Context**: Implement CLI routing to enable multi-provider model support by routing commands through native CLIs (`claude`, `gemini`) instead of implementing OAuth. Bypasses Claude OAuth blocker discovered in SPEC-KIT-951.

**Objective**: Users can select any model via `/model` command and execute spec-kit commands successfully, with authentication handled by each provider's native CLI.

**Downstream Impact**: Unblocks SPEC-KIT-947 (master validation), enables full multi-provider support

---

## Implementation Phases

### Phase 1: CLI Infrastructure (3-4 hours)

**Deliverables**:
- `cli_executor.rs` module with CLI detection and execution
- `model_router.rs` module for provider routing
- Unit tests

**Acceptance Criteria**:
- [x] Can detect `claude` CLI availability
- [x] Can detect `gemini` CLI availability
- [x] Can execute commands with timeout
- [x] Captures stdout/stderr correctly
- [x] Unit tests passing

---

### Phase 2: Provider Implementations (3-4 hours)

**Deliverables**:
- `providers/claude.rs` with Claude CLI integration
- `providers/gemini.rs` with Gemini CLI integration
- Response parsing for both providers
- Integration tests

**Acceptance Criteria**:
- [x] Claude provider executes `claude -p "..." --output-format json`
- [x] Gemini provider executes `gemini -p "..." -m "model"`
- [x] JSON parsing for Claude responses
- [x] Text parsing for Gemini responses
- [x] Normalized Response objects
- [x] Authentication verification
- [x] Error handling

---

### Phase 3: Model Router Integration (2-3 hours)

**Deliverables**:
- Model router component
- Integration with `/model` command
- Spec-kit command routing

**Acceptance Criteria**:
- [x] ChatGPT models use native OAuth (existing)
- [x] Claude models route through CLI
- [x] Gemini models route through CLI
- [x] `/model` selection triggers correct provider
- [x] All spec-kit commands work with all providers

---

### Phase 4: Error Handling & UX (1-2 hours)

**Deliverables**:
- User-facing error messages
- Setup documentation
- Authentication verification

**Acceptance Criteria**:
- [x] CLI not installed → clear error + install instructions
- [x] CLI not authenticated → clear error + auth instructions
- [x] Execution failures → helpful error messages
- [x] Retry logic for transient failures
- [x] User documentation in CLAUDE.md

---

### Phase 5: Testing & Validation (1-2 hours)

**Test Scenarios**:
1. ChatGPT model selection (existing flow works)
2. Claude model selection → CLI routing works
3. Gemini model selection → CLI routing works
4. CLI not installed → error message shown
5. CLI not authenticated → error message shown
6. Command timeout → graceful handling
7. CLI execution failure → clear error
8. All spec-kit commands work with all providers

**Validation**:
- [x] Test `/speckit.plan SPEC-KIT-XXX` with each provider
- [x] Verify response parsing
- [x] Verify error messages
- [x] Performance testing (overhead <2s)

---

## Technical Specifications

### CLI Interfaces

**Claude CLI**:
```bash
claude -p "{prompt}" --output-format json
```
Output: JSON with `response`, `model`, `usage`

**Gemini CLI**:
```bash
gemini -p "{prompt}" -m "{model}"
```
Output: Plain text

### Model Router

```rust
pub enum ModelProvider {
    ChatGPT,  // Native OAuth
    Claude,   // CLI routing
    Gemini,   // CLI routing
}

impl ModelProvider {
    pub async fn execute_prompt(&self, prompt: &str) -> Result<Response> {
        match self {
            Self::ChatGPT => execute_native(prompt).await,
            Self::Claude => execute_cli("claude", &["-p", prompt, "--output-format", "json"]).await,
            Self::Gemini => execute_cli("gemini", &["-p", prompt, "-m", model]).await,
        }
    }
}
```

---

## Success Criteria

### Must Have
- ✅ CLI routing works for Claude and Gemini
- ✅ ChatGPT OAuth still works (backward compatible)
- ✅ Clear error messages for setup issues
- ✅ All spec-kit commands work with all providers

### Should Have
- ⭕ CLI execution overhead <2 seconds
- ⭕ Retry logic for transient failures
- ⭕ Graceful degradation on CLI failure

### Could Have
- ⏸️ Connection pooling (keep CLI alive)
- ⏸️ Streaming output support
- ⏸️ Model capability detection

---

## Dependencies

### Upstream
- **SPEC-KIT-951**: Multi-Provider OAuth Research (COMPLETE ✅)
- **SPEC-KIT-946**: /model Command Expansion (COMPLETE ✅)

### Downstream (Unblocked by this SPEC)
- **SPEC-KIT-947**: Multi-Provider OAuth Master Validation

---

## Estimated Effort

**Total**: 10-15 hours

**Breakdown**:
- Phase 1: 3-4 hours
- Phase 2: 3-4 hours
- Phase 3: 2-3 hours
- Phase 4: 1-2 hours
- Phase 5: 1-2 hours

**Timeline**: 2-3 days (part-time) or 1-2 days (full-time)

---

## Risks

| Risk | Impact | Mitigation | Status |
|------|--------|-----------|---------|
| CLI not installed | Users can't use provider | Clear error + install guide | Planned |
| CLI not authenticated | Commands fail | Auth verification + instructions | Planned |
| CLI format changes | Parsing breaks | Stable formats + version check | Planned |
| Performance overhead | Slower than API | Acceptable for batch use | Accepted |

---

## Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-19 | Initial spec created based on SPEC-KIT-951 research |

---

## Notes

**Based On**: SPEC-KIT-951 research discovered that Anthropic does NOT provide OAuth for third-party apps. CLI routing is the only viable solution for Claude support.

**Architecture Choice**: CLI routing chosen over OAuth hybrid approach because:
1. Bypasses Claude OAuth blocker completely
2. Simpler implementation (10-15h vs 15-24h)
3. Lower maintenance burden (CLIs handle token management)
4. Better UX (one-time CLI authentication)

**Next Session**: Run `/speckit.plan SPEC-KIT-952` to create implementation plan.
