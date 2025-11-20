# Multi-Provider CLI Routing - Next Session (Post SPEC-952)

**Copy this entire prompt to start your next session**

---

## Session Context: SPEC-952 Complete, SPEC-952-B Ready

**Date**: 2025-11-20
**Repository**: theturtlecsz/code (fork of just-every/code)
**Location**: /home/thetu/code
**Branch**: main

**Current State**: SPEC-KIT-952 (Claude CLI routing) ✅ **COMPLETE**
**Next Task**: Create and implement SPEC-952-B (Gemini history management)

---

## What Was Accomplished (SPEC-952)

### ✅ Deliverables

**1. Production Claude CLI Routing** (3 models working):
- claude-opus-4.1 ✅ (multi-turn tested: 2-3s first msg, 25s second msg)
- claude-sonnet-4.5 ✅ (multi-turn tested: ~20s response)
- claude-haiku-4.5 ✅ (untested but same implementation)

**2. Core Infrastructure** (~1,900 LOC):
```
codex-rs/core/src/cli_executor/
├── mod.rs           - CliExecutor trait
├── claude.rs        - ClaudeCliExecutor (spawns CLI, manages stdin/stdout)
├── gemini.rs        - GeminiCliExecutor (single-turn only)
├── context.rs       - CliContextManager (history formatting)
├── stream.rs        - Stream parsers (parse_claude_stream, parse_gemini_stream)
└── types.rs         - Shared types
```

**3. TUI Integration**:
- `codex-rs/tui/src/providers/claude_streaming.rs` - ClaudeStreamingProvider
- `codex-rs/tui/src/providers/gemini_streaming.rs` - GeminiStreamingProvider (limited)
- `codex-rs/tui/src/model_router.rs` - execute_with_cli_streaming() (lines 410-455)
- `codex-rs/tui/src/chatwidget/mod.rs` - Routing logic (lines 5659-5732)

**4. Key Features**:
- ✅ Streaming responses with real-time deltas
- ✅ Multi-turn conversation support (Claude)
- ✅ Model name mapping (presets → API names)
- ✅ Queue routing fix (prevents OAuth fallback)
- ✅ Error handling (CLI not found, auth, timeout)

**5. Documentation**:
- `docs/SPEC-KIT-952-cli-routing-multi-provider/README.md` - Complete summary
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-1-COMPLETE.md` - Phase 1 details
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-2-PROMPT.md` - Phase 2 details
- `docs/SPEC-KIT-952-cli-routing-multi-provider/TEST-PLAN.md` - Test results
- `docs/SPEC-KIT-952-cli-routing-multi-provider/SPEC-952-B-PROMPT.md` - Gemini implementation guide
- `docs/ENHANCEMENTS.md` - Tracked model indicator UX request
- Updated `SPEC.md` line 143 - Marked SPEC-952 complete
- Updated `CLAUDE.md` lines 25-60 - Documented Claude-only support

---

## Critical Discoveries

### 1. Gemini CLI is Stateless (Not a Bug!)

**Finding**: Gemini CLI headless mode (`gemini -p`) is stateless by design
- ✅ Interactive mode (`gemini`): Full multi-turn, `/chat save/resume`, stateful
- ❌ Headless mode: Single-turn only, no `--session-id` or `--resume` flags

**Impact**: Cannot use same approach as Claude (which has native session management)

**Evidence**:
- Single-turn works: gemini-2.5-flash "What's 2+2?" → "4" in 3s ✅
- Multi-turn fails: Second message with history → 120s timeout ❌
- Manual test: Formatted history treated as single 10k-token message

**Solution**: SPEC-952-B - Client-side history management layer

### 2. Model Name Mapping Required

**Claude**:
- claude-sonnet-4.5 → claude-sonnet-4-5-20250929
- claude-opus-4.1 → claude-opus-4-1-20250805
- claude-haiku-4.5 → claude-haiku-4-5-20251001

