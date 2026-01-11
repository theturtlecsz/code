**SPEC-ID**: SYNC-016
**Feature**: Device Code Auth Fallback
**Status**: Backlog
**Created**: 2025-11-28
**Branch**: feature/sync-016
**Owner**: Code

**Context**: Port device code authentication flow from upstream's login module. This provides an alternative authentication method where users enter a code on another device (phone/laptop browser), useful for headless servers, SSH sessions, and environments where browser-based OAuth isn't possible.

**Source**: `~/old/code/codex-rs/login/src/` (device code flow)

---

## User Scenarios

### P1: Headless Server Authentication

**Story**: As a user on a headless server, I want device code auth so that I can authenticate without a local browser.

**Priority Rationale**: Headless/SSH is common deployment scenario; current auth may not work.

**Testability**: SSH to server, run auth, verify device code flow works.

**Acceptance Scenarios**:
- Given no browser available, when auth initiated, then device code is displayed
- Given device code shown, when user enters code on another device, then auth completes
- Given code expires, when timeout reached, then clear error with retry option

### P2: Corporate Proxy Environments

**Story**: As a user behind corporate proxy, I want device code auth so that I can authenticate when browser redirect is blocked.

**Priority Rationale**: Corporate environments often block OAuth redirects; device code works around this.

**Testability**: Simulate blocked redirect, verify device code fallback works.

**Acceptance Scenarios**:
- Given browser OAuth fails, when fallback triggered, then device code flow starts
- Given device code flow, when user authenticates on allowed device, then tokens received

---

## Edge Cases

- Device code expires before user enters it (clear retry message)
- Network interruption during polling (retry with backoff)
- User enters wrong code (clear error, show code again)
- Multiple auth attempts simultaneously (handle or serialize)
- Provider doesn't support device code (skip, use other methods)

---

## Requirements

### Functional Requirements

- **FR1**: Implement device code request to OAuth provider
- **FR2**: Display user code and verification URL clearly in terminal
- **FR3**: Poll for token completion with configurable interval
- **FR4**: Handle code expiration with clear retry flow
- **FR5**: Integrate with existing auth module as fallback method
- **FR6**: Support at least OpenAI device code flow

### Non-Functional Requirements

- **Performance**: Polling interval 5s (configurable)
- **Usability**: Clear instructions and copy-pasteable code
- **Security**: Secure token storage after auth (use keyring if available)

---

## Success Criteria

- Device code flow works for OpenAI authentication
- Headless/SSH authentication succeeds
- Clear display of code and verification URL
- Proper expiration and retry handling
- Integration with existing auth as fallback option

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-tui

# Test in headless environment
ssh server "DISPLAY= ./codex-tui"
# Verify device code prompt appears
# Complete auth on another device
# Verify auth succeeds
```

---

## Dependencies

- OAuth provider device code endpoint support
- Auth module (existing)
- HTTP client for polling (existing)

---

## Notes

- Estimated 3-4h
- Fork uses different auth approach (CLI routing) - verify compatibility
- Not all OAuth providers support device code - document which do
- Consider integration with keyring-store (SYNC-005) for token storage
- May want to detect headless environment and auto-suggest device code
