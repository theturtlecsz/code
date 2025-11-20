# SPEC-KIT-952: CLI Routing for Multi-Provider Model Support

**Status**: ✅ **COMPLETE** (Claude-only, 2025-11-20)
**Continuation**: SPEC-952-B (Gemini history management) - see `SPEC-952-B-PROMPT.md`

---

## Summary

Implemented production-grade CLI routing with streaming for **Claude models** using external CLI processes.

**Delivered**:
- 3 Claude models working with multi-turn conversation support
- Streaming responses with real-time delta updates
- Clean integration with existing ChatGPT OAuth flow
- Error handling and user-friendly messages

**Not included** (deferred to SPEC-952-B):
- Gemini CLI routing (requires history management layer)

---

## Architecture

### Production Routing Strategy

| Provider | Models | Auth Method | Implementation | Status |
|----------|--------|-------------|----------------|--------|
| **ChatGPT** | gpt-5, gpt-5.1-*, gpt-5-codex | Native OAuth | Existing codex-core | ✅ Working |
| **Claude** | claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5 | CLI routing | SPEC-952 | ✅ Working |
| **Gemini** | gemini-3-pro, gemini-2.5-pro, gemini-2.5-flash | History layer needed | SPEC-952-B | ⏸️ Pending |

### Why CLI Routing for Claude?

**Advantages**:
1. ✅ No API key management - users authenticate CLIs once
2. ✅ Simpler setup - `claude` command handles auth
3. ✅ Consistent UX - same auth flow as standalone CLI usage
4. ✅ Session management - CLI handles token refresh, rate limits
5. ✅ Official support - using vendor-provided tools

**Trade-offs accepted**:
- ~2-25s response latency (higher than expected 4-6s)
- Re-sends history each request (acceptable with compression)
- Depends on external binary (validated in Phase 0)

---

## Implementation Details

### Core Components

**1. CLI Executor Layer** (`codex-rs/core/src/cli_executor/`):
```
cli_executor/
├── mod.rs           - CliExecutor trait, public API
├── claude.rs        - ClaudeCliExecutor implementation
├── gemini.rs        - GeminiCliExecutor implementation (single-turn only)
├── context.rs       - CliContextManager for history formatting
├── stream.rs        - Stream parsers (parse_claude_stream, parse_gemini_stream)
└── types.rs         - Shared types (Conversation, Message, StreamEvent, etc.)
```

**2. TUI Provider Layer** (`codex-rs/tui/src/providers/`):
```
providers/
├── claude_streaming.rs  - ClaudeStreamingProvider (bridges executor to TUI)
└── gemini_streaming.rs  - GeminiStreamingProvider (single-turn only)
```

**3. Router Integration** (`codex-rs/tui/src/model_router.rs`):
- `execute_with_cli_streaming()` - Main routing function (lines 410-455)
- Routes Claude → ClaudeStreamingProvider
- Routes Gemini → GeminiStreamingProvider (limited)
- Routes ChatGPT → Error (should use native OAuth)

**4. ChatWidget Integration** (`codex-rs/tui/src/chatwidget/mod.rs`):
- Line 5660: Check if model supports CLI routing
- Line 5711: Call `execute_with_cli_streaming()`
- Lines 5093-5124: Queue routing (prevents OAuth fallback)

### Data Flow

```
User types message in TUI
    ↓
ChatWidget: Check model type
    ↓
supports_native_streaming() returns true for Claude/Gemini
    ↓
Build conversation history (context_manager::Message format)
    ↓
execute_with_cli_streaming(model, messages, tx)
    ↓
ClaudeStreamingProvider::execute_streaming()
    ↓
Convert messages: context_manager::Message → cli_executor::Message
Map model name: claude-sonnet-4.5 → claude-sonnet-4-5-20250929
    ↓
ClaudeCliExecutor::execute(conversation, current_message)
    ↓
Format history: CliContextManager::format_history()
    ↓
Spawn: `claude --print --output-format stream-json --model claude-sonnet-4-5-20250929`
    ↓
Write formatted history to stdin, close stdin
    ↓
Parse stdout: parse_claude_stream()
  - {"type":"system",...} → Log metadata
  - {"type":"assistant","message":{"content":[{"type":"text","text":"..."}]}} → Delta
    ↓
Stream events to TUI:
  - StreamEvent::Delta(text) → tx.send_native_stream_delta()
  - StreamEvent::Metadata(usage) → tx.send_native_stream_complete()
    ↓
User sees response appear incrementally ✨
```

---

## Model Name Mapping

**Claude models** (presets → API names):
- `claude-opus-4.1` → `claude-opus-4-1-20250805`
- `claude-sonnet-4.5` → `claude-sonnet-4-5-20250929`
- `claude-haiku-4.5` → `claude-haiku-4-5-20251001`

**Gemini models** (for SPEC-952-B):
- `gemini-3-pro` → `gemini-3-pro-preview`
- `gemini-2.5-pro` → `gemini-2.5-pro` (unchanged)
- `gemini-2.5-flash` → `gemini-2.5-flash` (unchanged)

---

## Testing Results

### Test Matrix (Claude Models)

| Test | Model | Input | Expected | Result | Notes |
|------|-------|-------|----------|--------|-------|
| Single message | claude-sonnet-4.5 | "What's 2+2?" | "4" with streaming | ✅ PASS | ~20s response |
| Multi-turn | claude-opus-4.1 | "My name is Alice" → "What's my name?" | "Alice" | ✅ PASS | 2-3s first, 25s second |
| Streaming visible | claude-sonnet-4.5 | Any message | Incremental text | ✅ PASS | Delta events working |
| Token usage | All Claude models | Any message | Token counts displayed | ✅ PASS | Metadata received |
| History preservation | claude-opus-4.1 | 3+ turn conversation | Context maintained | ✅ PASS | Full history preserved |

