**SPEC-ID**: SYNC-005
**Feature**: Keyring Store Crate
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-005
**Owner**: Code

**Context**: Port the `keyring-store` crate from upstream providing secure credential storage via system keyrings. This abstracts platform-specific secure storage (macOS Keychain, Windows Credential Manager, Linux Secret Service) behind a unified API. Currently, the fork stores credentials in config files or environment variables, which is less secure.

**Source**: `~/old/code/codex-rs/keyring-store/` (~200 LOC)

---

## User Scenarios

### P1: Secure API Key Storage

**Story**: As a user, I want my API keys stored in the system keyring so that they are protected by OS-level security rather than plaintext config files.

**Priority Rationale**: API keys in plaintext config files are a significant security risk; system keyrings provide encryption at rest.

**Testability**: Store and retrieve credentials, verify they don't appear in config files.

**Acceptance Scenarios**:
- Given a user authenticates with an API key, when stored, then it goes to system keyring not config file
- Given a stored API key, when retrieved, then the correct key is returned
- Given a user logs out, when `delete_password()` is called, then the key is removed from keyring

### P2: Cross-Platform Support

**Story**: As a developer, I want a unified keyring API so that credential storage works consistently across macOS, Windows, and Linux.

**Priority Rationale**: Multi-platform support is important but primary target is macOS/Linux.

**Testability**: Run storage tests on each platform.

**Acceptance Scenarios**:
- Given macOS, when storing credentials, then Keychain is used
- Given Windows, when storing credentials, then Credential Manager is used
- Given Linux with Secret Service, when storing credentials, then D-Bus secret service is used

### P3: Mock Implementation for Testing

**Story**: As a developer, I want a mock keyring implementation so that tests can run without system keyring access.

**Priority Rationale**: CI environments may not have keyring access; mock enables testing.

**Testability**: Unit tests use mock implementation.

**Acceptance Scenarios**:
- Given mock keyring, when credentials stored, then they persist in memory only
- Given test environment, when running tests, then mock is used automatically

---

## Edge Cases

- System keyring locked/unavailable (fallback to prompting user or error)
- Permission denied accessing keyring (clear error message)
- Keyring entry already exists (overwrite vs error - configurable)
- Very long credentials exceeding keyring limits (error with message)
- Headless/SSH environment without keyring daemon (graceful degradation)

---

## Requirements

### Functional Requirements

- **FR1**: Implement `KeyringStore` trait with `get_password()`, `set_password()`, `delete_password()`
- **FR2**: Implement platform-specific backends: macOS Keychain, Windows Credential Manager, Linux Secret Service
- **FR3**: Use service name + username as key identifier (e.g., "codex-tui" + "openai-api-key")
- **FR4**: Provide `MockKeyringStore` for testing environments
- **FR5**: Handle keyring unavailability gracefully with clear error messages

### Non-Functional Requirements

- **Performance**: Keyring operations should complete in <100ms
- **Security**: Never log or print credential values
- **Reliability**: Graceful fallback when keyring unavailable
- **Compatibility**: Support macOS 10.15+, Windows 10+, Linux with Secret Service

---

## Success Criteria

- Crate compiles on all target platforms
- Credentials can be stored and retrieved via system keyring
- Mock implementation works for tests
- No credentials appear in config files when keyring is available
- Integration with auth module documented (separate task)

---

## Evidence & Validation

**Validation Commands**:
```bash
cd codex-rs && cargo build -p codex-keyring-store
cd codex-rs && cargo test -p codex-keyring-store

# Manual verification (macOS)
security find-generic-password -s "codex-tui" -a "test-key"
```

---

## Dependencies

- `keyring` crate (or equivalent platform abstraction)
- Platform SDKs: Security.framework (macOS), credui (Windows), libsecret (Linux)

---

## Notes

- Crate copy is ~1h; full auth integration is 4-8h (separate task)
- Consider making keyring optional with feature flag for minimal builds
- Fork's current auth flow would need modification to use this (SYNC-005-integration)
- Linux requires `libsecret` dev package for compilation
