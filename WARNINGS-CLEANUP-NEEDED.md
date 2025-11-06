# Build Warnings Cleanup - Future SPEC

**Current Status**: 140 total build warnings
- codex-tui: 134 warnings (16 auto-fixable)
- codex-core: 6 warnings

**Impact**: Low (warnings, not errors) but affects code quality

**Recommendation**: Create dedicated SPEC for warning cleanup
- Run `cargo fix --lib -p codex-tui` for auto-fixes
- Manually address remaining warnings
- Enforce warning-free builds in CI

**Priority**: Medium (technical debt)
**Effort**: ~2-4 hours

**Not urgent** - system works correctly, but should be addressed for code quality.
