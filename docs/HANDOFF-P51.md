# P51 Session Handoff

## Current State (2025-11-29)
- **Commit**: `62c752752` on main (pushed)
- **Tests**: All passing (435 workspace tests)
- **Tree**: Clean

## Prior Session (P50) Summary
- Closed SPEC-KIT-953 (Native Multi-Provider OAuth) as "Won't Do"
- Deleted 11,462 LOC of unused OAuth/native API infrastructure
- CLI routing (SPEC-952) confirmed as production path
- Simplified model_router.rs from 365 to 103 LOC

## Priority Tasks (Security Hardening Batch)

### 1. SYNC-001: Dangerous Command Detection (~2-3h)
**Goal**: Add detection for dangerous commands before execution

**Files to create**:
- `codex-rs/core/src/command_safety/is_dangerous_command.rs`
- `codex-rs/core/src/command_safety/windows_dangerous_commands.rs`

**Patterns to detect**:
- `git reset --hard`
- `rm -rf /` or `rm -rf ~`
- Nested shell commands (`bash -c`, `sh -c`)
- Force flags on destructive ops (`--force`, `-f` with rm)

**Integration**: Update `core/src/safety.rs` approval flow

**Reference**: `docs/UPSTREAM-ANALYSIS-2025-11-27.md` section SYNC-001

### 2. SYNC-002: Process Hardening Crate (~1-2h)
**Goal**: Add security hardening at process startup

**Create**: `codex-rs/process-hardening/` crate (~150 LOC)

**Features**:
- Disable core dumps: `RLIMIT_CORE=0`
- Disable ptrace: `PR_SET_DUMPABLE=0`
- Sanitize environment: Remove `LD_PRELOAD`, `DYLD_*`

**Integration**: Call `pre_main_hardening()` in `tui/src/main.rs`

**Reference**: `docs/UPSTREAM-ANALYSIS-2025-11-27.md` section SYNC-002

### 3. SYNC-003: Cargo Deny Configuration (~30min)
**Goal**: Add license and vulnerability auditing

**Create**: `codex-rs/deny.toml`

**Features**:
- License allowlist (MIT, Apache-2.0, etc.)
- RustSec advisory database integration
- CI-ready configuration

**Reference**: `docs/UPSTREAM-ANALYSIS-2025-11-27.md` section SYNC-003

### 4. CLAUDE.md Updates (~30min)
**Goal**: Remove obsolete SPEC-953 references

**Updates needed**:
- Remove Native Multi-Provider sections
- Update multi-provider guidance to reference CLI routing only
- Remove context_manager, api_clients, provider_auth mentions

## Validation Checklist
After each task:
- [ ] `cargo fmt --all`
- [ ] `cargo clippy --workspace -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Update SPEC.md task status

## Reference Documents
- `docs/UPSTREAM-ANALYSIS-2025-11-27.md` - Detailed sync item specs
- `SPEC.md` - Backlog tracking (SYNC-001 through SYNC-016)
- `core/src/command_safety/` - Existing safety infrastructure
- `core/src/safety.rs` - Approval flow integration point

## Estimated Effort
- SYNC-001: 2-3h
- SYNC-002: 1-2h
- SYNC-003: 30min
- CLAUDE.md: 30min
- **Total**: ~4-6h

## Success Criteria
1. Dangerous commands trigger approval prompt
2. Process hardening active on TUI startup
3. `cargo deny check` passes
4. CLAUDE.md updated, no 953 references
5. All tests passing, clean clippy