**Gemini**:
- gemini-3-pro → gemini-3-pro-preview (**NOT** gemini-3.0-pro)
- gemini-2.5-pro → gemini-2.5-pro (unchanged)
- gemini-2.5-flash → gemini-2.5-flash (unchanged)

**Pattern**: CLIs forward model names directly to APIs - must map before invocation.

### 3. Queue Routing Bug

**Issue**: Messages typed while task running bypassed CLI routing → ChatGPT OAuth
**Symptom**: "model not supported with ChatGPT account" error
**Fix**: Added CLI model check in queue handler (chatwidget/mod.rs:5093-5124)
**Pattern**: ALL code paths (queues, retries, fallbacks) must respect routing decisions

---

## Local-Memory Context

**Stored learnings** (importance ≥8):
1. **Claude CLI routing complete** (ID: 5c9a98fd)
   - Architecture, implementation, files, patterns
2. **Gemini stateless discovery** (ID: 4f78887b)
   - Root cause, impact, solution approach
3. **Queue routing bug fix** (ID: a117fe2a)
   - Problem, symptom, fix, testing
4. **Model name mapping** (ID: 0de72546)
   - Requirements, implementation, validation

**Query for context**:
```
Use mcp__local-memory__search:
  query: "SPEC-952 CLI routing Gemini history"
  tags: ["spec:SPEC-KIT-952"]
  search_type: "semantic"
  limit: 10
```

**Query all SPEC-952 work**:
```
Use mcp__local-memory__search:
  tags: ["spec:SPEC-KIT-952"]
  search_type: "tags"
  limit: 20
```

---

## Current Codebase State

### Working Features

**Multi-Provider Model Support**:
| Provider | Models | Auth Method | Status |
|----------|--------|-------------|--------|
| ChatGPT | gpt-5, gpt-5.1-*, gpt-5-codex | Native OAuth | ✅ Working |
| Claude | claude-opus-4.1, claude-sonnet-4.5, claude-haiku-4.5 | CLI routing | ✅ Working |
| Gemini | gemini-3-pro, gemini-2.5-*, gemini-2.0-flash | Single-turn only | ⚠️ Limited |

**Binary Location**: `./codex-rs/target/dev-fast/code`
**Build Command**: `~/code/build-fast.sh`

### Known Limitations

1. **Gemini multi-turn not supported** (SPEC-952-B required)
2. **Claude response latency** 2-25s (higher than expected 4-6s)
3. **Model indicator UX** (tracked in `docs/ENHANCEMENTS.md`)
4. **Input parsing bug** `/model foo<enter>text` treats text as model name (pre-existing)

### Architecture

**Routing flow**:
```
User message → ChatWidget (chatwidget/mod.rs:5660)
              ↓
      supports_native_streaming()? (Claude/Gemini = true)
              ↓
      execute_with_cli_streaming() (model_router.rs:410)
              ↓
      Match provider:
        - Claude → ClaudeStreamingProvider
        - Gemini → GeminiStreamingProvider (limited)
        - ChatGPT → Error (use OAuth)
              ↓
      Provider → Executor → CLI process → Stream parser
              ↓
      Delta events → TUI (real-time display)
```

---

## Next Action: Create SPEC-952-B

### Step 1: Create the SPEC

Run this command in TUI:
```bash
/speckit.new Gemini Multi-Turn History Management Layer (SPEC-952-B)
```

### Step 2: Provide Full Context

**Paste this summary** when prompted or in follow-up:

