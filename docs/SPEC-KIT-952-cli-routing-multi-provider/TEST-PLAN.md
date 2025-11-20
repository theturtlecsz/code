# CLI Routing Test Plan - Phase 2 (SPEC-KIT-952)

**Status**: Ready for manual testing
**Build**: ✅ Successful (2m 21s, binary at `./codex-rs/target/dev-fast/code`)
**CLIs**: ✅ Claude 2.0.47 + Gemini 0.16.0 available

---

## Quick Start

```bash
cd /home/thetu/code
./codex-rs/target/dev-fast/code
```

---

## Test Matrix

### Test 1: Single Message - Claude Sonnet 4.5

**Steps**:
```
/model claude-sonnet-4.5
> What's 2+2?
```

**Expected**:
- ✅ "Thinking..." indicator appears immediately
- ✅ Response starts streaming within 4-6s
- ✅ Text appears incrementally (Delta events)
- ✅ Final response: "4" or "2+2 equals 4"
- ✅ Token usage shown: ~10 input, ~5-10 output

**Result**: ✅ **PASSED** (2025-11-20)
- Response time: ~20s (higher than expected, but functional)
- Model mapping fix successful (404 error resolved)
- Streaming works correctly

---

### Test 2: Multi-turn Conversation - Claude Opus 4.1

**Steps**:
```
/model claude-opus-4.1
> My name is Alice
[Wait for response acknowledging name]
> What's my name?
```

**Expected**:
- ✅ Second response mentions "Alice"
- ✅ History from first message preserved
- ✅ Context manager formatting visible in logs

**Result**: ✅ **PASSED** (2025-11-20)
- First message: ~2-3s
- Second message: ~25s
- Context preserved correctly
- Routing bug fix confirmed (no 400 OAuth error)

---

### Test 3: Long Prompt - Gemini 3 Pro

**Steps**:
```
/model gemini-3-pro
> [Paste 1000+ word code file or long text]
```

**Expected**:
- ✅ Full code accepted (no truncation)
- ✅ Response addresses entire input
- ✅ Token estimation accurate (~250-300 tokens)
- ✅ Context compression doesn't trigger (under limit)

**Sample long prompt**:
```
Explain this code in detail:

[Paste contents of any large file, e.g., codex-rs/tui/src/chatwidget/mod.rs]
```

**Result**: ⏸️ PENDING

---

### Test 4: Streaming Visibility - Gemini 2.5 Flash

**Steps**:
```
/model gemini-2.5-flash
> Count from 1 to 10, one number per line
```

**Expected**:
- ✅ Numbers appear incrementally (1... 2... 3...)
- ✅ Visible streaming (not all at once)
- ✅ Fast response (<3s cold start for Gemini)

**Result**: ⏸️ PENDING

---

### Test 5: Token Usage Display - Claude Haiku 4.5

**Steps**:
```
/model claude-haiku-4.5
> Write a haiku about programming
```

**Expected**:
- ✅ Response appears
- ✅ Token counts displayed in UI
- ✅ Input tokens ~15-20
- ✅ Output tokens ~30-40

**Result**: ⏸️ PENDING

---

### Test 6: History Preservation - Gemini 2.5 Pro

**Steps**:
```
/model gemini-2.5-pro
> I love pizza
[Response acknowledges]
> What's your favorite topping?
[Response about toppings]
> What did I say I love?
```

**Expected**:
- ✅ Third response mentions "pizza"
- ✅ Full conversation context preserved
- ✅ All 3 exchanges maintain coherence

**Result**: ⏸️ PENDING

---

### Test 7: All 6 Models Smoke Test

**For each model**, run:
```
/model [model-name]
> Say hello and tell me your model name
```

**Models to test**:
1. `claude-opus-4.1`
2. `claude-sonnet-4.5`
3. `claude-haiku-4.5`
4. `gemini-3-pro`
5. `gemini-2.5-pro`
6. `gemini-2.5-flash`

**Expected (for each)**:
- ✅ Model responds successfully
- ✅ Model name mentioned in response
- ✅ No errors or crashes
- ✅ Streaming works

**Results**:
- claude-opus-4.1: ⏸️
- claude-sonnet-4.5: ⏸️
- claude-haiku-4.5: ⏸️
- gemini-3-pro: ⏸️
- gemini-2.5-pro: ⏸️
- gemini-2.5-flash: ⏸️

---

## Success Criteria

Phase 2 Step 2 is **COMPLETE** when:

- ✅ All 6 tests pass (1-6)
- ✅ All 6 models respond in smoke test (7)
- ✅ No crashes or errors
- ✅ Streaming is visible
- ✅ Token counts displayed
- ✅ History preserved across turns

---

## Debugging Tips

**If model doesn't respond**:
1. Check logs: Look for error messages in terminal
2. Verify CLI: Run `claude --version` and `gemini --version`
3. Test CLI directly: `echo "test" | claude --print --output-format stream-json --model claude-sonnet-4.5`

**If streaming is slow**:
- Expected: 4-6s for Claude, 2-3s for Gemini (cold start)
- This is normal due to CLI initialization
- "Thinking..." indicator should appear immediately

**If tokens not shown**:
- Check UI for token display area
- Verify events are being sent (check logs)

---

## After Testing

**If all tests pass**:
1. Mark results as ✅ in this file
2. Document any issues or observations
3. Move to Step 3 (Error scenario testing)

**If any test fails**:
1. Document failure details
2. Check logs for errors
3. Verify CLI streaming providers are correctly wired
4. Report to developer for debugging

---

**Next Step**: Step 3 - Error Scenario Testing (see PHASE-2-PROMPT.md)
