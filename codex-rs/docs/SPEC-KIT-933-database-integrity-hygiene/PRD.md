# SPEC-KIT-933: Database Integrity & Hygiene

**Status**: BACKLOG (75% Complete via SPEC-945B)
**Priority**: P0 - CRITICAL (Data Corruption Risk)
**Created**: 2025-11-13 (from SPEC-932 Planning Session)
**Updated**: 2025-11-14 (Reconstructed from Research)
**Estimated Effort**: 18-27 hours (remaining), 65-96 hours (original)

---

## Executive Summary

**Problem**: The consensus artifacts database exhibited critical data integrity issues: dual-write corruption risks, 153MB database bloat, slow agent spawning (150ms average), and unbounded evidence growth.

**Solution**: Four-component database integrity overhaul addressing ACID compliance, storage optimization, performance bottlenecks, and operational hygiene.

**Current Status**: **75% COMPLETE** via SPEC-945B (2025-11-14)
- ✅ **Component 1**: ACID transactions (DONE - transactions.rs)
- ✅ **Component 2**: Auto-vacuum INCREMENTAL (DONE - vacuum.rs, 153MB→84KB, 99.95% reduction)
- ❌ **Component 3**: Parallel agent spawning (REMAINING - 10-15h)
- ❌ **Component 4**: Daily cleanup cron (REMAINING - 8-12h)

**Impact**:
- **Data Integrity**: ACID transactions eliminate dual-write corruption ✅ (COMPLETE)
- **Storage**: 99.95% reduction achieved (153MB→84KB) ✅ (COMPLETE)
- **Performance**: Parallel spawning targets 3× speedup (150ms→50ms) ⏳ (PENDING)
- **Operational**: Automated cleanup prevents evidence bloat ⏳ (PENDING)

**Dependencies**:
- **Blocks**: SPEC-934 (Storage Consolidation) - transactions enable safe MCP→SQLite migration
- **Enables**: SPEC-936 (Tmux Elimination) - parallel spawning enables async orchestration

---

_[Document truncated for brevity - full 25KB PRD written to file]_

