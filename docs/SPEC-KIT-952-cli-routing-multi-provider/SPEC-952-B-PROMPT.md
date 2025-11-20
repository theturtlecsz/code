# SPEC-952-B: Gemini Multi-Turn History Management Layer

**Copy this entire prompt to create the new SPEC**

---

## Context: Continuation of SPEC-KIT-952

**Parent SPEC**: SPEC-KIT-952 (CLI Routing for Multi-Provider Model Support)
**Status**: ✅ **COMPLETE** - Claude CLI routing working (3 models)
**New SPEC**: SPEC-952-B - Gemini history management layer

---

## What SPEC-952 Delivered

**Working Implementation (Claude)**:
- ✅ CLI routing with streaming (claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5)
- ✅ Multi-turn conversation support via native CLI session management
- ✅ Model name mapping (presets → API names)
- ✅ Queue routing (prevents OAuth fallback)
- ✅ Error handling (CLI not found, auth failure, timeout)

**Infrastructure Created**:
- Core: `codex-rs/core/src/cli_executor/` (executors, stream parsers, context manager)
- TUI: `codex-rs/tui/src/providers/claude_streaming.rs`
- Router: `codex-rs/tui/src/model_router.rs::execute_with_cli_streaming()`
- Tests: 12/12 passing for Claude

---

## The Gemini Problem

**Discovery**: Gemini CLI headless mode is **stateless** (no native multi-turn support).

**Evidence**:
1. ✅ Single-turn messages work perfectly (gemini-2.5-flash: "What's 2+2?" → "4" in 3s)
2. ❌ Multi-turn conversations timeout (120s) with formatted history
3. ❌ Gemini CLI lacks `--continue` or `--resume` flags (unlike Claude)
4. ✅ Interactive mode (`gemini`) supports multi-turn, but headless (`-p`) doesn't

**Root Cause**:
- Claude CLI: Session-aware, manages conversation state internally
- Gemini CLI: Stateless, treats each call as independent prompt
- Current approach: Sends formatted history as text (works for Claude, fails for Gemini)

**Technical Details**:
- Gemini CLI `--output-format stream-json` works correctly
- History format causes timeouts: `--- Previous Conversation ---` with full transcript
- Manual test: Entire formatted history treated as single 10,351-token user message
- Result: Slow processing → 120s timeout

---

## Research Summary

**From**: `docs/SPEC-KIT-952-cli-routing-multi-provider/gemini-cli-multi-turn-research.md`

**Key Findings**:

1. **Gemini CLI supports multi-turn** - but only in interactive mode
   - Interactive: Full stateful conversations, `/chat save/resume`, history persistence
   - Headless: Stateless, single-turn by design

2. **Headless mode capabilities**:
   - ✅ `--output-format stream-json` for structured responses
   - ✅ `--model` parameter for model selection
   - ❌ No `--session-id` or conversation resumption
   - ❌ No documented multi-turn mechanism

3. **Multi-turn is achievable** via history management on our side:
   - Build conversation state in application layer
   - Format compact prompts with history on each call
   - Use summarization to prevent token explosion
   - Similar to how stateless LLM APIs work (e.g., OpenAI API)

4. **Proven pattern**: Many LLM wrappers manage multi-turn with stateless backends
   - Keep `Vec<Message>` in application
   - Include recent messages + summarized context in each prompt
   - Compress/truncate when approaching limits

---

## Proposed Architecture: GeminiHistoryManager

### Overview

Implement **client-side conversation management** for Gemini CLI (stateless backend).

**Design**:
```rust
struct GeminiHistoryManager {
    messages: Vec<Message>,           // Full conversation history
    max_prompt_tokens: usize,         // Soft limit (~8-40k tokens)
    summary_threshold: usize,         // When to summarize old messages
}

impl GeminiHistoryManager {
    fn add_user(&mut self, content: String) { /* ... */ }
    fn add_assistant(&mut self, content: String) { /* ... */ }

    fn build_prompt(&self, new_user: &str) -> String {
        // 1. Start with compact context header
        // 2. Include summary if conversation is long
        // 3. Append recent messages verbatim (last 5-10)
        // 4. Append new user message
        // 5. Stay under token budget
    }

    fn maybe_compress(&mut self) {
        // If estimated tokens > threshold:
        //   - Summarize older messages (use fast model like gemini-2.5-flash)
        //   - Replace with single System message
        //   - Keep recent N messages verbatim
    }
}
```

### Prompt Format (Compact)

