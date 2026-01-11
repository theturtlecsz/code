**SPEC-ID**: SYNC-003
**Feature**: Cargo Deny Configuration
**Status**: Done
**Created**: 2025-11-28
**Completed**: 2025-11-28
**Branch**: main
**Owner**: Code

**Context**: Add cargo-deny configuration for dependency vulnerability scanning, license compliance, and ban checking. Enables automated security auditing in CI and local development.

**Source**: Upstream `deny.toml` (~273 LOC) with fork-specific additions

---

## Implementation Summary

### Configuration Added
- `codex-rs/deny.toml` (288 LOC)

### Checks Enabled
1. **Advisories**: Vulnerability scanning against RustSec database
2. **Licenses**: SPDX license compliance (Apache-2.0, MIT, BSD, etc.)
3. **Bans**: Duplicate crate detection (warning level)
4. **Sources**: Registry and git source validation

### Fork-Specific Modifications

**Advisories Ignored** (no fix available):
- RUSTSEC-2025-0120: json5 unmaintained (config→codex-spec-kit)
- RUSTSEC-2024-0320: yaml-rust unmaintained (syntect→tui-markdown)
- RUSTSEC-2023-0080: transpose buffer overflow (img_hash chain, impractical attack vector)

**Licenses Added**:
- NCSA (libfuzzer-sys transitive)
- Apache-2.0 WITH LLVM-exception (target-lexicon)

**Git Sources Allowed**:
- github.com/nornagon (ratatui fork)

### Workspace License Standardization
All 28 workspace crates now have `license = "Apache-2.0"` in their Cargo.toml.

---

## Validation

\`\`\`bash
# Run all checks
cd codex-rs && cargo deny check

# Run specific check
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources
\`\`\`

**Result**: advisories ok, bans ok, licenses ok, sources ok

---

## CI Integration (Future)

Add to `.github/workflows/ci.yml`:
\`\`\`yaml
- name: Check dependencies
  run: |
    cargo install cargo-deny
    cargo deny check
\`\`\`

---

## Notes

- Upstream ignored advisories retained for compatibility
- Fork adds 3 new advisory ignores for fork-specific dependencies
- All workspace crates standardized to Apache-2.0 license
