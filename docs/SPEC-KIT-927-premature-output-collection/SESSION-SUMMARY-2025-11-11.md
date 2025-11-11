# SPEC-KIT-927 Session Summary - 2025-11-11

## Status: PARTIALLY FIXED - Core Issues Remain

### What Was Fixed ✅

1. **File Stability Check** (78cd94454) - WORKING
   - Monitors output file size for 2+ seconds stability
   - Requires >1KB minimum before reading
   - Prevents reading headers before agents finish

2. **Observable Agents Default** (ca5ebb970) - WORKING
   - Tmux mode enabled by default (.unwrap_or(true))
   - All agents get file stability protection
   - Set SPEC_KIT_OBSERVABLE_AGENTS=0 to disable

3. **Orchestration Fix** (bceb5e6de) - FIXED
   - GPT-5 validation spawns 1 agent directly (not via LLM)
   - Eliminated 18-agent chaos from run_agent tool
   - Main LLM no longer orchestrates quality gates

### What's Still Broken ❌

**CORE ISSUE: Agents still returning invalid JSON**

Pattern observed across 3 test runs:
- ✅ **gemini**: Works (extracts from markdown, validates)
- ❌ **claude**: Fails - "expected value at line 1 column 1"
- ❌ **code**: Fails - "expected value at line 7 column 13" or never completes

### Evidence

**Run 1 (19:01:35)**: gemini ✅, claude ✅, code ❌ (never completed)
**Run 2 (21:43:56)**: gemini ✅, claude ❌, code ❌  
**Run 3 (21:56:41)**: gemini ✅, claude ✅, code ❌ (never completed)

**Patterns**:
1. **code agent**: Consistently fails or hangs (completed_at = NULL)
2. **claude agent**: Intermittent - sometimes works, sometimes fails
3. **gemini agent**: Reliable - always works
4. **Extraction attempts**: Multiple failed (too aggressive, broke on edge cases)

### Attempted Fixes That Failed

1. **JSON Extraction v1** (e61aba589) - REVERTED
   - Too aggressive, tried to parse timestamps as JSON arrays
   - Broke with "expected , or ] at line 1 column 6"

2. **Enhanced Schema Detection** (82ba3ac55) - REVERTED  
   - Added TypeScript pattern detection
   - Didn't solve core issue (agents not producing valid output)

3. **Conservative Markdown Extraction** (c295ac241-94244f1ee) - CURRENT
   - Works for gemini (markdown wrapped)
   - Fails for claude (text before fence not extracted properly)
   - Fails for code (extracts prompt schema instead of response)

### Root Causes (NOT FULLY SOLVED)

**Issue 1: Code Agent Hangs** (UNRESOLVED)
- Never marks completed_at in database
- Tmux pane shows completion marker
- Output file either missing or contains invalid content
- 3/3 runs: code agent failed

**Issue 2: Output Extraction Too Complex** (PARTIAL)
- gemini: Returns ```json\n{...}\n``` → works
- claude: Returns "text\n\n```json\n{...}\n```" → extraction broken
- code: Returns "[timestamp] headers...User instructions:...{schema}...{response}" → extraction grabs wrong JSON

**Issue 3: Validation Too Strict?** (UNCLEAR)
- Maybe some agents legitimately return non-JSON explanations
- Current validation rejects anything not pure JSON
- But prompts explicitly request JSON output

### Current Code State

**Commits** (37 total, 6 for SPEC-927):
1. 78cd94454 - File stability (KEEP)
2. 2be6c84fc - Revert bad extraction (metadata)
3. ca5ebb970 - Observable default (KEEP)
4. c295ac241 - Markdown extraction v1
5. 94244f1ee - Improved extraction (headers)
6. 5d8975a86 - WIP timestamp extraction
7. bceb5e6de - Orchestration fix (KEEP)

**Binary**: b72283d2a0fa0af178102dfac5ba5f49b1b9d0878bfdaa62559696b88608f2d7

### Recommendation for Next Session

**STOP trying to fix extraction in validation layer.** Root causes:

1. **Agents are producing wrong output format**
   - They should return pure JSON
   - They're returning markdown/text/explanations
   - Fix: Update prompts to enforce JSON-only output
   - OR: Fix agent execution to strip non-JSON

2. **Code agent execution is fundamentally broken**
   - Hangs consistently (3/3 runs)
   - Output file issues
   - May need separate investigation

3. **Too many layers of extraction/validation**
   - Complexity is the enemy
   - Simplify: Either agents return clean JSON OR fix at source

### Next Steps

**Option A: Fix Prompts** (Recommended)
- Update agent prompts to explicitly forbid markdown wrapping
- Add "Return ONLY JSON, no markdown fences, no explanations"
- Test if this eliminates need for extraction

**Option B: Fix Code Agent** (High Priority)
- Investigate why code agent never completes
- Check stdout redirection, file permissions, execution path
- May be separate bug from extraction issues

**Option C: Simplify Validation** (Nuclear Option)
- Remove all extraction logic
- Accept any output that parses as JSON (even if wrapped)
- Use quality-gate broker extraction (already exists)

### Don't Repeat

❌ More extraction heuristics
❌ More validation layers  
❌ More diagnosis without fixing root cause
❌ Circular debugging of symptoms

✅ Fix agent output at source (prompts or execution)
✅ Investigate code agent hang separately
✅ Test ONE thing at a time
✅ Fresh session, fresh approach