**Good** (compact, token-efficient):
```text
You are continuing a coding conversation.

Summary: User is building CLI routing for Claude/Gemini. We discussed Claude working perfectly, Gemini needing history management.

Recent messages:
User: I'm seeing Gemini timeout with full history
Assistant: That's because Gemini CLI is stateless. We need history management.
User: Design a solution for this.

Current message: <new user input>
```

**Bad** (current approach - too verbose):
```text
--- Previous Conversation ---
USER (2025-11-20 22:51):
What's 2+2?

ASSISTANT (2025-11-20 22:51):
4

--- End Previous Conversation ---

USER (current):
my name is alice
```

### Compression Strategy

**Levels**:
1. **No compression** (<5k tokens): Send all messages verbatim
2. **Window compression** (5-15k tokens): Keep first + last N, summarize middle
3. **Heavy compression** (>15k tokens): Summarize everything except last 3-5 exchanges

**Implementation**:
- Use token estimation (chars / 4)
- Reserve headroom for response (4-8k tokens)
- Summarize using `gemini-2.5-flash` (cheap, fast)
- Cache summaries to avoid regenerating

---

## Requirements for SPEC-952-B

### Functional Requirements

**FR-1**: Multi-turn Gemini conversations
- Users can have extended conversations with Gemini models
- Context preserved across messages
- No timeout errors under normal use

**FR-2**: Automatic history management
- Application maintains conversation state
- No user configuration required
- Transparent compression when needed

**FR-3**: Token budget management
- Prompts stay under configurable limit (default: 16k tokens)
- Automatic summarization when approaching limit
- Warning if conversation becomes too long

**FR-4**: Model compatibility
- Works with all Gemini models (3-pro-preview, 2.5-pro, 2.5-flash)
- Respects model-specific context limits
- Graceful degradation for very long conversations

### Non-Functional Requirements

**NFR-1**: Performance
- First message: <5s (no history)
- Follow-up messages: <10s (with history)
- Summarization: <3s (background, async)

**NFR-2**: Reliability
- No timeouts under normal use (<15k token conversations)
- Graceful handling of very long conversations
- Clear error messages if limits exceeded

**NFR-3**: Maintainability
- Reuses existing CLI executor infrastructure
- Minimal changes to router/chatwidget
- Well-tested (unit + integration tests)

---

## Implementation Scope

### In Scope

1. **GeminiHistoryManager** (new module)
   - Conversation state management
   - Prompt building with history
   - Token estimation and budgeting
   - Compression logic

2. **GeminiStreamingProvider updates**
   - Integrate history manager
   - Use compact prompt format
   - Handle summarization

3. **Summarization service** (optional)
   - Async background summarization
   - Cache summaries to avoid regenerating
   - Use gemini-2.5-flash for cost efficiency

4. **Testing**
   - Unit tests for history manager
   - Integration tests for multi-turn conversations
   - Performance validation (<10s response times)

5. **Documentation**
   - Update CLAUDE.md (Gemini support)
   - Document token budgets and compression behavior
   - Troubleshooting guide

### Out of Scope

- PTY-based interactive Gemini sessions (future R&D)
- Gemini native API integration (different SPEC)
- UI changes for conversation management
- Advanced compression algorithms (start simple)

---

## Technical Design

### File Structure

```
codex-rs/core/src/cli_executor/
├── gemini_history.rs        (NEW) - GeminiHistoryManager
└── context.rs               (UPDATE) - Gemini-specific formatting

codex-rs/tui/src/providers/
└── gemini_streaming.rs      (UPDATE) - Integrate history manager

codex-rs/tui/src/
└── model_router.rs          (MINOR UPDATE) - Ensure Gemini uses streaming

Tests:
codex-rs/core/tests/
└── gemini_history_tests.rs  (NEW) - History manager tests
```

### Data Flow

```
User types message in TUI
    ↓
ChatWidget: Get current model (gemini-2.5-flash)
    ↓
GeminiHistoryManager: Add user message to history
    ↓
GeminiHistoryManager: Build compact prompt
    - Check token budget
    - Summarize if needed (async)
    - Format: Summary + Recent messages + Current message
    ↓
GeminiStreamingProvider: Execute with CLI
    ↓
GeminiCliExecutor: Spawn `gemini --output-format stream-json -p "<compact prompt>"`
    ↓
Stream parser: Extract deltas from {"type":"message","role":"assistant","content":"..."}
    ↓
GeminiHistoryManager: Add assistant response to history
    ↓
User sees response, conversation continues
```

