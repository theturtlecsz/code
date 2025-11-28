**SPEC-ID**: SYNC-015
**Feature**: Character Encoding Detection
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-015
**Owner**: Code

**Context**: Port `chardetng` usage from upstream's bash.rs for automatic character encoding detection in command output. This improves Unicode log decoding for international users whose systems may output non-UTF-8 text. Small change with high impact for i18n support.

**Source**: `~/old/code/codex-rs/exec/src/bash.rs` (chardetng usage)

---

## User Scenarios

### P1: Non-UTF-8 Output Handling

**Story**: As a user with non-UTF-8 locale, I want command output decoded correctly so that I see readable text instead of mojibake.

**Priority Rationale**: Broken text display is confusing and unprofessional; encoding detection fixes this.

**Testability**: Run command that outputs non-UTF-8 text, verify correct display.

**Acceptance Scenarios**:
- Given command outputs Shift-JIS text, when displayed, then Japanese characters render correctly
- Given command outputs Latin-1 text, when displayed, then accented characters render correctly
- Given UTF-8 output, when processed, then no regression in existing behavior

---

## Edge Cases

- Mixed encodings in single output (best-effort detection)
- Binary output misdetected as text (length/entropy heuristics)
- Very short output (insufficient data for detection - assume UTF-8)
- Encoding detection confidence low (fallback to UTF-8 with replacement chars)

---

## Requirements

### Functional Requirements

- **FR1**: Add `chardetng` crate dependency to exec crate
- **FR2**: Detect encoding of command stdout/stderr before string conversion
- **FR3**: Transcode non-UTF-8 output to UTF-8 for display
- **FR4**: Preserve original bytes for logging/debugging if needed
- **FR5**: Handle detection failures gracefully (fallback to UTF-8 lossy)

### Non-Functional Requirements

- **Performance**: Encoding detection <5ms for typical output (<100KB)
- **Accuracy**: Correct detection for common encodings (UTF-8, Latin-1, Shift-JIS, GB2312)
- **Compatibility**: No regression for existing UTF-8 workflows

---

## Success Criteria

- `chardetng` integrated into exec crate
- Non-UTF-8 command output displays correctly
- UTF-8 output continues to work (no regression)
- Detection performance acceptable (<5ms typical)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-exec

# Test with non-UTF-8 output
echo -e '\x82\xb1\x82\xf1\x82\xc9\x82\xbf\x82\xcd' | iconv -f SHIFT-JIS -t SHIFT-JIS > /tmp/sjis.txt
cat /tmp/sjis.txt  # In TUI, verify correct Japanese display
```

---

## Dependencies

- `chardetng` crate
- exec crate output handling (existing)

---

## Notes

- Estimated 2-3h - small, focused change
- Quick win for international users
- Consider making detection optional via feature flag for minimal builds
- May want to log detected encoding for debugging