### Gemini Results (Deferred to SPEC-952-B)

| Test | Model | Result | Notes |
|------|-------|--------|-------|
| Single message | gemini-2.5-flash | ✅ PASS | "What's 2+2?" → "4" in ~3s |
| Multi-turn | gemini-2.5-flash | ❌ FAIL | 120s timeout with history |
| Single message | gemini-2.5-pro | ✅ PASS | Works without history |
| Multi-turn | gemini-2.5-pro | ❌ FAIL | 120s timeout with history |
| Single message | gemini-3-pro | ❌ TIMEOUT | 120s timeout (even single-turn) |

**Conclusion**: Gemini requires different history approach (SPEC-952-B).

---

## Performance Characteristics

**Claude (measured)**:
- Cold start: 2-25s (variable, higher than Phase 0 estimate of 4-6s)
- First message: 2-3s (best case) to 20s (typical)
- Follow-up messages: 25s (with history)
- Streaming: ✅ Visible incremental updates
- Token counts: ✅ Displayed correctly

**Gemini (limited testing)**:
- Single-turn: 3s (fast, working)
- Multi-turn: 120s timeout (broken - needs SPEC-952-B)

---

## Bugs Fixed During Implementation

### 1. Model Name Mapping (404 errors)

**Issue**: Claude API rejected preset names like `claude-sonnet-4.5`
**Fix**: Added `map_model_name()` to streaming providers
**Files**: `claude_streaming.rs:185-199`, `gemini_streaming.rs:185-202`

### 2. Empty Prompt Fallthrough (OAuth bypass)

**Issue**: Empty `prompt_text` caused fallthrough to ChatGPT OAuth
**Fix**: Added early return with warning (chatwidget/mod.rs:5671-5675)
**Impact**: Prevents silent routing failures

### 3. Queue Routing (OAuth fallthrough on rapid messages)

**Issue**: Messages typed while task running sent to ChatGPT OAuth for all models
**Fix**: Added CLI model check in queue handler (chatwidget/mod.rs:5093-5124)
**Impact**: Queued messages now respect CLI routing

### 4. Gemini Stream Parser (no output displayed)

**Issue**: Parser looked for `json["text"]`, Gemini sends `json["content"]`
**Fix**: Updated parser to handle Gemini format (stream.rs:126-151)
**Impact**: Gemini responses now display correctly (single-turn)

---

## Documentation

**Updated files**:
- ✅ `SPEC.md` - Marked SPEC-952 complete (Claude-only)
- ✅ `CLAUDE.md` - Updated with Claude CLI setup, noted Gemini pending
- ✅ `TEST-PLAN.md` - Documented test results
- ✅ `SPEC-952-B-PROMPT.md` - Complete guide for Gemini implementation

**New files**:
- ✅ `ENHANCEMENTS.md` - Tracked model indicator UX request
- ✅ `gemini-cli-multi-turn-research.md` - Comprehensive Gemini architecture research
- ✅ This README

---

## Known Limitations

1. **Gemini multi-turn not supported** - Requires SPEC-952-B implementation
2. **Higher latency than expected** - 2-25s vs expected 4-6s (functional but slower)
3. **Model indicator UX** - No persistent model display (tracked in ENHANCEMENTS.md)
4. **Input parsing bug** - `/model foo` on same line as message gets confused (pre-existing bug)

---

## Files Modified

**Created** (~1,800 LOC):
- `codex-rs/core/src/cli_executor/mod.rs`
- `codex-rs/core/src/cli_executor/claude.rs`
- `codex-rs/core/src/cli_executor/gemini.rs`
- `codex-rs/core/src/cli_executor/context.rs`
- `codex-rs/core/src/cli_executor/stream.rs`
- `codex-rs/core/src/cli_executor/types.rs`
- `codex-rs/tui/src/providers/claude_streaming.rs`
- `codex-rs/tui/src/providers/gemini_streaming.rs`
- `codex-rs/tui/src/model_router.rs`

**Modified** (~100 LOC):
- `codex-rs/tui/src/chatwidget/mod.rs` (lines 5659-5732)
- `codex-rs/core/src/lib.rs` (exports)
- `codex-rs/tui/src/lib.rs` (exports)

**Documentation** (~3,500 words):
- `PHASE-1-COMPLETE.md`
- `PHASE-2-PROMPT.md`
- `TEST-PLAN.md`
- `SPEC-952-B-PROMPT.md`
- `gemini-cli-multi-turn-research.md`
- `README.md` (this file)
- Updated `CLAUDE.md`
- Updated `SPEC.md`

---

## Next Steps

**To implement Gemini support**:

1. Create new SPEC:
   ```bash
   /speckit.new Gemini multi-turn history management layer (see SPEC-952-B-PROMPT.md)
   ```

2. Copy prompt content from `SPEC-952-B-PROMPT.md` to provide full context

3. Implement using spec-kit automation or manual phases

**To use Claude models now**:

```bash
# Install Claude CLI (if not already installed)
# Download from: https://claude.ai/download

# Authenticate
claude

# Use in TUI
./codex-rs/target/dev-fast/code
/model claude-sonnet-4.5
> Your message here
```

---

## Contact & Support

**Issues**: Report to repository issue tracker
**Documentation**: See `docs/SPEC-KIT-952-cli-routing-multi-provider/`
**Questions**: Check local-memory (search tags: `spec:SPEC-KIT-952`)

---

**SPEC-KIT-952: Mission Accomplished** ✅

3 Claude models with full multi-turn CLI routing support delivered.