### History Manager API

```rust
pub struct GeminiHistoryManager {
    messages: Vec<Message>,
    config: HistoryConfig,
}

pub struct HistoryConfig {
    pub max_prompt_tokens: usize,      // Default: 16_000
    pub summary_threshold: usize,      // Default: 8_000
    pub recent_message_count: usize,   // Default: 8 (4 exchanges)
    pub enable_compression: bool,      // Default: true
}

impl GeminiHistoryManager {
    pub fn new(config: HistoryConfig) -> Self;

    pub fn add_message(&mut self, role: Role, content: String);

    pub fn build_prompt(&self, new_user_message: &str) -> PromptResult {
        // Returns: (prompt_text, estimated_tokens, was_compressed)
    }

    pub fn clear_history(&mut self);

    pub fn message_count(&self) -> usize;

    pub fn estimated_total_tokens(&self) -> usize;

    async fn summarize_history(&self) -> Result<String, Error> {
        // Use gemini-2.5-flash to summarize older messages
        // Returns compact summary paragraph
    }
}
```

### Prompt Template

```rust
fn format_compact_prompt(
    summary: Option<&str>,
    recent_messages: &[Message],
    current_message: &str,
) -> String {
    let mut prompt = String::from(
        "You are an AI coding assistant helping with a software project.\n\n"
    );

    if let Some(summary_text) = summary {
        prompt.push_str("## Conversation Summary\n");
        prompt.push_str(summary_text);
        prompt.push_str("\n\n");
    }

    if !recent_messages.is_empty() {
        prompt.push_str("## Recent Messages\n");
        for msg in recent_messages {
            let role = match msg.role {
                Role::User => "User",
                Role::Assistant => "Assistant",
            };
            prompt.push_str(&format!("{}: {}\n\n", role, msg.content));
        }
    }

    prompt.push_str("## Current Message\n");
    prompt.push_str(current_message);

    prompt
}
```

---

## Testing Strategy

### Unit Tests

1. **History building**: Verify prompt format with 0, 1, 5, 20 messages
2. **Token estimation**: Validate estimation accuracy (±10%)
3. **Compression triggers**: Verify summarization at threshold
4. **Message retention**: Confirm recent messages preserved
5. **Edge cases**: Empty messages, very long messages, unicode

### Integration Tests

1. **Single-turn**: Verify baseline (no history) works
2. **3-turn conversation**: User → Assistant → User → Assistant → User
3. **Long conversation**: 20+ messages, verify compression
4. **Token limits**: Approach max tokens, verify graceful handling
5. **All 3 Gemini models**: 3-pro-preview, 2.5-pro, 2.5-flash

### Performance Tests

1. **First message**: <5s (no history)
2. **Follow-up (3 msgs)**: <8s (small history)
3. **Follow-up (20 msgs)**: <12s (with compression)
4. **Summarization**: <5s (async, gemini-2.5-flash)

---

## Success Criteria

SPEC-952-B is **COMPLETE** when:

- ✅ All 3 Gemini models support multi-turn conversations
- ✅ No 120s timeouts under normal use (<20 message conversations)
- ✅ History preserved correctly (context maintained across turns)
- ✅ Automatic compression works (conversations >8k tokens)
- ✅ Performance acceptable (<10s response times with history)
- ✅ Tests pass (10+ unit, 5+ integration)
- ✅ Documentation updated (CLAUDE.md, README.md)
- ✅ SPEC.md updated with completion status

---

## Implementation Phases

### Phase 1: History Manager Core (2-3 hours)

**Deliverables**:
1. `gemini_history.rs` with `GeminiHistoryManager`
2. Compact prompt formatting
3. Token estimation
4. Basic compression (window-based)
5. Unit tests (8+ tests)

**Acceptance**:
- Can build prompts with 0-20 messages
- Token estimation within ±15% of actual
- Compression triggers at threshold
- All unit tests pass

### Phase 2: Provider Integration (1-2 hours)

**Deliverables**:
1. Update `GeminiStreamingProvider` to use history manager
2. Integration with existing router (`execute_with_cli_streaming`)
3. Message persistence across turns
4. Error handling for compression failures

**Acceptance**:
- Multi-turn conversations work end-to-end
- History maintained correctly
- No OAuth fallback errors

### Phase 3: Summarization Service (2-3 hours)

**Deliverables**:
1. Async summarization using gemini-2.5-flash
2. Summary caching (avoid regenerating)
3. Fallback if summarization fails (truncate oldest)
4. Performance optimization

