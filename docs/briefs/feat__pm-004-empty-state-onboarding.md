# PM-004: Empty State Onboarding View

<!-- REFRESH-BLOCK
query: "PM-004 empty state onboarding"
snapshot: (none - read-only UI enhancement)
END-REFRESH-BLOCK -->

## Objective

Implement PM-UX-D24 empty-state onboarding view in PM list mode when PM data is empty and service is healthy.

## Spec

* **PM-UX-D7**: Empty state triggers guided onboarding wizard
* **PM-UX-D24**: Onboarding wizard displays 3-step flow
* **Decisions**: D113, D138, D143

## Implementation

### Changes

1. **pm\_overlay.rs**:
   * Added conditional branch in `render_pm_overlay`: when `!degraded && nodes.is_empty()` → render onboarding
   * Implemented `render_empty_state_onboarding()` function displaying 3-step guide
   * PM-UX-D15 worst-case fallback unchanged (degraded + empty takes precedence)

2. **Unit tests added** (4 new tests):
   * `test_empty_state_onboarding_renders_when_healthy_and_empty`: Verifies onboarding displays when service healthy and nodes empty
   * `test_empty_state_onboarding_shows_three_steps`: Verifies all 3 PM-UX-D24 steps are rendered
   * `test_degraded_empty_shows_worst_case_not_onboarding`: Verifies PM-UX-D15 takes precedence over PM-UX-D24
   * `test_non_empty_does_not_show_onboarding`: Verifies onboarding does NOT show when nodes exist

### Onboarding Panel Content (PM-UX-D24)

The empty-state panel displays:

**Step 1**: Confirm project container (default: repository name)
**Step 2**: Choose first work item type (Feature or SPEC)
**Step 3**: Enter title/name and begin maieutic intake

Plus guidance on using CLI commands to create the first work item.

### Behavior

* **Healthy + Empty** (`!degraded && nodes.is_empty()`): Show onboarding panel
* **Degraded + Empty** (`degraded && nodes.is_empty()`): Show worst-case fallback (PM-UX-D15 precedence)
* **Non-empty**: Show normal summary bar + list (no onboarding)

## Constraints Met

* ✅ No changes to pm-service protocol, RPC methods, or CLI behavior
* ✅ Read-only / no mutations (display-only onboarding)
* ✅ Only touched pm\_overlay.rs and this brief file (2 files)
* ✅ LOC delta: \~120 lines (within budget of <= 180)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass, including 4 new PM-UX-D24 onboarding tests.

## Verification Checklist

* [x] `cargo fmt --all -- --check` passes
* [x] `cargo test -p codex-tui --lib` passes (31/31 tests)
* [ ] Visual verification: PM overlay shows onboarding when empty + healthy
* [ ] Edge case: Degraded + empty shows worst-case fallback, NOT onboarding
