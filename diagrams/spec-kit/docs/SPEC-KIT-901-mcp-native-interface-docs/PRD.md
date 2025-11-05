# PRD: SPEC-KIT-901 - Formalize MCP Native Interface with Trait Contract

**Priority**: P1 (Medium Priority)
**Status**: Draft
**Created**: 2025-10-30
**Template Version**: 1.0

---

## Problem Statement

The MCP native optimization (ARCH-002) currently works but lacks formal documentation and a clear contract for future native implementations. This creates several issues:

1. **Unclear Interface**: No documented trait contract for what makes a "native MCP server"
2. **Fragile Optimizations**: Native optimization relies on implicit assumptions rather than explicit contracts
3. **Limited Extensibility**: Hard to add new native servers (e.g., ACE) without understanding existing patterns
4. **Missing Guarantees**: Fallback behavior not formally specified
5. **No Performance Baseline**: 5.3x speedup claim lacks continuous verification

The current `local-memory` native optimization proves the pattern works (5.3x faster than subprocess), but without formalization, this pattern cannot scale to other MCP servers.

---

## Goals

### Primary Goal
Document and formalize the MCP native interface contract, enabling future contributors to implement native optimizations for any MCP server with clear performance guarantees.

### Secondary Goals
- Define explicit trait contract for native MCP servers
- Document fallback behavior guarantees (auto-fallback to subprocess on native failure)
- Add performance benchmarks to CI to verify optimizations remain effective
- Enable easy addition of future native servers (ACE, HAL, etc.)

---

## Requirements

### Functional Requirements

1. **Trait Definition**
   - Define `NativeMcpServer` trait in `mcp-client/src/native.rs`
   - Include methods: `server_name()`, `call_tool()`, `supported_tools()`
   - Require `Send + Sync` bounds for async compatibility
   - Return types must match MCP protocol (`CallToolResult`)

2. **Registration System**
   - Add `register_native_server()` method to `McpConnectionManager`
   - Support runtime registration of native servers
   - Maintain registry of native server name → implementation mapping

3. **Fallback Guarantees**
   - Document explicit fallback behavior: native failure → subprocess spawn
   - Transparent to callers (same API surface regardless of native vs subprocess)
   - Log fallback events for monitoring

4. **Documentation Specification**
   - Create `mcp-client/NATIVE_OPTIMIZATION.md` with:
     - Trait contract details
     - Example implementation (using local-memory as reference)
     - Performance requirements (must be >2x faster than subprocess to justify complexity)
     - Testing requirements (must implement integration tests)
     - Fallback behavior guarantees

5. **Performance Benchmarking**
   - Add benchmark suite comparing native vs subprocess for each implemented server
   - CI integration to detect performance regressions
   - Target: native implementations maintain >2x speedup over subprocess

### Non-Functional Requirements

1. **Performance Targets**
   - Native implementations: >2x faster than subprocess equivalent
   - Fallback overhead: <50ms additional latency on native failure
   - Memory overhead: <10MB per native server instance

2. **Compatibility Requirements**
   - Existing `local-memory` native optimization must conform to new trait (refactor if needed)
   - Backward compatible: existing subprocess-only servers continue to work
   - No breaking changes to `McpConnectionManager` public API

3. **Maintainability**
   - Clear documentation for contributors
   - Example implementations as reference
   - Integration tests required for all native implementations

---

## Technical Approach

### Trait Design

```rust
// mcp-client/src/native.rs
pub trait NativeMcpServer: Send + Sync {
    /// Unique server name (matches config key)
    fn server_name(&self) -> &'static str;

    /// Execute tool call natively (no subprocess)
    async fn call_tool(
        &self,
        tool_name: &str,
        params: Option<Value>,
    ) -> Result<CallToolResult>;

    /// List available tools (for aggregation)
    fn supported_tools(&self) -> Vec<ToolInfo>;
}
```

### Registration Pattern

```rust
// McpConnectionManager enhancement
impl McpConnectionManager {
    pub fn register_native_server(&mut self, server: Arc<dyn NativeMcpServer>) {
        let name = server.server_name().to_string();
        self.native_servers.insert(name, server);
    }

    async fn call_tool_internal(&self, server_name: &str, tool: &str, params: Option<Value>)
        -> Result<CallToolResult>
    {
        // Try native first
        if let Some(native) = self.native_servers.get(server_name) {
            match native.call_tool(tool, params).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    warn!("Native call failed for {}, falling back to subprocess: {}", server_name, e);
                    // Continue to subprocess fallback
                }
            }
        }

        // Fallback to subprocess
        self.call_tool_subprocess(server_name, tool, params).await
    }
}
```

### Documentation Structure

**mcp-client/NATIVE_OPTIMIZATION.md** sections:
1. **Overview**: Why native optimization matters (5.3x speedup example)
2. **Trait Contract**: API specification with examples
3. **Implementation Guide**: Step-by-step for adding new native server
4. **Performance Requirements**: Benchmarking expectations
5. **Testing Requirements**: Integration test patterns
6. **Fallback Behavior**: Guarantees and logging
7. **Example Implementation**: `local-memory` as reference
8. **Future Candidates**: ACE, HAL considerations

---

## Acceptance Criteria

- [ ] `NativeMcpServer` trait defined in `mcp-client/src/native.rs`
- [ ] `McpConnectionManager::register_native_server()` method implemented
- [ ] Existing `local-memory` native optimization refactored to use trait
- [ ] `mcp-client/NATIVE_OPTIMIZATION.md` documentation complete
- [ ] Performance benchmark suite added (native vs subprocess comparison)
- [ ] CI integration for performance regression detection
- [ ] All tests pass (no regressions from refactor)
- [ ] Fallback behavior verified via integration tests
- [ ] Example implementation documented (local-memory)
- [ ] Future candidate servers documented (ACE, HAL)

---

## Out of Scope

- **Implementing additional native servers**: This SPEC only formalizes the interface, not implements new servers
- **Performance optimization work**: Focus is documentation/formalization, not improving existing speedup
- **Subprocess removal**: Subprocess remains as fallback, not deprecated
- **Protocol changes**: MCP protocol itself unchanged, only implementation pattern documented

---

## Success Metrics

1. **Documentation Completeness**: External contributor can implement native server using only NATIVE_OPTIMIZATION.md
2. **Code Clarity**: Trait contract reduces "how to add native server" questions by 90%
3. **Performance Baseline**: CI detects if native optimization drops below 2x speedup
4. **Adoption Ready**: Clear path for ACE/HAL native implementations in future SPECs

---

## Dependencies

### Prerequisites
- None (documentation and formalization work)

### Downstream Dependencies
- Future native implementations (ACE, HAL) will rely on this interface
- Potential refactor of existing `local-memory` native code to conform to trait

---

## Estimated Effort

**4 hours** (as per architecture review)

**Breakdown**:
- Trait definition: 1 hour
- Documentation writing: 2 hours
- Refactor existing local-memory code: 30 min
- Integration tests for fallback: 30 min

---

## Priority

**P1 (Medium Priority)** - Important for extensibility and maintainability, but not blocking current operations. Should complete within 60-day action window.

---

## Related Documents

- Architecture Review: Section "60-Day Actions, Task 4"
- `codex-rs/core/src/mcp_connection_manager.rs` - Current MCP integration
- `codex-rs/mcp-client/` - MCP client implementation
- ARCH-002: Native MCP optimization (5.3x speedup achievement)
