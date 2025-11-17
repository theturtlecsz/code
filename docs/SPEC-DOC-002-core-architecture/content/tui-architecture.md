# TUI Architecture

Detailed architecture of the Terminal User Interface layer.

---

## Overview

The TUI layer uses **Ratatui** (v0.29.0, patched fork) with a hybrid **async/sync** architecture:

- **Sync Layer**: Ratatui event loop (blocking terminal operations)
- **Async Layer**: Tokio tasks (network I/O, subprocess management)
- **Bridge**: Channels (`UnboundedSender`, `AppEventSender`)

**File**: `codex-rs/tui/src/`

---

## Component Hierarchy

```
App (top-level state)
 ├── ChatWidget (conversation interface)
 │    ├── BottomPane (input + status)
 │    ├── HistoryCell[] (message history)
 │    ├── StreamController (token streaming)
 │    ├── InterruptManager (Ctrl+C handling)
 │    └── SpecKitState (fork features)
 ├── TerminalInfo (size, capabilities)
 └── AppEventReceiver (backend events)
```

---

## ChatWidget Structure

**Location**: `codex-rs/tui/src/chatwidget/mod.rs:373+`

```rust
pub(crate) struct ChatWidget<'a> {
    // === Backend Communication ===
    app_event_tx: AppEventSender,                   // Send events to App
    codex_op_tx: UnboundedSender<Op>,               // Submit operations to backend

    // === UI Components ===
    bottom_pane: BottomPane<'a>,                    // Input composer + status bar
    history_cells: Vec<Box<dyn HistoryCell>>,       // Conversation history

    // === State ===
    config: Config,                                 // Configuration
    auth_manager: Arc<AuthManager>,                 // Authentication state

    // === Spec-Kit (Fork) ===
    spec_auto_state: Option<SpecAutoState>,         // Pipeline orchestration
    cost_tracker: Arc<spec_kit::cost_tracker::CostTracker>, // Cost tracking

    // === Agent Tracking ===
    active_agents: Vec<AgentInfo>,                  // Running agents
    agent_runtime: HashMap<String, AgentRuntime>,   // Agent metadata

    // === Execution State ===
    exec: ExecState,                                // Command execution tracking
    terminal: TerminalState,                        // Tmux session state

    // === Rendering ===
    stream: StreamController,                       // Token streaming
    interrupts: InterruptManager,                   // Interrupt handling
    cached_cell_size: OnceCell<(u16, u16)>,        // Terminal dimensions
}
```

**Key Insights**:
- **912K LOC in mod.rs** (large monolithic file)
- **Friend module access**: `spec_kit` modules can access private fields
- **Hybrid ownership**: `Arc<T>` for shared state, direct ownership for UI

---

## Async/Sync Boundary Pattern

### The Problem

**Ratatui requires synchronous event loop**:
```rust
loop {
    terminal.draw(|f| ui(f, &app))?;  // Blocking draw
    let event = event::read()?;       // Blocking read
    app.handle_event(event);          // Sync handler
}
```

**Backend requires async I/O**:
```rust
async fn conversation_loop() {
    let response = model_provider.chat(request).await?; // Async network I/O
    while let Some(token) = response.next().await? {    // Async streaming
        // ...
    }
}
```

**Constraint**: Can't `.await` in sync event loop ❌

---

### The Solution: Channel Bridge

**Pattern**:
```
Sync UI Thread                   Async Backend Task
   ChatWidget                     ConversationManager
       ↓                                 ↓
   submit_prompt()            async conversation_loop()
       ↓                                 ↓
   codex_op_tx.send(op)  ──→   codex_op_rx.recv().await
       ↓                                 ↓
   app_event_rx.recv()   ←──   app_event_tx.send(event)
```

**Implementation**: `codex-rs/tui/src/chatwidget/agent.rs:16-62`

