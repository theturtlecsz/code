Integration Architecture: NotebookLM ↔ Claude Code
Overview
The integration of NotebookLM will execute the deferred Research/Reasoning Integration feature (Phase 3), which enhances the Productivity (RProd) dimension of the PPP Framework. NotebookLM is integrated as an external knowledge tool via the Model Context Protocol (MCP), providing high-effort research and contextual synthesis. This integration is cost-optimized, reserving the high-cost NotebookLM calls (∼$0.50 estimated) only for Specific + Complex tasks, preventing wasted compute.
Integration Points
NotebookLM integration focuses on stages requiring deep contextual knowledge or conflict resolution based on historical data.
Stage
Trigger
Input
Process
Output Format
Handoff Method
Specify/New
Specific + Complex prompt detected in /new.spec or /specify flow. Triggered after native Clarify gate passes.
JSON-RPC query containing spec_id, stage, and a research question (e.g., "Synthesize PKCE implementation standards").
Accesses external knowledge base (historical SPECs, ADRs). Synthesizes domain best practices and required constraints.
Markdown Template (Research Brief format with synthesized insights and trade-offs).
Tool Result (MCP): MCP Client receives the response and injects the structured brief into the Claude agent's context.
Plan
Multi-agent consensus identifies a Critical Architectural Conflict (e.g., layered vs. monolithic).
JSON-RPC query listing conflicting outputs and the architectural aspect.
Queries historical ADRs and Evidence Repository to find precedent or consensus. Summarizes risk of each option.
JSON (Structured list of recommended patterns, confidence scores, and risks).
MCP Tool Invocation: The result is consumed by the Consensus Coordinator for synthesis.
Audit
Tier 3 Premium Agents are auditing code/dependencies against project compliance failures (e.g., licensing, security policy).
JSON-RPC query listing dependencies or security requirements.
Queries internal security policies or project-specific standards stored in NotebookLM.
JSON (Structured report with status and conflict_severity).
MCP Tool Invocation: Results are integrated into the final audit_report.md artifact.
RScore
Data-Driven Weight Optimization is run (Phase 3).
Query for historical consensus.json artifacts and execution.json telemetry.
Analyzes past Reasoning Regret (linking architectural decisions to success rates) to find optimal PPP consensus weights.
JSON (Structured output for the Grid Search algorithm).
Direct Analysis: Feeds data back to configuration defaults (config.toml).
Data Flow Diagrams
The workflow adheres to the Separate Systems decision, mandating that Vagueness (RProact) is checked before Complexity (RProd) research is initiated.
graph TD
    A[1. User Input (Vague/Complex)] --> B{2. Vagueness Check (Native)?};
    B -- Vague --> C[3. Agent Q&A (RProact)];
    C --> D{4. Clarification Complete?};
    D -- Yes --> E{5. Complexity Check (Heuristic)?};
    D -- No --> F[Escalate to Human (Max Turns)];
    E -- Complex --> G[6. Allocate High Reasoning (RProd)];
    G --> H[7. Claude Agent → MCP Trigger];
    H --> I[8. MCP Connection Manager];
    I --> J[9. NotebookLM Server];
    J --> K[10. Structured Brief (Tool Result)];
    K --> L[11. Claude Code Consumes Brief];
    L --> M[12. Draft SPEC Artifact (spec.md)];
    M --> N[13. Final Consensus (70/30)];
    E -- Simple --> M;
File Structure
The integration primarily affects the prompt processing layer, where the decision to invoke NotebookLM is made.
• Path: docs/SPEC-PPP-XXX-notebooklm-integration/ (NEW SPEC documentation directory)
• Path: codex-rs/core/src/config_types.rs (Modification: Add configuration structure for [mcp_servers.notebooklm]).
• Path: codex-rs/tui/src/chatwidget/spec_kit/prompt_processor.rs (Modification: Implement Complexity Analysis component and Sequential Logic for research allocation).
• Path: codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs (Modification: Logic to execute Claude Code with ReasoningEffort::High and manage MCP response injection).
• Path: codex-rs/tui/src/output_formatter.rs (Modification: Potentially use structured outputs from NotebookLM to ensure compliance with RPers preferences, like require_json).
Dependencies
• New Tools/Libraries Needed: No new Rust libraries are required, as the existing MCP Client is sufficient.
• MCP Servers Required: The public notebooklm-mcp server must be installed and configured.
• Configuration Changes:
    ◦ The server must be defined in ~/.code/config.toml under [mcp_servers.notebooklm].
    ◦ model_reasoning_effort must be configurable to high.
Implementation Phases
The integration follows the recommended phased rollout, prioritizing cost-effective validation before full deployment.
Phase
Goal
Rationale
Phase 1 (Minimal Viable Integration)
Configure and Validate MCP: Implement Phase 1 (foundation) by configuring the notebooklm-mcp server in config.toml and verifying connection via the McpConnectionManager.
Dependency Foundation: Validates external tool integration and communication framework (MCP).
Phase 2 (Enhanced Capabilities)
High Reasoning Tool Invocation: Implement Complexity Analysis and the Sequential Logic in the prompt processor. Enable agents to invoke NotebookLM during the /specify or /plan flow for Specific + Complex prompts.
Highest ROI: Activates the core feature that prevents wasted compute in subsequent multi-agent stages, justifying the external tool cost.
Phase 3 (Full Automation)
Structured Handoff and Cross-Stage Use: Ensure NotebookLM returns outputs in structured JSON and Markdown Templates. Implement cross-stage use (e.g., Compliance Policy Lookup during /audit and Data-Driven Weight Input for RScore optimization).
Full PPP Compliance: Enables empirical optimization of the 70/30 consensus weights and utilizes NotebookLM for specialized Tier 3 (Premium) audit validation.

