# Architect Brief: Maieutic System Design Drift & PRD Generation

**Prepared for**: Architect review
**Date**: 2026-02-06
**Context**: SPEC-PM-001 discovery session — research into external tools, comparison with current maieutic implementation, identification of design drift.

***

## 1. Executive Summary

During research for SPEC-PM-001 (PM-as-spec-management), a significant design drift was identified in the maieutic elicitation system. The word "maieutic" means Socratic inquiry — drawing out knowledge through guided questioning. The current implementation is a static form with validation, not an interactive consulting experience. This brief documents: (a) the current state of the maieutic and PRD systems, (b) external skills/tools that specialize in elicitation, and (c) the gap between intent and implementation.

**Key finding**: The maieutic system was designed to be a Tier-1 interactive consultant that leverages WebSearch, AskUserQuestion, Stage0 product knowledge, and NotebookLM data to guide users through feature definition. What exists today is a checkbox form with field validation. The output quality is bounded by what the user already knows, rather than enriched by what the system knows about the product.

***

## 2. Current State: Maieutic System

### 2.1 Architecture Overview

Two separate elicitation layers exist, neither of which performs Socratic inquiry:

**Layer 1: Spec Intake Modal** (`spec_intake_modal.rs`)

* 11 baseline questions + 8 deep questions
* UI: TUI modal with multiple-choice + custom text input
* Questions presented sequentially, one at a time
* No branching, no context-awareness, no back-and-forth
* Validation in `intake_core.rs`: enforces completeness (non-empty fields, 3-7 items per list, format checks on acceptance criteria)
* Output: `HashMap<String, String>` of answers → fed to `DesignBrief` builder

**Layer 2: Maieutic Pre-Execution** (`maieutic.rs`)

* 5 fast-path questions: Goal, Constraints, Acceptance, Risks, Delegation Bounds
* UI: Same modal pattern with A/B/C/D options + custom
* Duration: 30-90 seconds
* Output: `MaieuticSpec` struct that gates pipeline execution (D130)
* Validation: goal non-empty, acceptance criteria present

### 2.2 Baseline Questions (Layer 1)

| #  | Key                   | Question                             | Format    |
| -- | --------------------- | ------------------------------------ | --------- |
| 1  | `problem`             | What problem does this spec solve?   | Free text |
| 2  | `target_users`        | Semicolon-separated user list        | Free text |
| 3  | `outcome`             | What changes when this is done?      | Free text |
| 4  | `scope_in`            | 3-7 scope items, semicolon-separated | Free text |
| 5  | `non_goals`           | 3-7 non-goals, semicolon-separated   | Free text |
| 6  | `acceptance_criteria` | `<criterion> (verify: <how>)` format | Free text |
| 7  | `constraints`         | Semicolon-separated                  | Free text |
| 8  | `integration_points`  | Semicolon-separated, not "unknown"   | Free text |
| 9  | `risks`               | Semicolon-separated                  | Free text |
| 10 | `open_questions`      | Semicolon-separated                  | Free text |
| 11 | `assumptions`         | Semicolon-separated (optional)       | Free text |

### 2.3 Deep Questions (Layer 1, `--deep` flag)

| #  | Key                       | Question                          |
| -- | ------------------------- | --------------------------------- |
| 12 | `architecture_components` | Components/modules/services       |
| 13 | `architecture_dataflows`  | Edges like "A->B"                 |
| 14 | `integration_mapping`     | `<point> => <modules/files/APIs>` |
| 15 | `test_plan`               | Unit/integration/e2e + key cases  |
| 16 | `threat_model`            | Threats + controls                |
| 17 | `rollout_plan`            | Flags, phases, rollback           |
| 18 | `risk_register`           | `<risk> => <mitigation>`          |
| 19 | `non_goals_rationale`     | `<non-goal> => <why excluded>`    |

### 2.4 Grounding (Deep Mode Only)

