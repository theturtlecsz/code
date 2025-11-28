**SPEC-ID**: SYNC-003
**Feature**: Cargo Deny Configuration
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-003
**Owner**: Code

**Context**: Add `deny.toml` configuration for dependency security auditing via `cargo deny`. This enables automated checking of licenses, known vulnerabilities (RustSec advisory database), and banned crate detection. Essential for maintaining a secure supply chain and ensuring license compliance.

**Source**: `~/old/code/codex-rs/deny.toml`

---

## User Scenarios

### P1: Vulnerability Detection

**Story**: As a maintainer, I want automated vulnerability scanning so that known security issues in dependencies are detected before deployment.

**Priority Rationale**: Vulnerable dependencies are a primary attack vector; automated detection is critical for security.

**Testability**: Run `cargo deny check advisories` and verify it catches test vulnerabilities.

**Acceptance Scenarios**:
- Given a dependency has a RustSec advisory, when `cargo deny check` runs, then the advisory is reported
- Given all dependencies are clean, when `cargo deny check` runs, then it passes with exit code 0
- Given a new advisory is published, when CI runs, then the build fails with advisory details

### P2: License Compliance

**Story**: As a maintainer, I want license auditing so that the project maintains compliance with approved open-source licenses.

**Priority Rationale**: License compliance is legally important but less urgent than security vulnerabilities.

**Testability**: Run `cargo deny check licenses` and verify allowed/denied licenses.

**Acceptance Scenarios**:
- Given a dependency uses MIT license, when license check runs, then it passes (MIT is allowed)
- Given a dependency uses GPL-3.0, when license check runs, then it fails (GPL not in allow-list)
- Given a new dependency is added, when PR check runs, then license is validated

### P3: Banned Crate Detection

**Story**: As a maintainer, I want to ban specific problematic crates so that known-bad dependencies cannot be introduced.

**Priority Rationale**: Banned crates are rare but provide defense against specific known issues.

**Testability**: Attempt to add a banned crate and verify denial.

**Acceptance Scenarios**:
- Given `openssl` is banned (prefer rustls), when someone adds openssl, then build fails
- Given a crate wrapper for banned functionality, when check runs, then transitive ban applies

---

## Edge Cases

- Fork-specific dependencies may need advisory ignores (document each ignore with justification)
- Yanked crates should be flagged even without advisories
- Private/unpublished crates won't have license metadata (use `clarify` for unknown)
- Multiple versions of same crate may have different vulnerability status

---

## Requirements

### Functional Requirements

- **FR1**: Create `codex-rs/deny.toml` with sections: `[advisories]`, `[licenses]`, `[bans]`, `[sources]`
- **FR2**: Configure license allow-list including: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Zlib, MPL-2.0, CC0-1.0, Unlicense
- **FR3**: Enable RustSec advisory database integration with `db-path` and `db-urls` configuration
- **FR4**: Document any advisory ignores with RUSTSEC ID and justification comment
- **FR5**: Configure source restrictions to crates.io only (no git dependencies without explicit allow)
- **FR6**: Add `cargo deny check` to CI workflow (optional, can be manual initially)

### Non-Functional Requirements

- **Performance**: `cargo deny check` should complete in <30s for full workspace
- **Maintainability**: All ignores must have inline comments explaining why
- **Automation**: Configuration should work with both local runs and CI
- **Compatibility**: Support cargo-deny 0.14+ features

---

## Success Criteria

- `deny.toml` exists in `codex-rs/` directory
- `cargo deny check` passes on current codebase (or documents known ignores)
- License allow-list covers all current dependencies
- No unacknowledged RustSec advisories
- CI integration documented (even if not implemented in this task)

---

## Evidence & Validation

**Acceptance Tests**: See tasks.md for detailed test mapping

**Telemetry Path**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/commands/SYNC-003/`

**Validation Commands**:
```bash
# Install cargo-deny if not present
cargo install cargo-deny

# Run full check
cd codex-rs && cargo deny check

# Run individual checks
cargo deny check advisories
cargo deny check licenses
cargo deny check bans
cargo deny check sources

# Generate license report
cargo deny list
```

---

## Clarifications

### 2025-11-27 - Initial Spec Creation

**Clarification needed**: Should we fail CI on advisory warnings or only errors?

**Resolution**: Fail on errors (vulnerabilities), warn on unmaintained crates. Configure `vulnerability = "deny"`, `unmaintained = "warn"`.

**Updated sections**: FR3 updated with severity configuration.

---

## Dependencies

- `cargo-deny` tool (install via `cargo install cargo-deny`)
- Internet access for RustSec database fetch
- No code dependencies - configuration file only

---

## Notes

- Upstream deny.toml may have ignores for dependencies we don't use - review and remove irrelevant ones
- Fork-specific crates (spec-kit, ACE) won't have upstream ignores - may need to add
- Consider adding pre-commit hook for `cargo deny check` (separate enhancement)
- RustSec database updates automatically; periodic manual review recommended