```
# SPEC-952-B: Gemini Multi-Turn History Management Layer

Continuation of SPEC-KIT-952 (Claude CLI routing ✅ complete).

## Problem Statement
Gemini CLI headless mode is stateless (no native multi-turn like Claude). Current approach sends formatted history as text, causing 120s timeouts.

## Root Cause
- Claude CLI: Session-aware, has --continue/--resume flags
- Gemini CLI: Stateless, treats each call as independent
- Formatted history (--- Previous Conversation ---) treated as single huge message
- Result: Slow processing → timeout

## Proposed Solution
Implement client-side history management:

1. GeminiHistoryManager
   - Maintains Vec<Message> conversation state
   - Builds compact prompts (<8-40k tokens)
   - Automatic compression/summarization when needed

2. Compact Prompt Format
   - Summary paragraph (if conversation long)
   - Recent messages verbatim (last 5-10)
   - Current user message
   - No verbose headers or delimiters

3. Compression Strategy
   - No compression: <5k tokens total
   - Window compression: 5-15k tokens (summarize middle)
   - Heavy compression: >15k tokens (summarize all except last 3-5)
   - Use gemini-2.5-flash for summarization (cheap, fast)

## Success Criteria
- All 3 Gemini models support multi-turn
- No timeouts under normal use (<20 message conversations)
- Performance <10s response times with history
- Automatic compression transparent to user

## Effort Estimate
6-10 hours (4 phases + testing)

## Reference
Complete guide: docs/SPEC-KIT-952-cli-routing-multi-provider/SPEC-952-B-PROMPT.md
Research: docs/SPEC-KIT-952-cli-routing-multi-provider/gemini-cli-multi-turn-research.md

## Reuse from SPEC-952
- ✅ GeminiCliExecutor (working for single-turn)
- ✅ Stream parser (parse_gemini_stream working)
- ✅ Model name mapping (gemini-3-pro → gemini-3-pro-preview)
- ✅ Router integration (execute_with_cli_streaming)
- 🔄 GeminiStreamingProvider: Update to use history manager

## Local-Memory
Query tags: ["spec:SPEC-KIT-952"]
IDs: 5c9a98fd (Claude impl), 4f78887b (Gemini discovery), a117fe2a (queue fix), 0de72546 (model mapping)
```

### Step 3: Review and Plan

After SPEC creation:
```bash
# Review generated PRD
cat docs/SPEC-KIT-952-B-gemini-history-management/spec.md

# Generate detailed requirements
/speckit.specify SPEC-KIT-952-B

# Create work breakdown
/speckit.plan SPEC-KIT-952-B

# OR run full automation
/speckit.auto SPEC-KIT-952-B
```

---

## Complete File Reference

### Implementation Documentation

**Phase 1** (Infrastructure):
- `docs/SPEC-KIT-952-cli-routing-multi-provider/discovery.md` - CLI validation (Phase 0)
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-1-COMPLETE.md` - Core executors + providers

**Phase 2** (Integration):
- `docs/SPEC-KIT-952-cli-routing-multi-provider/PHASE-2-PROMPT.md` - Router integration guide
- `docs/SPEC-KIT-952-cli-routing-multi-provider/TEST-PLAN.md` - Test results

**Gemini Research**:
- `docs/SPEC-KIT-952-cli-routing-multi-provider/gemini-cli-multi-turn-research.md` - Architecture analysis
- `docs/SPEC-KIT-952-cli-routing-multi-provider/SPEC-952-B-PROMPT.md` - Complete implementation guide

**Summary**:
- `docs/SPEC-KIT-952-cli-routing-multi-provider/README.md` - Complete overview

### Code Files Created/Modified

**Core layer** (codex-rs/core/src/):
```
cli_executor/
├── mod.rs               - CliExecutor trait, exports
├── claude.rs            - ClaudeCliExecutor (293 LOC)
├── gemini.rs            - GeminiCliExecutor (267 LOC, single-turn only)
├── context.rs           - CliContextManager (175 LOC, history formatting)
├── stream.rs            - parse_claude_stream(), parse_gemini_stream() (169 LOC)
└── types.rs             - Conversation, Message, StreamEvent, CliError (98 LOC)

Total: ~1,002 LOC
```

**TUI layer** (codex-rs/tui/src/):
```
providers/
├── claude_streaming.rs  - ClaudeStreamingProvider (250 LOC)
└── gemini_streaming.rs  - GeminiStreamingProvider (240 LOC, needs history layer)

