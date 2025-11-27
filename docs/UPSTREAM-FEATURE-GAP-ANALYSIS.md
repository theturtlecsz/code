# Upstream Feature Gap Analysis: Product-Level Capabilities

**Generated**: 2025-11-27
**Source A (Fork)**: `~/code` (theturtlecsz/code)
**Source B (Upstream)**: `~/old/code` (just-every/code)
**Analysis Type**: User-facing capabilities, workflows, and architectural superpowers

---

## Executive Summary

This analysis identifies **significant product-level gaps** between the fork and upstream, focusing on user-facing capabilities rather than file-level diffs. The analysis reveals:

**Upstream Has (Fork Missing)**: 12 major feature categories
**Fork Has (Upstream Missing)**: 5 major feature categories (your unique value)
**Recommended Action**: Adopt 8 features, Ignore 4 (incompatible or redundant)

### The Strategic Picture

| Category | Upstream Strength | Fork Strength |
|----------|------------------|---------------|
| **Automation** | Auto Drive (hands-off runs) | spec-kit (multi-agent consensus) |
| **Observability** | OpenTelemetry, metrics | Evidence repository, telemetry |
| **Provider Support** | Native API calls | CLI routing (Claude/Gemini) |
| **Security** | Process hardening, keyring | Guardrails, FORK-SPECIFIC markers |
| **External Integration** | TypeScript SDK, Shell MCP | ACE playbooks, local-memory MCP |

---

## Section 1: The Feature Forest (High-Level Gaps)

### GAP-001: Auto Drive Orchestration System

**What It Is**: A fully autonomous execution mode where the model plans, executes, monitors, and recovers from errors without human intervention. Users can "hand off" a task and return to completed work.

**Upstream Implementation**:
- `code-auto-drive-core` crate (113KB coordinator!)
- Components: `auto_coordinator.rs`, `controller.rs`, `retry.rs`, `session_metrics.rs`
- Features: Compaction during runs, observer telemetry, retry/backoff, resume safety
- UX: Card-based progress, elapsed time tracking, celebration on completion

**Fork Equivalency Check**:
```bash
grep -r "auto.drive|AutoDrive" ~/code/codex-rs → No results
```
Fork has **spec-kit** which is different: multi-agent consensus for PRD workflows, not autonomous execution.

**Fork Strategy**: **EVALUATE**
- Auto Drive and spec-kit serve different purposes
- Auto Drive = "run until done, I'll be back"
- spec-kit = "coordinate multiple AIs to produce quality artifacts"
- Consider: Port Auto Drive's observer/retry patterns to spec-kit
- Alternative: Build "hands-off mode" using DirectProcessExecutor + spec-kit pipeline

**Value**: High for users who want to queue tasks and walk away. The fork's spec-kit requires more interaction.

---

### GAP-002: OpenTelemetry Observability

**What It Is**: Production-grade distributed tracing and metrics collection using OpenTelemetry standard. Enables monitoring, debugging, and performance analysis of agent runs.

**Upstream Implementation**:
- `codex-otel` crate with `otel_event_manager.rs` (19KB), `otel_provider.rs` (9KB)
- Features: Span tracing, event emission, metrics collection
- Integration: OTLP export to any OTel-compatible backend (Jaeger, Honeycomb, etc.)

**Fork Equivalency Check**:
```bash
grep -r "opentelemetry|otel" ~/code/codex-rs/core → No results
```
Fork has evidence repository + telemetry JSON, but not OTel-standard tracing.

**Fork Strategy**: **ADAPT**
- Port `codex-otel` crate as-is (standalone)
- Integrate spans into DirectProcessExecutor and spec-kit
- Use fork's existing telemetry JSON as OTel event data source

**Value**: Critical for production deployments, debugging complex multi-agent workflows.

---

### GAP-003: TypeScript SDK for External Integration

**What It Is**: A TypeScript SDK (`@openai/codex-sdk`) that allows external applications to programmatically control Codex. Enables building custom UIs, CI/CD integrations, and automation tools.

**Upstream Implementation**:
- `sdk/typescript/` with `thread.ts`, `exec.ts`, `events.ts`, `items.ts`
- Features: Thread management, event streaming, execution control
- Package: Publishable to npm, type-safe API

**Fork Equivalency Check**:
```bash
ls ~/code/sdk → Directory not found
```
Fork has no external SDK. All automation is internal (spec-kit, ACE).

