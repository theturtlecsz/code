# Next Session Continuation Prompt

Copy the prompt below to start the next session.

---

```
Load HANDOFF.md and NEXT_FOCUS_ROADMAP.md. Execute SPEC-KIT-920 (Automation MVP).

## Context

- PR7-PR9 complete (gate/review vocabulary migration)
- CI hardening in place (vocabulary drift canary + golden wire-format tests)
- SPEC-KIT-900 gold run created but **should not be used as smoke test**
  - It's a heavyweight integration benchmark (Stage0, NotebookLM, external deps)
  - Keep for quarterly/release testing only

## Critical Insight

Claude Code can implement changes but cannot drive `/speckit.auto` without manual TUI interaction. This blocks CI/CD and repeatable testing. **Automation is the true P0.**

## SPEC-KIT-920 Status

**Already implemented:**
- `--initial-command "<slash command>"` - works, dispatches after first redraw
- Located in `tui/src/cli.rs:109`, wired through `tui/src/app.rs`

**NOT implemented (this session's work):**
- `--exit-on-complete` - defined in CLI (line 113) but app never reads it
- Exit code based on success/failure

## Minimum Viable Scope

1. **Wire `exit_on_complete` through App**
   - Pass from CLI to App struct (already has `initial_command` pattern to follow)
   - Store as `exit_on_complete: bool` field

2. **Detect pipeline completion**
   - For `/speckit.auto`: detect `PipelineComplete` or `PipelineAborted` event
   - For other commands: detect when command handler returns/completes

3. **Trigger exit with appropriate code**
   - Success (0): pipeline completed with all stages passing
   - Failure (non-zero): pipeline aborted, gate failed, or error

4. **Test manually**
   ```bash
   cargo build -p codex-tui --release

   # Should exit 0 after /speckit.status completes
   ./target/release/codex-tui --initial-command "/speckit.status SPEC-KIT-900" --exit-on-complete
   echo "Exit code: $?"
   ```

## Files to Modify

| File | Change |
|------|--------|
| `tui/src/cli.rs` | Already has flag (line 113), no changes needed |
| `tui/src/lib.rs` | Pass `exit_on_complete` to App::new() |
| `tui/src/app.rs` | Add field, detect completion, call `self.should_quit = true` |
| `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` | Emit completion signal if not already |

## Anti-patterns to Avoid

- Don't try to fix SPEC-KIT-900 to be the smoke test
- Don't start SPEC-KIT-926 (progress visibility) before 920 is done
- Don't add `--json` output yet (nice to have, not MVP)

## After 920 is Complete

1. **Create lightweight smoke spec** (SPEC-KIT-SMOKE-001 or similar)
   - `/speckit.doctor` or `/speckit.health` - zero-LLM, fast
   - Validates config, templates, environment

2. **Then proceed to SPEC-KIT-926** (progress visibility)
   - Much easier to validate once headless runs exist

## Acceptance Criteria

- [ ] `--exit-on-complete` actually exits the TUI
- [ ] Exit code 0 on success
- [ ] Exit code non-zero on failure
- [ ] Can run `/speckit.status` headlessly and capture result
- [ ] All existing tests pass

## Commits

Start of session state:
- 6fc0dbbd1 docs: add SPEC-KIT-900 gold run spec and playbook (P0)
- 9a5e5a743 docs: add NEXT_FOCUS_ROADMAP.md (post-PR7 architect review)
- 379030a13 feat(spec-kit): add vocabulary audit script and golden evidence tests
```

---

## Quick Reference

### Build Command
```bash
cd codex-rs && cargo build -p codex-tui --release
```

### Test Pattern (from SPEC-KIT-920)
```bash
# Headless run with exit
./target/release/codex-tui \
  --initial-command "/speckit.status SPEC-KIT-900" \
  --exit-on-complete

# Check exit code
echo "Exit code: $?"
```

### Key Files
- `tui/src/cli.rs:113` - `exit_on_complete` flag defined
- `tui/src/app.rs:183-185` - `initial_command` pattern to follow
- `tui/src/app.rs:528-534` - `dispatch_initial_command` implementation
- `tui/src/chatwidget/spec_kit/pipeline_coordinator.rs` - pipeline completion events

---

**Last Updated:** 2025-12-19