model_router.rs          - execute_with_cli_streaming() (lines 410-455)
                         - Deprecated execute_with_native_streaming() (lines 221-408)

chatwidget/mod.rs        - CLI routing logic (lines 5659-5732)
                         - Queue routing fix (lines 5093-5124)

Total: ~740 LOC
```

**Tests**:
```
codex-rs/core/tests/
└── cli_executor_tests.rs - 12/12 passing (Claude health checks)

Status: ✅ All compilation clean, 12 tests passing
```

### Bugs Fixed

**1. Model name mapping** (404 errors):
- File: `claude_streaming.rs:185-199`, `gemini_streaming.rs:185-202`
- Added `map_model_name()` functions
- Maps presets to API names

**2. Empty prompt fallthrough** (OAuth bypass):
- File: `chatwidget/mod.rs:5671-5675`
- Added early return if `prompt_text` empty
- Prevents silent routing failures

**3. Queue routing** (OAuth fallthrough):
- File: `chatwidget/mod.rs:5093-5124`
- Skip codex-core queue for CLI models
- Prevents queued messages from using wrong provider

**4. Gemini stream parser** (no output):
- File: `cli_executor/stream.rs:126-151`
- Fixed to parse `{"type":"message","content":"..."}` format
- Added stats parsing from `{"type":"result","stats":{...}}`

---

## Why Gemini Needs SPEC-952-B

### Technical Analysis

**Gemini CLI Architecture**:
- **Interactive mode** (`gemini`): Stateful, multi-turn, `/chat` commands
- **Headless mode** (`gemini -p`): Stateless, single-turn, no session management

**Claude CLI Architecture**:
- **Both modes**: Session-aware, `--continue`, `--resume` flags
- **History handling**: CLI manages conversation state internally

**Current Implementation (SPEC-952)**:
- Uses `CliContextManager::format_history()` for both Claude and Gemini
- Format: `--- Previous Conversation ---` with full transcript
- Works for Claude ✅ (CLI understands format)
- Fails for Gemini ❌ (treated as one huge message → timeout)

**Evidence**:
```bash
# Gemini single-turn (no history)
echo "What's 2+2?" | gemini --model gemini-2.5-flash --output-format stream-json
# Result: ✅ "4" in 3 seconds

