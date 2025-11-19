ntegration Architecture: NotebookLM ↔ Claude Code
Overview
The integration of NotebookLM will execute the deferred Research/Reasoning Integration feature (Phase 3), which enhances the Productivity (RProd) dimension of the PPP Framework. NotebookLM is integrated as an external knowledge tool via the Model Context Protocol (MCP), providing high-effort research and contextual synthesis. This approach respects the cost-optimized philosophy by reserving the high-cost NotebookLM calls (∼$0.50 estimated) only for Specific + Complex tasks, preventing wasted compute.
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
Markdown Template (Research Brief format with # Key Insight and ## Trade-Offs sections).
Tool Result (MCP): MCP Client injects the structured brief into the Claude agent's context.
Plan
Multi-agent consensus identifies a Critical Architectural Conflict (e.g., layered vs. monolithic).
JSON-RPC query listing conflicting outputs and the architectural aspect.
Queries historical ADRs and Evidence Repository to find precedent or consensus. Summarizes risk of each option.
JSON (Structured list of recommended patterns, confidence scores, and risks).
MCP Tool Invocation: Passed to the Consensus Coordinator for synthesis.
Audit
Tier 3 Premium Agents are auditing code/dependencies. Checks for compliance failures (e.g., licensing, security policy).
JSON-RPC query listing dependencies or security requirements (e.g., "Check component X for compliance with internal policy Y").
Queries internal security policies or license databases for the specific project context.
JSON (Structured report with status and conflict_severity).
MCP Tool Invocation: Results are integrated into the final audit_report.md artifact.
RScore
Data-Driven Weight Optimization is run (Phase 3).
Query for historical consensus.json artifacts and execution.json telemetry.
Analyzes past Reasoning Regret (linking architectural decisions to success rates) to find optimal PPP consensus weights.
JSON (Structured output for Grid Search algorithm).
Direct DB Query/Analysis: Runs outside the core pipeline, feeding data back to config.toml defaults.
Data Flow Diagrams
The workflow adheres to the Sequential Decision Logic:
graph TD
    A[1. User Input (Vague + Complex)] --> B{2. Vagueness Check?};
    B -- Vague --> C[3. Agent Q&A (RProact)];
    C --> D{4. Clarification Complete?};
    D -- Yes --> E{5. Complexity Check?};
    D -- No --> F[Escalate to Human (Max Turns)]
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
The integration requires modifications across the configuration, core services, and the Spec-Kit framework.
• Path: docs/SPEC-PPP-XXX-notebooklm-integration/ (NEW directory for this SPEC)
• Path: codex-rs/core/src/config_types.rs (Modification: Add configuration section for [mcp_servers.notebooklm])
• Path: codex-rs/core/src/mcp_connection_manager.rs (Modification: Add logic to initialize and monitor the notebooklm-mcp server)
• Path: codex-rs/tui/src/chatwidget/spec_kit/prompt_processor.rs (Modification: Implement Complexity Analysis component and Sequential Logic to trigger research. This is the core execution point.)
• Path: codex-rs/tui/src/chatwidget/spec_kit/agent_orchestrator.rs (Modification: Logic to execute Claude Code with ReasoningEffort::High and instruct tool usage)
• Path: codex-rs/tui/src/chatwidget/spec_kit/consensus.rs (Modification: Logic to handle NotebookLM output during synthesis in the Plan and Audit stages)
Dependencies
• New Tools/Libraries Needed: No new libraries are required for the core Rust application, as the MCP Client and Connection Manager already exist in codex-rs/core/.
• MCP Servers Required: The public notebooklm-mcp server must be installed and configured locally by the user.
• Configuration Changes:
    ◦ Add the [mcp_servers.notebooklm] definition to ~/.code/config.toml.
    ◦ Update model_reasoning_effort and potentially quality_gates.audit in config.toml to utilize the new tool.
Implementation Phases
The NotebookLM integration is primarily a Phase 3 feature, prioritized based on ROI (preventing wasted compute).
Phase
Goal
Rationale
Phase 1 (Minimal Viable Integration)
Configure MCP Server: Ensure the notebooklm-mcp server launches successfully and is tracked by the McpConnectionManager. No agents use it yet.
Dependency Foundation: Validates that the external tool ecosystem can be correctly integrated without breaking existing pipelines.
Phase 2 (Enhanced Capabilities)
High Reasoning Tool Invocation: Implement the Complexity Analysis component and Sequential Logic. Allow agents to invoke NotebookLM only for Specific + Complex prompts during /specify or /plan.
Highest ROI: Prevents wasted compute by ensuring the high cost (∼$0.50) of research is only incurred when genuinely necessary.
Phase 3 (Full Automation)
Full Cross-Stage Integration: Enable NotebookLM to perform Compliance Policy Lookup during /audit and Data-Driven Weight Input for the Grid Search algorithm. Implement structured JSON/Markdown parsing of all returned research briefs.
95% PPP Compliance: Achieves maximum RProd compliance and enables empirical optimization of the 70/30 consensus weights.

