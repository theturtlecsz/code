# P71 Session Handoff

**Previous**: P70 | **Commit**: `731aaa3f4` | **Status**: Ready to Continue
**Focus**: `/speckit.clarify` + `/speckit.new` enhancements

---

## Session Summary (P70)

### Completed This Session
1. **SPEC-KIT-099 Documentation** - Research-to-Code Context Bridge spec committed
   - Complete architecture at `docs/SPEC-KIT-099-context-bridge/spec.md`
   - Hardened implementation guide (Section 16) with MCP constraints
   - Implementation deferred to future session

2. **Analysis Documents Created**
   - `docs/LOCAL-MEMORY-ENVIRONMENT.md` - Complete local-memory installation capture
   - `docs/SPECKIT-AUTO-PIPELINE-ANALYSIS.md` - Pipeline prompt flow (19-28 prompts)

3. **Documentation Committed** (`731aaa3f4`)
   - SPEC-KIT-099 spec, handoffs P68-P70, analysis docs

### Unstaged Work (Ready to Continue)
The `/speckit.clarify` implementation is **in progress** with these files modified:

| File | Status | Description |
|------|--------|-------------|
| `bottom_pane/clarify_modal.rs` | **NEW** | Modal UI for clarification questions |
| `spec_kit/clarify_handler.rs` | **NEW** | Event handlers for modal completion |
| `spec_kit/clarify_native.rs` | Modified | Pattern detection + marker resolution |
| `spec_kit/commands/quality.rs` | Modified | Command wiring |
| `spec_kit/mod.rs` | Modified | Module exports |
| `app.rs` | Modified | AppEvent handling |
| `app_event.rs` | Modified | New event types |
| `bottom_pane/mod.rs` | Modified | Modal integration |
| `chatwidget/mod.rs` | Modified | Widget state |

---

## P71 Goals

### Priority 1: Complete `/speckit.clarify` (SPEC-KIT-971)

**Current State**: Modal + handlers exist but not fully wired

**Remaining Tasks**:

1. **Wire the command entry point** (~15 min)
   - Add `SpecKitClarifyCommand` to `commands/` if not exists
   - Register in `command_registry.rs`
   - Connect to `clarify_native::scan_for_markers()`

2. **Test marker scanning** (~10 min)
   ```bash
   # Create test spec with markers
   echo "[NEEDS CLARIFICATION: What is X?]" >> /tmp/test-spec.md
   # Run /speckit.clarify on it
   ```

3. **Verify modal flow** (~10 min)
   - Modal displays questions correctly
   - Answers are collected
   - Files are updated with resolutions

4. **Handle edge cases** (~15 min)
   - No markers found (show "nothing to clarify")
   - Invalid spec ID
   - File permission errors

### Priority 2: `/speckit.new` Question Customization

**Goal**: Project-type-aware questions for PRD builder

**Design**:
```rust
enum ProjectType {
    Rust,       // Cargo.toml
    Python,     // pyproject.toml or setup.py
    TypeScript, // package.json with typescript
    Go,         // go.mod
    Generic,    // Fallback
}

fn detect_project_type(cwd: &Path) -> ProjectType {
    if cwd.join("Cargo.toml").exists() { return Rust; }
    if cwd.join("pyproject.toml").exists() { return Python; }
    // ...
}
```

**Tasks**:

1. **Add project detection** (~15 min)
   - Create `project_detector.rs` in spec_kit
   - Implement `detect_project_type()`

2. **Create question sets** (~30 min)
   - Rust: "Crate or binary?", "Workspace member?", "Target MSRV?"
   - Python: "Library or CLI?", "Async required?", "Min Python version?"
   - TypeScript: "Frontend or backend?", "Framework (React/Vue/etc)?"
   - Generic: Current default questions

3. **Wire into PRD builder** (~15 min)
   - Modify `prd_builder_modal.rs` to accept dynamic questions
   - Call `detect_project_type()` before showing modal