# Gemini with formatted history
cat > /tmp/test.txt << EOF
--- Previous Conversation ---
USER: What's 2+2?
ASSISTANT: 4
--- End Previous Conversation ---
USER: What did I ask?
EOF
cat /tmp/test.txt | gemini --model gemini-2.5-flash --output-format stream-json
# Result: ❌ Treats entire thing as one message, 10k+ tokens, slow/timeout
```

**Conclusion**: Gemini needs **compact prompt format** with **summarization**, not verbose history format.

---

## SPEC-952-B: Gemini History Management Layer

### Architecture Overview

**Goal**: Enable multi-turn Gemini conversations via client-side history management.

**Components**:

1. **GeminiHistoryManager** (new module)
   - Owns conversation state (`Vec<Message>`)
   - Builds compact prompts with history
   - Automatic compression when >8k tokens
   - Token estimation and budgeting

2. **Compact Prompt Format** (replaces verbose format)
   ```
   You are an AI coding assistant.

   Summary: <1-2 paragraph summary of earlier context>

   Recent messages:
   User: <recent message 1>
   Assistant: <recent response 1>
   ...

   Current message: <new user input>
   ```

3. **Compression Strategy**
   - Level 1 (0-5k tokens): No compression, send all messages
   - Level 2 (5-15k tokens): Summarize older messages, keep recent 8 verbatim
   - Level 3 (>15k tokens): Heavy summarization, keep only last 3-5 exchanges

4. **Summarization Service** (async, background)
   - Use gemini-2.5-flash for cost efficiency
   - Cache summaries to avoid regenerating
   - Fallback: Truncate oldest messages if summarization fails

### Implementation Phases

**Phase 1**: History manager core (2-3h)
- Create `gemini_history.rs`
- Implement prompt building
- Token estimation
- Window-based compression
- Unit tests (8+ tests)

**Phase 2**: Provider integration (1-2h)
- Update `GeminiStreamingProvider`
- Use history manager instead of formatted text
- Message persistence
- Error handling

**Phase 3**: Summarization (2-3h)
- Async summarization service
- Summary caching
- Integration with history manager
- Fallback handling

**Phase 4**: Testing (1-2h)
- Integration tests (5+ tests)
- Manual testing all 3 models
- Performance validation

**Phase 5**: Documentation (30min)
- Update CLAUDE.md
- Update SPEC.md
- Create README

**Total**: 6-10 hours

### Success Criteria

- ✅ All 3 Gemini models work with multi-turn
- ✅ No timeouts (<10s response times with history)
- ✅ Automatic compression transparent
- ✅ Context preserved correctly
- ✅ Tests pass (15+ total)
- ✅ Documentation complete

---

## Technical Reference

### Existing Infrastructure (Reuse)

**Working components**:
- ✅ `GeminiCliExecutor::execute()` - CLI spawning and streaming
- ✅ `parse_gemini_stream()` - Stream event parsing
- ✅ `GeminiStreamingProvider::map_model_name()` - Model mapping
- ✅ `execute_with_cli_streaming()` - Router integration

**Components to update**:
- 🔄 `GeminiStreamingProvider::convert_messages()` - Use history manager
- 🔄 Prompt building - Replace `CliContextManager::format_history()` with compact format

**Components to create**:
- 🆕 `GeminiHistoryManager` - Conversation state management
- 🆕 Compact prompt formatter - Token-efficient history
- 🆕 Summarization service - Async compression

### Code Patterns to Follow

**From ClaudeStreamingProvider** (`claude_streaming.rs`):
- Provider structure (new(), execute_streaming())
- Message conversion (context_manager::Message → cli_executor::Message)
- Model name mapping (map_model_name())
- Error handling (map_cli_error())
- Event streaming (Delta, Metadata, Done)

**From CliContextManager** (`cli_executor/context.rs`):
- Token estimation (estimate_tokens())
- Context limits (get_context_limit())
- Compression logic (compress_if_needed())

**Adapt for Gemini**:
- Replace verbose format with compact format
- Add summarization layer
- Tune for Gemini-specific performance characteristics

---

## Performance Targets

### Response Times (with history)

| Scenario | Target | Claude (actual) | Gemini (target) |
|----------|--------|-----------------|-----------------|
| First message (no history) | <5s | 2-20s | <5s |
| Follow-up (3 messages) | <8s | 25s | <8s |
| Follow-up (10 messages) | <10s | untested | <10s |
| Follow-up (20+ messages, compressed) | <12s | untested | <12s |

### Token Budgets

| Level | Total Tokens | Strategy | Performance |
|-------|--------------|----------|-------------|
| Small | 0-5k | No compression | <5s |
| Medium | 5-15k | Window compression | <8s |
| Large | 15-40k | Heavy compression | <12s |
| Extreme | >40k | Aggressive truncation | <15s or warn |

---

## Commands for Next Session

### Query Context
```bash
# Get SPEC-952 work history
Use mcp__local-memory__search with tags: ["spec:SPEC-KIT-952"]

# Get Gemini-specific insights
Use mcp__local-memory__search with query: "Gemini stateless history"
```

### Create SPEC-952-B
```bash
/speckit.new Gemini Multi-Turn History Management Layer (SPEC-952-B continuation)
```

**Then provide context** from "Step 2" above (problem statement, solution, requirements).

### Build and Test
```bash
# Build TUI
~/code/build-fast.sh

# Run TUI
./codex-rs/target/dev-fast/code

# Test Claude (verify still working)
/model claude-sonnet-4.5
> test

