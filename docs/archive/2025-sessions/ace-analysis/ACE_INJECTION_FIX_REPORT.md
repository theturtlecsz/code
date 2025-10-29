# 🧠 ULTRATHINK: ACE Injection Fix - Comprehensive Analysis

## Current Problem

**Symptom**: ACE bullets not injected into prompts
**Root Cause**: Synchronous routing can't call async MCP

## Architecture Analysis

### Current Call Chain (ALL SYNCHRONOUS)

```rust
// User types: /speckit.implement SPEC-KIT-123

app.rs event loop (SYNC)
  ↓
AppEvent::DispatchCommand (SYNC event)
  ↓
spec_kit::try_dispatch_spec_kit_command(widget, "/speckit.implement...") (SYNC function)
  ↓
if spec_cmd.expand_prompt(&args).is_some() {  ← TRUE for /speckit.implement
    let formatted = format_subagent_command(...);  (SYNC - returns SubagentResolution)
    ↓
    let final_prompt = inject_ace_section(...)  (SYNC function)
        ↓
        NEEDS: ace_client::playbook_slice(...).await  (ASYNC!)
        ↓
        ❌ BLOCKED: Can't .await in sync function
        ❌ Can't use block_on (already on runtime)
    ↓
    widget.submit_prompt_with_display(final_prompt);  (SYNC)
}
```

**Problem**: Need to go async→sync→async, but we're already on tokio runtime.

---

## Solution Options Analysis

### Option A: Make Routing Async ⭐ (Best)

**Change**: Convert `try_dispatch_spec_kit_command` to async

```rust
// In routing.rs
pub async fn try_dispatch_spec_kit_command_async(
    widget: &mut ChatWidget,
    command_text: &str,
    app_event_tx: &AppEventSender,
) -> bool {
    // ... existing logic ...

    if spec_cmd.expand_prompt(&args).is_some() {
        let formatted = format_subagent_command(...);

        // CAN NOW AWAIT!
        let final_prompt = inject_ace_section_async(
            &widget.config.ace,
            config_name,
            repo_root,
            branch,
            formatted.prompt,
        ).await;  // ← This works!

        widget.submit_prompt_with_display(command_text, final_prompt);
    }

    true
}
```

**Changes Required**:
1. Make `try_dispatch_spec_kit_command` async
2. Make `inject_ace_section` async (remove sync wrapper)
3. Make app.rs caller spawn the async function

**Effort**: ~50 lines
**Complexity**: Low-Medium
**Impact**: Clean, proper async flow

---

### Option B: Two-Phase Approach (Channel-Based)

**Idea**: Fetch bullets in background, inject when ready

```rust
// Phase 1: Request bullets (non-blocking)
let (tx, rx) = oneshot::channel();
tokio::spawn(async move {
    let bullets = ace_client::playbook_slice(...).await;
    tx.send(bullets);
});

// Phase 2: Inject when available
// Problem: How to wait for channel in sync context?
// Still need async somewhere!
```

**Verdict**: Doesn't solve the core problem (still need async wait)

---

### Option C: Pre-cached Bullets

**Idea**: Fetch bullets in advance, store in widget state

```rust
// On widget creation or periodically:
tokio::spawn(async {
    let bullets = ace_client::playbook_slice(...).await;
    // Store in widget.ace_bullet_cache
});

// In routing (sync):
let bullets = widget.ace_bullet_cache.get(scope);  // Instant!
let final_prompt = inject_cached_bullets(bullets, prompt);
```

**Pros**:
- Sync-compatible
- Fast (no MCP call during routing)

**Cons**:
- Bullets might be stale
- Need cache invalidation
- More complex state management

**Effort**: ~150 lines
**Complexity**: Medium

---

### Option D: Spawn-and-Skip Approach (Current Workaround)

**Current**: Skip injection, log warning

**Pros**: No crash
**Cons**: ACE doesn't work

**Verdict**: Temporary only

---

## Recommended Solution: **Option A** (Async Routing)

### Why This is Best

1. **Cleanest**: Proper async flow, no hacks
2. **Simplest**: Just add `async` keywords
3. **Most flexible**: Enables future async operations
4. **Lowest effort**: ~50 lines of changes

### Implementation Plan

**Step 1**: Make routing async
```rust
// In routing.rs
pub async fn try_dispatch_spec_kit_command(
    widget: &mut ChatWidget,
    command_text: &str,
    app_event_tx: &AppEventSender,
) -> bool {
    // ... (mostly unchanged) ...

    if spec_cmd.expand_prompt(&args).is_some() {
        let formatted = format_subagent_command(...);

        // NOW ASYNC!
        let repo_root = get_repo_root(&widget.config.cwd);
        let branch = get_current_branch(&widget.config.cwd);

        let final_prompt = if ace_prompt_injector::should_use_ace(&widget.config.ace, config_name) {
            let bullets = ace_client::playbook_slice(
                repo_root.unwrap_or(".".to_string()),
                branch.unwrap_or("main".to_string()),
                scope.to_string(),
                widget.config.ace.slice_size,
                false,
            ).await;

            // Format and inject
            match bullets {
                AceResult::Ok(response) => {
                    let (section, ids) = format_bullets(&response.bullets);
                    inject_into_prompt(formatted.prompt, section)
                }
                _ => formatted.prompt
            }
        } else {
            formatted.prompt
        };

        widget.submit_prompt_with_display(command_text, final_prompt);
    }

    true
}
```

