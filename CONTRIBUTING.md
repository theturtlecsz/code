# CONTRIBUTING.md

## Rules of Engagement for codex-rs Development

Welcome to the codex-rs team. As Principal Technical Lead, my expectation is that all contributions adhere to the architectural principles defined in our current risk assessment. We must prioritize modularity, explicit dependencies, and maintainability to protect the long-term health of the codebase.

---

## 1. Architecture Overview: Mental Model and Communication

The codex-rs system follows a clear separation of concerns, fundamentally dividing the user interface from the core operational logic.

### Mental Model

1. **TUI (Terminal User Interface)**: The TUI components, primarily managed by `codex-rs/tui/src/app.rs`, serve as the user interaction surface. The ChatWidget component, despite its current complexity, is intended to manage the conversational state, rendering pipeline, event routing, and complex sub-features like agent orchestration and history management.

2. **Backend/Core (Codex Engine)**: This layer handles core conversational operations (`Op`) and complex business logic (like the DCC pipeline or sandbox policies).

### North Star for Communication

Communication between the TUI and the Core must be explicit and message-driven, limiting the blast radius of changes:

- **TUI to Core**: The TUI uses channels (`codex_op_tx`) to send core conversational operations (`Op`) to the Codex engine.
- **Core to TUI**: The core application logic (`app.rs`) consumes and handles internal events defined by the `AppEvent` enumeration in `codex-rs/tui/src/app_event.rs`. If you modify the structure of an event, you must directly update the matching logic in `app.rs`.
- **UI Events**: The ChatWidget sends internal UI events back to the main application loop (`app.rs`) via `app_event_tx`.

---

## 2. The "Danger Zones" (High-Risk Modules)

The following components exhibit critical volatility, tight coupling, or disproportionately high quantitative risk. These areas require extreme caution and lead architect review before modification.

### A. The TUI Core (The Chat Nexus)

The TUI is the module suffering the absolute highest churn, indicating ongoing instability or functional entanglement.

| File Path | Metric | Assessment |
|-----------|--------|------------|
| `codex-rs/tui/src/chatwidget.rs` | 509 commits (Highest Churn) | **High Volatility**. This file is an entangled monolith managing presentation, state, and event routing simultaneously. |
| `codex-rs/tui/src/app.rs` | 22 co-changes (Highest Coupling) | **Tight Coupling**. Changes to `chatwidget.rs` frequently require cascading modifications in the main application orchestration in `app.rs`. |

### B. Glue Components (Build and Control Scripts)

These non-Rust components operate as critical control planes (installation, build, execution policy) or external integrations. They lack the type safety of the Rust core and exhibit extremely high risk scores relative to their size, making them fragile and hard to debug.

| Language | Metric | Assessment |
|----------|--------|------------|
| JavaScript | **286.9 Risk Score** | Highest Quantitative Risk. Logic in `codex-cli/package.json` and supporting files is highly prone to failure. |
| BASH/Shell | **265.3 and 157.0 Risk Scores** | Highly Fragile. Complex shell scripts are used for environment configuration and automated validation (using tools like `shellcheck` and `hadolint`). |

---

## 3. Code Standards and Architectural Constraints

Due to observed cognitive complexity anti-patterns, we enforce the following architectural constraints to limit the creation of new "God Functions":

### 1. Avoid Monolithic Control Flow

Do not introduce large, centralized `match` statements handling numerous pathways. For example, the `summarize_item` function currently handles approximately 20 distinct variants of `ResponseItem`, resulting in an unmanageably high cognitive load.

**Standard**: Use Strategy and Polymorphism (Rust traits) to abstract complexity. Implement logic within a trait for each variant and reduce the orchestration function to a single trait call (e.g., `item.to_summary()`).

### 2. Limit Parameter Arity

Avoid creating functions with complex, numerous optional parameters (e.g., the sandbox policy definition function that takes nearly 10 optional parameters).

**Standard**: Employ the Fluent Builder Pattern to decouple initialization complexity and improve readability.

### 3. No New JavaScript or Shell Control Logic

Due to the existing high-risk scores (JS: **286.9**; Shell: **157.0**), do not introduce new, complex build logic, external integrations, or execution policies via JavaScript or Shell scripts. Priority should be given to refactoring existing high-risk code.

### 4. Enforce Explicit Dependencies

Be mindful of creating Leaky Abstractions. If a change to Module A forces a change to Module B, there must be an explicit structural dependency (an import or method reference). If changes are required without an explicit link, this indicates that the abstraction has failed by leaking hidden behavioral or integration requirements.

---

## 4. Current Objectives: ChatWidget Decomposition

The most critical refactoring task is the decomposition of the ChatWidget component to address its high churn (509 commits) and functional entanglement. This is an active roadmap item:

The goal is to implement the **Single Responsibility Principle** by splitting the monolithic ChatWidget into three orthogonal components: `ChatState` (Data/Business Logic), `ChatRender` (Presentation/View), and `ChatInput` (Control/Input Handling).

### Implementation Plan

| New Component | Primary Responsibility | Data/Fields to Own (Moved from `ChatWidget<'a>`) |
|---------------|------------------------|--------------------------------------------------|
| `ChatState` | Data Model & Business Logic. Holds all persistent and volatile session data. Implements event handling logic. | `history_cells`, `stream`, `tools_state`, agents terminal state, and cost/rate limit tracking. |
| `ChatRender` | View & Presentation. Contains non-stateful rendering logic. | Read-only access to state; manages layout and height calculation. |
| `ChatInput` | Control & User Interaction. Translates user input (keys, composer text) into operations for the ChatState or the Core engine. | `app_event_tx`, `codex_op_tx`, and manages the `BottomPane` input structure. |
