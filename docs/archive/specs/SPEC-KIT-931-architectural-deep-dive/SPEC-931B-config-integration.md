# SPEC-931B: Configuration & Integration Points Analysis

**Parent**: SPEC-931 (Master Index)
**Status**: IN PROGRESS
**Created**: 2025-11-12
**Effort**: 1-2 hours (single session)

---

## Purpose

Analyze all configuration surfaces and MCP integration points for agent orchestration.

**Focus Areas**:
1. config.toml agent definitions and runtime behavior
2. prompts.json template system
3. Environment variable usage and key management
4. MCP operations (which are essential, which can be removed)
5. Runtime flexibility (what can change without restart)

**Out of Scope**: Implementation (analysis only)

---

## Analysis Framework

### 1. Configuration Surface Mapping

**Goal**: Document every configuration point that affects agent orchestration

**Scope**:
- config.toml: [[agents]] definitions, paths, timeouts
- prompts.json: Quality gate templates, stage prompts
- Environment: API keys, feature flags, telemetry controls
- Runtime state: What changes without restart?

### 2. MCP Integration Inventory

**Goal**: Classify MCP operations by necessity (essential vs convenience vs harmful)

**Scope**:
- local-memory operations (store, search)
- Current usage patterns
- Performance impact
- Alternative implementations

### 3. Configuration Validation

**Goal**: Understand how config errors are detected and handled

**Scope**:
- Startup validation
- Runtime validation
- Error messages and recovery
- Schema evolution strategy

---

## Deliverables

**1. Configuration Map**
- All config files, fields, defaults
- Runtime vs restart-required changes
- Validation logic and error handling

**2. MCP Operation Inventory**
- Every MCP call with frequency, latency, necessity
- Keep/remove/redesign decision per operation

**3. Decision Matrix**
- Which configs are essential vs optional
- Which MCP operations to keep vs eliminate
- Runtime flexibility recommendations

**4. Action Items**
- Immediate config improvements
- MCP migration plan
- Configuration hot-reload opportunities

---

## Status

**Started**: 2025-11-12
**Progress**: 0% (spec created, analysis not started)
**Blocking**: None
**Next**: Begin configuration surface analysis
