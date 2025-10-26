# 🧠 ULTRATHINK: ACE Injection - Architectural Rethink

## The Requirement

**Injection is required** - bullets MUST appear in prompts for ACE to provide value.

## What I Tried (Failed Approach)

**Async routing**:
- Make try_dispatch_spec_kit_command async
- Use .await for MCP calls
- **Problem**: Unsafe code, lifetime complexity, 3-4+ hours

**Why it failed**: Fighting against the event loop architecture

---

## Better Architectures (Fresh Perspective)

### Option 1: Split Routing (Deferred Submission) ⭐⭐⭐

**Key Insight**: Don't try to make routing async. Instead, split prompt submission into async task.

```rust
// In routing.rs (stays SYNC)
pub fn try_dispatch_spec_kit_command(...) -> bool {
    // ... same logic ...

    if spec_cmd.expand_prompt(&args).is_some() {
        let formatted = format_subagent_command(...);

        // Spawn async task for ACE injection + submission
        let config = widget.config.clone();
        let cwd = widget.config.cwd.clone();
        let cmd_text = command_text.to_string();

        // Get widget's submit channel or create submission mechanism
        let submit_tx = widget.get_submission_channel();

        tokio::spawn(async move {
            // Fetch bullets (async)
            let bullets = ace_client::playbook_slice(...).await;

            // Inject bullets
            let final_prompt = inject_bullets(formatted.prompt, bullets);

            // Submit via channel
            submit_tx.send(SubmitPrompt {
                display: cmd_text,
                prompt: final_prompt,
            });
        });

        return true; // Handled (async submission in progress)
    }

    false
}
```

**Changes needed**:
- Add submission channel to ChatWidget
- Spawn async task for injection + submit
- Routing stays sync (clean)

**Effort**: ~100 lines, 2-3 hours
**Complexity**: Medium (need submission channel)

---

### Option 2: Background Bullet Cache ⭐⭐

**Key Insight**: Pre-fetch bullets, inject synchronously from cache

```rust
// In ChatWidget initialization
pub struct ChatWidget {
    ace_bullet_cache: Arc<Mutex<AceBulletCache>>,
}

struct AceBulletCache {
    bullets: HashMap<String, Vec<PlaybookBullet>>,
    last_refresh: Instant,
}

// Background refresh task (started once at TUI startup)
tokio::spawn(async move {
    loop {
        for scope in ["global", "specify", "tasks", "implement", "test"] {
            match ace_client::playbook_slice(..., scope, 20).await {
                Ok(bullets) => cache.insert(scope, bullets),
                Err(_) => continue,
            }
        }
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
});

// In routing (SYNC - instant!)
let bullets = widget.ace_bullet_cache.lock().unwrap()
    .bullets.get(scope).cloned().unwrap_or_default();
let final_prompt = inject_cached_bullets(bullets, formatted.prompt);
```

**Pros**:
- Routing stays sync (no complexity)
- Injection is instant (no latency)
- Clean architecture

**Cons**:
- Bullets up to 30s stale
- Need cache management
- More widget state

**Effort**: ~150 lines, 2-3 hours
**Complexity**: Medium

---

### Option 3: Spawn + Wait Pattern ⭐

**Key Insight**: Use oneshot channel to wait for bullets without block_on

```rust
// In routing (stays SYNC)
pub fn try_dispatch_spec_kit_command(...) -> bool {
    if spec_cmd.expand_prompt(&args).is_some() {
        let formatted = format_subagent_command(...);

        // Create oneshot channel
        let (tx, mut rx) = tokio::sync::oneshot::channel();

        // Spawn bullet fetch
        tokio::spawn(async move {
            let bullets = ace_client::playbook_slice(...).await;
            let _ = tx.send(bullets);
        });

        // Poll channel with timeout (doesn't block runtime)
        let bullets = match rx.try_recv() {
            Ok(b) => b,
            Err(_) => {
                // Not ready yet, inject without bullets
                warn!("ACE bullets not ready, proceeding without");
                Vec::new()
            }
        };

        let final_prompt = inject_bullets(formatted.prompt, bullets);
        widget.submit_prompt_with_display(command_text, final_prompt);
    }
}
```

**Problem**: try_recv() is non-blocking but bullets might not be ready

**Better**: Use spawn + callback

---

### Option 4: Two-Stage Submission ⭐⭐⭐⭐ (SIMPLEST)

**Key Insight**: Fetch bullets in background, submit when ready

```rust
// In routing.rs (SYNC, simple)
pub fn try_dispatch_spec_kit_command(...) -> bool {
    if spec_cmd.expand_prompt(&args).is_some() {
        let formatted = format_subagent_command(...);

        // Show "preparing prompt..." message
        widget.history_push(PlainHistoryCell::new(
            vec![Line::from("Preparing prompt with ACE context...")],
            HistoryCellType::Notice,
        ));

        // Clone data for async task
        let config = widget.config.clone();
        let cwd = widget.config.cwd.clone();
        let cmd_text = command_text.to_string();
        let formatted_prompt = formatted.prompt.clone();

        // Get widget method to submit (via app_event_tx)
        let tx = app_event_tx.clone();

        tokio::spawn(async move {
            // Fetch bullets
            let bullets = ace_client::playbook_slice(...).await;

            // Inject
            let final_prompt = inject_bullets_from_result(formatted_prompt, bullets);

            // Submit via event
            tx.send(AppEvent::SubmitEnhancedPrompt {
                display: cmd_text.clone(),
                prompt: final_prompt,
            });
        });

        return true; // Handled (submission will happen async)
    }

    false
}
```

**Pros**:
- Routing stays sync (clean)
- No unsafe code
- No complex lifetimes
- User sees "preparing..." feedback

**Cons**:
- Slight delay (fetch + inject before submission)
- Need new AppEvent type

**Effort**: ~80 lines, 1-2 hours
**Complexity**: Low-Medium

---

## Recommended Solution: **Option 4** (Two-Stage)

### Why This is Best

1. **Simplest**: No async routing, no unsafe, no complex lifetimes
2. **User-friendly**: Shows "preparing prompt" message
3. **Clean**: Routing logic unchanged, async isolated
4. **Testable**: Easy to verify injection worked

### Implementation Steps

**Step 1**: Add AppEvent variant
```rust
// In app_event.rs
pub enum AppEvent {
    // ... existing ...
    SubmitEnhancedPrompt {
        display: String,
        prompt: String,
    },
}
```

**Step 2**: Update routing to spawn async injection
```rust
// Just spawn async task, return immediately
```

**Step 3**: Handle new event in app.rs
```rust
AppEvent::SubmitEnhancedPrompt { display, prompt } => {
    if let AppState::Chat { widget } = &mut self.app_state {
        widget.submit_prompt_with_display(display, prompt);
    }
}
```

**Total**: ~80 lines, clean architecture, 1-2 hours

---

## Comparison Matrix

| Approach | Complexity | Effort | Unsafe Code | Result |
|----------|-----------|---------|-------------|---------|
| Async routing | High | 3-4h | Yes | Full async |
| Background cache | Medium | 2-3h | No | Stale bullets |
| **Two-stage submit** | **Low-Medium** | **1-2h** | **No** | **Clean & simple** |

---

## Recommendation

**Implement Option 4** (Two-stage submission):

1. Add `SubmitEnhancedPrompt` event
2. Spawn async task in routing
3. Inject + submit when bullets ready
4. Show "preparing..." feedback to user

**Advantages**:
- Works with current architecture
- No fighting the borrow checker
- Clean separation of concerns
- User sees what's happening

**Want me to implement this approach?**