In deep mode, `capture_grounding_for_spec_intake()` runs **after** the user completes the form. It captures:

* **Architect Harvest**: Churn analysis, complexity metrics, code skeleton from the codebase
* **Stage0 Project Intel**: Project snapshot (feeds, dependencies, structure)

These are persisted to capsule as grounding artifacts. But they are **not used to inform the questions** — they're captured post-hoc for audit/replay. The user answers questions blind, then grounding data is attached to the capsule record.

### 2.5 PRD Generation Flow

```
User: /speckit.new CORE "Add authentication"
  → SpecIntakeModal shown (11-19 questions)
  → User fills form
  → on_spec_intake_submitted()
    → validate_spec_answers() — field validation
    → generate_next_feature_id() — AREA-FEAT-####
    → [deep] capture_grounding_for_spec_intake() — post-hoc
    → persist_spec_intake_to_capsule() — SoR first
    → create_spec_filesystem_projections() — docs/SPEC-*/
    → update_spec_tracker() — SPEC.md row insertion
```

The PRD itself is generated by `fill_prd_template()` / `fill_prd_template_with_context()` in `new_native.rs` — pure template filling with 80+ placeholder replacements. **No AI reasoning, no enrichment, no product context injection.**

Alternative entry: `on_prd_builder_submitted()` in `prd_builder_handler.rs` collects only 3 fields (Problem, Target, Success) and calls `create_spec_with_context()`.

### 2.6 What's Missing (the drift)

| Intended Capability                                                                             | Current State                                             | Gap          |
| ----------------------------------------------------------------------------------------------- | --------------------------------------------------------- | ------------ |
| Socratic inquiry — adaptive follow-up questions based on answers                                | Static sequential form, no branching                      | **Critical** |
| Product knowledge injection — use Stage0 data to suggest integration points, constraints, risks | Grounding captured post-hoc, not used during questioning  | **Critical** |
| WebSearch for market/technical context                                                          | Not used at all during intake                             | **High**     |
| NotebookLM queries for prior decisions, patterns                                                | Not integrated into intake flow                           | **High**     |
| AskUserQuestion-style interactive dialog                                                        | Modal form with predefined fields                         | **High**     |
| Iterative refinement — ask follow-ups until quality threshold met                               | Single pass through fixed question list                   | **High**     |
| Context-aware question generation — skip questions already answered by project docs             | All questions always asked regardless of existing context | **Medium**   |
| Constitution/vision alignment check during intake                                               | Constitution version captured, no alignment analysis      | **Medium**   |

***

## 3. External Skills Research

### 3.1 Methodology

Searched MCPMarket, GitHub skill repositories (VoltAgent, Anthropic, obra/superpowers, ComposioHQ, Exploration-labs, levnikolaevich), and SaaS platforms (ChatPRD). Fetched and read raw SKILL.md files where available. Compared against spec-kit's maieutic design intent.

### 3.2 Skills Evaluated

#### 3.2.1 obra/superpowers — Brainstorming Skill