**Fork Strategy**: **MERGE AS-IS**
- Copy `sdk/typescript/` directory
- Adapt for fork's protocol changes if any
- Enables: External tooling, VS Code extensions, CI/CD hooks

**Value**: Medium-High. Opens ecosystem integration possibilities.

---

### GAP-004: Shell MCP Server

**What It Is**: An MCP (Model Context Protocol) server that provides shell tool capabilities. Allows other MCP clients to execute shell commands through Codex.

**Upstream Implementation**:
- `shell-tool-mcp/` npm package
- Features: Bash execution, patched wrappers, npm publishable
- Integration: MCP protocol compliance, capabilities declaration

**Fork Equivalency Check**:
```bash
ls ~/code/shell-tool-mcp → Directory not found
```
Fork has `mcp-server` but focused on Codex-specific tools, not general shell.

**Fork Strategy**: **MERGE AS-IS**
- Copy `shell-tool-mcp/` directory
- Useful for MCP ecosystem compatibility

**Value**: Medium. Enables other AI tools to use fork's shell execution.

---

### GAP-005: Prompt Management System

**What It Is**: Full prompt management with save/reload, slash command access, and alias autocomplete. Users can customize and persist their prompts.

**Upstream Implementation**:
- `custom_prompt_view.rs`, `prompt_args.rs` in TUI
- Features: `/prompts` section, save to disk, reload, alias autocomplete
- Legacy support: Reads from `~/.codex/prompts`

**Fork Equivalency Check**:
```bash
ls ~/code/codex-rs/tui/src/bottom_pane/prompt_args.rs → Not found
```
Fork has `custom_prompt_view.rs` but lacks full management UI.

**Fork Strategy**: **ADAPT**
- Extract prompt management patterns from upstream
- Integrate with fork's TUI architecture
- Consider: Persist prompts to ACE playbooks for cross-session memory

**Value**: High for power users who customize prompts frequently.

---

### GAP-006: Character Encoding Detection

**What It Is**: Automatic detection and decoding of shell output encodings so Unicode logs stay readable across platforms.

**Upstream Implementation**:
- `chardetng` dependency in core
- Integration: Decodes shell output in `bash.rs`, `exec.rs`
- CHANGELOG: "decode shell output with detected encodings" (v0.5.7)

**Fork Equivalency Check**:
```bash
grep "chardetng" ~/code/codex-rs/Cargo.lock → Found (dependency exists but may not be used)
```
Fork has the dependency but may not use it actively.

**Fork Strategy**: **MERGE LOGIC**
- Port encoding detection from upstream's `bash.rs`
- Small change, high impact for international users

**Value**: Medium. Critical for non-ASCII environments.

---

### GAP-007: Device Code Authentication Fallback

**What It Is**: OAuth device code flow as fallback for environments where browser-based auth fails. Displays a code for user to enter on another device.

**Upstream Implementation**:
- `login.rs` device-code fallback
- CHANGELOG: "add device-code fallback and ensure ChatGPT auth links wrap cleanly" (v0.4.15)

**Fork Equivalency Check**:
```bash
grep -r "device.code|DeviceCode" ~/code/codex-rs/tui → Limited results
```
Fork's login may not have full device code support.

**Fork Strategy**: **MERGE**
- Port device code flow from upstream's login module
- Improves auth reliability in headless/SSH environments

**Value**: Medium. Critical for certain deployment scenarios.

---

### GAP-008: Responses API Proxy

**What It Is**: A proxy server that forwards Responses API calls for shared hosts. Enables multi-tenant deployments.

**Upstream Implementation**:
- `responses-api-proxy/` crate with `lib.rs` (7.5KB), `read_api_key.rs` (10KB)
- Features: SSE streaming, header handling, process hardening
- Binary: `codex-responses-api-proxy`

**Fork Equivalency Check**:
```bash
ls ~/code/codex-rs/responses-api-proxy → Not found
```
Fork has no equivalent.

**Fork Strategy**: **EVALUATE**
- Only needed for shared hosting scenarios
- If deploying fork as service: PORT
- If single-user only: SKIP

**Value**: Low-Medium. Depends on deployment model.

---

### GAP-009: /review and /merge Workflow Commands

**What It Is**: Built-in commands for code review workflows: `/review uncommitted` diffs local edits, `/merge` handles branch merging with diff summaries.

**Upstream Implementation**:
- CHANGELOG: "/review uncommitted preset" (v0.4.17), "/merge command" (v0.2.150)
- TUI integration with diff summary display