# Test Gemini single-turn (verify baseline)
/model gemini-2.5-flash
> test
```

---

## Git Status

**Modified files** (not committed):
```
M  CLAUDE.md                                  (Updated with Claude-only support)
M  SPEC.md                                    (Marked SPEC-952 complete)
M  codex-rs/tui/src/chatwidget/mod.rs        (CLI routing + queue fix)
?? codex-rs/core/src/cli_executor/           (New: 6 files, ~1,002 LOC)
?? codex-rs/tui/src/model_router.rs          (New: 493 LOC)
?? codex-rs/tui/src/providers/*_streaming.rs (New: 2 files, ~490 LOC)
?? docs/SPEC-KIT-952-cli-routing-multi-provider/ (7 docs, ~8,500 words)
?? docs/ENHANCEMENTS.md                      (Model indicator request)
```

**Recommendation**: Commit SPEC-952 work before starting SPEC-952-B
```bash
git add -A
git commit -m "feat(cli-routing): Complete SPEC-952 Claude CLI routing with streaming

Implement production CLI routing for 3 Claude models with multi-turn support.

Delivered:
- Core CLI executors with streaming (claude.rs, gemini.rs, stream parsers)
- TUI streaming providers (ClaudeStreamingProvider, GeminiStreamingProvider)
- Router integration (execute_with_cli_streaming)
- Model name mapping (presets → API names)
- Queue routing fix (prevents OAuth fallback)
- Comprehensive documentation (7 docs)

Models working:
- claude-opus-4.1 (multi-turn: 2-25s)
- claude-sonnet-4.5 (multi-turn: ~20s)
- claude-haiku-4.5 (untested, same impl)

Gemini limitation:
- Single-turn works (gemini-2.5-flash: 3s)
- Multi-turn times out (CLI is stateless)
- Requires history management layer (SPEC-952-B)

Files:
- Core: codex-rs/core/src/cli_executor/ (6 files, ~1,002 LOC)
- TUI: providers/*_streaming.rs, model_router.rs, chatwidget/mod.rs (~1,100 LOC)
- Docs: docs/SPEC-KIT-952-cli-routing-multi-provider/ (7 files)
- Tests: 12/12 passing

SPEC-952: ✅ Complete (Claude-only)
Next: SPEC-952-B (Gemini history layer)

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Quick Start for Next Session

**Paste this into new session**:

```
Continue SPEC-952 work. SPEC-952 (Claude CLI routing) ✅ complete.

Current task: Create SPEC-952-B (Gemini history management layer).

Context:
- SPEC-952 delivered: 3 Claude models with CLI routing + multi-turn
- Limitation found: Gemini CLI headless is stateless (no native multi-turn)
- Solution: Client-side history management with compression
- Full guide: docs/SPEC-KIT-952-cli-routing-multi-provider/SPEC-952-B-PROMPT.md

Local-memory:
Query tags: ["spec:SPEC-KIT-952"]
IDs: 5c9a98fd, 4f78887b, a117fe2a, 0de72546

Next action:
/speckit.new Gemini Multi-Turn History Management Layer (SPEC-952-B)

[Paste full context from SPEC-952-B-PROMPT.md]
```

---

## 🎯 YOUR NEXT ACTION

**Immediate** (this session, if time):
```bash
# Commit SPEC-952 work
git add -A
git commit -m "feat(cli-routing): Complete SPEC-952 Claude CLI routing"
# (Use full commit message from "Git Status" section above)
```

**Next session** (when ready for SPEC-952-B):
1. ✅ Paste "Quick Start" prompt above
2. ✅ Query local-memory for context
3. ✅ Run `/speckit.new` to create SPEC-952-B
4. ✅ Provide full context from SPEC-952-B-PROMPT.md
5. ✅ Choose automation level (/speckit.auto or manual phases)

---

**Status**: SPEC-952 ✅ Complete | SPEC-952-B 📋 Ready to Create | Commit 📝 Recommended
