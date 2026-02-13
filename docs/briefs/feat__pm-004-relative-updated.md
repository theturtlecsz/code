# PM-004: Relative "Last Updated" Time

<!-- REFRESH-BLOCK
query: "PM-004 relative time last updated"
snapshot: (none - read-only UI change)
END-REFRESH-BLOCK -->

## Objective

Display human-readable relative time (e.g., "2h ago") in the PM list view Updated column instead of raw YYYY-MM-DD dates.

## Spec

* **PM-UX-D5**: Last Updated column uses relative time
* **PM-UX-D3**: 30s-to-truth Q4: what changed most recently
* **Decisions**: D113, D138, D143

## Implementation

### Changes

1. **pm\_overlay.rs**:
   * Added import: `use super::session_handlers::human_ago;`
   * Line 1163 (wide layout, width >= 120): Replaced `short_date(&node.updated_at)` with `human_ago(&node.updated_at)`
   * Line 1232 (medium layout, width >= 80): Replaced `short_date(&node.updated_at)` with `human_ago(&node.updated_at)`

2. **Unit tests added**:
   * `test_relative_time_recent_timestamp`: Verifies recent timestamps show relative time (e.g., "2h ago")
   * `test_relative_time_old_timestamp`: Verifies timestamps >= 7 days show YYYY-MM-DD format
   * `test_relative_time_invalid_timestamp`: Verifies invalid timestamps return safe fallback

### Behavior

* **Recent updates (< 7 days)**: Display as relative time
  * < 1 minute: "just now"
  * < 1 hour: "Xm ago"
  * < 1 day: "Xh ago"
  * < 7 days: "Xd ago"
* **Old updates (>= 7 days)**: Display as YYYY-MM-DD (same as before)
* **Invalid timestamps**: Display as-is (no panic)

## Constraints Met

* ✅ No changes to pm-service protocol, RPC surface, or CLI behavior
* ✅ Read-only / no mutations
* ✅ Only touched pm\_overlay.rs and this brief file (2 files)
* ✅ LOC delta: \~50 lines (well under 150)

## Testing

```bash
cd codex-rs && cargo test -p codex-tui --lib pm_overlay
```

Expected output: All tests pass, including 3 new PM-004 relative time tests.

## Verification Checklist

* [ ] `cargo fmt --all -- --check` passes
* [ ] `cargo test -p codex-tui --lib` passes
* [ ] Visual verification: PM overlay shows relative time for recent items
* [ ] Edge case: Items >= 7 days old show YYYY-MM-DD