**Fork Equivalency Check**:
```bash
grep -r "review|merge" ~/code/codex-rs/tui/src/slash_command.rs → Partial
```
Fork has git tooling but may lack these specific UX flows.

**Fork Strategy**: **ADAPT**
- Extract workflow patterns
- Integrate with fork's git-tooling module
- Consider: Add to spec-kit as `/speckit.review`

**Value**: High for daily development workflows.

---

### GAP-010: Cloud Tasks Integration

**What It Is**: Integration with cloud task queues for distributed agent execution.

**Upstream Implementation**:
- `cloud-tasks/` and `cloud-tasks-client/` crates
- Features: Task queue submission, async completion

**Fork Equivalency Check**:
```bash
ls ~/code/codex-rs/cloud-tasks → Not found
```

**Fork Strategy**: **SKIP**
- Fork uses DirectProcessExecutor for local execution
- Cloud tasks require significant infrastructure
- Incompatible with fork's "native Rust, no external deps" philosophy

**Value**: Low for fork's use case.

---

### GAP-011: LMStudio Integration

**What It Is**: Native support for LMStudio local models.

**Upstream Implementation**:
- `lmstudio/` crate
- Features: Local model provider, API compatibility

**Fork Equivalency Check**:
```bash
ls ~/code/codex-rs/lmstudio → Not found
grep "ollama" ~/code/codex-rs/core → Found
```
Fork has Ollama support but not LMStudio specifically.

**Fork Strategy**: **OPTIONAL**
- Port if users request LMStudio specifically
- Ollama covers most local model use cases

**Value**: Low-Medium.

---

### GAP-012: Branch-Aware Session Resume

**What It Is**: Filter session resume by git branch to find the right session in large workspaces.

**Upstream Implementation**:
- CHANGELOG: "add branch-aware filtering to `codex resume`" (v0.4.21)
- Sorts sessions by latest activity

**Fork Equivalency Check**:
Fork has session management but may lack branch filtering.

**Fork Strategy**: **MERGE**
- Small UX improvement
- Port branch filtering logic to fork's resume picker

**Value**: Medium. Quality-of-life improvement.

---

## Section 2: What Fork Has That Upstream Doesn't

These are your **unique advantages**. Do NOT merge upstream patterns that would regress these.

### FORK-001: spec-kit Multi-Agent Orchestration

**What It Is**: 13 `/speckit.*` commands orchestrating 5 AI models (gemini, claude, gpt_pro, gpt_codex, code) for PRD-driven development.

**Fork Implementation**:
- `tui/src/chatwidget/spec_kit/` (84KB agent_orchestrator.rs + 30+ modules)
- Features: Native quality commands, consensus synthesis, quality gates, evidence tracking
- Cost: ~$2.70 per full pipeline (75% reduction via native commands)

**Upstream Has**: Nothing equivalent. Auto Drive is autonomous execution, not multi-agent consensus.

**Protect**: All `spec_kit/` modules, FORK-SPECIFIC markers

---

### FORK-002: ACE (Agentic Context Engine)

**What It Is**: Strategy memory system that injects relevant playbook bullets into agent prompts based on past successes/failures.

**Fork Implementation**:
- `ace_*.rs` modules: client, orchestrator, prompt_injector, route_selector, learning
- Config: `[ace]` section in config.toml
- Integration: MCP server for playbook management

**Upstream Has**: Nothing equivalent.

**Protect**: ACE modules, ace MCP server config

---

### FORK-003: DirectProcessExecutor

**What It Is**: Native async agent execution without tmux overhead.

**Fork Implementation**:
- `async_agent_executor.rs`, `DirectProcessExecutor`
- Performance: <50ms per agent (vs 6.5s with tmux)
- SPEC-936: Complete tmux elimination

**Upstream Has**: Still uses tmux-based execution in places.

**Protect**: DirectProcessExecutor, async patterns

---

### FORK-004: CLI Routing for Multi-Provider

**What It Is**: Route Claude and Gemini models through their native CLIs instead of OAuth.

**Fork Implementation**:
- `cli_executor/claude_pipes.rs`, `gemini_pipes.rs`
- Streaming providers, session management
- SPEC-952: Complete implementation

**Upstream Has**: Native API calls only (requires OAuth).

**Protect**: cli_executor/, streaming providers

---

### FORK-005: Native Quality Commands

**What It Is**: Zero-agent quality analysis commands that run instantly and free.

**Fork Implementation**:
- `clarify_native.rs`, `analyze_native.rs`, `checklist_native.rs`, `new_native.rs`
- Pattern matching, structural diff, rubric scoring
- Cost: $0 vs $0.80+ for agent-based