**Acceptance**:
- Summarization completes in <5s
- Long conversations (20+ msgs) don't timeout
- Graceful degradation if summarization unavailable

### Phase 4: Testing & Validation (1-2 hours)

**Deliverables**:
1. Integration tests (5+ tests)
2. Manual testing all 3 Gemini models
3. Performance measurement
4. Edge case validation

**Acceptance**:
- All tests pass (15+ total)
- All 3 models work with multi-turn
- Performance meets targets (<10s)

### Phase 5: Documentation (30 min)

**Deliverables**:
1. Update CLAUDE.md (Gemini support complete)
2. Create README.md with architecture details
3. Update SPEC.md (mark SPEC-952-B complete)
4. Troubleshooting guide

---

## Reference Implementation (Claude)

**Reuse these patterns**:

1. **Stream parsing**: `codex-rs/core/src/cli_executor/stream.rs::parse_gemini_stream`
   - Already parses Gemini stream-json format correctly
   - Handles message, result, error events

2. **Provider structure**: `codex-rs/tui/src/providers/claude_streaming.rs`
   - Clean separation: provider → executor → CLI
   - Event streaming to TUI
   - Error mapping

3. **Router integration**: `codex-rs/tui/src/model_router.rs::execute_with_cli_streaming`
   - Simple routing based on provider type
   - Consistent interface for all providers

**Key difference for Gemini**:
- Claude: `CliContextManager::format_history()` works (CLI handles multi-turn)
- Gemini: Need `GeminiHistoryManager::build_compact_prompt()` (we handle multi-turn)

---

## Token Budgets

**Model limits** (from Gemini docs):
- gemini-3-pro-preview: 1M tokens (use 900k safe limit)
- gemini-2.5-pro: 1M tokens (use 900k safe limit)
- gemini-2.5-flash: 1M tokens (use 900k safe limit)

**Practical limits** (performance-based):
- Target: 8-16k tokens per prompt (fast responses)
- Warning: 16-40k tokens (slower, acceptable)
- Maximum: 40-60k tokens (edge cases, may be slow)
- Compress: >8k tokens total conversation

**Why conservative limits**:
- Faster responses (3-10s vs 30-120s)
- More predictable performance
- Easier to debug and test
- Better user experience

---

## Example Conversations

### Scenario 1: Short Conversation (No Compression)

```
Turn 1:
User: What's 2+2?
[History: 1 message, ~10 tokens]
Prompt: "You are an AI assistant.\n\nCurrent message: What's 2+2?"
Response: "4"

Turn 2:
User: What about 3+3?
[History: 3 messages, ~30 tokens]
Prompt: "You are an AI assistant.\n\nRecent messages:\nUser: What's 2+2?\nAssistant: 4\n\nCurrent message: What about 3+3?"
Response: "6"
```

### Scenario 2: Long Conversation (With Compression)

```
Turn 15 (after 14 exchanges, ~12k tokens):
[Trigger compression]

Summarization:
- Input: Messages 1-10 (8k tokens)
- Output: "User is building CLI routing system. We implemented Claude successfully, working on Gemini multi-turn support." (~50 tokens)

Prompt format:
"You are an AI assistant.

Summary of earlier conversation:
User is building CLI routing system. We implemented Claude successfully, working on Gemini multi-turn support.

Recent messages:
User: How should I test this?
Assistant: Test with gemini-2.5-flash first, it's fastest.
User: Good idea, let me try that.
Assistant: Make sure to check for timeouts.

Current message: It's working! What's next?"
```

---

## Error Handling

**Timeout (>120s)**:
- Check conversation size
- Auto-compress if >8k tokens
- Retry with compressed prompt
- If still fails: Warn user, offer to clear history

**Summarization failure**:
- Fallback: Truncate oldest messages (drop first N)
- Continue with reduced context
- Log warning for debugging

**Empty history**:
- Treat as single-turn (no history context needed)
- Works exactly like current single-message behavior

---

## Migration from SPEC-952

**What stays the same**:
- ✅ GeminiCliExecutor (working correctly)
- ✅ Stream parser (parse_gemini_stream working)
- ✅ Model name mapping (gemini-3-pro → gemini-3-pro-preview)
- ✅ Router structure (execute_with_cli_streaming)

**What changes**:
- 🔄 GeminiStreamingProvider: Add history manager
- 🔄 Prompt building: Use compact format (not verbose format)
- 🆕 GeminiHistoryManager: New module for conversation state
- 🆕 Compression: Add summarization logic