**Source**: [github.com/obra/superpowers](https://github.com/obra/superpowers) (`skills/brainstorming/SKILL.md`)

**What it does**: Collaborative design exploration through Socratic questioning.

**Process**:

1. Read project context first (files, docs, recent commits)
2. Ask questions one at a time — multiple-choice preferred, open-ended acceptable
3. Focus on: purpose, constraints, success criteria
4. Propose 2-3 approaches with trade-offs, lead with recommendation
5. Present design in 200-300 word sections, validate each incrementally
6. Write validated design to `docs/plans/YYYY-MM-DD-<topic>-design.md`
7. Optionally hand off to implementation (worktree + plan)

**Key principles**:

* One question at a time — don't overwhelm
* Multiple choice preferred — easier to answer
* YAGNI ruthlessly — remove unnecessary features
* Explore alternatives — always 2-3 approaches before settling
* Incremental validation — present sections, validate each

**Relevance to maieutic**: **High philosophical match**. This is closest to the intended Socratic approach. Reads project context before questioning. Presents alternatives. Validates incrementally. But: no structured rubric, no validation gates, no quality scoring, no capsule integration.

***

#### 3.2.2 Exploration-labs — Requirements Elicitation Skill

**Source**: [github.com/Exploration-labs/Nates-Substack-Skills](https://github.com/Exploration-labs/Nates-Substack-Skills) (`requirements-elicitation/SKILL.md`)

**What it does**: 5-phase systematic requirements analysis.

**Process**:

1. **Initial Analysis**: Read entire doc, load technical dimensions checklist, systematically review against each dimension, document gaps
2. **Question Generation**: Organize by stakeholder (PM vs Engineering), make specific, explain why each matters
3. **Risk Assessment**: 8 categories (Implementation, Performance, Security, Data Integrity, Integration, Operations, UX, Compliance), 4 severity levels, link risks to gaps
4. **Output**: Gap analysis template + clarifying questions document
5. **Post-Clarification**: Only THEN create tech specs/stories/APIs

**Core principle**: "ELICIT, DON'T INVENT" — identify gaps and ask questions, never fill in missing details with assumptions.

**Reference files**: `references/technical_dimensions.md` (comprehensive checklist), `references/question_templates.md` (structuring principles), `references/risk_assessment.md` (framework)

**Relevance to maieutic**: **Highest methodological match**. The "elicit, don't invent" principle IS the maieutic principle. Phase 5 gate ("don't create specs until gaps are filled") mirrors D130. Technical dimensions checklist is analogous to baseline/deep tiers. **Gap**: Designed to analyze an existing document, not create one from scratch. Needs to work in reverse — build up rather than tear down.

***

#### 3.2.3 levnikolaevich — Opportunity Discoverer (ln-201)

**Source**: [github.com/levnikolaevich/claude-code-skills](https://github.com/levnikolaevich/claude-code-skills) (`ln-201-opportunity-discoverer/SKILL.md`)

**What it does**: Sequential KILL funnel for opportunity validation.

**Process**: 6 filters — Traffic Channel → Existing Demand → Competition → Revenue Potential → Personal Interest → MVP-ability. Fail any = KILL immediately.

**Key methods**:

* Uses `WebSearch` for real data at each filter (search volume, competitor analysis, pricing)
* Uses `AskUserQuestion` for subjective inputs (interest rating 1-5)
* Sequential gating with early termination
* Structured output: recommendation + KILL log

**Relevance to maieutic**: **Highest structural match**. Sequential gating with early termination is exactly the pipeline model. Uses real-time research (WebSearch) to inform decisions rather than relying solely on user input. Mixes automated research with interactive questions. **Gap**: Wrong domain (market validation, not feature definition). The *pattern* is directly applicable.

***

#### 3.2.4 alirezarezvani — Product Manager Toolkit

**Source**: [github.com/alirezarezvani/claude-skills](https://github.com/alirezarezvani/claude-skills) (`product-team/product-manager-toolkit/SKILL.md`)

**What it does**: RICE prioritization, customer interview analysis, PRD templates.

**Relevance to maieutic**: **Low match**. Framework toolbox, not guided elicitation. RICE scoring and interview analysis are post-discovery, not discovery itself.

***

#### 3.2.5 ChatPRD (SaaS)

**Source**: [chatprd.ai](https://www.chatprd.ai/)

**What it does**: AI-driven PRD creation platform. Asks targeted questions about audience and features. Has MCP server for Claude Code integration (live PRD querying).

**Relevance to maieutic**: **Medium match**. Interactive questioning approach. MCP integration is interesting for querying PRDs during development. But: proprietary SaaS, not inspectable, no capsule integration.

***

#### 3.2.6 MCPMarket Skills (not fully inspectable — rate-limited)

Three skills identified but raw content not retrievable:

* **Product Requirements ("Sarah")**: Virtual Product Owner with 100-point quality scoring across business value, functional requirements, UX, technical constraints. Iteratively asks questions until 90+ quality threshold. Reads project context (README, configs).
* **Business Analysis & Requirements Elicitation**: Professional BA with adaptive questioning, document analysis, stakeholder mapping.
* **Brainstorming & Design**: Socratic questioning to uncover purpose and constraints, multi-path exploration into architectural designs.

***

### 3.3 Comparison Matrix

| Capability                               |    Current Maieutic   |      obra Brainstorming     |     Nates Elicitation    | ln-201 Opportunity |      "Sarah" PRD      |
| ---------------------------------------- | :-------------------: | :-------------------------: | :----------------------: | :----------------: | :-------------------: |
| Reads project context before questioning |  Post-hoc only (deep) |  Yes (files, docs, commits) |    Yes (existing doc)    |         No         | Yes (README, configs) |
| Adaptive follow-up questions             |           No          |     Yes (one at a time)     |       Yes (by gap)       |  Yes (per filter)  |    Yes (iterative)    |
| Uses WebSearch for real data             |           No          |              No             |            No            | Yes (core feature) |        Unknown        |
| Quality scoring / threshold gate         | Field validation only |              No             |   Risk severity levels   |  KILL/PASS binary  |    100-point rubric   |
| Multiple approaches / alternatives       |           No          |     Yes (2-3 per topic)     |            No            |         N/A        |        Unknown        |
| Structured output to specific format     |   PRD template fill   |          Design doc         | Gap analysis + questions | Recommendation doc |          PRD          |
| Capsule/SoR integration                  |          Yes          |              No             |            No            |         No         |           No          |
| "Don't invent" / elicit-only principle   |      Not explicit     |           Implicit          |     **Explicit core**    |      Implicit      |        Unknown        |
| Incremental validation                   |           No          | Yes (200-300 word sections) |         By phase         |      By filter     |    By quality score   |

***

## 4. Synthesis: What a Maieutic System Should Be

Based on the original design intent, the external research, and the identified drift, the maieutic system should combine:

### 4.1 From obra/superpowers (philosophy)

* One question at a time, context-aware
* Read project state BEFORE asking anything
* Propose 2-3 alternatives with trade-offs
* Validate incrementally, not all-at-once

### 4.2 From Nates elicitation (methodology)

* "Elicit, don't invent" as core principle
* Technical dimensions checklist (systematically cover all angles)
* Risk assessment linked to gaps
* Post-clarification gate: only produce artifacts after gaps are filled (= D130)

### 4.3 From ln-201 opportunity discoverer (structure)

* Sequential gating with early termination
* WebSearch for real data at decision points
* Mix automated research with AskUserQuestion for judgment calls
* Structured kill/pass log

### 4.4 From the spec-kit ecosystem (unique capabilities)

* **Stage0 Project Intel**: Codebase structure, dependencies, existing patterns — should inform questions about integration points, architecture, constraints
* **Capsule history**: Prior specs, their outcomes, what worked — should inform scoping and risk assessment
* **NotebookLM product knowledge**: Design decisions, product vision, constitution — should check alignment and surface relevant prior art
* **Constitution/vision**: Should validate that proposed feature aligns with product direction
* **Capsule persistence**: All elicitation artifacts persisted to SoR for replay/audit

### 4.5 Proposed Flow (conceptual)

```
User: /speckit.new CORE "Add authentication" [--deep]

Phase 0: Context Gathering (automated, ~5s)
├── Stage0 Project Intel snapshot
├── Read constitution + vision
├── Capsule: query related specs (completed, planned, deprecated)
├── NotebookLM: query for prior decisions about auth/security
└── Output: ContextBundle (available to all subsequent phases)

Phase 1: Problem Discovery (interactive, ~2-5 min)
├── Present what system already knows: "Based on project analysis..."
├── Ask focused questions informed by context:
│   "I see 3 integration points in the codebase that touch auth.
│    Which of these are in scope? [A] [B] [C] [D: Other]"
├── WebSearch for relevant prior art / standards
├── Follow-up on ambiguous answers
├── Gate: Problem + Users + Outcome clear? If not, loop.
└── Output: ProblemFrame

Phase 2: Scope & Constraints (interactive, ~2-5 min)
├── Propose scope based on context + problem frame
│   "Given the codebase structure, I'd suggest scope includes X, Y, Z.
│    Does this look right?"
├── Surface constraints from project (existing APIs, policy, etc.)
├── Ask about additional constraints
├── Gate: Scope bounded? Non-goals explicit? If not, loop.
└── Output: ScopeFrame

Phase 3: Risk & Feasibility (interactive, ~2-3 min)
├── Auto-identify risks from integration points + codebase analysis
├── WebSearch for known issues in proposed approach
├── Ask user to confirm/add risks
├── Assess MVP-ability (similar to ln-201 Filter 6)
├── Gate: Risks acknowledged? Feasibility confirmed?
└── Output: RiskFrame

Phase 4: Acceptance & Success (interactive, ~1-2 min)
├── Propose acceptance criteria based on scope
├── Ask for verification methods
├── Check against existing test patterns in codebase
└── Output: AcceptanceFrame

Phase 5: [Deep only] Architecture & Design (~3-5 min)
├── Propose architecture based on existing codebase patterns
├── Present 2-3 approaches with trade-offs
├── Threat model informed by security patterns
├── Test plan informed by existing test structure
└── Output: DesignFrame

Phase 6: Synthesis & PRD Generation (automated, ~5s)
├── Assemble all frames into DesignBrief
├── Generate PRD from brief (template + context enrichment)
├── Persist to capsule (SoR)
├── Create filesystem projections
├── Quality score + completeness check
└── Output: PRD + capsule artifacts
```

***

## 5. Design Questions for Architect

### 5.1 Scope

1. **Is the Phase 0 → Phase 6 flow the right decomposition, or should phases be different?**
2. **Should the maieutic flow replace the current intake modal entirely, or coexist as "assisted" vs "quick" modes?**
3. **What's the right interaction model — conversational (like brainstorming skill) or structured-with-guidance (like current modal but enriched)?**

### 5.2 Data Sources

4. **Which Stage0 data should inform intake questions?** Project Intel snapshot is available. Architect Harvest (churn, complexity) is available. What's most valuable for the user during elicitation?
5. **Should NotebookLM be queried during intake?** If yes, what queries? Prior decisions? Similar features? Product vision alignment?
6. **Should capsule history (completed specs, deprecated specs) be surfaced?** E.g., "SPEC-KIT-972 implemented hybrid retrieval last month — is this related?"

### 5.3 Quality & Gating

7. **Should there be a quality score (like "Sarah"'s 100-point rubric)?** Or is the current field validation + D130 gate sufficient?
8. **What's the minimum viable maieutic?** The full Phase 0-6 flow is ambitious. What's the smallest increment that moves from "checkbox form" to "interactive consultant"?

### 5.4 Architecture

9. **Where does the maieutic conversation state live?** Currently answers are a `HashMap<String, String>` in the modal. An interactive flow needs richer state (context bundle, partial frames, follow-up history).
10. **Does the maieutic flow need an agent?** Current intake is zero-agent ($0). Interactive elicitation with WebSearch/NotebookLM likely needs at least one agent. What's the cost/latency budget?
11. **TUI modal vs conversation mode?** The current TUI modal is a constrained input surface. Socratic inquiry may need a richer interaction pattern (conversation thread with inline questions).

### 5.5 Relationship to SPEC-PM-001

12. **Should maieutic improvement be a prerequisite for SPEC-PM-001, or can they proceed in parallel?** The PM analysis assumes the intake produces high-quality work items. If intake is a checkbox, the PM layer tracks checkbox outputs.
13. **Should the maieutic produce a `WorkItemRegistered` event?** If yes, this is the natural integration point between the two efforts.

***

## 6. External Resources

### Skills (installable, raw content reviewed)

| Skill                    | Repo                                                                                                | Match Type                                |
| ------------------------ | --------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| Brainstorming            | [obra/superpowers](https://github.com/obra/superpowers)                                             | Philosophy (Socratic, incremental)        |
| Requirements Elicitation | [Exploration-labs/Nates-Substack-Skills](https://github.com/Exploration-labs/Nates-Substack-Skills) | Methodology ("elicit, don't invent")      |
| Opportunity Discoverer   | [levnikolaevich/claude-code-skills](https://github.com/levnikolaevich/claude-code-skills)           | Structure (sequential gating + WebSearch) |
| Product Manager Toolkit  | [alirezarezvani/claude-skills](https://github.com/alirezarezvani/claude-skills)                     | Low (framework toolbox)                   |

### Platforms

| Platform            | URL                                                     | Notes                                                  |
| ------------------- | ------------------------------------------------------- | ------------------------------------------------------ |
| ChatPRD             | [chatprd.ai](https://www.chatprd.ai/)                   | MCP server for live PRD querying; SaaS                 |
| MCPMarket           | [mcpmarket.com](https://mcpmarket.com)                  | Skills marketplace; "Sarah" PRD skill (100-pt scoring) |
| Claude Code for PMs | [ccforpms.com](https://ccforpms.com/advanced/write-prd) | Teaching resource; Socratic questioning methodology    |

### Skill collections

| Collection                       | URL                                                                                                |
| -------------------------------- | -------------------------------------------------------------------------------------------------- |
| VoltAgent awesome-agent-skills   | [github.com/VoltAgent/awesome-agent-skills](https://github.com/VoltAgent/awesome-agent-skills)     |
| Anthropic official skills        | [github.com/anthropics/skills](https://github.com/anthropics/skills)                               |
| Awesome Claude Skills (Composio) | [github.com/ComposioHQ/awesome-claude-skills](https://github.com/ComposioHQ/awesome-claude-skills) |

***

## 7. Key Source Files (Current Implementation)

| File                                                          | Lines   | Purpose                                                    |
| ------------------------------------------------------------- | ------- | ---------------------------------------------------------- |
| `codex-rs/tui/src/bottom_pane/spec_intake_modal.rs`           | 316-514 | Baseline + deep questions (the form)                       |
| `codex-rs/tui/src/chatwidget/spec_kit/maieutic.rs`            | 1-823   | MaieuticSpec struct, fast-path questions, persistence      |
| `codex-rs/tui/src/chatwidget/spec_kit/maieutic_handler.rs`    | 1-103   | Modal event handlers                                       |
| `codex-rs/tui/src/chatwidget/spec_kit/intake.rs`              | 1-80+   | Intake schemas (DesignBrief, AceIntakeFrame)               |
| `codex-rs/tui/src/chatwidget/spec_kit/intake_core.rs`         | 1-1935  | Validation + capsule persistence (UI-independent)          |
| `codex-rs/tui/src/chatwidget/spec_kit/spec_intake_handler.rs` | 1-264+  | Intake submission → capsule → filesystem                   |
| `codex-rs/tui/src/chatwidget/spec_kit/grounding.rs`           | 1-60+   | Deep grounding capture (Architect Harvest + Project Intel) |
| `codex-rs/tui/src/chatwidget/spec_kit/new_native.rs`          | 43-792  | Template filling + SPEC.md tracker update                  |
| `codex-rs/tui/src/chatwidget/spec_kit/prd_builder_handler.rs` | 1-80+   | PRD builder modal (3 questions only)                       |

***

*End of brief.*