```rust
pub(crate) fn spawn_agent(
    config: Config,
    app_event_tx: AppEventSender,
    server: Arc<ConversationManager>,
) -> UnboundedSender<Op> {
    // Create channel for UI → Backend
    let (codex_op_tx, mut codex_op_rx) = unbounded_channel::<Op>();

    // Spawn async task
    tokio::spawn(async move {
        // Create conversation
        let NewConversation { conversation, .. } =
            server.new_conversation(config).await?;

        // Forward operations to conversation
        tokio::spawn(async move {
            while let Some(op) = codex_op_rx.recv().await {
                conversation.submit(op).await;
            }
        });

        // Forward events back to UI
        while let Ok(event) = conversation.next_event().await {
            app_event_tx.send(AppEvent::CodexEvent(event))?;
        }
    });

    // Return sync sender to UI
    codex_op_tx
}
```

**Key Points**:
- **`UnboundedSender<Op>`**: Sync side can send without `.await`
- **`app_event_tx.send()`**: Backend sends events to UI queue
- **Tokio runtime**: Spawned on separate thread pool
- **No blocking**: UI thread never blocks on network I/O

---

### Operation Types (`Op` enum)

```rust
pub enum Op {
    NewMessage(String),              // User prompt
    ToolResponse(ToolCallId, Result<String>), // Tool execution result
    RegenerateLastMessage,           // Retry last response
    Interrupt,                       // Cancel current operation
    // ... 10+ variants
}
```

**Flow**:
1. User types message → `ChatWidget.submit_prompt()`
2. Create `Op::NewMessage` → `codex_op_tx.send(op)`
3. Backend receives → `conversation.submit(op).await`
4. Model responds → `app_event_tx.send(Event::Token)`
5. UI receives → `ChatWidget.handle_event()` → Render

---

## Event Loop

**Location**: `codex-rs/tui/src/app.rs`

```rust
impl App {
    pub fn run(mut self) -> Result<()> {
        loop {
            // Draw UI
            self.terminal.draw(|f| {
                self.chat_widget.render(f, f.size());
            })?;

            // Handle events (non-blocking with timeout)
            if event::poll(Duration::from_millis(16))? {
                match event::read()? {
                    Event::Key(key) => self.handle_key(key)?,
                    Event::Resize(w, h) => self.handle_resize(w, h)?,
                    Event::Mouse(mouse) => self.handle_mouse(mouse)?,
                    _ => {}
                }
            }

            // Process backend events
            while let Ok(app_event) = self.app_event_rx.try_recv() {
                self.handle_app_event(app_event)?;
            }

            // Check for exit
            if self.should_exit {
                break;
            }
        }
        Ok(())
    }
}
```

**Loop Phases**:
1. **Draw**: Render UI to terminal buffer
2. **Poll**: Check for terminal events (16ms timeout = ~60 FPS)
3. **Handle Terminal Events**: Keyboard, mouse, resize
4. **Process Backend Events**: Tokens, completions, errors
5. **Check Exit**: Break if requested

---

## Rendering System

### Immediate Mode Rendering

**Ratatui uses immediate-mode**:
- No retained UI tree
- Full re-render every frame
- Layout calculated on-the-fly

**Performance**: ~60 FPS for typical conversation UI

---

### Widget Composition

```rust
impl ChatWidget {
    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Split layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),       // History
                Constraint::Length(3),    // Composer
                Constraint::Length(1),    // Status bar
            ])
            .split(area);

        // Render history
        self.render_history(f, chunks[0]);

        // Render input composer
        self.bottom_pane.render(f, chunks[1]);

        // Render status bar
        self.render_status(f, chunks[2]);
    }
}
```

---

### HistoryCell Trait

**Location**: `codex-rs/tui/src/chatwidget/history/`

```rust
pub trait HistoryCell: Send {
    fn height(&self, width: u16) -> u16;           // Calculate cell height
    fn render(&self, frame: &mut Frame, area: Rect); // Render to area
}

// Implementations:
pub struct UserMessageCell { /* ... */ }      // User prompt
pub struct AssistantMessageCell { /* ... */ } // AI response
pub struct ToolExecutionCell { /* ... */ }    // Tool call/result
pub struct ExecCell { /* ... */ }             // Command execution
pub struct BackgroundEventCell { /* ... */ }  // System messages
```

