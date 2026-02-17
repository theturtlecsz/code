# Planner Vision (Project Truth)

Planner is a terminal TUI focused on **Spec-Kit workflows**.

This document captures product identity at a glance. For governance and scope, treat `memory/constitution.md` (charter) and `product-requirements.md` (requirements) as authoritative; `SPEC.md` is the canonical work tracker. If runtime behavior conflicts with this doc, treat it as a migration gap or code regression candidate (don't "fix docs to match" without explicitly calling out the divergence).

## Product Surface Area (from code)

* **Primary binary name**: `code` (`codex-rs/cli/Cargo.toml`)
* **Primary UX**: interactive TUI (default `code` behavior; no subcommand)
* **Primary workflow contract**: Spec-Kit slash commands under the `/speckit.*` namespace
* **Deprecated legacy UX**: `/plan`, `/solve`, `/code` are removed; invoking them shows a migration message

## Where Spec-Kit Lives

* **TUI integration (slash routing, pipeline orchestration, UI)**: `codex-rs/tui/src/chatwidget/spec_kit/`
* **Shared Spec-Kit library crate (config/retry/types)**: `codex-rs/spec-kit/`
* **Templates**: project-local `./templates/*.md` (optional) plus embedded fallbacks
* **Evidence storage**: `docs/SPEC-OPS-004-integrated-coder-hooks/evidence/` (guardrails/telemetry/consensus artifacts)

## Canonical Invocation

* Build and run locally via `./build-fast.sh run`
* Use `/speckit.project` (optional) to scaffold a new project, then `/speckit.new` + `/speckit.auto` for end-to-end runs

***

## Spec-Kit Product Vision & Operating Contract

**Version:** 1.1.0 (Epoch 1 - Hardened)

### The Core Promise

Spec-Kit is a **Consultant-First, Spec-Driven Development System** that evolves into a **Semi-Autonomous Lab**.

1. **Consultant Mode (Intake):**
   * *Standard:* Interactive chat transforms abstract intent into a structured **Project Packet** (Charter + Milestones).
   * *Fast-Track:* "Infer & Confirm" mode scrapes existing context to auto-generate the Packet for 1-click sign-off.
2. **Lab Mode (Execution):** A semi-autonomous loop (Research -> Prototype -> Implement -> Review) executes the Packet.
3. **The Bridge:** The **Packet** (`.speckit/packet.yaml`) is the source of truth. The Consultant builds it; the Lab executes it.

**We do not build "Chat with Code." We build "Packet-Driven Autonomy."**

### Sacred Anchors & Epochs

To prevent drift, we define **Sacred Anchors**. Silent violation is a system failure.

#### The Sacred Fields

1. **User Intent Summary:** The "Why" and "What" (immutable without agreement).
2. **Success Criteria:** The definition of "Done" (immutable without agreement).

#### The Epoch Protocol

* **Material Drift = New Epoch.** If reality forces a change to Sacred Anchors (e.g., "Must switch tech stack"), the system **MUST NOT** drift silently.
* **Charter Amendment.** The system triggers a **Class 2 Decision** to amend the Packet.
* **Reverse Sync.** If implementation details shift (e.g., we swapped a library), the Agent *must* propose a Packet update to keep the spec true to the code.

### The Thread Model

We prioritize **green build guarantees** over raw velocity.

#### Single Primary Thread (The Merge Train)

* **Cardinality:** ONE per project.
* **Role:** Integration & Shipping.
* **Constraint:** Serialized execution. "The Train leaves only when green."

#### Research & Review Threads (Speculative)

* **Cardinality:** Many.
* **Role:** Prototyping, Refactoring, Auditing.
* **Privilege:** **NEVER MERGE.** They output Proposals or PRs for the Train.

### The Milestone Contract

"Progress" = **Completing the current Milestone Contract.**

#### Milestone Types

1. **Ship:** Delivers code. *Done* = Hard Gates Passed.
2. **Decision:** Delivers a choice. *Done* = Recommendation + Evidence + Sign-off.
3. **Artifact:** Delivers a plan/spec. *Done* = Artifact verified.

#### The Milestone Boundary

* **Definition:** The clean state where a Milestone is `Done`.
* **The Checkpoint:** **Class 2 Changes** (Major Architecture/Deps) can *only* be adopted here.

### Governance & Change Budgets

#### Change Classes

| Class | Name            | Scope                     | Permission                   |
| :---- | :-------------- | :------------------------ | :--------------------------- |
| **0** | **Routine**     | Typos, docs, small fixes. | Auto-merge (Attended).       |
| **1** | **Significant** | Logic, internal files.    | Auto-merge (Attended).       |
| **2** | **Major**       | New deps, Arch refactors. | **Milestone Boundary ONLY.** |
| **E** | **Emergency**   | Critical Security/Hotfix. | **Immediate Bypass.**        |

#### The "Medium" Budget (Default)

* **Allowed:** Internal logic, dev-deps, < 15 files.
* **Forbidden:** Runtime deps, public API breaks, > 15% refactor.

#### Emergency Protocol (Class E)

* **Trigger:** Critical Security Vulnerability (CVE) or Production Outage.
* **Action:** Bypasses Milestone Boundary. Bypasses Serialization (Priority Interrupt).
* **Constraint:** Must be minimal blast radius.

### Autonomy & User Presence

#### Presence Modes

1. **Attended:**
   * *Primary:* Auto-merge Class 0/1 (Gates passed).
   * *Intake:* Interactive.
2. **Unattended:**
   * *Primary:* **NO MERGES.**
   * *Behavior:* **Stacking.** System prepares a "Morning Brief" PR (verified stack).
   * *Research:* Active.

#### The Recap Rule

**Before ANY execution shift or merge:**
The system outputs a **Recap**:

1. **Intent** (Refresh context).
2. **Plan** (Next actions).
3. **Gates** (Safety checks).
4. **Rollback** (Undo path).

### Scoring & Confidence

Math, not vibes.

#### Score Composition

`Score = (Intent_Match * 0.4) + (Performance * 0.2) + (Simplicity * 0.2) - (Thrash_Penalty)`

#### Hysteresis (Stability Bias)

New proposals must exceed current score by a **Dominance Margin** (default 15%) to displace the Plan-of-Record.

#### Confidence Thresholds

"High Confidence" (Auto-pick) requires:

1. `Confidence > 0.85` (derived from Test Coverage + Lint Cleanliness).
2. `Dominance Margin > 0.15`.
3. `Evidence = High` (Prototype verified).

### Postures (Risk Profiles)

| Posture      | Class 2 Policy         | Intake Mode      | Auto-Merge            |
| :----------- | :--------------------- | :--------------- | :-------------------- |
| **Safety**   | Manual Only.           | Interactive.     | Disabled.             |
| **Balanced** | Manual.                | Standard.        | Class 0/1 (Attended). |
| **Speed**    | Auto-pick (High Conf). | Infer & Confirm. | Class 0/1.            |

### Next 90 Days Build Plan

#### Phase 1: The Trust Foundation (Days 1-30)

* [ ] **Packet Persistence:** Implement `.speckit/packet.yaml` read/write.
* [ ] **Gatekeeper:** Implement "No Merge Without Recap" and "No Class 2 Mid-Milestone" logic.
* [ ] **Emergency Valve:** Implement Class E bypass.

#### Phase 2: The Autonomous Lab (Days 31-60)

* [ ] **Stacking:** Implement "Morning Brief" logic for Unattended mode.
* [ ] **Proposal Ranking:** Implement Score-based inbox filtering.
* [ ] **Reverse Sync:** Agent detects Code/Packet drift and prompts updates.

#### Phase 3: The Learning Loop (Days 61-90)

* [ ] **Hysteresis Engine:** Implement Stability Bias in decision scoring.
* [ ] **Self-Correction:** Agent auto-retries failed builds N times with new context before escalating.

***

Back to [Key Docs](KEY_DOCS.md)