### Priority 3 (If Time): Ferris Benchmark

**Reference**: `/home/thetu/benchmark/reference/ferris-says`

Compare spec-kit output quality against reference implementation.

---

## Implementation Details

### Clarify Command Flow

```
/speckit.clarify SPEC-KIT-###
    │
    ▼
┌──────────────────────────────────────┐
│ clarify_native::scan_for_markers()   │
│ - Reads spec.md, PRD.md              │
│ - Pattern: [NEEDS CLARIFICATION: X]  │
│ - Returns Vec<ClarificationMarker>   │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│ If markers found:                    │
│   -> Show ClarifyModal               │
│ If no markers:                       │
│   -> "No clarifications needed"      │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│ ClarifyModal (freeform input)        │
│ - Shows question + context           │
│ - User types answer                  │
│ - Enter to submit, Esc to cancel     │
└──────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────┐
│ on_clarify_submitted()               │
│ - Calls resolve_markers()            │
│ - Updates files in-place             │
│ - Shows summary                      │
└──────────────────────────────────────┘
```

### Marker Pattern

```
[NEEDS CLARIFICATION: Should we use async or sync?]
                      ↑
                      This becomes the question text
```

Resolution replaces entire marker with the answer.

---

## Files Reference

### Clarify Implementation

| File | Purpose |
|------|---------|
| `bottom_pane/clarify_modal.rs` | Modal UI component |
| `spec_kit/clarify_handler.rs` | Event handlers |
| `spec_kit/clarify_native.rs` | Scanning + resolution logic |
| `spec_kit/commands/quality.rs` | Command registration |

### New Command (If Needed)

```rust
// commands/clarify.rs
pub struct SpecKitClarifyCommand;

impl SpecKitCommand for SpecKitClarifyCommand {
    fn name(&self) -> &'static str { "speckit.clarify" }
    fn aliases(&self) -> &[&'static str] { &["clarify"] }
    fn description(&self) -> &'static str { "resolve ambiguities in spec" }

    fn execute(&self, widget: &mut ChatWidget, args: String) {
        // 1. Parse spec_id from args
        // 2. Call scan_for_markers(spec_id)
        // 3. If markers: show ClarifyModal
        // 4. If none: show "nothing to clarify"
    }
}
```

---

## Quick Start Commands

```bash
# Build and run
~/code/build-fast.sh run

# Check current changes
git status
git diff codex-rs/tui/src/chatwidget/spec_kit/clarify_native.rs

# Test clarify command (in TUI)
/speckit.clarify SPEC-KIT-099

# Run tests
cd codex-rs && cargo test -p codex-tui clarify
```

---

## Context Documents

| Document | Purpose |
|----------|---------|
| `docs/HANDOFF-P68.md` | Original clarify spec |
| `docs/HANDOFF-P70.md` | SPEC-099 implementation roadmap (deferred) |
| `docs/SPEC-KIT-099-context-bridge/spec.md` | Full context bridge spec |
| `docs/SPECKIT-AUTO-PIPELINE-ANALYSIS.md` | Pipeline prompt analysis |
| `docs/LOCAL-MEMORY-ENVIRONMENT.md` | Local memory setup reference |

---

## Continuation Prompt

```
I'm continuing the /speckit.clarify and /speckit.new work from P70.

Current state:
- Clarify modal + handlers exist (unstaged)
- Need to wire command entry point and test
- Question customization for /speckit.new is next

Priority order:
1. Complete /speckit.clarify wiring and test
2. Add project-type detection for /speckit.new questions
3. Ferris benchmark (if time)

Start by checking git status to see unstaged changes,
then wire the clarify command if not done.
```

---

## Open Questions (For Next Session)

1. Should `/speckit.clarify` auto-detect spec ID from current branch?
2. Should question customization be config-driven or code-driven?
3. Should we add `/speckit.clarify --all` to process all SPECs?

---

*Handoff created: 2025-11-30*