**Upstream Has**: All quality commands use agents.

**Protect**: *_native.rs modules

---

## Section 3: Implementation Tracker

### SPEC.md Compatible Task Table

| Order | Task ID | Title | Status | Owners | PRD | Branch | PR | Last Validation | Evidence | Notes |
|-------|---------|-------|--------|--------|-----|--------|----|-----------------|----------|-------|
| 1 | SYNC-010 | Evaluate Auto Drive patterns for spec-kit | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P1 Feature**: Analyze Auto Drive's observer/retry/coordinator patterns. Determine which can enhance spec-kit without architectural conflict. NOT a full port - cherry-pick patterns. Est: 8h research, 20-40h selective implementation. |
| 2 | SYNC-011 | Add OpenTelemetry observability crate | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P1 Observability**: Port `codex-otel` crate. Integrate with DirectProcessExecutor and spec-kit pipeline. Enables production monitoring. Est: 8-12h. |
| 3 | SYNC-012 | Add TypeScript SDK | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P2 Integration**: Copy `sdk/typescript/`. Adapt for fork protocol. Enables external tooling, VS Code extensions. Est: 4-6h. |
| 4 | SYNC-013 | Add Shell MCP server | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P2 Integration**: Copy `shell-tool-mcp/`. MCP ecosystem compatibility. Est: 2-3h. |
| 5 | SYNC-014 | Add prompt management UI | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P2 UX**: Port prompt save/reload, alias autocomplete from upstream. Integrate with ACE for persistence. Est: 6-10h. |
| 6 | SYNC-015 | Add character encoding detection | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P3 Quality**: Port chardetng usage from upstream bash.rs. Small change, high impact for i18n. Est: 2-3h. |
| 7 | SYNC-016 | Add device code auth fallback | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P3 Auth**: Port device code flow from upstream login. Improves headless/SSH auth. Est: 3-4h. |
| 8 | SYNC-017 | Add /review and /merge workflows | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P2 Workflow**: Port review/merge commands. Integrate with fork's git-tooling. Est: 6-8h. |
| 9 | SYNC-018 | Add branch-aware session resume | **Backlog** | Code | docs/UPSTREAM-FEATURE-GAP-ANALYSIS.md | | | | | **P3 UX**: Port branch filtering to resume picker. Small QoL improvement. Est: 2-3h. |

### Rejected Items (Incompatible or Low Value)

| Item | Reason | Alternative |
|------|--------|-------------|
| SKIP: Cloud Tasks | Requires infrastructure, conflicts with native execution | Use DirectProcessExecutor |
| SKIP: LMStudio | Ollama covers use case | Keep Ollama support |
| SKIP: Responses API Proxy | Only needed for shared hosting | N/A unless deploying as service |
| SKIP: Full Auto Drive port | Different paradigm than spec-kit | Cherry-pick patterns only |

---

## Summary

### Effort Breakdown

| Priority | Items | Total Effort | Strategic Impact |
|----------|-------|--------------|------------------|
| **P1** | SYNC-010 (Auto Drive patterns), SYNC-011 (OTel) | 28-52h | High - Production readiness |
| **P2** | SYNC-012, 013, 014, 017 | 18-27h | Medium - Ecosystem & UX |
| **P3** | SYNC-015, 016, 018 | 7-10h | Low - Quality of life |
| **Total** | **9 items** | **53-89 hours** | |

### Recommended Execution Order

**Phase 1 (Weeks 1-2)**: Foundation
- SYNC-011: OpenTelemetry (production monitoring)
- SYNC-010: Auto Drive pattern analysis (strategic decision)

**Phase 2 (Weeks 3-4)**: Integration
- SYNC-012: TypeScript SDK
- SYNC-017: /review and /merge workflows
- SYNC-014: Prompt management

**Phase 3 (Week 5)**: Polish
- SYNC-013: Shell MCP
- SYNC-015: Encoding detection
- SYNC-016: Device code auth
- SYNC-018: Branch-aware resume

### Protected Fork Features

Do NOT regress these unique capabilities:
1. **spec-kit/** - Multi-agent orchestration (80+ files)
2. **ACE/** - Agentic Context Engine
3. **DirectProcessExecutor** - Native async execution
4. **cli_executor/** - CLI routing for Claude/Gemini
5. **\*_native.rs** - Zero-cost quality commands

---

*Report generated by product gap analysis session 2025-11-27*
