**SPEC-ID**: SYNC-012
**Feature**: TypeScript SDK
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-012
**Owner**: Code

**Context**: Port the TypeScript SDK (`@upstream repository-sdk`) from upstream enabling external tooling integration. This SDK provides thread management, event streaming, and execution control for building VS Code extensions, web interfaces, and other integrations with the Planner/TUI backend.

**Source**: `~/old/code/sdk/typescript/`

---

## User Scenarios

### P1: VS Code Extension Development

**Story**: As an extension developer, I want a TypeScript SDK so that I can build VS Code integrations with Planner.

**Priority Rationale**: VS Code is the primary editor for many users; SDK enables rich integrations.

**Testability**: Build sample VS Code extension using SDK.

**Acceptance Scenarios**:
- Given SDK is installed, when imported in TypeScript, then types are available
- Given SDK client, when starting conversation, then thread is created
- Given running thread, when events occur, then callbacks are invoked

### P2: Event Streaming

**Story**: As a developer, I want to stream execution events so that I can build real-time UIs.

**Priority Rationale**: Real-time feedback is essential for good UX in integrations.

**Testability**: Subscribe to events and verify real-time delivery.

**Acceptance Scenarios**:
- Given active conversation, when model responds, then streaming events are received
- Given command execution, when output is generated, then it streams incrementally
- Given error occurs, when streaming, then error event is delivered

### P3: Thread Management

**Story**: As a developer, I want thread lifecycle management so that I can maintain conversation state.

**Priority Rationale**: Thread management enables persistent conversations across sessions.

**Testability**: Create, pause, resume, and delete threads.

**Acceptance Scenarios**:
- Given new conversation, when thread created, then unique ID is returned
- Given existing thread, when resumed, then conversation history is restored
- Given completed thread, when deleted, then resources are cleaned up

---

## Edge Cases

- Connection lost during streaming (reconnection with event replay)
- Invalid API credentials (clear error before any API calls)
- Thread not found (explicit error vs silent failure)
- Concurrent modifications to same thread (last-write-wins or conflict error)
- Large conversation history (pagination, summarization)

---

## Requirements

### Functional Requirements

- **FR1**: Publish npm package with TypeScript types
- **FR2**: Implement `CodexClient` class for API interaction
- **FR3**: Support thread CRUD operations (create, read, update, delete)
- **FR4**: Implement event streaming via SSE or WebSocket
- **FR5**: Provide execution control (start, pause, cancel)
- **FR6**: Support authentication via API key or OAuth tokens

### Non-Functional Requirements

- **Performance**: Event latency <100ms from backend to SDK callback
- **Compatibility**: Support Node.js 18+, modern browsers
- **Bundle Size**: <50KB minified+gzipped for browser usage
- **Documentation**: JSDoc comments and README with examples

---

## Success Criteria

- Package builds and publishes to npm (or local registry)
- TypeScript types are correct and complete
- Sample VS Code extension works with SDK
- Event streaming delivers real-time updates
- README includes quickstart guide

---

## Evidence & Validation

**Validation Commands**:
```bash
cd sdk/typescript
npm install
npm run build
npm run test

# Local usage test
npm link
cd /tmp/test-project
npm link @upstream repository-sdk
# Create test script using SDK
```

---

## Dependencies

- Node.js 18+
- TypeScript 5+
- Backend API (TUI or server mode)

---

## Notes

- Upstream uses `@upstream repository-sdk` name - consider fork-specific name
- May need to adapt for fork's API differences (CLI routing, etc.)
- Consider WebSocket support in addition to SSE for bidirectional communication
- VS Code extension is separate project that uses this SDK
