# Session Handoff: Prompt Pack Implementation (WP-A through WP-D)

**Generated:** 2026-01-25
**Session:** Prompt Pack - 4 Work Packages
**Status:** WP-A Complete, WP-B/C/D Pending

***

## Restart Prompt

Copy everything below the `---` to start a new session:

***

Continue implementing the Prompt Pack (4 Work Packages). **WP-A is complete and committed to working tree.** Continue with WP-B.

## Session Context

You are implementing a prompt pack with 4 work packages for the spec-kit system. The full plan is at `/home/thetu/.claude/plans/lazy-launching-trinket.md`.

**User preference:** Separate PRs per work package.

## Completed: WP-A ("Filesystem Is Projection" Rebuild Command)

All files created/modified and build verified (35s build, no errors):

**New Files:**

* `codex-rs/tui/src/chatwidget/spec_kit/rebuild_projections.rs` - Shared core (`RebuildRequest`, `RebuildResult`, `rebuild_projections()`)
* `codex-rs/tui/src/chatwidget/spec_kit/commands/projections.rs` - TUI `/speckit.projections rebuild` command

**Modified Files:**

* `codex-rs/cli/src/speckit_cmd.rs` - Added `Projections(ProjectionsArgs)` subcommand with `rebuild`
* `codex-rs/tui/src/chatwidget/spec_kit/error.rs` - Added `RebuildError(String)` variant
* `codex-rs/tui/src/chatwidget/spec_kit/mod.rs` - Added `pub mod rebuild_projections;`
* `codex-rs/tui/src/chatwidget/spec_kit/commands/mod.rs` - Added `mod projections;` and `pub use projections::*;`
* `codex-rs/tui/src/chatwidget/spec_kit/command_registry.rs` - Added `registry.register(Box::new(ProjectionsCommand));`
* `codex-rs/tui/src/lib.rs` - Added exports for rebuild types

**CLI verified working:**

```bash
code speckit projections rebuild --help  # Shows full help
code speckit projections rebuild --dry-run --no-vision --json  # Returns proper JSON exit code 2 (no capsule)
```

***

## Next: WP-B (Headless Deep Parity for Grounding)

**Goal:** Make `code speckit new --deep` and `code speckit projectnew --deep` populate `grounding_uris[]` like TUI does.

**Key Finding:** CLI currently passes `Vec::new()` for grounding\_uris. Comments in code state "grounding capture is TUI-only".

### Implementation Steps

**1. Export grounding functions from lib.rs:**

```rust
// Add to codex-rs/tui/src/lib.rs after vision exports (~line 267)
pub use chatwidget::spec_kit::grounding::{
    GroundingCaptureResult, capture_grounding_for_spec_intake, capture_grounding_for_project_intake,
};
```

**2. Modify `run_new()` in speckit\_cmd.rs (\~line 6160):**

Current code:

```rust
let design_brief = build_design_brief(..., Vec::new());  // Empty grounding
```

Change to:

```rust
// Step 7.5: Deep grounding capture (if deep mode)
let grounding_uris = if args.deep {
    match capture_grounding_for_spec_intake(&cwd, &spec_id, &intake_id) {
        Ok(result) => {
            if !args.json {
                eprintln!("Deep grounding captured: {} artifacts", result.grounding_uris.len());
            }
            result.grounding_uris
        }
        Err(e) => {
            // Deep requires grounding; failure blocks completion
            let exit_code = headless_exit::HARD_FAIL;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                    "schema_version": SCHEMA_VERSION,
                    "tool_version": tool_version(),
                    "exit_code": exit_code,
                    "error": format!("Deep grounding failed: {}", e),
                }))?);
            } else {
                eprintln!("Error: Deep grounding failed: {}", e);
            }
            std::process::exit(exit_code);
        }
    }
} else {
    Vec::new()
};

let design_brief = build_design_brief(..., grounding_uris);
```

**3. Modify `run_projectnew()` (\~line 6464):**
Same pattern with `capture_grounding_for_project_intake(&cwd, &project_id)`.

**4. Add import at top of speckit\_cmd.rs:**

```rust
use codex_tui::{capture_grounding_for_spec_intake, capture_grounding_for_project_intake};
```

**5. Extend JSON output:**
Add `grounding_result` field to success JSON output.

### Key Files for WP-B

| File                                                | Purpose                           |
| --------------------------------------------------- | --------------------------------- |
| `codex-rs/tui/src/chatwidget/spec_kit/grounding.rs` | Grounding capture functions       |
| `codex-rs/cli/src/speckit_cmd.rs:6010-6264`         | `run_new()` implementation        |
| `codex-rs/cli/src/speckit_cmd.rs:6267-6588`         | `run_projectnew()` implementation |
| `codex-rs/tui/src/lib.rs`                           | Exports                           |

### Acceptance Criteria (WP-B)

* `code speckit new --deep --answers answers.json` produces non-empty `grounding_uris[]` in `DesignBrief`
* `code speckit projectnew --deep --answers answers.json` produces non-empty `grounding_uris[]` in `ProjectBrief`
* Capsule contains grounding artifacts under same namespaces as TUI
* JSON output includes `grounding_result` summary
* Deep grounding failure is exit code 2 (HARD\_FAIL)

***

## Remaining: WP-C and WP-D

### WP-C: Documentation Alignment

**Files:**

* `docs/spec-kit/COMMANDS.md` - Document all spec-kit commands (create)
* `docs/OPERATIONAL-PLAYBOOK.md` - Add capsule SoR section (update)
* `docs/spec-kit/CAPSULE-NAMESPACES.md` - Document URI schemes (create)

**Content:** Commands reference table, capsule SoR contract, enforcement gates with exit codes.

### WP-D: Enforcement Tests

**Files to create:**

* `codex-rs/tui/src/chatwidget/spec_kit/tests/deep_validation_tests.rs`
* `codex-rs/tui/src/chatwidget/spec_kit/tests/projection_tests.rs`

**Tests:** Deep validation hard-fails, projection provenance, schema stability.

***

## Key Patterns Reference

**Capsule API:**

```rust
let capsule = CapsuleHandle::open(config)?;
let events = capsule.list_events();
let intake_events: Vec<_> = events.iter()
    .filter(|e| e.event_type == EventType::IntakeCompleted)
    .collect();
let payload: IntakeCompletedPayload = serde_json::from_value(event.payload.clone())?;
let brief_bytes = capsule.get_bytes_str(&payload.brief_uri, None, None)?;
```

**Grounding functions:**

```rust
// Returns GroundingCaptureResult { grounding_uris, artifact_hashes, harvest, project_intel }
capture_grounding_for_spec_intake(cwd, spec_id, intake_id)
capture_grounding_for_project_intake(cwd, project_id)
```

**Build commands:**

```bash
~/code/build-fast.sh              # Fast build (~35s)
cargo check -p codex-tui          # Check TUI only
cargo check -p codex-cli          # Check CLI only
```

***

## Plan File Location

Full implementation plan: `/home/thetu/.claude/plans/lazy-launching-trinket.md`
