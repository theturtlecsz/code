**SPEC-ID**: SYNC-013
**Feature**: Shell MCP Server
**Status**: Backlog
**Created**: 2025-11-27
**Branch**: feature/sync-013
**Owner**: Code

**Context**: Port the `shell-tool-mcp` npm package from upstream providing an MCP (Model Context Protocol) shell tool server. This enables MCP-compatible clients (like Claude Desktop) to execute shell commands through a standardized interface, improving ecosystem compatibility.

**Source**: `~/old/code/shell-tool-mcp/`

---

## User Scenarios

### P1: MCP Client Integration

**Story**: As a user of Claude Desktop, I want to use Planner shell capabilities so that I have consistent tooling across MCP-compatible clients.

**Priority Rationale**: MCP is an emerging standard; supporting it increases adoption.

**Testability**: Connect Claude Desktop to shell-tool-mcp and execute commands.

**Acceptance Scenarios**:
- Given MCP client connects, when shell tool is invoked, then command executes
- Given command output, when returned to client, then it's properly formatted
- Given command timeout, when exceeded, then appropriate error is returned

### P2: Safe Command Execution

**Story**: As an operator, I want shell commands sandboxed so that MCP clients cannot cause system damage.

**Priority Rationale**: Security is critical when exposing shell access to external clients.

**Testability**: Attempt dangerous commands and verify they're blocked or sandboxed.

**Acceptance Scenarios**:
- Given dangerous command (rm -rf), when attempted, then it requires approval or is blocked
- Given whitelisted command, when executed, then it runs without prompts
- Given command output, when returned, then sensitive data is redacted

### P3: Bash Wrapper Compatibility

**Story**: As a developer, I want patched Bash wrappers so that common shell patterns work correctly.

**Priority Rationale**: Compatibility with existing scripts reduces friction.

**Testability**: Run common Bash patterns and verify correct behavior.

**Acceptance Scenarios**:
- Given heredoc syntax, when executed, then it works correctly
- Given pipe chains, when executed, then output flows correctly
- Given environment variables, when set, then they persist in session

---

## Edge Cases

- Long-running commands (timeout handling, streaming output)
- Interactive commands (not supported, clear error)
- Shell injection attempts (input sanitization)
- Binary output (encoding, truncation)
- Concurrent command execution (queue or parallel)

---

## Requirements

### Functional Requirements

- **FR1**: Implement MCP server following Model Context Protocol spec
- **FR2**: Expose shell execution as MCP tool
- **FR3**: Support command approval workflow (dangerous command detection from SYNC-001)
- **FR4**: Stream command output via MCP events
- **FR5**: Provide configurable sandboxing (use fork's existing sandbox infrastructure)
- **FR6**: Support working directory specification

### Non-Functional Requirements

- **Performance**: Command startup latency <100ms
- **Security**: All commands subject to safety checks
- **Compatibility**: Work with Claude Desktop and other MCP clients
- **Reliability**: Graceful handling of client disconnection

---

## Success Criteria

- MCP server starts and accepts connections
- Claude Desktop can connect and execute commands
- Dangerous command detection works (from SYNC-001)
- Output streaming works for long-running commands
- README documents setup for various MCP clients

---

## Evidence & Validation

**Validation Commands**:
```bash
cd shell-tool-mcp
npm install
npm run build
npm start

# Test with MCP client
# Configure Claude Desktop to use shell-tool-mcp
# Execute test command and verify output
```

---

## Dependencies

- MCP SDK (from Anthropic or compatible)
- Node.js 18+
- Fork's dangerous command detection (SYNC-001)

---

## Notes

- Relatively small package - 2-3h estimated
- Consider integration with fork's sandbox infrastructure
- May need to adapt for fork's specific safety checks
- MCP specification may have evolved since upstream version