**Step 2**: Update caller in app.rs
```rust
// In app.rs, around line 1708
AppEvent::DispatchCommand(command, command_text) => {
    if let AppState::Chat { widget } = &mut self.app_state {
        // Spawn async routing
        let widget_ptr = widget as *mut ChatWidget;
        let cmd_text = command_text.clone();
        let tx = self.app_event_tx.clone();

        tokio::spawn(async move {
            unsafe {
                let widget = &mut *widget_ptr;
                spec_kit::try_dispatch_spec_kit_command(widget, &cmd_text, &tx).await;
            }
        });

        continue;
    }
    // ... rest of handling ...
}
```

**Step 3**: Remove sync wrappers
- Delete `inject_ace_section` sync version
- Make it properly async
- Remove warning logs

---

## Effort Estimation

**Code Changes**:
- routing.rs: +10 lines (add async/await)
- ace_prompt_injector.rs: -20 lines (remove sync wrapper), +30 (proper async)
- app.rs: +15 lines (spawn async routing)
- Total: ~35 net new lines

**Testing**:
- Verify /speckit.implement still works
- Verify bullets appear in prompts
- Check logs for injection success

**Time**: 1-2 hours

---

## Alternative: Simpler Cache Approach

If async routing seems risky, we could use **Option C** (cached bullets):

### Cached Approach Design

```rust
// In ChatWidget
pub struct ChatWidget {
    // ... existing fields ...
    ace_bullet_cache: Arc<Mutex<HashMap<String, Vec<PlaybookBullet>>>>,
}

// Background refresh task (started once)
tokio::spawn(async move {
    loop {
        for scope in ["global", "specify", "tasks", "implement", "test"] {
            let bullets = ace_client::playbook_slice(..., scope, 20).await;
            cache.insert(scope, bullets);
        }
        tokio::time::sleep(Duration::from_secs(60)).await;  // Refresh every minute
    }
});

// In routing (sync - instant!)
let bullets = widget.ace_bullet_cache.get(scope).unwrap_or_default();
let final_prompt = inject_cached_bullets(bullets, formatted.prompt);
```

**Pros**:
- Zero latency (cache is instant)
- No async in routing
- Simple to implement

**Cons**:
- Bullets up to 60s stale
- More state to manage
- Cache invalidation complexity

**Effort**: ~150 lines

---

## Recommendation

### Short Term (This Week)
**Use current state**: Constitution pinning works, injection disabled

**Test**: Verify Reflector/Curator work on actual runs

### Medium Term (Next Week)
**Implement Option A**: Async routing (proper solution)

**Effort**: 1-2 hours
**Risk**: Low (well-understood pattern)

### Alternative (If Async Too Risky)
**Implement Option C**: Cached bullets

**Effort**: 3-4 hours
**Risk**: Medium (more complex state)

---

## Critical Realization

**ACE can still provide value WITHOUT injection**:

1. ✅ **Constitution pinning** works (seeds playbook)
2. ✅ **Learning** works (Reflector/Curator after execution)
3. ✅ **Playbook grows** from reflections
4. ❌ **Injection** doesn't work (bullets not in prompts)

**Impact**: You get **intelligent learning** but not **prompt enhancement**.

**Still valuable?** Debatable. The learning is interesting but prompts don't benefit yet.

---

## My Honest Take

### The Situation

We've built a sophisticated system (3,195 lines) but hit a fundamental architectural issue: **sync/async impedance mismatch**.

### Options

**1. Fix it properly** (async routing, 1-2 hours)
   - Enables full ACE value
   - Clean solution
   - Worth doing if committed to ACE

**2. Use workaround** (cached bullets, 3-4 hours)
   - Works but inelegant
   - More complexity
   - Questionable value

**3. Simplify entirely** (50-line constitution injector)
   - No async issues
   - No MCP calls
   - Just read constitution.md and inject
   - Same prompt enhancement, 1/60th the code

### The Question

Given that we've already spent significant effort on ACE but it doesn't fully work yet:

**Should we**:
- A) Invest 1-2 more hours to fix async routing (get full value)?
- B) Accept current state (learning works, injection doesn't)?
- C) Simplify to 50-line injector (cut complexity)?

**I'd recommend**: Try **Option A** (async routing) for 1-2 hours. If it works, great. If it's harder than expected, consider Option C (simplify).

The code is already committed. The question is whether to finish it or simplify it.

What do you want to do?