**Dynamic Dispatch**: `Vec<Box<dyn HistoryCell>>` allows heterogeneous message types

---

## Spec-Kit Integration (Friend Module)

### Friend Module Pattern

**Declared in `chatwidget/mod.rs`**:
```rust
pub mod spec_kit;  // Friend module - can access private fields
```

**Benefits**:
- ✅ Spec-kit can read/write `ChatWidget` internals
- ✅ No public API pollution
- ✅ Clear encapsulation boundary
- ✅ Easy to test (spec_kit modules are independent)

---

### Context Trait Abstraction

**Location**: `codex-rs/tui/src/chatwidget/spec_kit/context.rs:1-140`

```rust
pub trait SpecKitContext {
    // History operations
    fn history_push(&mut self, cell: impl HistoryCell + 'static);
    fn push_error(&mut self, message: String);
    fn push_background(&mut self, message: String, placement: BackgroundPlacement);

    // UI operations
    fn request_redraw(&mut self);

    // Agent/operation submission
    fn submit_operation(&self, op: Op);
    fn submit_prompt(&mut self, display: String, prompt: String);

    // Configuration access
    fn working_directory(&self) -> &Path;
    fn agent_config(&self) -> &[AgentConfig];

    // Spec auto state
    fn spec_auto_state_mut(&mut self) -> &mut Option<SpecAutoState>;
    fn spec_auto_state(&self) -> &Option<SpecAutoState>;

    // Guardrail operations
    fn collect_guardrail_outcome(&self, spec_id: &str, stage: SpecStage) -> Result<GuardrailOutcome>;
    fn run_spec_consensus(&mut self, spec_id: &str, stage: SpecStage)
        -> Result<(Vec<Line<'static>>, bool)>;
}

impl SpecKitContext for ChatWidget {
    // Implementation delegates to ChatWidget methods
}
```

**Purpose**: Decouples spec-kit from ChatWidget implementation

**Testing**: Mock implementation in `context::test_mock`

---

## Streaming & Interrupts

### StreamController

**Location**: `codex-rs/tui/src/streaming/controller.rs`

```rust
pub struct StreamController {
    active_stream: Option<StreamState>,
}

impl StreamController {
    pub fn start_stream(&mut self, request_id: String) {
        self.active_stream = Some(StreamState {
            request_id,
            tokens: Vec::new(),
            started_at: Instant::now(),
        });
    }

    pub fn append_token(&mut self, token: String) {
        if let Some(stream) = &mut self.active_stream {
            stream.tokens.push(token);
        }
    }

    pub fn finish_stream(&mut self) -> Option<StreamState> {
        self.active_stream.take()
    }
}
```

**Flow**:
1. Backend sends `Event::StreamStart`
2. `StreamController.start_stream()`
3. Backend sends `Event::Token(tok)` repeatedly
4. `StreamController.append_token(tok)`
5. UI renders partial response on each frame
6. Backend sends `Event::StreamEnd`
7. `StreamController.finish_stream()`

---

### InterruptManager

**Location**: `codex-rs/tui/src/chatwidget/interrupts.rs`

```rust
pub struct InterruptManager {
    pending_interrupt: bool,
    last_interrupt_at: Option<Instant>,
}

impl InterruptManager {
    pub fn request_interrupt(&mut self) {
        self.pending_interrupt = true;
        self.last_interrupt_at = Some(Instant::now());
    }

    pub fn consume_interrupt(&mut self) -> bool {
        std::mem::replace(&mut self.pending_interrupt, false)
    }
}
```

**Usage**: Ctrl+C → `InterruptManager.request_interrupt()` → Send `Op::Interrupt` to backend → Backend cancels operation

---

## Input Handling