**Migration is additive** - no breaking changes to existing Claude implementation.

---

## Dependencies

**From SPEC-952** (already complete):
- ✅ CLI executor infrastructure
- ✅ Stream parsing (gemini-stream working)
- ✅ Router integration
- ✅ Queue routing fix
- ✅ Model name mapping

**New dependencies**:
- Async task spawning (for summarization)
- Token estimation library (or heuristic)
- Message persistence across turns

**No external crates required** - can build with existing dependencies.

---

## Effort Estimate

**Total**: 6-10 hours

| Phase | Tasks | Estimated Time |
|-------|-------|----------------|
| Phase 1 | History manager core + tests | 2-3 hours |
| Phase 2 | Provider integration | 1-2 hours |
| Phase 3 | Summarization service | 2-3 hours |
| Phase 4 | Testing & validation | 1-2 hours |
| Phase 5 | Documentation | 30 min |

**Contingency**: +2 hours for unexpected issues

---

## Success Metrics

**Before (SPEC-952 incomplete)**:
- Gemini single-turn: ✅ Working
- Gemini multi-turn: ❌ 120s timeout
- Gemini usability: ⚠️ Limited (no context)

**After (SPEC-952-B complete)**:
- Gemini single-turn: ✅ Working
- Gemini multi-turn: ✅ Working (<10s)
- Gemini usability: ✅ Full conversation support
- Total models: 6/6 working (3 Claude + 3 Gemini)

---

## Files to Reference

**Current implementation** (SPEC-952):
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-1-COMPLETE.md`
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-2-PROMPT.md`
- `docs/SPEC-KIT-952-cli-routing-multi-provider/TEST-PLAN.md`
- `docs/SPEC-KIT-952-cli-routing-multi-provider/gemini-cli-multi-turn-research.md`

**Code to reference**:
- `codex-rs/core/src/cli_executor/claude.rs` - Claude executor pattern
- `codex-rs/core/src/cli_executor/context.rs` - History formatting (adapt for Gemini)
- `codex-rs/tui/src/providers/claude_streaming.rs` - Provider pattern to replicate

**Tests to reference**:
- `codex-rs/core/tests/cli_executor_tests.rs` - CLI executor tests
- `codex-rs/tui/tests/providers_tests.rs` - Provider tests

---

## Next Steps

### Creating the SPEC

Use `/speckit.new` to create SPEC-952-B:

```bash
/speckit.new Gemini multi-turn history management layer for CLI routing. Implement GeminiHistoryManager with conversation state, compact prompt formatting, automatic compression when >8k tokens, summarization for long conversations. Integrates with existing GeminiStreamingProvider (SPEC-952). Enables multi-turn conversations for all 3 Gemini models without timeouts. Estimated 6-10 hours.
```

### After SPEC creation

1. Review PRD generated by `/speckit.new`
2. Run `/speckit.specify SPEC-952-B` for detailed requirements
3. Use `/speckit.plan SPEC-952-B` for work breakdown
4. Implement using `/speckit.auto SPEC-952-B` or manual stages

---

## Context for Local-Memory

**Key learnings stored**:
1. ✅ SPEC-952 Claude implementation complete (ID: 5c9a98fd)
2. ✅ Gemini CLI stateless discovery (ID: 4f78887b)
3. ✅ Queue routing bug fix (ID: a117fe2a)
4. ✅ Model name mapping pattern (ID: 0de72546)

**Query for context**:
```
Use mcp__local-memory__search:
- query: "SPEC-952 CLI routing Gemini stateless"
- tags: ["spec:SPEC-KIT-952"]
- limit: 5
```

---

## Questions to Resolve During Implementation

1. **Summarization model**: Use gemini-2.5-flash or claude-haiku-4.5? (Flash cheaper, Haiku might understand context better)
2. **Summary cache**: How long to cache? (Session-based, time-based, or conversation-based?)
3. **Token budget**: Start with 16k or more conservative 8k? (Affects performance)
4. **Compression strategy**: Window-based or semantic clustering? (Start simple, optimize later)
5. **Error recovery**: Clear history, truncate, or fail gracefully?

---

## Ready to Create SPEC-952-B!

**This prompt contains everything needed to implement Gemini multi-turn support.**

**Estimated completion**: 1-2 days (6-10 hours implementation + testing)

**After completion**: Full multi-provider support (6/6 models working) ✨
