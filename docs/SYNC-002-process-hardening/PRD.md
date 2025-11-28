**SPEC-ID**: SYNC-002
**Feature**: Process Hardening Crate
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-002
**Owner**: Code

**Context**: Port the `process-hardening` crate from upstream to provide security hardening at process startup. This crate prevents sensitive data exposure through core dumps, debugger attachment, and library injection attacks. Critical for any production deployment handling sensitive data (API keys, user credentials, conversation history).

**Source**: `~/old/code/codex-rs/process-hardening/` (~150 LOC)

---

## User Scenarios

### P1: Prevent Core Dump Data Exposure

**Story**: As a security-conscious user, I want the TUI to disable core dumps so that sensitive data (API keys, conversation history) cannot be recovered from crash dumps.

**Priority Rationale**: Core dumps can contain plaintext secrets and are a common attack vector for credential theft.

**Testability**: Verify RLIMIT_CORE is set to 0 after process startup.

**Acceptance Scenarios**:
- Given the TUI starts, when a crash occurs, then no core dump file is created
- Given RLIMIT_CORE check, when queried after startup, then both soft and hard limits are 0
- Given a user triggers SIGABRT, when the signal is received, then no core file is written

### P2: Prevent Debugger Attachment

**Story**: As a security-conscious user, I want to prevent debuggers from attaching to the TUI process so that attackers cannot inspect memory for secrets.

**Priority Rationale**: Debugger attachment allows runtime memory inspection, but requires local access making it lower priority than core dumps.

**Testability**: Attempt to attach gdb/lldb to running process and verify failure.

**Acceptance Scenarios**:
- Given the TUI is running on Linux, when `gdb -p <pid>` is attempted, then attachment fails (PR_SET_DUMPABLE=0)
- Given the TUI is running on macOS, when `lldb -p <pid>` is attempted, then attachment fails (PT_DENY_ATTACH)

### P3: Remove Dangerous Environment Variables

**Story**: As a security-conscious user, I want dangerous environment variables removed so that library injection attacks are prevented.

**Priority Rationale**: LD_PRELOAD/DYLD_* attacks require the attacker to set env vars before process start, making this defense-in-depth.

**Testability**: Check env vars after startup to verify LD_*/DYLD_* are cleared.

**Acceptance Scenarios**:
- Given LD_PRELOAD is set before launch on Linux, when TUI starts, then LD_PRELOAD is unset
- Given DYLD_INSERT_LIBRARIES is set before launch on macOS, when TUI starts, then DYLD_* vars are cleared

---

## Edge Cases

- Process running without CAP_SYS_RESOURCE cannot reduce RLIMIT_CORE (should not fail startup)
- PT_DENY_ATTACH fails when already being debugged (exit with error code 6)
- prctl(PR_SET_DUMPABLE) fails on non-standard Linux kernels (exit with error code 5)
- setrlimit(RLIMIT_CORE) fails (exit with error code 7)
- Windows platform has no-op implementation (stub for future enhancement)

---

## Requirements

### Functional Requirements

- **FR1**: Call `pre_main_hardening()` early in TUI main.rs before any sensitive data is loaded
- **FR2**: On Linux/Android: Set PR_SET_DUMPABLE=0 via prctl(), set RLIMIT_CORE=0, clear LD_* env vars
- **FR3**: On macOS: Call PT_DENY_ATTACH via ptrace(), set RLIMIT_CORE=0, clear DYLD_* env vars
- **FR4**: On FreeBSD/OpenBSD: Set RLIMIT_CORE=0, clear LD_* env vars
- **FR5**: On Windows: No-op stub (document future work)
- **FR6**: Exit with specific error codes on hardening failures (5=prctl, 6=ptrace, 7=rlimit)

### Non-Functional Requirements

- **Performance**: Hardening must complete in <10ms (runs once at startup)
- **Security**: No fallback modes - if hardening fails, process must exit
- **Reliability**: Must not interfere with normal operation after hardening completes
- **Compatibility**: Support Linux (glibc/musl), macOS, FreeBSD, OpenBSD, Windows (stub)

---

## Success Criteria

- TUI binary has process-hardening as a dependency
- `pre_main_hardening()` is called in main.rs before other initialization
- Core dumps are disabled (verified via `/proc/<pid>/limits` on Linux)
- Debugger attachment fails on Linux and macOS
- LD_*/DYLD_* environment variables are cleared
- All hardening failures result in immediate process exit with documented codes

---

## Evidence & Validation

**Acceptance Tests**: See tasks.md for detailed test mapping

**Telemetry Path**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SYNC-002/`

**Validation Commands**:
```bash
# Build and verify crate compiles
cd codex-rs && cargo build -p codex-process-hardening

# Run TUI and check rlimit
./target/debug/codex-tui &
cat /proc/$!/limits | grep "Max core file size"
# Expected: 0 0 bytes

# Attempt debugger attachment (should fail)
gdb -p $! -ex "quit"
# Expected: ptrace: Operation not permitted

# Check env var clearing
LD_PRELOAD=/tmp/test.so ./target/debug/codex-tui &
cat /proc/$!/environ | grep LD_
# Expected: no output
```

---

## Clarifications

### 2025-11-27 - Initial Spec Creation

**Clarification needed**: Should hardening failures be configurable to warn-only for development?

**Resolution**: No - security hardening must be mandatory. Developers can use `CODEX_SKIP_HARDENING=1` env var to disable (documented security risk).

**Updated sections**: Added note to FR6 about optional development bypass.

---

## Dependencies

- `libc` crate (workspace dependency, already present)
- TUI main.rs modification for integration
- No external service dependencies

---

## Notes

- Upstream uses `#[ctor::ctor]` for pre-main execution; we'll call explicitly in main() for clarity
- Windows implementation is a stub - future work tracked separately
- MUSL-linked binaries ignore LD_PRELOAD anyway, but we clear for defense-in-depth