### BottomPane (Composer)

**Location**: `codex-rs/tui/src/chatwidget/bottom_pane.rs`

```rust
pub struct BottomPane<'a> {
    input: String,                      // Current input buffer
    cursor_position: usize,             // Cursor position
    history_index: Option<usize>,       // Command history navigation
    file_search: Option<FileSearch>,    // @ file search state
}

impl BottomPane {
    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        match key.code {
            KeyCode::Enter => InputAction::Submit(std::mem::take(&mut self.input)),
            KeyCode::Char('@') if self.input.is_empty() => {
                self.file_search = Some(FileSearch::new());
                InputAction::None
            },
            KeyCode::Char(c) => {
                self.input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                InputAction::None
            },
            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.input.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
                InputAction::None
            },
            // ... more key handlers
        }
    }
}
```

**Features**:
- Multi-line input (Shift+Enter)
- Cursor movement (arrows, Home, End)
- History navigation (Esc Esc)
- File search (@ trigger)
- Paste support (Ctrl+V with image detection)

---

## File Search (@-trigger)

**Location**: `codex-rs/file-search/src/lib.rs`

```rust
pub struct FileSearch {
    query: String,
    results: Vec<PathBuf>,
    selected_index: usize,
}

impl FileSearch {
    pub fn update_query(&mut self, query: String) {
        self.query = query;
        self.results = fuzzy_search(&query, max_results: 10);
        self.selected_index = 0;
    }
}

fn fuzzy_search(query: &str, max_results: usize) -> Vec<PathBuf> {
    // Use nucleo-matcher for fuzzy matching
    // Search workspace for files matching query
}
```

**UI Flow**:
1. User types `@` → Activate file search
2. User types `main` → Update query, show results
3. User presses Up/Down → Navigate results
4. User presses Tab/Enter → Insert file path, exit search
5. User presses Esc → Cancel search

---

## Performance Considerations

### Rendering Optimizations

**Lazy Height Calculation**:
```rust
pub fn height(&self, width: u16) -> u16 {
    // Calculate height only when width changes
    if self.cached_width == Some(width) {
        return self.cached_height;
    }
    let height = self.calculate_height(width);
    self.cached_width = Some(width);
    self.cached_height = height;
    height
}
```

**Viewport Culling**:
- Only render visible history cells
- Skip cells outside viewport
- Recalculate on scroll

---

### Event Processing

**Non-Blocking Event Poll**:
```rust
if event::poll(Duration::from_millis(16))? {
    // Process event
}
```

- 16ms = ~60 FPS target
- Non-blocking (returns immediately if no events)
- Allows backend event processing every frame

---

## Summary

**TUI Architecture Highlights**:

1. **Async/Sync Hybrid**: Channels bridge sync UI and async backend
2. **Immediate-Mode Rendering**: Full re-render every frame (~60 FPS)
3. **Dynamic Dispatch**: `Box<dyn HistoryCell>` for heterogeneous messages
4. **Friend Module Pattern**: Spec-kit access to ChatWidget internals
5. **Context Trait**: Decouples spec-kit from ChatWidget implementation
6. **Streaming**: Real-time token rendering
7. **Interrupts**: Ctrl+C gracefully cancels operations

**Next Steps**:
- [Core Execution](core-execution.md) - Agent orchestration
- [MCP Integration](mcp-integration.md) - Native client details
- [Database Layer](database-layer.md) - SQLite optimization

---

**File References**:
- ChatWidget: `codex-rs/tui/src/chatwidget/mod.rs:373+`
- Agent spawner: `codex-rs/tui/src/chatwidget/agent.rs:16-62`
- Event loop: `codex-rs/tui/src/app.rs`
- Context trait: `codex-rs/tui/src/chatwidget/spec_kit/context.rs:1-140`
- Streaming: `codex-rs/tui/src/streaming/controller.rs`
- Bottom pane: `codex-rs/tui/src/chatwidget/bottom_pane.rs`
